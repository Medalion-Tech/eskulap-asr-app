use crate::llm_engine::{build_prompt, LlmEngine};
use crate::model_manager::{self, DownloadProgress};
use crate::notes::{Note, NotesStore};
use crate::recorder::{AudioRecorder, LevelHistory};
use crate::settings::{self, AppSettings};
use crate::templates::{Template, TemplatesStore};
use crate::whisper_engine::WhisperEngine;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::Manager;

#[derive(Serialize)]
pub struct AcceleratorInfo {
    pub backend: String,
    pub device: String,
    pub platform: String,
    pub arch: String,
    pub threads: i32,
    pub cpu_model: String,
    pub build_variant: String,
}

const BUILD_VARIANT: &str = if cfg!(feature = "accel-cuda") {
    "cuda"
} else if cfg!(feature = "accel-metal") {
    "metal"
} else if cfg!(feature = "accel-vulkan") {
    "vulkan"
} else {
    "cpu"
};

#[tauri::command]
pub fn get_accelerator_info() -> AcceleratorInfo {
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
    let (backend, device) = detect_backend(&cpu_model);

    AcceleratorInfo {
        backend,
        device,
        platform,
        arch,
        threads,
        cpu_model,
        build_variant: BUILD_VARIANT.to_string(),
    }
}

fn detect_backend(cpu_model: &str) -> (String, String) {
    #[cfg(target_os = "macos")]
    {
        let device = if cpu_model.is_empty() {
            "Apple GPU".to_string()
        } else {
            cpu_model.to_string()
        };
        return ("Metal".to_string(), device);
    }
    #[cfg(target_os = "linux")]
    {
        let devices = std::panic::catch_unwind(whisper_rs::vulkan::list_devices)
            .unwrap_or_default();
        if let Some(first) = devices.into_iter().next() {
            return ("Vulkan".to_string(), first.name);
        }
        return ("CPU".to_string(), cpu_model.to_string());
    }
    #[cfg(target_os = "windows")]
    {
        // Try nvidia-smi to detect CUDA-capable GPU
        if let Ok(out) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
        {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() {
                    let first = name.lines().next().unwrap_or("").trim().to_string();
                    if !first.is_empty() {
                        return ("CUDA".to_string(), first);
                    }
                }
            }
        }
        return ("CPU".to_string(), cpu_model.to_string());
    }
    #[allow(unreachable_code)]
    ("CPU".to_string(), cpu_model.to_string())
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
pub struct LlmState(pub Arc<Mutex<Option<LlmEngine>>>);
pub struct TemplatesState(pub Mutex<TemplatesStore>);

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

#[tauri::command]
pub fn add_note_with_template(
    notes: tauri::State<'_, NotesState>,
    text: String,
    raw_transcription: String,
    template_id: String,
    template_name: String,
) -> Result<Note, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.add_with_template(text, raw_transcription, template_id, template_name))
}

#[tauri::command]
pub fn update_note_with_template(
    notes: tauri::State<'_, NotesState>,
    id: String,
    text: String,
    template_id: String,
    template_name: String,
) -> Result<Option<Note>, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.update_with_template(&id, text, template_id, template_name))
}

// ---- LLM model management ----

#[tauri::command]
pub fn check_llm_model_exists(app: tauri::AppHandle) -> bool {
    model_manager::llm::model_exists(&app)
}

#[tauri::command]
pub async fn download_llm_model(
    app: tauri::AppHandle,
    on_progress: Channel<DownloadProgress>,
) -> Result<(), String> {
    model_manager::llm::download(&app, &on_progress).await?;
    Ok(())
}

#[tauri::command]
pub fn load_llm_model(
    app: tauri::AppHandle,
    llm: tauri::State<'_, LlmState>,
) -> Result<(), String> {
    let path = model_manager::llm::model_path(&app);
    if !path.exists() {
        return Err("LLM model not downloaded yet".to_string());
    }
    let engine = LlmEngine::new(&path)?;
    let mut guard = llm.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(engine);
    log::info!("LLM model loaded successfully");
    Ok(())
}

#[tauri::command]
pub fn is_llm_loaded(llm: tauri::State<'_, LlmState>) -> bool {
    llm.0.lock().map(|g| g.is_some()).unwrap_or(false)
}

#[tauri::command]
pub fn unload_llm_model(llm: tauri::State<'_, LlmState>) -> Result<(), String> {
    let mut guard = llm.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    log::info!("LLM model unloaded");
    Ok(())
}

// ---- Settings ----

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    Ok(settings::load(&data_dir(&app)?))
}

#[tauri::command]
pub fn set_settings(app: tauri::AppHandle, new_settings: AppSettings) -> Result<(), String> {
    settings::save(&data_dir(&app)?, &new_settings)
}

// ---- Templates ----

#[tauri::command]
pub fn get_templates(
    templates: tauri::State<'_, TemplatesState>,
) -> Result<Vec<Template>, String> {
    let guard = templates.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.get_all())
}

#[tauri::command]
pub fn add_template(
    templates: tauri::State<'_, TemplatesState>,
    template: Template,
) -> Result<Template, String> {
    let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.add(template))
}

#[tauri::command]
pub fn update_template(
    templates: tauri::State<'_, TemplatesState>,
    id: String,
    template: Template,
) -> Result<Template, String> {
    let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
    guard.update(&id, template)
}

#[tauri::command]
pub fn delete_template(
    templates: tauri::State<'_, TemplatesState>,
    id: String,
) -> Result<bool, String> {
    let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
    guard.delete(&id)
}

#[tauri::command]
pub fn reset_builtin_templates(
    templates: tauri::State<'_, TemplatesState>,
) -> Result<(), String> {
    let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
    guard.reset_builtins();
    Ok(())
}

// ---- Template-based generation ----

#[tauri::command]
pub async fn generate_from_template(
    app: tauri::AppHandle,
    llm: tauri::State<'_, LlmState>,
    templates: tauri::State<'_, TemplatesState>,
    template_id: String,
    raw_transcription: String,
    on_token: Channel<String>,
) -> Result<String, String> {
    let template = {
        let guard = templates.0.lock().map_err(|e| e.to_string())?;
        guard.get(&template_id).ok_or("Template not found")?
    };

    // Lazy-load: if LLM not yet loaded, load it now from disk.
    {
        let needs_load = llm.0.lock().map(|g| g.is_none()).unwrap_or(false);
        if needs_load {
            let path = model_manager::llm::model_path(&app);
            if !path.exists() {
                return Err("LLM model not downloaded".to_string());
            }
            let engine_arc = llm.0.clone();
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let engine = LlmEngine::new(&path)?;
                let mut guard = engine_arc.lock().map_err(|e| e.to_string())?;
                *guard = Some(engine);
                log::info!("LLM model lazy-loaded");
                Ok(())
            })
            .await
            .map_err(|e| format!("LLM load task failed: {}", e))??;
        }
    }

    let engine_arc = llm.0.clone();

    let raw = raw_transcription;
    let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let guard = engine_arc.lock().map_err(|e| e.to_string())?;
        let engine = guard.as_ref().ok_or("LLM model not loaded")?;

        let prompt = build_prompt(
            &template.content,
            template.example_input.as_deref(),
            template.example_output.as_deref(),
            &raw,
        );

        engine.generate(&prompt, 1500, &mut |piece| {
            let _ = on_token.send(piece.to_string());
        })
    })
    .await
    .map_err(|e| format!("LLM task failed: {}", e))??;

    log::info!("LLM generated {} chars", result.len());
    Ok(result.trim().to_string())
}
