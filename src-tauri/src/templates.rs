use crate::builtin_templates;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub content: String,
    pub example_input: Option<String>,
    pub example_output: Option<String>,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,

    // Legacy fields kept for one-time migration from the pre-`content` format.
    // Present in on-disk JSON written by older builds; merged into `content` on load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

pub struct TemplatesStore {
    templates: Vec<Template>,
    file_path: PathBuf,
}

impl TemplatesStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let file_path = data_dir.join("templates.json");
        let mut store = if file_path.exists() {
            let templates: Vec<Template> = std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            Self {
                templates,
                file_path,
            }
        } else {
            Self {
                templates: Vec::new(),
                file_path,
            }
        };
        store.migrate_legacy_fields();
        store.refresh_builtins();
        store
    }

    /// For entries written by older builds (`schema` + `system_prompt`) without `content`,
    /// merge the two legacy fields into `content` and drop them.
    fn migrate_legacy_fields(&mut self) {
        let mut changed = false;
        for t in self.templates.iter_mut() {
            let has_legacy = t.schema.is_some() || t.system_prompt.is_some();
            if t.content.is_empty() && has_legacy {
                let schema = t.schema.take().unwrap_or_default();
                let sys = t.system_prompt.take().unwrap_or_default();
                t.content = match (schema.trim().is_empty(), sys.trim().is_empty()) {
                    (true, true) => String::new(),
                    (false, true) => schema,
                    (true, false) => sys,
                    (false, false) => format!("{}\n\n{}", sys.trim(), schema.trim()),
                };
                changed = true;
            } else if has_legacy {
                // content already present — just drop the legacy fields
                t.schema = None;
                t.system_prompt = None;
                changed = true;
            }
        }
        if changed {
            self.save();
        }
    }

    /// Always re-seed built-ins from code so prompt improvements in new releases
    /// propagate. User-created templates are untouched.
    fn refresh_builtins(&mut self) {
        self.templates.retain(|t| !t.is_builtin);
        for b in builtin_templates::all() {
            self.templates.push(b);
        }
        self.save();
    }

    pub fn get_all(&self) -> Vec<Template> {
        self.templates.clone()
    }

    pub fn get(&self, id: &str) -> Option<Template> {
        self.templates.iter().find(|t| t.id == id).cloned()
    }

    pub fn add(&mut self, mut template: Template) -> Template {
        template.id = uuid::Uuid::new_v4().to_string();
        template.is_builtin = false;
        template.schema = None;
        template.system_prompt = None;
        let now = crate::notes::chrono_now();
        template.created_at = now.clone();
        template.updated_at = now;
        self.templates.push(template.clone());
        self.save();
        template
    }

    pub fn update(&mut self, id: &str, patch: Template) -> Result<Template, String> {
        let idx = self
            .templates
            .iter()
            .position(|t| t.id == id)
            .ok_or("Template not found")?;
        if self.templates[idx].is_builtin {
            return Err("Nie można edytować wbudowanego szablonu".to_string());
        }
        let existing = &mut self.templates[idx];
        existing.name = patch.name;
        existing.description = patch.description;
        existing.content = patch.content;
        existing.example_input = patch.example_input;
        existing.example_output = patch.example_output;
        existing.updated_at = crate::notes::chrono_now();
        let result = existing.clone();
        self.save();
        Ok(result)
    }

    pub fn delete(&mut self, id: &str) -> Result<bool, String> {
        let idx = self
            .templates
            .iter()
            .position(|t| t.id == id)
            .ok_or("Template not found")?;
        if self.templates[idx].is_builtin {
            return Err("Nie można usunąć wbudowanego szablonu".to_string());
        }
        self.templates.remove(idx);
        self.save();
        Ok(true)
    }

    pub fn reset_builtins(&mut self) {
        self.refresh_builtins();
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.templates) {
            std::fs::write(&self.file_path, json).ok();
        }
    }
}
