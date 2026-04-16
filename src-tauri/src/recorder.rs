use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const TARGET_SAMPLE_RATE: u32 = 16000;
pub const LEVEL_HISTORY_SIZE: usize = 512;

pub type LevelHistory = Arc<Mutex<VecDeque<f32>>>;

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    level_history: LevelHistory,
    stream: Option<Stream>,
    device_sample_rate: u32,
}

unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl AudioRecorder {
    pub fn start(level_history: LevelHistory) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No microphone found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let device_sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();

        // Clear level history
        if let Ok(mut hist) = level_history.lock() {
            hist.clear();
        }

        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_writer = samples.clone();
        let history_writer = level_history.clone();

        // Compute chunk size for ~8ms at device sample rate → more granular than callback
        let chunk_size = ((device_sample_rate / 125) as usize).max(32);

        let stream_config: cpal::StreamConfig = config.into();

        let err_fn = |err: cpal::StreamError| {
            log::error!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mono = to_mono(data, channels);
                        push_peaks(&history_writer, &mono, chunk_size);
                        if let Ok(mut buf) = samples_writer.lock() {
                            buf.extend_from_slice(&mono);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build audio stream: {}", e))?,
            cpal::SampleFormat::I16 => {
                let samples_writer = samples.clone();
                let history_writer = level_history.clone();
                device
                    .build_input_stream(
                        &stream_config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            let floats: Vec<f32> =
                                data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                            let mono = to_mono(&floats, channels);
                            push_peaks(&history_writer, &mono, chunk_size);
                            if let Ok(mut buf) = samples_writer.lock() {
                                buf.extend_from_slice(&mono);
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build audio stream: {}", e))?
            }
            other => return Err(format!("Unsupported sample format: {:?}", other)),
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start recording: {}", e))?;

        Ok(Self {
            samples,
            level_history,
            stream: Some(stream),
            device_sample_rate,
        })
    }

    /// Stop recording and return 16kHz mono f32 PCM samples
    pub fn stop(&mut self) -> Result<Vec<f32>, String> {
        self.stream.take();
        if let Ok(mut hist) = self.level_history.lock() {
            hist.clear();
        }

        let raw_samples = self
            .samples
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?
            .clone();

        if raw_samples.is_empty() {
            return Err("No audio recorded".to_string());
        }

        if self.device_sample_rate == TARGET_SAMPLE_RATE {
            Ok(raw_samples)
        } else {
            Ok(resample(
                &raw_samples,
                self.device_sample_rate,
                TARGET_SAMPLE_RATE,
            ))
        }
    }
}

fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn push_peaks(history: &LevelHistory, samples: &[f32], chunk_size: usize) {
    let Ok(mut hist) = history.lock() else {
        return;
    };
    for chunk in samples.chunks(chunk_size) {
        let peak = chunk.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        hist.push_back(peak);
        if hist.len() > LEVEL_HISTORY_SIZE {
            hist.pop_front();
        }
    }
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let frac = (src_pos - idx as f64) as f32;
            if idx + 1 < samples.len() {
                samples[idx] * (1.0 - frac) + samples[idx + 1] * frac
            } else {
                samples[idx.min(samples.len() - 1)]
            }
        })
        .collect()
}
