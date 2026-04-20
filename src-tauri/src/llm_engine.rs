use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel, Special};
use llama_cpp_4::sampling::LlamaSampler;

#[cfg(any(feature = "accel-metal", feature = "accel-vulkan", feature = "accel-cuda"))]
const GPU_LAYERS: u32 = 999;
#[cfg(not(any(feature = "accel-metal", feature = "accel-vulkan", feature = "accel-cuda")))]
const GPU_LAYERS: u32 = 0;

static BACKEND: OnceLock<Arc<LlamaBackend>> = OnceLock::new();

fn backend() -> Result<Arc<LlamaBackend>, String> {
    if let Some(b) = BACKEND.get() {
        return Ok(b.clone());
    }
    let b = LlamaBackend::init().map_err(|e| format!("llama backend init failed: {}", e))?;
    let arc = Arc::new(b);
    let _ = BACKEND.set(arc.clone());
    Ok(BACKEND.get().cloned().unwrap_or(arc))
}

pub struct LlmEngine {
    model: LlamaModel,
    n_ctx: u32,
}

unsafe impl Send for LlmEngine {}
unsafe impl Sync for LlmEngine {}

impl LlmEngine {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let backend = backend()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(GPU_LAYERS);
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load LLM model: {}", e))?;
        Ok(Self {
            model,
            n_ctx: 8192,
        })
    }

    pub fn generate(
        &self,
        prompt: &str,
        max_tokens: u32,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<String, String> {
        let backend = backend()?;
        let n_batch: u32 = 512;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(self.n_ctx))
            .with_n_batch(n_batch);
        let mut ctx = self
            .model
            .new_context(&backend, ctx_params)
            .map_err(|e| format!("Failed to create LLM context: {}", e))?;

        let tokens_list = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize prompt: {}", e))?;

        let n_prompt = tokens_list.len() as i32;
        if n_prompt as u32 >= self.n_ctx {
            return Err(format!(
                "Prompt too long: {} tokens vs {} ctx",
                n_prompt, self.n_ctx
            ));
        }

        // Decode the prompt in chunks of n_batch. Only the very last token needs
        // logits; for every other prompt token we skip them to save compute.
        let mut batch = LlamaBatch::new(n_batch as usize, 1);
        let chunk_size = n_batch as usize;
        let last_idx = tokens_list.len() - 1;

        for (chunk_start, chunk) in tokens_list
            .chunks(chunk_size)
            .enumerate()
            .map(|(ci, c)| (ci * chunk_size, c))
        {
            batch.clear();
            for (i, tok) in chunk.iter().enumerate() {
                let abs_pos = (chunk_start + i) as i32;
                let needs_logits = chunk_start + i == last_idx;
                batch
                    .add(*tok, abs_pos, &[0], needs_logits)
                    .map_err(|e| format!("batch.add failed: {}", e))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| format!("decode failed: {}", e))?;
        }

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.4),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(1234),
        ]);

        let mut output = String::new();
        let mut n_cur = n_prompt;
        let mut decoded_buf: Vec<u8> = Vec::new();

        let max_total = (n_prompt as u32 + max_tokens).min(self.n_ctx);

        while (n_cur as u32) < max_total {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);

            if self.model.is_eog_token(token) {
                break;
            }

            let token_bytes = self
                .model
                .token_to_bytes(token, Special::Tokenize)
                .map_err(|e| format!("token_to_bytes failed: {}", e))?;

            decoded_buf.extend_from_slice(&token_bytes);

            if let Ok(piece) = std::str::from_utf8(&decoded_buf) {
                on_token(piece);
                output.push_str(piece);
                decoded_buf.clear();
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| format!("batch.add failed: {}", e))?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| format!("decode failed: {}", e))?;
        }

        if !decoded_buf.is_empty() {
            let tail = String::from_utf8_lossy(&decoded_buf).to_string();
            on_token(&tail);
            output.push_str(&tail);
        }

        Ok(output)
    }
}

/// Universal rules shared by every template. Kept in code (not in Template) so
/// users editing templates do not have to know or repeat them.
const GLOBAL_PREAMBLE: &str = "\
Jesteś asystentem lekarza. Przekształcasz dyktowanie głosowe w sformatowaną notatkę medyczną po polsku.

ZASADY:
- Używaj WYŁĄCZNIE faktów z dyktowania. Nie dodawaj własnych objawów, badań, leków ani rozpoznań.
- Liczby słowne zamieniaj na cyfry z jednostkami: \"pięćset miligramów\" → \"500 mg\", \"sto trzydzieści na osiemdziesiąt\" → \"130/80 mmHg\", \"trzydzieści osiem i pięć\" → \"38,5°C\".
- Zachowuj łacińską terminologię medyczną (objaw Blumberga, Lasègue'a, status post) — nie tłumacz.
- Kody ICD-10 podawaj w nawiasach [ICD-10: ___] tylko gdy rozpoznanie jednoznacznie na nie wskazuje.
- Jeśli sekcja nie ma danych z dyktowania, wpisz \"brak danych\" lub \"nie zgłasza\" — nie pomijaj.
- Odpowiadaj TYLKO sformatowaną notatką. Bez wstępu, komentarzy, podsumowania.";

pub fn build_prompt(
    template_content: &str,
    example_input: Option<&str>,
    example_output: Option<&str>,
    user_input: &str,
) -> String {
    let mut p = String::new();

    if let (Some(ex_in), Some(ex_out)) = (example_input, example_output) {
        // Few-shot: show the model one worked example before the real dictation.
        p.push_str("<start_of_turn>user\n");
        p.push_str(GLOBAL_PREAMBLE);
        p.push_str("\n\n");
        p.push_str(template_content.trim());
        p.push_str("\n\nDYKTOWANIE:\n");
        p.push_str(ex_in.trim());
        p.push_str("<end_of_turn>\n<start_of_turn>model\n");
        p.push_str(ex_out.trim());
        p.push_str("<end_of_turn>\n<start_of_turn>user\nDYKTOWANIE:\n");
        p.push_str(user_input.trim());
        p.push_str("<end_of_turn>\n<start_of_turn>model\n");
    } else {
        p.push_str("<start_of_turn>user\n");
        p.push_str(GLOBAL_PREAMBLE);
        p.push_str("\n\n");
        p.push_str(template_content.trim());
        p.push_str("\n\nDYKTOWANIE:\n");
        p.push_str(user_input.trim());
        p.push_str("<end_of_turn>\n<start_of_turn>model\n");
    }

    p
}
