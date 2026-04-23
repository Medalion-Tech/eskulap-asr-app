// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// NOTE: mimalloc global allocator removed in v0.3.14. See src-tauri/Cargo.toml
// for the full rationale — tl;dr the allocator override broke Windows LTO
// linking of llama.cpp's split build-info static lib.

fn main() {
    app_lib::run();
}
