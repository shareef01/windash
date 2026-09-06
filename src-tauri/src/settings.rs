// settings.rs — persisted user preferences (JSON in AppData).
// Single source of truth for always-on-top, refresh interval, theme,
// dock mode, and section visibility. Kept deliberately small.

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemePreference {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

fn deserialize_theme_lenient<'de, D>(deserializer: D) -> Result<ThemePreference, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(s) => s,
        Err(_) => return Ok(ThemePreference::System),
    };
    match s.to_lowercase().as_str() {
        "dark" => Ok(ThemePreference::Dark),
        "light" => Ok(ThemePreference::Light),
        _ => Ok(ThemePreference::System),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProcessSort {
    #[default]
    Cpu,
    Mem,
    Name,
}

impl ProcessSort {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Mem => "mem",
            Self::Name => "name",
        }
    }
}

fn deserialize_sort_lenient<'de, D>(deserializer: D) -> Result<ProcessSort, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(s) => s,
        Err(_) => return Ok(ProcessSort::Cpu),
    };
    match s.to_lowercase().as_str() {
        "mem" => Ok(ProcessSort::Mem),
        "name" => Ok(ProcessSort::Name),
        _ => Ok(ProcessSort::Cpu),
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct Settings {
    pub always_on_top: bool,
    /// Refresh interval in milliseconds (clamped 1000..10000).
    pub refresh_ms: u64,
    /// Theme preference: System, Dark, or Light
    #[serde(deserialize_with = "deserialize_theme_lenient")]
    pub theme: ThemePreference,
    pub show_disks: bool,
    pub show_processes: bool,
    pub show_notes: bool,
    pub show_actions: bool,
    /// Use the Windows 11 Mica backdrop for the window.
    pub mica_enabled: bool,
    /// Last process sort key, persisted so it survives restarts.
    #[serde(deserialize_with = "deserialize_sort_lenient")]
    pub sort_key: ProcessSort,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            always_on_top: true,
            refresh_ms: 2000,
            theme: ThemePreference::System,
            show_disks: true,
            show_processes: true,
            show_notes: true,
            show_actions: true,
            mica_enabled: true,
            sort_key: ProcessSort::Cpu,
        }
    }
}

impl Settings {
    /// Canonical normalization path ensuring all values conform to safe operational bounds.
    pub fn normalize(mut self) -> Self {
        self.refresh_ms = self.refresh_ms.clamp(1000, 10_000);
        self
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
        let cache = store.load()?;
        Ok(Self {
            path: store.path,
            cache,
        })
    }

    fn ensure_exists(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        if !self.path.exists() {
            let data = SettingsData {
                settings: Settings::default(),
            };
            crate::persist::atomic_write(
                &self.path,
                &serde_json::to_string_pretty(&data).map_err(|e| format!("serialize: {e}"))?,
            )?;
        }
        Ok(())
    }

    pub fn load(&self) -> Result<Settings, String> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(format!("read: {e}")),
        };

        match serde_json::from_str::<SettingsData>(&text) {
            Ok(data) => {
                let normalized = data.settings.clone().normalize();
                if normalized != data.settings {
                    let _ = self.save(&normalized);
                }
                Ok(normalized)
            }
            Err(e) => {
                log::warn!(
                    "settings file corrupted ({e}), backing up: {}",
                    self.path.display()
                );
                crate::persist::backup_corrupt_file(&self.path);
                let default = Settings::default();
                let _ = self.save(&default);
                Ok(default)
            }
        }
    }

    fn save(&self, s: &Settings) -> Result<(), String> {
        let data = SettingsData {
            settings: s.clone(),
        };
        crate::persist::atomic_write(
            &self.path,
            &serde_json::to_string_pretty(&data).map_err(|e| format!("serialize: {e}"))?,
        )
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
            s.refresh_ms = v;
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
        let s = s.normalize();
        self.save(&s)?;
        self.cache = s.clone();
        Ok(s)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SettingsPatch {
    pub always_on_top: Option<bool>,
    pub refresh_ms: Option<u64>,
    pub theme: Option<ThemePreference>,
    pub show_disks: Option<bool>,
    pub show_processes: Option<bool>,
    pub show_notes: Option<bool>,
    pub show_actions: Option<bool>,
    pub mica_enabled: Option<bool>,
    pub sort_key: Option<ProcessSort>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_defaults() {
        let s = Settings::default();
        assert!(s.always_on_top);
        assert_eq!(s.refresh_ms, 2000);
        assert_eq!(s.theme, ThemePreference::System);
        assert!(s.show_disks);
        assert!(s.show_processes);
        assert!(s.show_notes);
        assert!(s.show_actions);
        assert!(s.mica_enabled);
        assert_eq!(s.sort_key, ProcessSort::Cpu);
    }

    #[test]
    fn test_settings_json_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.refresh_ms, 2000);
        assert_eq!(parsed.theme, ThemePreference::System);
        assert_eq!(parsed.sort_key, ProcessSort::Cpu);
    }

    #[test]
    fn test_settings_normalize_clamps_refresh_ms() {
        let zero = Settings {
            refresh_ms: 0,
            ..Default::default()
        }
        .normalize();
        assert_eq!(zero.refresh_ms, 1000);

        let small = Settings {
            refresh_ms: 500,
            ..Default::default()
        }
        .normalize();
        assert_eq!(small.refresh_ms, 1000);

        let large = Settings {
            refresh_ms: 99_999,
            ..Default::default()
        }
        .normalize();
        assert_eq!(large.refresh_ms, 10_000);
    }

    #[test]
    fn test_invalid_enums_repair_to_defaults() {
        let json = r#"{"refresh_ms": 0, "theme": "invalid_theme", "sort_key": "junk"}"#;
        let parsed: Settings = serde_json::from_str(json).unwrap();
        let normalized = parsed.normalize();
        assert_eq!(normalized.refresh_ms, 1000);
        assert_eq!(normalized.theme, ThemePreference::System);
        assert_eq!(normalized.sort_key, ProcessSort::Cpu);
    }

    #[test]
    fn test_unknown_fields_are_ignored() {
        let json = r#"{"always_on_top":false,"refresh_ms":3000,"theme":"light","show_disks":false,"show_processes":true,"show_notes":true,"show_actions":true,"mica_enabled":false,"sort_key":"mem","future_field":true}"#;
        let parsed: Settings = serde_json::from_str(json).unwrap();
        assert!(!parsed.always_on_top);
        assert_eq!(parsed.refresh_ms, 3000);
        assert_eq!(parsed.theme, ThemePreference::Light);
        assert_eq!(parsed.sort_key, ProcessSort::Mem);
    }
}
