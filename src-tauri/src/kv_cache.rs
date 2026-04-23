use crate::llm_engine::{CachedPrefix, PREAMBLE_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvCacheEntry {
    pub key: String,
    pub file_name: String,
    pub prefix_token_count: u32,
    pub preamble_version: u32,
    pub template_id: String,
    pub template_ast_hash: String,
    pub created_at: String,
    pub last_used_at: String,
    pub bytes: u64,
}

pub struct KvCacheIndex {
    dir: PathBuf,
    index_path: PathBuf,
    entries: HashMap<String, KvCacheEntry>,
}

impl KvCacheIndex {
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.join("kv_cache");
        std::fs::create_dir_all(&dir).ok();
        let index_path = dir.join("index.json");
        let entries: HashMap<String, KvCacheEntry> = std::fs::read_to_string(&index_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<KvCacheEntry>>(&s).ok())
            .map(|v| v.into_iter().map(|e| (e.key.clone(), e)).collect())
            .unwrap_or_default();

        let mut me = Self {
            dir,
            index_path,
            entries,
        };
        me.gc_preamble_mismatch();
        me
    }

    /// Compute the cache key for this template prefix.
    pub fn key(template_id: &str, ast_hash: &str, preamble_version: u32) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(template_id.as_bytes());
        h.update([0]);
        h.update(ast_hash.as_bytes());
        h.update([0]);
        h.update(preamble_version.to_le_bytes());
        hex::encode(h.finalize())[..16].to_string()
    }

    pub fn lookup(
        &mut self,
        template_id: &str,
        ast_hash: &str,
    ) -> Option<(CachedPrefix, String)> {
        let key = Self::key(template_id, ast_hash, PREAMBLE_VERSION);
        let entry = self.entries.get(&key)?.clone();
        let path = self.dir.join(&entry.file_name);
        if !path.exists() {
            // Stale index entry — drop it.
            self.entries.remove(&key);
            self.save();
            return None;
        }
        // Update last_used_at.
        if let Some(e) = self.entries.get_mut(&key) {
            e.last_used_at = crate::notes::chrono_now();
        }
        self.save();
        Some((
            CachedPrefix {
                path,
                prefix_token_count: entry.prefix_token_count,
            },
            key,
        ))
    }

    /// Reserve a path for a fresh cache entry (the file is written by llama-cpp
    /// via LlmEngine::generate, not by us). Returns (path_to_write, key).
    pub fn reserve(&self, template_id: &str, ast_hash: &str) -> (PathBuf, String) {
        let key = Self::key(template_id, ast_hash, PREAMBLE_VERSION);
        let file_name = format!("{}.bin", key);
        (self.dir.join(&file_name), key)
    }

    /// Register a successfully-written cache file in the index.
    pub fn insert(
        &mut self,
        key: String,
        file_name: String,
        prefix_token_count: u32,
        template_id: String,
        template_ast_hash: String,
    ) {
        let path = self.dir.join(&file_name);
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let now = crate::notes::chrono_now();
        let entry = KvCacheEntry {
            key: key.clone(),
            file_name,
            prefix_token_count,
            preamble_version: PREAMBLE_VERSION,
            template_id,
            template_ast_hash,
            created_at: now.clone(),
            last_used_at: now,
            bytes,
        };
        self.entries.insert(key, entry);
        self.enforce_lru_cap();
        self.save();
    }

    /// Drop all cache entries tied to a specific template (called on template edit/delete).
    pub fn invalidate_template(&mut self, template_id: &str) {
        let to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.template_id == template_id)
            .map(|(k, _)| k.clone())
            .collect();
        for key in to_remove {
            if let Some(e) = self.entries.remove(&key) {
                let _ = std::fs::remove_file(self.dir.join(&e.file_name));
            }
        }
        self.save();
    }

    fn gc_preamble_mismatch(&mut self) {
        let to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.preamble_version != PREAMBLE_VERSION)
            .map(|(k, _)| k.clone())
            .collect();
        if !to_remove.is_empty() {
            for key in to_remove {
                if let Some(e) = self.entries.remove(&key) {
                    let _ = std::fs::remove_file(self.dir.join(&e.file_name));
                }
            }
            self.save();
        }
    }

    fn enforce_lru_cap(&mut self) {
        if self.entries.len() <= MAX_ENTRIES {
            return;
        }
        let mut by_age: Vec<_> = self.entries.values().cloned().collect();
        by_age.sort_by(|a, b| a.last_used_at.cmp(&b.last_used_at));
        while self.entries.len() > MAX_ENTRIES {
            if let Some(oldest) = by_age.first().cloned() {
                if let Some(e) = self.entries.remove(&oldest.key) {
                    let _ = std::fs::remove_file(self.dir.join(&e.file_name));
                }
                by_age.remove(0);
            } else {
                break;
            }
        }
    }

    fn save(&self) {
        let list: Vec<&KvCacheEntry> = self.entries.values().collect();
        if let Ok(json) = serde_json::to_string_pretty(&list) {
            let _ = std::fs::write(&self.index_path, json);
        }
    }
}
