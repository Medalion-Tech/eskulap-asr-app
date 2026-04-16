use futures_util::StreamExt;
use serde::Serialize;
use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::Manager;

const MODEL_URL: &str =
    "https://huggingface.co/lion-ai/eskulap-asr-turbo-beta/resolve/main/ggml-model-q8_0.bin";
const MODEL_FILENAME: &str = "eskulap-asr-turbo-q8.bin";

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

pub fn model_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("models");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn model_path(app: &tauri::AppHandle) -> PathBuf {
    model_dir(app).join(MODEL_FILENAME)
}

pub fn model_exists(app: &tauri::AppHandle) -> bool {
    model_path(app).exists()
}

pub async fn download_model(
    app: &tauri::AppHandle,
    on_progress: &Channel<DownloadProgress>,
) -> Result<PathBuf, String> {
    let path = model_path(app);

    if path.exists() {
        return Ok(path);
    }

    let partial_path = path.with_extension("bin.partial");

    let client = reqwest::Client::new();
    let resp = client
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(&partial_path)
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
                downloaded,
                total,
                percent,
            })
            .ok();
    }

    tokio::fs::rename(&partial_path, &path)
        .await
        .map_err(|e| format!("Failed to finalize download: {}", e))?;

    Ok(path)
}
