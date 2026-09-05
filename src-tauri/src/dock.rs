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
                edge: "none".to_string(),
                width: 390,
                always_on_top: true,
            };
            crate::persist::atomic_write(
                &self.path,
                &serde_json::to_string_pretty(&default).map_err(|e| format!("serialize: {e}"))?,
            )?;
        }
        Ok(())
    }

    fn load(&self) -> Result<DockConfig, String> {
        let text = fs::read_to_string(&self.path).map_err(|e| format!("read: {}", e))?;
        match serde_json::from_str::<DockConfig>(&text) {
            Ok(cfg) => Ok(cfg),
            Err(_) => {
                log::warn!("dock file corrupted, repairing: {}", self.path.display());
                let cfg = DockConfig {
                    edge: "none".to_string(),
                    width: 390,
                    always_on_top: true,
                };
                let _ = self.save(&cfg);
                Ok(cfg)
            }
        }
    }

    fn save(&self, cfg: &DockConfig) -> Result<(), String> {
        crate::persist::atomic_write(
            &self.path,
            &serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize: {e}"))?,
        )
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
    let width = cfg.width.clamp(280, 640);

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
            let _ = window.set_skip_taskbar(false);
            Ok(())
        }
    }
}

/// How close (logical px) the window's left/right edge must be to a monitor edge
/// before we treat a drag as an intentional dock gesture.
const SNAP_THRESHOLD: f64 = 48.0;

/// Geometry rectangle in logical coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Pure function to calculate edge snapping given monitor work area and window geometry.
pub fn calculate_snap_edge(
    work_area: Rect,
    win_x: f64,
    win_w: f64,
    win_y: f64,
    snap_threshold: f64,
) -> Option<DockEdge> {
    let dist_right = (work_area.x + work_area.width) - (win_x + win_w);
    let dist_left = win_x - work_area.x;

    if (-snap_threshold..=snap_threshold).contains(&dist_right)
        && win_y > work_area.y - snap_threshold
        && win_y < work_area.y + work_area.height
    {
        return Some(DockEdge::Right);
    }
    if (-snap_threshold..=snap_threshold).contains(&dist_left)
        && win_y > work_area.y - snap_threshold
        && win_y < work_area.y + work_area.height
    {
        return Some(DockEdge::Left);
    }
    None
}

/// Given the window's current top-left position, decide whether it should snap
/// to the left or right edge of its current monitor. Returns None when it is not
/// close enough to any edge (so a drag in open space leaves the window floating).
///
/// Uses the monitor *work area* (excludes the taskbar) so a docked sidebar never
/// sits underneath the taskbar.
pub fn detect_edge(window: &WebviewWindow) -> Option<DockEdge> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let wa = monitor.work_area();
    let work_area = Rect {
        x: wa.position.x as f64 / scale,
        y: wa.position.y as f64 / scale,
        width: wa.size.width as f64 / scale,
        height: wa.size.height as f64 / scale,
    };

    let pos = window.outer_position().ok()?;
    let px = pos.x as f64 / scale;
    let py = pos.y as f64 / scale;
    let size = window.outer_size().ok()?;
    let win_w = size.width as f64 / scale;

    calculate_snap_edge(work_area, px, win_w, py, SNAP_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_edge_from_str() {
        assert_eq!(DockEdge::from_str("left"), DockEdge::Left);
        assert_eq!(DockEdge::from_str("right"), DockEdge::Right);
        assert_eq!(DockEdge::from_str("none"), DockEdge::None);
        assert_eq!(DockEdge::from_str("other"), DockEdge::None);
    }

    #[test]
    fn test_dock_config_default() {
        let cfg = DockConfig::default();
        assert_eq!(cfg.edge(), DockEdge::None);
    }

    #[test]
    fn test_calculate_snap_edge() {
        // Monitor: 1920x1080 at (0,0)
        let wa = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        // Window width: 390
        // Near right edge: win_x = 1920 - 390 = 1530
        let right_snap = calculate_snap_edge(wa, 1520.0, 390.0, 100.0, 48.0);
        assert_eq!(right_snap, Some(DockEdge::Right));

        // Near left edge: win_x = 10
        let left_snap = calculate_snap_edge(wa, 10.0, 390.0, 100.0, 48.0);
        assert_eq!(left_snap, Some(DockEdge::Left));

        // In the middle: win_x = 800
        let mid = calculate_snap_edge(wa, 800.0, 390.0, 100.0, 48.0);
        assert_eq!(mid, None);

        // Near right edge but too far above the monitor
        let too_high = calculate_snap_edge(wa, 1530.0, 390.0, -100.0, 48.0);
        assert_eq!(too_high, None);
    }
}
