use crate::ast::{self, FilledTemplate, FilledValue};
use crate::llm_engine::{build_prompt_split, LlmEngine};
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

pub const BUILD_VARIANT: &str = if cfg!(feature = "accel-cuda") {
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

    let threads = num_cpus::get_physical().min(12).max(1) as i32;

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
    #[cfg(all(target_os = "linux", feature = "accel-vulkan"))]
    {
        let devices = std::panic::catch_unwind(whisper_rs::vulkan::list_devices)
            .unwrap_or_default();
        if let Some(first) = devices.into_iter().next() {
            return ("Vulkan".to_string(), first.name);
        }
        return ("CPU".to_string(), cpu_model.to_string());
    }
    #[cfg(all(target_os = "linux", not(feature = "accel-vulkan")))]
    {
        return ("CPU".to_string(), cpu_model.to_string());
    }
    #[cfg(all(target_os = "windows", feature = "accel-cuda"))]
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
    #[cfg(all(target_os = "windows", feature = "accel-vulkan"))]
    {
        // Enumerate Vulkan devices via whisper-rs's ggml-vulkan bindings.
        // Wrapped in catch_unwind because list_devices() calls into C++ and
        // a missing vulkan-1.dll / incompatible driver would otherwise crash
        // the process instead of letting us fall back to CPU with a clean label.
        let devices = std::panic::catch_unwind(whisper_rs::vulkan::list_devices)
            .unwrap_or_default();
        if let Some(first) = devices.into_iter().next() {
            return ("Vulkan".to_string(), first.name);
        }
        return ("CPU".to_string(), cpu_model.to_string());
    }
    #[cfg(all(target_os = "windows", not(any(feature = "accel-cuda", feature = "accel-vulkan"))))]
    {
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

pub struct WhisperState(pub Arc<Mutex<Option<WhisperEngine>>>);
pub struct RecorderState(pub Mutex<Option<AudioRecorder>>);
pub struct NotesState(pub Mutex<NotesStore>);
pub struct AudioLevelState(pub LevelHistory);
pub struct LlmState(pub Arc<Mutex<Option<LlmEngine>>>);
pub struct TemplatesState(pub Mutex<TemplatesStore>);
pub struct KvCacheState(pub Mutex<crate::kv_cache::KvCacheIndex>);

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

/// Transcribe a full audio buffer with a progress callback (0–100 %).
/// The UI can display a progress ring while whisper.cpp processes the file.
#[tauri::command]
pub async fn transcribe(
    whisper: tauri::State<'_, WhisperState>,
    audio: Vec<f32>,
    on_progress: Channel<i32>,
) -> Result<String, String> {
    // Clone the engine (cheap: it's an Arc<WhisperContext> internally) so we
    // don't hold the outer Mutex across the `spawn_blocking` transcription,
    // which would serialize other whisper-touching commands (e.g. status).
    let engine = {
        let guard = whisper.0.lock().map_err(|e| e.to_string())?;
        guard.as_ref().ok_or("Model not loaded")?.clone()
    };

    let text = tokio::task::spawn_blocking(move || {
        engine.transcribe(&audio, move |p: i32| {
            let _ = on_progress.send(p);
        })
    })
    .await
    .map_err(|e| format!("Transcription task failed: {:?}", e))??;


    log::info!("Transcribed: {} chars", text.len());
    Ok(text)
}

/// Transcribe audio in 30-second chunks, streaming each segment's text via `on_segment`.
/// Returns the full assembled transcription when done.
/// For audio ≤ 30 s this is equivalent to a single `transcribe` call.
#[tauri::command]
pub async fn transcribe_streaming(
    whisper: tauri::State<'_, WhisperState>,
    audio: Vec<f32>,
    on_segment: Channel<String>,
) -> Result<String, String> {
    let engine_arc = whisper.0.clone();
    tokio::task::spawn_blocking(move || {
        let guard = engine_arc.lock().map_err(|e| e.to_string())?;
        let engine = guard.as_ref().ok_or("Model not loaded")?;
        // 30 s chunks with 2 s overlap
        engine.transcribe_chunked(&audio, 480_000, 32_000, |seg| {
            let _ = on_segment.send(seg);
        })
    })
    .await
    .map_err(|e| format!("Transcribe task failed: {}", e))?
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
    filled: Option<FilledTemplate>,
    raw_llm_output: Option<String>,
) -> Result<Note, String> {
    let mut guard = notes.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.add_with_template(
        text,
        raw_transcription,
        template_id,
        template_name,
        filled,
        raw_llm_output,
    ))
}

/// Re-parse the raw_llm_output of a note with the current parser + template
/// AST. Use this to recover notes generated before a parser fix.
#[tauri::command]
pub fn reparse_note(
    notes: tauri::State<'_, NotesState>,
    templates: tauri::State<'_, TemplatesState>,
    note_id: String,
) -> Result<Note, String> {
    let (template, raw_output) = {
        let notes_guard = notes.0.lock().map_err(|e| e.to_string())?;
        let note = notes_guard.get(&note_id).ok_or("Note not found")?;
        let template_id = note.template_id.clone().ok_or("Note has no template")?;
        let raw = note.raw_llm_output.clone().ok_or("Note has no raw_llm_output")?;
        let tpl_guard = templates.0.lock().map_err(|e| e.to_string())?;
        let tpl = tpl_guard.get(&template_id).ok_or("Template not found")?;
        (tpl, raw)
    };

    let (mut filled, _q) = ast::parse_llm_output(&template.ast, &raw_output);
    filled.template_id = template.id.clone();
    let new_text = ast::render_display(&template.ast, &filled);

    let mut notes_guard = notes.0.lock().map_err(|e| e.to_string())?;
    notes_guard
        .set_filled(&note_id, filled, new_text)
        .ok_or_else(|| "Note not found".to_string())
}

/// Atomically update a single slot's filled value and re-render Note.text.
/// Marks slot_id as user-edited.
#[tauri::command]
pub fn update_filled_value(
    notes: tauri::State<'_, NotesState>,
    templates: tauri::State<'_, TemplatesState>,
    note_id: String,
    slot_id: String,
    value: FilledValue,
) -> Result<Note, String> {
    let (ast, mut filled) = {
        let notes_guard = notes.0.lock().map_err(|e| e.to_string())?;
        let note = notes_guard.get(&note_id).ok_or("Note not found")?;
        let template_id = note.template_id.clone().ok_or("Note has no template")?;
        let filled = note.filled.clone().ok_or("Note has no filled template")?;
        let tpl_guard = templates.0.lock().map_err(|e| e.to_string())?;
        let tpl = tpl_guard.get(&template_id).ok_or("Template not found")?;
        (tpl.ast, filled)
    };

    filled.values.insert(slot_id.clone(), value);
    if !filled.user_edited.contains(&slot_id) {
        filled.user_edited.push(slot_id);
    }
    let new_text = ast::render_display(&ast, &filled);

    let mut notes_guard = notes.0.lock().map_err(|e| e.to_string())?;
    notes_guard
        .set_filled(&note_id, filled, new_text)
        .ok_or_else(|| "Note not found".to_string())
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
    kv_cache: tauri::State<'_, KvCacheState>,
    id: String,
    template: Template,
) -> Result<Template, String> {
    let result = {
        let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
        guard.update(&id, template)?
    };
    if let Ok(mut cache) = kv_cache.0.lock() {
        cache.invalidate_template(&id);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_template(
    templates: tauri::State<'_, TemplatesState>,
    kv_cache: tauri::State<'_, KvCacheState>,
    id: String,
) -> Result<bool, String> {
    let ok = {
        let mut guard = templates.0.lock().map_err(|e| e.to_string())?;
        guard.delete(&id)?
    };
    if ok {
        if let Ok(mut cache) = kv_cache.0.lock() {
            cache.invalidate_template(&id);
        }
    }
    Ok(ok)
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

#[derive(Debug, Clone, Serialize)]
pub struct GenerationResult {
    pub display_text: String,
    pub filled: FilledTemplate,
    pub raw_output: String,
    pub parse_quality_low: bool,
    pub total_slots: u32,
    pub parsed_ok: u32,
    pub cache_hit: bool,
}

#[tauri::command]
pub async fn generate_from_template(
    app: tauri::AppHandle,
    llm: tauri::State<'_, LlmState>,
    templates: tauri::State<'_, TemplatesState>,
    kv_cache: tauri::State<'_, KvCacheState>,
    template_id: String,
    raw_transcription: String,
    on_token: Channel<String>,
) -> Result<GenerationResult, String> {
    let template = {
        let guard = templates.0.lock().map_err(|e| e.to_string())?;
        guard.get(&template_id).ok_or("Template not found")?
    };

    // KV cache: look up (and reserve a slot if miss). We hold the lock only
    // briefly; generation runs with the cache state unlocked.
    let ast_hash = ast::ast_hash(&template.ast);
    let (cache_entry, cache_key, save_path) = {
        let mut cache_guard = kv_cache.0.lock().map_err(|e| e.to_string())?;
        match cache_guard.lookup(&template.id, &ast_hash) {
            Some((entry, key)) => (Some(entry), key, None),
            None => {
                let (path, key) = cache_guard.reserve(&template.id, &ast_hash);
                (None, key, Some(path))
            }
        }
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
    let template_for_task = template.clone();
    let raw = raw_transcription;
    let save_path_for_task = save_path.clone();
    let cache_entry_for_task = cache_entry.clone();

    let generate_output = tokio::task::spawn_blocking(
        move || -> Result<crate::llm_engine::GenerateOutput, String> {
            let guard = engine_arc.lock().map_err(|e| e.to_string())?;
            let engine = guard.as_ref().ok_or("LLM model not loaded")?;

            let stringified = ast::stringify_for_llm(&template_for_task.ast);
            let example_out = template_for_task
                .example_filled
                .as_ref()
                .map(|ef| ast::render_with_values(&template_for_task.ast, ef));
            let (prefix, suffix) = build_prompt_split(
                &stringified,
                template_for_task.example_input.as_deref(),
                example_out.as_deref(),
                &raw,
            );

            engine.generate(
                &prefix,
                &suffix,
                cache_entry_for_task.as_ref(),
                save_path_for_task.as_deref(),
                2000,
                &mut |piece| {
                    let _ = on_token.send(piece.to_string());
                },
            )
        },
    )
    .await
    .map_err(|e| format!("LLM task failed: {}", e))??;

    // On cache miss with successful save, register the entry.
    if let Some(saved) = &generate_output.saved_prefix {
        if let Ok(mut cache_guard) = kv_cache.0.lock() {
            if let Some(file_name) = saved
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
            {
                cache_guard.insert(
                    cache_key,
                    file_name,
                    saved.prefix_token_count,
                    template.id.clone(),
                    ast_hash.clone(),
                );
            }
        }
    }

    let raw_output = generate_output.text;

    let trimmed = raw_output.trim().to_string();
    let (mut filled, quality) = ast::parse_llm_output(&template.ast, &trimmed);
    filled.template_id = template.id.clone();
    let display_text = ast::render_display(&template.ast, &filled);

    log::info!(
        "LLM generated {} chars; slots parsed {}/{}; cache_hit={}",
        trimmed.len(),
        quality.parsed_ok,
        quality.total_slots,
        generate_output.cache_hit,
    );

    Ok(GenerationResult {
        display_text,
        filled,
        raw_output: trimmed,
        parse_quality_low: quality.low,
        total_slots: quality.total_slots,
        parsed_ok: quality.parsed_ok,
        cache_hit: generate_output.cache_hit,
    })
}
