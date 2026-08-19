// Windash — personal Windows system dashboard
// Rust backend: system metrics (sysinfo 0.32), persistent notes (JSON file in
// AppData), persisted settings, real Windows shell actions (Explorer / Search /
// process termination), and the Tauri app setup with system tray.

mod dock;
mod geom;
mod metrics;
mod notes;
mod settings;

use dock::DockStore;
use geom::GeomStore;
use metrics::SystemMetrics;
use notes::NotesStore;
use settings::SettingsStore;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
    image::Image,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

struct AppState {
    metrics: Mutex<SystemMetrics>,
    notes: Mutex<NotesStore>,
    dock: Mutex<DockStore>,
    geom: Mutex<GeomStore>,
    settings: Mutex<SettingsStore>,
}

pub fn run() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let metrics = SystemMetrics::new();
            let notes = NotesStore::new(app.handle())?;
            let dock_store = DockStore::new(app.handle())?;
            let geom_store = GeomStore::new(app.handle())?;
            let settings = SettingsStore::new(app.handle())?;
            let state = AppState {
                metrics: Mutex::new(metrics),
                notes: Mutex::new(notes),
                dock: Mutex::new(dock_store),
                geom: Mutex::new(geom_store),
                settings: Mutex::new(settings),
            };
            app.manage(state);

            // Global show/hide shortcut: Ctrl+Shift+D toggles the window.
            // Registered from Rust so it works even when the window is hidden
            // (the frontend cannot listen while hidden).
            if let Err(e) = app.global_shortcut().on_shortcut(
                "CmdOrCtrl+Shift+D",
                |app, _shortcut, event| {
                    // Only act on key press, not release (avoids double-toggle).
                    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                },
            ) {
                log::error!("failed to register global shortcut: {e}");
            }

            // Apply saved dock + always-on-top to the main window once it exists.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let cfg = app.state::<AppState>().dock.lock().unwrap().get();
                let _ = dock::apply_dock(&window, &cfg);
                let s = app.state::<AppState>().settings.lock().unwrap().get();
                let _ = window.set_always_on_top(s.always_on_top);
            }

            // System tray
            let quit_item = MenuItem::with_id(app, "quit", "Quit Windash", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide to Tray", true, None::<&str>)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&hide_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let icon_bytes = include_bytes!("../icons/icon.png");
            let icon = Image::from_bytes(icon_bytes).expect("icon.png parse");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("Windash — system dashboard")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Restore floating geometry (only meaningful when not docked).
            if let Some(window) = app.get_webview_window("main") {
                let dock_cfg = app.state::<AppState>().dock.lock().unwrap().get();
                if dock_cfg.edge() == dock::DockEdge::None {
                    let g = app.state::<AppState>().geom.lock().unwrap().get();
                    use tauri::{LogicalPosition, LogicalSize};
                    let _ = window.set_size(LogicalSize::new(g.width as f64, g.height as f64));
                    let _ = window.set_position(LogicalPosition::new(g.x as f64, g.y as f64));
                }
            }

            log::info!("Windash started with system tray");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            get_notes,
            add_note,
            delete_note,
            launch_app,
            get_dock,
            set_dock,
            apply_immersive,
            window_minimize,
            window_hide,
            open_explorer,
            windows_search,
            end_process,
            process_path,
            get_settings,
            update_settings,
            set_always_on_top,
            get_system_theme,
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_pos) => {
                // Only auto-dock while floating (a docked sidebar is not movable).
                let state = window.state::<AppState>();
                let edge = { state.dock.lock().unwrap().get().edge };
                if edge != "none" {
                    return;
                }
                // The event gives a `&Window`; get the `WebviewWindow` for the
                // docking helpers.
                let wv = match window.get_webview_window("main") {
                    Some(w) => w,
                    None => return,
                };
                // Persist floating geometry (position) on every move while floating.
                if let (Ok(pos), Ok(outer)) = (wv.outer_position(), wv.outer_size()) {
                    let g = geom::WindowGeom {
                        x: pos.x,
                        y: pos.y,
                        width: outer.width,
                        height: outer.height,
                    };
                    let _ = state.geom.lock().unwrap().set(&g);
                }
                if let Some(detected) = dock::detect_edge(&wv) {
                    let d = match detected {
                        dock::DockEdge::Left => "left".to_string(),
                        dock::DockEdge::Right => "right".to_string(),
                        dock::DockEdge::None => "none".to_string(),
                    };
                    // Snap + persist.
                    let store = state.dock.lock().unwrap();
                    let mut cfg = store.get();
                    cfg.edge = d.clone();
                    let _ = store.set(&cfg);
                    drop(store);
                    let _ = dock::apply_dock(&wv, &cfg);
                    // Tell the frontend the dock mode changed.
                    let _ = window.emit("windash://dock", d);
                }
            }
            tauri::WindowEvent::Resized(size) => {
                let scale = window.scale_factor().unwrap_or(1.0);
                let _ = window.emit(
                    "windash://resized",
                    serde_json::json!({
                        "width": size.width as f64 / scale,
                        "height": size.height as f64 / scale,
                    }),
                );
                // Persist floating geometry (size) when not docked.
                let st = window.state::<AppState>();
                if st.dock.lock().unwrap().get().edge() == dock::DockEdge::None {
                    if let (Ok(pos), Ok(outer)) =
                        (window.outer_position(), window.outer_size())
                    {
                        let g = geom::WindowGeom {
                            x: pos.x,
                            y: pos.y,
                            width: outer.width,
                            height: outer.height,
                        };
                        let _ = st.geom.lock().unwrap().set(&g);
                    }
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_metrics(out: tauri::State<'_, AppState>) -> Result<metrics::MetricsSnapshot, String> {
    let mut metrics = out.metrics.lock().map_err(|e| e.to_string())?;
    metrics.refresh();
    metrics.snapshot().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_notes(out: tauri::State<'_, AppState>) -> Result<Vec<notes::Note>, String> {
    let notes = out.notes.lock().map_err(|e| e.to_string())?;
    Ok(notes.list().map_err(|e| e.to_string())?)
}

#[tauri::command]
fn add_note(text: String, out: tauri::State<'_, AppState>) -> Result<notes::Note, String> {
    let notes = out.notes.lock().map_err(|e| e.to_string())?;
    notes.add(&text).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_note(id: u64, out: tauri::State<'_, AppState>) -> Result<(), String> {
    let notes = out.notes.lock().map_err(|e| e.to_string())?;
    notes.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn launch_app(app: AppHandle, target: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(target, None::<&str>)
        .map_err(|e| format!("open url: {}", e))
}

/// Open File Explorer. If `pid` is provided, open the folder containing that
/// process' executable (a real "open file location" action). Otherwise opens
/// the user's home directory. Uses the native Windows `explorer.exe` so it
/// behaves exactly like a real Windows utility.
#[tauri::command]
fn open_explorer(pid: Option<u64>) -> Result<(), String> {
    let target = match pid {
        Some(p) => {
            let sys = SystemMetrics::new();
            sys.exe_dir_for_pid(p)
                .ok_or_else(|| "Could not locate that process' executable.".to_string())?
        }
        None => directories_home().unwrap_or_else(|| "shell:MyComputerFolder".into()),
    };
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer.exe")
            .arg(target)
            .spawn()
            .map_err(|e| format!("open explorer: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target;
        Err("File Explorer is only available on Windows.".into())
    }
}

#[tauri::command]
fn windows_search(query: Option<String>) -> Result<(), String> {
    let target = match query {
        Some(q) if !q.trim().is_empty() => format!("search-ms:query={}", q.trim()),
        _ => "search-ms:".into(),
    };
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer.exe")
            .arg(target)
            .spawn()
            .map_err(|e| format!("windows search: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target;
        Err("Windows Search is only available on Windows.".into())
    }
}

/// Terminate a process by PID using the native Windows task-killer. Requires the
/// process to be terminable by the current user; permission failures surface as
/// a clear error rather than a silent no-op.
#[tauri::command]
fn end_process(pid: u64) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let out = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| format!("spawn taskkill: {}", e))?;
        if out.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!(
                "Unable to terminate process {}. Administrator permission may be required. {}",
                pid,
                stderr.trim()
            ))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = pid;
        Err("Process termination is only supported on Windows.".into())
    }
}

/// Return the directory containing a process' executable (for "open file
/// location"). Resolves the path through sysinfo.
#[tauri::command]
fn process_path(pid: u64) -> Result<String, String> {
    let sys = SystemMetrics::new();
    sys.exe_dir_for_pid(pid)
        .ok_or_else(|| "Process path unavailable.".into())
}

#[tauri::command]
fn get_settings(out: tauri::State<'_, AppState>) -> Result<settings::Settings, String> {
    Ok(out.settings.lock().map_err(|e| e.to_string())?.get())
}

#[tauri::command]
fn update_settings(
    patch: settings::SettingsPatch,
    out: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<settings::Settings, String> {
    let s = out
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .update(patch)
        .map_err(|e| e.to_string())?;
    // Apply side-effects immediately.
    if let Some(edge) = if s.dock_edge != "none" {
        Some(s.dock_edge.clone())
    } else {
        None
    } {
        let _ = dock::apply_dock(&window, &dock::DockConfig {
            edge: edge.clone(),
            width: 390,
            always_on_top: s.always_on_top,
        });
    }
    let _ = window.set_always_on_top(s.always_on_top);
    Ok(s)
}

#[tauri::command]
fn set_always_on_top(value: bool, out: tauri::State<'_, AppState>) -> Result<(), String> {
    out.settings
        .lock()
        .map_err(|e| e.to_string())?
        .update(settings::SettingsPatch {
            always_on_top: Some(value),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_system_theme() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let out = Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();
        if let Ok(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            // AppsUseLightTheme: 0x1 = light apps, 0x0 = dark apps.
            if let Some(idx) = text.find("0x") {
                let hex = &text[idx..];
                if hex.starts_with("0x1") {
                    return "light".to_string();
                } else if hex.starts_with("0x0") {
                    return "dark".to_string();
                }
            }
        }
        "dark".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "dark".to_string()
    }
}

/// Remove native window chrome and apply a frosted-glass blur. Must run after
/// the window is fully created (i.e. from the frontend after mount).
///
/// NOTE: use `apply_blur`, NOT `apply_acrylic`. On Windows 11 `apply_acrylic`
/// forces a `WS_CAPTION` (native titlebar) back onto the window.
#[tauri::command]
fn apply_immersive(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.set_decorations(false);
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::apply_blur;
        let _ = apply_blur(&window, Some((18, 18, 22, 255)));
    }
    Ok(())
}

#[tauri::command]
fn set_dock(
    edge: String,
    out: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<dock::DockConfig, String> {
    let store = out.dock.lock().map_err(|e| e.to_string())?;
    let mut cfg = store.get();
    cfg.edge = edge;
    store.set(&cfg)?;
    drop(store);
    dock::apply_dock(&window, &cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn get_dock(out: tauri::State<'_, AppState>) -> Result<dock::DockConfig, String> {
    Ok(out.dock.lock().map_err(|e| e.to_string())?.get())
}

#[tauri::command]
fn window_minimize(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn window_hide(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// --- small platform helpers ---

#[cfg(target_os = "windows")]
fn directories_home() -> Option<String> {
    std::env::var("USERPROFILE").ok()
}

#[cfg(not(target_os = "windows"))]
fn directories_home() -> Option<String> {
    std::env::var("HOME").ok()
}
