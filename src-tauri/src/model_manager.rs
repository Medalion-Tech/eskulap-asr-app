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

    pub const MODEL_REPO: &str = "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main";
    pub const STAGE: &str = "llm";
    pub const DEFAULT_MODEL_ID: &str = "Q4_K_M";
    const LEGACY_Q4_K_M_FILENAME: &str = "gemma-4-e4b-q4.gguf";

    #[derive(Clone, Serialize)]
    pub struct LlmModelVariant {
        pub id: String,
        pub label: String,
        pub filename: String,
        pub size_bytes: u64,
        pub downloaded: bool,
        pub active: bool,
    }

    #[derive(Clone, Copy)]
    pub struct LlmModelVariantDef {
        pub id: &'static str,
        pub filename: &'static str,
        pub size_bytes: u64,
    }

    pub const VARIANTS: &[LlmModelVariantDef] = &[
        LlmModelVariantDef {
            id: "UD-IQ2_M",
            filename: "gemma-4-E4B-it-UD-IQ2_M.gguf",
            size_bytes: 3_530_232_512,
        },
        LlmModelVariantDef {
            id: "UD-IQ3_XXS",
            filename: "gemma-4-E4B-it-UD-IQ3_XXS.gguf",
            size_bytes: 3_702_264_512,
        },
        LlmModelVariantDef {
            id: "UD-Q2_K_XL",
            filename: "gemma-4-E4B-it-UD-Q2_K_XL.gguf",
            size_bytes: 3_742_794_432,
        },
        LlmModelVariantDef {
            id: "Q3_K_S",
            filename: "gemma-4-E4B-it-Q3_K_S.gguf",
            size_bytes: 3_862_377_152,
        },
        LlmModelVariantDef {
            id: "Q3_K_M",
            filename: "gemma-4-E4B-it-Q3_K_M.gguf",
            size_bytes: 4_058_135_232,
        },
        LlmModelVariantDef {
            id: "UD-Q3_K_XL",
            filename: "gemma-4-E4B-it-UD-Q3_K_XL.gguf",
            size_bytes: 4_563_503_808,
        },
        LlmModelVariantDef {
            id: "IQ4_XS",
            filename: "gemma-4-E4B-it-IQ4_XS.gguf",
            size_bytes: 4_715_414_208,
        },
        LlmModelVariantDef {
            id: "IQ4_NL",
            filename: "gemma-4-E4B-it-IQ4_NL.gguf",
            size_bytes: 4_835_836_608,
        },
        LlmModelVariantDef {
            id: "Q4_0",
            filename: "gemma-4-E4B-it-Q4_0.gguf",
            size_bytes: 4_836_000_448,
        },
        LlmModelVariantDef {
            id: "Q4_K_S",
            filename: "gemma-4-E4B-it-Q4_K_S.gguf",
            size_bytes: 4_844_847_808,
        },
        LlmModelVariantDef {
            id: "Q4_K_M",
            filename: "gemma-4-E4B-it-Q4_K_M.gguf",
            size_bytes: 4_977_169_088,
        },
        LlmModelVariantDef {
            id: "Q4_1",
            filename: "gemma-4-E4B-it-Q4_1.gguf",
            size_bytes: 5_074_387_648,
        },
        LlmModelVariantDef {
            id: "UD-Q4_K_XL",
            filename: "gemma-4-E4B-it-UD-Q4_K_XL.gguf",
            size_bytes: 5_101_718_208,
        },
        LlmModelVariantDef {
            id: "Q5_K_S",
            filename: "gemma-4-E4B-it-Q5_K_S.gguf",
            size_bytes: 5_404_852_928,
        },
        LlmModelVariantDef {
            id: "Q5_K_M",
            filename: "gemma-4-E4B-it-Q5_K_M.gguf",
            size_bytes: 5_481_796_288,
        },
        LlmModelVariantDef {
            id: "UD-Q5_K_XL",
            filename: "gemma-4-E4B-it-UD-Q5_K_XL.gguf",
            size_bytes: 6_647_847_616,
        },
        LlmModelVariantDef {
            id: "Q6_K",
            filename: "gemma-4-E4B-it-Q6_K.gguf",
            size_bytes: 7_074_927_296,
        },
        LlmModelVariantDef {
            id: "UD-Q6_K_XL",
            filename: "gemma-4-E4B-it-UD-Q6_K_XL.gguf",
            size_bytes: 7_457_759_936,
        },
        LlmModelVariantDef {
            id: "Q8_0",
            filename: "gemma-4-E4B-it-Q8_0.gguf",
            size_bytes: 8_192_950_976,
        },
        LlmModelVariantDef {
            id: "UD-Q8_K_XL",
            filename: "gemma-4-E4B-it-UD-Q8_K_XL.gguf",
            size_bytes: 8_658_666_176,
        },
        LlmModelVariantDef {
            id: "BF16",
            filename: "gemma-4-E4B-it-BF16.gguf",
            size_bytes: 15_053_095_360,
        },
    ];

    pub fn variant(id: &str) -> Option<LlmModelVariantDef> {
        VARIANTS.iter().copied().find(|v| v.id == id)
    }

    fn url_for(def: LlmModelVariantDef) -> String {
        format!("{}/{}", MODEL_REPO, def.filename)
    }

    fn canonical_path(app: &tauri::AppHandle, def: LlmModelVariantDef) -> PathBuf {
        models_dir(app).join(def.filename)
    }

    fn legacy_q4_path(app: &tauri::AppHandle) -> PathBuf {
        models_dir(app).join(LEGACY_Q4_K_M_FILENAME)
    }

    pub fn model_path(app: &tauri::AppHandle) -> PathBuf {
        let settings = app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| crate::settings::load(&dir))
            .unwrap_or_default();
        model_path_for(app, &settings.llm.model_id).unwrap_or_else(|| {
            model_path_for(app, DEFAULT_MODEL_ID).expect("default LLM model variant exists")
        })
    }

    pub fn model_path_for(app: &tauri::AppHandle, id: &str) -> Option<PathBuf> {
        let def = variant(id)?;
        if def.id == DEFAULT_MODEL_ID {
            let legacy = legacy_q4_path(app);
            if legacy.exists() {
                return Some(legacy);
            }
        }
        Some(canonical_path(app, def))
    }

    pub fn model_exists(app: &tauri::AppHandle) -> bool {
        model_path(app).exists()
    }

    pub fn model_exists_for(app: &tauri::AppHandle, id: &str) -> bool {
        model_path_for(app, id).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn list(app: &tauri::AppHandle, active_id: &str) -> Vec<LlmModelVariant> {
        VARIANTS
            .iter()
            .map(|def| LlmModelVariant {
                id: def.id.to_string(),
                label: def.id.to_string(),
                filename: def.filename.to_string(),
                size_bytes: def.size_bytes,
                downloaded: model_exists_for(app, def.id),
                active: def.id == active_id,
            })
            .collect()
    }

    pub async fn download(
        app: &tauri::AppHandle,
        on_progress: &Channel<DownloadProgress>,
    ) -> Result<PathBuf, String> {
        let settings = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())
            .map(|dir| crate::settings::load(&dir))?;
        download_variant(app, &settings.llm.model_id, on_progress).await
    }

    pub async fn download_variant(
        app: &tauri::AppHandle,
        id: &str,
        on_progress: &Channel<DownloadProgress>,
    ) -> Result<PathBuf, String> {
        let def = variant(id).ok_or_else(|| format!("Unknown LLM model variant: {}", id))?;
        let dest = canonical_path(app, def);
        download_to(&url_for(def), &dest, STAGE, on_progress).await
    }

    pub fn delete_variant(app: &tauri::AppHandle, id: &str) -> Result<bool, String> {
        let def = variant(id).ok_or_else(|| format!("Unknown LLM model variant: {}", id))?;
        let mut deleted = false;
        let path = canonical_path(app, def);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            deleted = true;
        }
        if def.id == DEFAULT_MODEL_ID {
            let legacy = legacy_q4_path(app);
            if legacy.exists() {
                std::fs::remove_file(&legacy).map_err(|e| e.to_string())?;
                deleted = true;
            }
        }
        Ok(deleted)
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
