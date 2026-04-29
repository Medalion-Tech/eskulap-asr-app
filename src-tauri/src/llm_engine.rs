use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel, Special};
use llama_cpp_4::sampling::LlamaSampler;

use crate::settings::LlmSettings;

#[cfg(any(
    feature = "accel-metal",
    feature = "accel-vulkan",
    feature = "accel-cuda"
))]
const GPU_LAYERS: u32 = 999;
#[cfg(not(any(
    feature = "accel-metal",
    feature = "accel-vulkan",
    feature = "accel-cuda"
)))]
const GPU_LAYERS: u32 = 0;

/// Bump when GLOBAL_PREAMBLE or slot-format instructions change — invalidates
/// KV cache entries carrying a different version.
pub const PREAMBLE_VERSION: u32 = 2;

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
}

unsafe impl Send for LlmEngine {}
unsafe impl Sync for LlmEngine {}

/// A cached prefix state: file path on disk containing saved KV state, and the
/// token count that state represents. Used by `generate` to skip prefix decode
/// when the caller has a warm cache.
#[derive(Debug, Clone)]
pub struct CachedPrefix {
    pub path: PathBuf,
    pub prefix_token_count: u32,
}

#[derive(Debug, Clone)]
pub struct GenerateOutput {
    pub text: String,
    pub cache_hit: bool,
    /// Prefix tokens re-used from cache (0 if cache miss).
    pub cached_tokens: u32,
    /// Newly generated (sampled) tokens, not counting prompt.
    pub new_tokens: u32,
    /// File + token count to write back to the KV cache index when caller wants
    /// to persist (None when cache was already hot or when save failed).
    pub saved_prefix: Option<CachedPrefix>,
}

impl LlmEngine {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let backend = backend()?;
        let model_params = LlamaModelParams::default().with_n_gpu_layers(GPU_LAYERS);
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load LLM model: {}", e))?;
        Ok(Self { model })
    }

    /// Generate a completion given a fixed `prefix` (cacheable across calls) and
    /// a variable `suffix` (e.g. the user dictation). When `cache_entry` is
    /// provided and matches the prefix length, KV state is loaded from disk
    /// instead of re-decoding the prefix tokens.
    ///
    /// When `save_cache_to` is provided, a fresh cache miss will write KV state
    /// to that path after decoding the prefix (so the next call hits).
    pub fn generate(
        &self,
        prefix: &str,
        suffix: &str,
        settings: &LlmSettings,
        cache_entry: Option<&CachedPrefix>,
        save_cache_to: Option<&Path>,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<GenerateOutput, String> {
        let backend = backend()?;
        let n_batch = settings.batch_size;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(settings.context_size))
            .with_n_batch(n_batch);
        // Flash attention is only meaningful on GPU-accelerated builds.
        #[cfg(any(
            feature = "accel-metal",
            feature = "accel-vulkan",
            feature = "accel-cuda"
        ))]
        let ctx_params = ctx_params.with_flash_attention(settings.flash_attention);
        let mut ctx = self
            .model
            .new_context(&backend, ctx_params)
            .map_err(|e| format!("Failed to create LLM context: {}", e))?;
        ctx.set_n_threads(settings.threads, settings.batch_threads);

        // Tokenize prefix and suffix separately. Prefix gets BOS (start of stream).
        let prefix_tokens = self
            .model
            .str_to_token(prefix, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize prefix: {}", e))?;
        let suffix_tokens = self
            .model
            .str_to_token(suffix, AddBos::Never)
            .map_err(|e| format!("Failed to tokenize suffix: {}", e))?;

        let n_prefix = prefix_tokens.len() as i32;
        let n_suffix = suffix_tokens.len() as i32;
        let n_total = n_prefix + n_suffix;
        if n_total as u32 >= settings.context_size {
            return Err(format!(
                "Prompt too long: {} tokens vs {} ctx",
                n_total, settings.context_size
            ));
        }

        let mut batch = LlamaBatch::new(n_batch as usize, 1);

        // --- Prefix phase ---
        let mut cache_hit = false;
        let mut cached_tokens = 0u32;
        let mut saved_prefix: Option<CachedPrefix> = None;

        let cache_usable = cache_entry
            .filter(|c| c.prefix_token_count as i32 == n_prefix && c.path.exists())
            .is_some();

        if cache_usable {
            let c = cache_entry.unwrap();
            match try_load_session(&mut ctx, &c.path, n_prefix as u32) {
                Ok(loaded_count) if loaded_count as i32 == n_prefix => {
                    cache_hit = true;
                    cached_tokens = loaded_count;
                    log::info!("LLM KV cache hit: reused {} prefix tokens", loaded_count);
                }
                Ok(other) => {
                    log::warn!(
                        "KV cache prefix length mismatch ({} vs {}); falling back to full decode",
                        other,
                        n_prefix
                    );
                    decode_tokens(&mut ctx, &mut batch, &prefix_tokens, 0, false, n_batch)?;
                }
                Err(e) => {
                    log::warn!("KV cache load failed ({}); decoding prefix", e);
                    decode_tokens(&mut ctx, &mut batch, &prefix_tokens, 0, false, n_batch)?;
                }
            }
        } else {
            // Cache miss — decode the prefix normally.
            decode_tokens(&mut ctx, &mut batch, &prefix_tokens, 0, false, n_batch)?;
            if let Some(save_path) = save_cache_to {
                match try_save_session(&ctx, save_path, &prefix_tokens) {
                    Ok(()) => {
                        saved_prefix = Some(CachedPrefix {
                            path: save_path.to_path_buf(),
                            prefix_token_count: n_prefix as u32,
                        });
                        log::info!("LLM KV cache saved: {} prefix tokens", n_prefix);
                    }
                    Err(e) => {
                        log::warn!("KV cache save failed: {}", e);
                    }
                }
            }
        }

        // --- Suffix phase: decode into the same context, last token needs logits ---
        decode_tokens(
            &mut ctx,
            &mut batch,
            &suffix_tokens,
            n_prefix,
            true,
            n_batch,
        )?;

        // --- Sampling loop ---
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(settings.temperature),
            LlamaSampler::top_p(settings.top_p, 1),
            LlamaSampler::dist(settings.seed),
        ]);

        let mut output = String::new();
        let mut n_cur = n_total;
        let mut decoded_buf: Vec<u8> = Vec::new();
        let mut new_tokens = 0u32;

        let max_total = (n_total as u32 + settings.max_tokens).min(settings.context_size);

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
            new_tokens += 1;

            ctx.decode(&mut batch)
                .map_err(|e| format!("decode failed: {}", e))?;
        }

        if !decoded_buf.is_empty() {
            let tail = String::from_utf8_lossy(&decoded_buf).to_string();
            on_token(&tail);
            output.push_str(&tail);
        }

        Ok(GenerateOutput {
            text: output,
            cache_hit,
            cached_tokens,
            new_tokens,
            saved_prefix,
        })
    }
}

/// Decode a range of tokens into `ctx`. Only the very last token (when
/// `last_needs_logits` is true) requests logits — for pure prefill we skip
/// all logits to save compute.
fn decode_tokens(
    ctx: &mut llama_cpp_4::context::LlamaContext,
    batch: &mut LlamaBatch,
    tokens: &[llama_cpp_4::token::LlamaToken],
    start_pos: i32,
    last_needs_logits: bool,
    chunk_size: u32,
) -> Result<(), String> {
    if tokens.is_empty() {
        return Ok(());
    }
    let chunk_size = chunk_size.max(1) as usize;
    let last_idx = tokens.len() - 1;

    for (ci, chunk) in tokens.chunks(chunk_size).enumerate() {
        let chunk_offset = ci * chunk_size;
        batch.clear();
        for (i, tok) in chunk.iter().enumerate() {
            let abs_pos = start_pos + (chunk_offset + i) as i32;
            let is_last = chunk_offset + i == last_idx;
            let needs_logits = last_needs_logits && is_last;
            batch
                .add(*tok, abs_pos, &[0], needs_logits)
                .map_err(|e| format!("batch.add failed: {}", e))?;
        }
        ctx.decode(batch)
            .map_err(|e| format!("decode failed: {}", e))?;
    }
    Ok(())
}

/// Save the current KV state to `path`. `tokens` is the full prefix sequence
/// the state represents (required by llama-cpp).
fn try_save_session(
    ctx: &llama_cpp_4::context::LlamaContext,
    path: &Path,
    tokens: &[llama_cpp_4::token::LlamaToken],
) -> Result<(), String> {
    ctx.save_session_file(path, tokens)
        .map_err(|e| format!("save_session_file: {:?}", e))
}

/// Load a session file into `ctx` and return the number of tokens now in the
/// context's KV cache. Caller must verify the count matches the expected
/// prefix length (we error out in the caller if it doesn't).
fn try_load_session(
    ctx: &mut llama_cpp_4::context::LlamaContext,
    path: &Path,
    max_tokens: u32,
) -> Result<u32, String> {
    let tokens = ctx
        .load_session_file(path, max_tokens as usize)
        .map_err(|e| format!("load_session_file: {:?}", e))?;
    Ok(tokens.len() as u32)
}

// ---------- Prompt construction ----------

const GLOBAL_PREAMBLE: &str = "\
Jesteś asystentem lekarza. Wypełniasz ustrukturyzowany szablon medycznej notatki po polsku na podstawie dyktowania.

ZASADY MEDYCZNE:
- Używaj WYŁĄCZNIE faktów z dyktowania. Nie dodawaj własnych objawów, badań, leków ani rozpoznań.
- Liczby słowne zamieniaj na cyfry z jednostkami: \"pięćset miligramów\" → \"500 mg\", \"sto trzydzieści na osiemdziesiąt\" → \"130/80 mmHg\", \"trzydzieści osiem i pięć\" → \"38,5°C\".
- Zachowuj łacińską terminologię medyczną (objaw Blumberga, Lasègue'a, status post) — nie tłumacz.
- Kody ICD-10 dopisuj w nawiasach [ICD-10: ___] tylko gdy rozpoznanie jednoznacznie na nie wskazuje.

FORMAT ODPOWIEDZI — BARDZO WAŻNE:
Szablon zawiera markery slotów w postaci [[typ:nazwa|atrybuty]]. Musisz zwrócić CAŁY szablon, gdzie KAŻDY marker [[...]] został zastąpiony konkretną wartością z dyktowania. NIGDY nie kopiuj markera dosłownie — zawsze go zastąp.

Reguły dla każdego typu:
- [[field:nazwa|...]] → zastąp krótkim tekstem (fraza/zdanie).
- [[longtext:nazwa|...]] → zastąp akapitem (jedno lub kilka zdań).
- [[pick:nazwa|A=opcja1|B=opcja2|X=nieokreślone|other]] → zastąp SAMYM KODEM wybranej opcji (np. \"A\", \"B\"). Jeśli żadna opcja nie pasuje: \"other: <twój opis>\". Jeśli dyktowanie nie mówi o tym: \"X\".
- [[list:nazwa|...]] → zastąp listą punktów: każdy w osobnej linii z prefiksem \"- \" (lub \"1. \" jeśli numbered=true).
- Jeśli dyktowanie nie wspomina o danym slocie, wpisz dokładnie [[X]].

PRZYKŁAD (szablon z 3 slotami → wypełniona odpowiedź):

Szablon:
Rozpoznanie: [[field:rozpoznanie|hint=ICD-10]]
Stan: [[pick:stan|A=stabilny|B=ciężki|X=nieokreślone|other]]
Zalecenia:
[[list:zalecenia|numbered=true]]

Dyktowanie: \"Pacjent z grypą, stan stabilny. Zalecam paracetamol pięćset trzy razy dziennie przez tydzień, odpoczynek, dużo płynów.\"

Odpowiedź:
Rozpoznanie: Grypa [ICD-10: J11]
Stan: A
Zalecenia:
1. Paracetamol 500 mg 3×/d przez 7 dni
2. Odpoczynek
3. Zwiększone nawodnienie

KONIEC PRZYKŁADU. NIE dodawaj własnych nagłówków ani tekstu poza tym, co jest w szablonie.";

/// Build (prefix, suffix) for a template-based generation.
///
/// Prefix is everything stable per template: preamble + stringified AST + (optional) one-shot.
/// Suffix is the variable part: user dictation framed as the final user turn.
///
/// Splitting this way lets the KV cache warm the prefix once per template and
/// re-use it across dictations.
pub fn build_prompt_split(
    template_stringified: &str,
    example_input: Option<&str>,
    example_output: Option<&str>,
    user_input: &str,
) -> (String, String) {
    let mut prefix = String::new();
    prefix.push_str("<start_of_turn>user\n");
    prefix.push_str(GLOBAL_PREAMBLE);
    prefix.push_str("\n\nSZABLON:\n");
    prefix.push_str(template_stringified.trim());

    if let (Some(ex_in), Some(ex_out)) = (example_input, example_output) {
        prefix.push_str("\n\nDYKTOWANIE:\n");
        prefix.push_str(ex_in.trim());
        prefix.push_str("<end_of_turn>\n<start_of_turn>model\n");
        prefix.push_str(ex_out.trim());
        prefix.push_str("<end_of_turn>\n");
    } else {
        prefix.push_str("<end_of_turn>\n");
    }

    let mut suffix = String::new();
    suffix.push_str("<start_of_turn>user\nDYKTOWANIE:\n");
    suffix.push_str(user_input.trim());
    suffix.push_str("<end_of_turn>\n<start_of_turn>model\n");

    (prefix, suffix)
}
