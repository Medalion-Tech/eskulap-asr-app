use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    pub llm_enabled: bool,
    pub asr: AsrSettings,
    pub llm: LlmSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AsrSettings {
    pub language: String,
    pub translate: bool,
    pub threads: i32,
    pub strategy: AsrStrategy,
    pub greedy_best_of: i32,
    pub beam_size: i32,
    pub temperature: f32,
    pub single_segment: bool,
    pub max_text_context: i32,
    pub initial_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AsrStrategy {
    Greedy,
    BeamSearch,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct LlmSettings {
    pub model_id: String,
    pub context_size: u32,
    pub batch_size: u32,
    pub threads: i32,
    pub batch_threads: i32,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub seed: u32,
    pub flash_attention: bool,
    pub kv_cache_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            llm_enabled: true,
            asr: AsrSettings::default(),
            llm: LlmSettings::default(),
        }
    }
}

impl Default for AsrSettings {
    fn default() -> Self {
        Self {
            language: "pl".to_string(),
            translate: false,
            threads: default_threads(),
            strategy: AsrStrategy::Greedy,
            greedy_best_of: 1,
            beam_size: 5,
            temperature: 0.0,
            single_segment: true,
            max_text_context: 0,
            initial_prompt: String::new(),
        }
    }
}

impl Default for AsrStrategy {
    fn default() -> Self {
        Self::Greedy
    }
}

impl Default for LlmSettings {
    fn default() -> Self {
        let threads = default_threads();
        Self {
            model_id: "Q4_K_M".to_string(),
            context_size: 8192,
            batch_size: 1024,
            threads,
            batch_threads: threads,
            max_tokens: 2000,
            temperature: 0.4,
            top_p: 0.9,
            seed: 1234,
            flash_attention: true,
            kv_cache_enabled: true,
        }
    }
}

pub fn default_threads() -> i32 {
    num_cpus::get_physical().min(12).max(1) as i32
}

pub fn validate(settings: &AppSettings) -> Result<(), String> {
    validate_language(&settings.asr.language)?;
    let max_threads = num_cpus::get_physical().max(1) as i32;
    validate_range_i32("ASR: liczba wątków", settings.asr.threads, 1, max_threads)?;
    validate_range_i32("ASR: best_of", settings.asr.greedy_best_of, 1, 10)?;
    validate_range_i32("ASR: beam_size", settings.asr.beam_size, 1, 10)?;
    validate_range_f32("ASR: temperatura", settings.asr.temperature, 0.0, 1.0)?;
    validate_range_i32(
        "ASR: maksymalny kontekst tekstowy",
        settings.asr.max_text_context,
        0,
        4096,
    )?;

    validate_range_i32("LLM: liczba wątków", settings.llm.threads, 1, max_threads)?;
    crate::model_manager::llm::variant(&settings.llm.model_id)
        .map(|_| ())
        .ok_or_else(|| format!("Nieobsługiwany model LLM: {}", settings.llm.model_id))?;
    validate_range_i32(
        "LLM: liczba wątków batch",
        settings.llm.batch_threads,
        1,
        max_threads,
    )?;
    validate_range_u32(
        "LLM: rozmiar kontekstu",
        settings.llm.context_size,
        1024,
        32768,
    )?;
    validate_range_u32("LLM: rozmiar batcha", settings.llm.batch_size, 32, 4096)?;
    validate_range_u32(
        "LLM: maksymalna liczba tokenów",
        settings.llm.max_tokens,
        128,
        4096,
    )?;
    validate_range_f32("LLM: temperatura", settings.llm.temperature, 0.0, 2.0)?;
    validate_range_f32("LLM: top_p", settings.llm.top_p, 0.05, 1.0)?;

    Ok(())
}

fn validate_language(language: &str) -> Result<(), String> {
    match language {
        "pl" | "auto" | "en" | "de" | "uk" => Ok(()),
        other => Err(format!("Nieobsługiwany język ASR: {}", other)),
    }
}

fn validate_range_i32(name: &str, value: i32, min: i32, max: i32) -> Result<(), String> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(format!("{} musi być w zakresie {}..={}", name, min, max))
    }
}

fn validate_range_u32(name: &str, value: u32, min: u32, max: u32) -> Result<(), String> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(format!("{} musi być w zakresie {}..={}", name, min, max))
    }
}

fn validate_range_f32(name: &str, value: f32, min: f32, max: f32) -> Result<(), String> {
    if value.is_finite() && value >= min && value <= max {
        Ok(())
    } else {
        Err(format!("{} musi być w zakresie {}..={}", name, min, max))
    }
}

fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub fn load(data_dir: &Path) -> AppSettings {
    let path = settings_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save(data_dir: &Path, settings: &AppSettings) -> Result<(), String> {
    validate(settings)?;
    let path = settings_path(data_dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_settings_deserialize_with_defaults() {
        let settings: AppSettings = serde_json::from_str(r#"{ "llm_enabled": false }"#).unwrap();
        assert!(!settings.llm_enabled);
        assert_eq!(settings.asr.language, "pl");
        assert_eq!(settings.asr.strategy, AsrStrategy::Greedy);
        assert_eq!(settings.llm.context_size, 8192);
        assert_eq!(settings.llm.model_id, "Q4_K_M");
    }

    #[test]
    fn defaults_match_current_behavior() {
        let settings = AppSettings::default();
        assert!(settings.llm_enabled);
        assert_eq!(settings.asr.language, "pl");
        assert!(!settings.asr.translate);
        assert_eq!(settings.asr.greedy_best_of, 1);
        assert_eq!(settings.llm.max_tokens, 2000);
        assert_eq!(settings.llm.seed, 1234);
    }

    #[test]
    fn validation_rejects_out_of_range_values() {
        let mut settings = AppSettings::default();
        settings.llm.top_p = 0.0;
        assert!(validate(&settings).is_err());

        settings = AppSettings::default();
        settings.asr.greedy_best_of = 11;
        assert!(validate(&settings).is_err());
    }

    #[test]
    fn language_validation_accepts_auto_and_pl() {
        let mut settings = AppSettings::default();
        settings.asr.language = "auto".to_string();
        assert!(validate(&settings).is_ok());
        settings.asr.language = "pl".to_string();
        assert!(validate(&settings).is_ok());
    }
}
