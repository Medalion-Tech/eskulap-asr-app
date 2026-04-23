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

/// Returns the number of threads to use for transcription.
/// Uses all available parallelism without an artificial cap —
/// the old `.min(8)` was leaving ~50% of CPU idle on 12/16-core machines.
pub fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
}

impl WhisperEngine {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            params,
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;

        log::info!(
            "WhisperEngine: loaded model, threads={}, accel_cpu={}, variant={}",
            default_threads(),
            cfg!(feature = "accel-cpu"),
            crate::commands::BUILD_VARIANT,
        );

        Ok(Self {
            ctx: Arc::new(ctx),
        })
    }

    /// Transcribe audio PCM (16 kHz, mono, f32) with an optional progress callback (0–100).
    /// Pass `|_| {}` as `on_progress` if you don't need progress updates.
    /// The progress callback is automatically deduped — only monotonically
    /// increasing values reach the user (prevents UI jitter from whisper.cpp's
    /// occasional repeat emissions of the same percent).
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

        let n_threads = default_threads();
        params.set_n_threads(n_threads);

        // ── Perf / quality tuning ─────────────────────────────────────────────
        // Don't carry encoder context from previous calls (each call is independent).
        params.set_no_context(true);
        // Suppress blank/silence tokens to avoid stray spaces.
        params.set_suppress_blank(true);
        // Temperature = 0 → greedy, deterministic, and disables the temperature-
        // fallback retry loop that can multiply wall-time on "hard" audio.
        // Risk: no fallback on garbled audio; acceptable for clean medical dictation.
        params.set_temperature(0.0);
        // Turbo defaults n_max_text_ctx to 16 384; short dictations need ≤ 64.
        params.set_n_max_text_ctx(64);
        // ─────────────────────────────────────────────────────────────────────

        // Dedupe progress emissions — whisper.cpp sometimes repeats the same
        // percent or emits them out of order; we only forward monotonically
        // increasing values to avoid UI jitter.
        let last = Arc::new(AtomicI32::new(-1));
        let last2 = Arc::clone(&last);
        params.set_progress_callback_safe(move |p: i32| {
            let old = last2.load(Ordering::Relaxed);
            if p > old {
                last2.store(p, Ordering::Relaxed);
                on_progress(p);
            }
        });

        let audio_seconds = audio_pcm.len() as f64 / 16_000.0;
        let t0 = std::time::Instant::now();

        state
            .full(params, audio_pcm)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        let elapsed = t0.elapsed();
        let ratio = audio_seconds / elapsed.as_secs_f64();
        log::info!(
            "whisper: audio={:.2}s wall={:.2}s ratio={:.2}x threads={} segments={}",
            audio_seconds,
            elapsed.as_secs_f64(),
            ratio,
            n_threads,
            state.full_n_segments(),
        );

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

    /// Transcribe long audio in `chunk_samples`-sized windows with `overlap_samples` overlap.
    ///
    /// Typical values: chunk = 480 000 (30 s @ 16 kHz), overlap = 32 000 (2 s).
    ///
    /// `on_segment` is called with each completed chunk's text as it becomes available,
    /// enabling streaming display in the UI.
    pub fn transcribe_chunked(
        &self,
        audio_pcm: &[f32],
        chunk_samples: usize,
        overlap_samples: usize,
        on_segment: impl Fn(String),
    ) -> Result<String, String> {
        let mut full_text = String::new();
        let mut start = 0usize;

        loop {
            let end = (start + chunk_samples).min(audio_pcm.len());
            let chunk = &audio_pcm[start..end];

            let segment_text = self.transcribe(chunk, |_| {})?;
            let trimmed = segment_text.trim().to_string();

            if !trimmed.is_empty() {
                if !full_text.is_empty() {
                    full_text.push(' ');
                }
                full_text.push_str(&trimmed);
                on_segment(trimmed);
            }

            if end >= audio_pcm.len() {
                break;
            }
            start = end.saturating_sub(overlap_samples);
        }

        Ok(full_text)
    }
}
