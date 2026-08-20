// geom.rs — persistence of floating-window geometry (position + size), keyed by
// dock edge. When docked (left/right) we remember the last floating geometry so
// un-docking restores a sensible shape; when floating (none) we remember the
// user's window position/size per edge so it's stable across restarts.
//
// Only used when the window is floating (edge == "none"); when docked, the
// window geometry is derived from the monitor + dock config instead.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

type GeomMap = HashMap<String, WindowGeom>;

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
            let mut map: GeomMap = HashMap::new();
            map.insert(
                "none".into(),
                WindowGeom {
                    x: 200,
                    y: 100,
                    width: 390,
                    height: 720,
                },
            );
            fs::write(&self.path, serde_json::to_string_pretty(&map).unwrap())
                .map_err(|e| format!("write: {}", e))?;
        }
        Ok(())
    }

    fn load_map(&self) -> GeomMap {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(_) => return GeomMap::new(),
        };
        // Current format: a JSON object keyed by edge.
        if let Ok(map) = serde_json::from_str::<GeomMap>(&text) {
            return map;
        }
        // Legacy format (<= v0.1.0): a single {x,y,width,height} object — treat
        // it as the "none" (floating) geometry and migrate on next write.
        if let Ok(g) = serde_json::from_str::<WindowGeom>(&text) {
            let mut map = GeomMap::new();
            map.insert("none".into(), g);
            return map;
        }
        GeomMap::new()
    }

    pub fn get(&self, edge: &str) -> WindowGeom {
        let map = self.load_map();
        if let Some(g) = map.get(edge) {
            return g.clone();
        }
        // Fall back to "none" if this edge has no saved geometry yet.
        if edge != "none" {
            if let Some(g) = map.get("none") {
                return g.clone();
            }
        }
        // Repair / first-run default.
        let g = WindowGeom {
            x: 200,
            y: 100,
            width: 390,
            height: 720,
        };
        let _ = self.set(edge, &g);
        g
    }

    pub fn set(&self, edge: &str, geom: &WindowGeom) -> Result<(), String> {
        let mut map = self.load_map();
        map.insert(edge.to_string(), geom.clone());
        fs::write(&self.path, serde_json::to_string_pretty(&map).unwrap())
            .map_err(|e| format!("write: {}", e))
    }
}
