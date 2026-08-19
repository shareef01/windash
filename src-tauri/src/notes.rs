// notes.rs — Notes persistence via direct JSON file I/O in the app data directory.
// Simple append-only notes with timestamp. No plugin dependency.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct NotesData {
    notes: Vec<Note>,
    next_id: u64,
}

pub struct NotesStore {
    path: PathBuf,
}

impl NotesStore {
    /// Create a notes store backed by a JSON file in the app's AppData directory.
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = app
            .path()
            .resolve("windash-notes.json", tauri::path::BaseDirectory::AppData)
            .map_err(|e| format!("path resolve: {}", e))?;

        let store = Self { path };
        store.ensure_exists().map_err(|e| format!("init: {}", e))?;
        Ok(store)
    }

    fn ensure_exists(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        if !self.path.exists() {
            fs::write(
                &self.path,
                serde_json::to_string_pretty(&NotesData::default()).unwrap(),
            )
            .map_err(|e| format!("write: {}", e))?;
        }
        Ok(())
    }

    fn load(&self) -> Result<NotesData, String> {
        let text = fs::read_to_string(&self.path).map_err(|e| format!("read: {}", e))?;
        match serde_json::from_str::<NotesData>(&text) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Corrupt notes file: repair with an empty store rather than fail.
                log::warn!("notes file corrupted, repairing: {}", self.path.display());
                let data = NotesData::default();
                let _ = self.save(&data);
                Ok(data)
            }
        }
    }

    fn save(&self, data: &NotesData) -> Result<(), String> {
        fs::write(&self.path, serde_json::to_string_pretty(data).unwrap())
            .map_err(|e| format!("write: {}", e))?;
        Ok(())
    }

    /// List all notes, newest first.
    pub fn list(&self) -> Result<Vec<Note>, String> {
        let data = self.load()?;
        let mut notes = data.notes;
        notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(notes)
    }

    /// Add a new note and persist.
    pub fn add(&self, text: &str) -> Result<Note, String> {
        let mut data = self.load()?;
        let id = data.next_id;
        data.next_id += 1;

        let note = Note {
            id,
            text: text.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        data.notes.push(note.clone());
        self.save(&data)?;
        Ok(note)
    }

    /// Delete a note by id and persist.
    pub fn delete(&self, id: u64) -> Result<(), String> {
        let mut data = self.load()?;
        let len_before = data.notes.len();
        data.notes.retain(|n| n.id != id);
        if data.notes.len() == len_before {
            return Err(format!("note id {} not found", id));
        }
        self.save(&data)?;
        Ok(())
    }
}
