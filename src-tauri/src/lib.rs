mod commands;
mod model_manager;
mod notes;
mod recorder;
mod whisper_engine;

use commands::{AudioLevelState, NotesState, RecorderState, WhisperState};
use notes::NotesStore;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tauri::Manager;

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

            app.manage(WhisperState(Mutex::new(None)));
            app.manage(RecorderState(Mutex::new(None)));
            app.manage(NotesState(Mutex::new(NotesStore::new(&data_dir))));
            app.manage(AudioLevelState(Arc::new(Mutex::new(VecDeque::with_capacity(
                recorder::LEVEL_HISTORY_SIZE,
            )))));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_model_exists,
            commands::download_model,
            commands::load_model,
            commands::start_recording,
            commands::stop_recording,
            commands::get_audio_levels,
            commands::transcribe,
            commands::get_notes,
            commands::add_note,
            commands::delete_note,
            commands::update_note,
            commands::clear_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
