// settings.rs — persisted user preferences (JSON in AppData).
// Single source of truth for always-on-top, refresh interval, theme,
// dock mode, and section visibility. Kept deliberately small.

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub always_on_top: bool,
    /// Refresh interval in milliseconds (clamped 1000..10000).
    pub refresh_ms: u64,
    /// "dark" | "light" | "system"
    pub theme: String,
    pub show_disks: bool,
    pub show_processes: bool,
    pub show_notes: bool,
    pub show_actions: bool,
    /// Use the Windows 11 Mica backdrop for the window (falls back to blur).
    pub mica_enabled: bool,
    /// Last process sort key, persisted so it survives restarts.
    pub sort_key: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            always_on_top: true,
            refresh_ms: 2000,
            theme: "system".into(),
            show_disks: true,
            show_processes: true,
            show_notes: true,
            show_actions: true,
            mica_enabled: true,
            sort_key: "cpu".into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct SettingsData {
    settings: Settings,
}

pub struct SettingsStore {
    path: std::path::PathBuf,
    pub cache: Settings,
}

impl SettingsStore {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = app
            .path()
            .resolve("windash-settings.json", tauri::path::BaseDirectory::AppData)
            .map_err(|e| format!("path resolve: {}", e))?;
        let store = Self {
            path,
            cache: Settings::default(),
        };
        store.ensure_exists()?;
        let cache = store.load()?.settings;
        Ok(Self {
            path: store.path,
            cache,
        })
    }

    fn ensure_exists(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        // If the file is missing, write defaults. If it exists but is corrupt,
        // repair it (overwrite with defaults) rather than crashing at startup.
        if !self.path.exists() {
            let data = SettingsData {
                settings: Settings::default(),
            };
            fs::write(&self.path, serde_json::to_string_pretty(&data).unwrap())
                .map_err(|e| format!("write: {}", e))?;
        } else if fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str::<SettingsData>(&t).ok())
            .is_none()
        {
            log::warn!(
                "settings file corrupted, repairing: {}",
                self.path.display()
            );
            let data = SettingsData {
                settings: Settings::default(),
            };
            fs::write(&self.path, serde_json::to_string_pretty(&data).unwrap())
                .map_err(|e| format!("write: {}", e))?;
        }
        Ok(())
    }

    fn load(&self) -> Result<SettingsData, String> {
        let text = fs::read_to_string(&self.path).map_err(|e| format!("read: {}", e))?;
        let data: SettingsData =
            serde_json::from_str(&text).map_err(|e| format!("parse: {}", e))?;
        Ok(data)
    }

    fn save(&self, s: &Settings) -> Result<(), String> {
        let data = SettingsData {
            settings: s.clone(),
        };
        fs::write(&self.path, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| format!("write: {}", e))?;
        Ok(())
    }

    pub fn get(&self) -> Settings {
        self.cache.clone()
    }

    /// Merge a partial update (only provided keys change) and persist.
    pub fn update(&mut self, patch: SettingsPatch) -> Result<Settings, String> {
        let mut s = self.cache.clone();
        if let Some(v) = patch.always_on_top {
            s.always_on_top = v;
        }
        if let Some(v) = patch.refresh_ms {
            s.refresh_ms = v.clamp(1000, 10000);
        }
        if let Some(v) = patch.theme {
            s.theme = v;
        }
        if let Some(v) = patch.show_disks {
            s.show_disks = v;
        }
        if let Some(v) = patch.show_processes {
            s.show_processes = v;
        }
        if let Some(v) = patch.show_notes {
            s.show_notes = v;
        }
        if let Some(v) = patch.show_actions {
            s.show_actions = v;
        }
        if let Some(v) = patch.mica_enabled {
            s.mica_enabled = v;
        }
        if let Some(v) = patch.sort_key {
            s.sort_key = v;
        }
        self.save(&s)?;
        self.cache = s.clone();
        Ok(s)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SettingsPatch {
    pub always_on_top: Option<bool>,
    pub refresh_ms: Option<u64>,
    pub theme: Option<String>,
    pub show_disks: Option<bool>,
    pub show_processes: Option<bool>,
    pub show_notes: Option<bool>,
    pub show_actions: Option<bool>,
    pub mica_enabled: Option<bool>,
    pub sort_key: Option<String>,
}
