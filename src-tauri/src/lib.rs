mod ast;
mod builtin_templates;
mod commands;
mod kv_cache;
mod llm_engine;
mod model_manager;
mod notes;
mod recorder;
mod settings;
mod templates;
mod whisper_engine;

use commands::{
    AudioLevelState, KvCacheState, LlmState, NotesState, RecorderState, TemplatesState,
    WhisperState,
};
use notes::NotesStore;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use templates::TemplatesStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let data_dir = app
                .handle()
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            app.manage(WhisperState(Arc::new(Mutex::new(None))));
            app.manage(RecorderState(Mutex::new(None)));
            app.manage(NotesState(Mutex::new(NotesStore::new(&data_dir))));
            app.manage(AudioLevelState(Arc::new(Mutex::new(
                VecDeque::with_capacity(recorder::LEVEL_HISTORY_SIZE),
            ))));
            app.manage(LlmState(Arc::new(Mutex::new(None))));
            app.manage(TemplatesState(Mutex::new(TemplatesStore::new(&data_dir))));
            app.manage(KvCacheState(Mutex::new(kv_cache::KvCacheIndex::new(
                &data_dir,
            ))));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_model_exists,
            commands::download_model,
            commands::load_model,
            commands::get_accelerator_info,
            commands::start_recording,
            commands::stop_recording,
            commands::get_audio_levels,
            commands::transcribe,
            commands::transcribe_streaming,
            commands::get_notes,
            commands::add_note,
            commands::delete_note,
            commands::update_note,
            commands::clear_notes,
            commands::add_note_with_template,
            commands::update_note_with_template,
            commands::update_filled_value,
            commands::reparse_note,
            commands::check_llm_model_exists,
            commands::get_llm_model_variants,
            commands::download_llm_model,
            commands::download_llm_model_variant,
            commands::delete_llm_model_variant,
            commands::load_llm_model,
            commands::is_llm_loaded,
            commands::unload_llm_model,
            commands::get_settings,
            commands::get_default_settings,
            commands::set_settings,
            commands::get_templates,
            commands::add_template,
            commands::update_template,
            commands::delete_template,
            commands::reset_builtin_templates,
            commands::generate_from_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
