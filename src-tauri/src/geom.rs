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

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
            crate::persist::atomic_write(
                &self.path,
                &serde_json::to_string_pretty(&map).map_err(|e| format!("serialize: {e}"))?,
            )?;
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
        let mut g = if let Some(g) = map.get(edge) {
            g.clone()
        } else if edge != "none" {
            if let Some(g) = map.get("none") {
                g.clone()
            } else {
                WindowGeom {
                    x: 200,
                    y: 100,
                    width: 390,
                    height: 720,
                }
            }
        } else {
            WindowGeom {
                x: 200,
                y: 100,
                width: 390,
                height: 720,
            }
        };
        // Sanity clamp so a bad geometry can never throw the window off-screen or blow up
        g.width = g.width.clamp(280, 720);
        g.height = g.height.clamp(440, 1000);
        if g.x < 0 || g.x > 3840 {
            g.x = 200;
        }
        if g.y < 0 || g.y > 2160 {
            g.y = 100;
        }
        g
    }

    pub fn set(&self, edge: &str, geom: &WindowGeom) -> Result<(), String> {
        let mut map = self.load_map();
        let sanitized = WindowGeom {
            x: geom.x.clamp(0, 3840),
            y: geom.y.clamp(0, 2160),
            width: geom.width.clamp(280, 720),
            height: geom.height.clamp(440, 1000),
        };
        if map.get(edge) == Some(&sanitized) {
            return Ok(());
        }
        map.insert(edge.to_string(), sanitized.clone());
        crate::persist::atomic_write(
            &self.path,
            &serde_json::to_string_pretty(&map).map_err(|e| format!("serialize: {e}"))?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_geom_default() {
        let g = WindowGeom::default();
        assert_eq!(g.x, 0);
        assert_eq!(g.y, 0);
        assert_eq!(g.width, 0);
        assert_eq!(g.height, 0);
    }

    #[test]
    fn test_geom_map_deserialization() {
        let json = r#"{"none": {"x": 200, "y": 100, "width": 390, "height": 720}}"#;
        let map: GeomMap = serde_json::from_str(json).unwrap();
        let g = map.get("none").unwrap();
        assert_eq!(g.x, 200);
        assert_eq!(g.y, 100);
        assert_eq!(g.width, 390);
        assert_eq!(g.height, 720);
    }
}
