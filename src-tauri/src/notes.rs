use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub timestamp: String,
    pub text: String,
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

    pub fn add(&mut self, text: String) -> Note {
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono_now(),
            text,
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

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Simple ISO-ish timestamp without chrono dependency
    let secs = now.as_secs();
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Approximate date calculation (good enough for display)
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }
    let months_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in &months_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    let d = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y, m, d, hours, minutes, seconds
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
