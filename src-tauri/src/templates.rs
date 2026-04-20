use crate::ast::{FilledTemplate, TemplateAst};
use crate::builtin_templates;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub ast: TemplateAst,
    #[serde(default)]
    pub example_input: Option<String>,
    #[serde(default)]
    pub example_filled: Option<FilledTemplate>,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_ast_version")]
    pub ast_version: u32,
}

fn default_ast_version() -> u32 {
    1
}

pub struct TemplatesStore {
    templates: Vec<Template>,
    file_path: PathBuf,
}

impl TemplatesStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let file_path = data_dir.join("templates.json");

        // Load + detect legacy (pre-AST) entries. If any entry lacks the `ast` field,
        // we wipe user templates (backward compat abandoned by design); built-ins are
        // always re-seeded from code anyway.
        let templates: Vec<Template> = if file_path.exists() {
            match std::fs::read_to_string(&file_path) {
                Ok(s) => {
                    let legacy_marker = is_legacy_json(&s);
                    if legacy_marker {
                        log::info!("Legacy templates.json detected — wiping (AST migration)");
                        Vec::new()
                    } else {
                        serde_json::from_str::<Vec<Template>>(&s).unwrap_or_default()
                    }
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        let mut store = Self {
            templates,
            file_path,
        };
        store.refresh_builtins();
        store
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
        let now = crate::notes::chrono_now();
        template.created_at = now.clone();
        template.updated_at = now;
        template.ast_version = 1;
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
        existing.ast = patch.ast;
        existing.example_input = patch.example_input;
        existing.example_filled = patch.example_filled;
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

/// Detect pre-AST templates (had `content: String` / `schema` / `system_prompt`
/// instead of `ast` object). Cheap structural check on raw JSON.
fn is_legacy_json(s: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(s) else {
        return false;
    };
    let Some(arr) = value.as_array() else {
        return false;
    };
    arr.iter().any(|entry| {
        entry.as_object().map(|obj| !obj.contains_key("ast")).unwrap_or(false)
    })
}
