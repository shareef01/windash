// dock.rs — dockable window behaviour.
// The main window can dock to the left/right edge of the current monitor as a
// sidebar (always-on-top), or float freely. Dock state persists as JSON in AppData.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{LogicalPosition, LogicalSize, Manager, WebviewWindow};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    None,
    Left,
    Right,
}

impl DockEdge {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> DockEdge {
        match s {
            "left" => DockEdge::Left,
            "right" => DockEdge::Right,
            _ => DockEdge::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DockEdge::Left => "left",
            DockEdge::Right => "right",
            DockEdge::None => "none",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DockConfig {
    pub edge: DockEdge,
    pub width: u32,
    pub always_on_top: bool,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            edge: DockEdge::None,
            width: 390,
            always_on_top: true,
        }
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
            let default = DockConfig::default();
            crate::persist::atomic_write(
                &self.path,
                &serde_json::to_string_pretty(&default).map_err(|e| format!("serialize: {e}"))?,
            )?;
        }
        Ok(())
    }

    fn load(&self) -> Result<DockConfig, String> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DockConfig::default()),
            Err(e) => return Err(format!("read: {e}")),
        };

        match serde_json::from_str::<DockConfig>(&text) {
            Ok(cfg) => Ok(cfg),
            Err(_) => {
                log::warn!(
                    "dock file corrupted, creating backup: {}",
                    self.path.display()
                );
                crate::persist::backup_corrupt_file(&self.path);
                let cfg = DockConfig::default();
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
        self.load().unwrap_or_default()
    }

    pub fn set(&self, cfg: &DockConfig) -> Result<(), String> {
        self.save(cfg)
    }
}

/// Calculate the docked window rectangle against the monitor's WORK AREA (excluding taskbar).
pub fn calculate_docked_rect(work_area: Rect, edge: DockEdge, width: f64) -> Rect {
    let width = width.clamp(280.0, 640.0);
    match edge {
        DockEdge::None => work_area,
        DockEdge::Left => Rect {
            x: work_area.x,
            y: work_area.y,
            width,
            height: work_area.height,
        },
        DockEdge::Right => Rect {
            x: work_area.x + work_area.width - width,
            y: work_area.y,
            width,
            height: work_area.height,
        },
    }
}

/// Position the window according to the dock config using the monitor's WORK AREA.
/// Native window manipulation errors are propagated rather than silently swallowed.
pub fn apply_dock(window: &WebviewWindow, cfg: &DockConfig) -> Result<(), String> {
    let width = cfg.width.clamp(280, 640);

    // Prefer the window's current monitor, but fall back to the primary monitor.
    let monitor = if let Ok(Some(m)) = window.current_monitor() {
        m
    } else {
        window
            .primary_monitor()
            .ok()
            .flatten()
            .ok_or_else(|| "no monitor available".to_string())?
    };

    // Monitor geometry: work strictly in LOGICAL coordinates derived from the monitor's WORK AREA.
    // The work area excludes taskbars on any side (bottom, top, left, right).
    let scale = window.scale_factor().unwrap_or(1.0);
    let wa = monitor.work_area();
    let work_area = Rect {
        x: wa.position.x as f64 / scale,
        y: wa.position.y as f64 / scale,
        width: wa.size.width as f64 / scale,
        height: wa.size.height as f64 / scale,
    };

    let target_rect = calculate_docked_rect(work_area, cfg.edge, width as f64);

    match cfg.edge {
        DockEdge::None => {
            window
                .set_always_on_top(cfg.always_on_top)
                .map_err(|e| format!("set_always_on_top: {e}"))?;
            window
                .set_resizable(true)
                .map_err(|e| format!("set_resizable: {e}"))?;
            Ok(())
        }
        DockEdge::Left | DockEdge::Right => {
            window
                .set_resizable(false)
                .map_err(|e| format!("set_resizable: {e}"))?;
            window
                .set_size(LogicalSize::new(target_rect.width, target_rect.height))
                .map_err(|e| format!("set_size: {e}"))?;
            window
                .set_position(LogicalPosition::new(target_rect.x, target_rect.y))
                .map_err(|e| format!("set_position: {e}"))?;
            window
                .set_always_on_top(cfg.always_on_top)
                .map_err(|e| format!("set_always_on_top: {e}"))?;
            window
                .set_skip_taskbar(false)
                .map_err(|e| format!("set_skip_taskbar: {e}"))?;
            Ok(())
        }
    }
}

/// How close (logical px) the window's left/right edge must be to a monitor edge
/// before we treat a drag as an intentional dock gesture.
const SNAP_THRESHOLD: f64 = 48.0;

/// Geometry rectangle in logical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
/// close enough to any edge.
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
    fn test_dock_edge_serde_valid() {
        assert_eq!(
            serde_json::from_str::<DockEdge>("\"left\"").unwrap(),
            DockEdge::Left
        );
        assert_eq!(
            serde_json::from_str::<DockEdge>("\"right\"").unwrap(),
            DockEdge::Right
        );
        assert_eq!(
            serde_json::from_str::<DockEdge>("\"none\"").unwrap(),
            DockEdge::None
        );
    }

    #[test]
    fn test_dock_edge_serde_invalid_rejected() {
        assert!(serde_json::from_str::<DockEdge>("\"top\"").is_err());
        assert!(serde_json::from_str::<DockEdge>("\"invalid\"").is_err());
    }

    #[test]
    fn test_dock_config_canonical_default() {
        let cfg = DockConfig::default();
        assert_eq!(cfg.edge, DockEdge::None);
        assert_eq!(cfg.width, 390);
        assert!(cfg.always_on_top);
    }

    #[test]
    fn test_calculate_docked_rect_bottom_taskbar() {
        // Monitor 1920x1080 with 40px taskbar at the bottom
        let wa = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        let right_dock = calculate_docked_rect(wa, DockEdge::Right, 390.0);
        assert_eq!(right_dock.x, 1920.0 - 390.0);
        assert_eq!(right_dock.y, 0.0);
        assert_eq!(right_dock.width, 390.0);
        assert_eq!(right_dock.height, 1040.0);

        let left_dock = calculate_docked_rect(wa, DockEdge::Left, 390.0);
        assert_eq!(left_dock.x, 0.0);
        assert_eq!(left_dock.y, 0.0);
        assert_eq!(left_dock.width, 390.0);
        assert_eq!(left_dock.height, 1040.0);
    }

    #[test]
    fn test_calculate_docked_rect_top_taskbar() {
        // Monitor 1920x1080 with 40px taskbar at the top
        let wa = Rect {
            x: 0.0,
            y: 40.0,
            width: 1920.0,
            height: 1040.0,
        };
        let right_dock = calculate_docked_rect(wa, DockEdge::Right, 390.0);
        assert_eq!(right_dock.x, 1530.0);
        assert_eq!(right_dock.y, 40.0);
        assert_eq!(right_dock.height, 1040.0);
    }

    #[test]
    fn test_calculate_docked_rect_left_and_right_taskbars() {
        // Left taskbar 60px wide
        let wa_left = Rect {
            x: 60.0,
            y: 0.0,
            width: 1860.0,
            height: 1080.0,
        };
        let dock_left = calculate_docked_rect(wa_left, DockEdge::Left, 390.0);
        assert_eq!(dock_left.x, 60.0);
        assert_eq!(dock_left.width, 390.0);

        // Right taskbar 60px wide
        let wa_right = Rect {
            x: 0.0,
            y: 0.0,
            width: 1860.0,
            height: 1080.0,
        };
        let dock_right = calculate_docked_rect(wa_right, DockEdge::Right, 390.0);
        assert_eq!(dock_right.x, 1860.0 - 390.0);
    }

    #[test]
    fn test_calculate_docked_rect_negative_monitor_origin() {
        // Secondary monitor left of primary at (-1920, 0)
        let wa = Rect {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        let right_dock = calculate_docked_rect(wa, DockEdge::Right, 390.0);
        assert_eq!(right_dock.x, -1920.0 + 1920.0 - 390.0); // -390.0
        assert_eq!(right_dock.y, 0.0);
        assert_eq!(right_dock.height, 1040.0);

        let left_dock = calculate_docked_rect(wa, DockEdge::Left, 390.0);
        assert_eq!(left_dock.x, -1920.0);
        assert_eq!(left_dock.y, 0.0);
        assert_eq!(left_dock.height, 1040.0);
    }

    #[test]
    fn test_calculate_snap_edge() {
        let wa = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let right_snap = calculate_snap_edge(wa, 1520.0, 390.0, 100.0, 48.0);
        assert_eq!(right_snap, Some(DockEdge::Right));

        let left_snap = calculate_snap_edge(wa, 10.0, 390.0, 100.0, 48.0);
        assert_eq!(left_snap, Some(DockEdge::Left));

        let mid = calculate_snap_edge(wa, 800.0, 390.0, 100.0, 48.0);
        assert_eq!(mid, None);

        let too_high = calculate_snap_edge(wa, 1530.0, 390.0, -100.0, 48.0);
        assert_eq!(too_high, None);
    }
}
