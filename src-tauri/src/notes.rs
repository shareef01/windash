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
            crate::persist::atomic_write(
                &self.path,
                &serde_json::to_string_pretty(&NotesData::default())
                    .map_err(|e| format!("serialize: {e}"))?,
            )?;
        } else if let Ok(text) = fs::read_to_string(&self.path) {
            if Self::parse_or_recover(&text).is_none() {
                crate::persist::backup_corrupt_file(&self.path);
                crate::persist::atomic_write(
                    &self.path,
                    &serde_json::to_string_pretty(&NotesData::default())
                        .map_err(|e| format!("serialize: {e}"))?,
                )?;
            }
        }
        Ok(())
    }

    fn parse_or_recover(text: &str) -> Option<NotesData> {
        // 1. Standard NotesData { notes, next_id }
        if let Ok(data) = serde_json::from_str::<NotesData>(text) {
            return Some(data);
        }

        // 2. Object with "notes" array where next_id is absent
        #[derive(Deserialize)]
        struct PartialNotesData {
            notes: Vec<Note>,
        }
        if let Ok(partial) = serde_json::from_str::<PartialNotesData>(text) {
            let next_id = partial
                .notes
                .iter()
                .map(|n| n.id)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
            return Some(NotesData {
                notes: partial.notes,
                next_id,
            });
        }

        // 3. Raw array of Note objects
        if let Ok(notes) = serde_json::from_str::<Vec<Note>>(text) {
            let next_id = notes.iter().map(|n| n.id).max().map(|m| m + 1).unwrap_or(0);
            return Some(NotesData { notes, next_id });
        }

        None
    }

    fn load(&self) -> Result<NotesData, String> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(NotesData::default()),
            Err(e) => return Err(format!("read: {e}")),
        };

        if let Some(data) = Self::parse_or_recover(&text) {
            return Ok(data);
        }

        // Corrupt notes file: preserve original bytes before resetting
        log::warn!(
            "notes file corrupted, preserving backup: {}",
            self.path.display()
        );
        crate::persist::backup_corrupt_file(&self.path);
        let data = NotesData::default();
        let _ = self.save(&data);
        Ok(data)
    }

    fn save(&self, data: &NotesData) -> Result<(), String> {
        crate::persist::atomic_write(
            &self.path,
            &serde_json::to_string_pretty(data).map_err(|e| format!("serialize: {e}"))?,
        )
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
        let text = text.trim();
        if text.is_empty() {
            return Err("Note is empty.".into());
        }
        if text.chars().count() > 500 {
            return Err("Note is too long (500 character limit).".into());
        }
        let mut data = self.load()?;
        if data.notes.len() >= 200 {
            return Err("Too many notes.".into());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_serialization() {
        let note = Note {
            id: 1,
            text: "Hello Windash".into(),
            created_at: "2026-08-25T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&note).unwrap();
        let deserialized: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.text, "Hello Windash");
    }

    #[test]
    fn test_notes_data_default() {
        let data = NotesData::default();
        assert!(data.notes.is_empty());
        assert_eq!(data.next_id, 0);
    }

    #[test]
    fn test_notes_recovery_missing_next_id() {
        let json =
            r#"{"notes": [{"id": 5, "text": "Recover me", "created_at": "2026-08-25T00:00:00Z"}]}"#;
        let recovered = NotesStore::parse_or_recover(json).expect("recovered");
        assert_eq!(recovered.notes.len(), 1);
        assert_eq!(recovered.notes[0].text, "Recover me");
        assert_eq!(recovered.next_id, 6);
    }

    #[test]
    fn test_notes_recovery_raw_array() {
        let json = r#"[{"id": 10, "text": "Array note", "created_at": "2026-08-25T00:00:00Z"}]"#;
        let recovered = NotesStore::parse_or_recover(json).expect("recovered");
        assert_eq!(recovered.notes.len(), 1);
        assert_eq!(recovered.notes[0].text, "Array note");
        assert_eq!(recovered.next_id, 11);
    }

    #[test]
    fn test_notes_recovery_rejects_unusable_json() {
        let json = r#"{"invalid": "unrelated object"}"#;
        assert!(NotesStore::parse_or_recover(json).is_none());
        assert!(NotesStore::parse_or_recover("not json").is_none());
    }
}
