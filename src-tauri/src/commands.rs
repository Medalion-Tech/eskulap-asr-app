use crate::model_manager::{self, DownloadProgress};
use crate::notes::{Note, NotesStore};
use crate::recorder::{AudioRecorder, LevelHistory};
use crate::whisper_engine::WhisperEngine;
use serde::Serialize;
use std::sync::Mutex;
use tauri::ipc::Channel;

#[derive(Serialize)]
pub struct AcceleratorInfo {
    pub backend: String,
    pub platform: String,
    pub arch: String,
    pub threads: i32,
    pub cpu_model: String,
}

#[tauri::command]
pub fn get_accelerator_info() -> AcceleratorInfo {
    let backend = if cfg!(target_os = "macos") {
        "Metal".to_string()
    } else {
        "CPU".to_string()
    };

    let platform = if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else {
        "unknown".to_string()
    };

    let arch = if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else {
        "unknown".to_string()
    };

    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
        .min(8);

    let cpu_model = detect_cpu_model();

    AcceleratorInfo {
        backend,
        platform,
        arch,
        threads,
        cpu_model,
    }
}

fn detect_cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("model name") {
                    if let Some((_, v)) = rest.split_once(':') {
                        return v.trim().to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = std::process::Command::new("wmic")
            .args(["cpu", "get", "name"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines().skip(1) {
                let t = line.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
    }
    String::new()
}

pub struct WhisperState(pub Mutex<Option<WhisperEngine>>);
pub struct RecorderState(pub Mutex<Option<AudioRecorder>>);
pub struct NotesState(pub Mutex<NotesStore>);
pub struct AudioLevelState(pub LevelHistory);

#[tauri::command]
pub fn check_model_exists(app: tauri::AppHandle) -> bool {
    model_manager::model_exists(&app)
}

#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    on_progress: Channel<DownloadProgress>,
) -> Result<(), String> {
    model_manager::download_model(&app, &on_progress).await?;
    Ok(())
}

#[tauri::command]
pub fn load_model(
    app: tauri::AppHandle,
    whisper: tauri::State<'_, WhisperState>,
) -> Result<(), String> {
    let path = model_manager::model_path(&app);
    if !path.exists() {
        return Err("Model not downloaded yet".to_string());
    }
    let engine = WhisperEngine::new(&path)?;
    let mut guard = whisper.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(engine);
    log::info!("Whisper model loaded successfully");
    Ok(())
}

#[tauri::command]
pub fn start_recording(
    recorder: tauri::State<'_, RecorderState>,
    audio_level: tauri::State<'_, AudioLevelState>,
) -> Result<(), String> {
    let mut guard = recorder.0.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("Already recording".to_string());
    }
    let rec = AudioRecorder::start(audio_level.0.clone())?;
    *guard = Some(rec);
    log::info!("Recording started");
    Ok(())
}

#[tauri::command]
pub fn stop_recording(recorder: tauri::State<'_, RecorderState>) -> Result<Vec<f32>, String> {
    let mut guard = recorder.0.lock().map_err(|e| e.to_string())?;
    let mut rec = guard.take().ok_or("Not recording")?;
    let samples = rec.stop()?;
    log::info!("Recording stopped, {} samples at 16kHz", samples.len());
    Ok(samples)
}

#[tauri::command]
pub fn get_audio_levels(
    audio_level: tauri::State<'_, AudioLevelState>,
    count: usize,
) -> Vec<f32> {
    let hist = match audio_level.0.lock() {
        Ok(h) => h,
        Err(_) => return vec![],
    };
    let len = hist.len();
    if len == 0 {
        return vec![];
    }
    let start = len.saturating_sub(count);
    let tail: Vec<f32> = hist.iter().skip(start).copied().collect();
    // Pad front with zeros if history shorter than requested
    if tail.len() < count {
        let mut padded = vec![0.0; count - tail.len()];
        padded.extend(tail);
        padded
    } else {
        tail
    }
}

#[tauri::command]
pub fn transcribe(
    whisper: tauri::State<'_, WhisperState>,
    audio: Vec<f32>,
) -> Result<String, String> {
    let guard = whisper.0.lock().map_err(|e| e.to_string())?;
    let engine = guard.as_ref().ok_or("Model not loaded")?;
    let text = engine.transcribe(&audio)?;
    log::info!("Transcribed: {} chars", text.len());
    Ok(text)
}

#[tauri::command]
pub fn get_notes(notes: tauri::State<'_, NotesState>) -> Result<Vec<Note>, String> {
    let guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.get_all())
}

#[tauri::command]
pub fn add_note(notes: tauri::State<'_, NotesState>, text: String) -> Result<Note, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.add(text))
}

#[tauri::command]
pub fn delete_note(notes: tauri::State<'_, NotesState>, id: String) -> Result<bool, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.delete(&id))
}

#[tauri::command]
pub fn update_note(
    notes: tauri::State<'_, NotesState>,
    id: String,
    text: String,
) -> Result<Option<Note>, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.update(&id, text))
}

#[tauri::command]
pub fn clear_notes(notes: tauri::State<'_, NotesState>) -> Result<(), String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    guard.clear();
    Ok(())
}
