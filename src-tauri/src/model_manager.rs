use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::Manager;

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    #[serde(default)]
    pub stage: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

pub fn models_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("models");
    std::fs::create_dir_all(&dir).ok();
    dir
}

async fn download_to(
    url: &str,
    dest: &Path,
    stage: &str,
    on_progress: &Channel<DownloadProgress>,
) -> Result<PathBuf, String> {
    if dest.exists() {
        return Ok(dest.to_path_buf());
    }

    let parent = dest.parent().ok_or("Invalid destination path")?;
    std::fs::create_dir_all(parent).ok();

    let partial = dest.with_extension("partial");

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(&partial)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded += chunk.len() as u64;
        let percent = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        on_progress
            .send(DownloadProgress {
                stage: stage.to_string(),
                downloaded,
                total,
                percent,
            })
            .ok();
    }

    tokio::fs::rename(&partial, dest)
        .await
        .map_err(|e| format!("Failed to finalize download: {}", e))?;

    Ok(dest.to_path_buf())
}

pub mod whisper {
    use super::*;

    pub const MODEL_URL: &str =
        "https://huggingface.co/lion-ai/eskulap-asr-turbo-beta/resolve/main/ggml-model-q8_0.bin";
    pub const MODEL_FILENAME: &str = "eskulap-asr-turbo-q8.bin";
    pub const STAGE: &str = "whisper";

    pub fn model_path(app: &tauri::AppHandle) -> PathBuf {
        models_dir(app).join(MODEL_FILENAME)
    }

    pub fn model_exists(app: &tauri::AppHandle) -> bool {
        model_path(app).exists()
    }

    pub async fn download(
        app: &tauri::AppHandle,
        on_progress: &Channel<DownloadProgress>,
    ) -> Result<PathBuf, String> {
        let dest = model_path(app);
        download_to(MODEL_URL, &dest, STAGE, on_progress).await
    }
}

pub mod llm {
    use super::*;

    pub const MODEL_URL: &str =
        "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf";
    pub const MODEL_FILENAME: &str = "gemma-4-e4b-q4.gguf";
    pub const STAGE: &str = "llm";

    pub fn model_path(app: &tauri::AppHandle) -> PathBuf {
        models_dir(app).join(MODEL_FILENAME)
    }

    pub fn model_exists(app: &tauri::AppHandle) -> bool {
        model_path(app).exists()
    }

    pub async fn download(
        app: &tauri::AppHandle,
        on_progress: &Channel<DownloadProgress>,
    ) -> Result<PathBuf, String> {
        let dest = model_path(app);
        download_to(MODEL_URL, &dest, STAGE, on_progress).await
    }
}

// Backward-compat shims for existing callers
pub fn model_path(app: &tauri::AppHandle) -> PathBuf {
    whisper::model_path(app)
}

pub fn model_exists(app: &tauri::AppHandle) -> bool {
    whisper::model_exists(app)
}

pub async fn download_model(
    app: &tauri::AppHandle,
    on_progress: &Channel<DownloadProgress>,
) -> Result<PathBuf, String> {
    whisper::download(app, on_progress).await
}
