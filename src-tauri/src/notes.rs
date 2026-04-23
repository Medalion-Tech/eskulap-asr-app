use crate::ast::FilledTemplate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub timestamp: String,
    pub text: String,
    #[serde(default)]
    pub raw_transcription: Option<String>,
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub template_name: Option<String>,
    #[serde(default)]
    pub filled: Option<FilledTemplate>,
    #[serde(default)]
    pub raw_llm_output: Option<String>,
}

pub struct NotesStore {
    notes: Vec<Note>,
    file_path: PathBuf,
}

impl NotesStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let file_path = data_dir.join("notes.json");
        let notes = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self { notes, file_path }
    }

    pub fn get_all(&self) -> Vec<Note> {
        self.notes.clone()
    }

    pub fn get(&self, id: &str) -> Option<Note> {
        self.notes.iter().find(|n| n.id == id).cloned()
    }

    pub fn add(&mut self, text: String) -> Note {
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono_now(),
            text,
            raw_transcription: None,
            template_id: None,
            template_name: None,
            filled: None,
            raw_llm_output: None,
        };
        self.notes.push(note.clone());
        self.save();
        note
    }

    pub fn add_with_template(
        &mut self,
        text: String,
        raw_transcription: String,
        template_id: String,
        template_name: String,
        filled: Option<FilledTemplate>,
        raw_llm_output: Option<String>,
    ) -> Note {
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono_now(),
            text,
            raw_transcription: Some(raw_transcription),
            template_id: Some(template_id),
            template_name: Some(template_name),
            filled,
            raw_llm_output,
        };
        self.notes.push(note.clone());
        self.save();
        note
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.notes.len();
        self.notes.retain(|n| n.id != id);
        if self.notes.len() != len {
            self.save();
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, id: &str, text: String) -> Option<Note> {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            note.text = text;
            let updated = note.clone();
            self.save();
            Some(updated)
        } else {
            None
        }
    }

    pub fn update_with_template(
        &mut self,
        id: &str,
        text: String,
        template_id: String,
        template_name: String,
    ) -> Option<Note> {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            note.text = text;
            note.template_id = Some(template_id);
            note.template_name = Some(template_name);
            let updated = note.clone();
            self.save();
            Some(updated)
        } else {
            None
        }
    }

    /// Replace the full filled template (after structural edits) and re-render `text`.
    /// Caller supplies the new `text` (computed from template AST + filled in commands layer).
    pub fn set_filled(&mut self, id: &str, filled: FilledTemplate, text: String) -> Option<Note> {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            note.filled = Some(filled);
            note.text = text;
            let updated = note.clone();
            self.save();
            Some(updated)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.notes.clear();
        self.save();
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.notes) {
            std::fs::write(&self.file_path, json).ok();
        }
    }
}

/// Current wall-clock time formatted as `YYYY-MM-DD HH:MM:SS` in the user's
/// **local** timezone. The frontend splits on the space and displays only the
/// HH:MM portion (see `NotesList.svelte::formatTime`), so the caller must
/// hand back a string that's already in local time — otherwise the UI shows
/// UTC, which visibly disagrees with the system clock (reported on a Polish
/// CEST machine: notes saved at 21:XX local displayed as 19:XX).
pub fn chrono_now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
