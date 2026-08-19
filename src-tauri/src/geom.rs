// geom.rs — persistence of floating-window geometry (position + size).
// Only used when the window is floating (edge == "none"); when docked, the
// window geometry is derived from the monitor + dock config instead.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct WindowGeom {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct GeomStore {
    path: PathBuf,
}

impl GeomStore {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = app
            .path()
            .resolve("windash-geom.json", tauri::path::BaseDirectory::AppData)
            .map_err(|e| format!("path resolve: {}", e))?;
        let store = Self { path };
        store.ensure_exists()?;
        Ok(store)
    }

    fn ensure_exists(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
        }
        if !self.path.exists() {
            let default = WindowGeom {
                x: 200,
                y: 100,
                width: 390,
                height: 720,
            };
            fs::write(
                &self.path,
                serde_json::to_string_pretty(&default).unwrap(),
            )
            .map_err(|e| format!("write: {}", e))?;
        }
        Ok(())
    }

    pub fn get(&self) -> WindowGeom {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| {
                // Repair / first-run default.
                let g = WindowGeom {
                    x: 200,
                    y: 100,
                    width: 390,
                    height: 720,
                };
                let _ = self.set(&g);
                g
            })
    }

    pub fn set(&self, geom: &WindowGeom) -> Result<(), String> {
        fs::write(&self.path, serde_json::to_string_pretty(geom).unwrap())
            .map_err(|e| format!("write: {}", e))
    }
}
