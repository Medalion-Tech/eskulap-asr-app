use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Clone)]
pub struct WhisperEngine {
    ctx: Arc<WhisperContext>,
}

unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}

impl WhisperEngine {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;

        Ok(Self {
            ctx: Arc::new(ctx),
        })
    }

    pub fn transcribe<F>(
        &self,
        audio_pcm: &[f32],
        mut on_progress: F,
    ) -> Result<String, String>
    where
        F: FnMut(i32) + Send + 'static,
    {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| format!("Failed to create whisper state: {}", e))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("pl"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_single_segment(true);
        params.set_n_max_text_ctx(0);

        let n_threads = num_cpus::get_physical().min(12).max(1) as i32;
        params.set_n_threads(n_threads);

        let progress = Arc::new(AtomicI32::new(0));
        let progress2 = Arc::clone(&progress);
        params.set_progress_callback_safe(move |p: i32| {
            let old = progress2.load(Ordering::Relaxed);
            if p > old {
                progress2.store(p, Ordering::Relaxed);
                on_progress(p);
            }
        });

        state
            .full(params, audio_pcm)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str_lossy() {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }

        Ok(text)
    }
}
