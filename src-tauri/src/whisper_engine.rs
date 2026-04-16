use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: WhisperContext,
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

        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio_pcm: &[f32]) -> Result<String, String> {
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

        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        params.set_n_threads(n_threads);

        state
            .full(params, audio_pcm)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| format!("Failed to get segments: {}", e))?;

        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(segment.trim());
            }
        }

        Ok(text)
    }
}
