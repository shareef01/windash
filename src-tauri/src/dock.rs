// dock.rs — dockable window behaviour.
// The main window can dock to the left/right edge of the current monitor as a
// sidebar (always-on-top), or float freely. Dock state persists as JSON in AppData.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewWindow};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum DockEdge {
    None,
    Left,
    Right,
}

impl DockEdge {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockEdge::None => "none",
            DockEdge::Left => "left",
            DockEdge::Right => "right",
        }
    }
    pub fn from_str(s: &str) -> DockEdge {
        match s {
            "left" => DockEdge::Left,
            "right" => DockEdge::Right,
            _ => DockEdge::None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DockConfig {
    pub edge: String,
    pub width: u32,
    pub always_on_top: bool,
}

impl DockConfig {
    pub fn edge(&self) -> DockEdge {
        DockEdge::from_str(&self.edge)
    }
}

pub struct DockStore {
    path: PathBuf,
}

impl DockStore {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        let path = app
            .path()
            .resolve("windash-dock.json", tauri::path::BaseDirectory::AppData)
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
            let default = DockConfig {
                edge: "right".to_string(),
                width: 390,
                always_on_top: true,
            };
            fs::write(&self.path, serde_json::to_string_pretty(&default).unwrap())
                .map_err(|e| format!("write: {}", e))?;
        }
        Ok(())
    }

    fn load(&self) -> Result<DockConfig, String> {
        let text = fs::read_to_string(&self.path).map_err(|e| format!("read: {}", e))?;
        serde_json::from_str(&text).map_err(|e| format!("parse: {}", e))
    }

    fn save(&self, cfg: &DockConfig) -> Result<(), String> {
        fs::write(&self.path, serde_json::to_string_pretty(cfg).unwrap())
            .map_err(|e| format!("write: {}", e))
    }

    pub fn get(&self) -> DockConfig {
        self.load().unwrap_or(DockConfig {
            edge: "right".to_string(),
            width: 360,
            always_on_top: true,
        })
    }

    pub fn set(&self, cfg: &DockConfig) -> Result<(), String> {
        self.save(cfg)
    }
}

/// Position the window according to the dock config. Returns an error string on failure.
pub fn apply_dock(window: &WebviewWindow, cfg: &DockConfig) -> Result<(), String> {
    let edge = cfg.edge();
    let width = cfg.width.max(280).min(640);

    // Monitor geometry (physical pixels). Prefer the window's current monitor, but
    // fall back to the primary monitor (current_monitor() can be None before the
    // window is first shown/positioned, e.g. during setup).
    let monitor = if let Ok(Some(m)) = window.current_monitor() {
        m
    } else {
        window
            .primary_monitor()
            .ok()
            .flatten()
            .ok_or_else(|| "no monitor available".to_string())?
    };
    // Monitor geometry. We want the window `width` CSS pixels wide on screen
    // regardless of display scale, so work in LOGICAL coordinates: convert the
    // monitor's physical size/position to logical by dividing by the scale factor.
    // (Sizing in physical pixels caused content to be clipped on scaled displays.)
    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_size = *monitor.size();
    let phys_pos = *monitor.position();
    let m_w = phys_size.width as f64 / scale;
    let m_h = phys_size.height as f64 / scale;
    let m_x = phys_pos.x as f64 / scale;
    let m_y = phys_pos.y as f64 / scale;
    let width_f = width as f64;

    match edge {
        DockEdge::None => {
            let _ = window.set_always_on_top(cfg.always_on_top);
            let _ = window.set_resizable(true);
            Ok(())
        }
        DockEdge::Left | DockEdge::Right => {
            let x = if edge == DockEdge::Right {
                m_x + m_w - width_f
            } else {
                m_x
            };
            let _ = window.set_resizable(false);
            let _ = window.set_size(LogicalSize::new(width_f, m_h));
            let _ = window.set_position(LogicalPosition::new(x, m_y));
            let _ = window.set_always_on_top(cfg.always_on_top);
            // When docked as a sidebar, skip the taskbar so it doesn't take a slot.
            let _ = window.set_skip_taskbar(true);
            Ok(())
        }
    }
}
