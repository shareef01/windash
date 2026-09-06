// geom.rs — persistence of floating-window geometry (position + size), keyed by
// dock edge. Remembers the user's floating position/size per edge across restarts.
// Supports multi-monitor Windows setups with negative coordinates, mixed DPI, and ultrawide/5K displays.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Debug)]
pub struct WindowGeom {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Determines whether a saved window rectangle meaningfully intersects a monitor work area.
/// Meaningful intersection requires sufficient overlap (at least 64px or 25% of the window)
/// in both dimensions so a window isn't restored off-screen.
pub fn rect_intersects_work_area(win: Rect, work_area: Rect) -> bool {
    let overlap_x = (win.x + win.width).min(work_area.x + work_area.width) - win.x.max(work_area.x);
    let overlap_y =
        (win.y + win.height).min(work_area.y + work_area.height) - win.y.max(work_area.y);
    let min_w = 64.0_f64.min(win.width * 0.25);
    let min_h = 64.0_f64.min(win.height * 0.25);
    overlap_x >= min_w && overlap_y >= min_h
}

/// Resolve window placement across current monitors. If the saved rectangle intersects
/// at least one active monitor work area, it is restored. If the monitor was disconnected,
/// the window is relocated to a sensible visible position on the primary monitor.
pub fn resolve_window_placement(saved: Rect, monitors: &[Rect], primary: Rect) -> Rect {
    let visible = monitors
        .iter()
        .any(|&m| rect_intersects_work_area(saved, m));
    if visible {
        saved
    } else {
        let safe_w = saved.width.clamp(280.0, primary.width.max(280.0));
        let safe_h = saved.height.clamp(440.0, primary.height.max(440.0));
        let safe_x = (primary.x + 120.0)
            .min(primary.x + primary.width - safe_w)
            .max(primary.x);
        let safe_y = (primary.y + 80.0)
            .min(primary.y + primary.height - safe_h)
            .max(primary.y);
        Rect {
            x: safe_x,
            y: safe_y,
            width: safe_w,
            height: safe_h,
        }
    }
}

type GeomMap = HashMap<String, WindowGeom>;

enum PersistMsg {
    Update(GeomMap),
    Flush(Sender<()>),
    Stop,
}

pub struct GeomStore {
    cache: Arc<Mutex<GeomMap>>,
    tx: Option<Sender<PersistMsg>>,
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl GeomStore {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        use tauri::Manager;
        let path = app
            .path()
            .resolve("windash-geom.json", tauri::path::BaseDirectory::AppData)
            .map_err(|e| format!("path resolve: {}", e))?;
        let store = Self::from_path(path)?;
        Ok(store)
    }

    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;

        let initial_map = Self::load_file(&path);
        let cache = Arc::new(Mutex::new(initial_map));

        let (tx, rx) = channel::<PersistMsg>();
        let worker_path = path.clone();
        let worker_handle = thread::Builder::new()
            .name("windash-geom-persist".into())
            .spawn(move || {
                let debounce_dur = Duration::from_millis(400);
                let mut pending_map: Option<GeomMap> = None;
                let mut last_event: Option<Instant> = None;

                loop {
                    let timeout = if pending_map.is_some() {
                        let elapsed = last_event.map(|t| t.elapsed()).unwrap_or_default();
                        debounce_dur.saturating_sub(elapsed)
                    } else {
                        Duration::from_secs(3600)
                    };

                    match rx.recv_timeout(timeout) {
                        Ok(PersistMsg::Update(map)) => {
                            pending_map = Some(map);
                            last_event = Some(Instant::now());
                        }
                        Ok(PersistMsg::Flush(ack)) => {
                            if let Some(map) = pending_map.take() {
                                Self::write_file(&worker_path, &map);
                            }
                            let _ = ack.send(());
                        }
                        Ok(PersistMsg::Stop) => {
                            if let Some(map) = pending_map.take() {
                                Self::write_file(&worker_path, &map);
                            }
                            break;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(map) = pending_map.take() {
                                Self::write_file(&worker_path, &map);
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            if let Some(map) = pending_map.take() {
                                Self::write_file(&worker_path, &map);
                            }
                            break;
                        }
                    }
                }
            })
            .map_err(|e| format!("spawn persist worker: {e}"))?;

        Ok(Self {
            cache,
            tx: Some(tx),
            worker_handle: Some(worker_handle),
        })
    }

    fn default_map() -> GeomMap {
        let mut map = HashMap::new();
        map.insert(
            "none".into(),
            WindowGeom {
                x: 200,
                y: 100,
                width: 390,
                height: 720,
            },
        );
        map
    }

    fn load_file(path: &Path) -> GeomMap {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => {
                let map = Self::default_map();
                Self::write_file(path, &map);
                return map;
            }
        };

        if let Ok(map) = serde_json::from_str::<GeomMap>(&text) {
            return map;
        }

        // Legacy format migration: single {x, y, width, height}
        if let Ok(g) = serde_json::from_str::<WindowGeom>(&text) {
            let mut map = HashMap::new();
            map.insert("none".into(), g);
            Self::write_file(path, &map);
            return map;
        }

        log::warn!("geom file corrupted, backing up: {}", path.display());
        crate::persist::backup_corrupt_file(path);
        let map = Self::default_map();
        Self::write_file(path, &map);
        map
    }

    fn write_file(path: &Path, map: &GeomMap) {
        if let Ok(json) = serde_json::to_string_pretty(map) {
            let _ = crate::persist::atomic_write(path, &json);
        }
    }

    pub fn get(&self, edge: &str) -> WindowGeom {
        let map = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut g = if let Some(g) = map.get(edge) {
            *g
        } else if let Some(g) = map.get("none") {
            *g
        } else {
            WindowGeom {
                x: 200,
                y: 100,
                width: 390,
                height: 720,
            }
        };
        // Clamp only width and height to safe UI limits; do not clamp signed x/y coordinates
        g.width = g.width.clamp(280, 1920);
        g.height = g.height.clamp(440, 1440);
        g
    }

    /// Update floating geometry in-memory immediately and schedule debounced disk write.
    pub fn set(&self, edge: &str, geom: &WindowGeom) -> Result<(), String> {
        let sanitized = WindowGeom {
            x: geom.x, // preserved signed as observed
            y: geom.y, // preserved signed as observed
            width: geom.width.clamp(280, 1920),
            height: geom.height.clamp(440, 1440),
        };

        let mut map = self.cache.lock().map_err(|e| e.to_string())?;
        if map.get(edge) == Some(&sanitized) {
            return Ok(());
        }
        map.insert(edge.to_string(), sanitized);

        if let Some(tx) = &self.tx {
            let _ = tx.send(PersistMsg::Update(map.clone()));
        }
        Ok(())
    }

    pub fn flush(&self) {
        if let Some(tx) = &self.tx {
            let (ack_tx, ack_rx) = channel();
            if tx.send(PersistMsg::Flush(ack_tx)).is_ok() {
                let _ = ack_rx.recv_timeout(Duration::from_millis(500));
            }
        }
    }
}

impl Drop for GeomStore {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(PersistMsg::Stop);
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
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
    fn test_rect_intersects_work_area() {
        let wa = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };

        // Fully inside
        let inside = Rect {
            x: 100.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        assert!(rect_intersects_work_area(inside, wa));

        // Meaningfully overlapping right edge
        let partial_right = Rect {
            x: 1800.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        assert!(rect_intersects_work_area(partial_right, wa));

        // Meaningfully overlapping left edge
        let partial_left = Rect {
            x: -200.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        assert!(rect_intersects_work_area(partial_left, wa));

        // Barely 10px overlap (insufficient)
        let barely = Rect {
            x: 1915.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        assert!(!rect_intersects_work_area(barely, wa));

        // Completely outside
        let outside = Rect {
            x: 3000.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        assert!(!rect_intersects_work_area(outside, wa));
    }

    #[test]
    fn test_resolve_window_placement_multi_monitor() {
        let primary = Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1040.0,
        };
        let left_monitor = Rect {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let monitors = [primary, left_monitor];

        // Saved on left monitor
        let saved_left = Rect {
            x: -1500.0,
            y: 100.0,
            width: 390.0,
            height: 720.0,
        };
        let res = resolve_window_placement(saved_left, &monitors, primary);
        assert_eq!(res, saved_left);

        // Saved on disconnected monitor -> falls back to primary
        let saved_disconnected = Rect {
            x: 4000.0,
            y: 200.0,
            width: 390.0,
            height: 720.0,
        };
        let res_fallback = resolve_window_placement(saved_disconnected, &monitors, primary);
        assert!(res_fallback.x >= primary.x);
        assert!(res_fallback.x + res_fallback.width <= primary.x + primary.width);
        assert!(res_fallback.y >= primary.y);
        assert!(res_fallback.y + res_fallback.height <= primary.y + primary.height);
    }

    #[test]
    fn test_resolve_window_placement_5k_ultrawide() {
        let ultrawide = Rect {
            x: 0.0,
            y: 0.0,
            width: 5120.0,
            height: 1400.0,
        };
        let monitors = [ultrawide];
        let saved_5k = Rect {
            x: 4500.0,
            y: 300.0,
            width: 400.0,
            height: 800.0,
        };
        let res = resolve_window_placement(saved_5k, &monitors, ultrawide);
        assert_eq!(res, saved_5k);
    }

    #[test]
    fn test_signed_coordinates_survive() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("test_geom_{nonce}.json"));
        let store = GeomStore::from_path(path.clone()).unwrap();

        let neg_geom = WindowGeom {
            x: -1200,
            y: -500,
            width: 390,
            height: 720,
        };
        store.set("none", &neg_geom).unwrap();

        let retrieved = store.get("none");
        assert_eq!(retrieved.x, -1200);
        assert_eq!(retrieved.y, -500);

        store.flush();
        let _ = fs::remove_file(&path);
    }
}
