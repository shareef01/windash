// Windash — personal Windows system dashboard
// Rust backend: system metrics (sysinfo 0.32), persistent notes (JSON file in
// AppData), persisted settings, real Windows shell actions (Explorer / Search /
// process termination), and the Tauri app setup with system tray.

mod dock;
mod geom;
mod metrics;
mod notes;
mod persist;
mod settings;

use dock::DockStore;
use geom::GeomStore;
use metrics::SystemMetrics;
use notes::NotesStore;
use settings::SettingsStore;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

struct AppState {
    metrics: Mutex<SystemMetrics>,
    notes: Mutex<NotesStore>,
    dock: Mutex<DockStore>,
    geom: Mutex<GeomStore>,
    settings: Mutex<SettingsStore>,
    #[allow(dead_code)]
    tray: Mutex<Option<tauri::tray::TrayIcon>>,
}

pub fn run() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch must NOT start another monitoring loop — just
            // bring the existing window to the front. This prevents multiple
            // sysinfo pollers from running at once (which would spike CPU).
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let metrics = SystemMetrics::new();
            let notes = NotesStore::new(app.handle())?;
            let dock_store = DockStore::new(app.handle())?;
            let geom_store = GeomStore::new(app.handle())?;
            let settings = SettingsStore::new(app.handle())?;
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

            let tray = TrayIconBuilder::with_id("windash-tray")
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Windash — system dashboard")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
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
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let state = AppState {
                metrics: Mutex::new(metrics),
                notes: Mutex::new(notes),
                dock: Mutex::new(dock_store),
                geom: Mutex::new(geom_store),
                settings: Mutex::new(settings),
                tray: Mutex::new(Some(tray)),
            };
            app.manage(state);

            // Global show/hide shortcut: Ctrl+Shift+D toggles the window.
            // Registered from Rust so it works even when the window is hidden
            // (the frontend cannot listen while hidden).
            if let Err(e) =
                app.global_shortcut()
                    .on_shortcut("CmdOrCtrl+Shift+D", |app, _shortcut, event| {
                        // Only act on key press, not release (avoids double-toggle).
                        if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            return;
                        }
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    })
            {
                log::error!("failed to register global shortcut: {e}");
            }

            // Apply saved dock + always-on-top to the main window once it exists.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
                if let Ok(dock_guard) = app.state::<AppState>().dock.lock() {
                    let cfg = dock_guard.get();
                    drop(dock_guard);
                    let _ = dock::apply_dock(&window, &cfg);
                }
                if let Ok(settings_guard) = app.state::<AppState>().settings.lock() {
                    let s = settings_guard.get();
                    drop(settings_guard);
                    let _ = window.set_always_on_top(s.always_on_top);
                }
            }

            // Restore floating geometry (only meaningful when not docked).
            if let Some(window) = app.get_webview_window("main") {
                let dock_edge = app
                    .state::<AppState>()
                    .dock
                    .lock()
                    .map(|d| d.get().edge())
                    .unwrap_or(dock::DockEdge::None);
                if dock_edge == dock::DockEdge::None {
                    if let Ok(geom_guard) = app.state::<AppState>().geom.lock() {
                        let g = geom_guard.get("none");
                        drop(geom_guard);
                        use tauri::{LogicalPosition, LogicalSize};
                        let _ = window.set_size(LogicalSize::new(g.width as f64, g.height as f64));
                        let _ = window.set_position(LogicalPosition::new(g.x as f64, g.y as f64));
                        // Tell the frontend the actual window width so the responsive
                        // layout mode matches the restored geometry.
                        let _ = window.emit(
                            "windash://resized",
                            serde_json::json!({
                                "width": g.width as f64,
                                "height": g.height as f64,
                            }),
                        );
                    }
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
            get_dock,
            set_dock,
            apply_immersive,
            window_minimize,
            window_hide,
            open_explorer,
            windows_search,
            end_process,
            get_settings,
            update_settings,
            set_always_on_top,
            get_system_theme,
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_pos) => {
                let state = window.state::<AppState>();
                let edge = match state.dock.lock() {
                    Ok(guard) => guard.get().edge,
                    Err(_) => return,
                };
                // The event gives a `&Window`; get the `WebviewWindow` for the
                // docking helpers.
                let wv = match window.get_webview_window("main") {
                    Some(w) => w,
                    None => return,
                };
                if edge != "none" {
                    // Docked: re-snap to the *current* monitor so dragging the
                    // window (or moving it across mixed-DPI displays) keeps it
                    // glued to the correct edge at the right scale.
                    if let Ok(guard) = state.dock.lock() {
                        let cfg = guard.get();
                        let _ = dock::apply_dock(&wv, &cfg);
                    }
                    return;
                }
                // Floating: persist geometry (keyed by edge) on every move.
                if let (Ok(pos), Ok(outer)) = (wv.outer_position(), wv.outer_size()) {
                    let scale = wv.scale_factor().unwrap_or(1.0);
                    let g = geom::WindowGeom {
                        x: (pos.x as f64 / scale) as i32,
                        y: (pos.y as f64 / scale) as i32,
                        width: (outer.width as f64 / scale) as u32,
                        height: (outer.height as f64 / scale) as u32,
                    };
                    if let Ok(geom_guard) = state.geom.lock() {
                        let _ = geom_guard.set("none", &g);
                    }
                }
                if let Some(detected) = dock::detect_edge(&wv) {
                    let d = match detected {
                        dock::DockEdge::Left => "left".to_string(),
                        dock::DockEdge::Right => "right".to_string(),
                        dock::DockEdge::None => "none".to_string(),
                    };
                    // Snap + persist.
                    if let Ok(store) = state.dock.lock() {
                        let mut cfg = store.get();
                        cfg.edge = d.clone();
                        let _ = store.set(&cfg);
                        drop(store);
                        let _ = dock::apply_dock(&wv, &cfg);
                        // Tell the frontend the dock mode changed.
                        let _ = window.emit("windash://dock", d);
                    }
                }
            }
            tauri::WindowEvent::Focused(focused) => {
                // When the window regains focus, tell the frontend to refresh
                // the OS theme (so "Follow Windows" stays in sync without
                // polling `reg` on a timer — which would flash consoles).
                if *focused {
                    let _ = window.emit("windash://focused", ());
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
                let is_floating = st
                    .dock
                    .lock()
                    .map(|d| d.get().edge() == dock::DockEdge::None)
                    .unwrap_or(false);
                if is_floating {
                    if let (Ok(pos), Ok(outer)) = (window.outer_position(), window.outer_size()) {
                        let g = geom::WindowGeom {
                            x: (pos.x as f64 / scale) as i32,
                            y: (pos.y as f64 / scale) as i32,
                            width: (outer.width as f64 / scale) as u32,
                            height: (outer.height as f64 / scale) as u32,
                        };
                        if let Ok(geom_guard) = st.geom.lock() {
                            let _ = geom_guard.set("none", &g);
                        }
                    }
                }
            }
            // DPI / scale-factor or theme change (e.g. moving across monitors, or
            // the user changes the display scaling): re-snap a docked sidebar to
            // the current monitor so it stays correctly placed and sized.
            tauri::WindowEvent::ScaleFactorChanged { .. } | tauri::WindowEvent::ThemeChanged(_) => {
                let state = window.state::<AppState>();
                let is_docked = state
                    .dock
                    .lock()
                    .map(|d| d.get().edge() != dock::DockEdge::None)
                    .unwrap_or(false);
                if is_docked {
                    if let Some(wv) = window.get_webview_window("main") {
                        if let Ok(dock_guard) = state.dock.lock() {
                            let cfg = dock_guard.get();
                            let _ = dock::apply_dock(&wv, &cfg);
                        }
                    }
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_metrics(
    sort_by: Option<String>,
    out: tauri::State<'_, AppState>,
) -> Result<metrics::MetricsSnapshot, String> {
    let mut metrics = out.metrics.lock().map_err(|e| e.to_string())?;
    metrics.refresh();
    let sort = match sort_by.as_deref() {
        Some("mem") => metrics::SortKey::Mem,
        Some("name") => metrics::SortKey::Name,
        _ => metrics::SortKey::Cpu,
    };
    metrics.snapshot(sort).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_notes(out: tauri::State<'_, AppState>) -> Result<Vec<notes::Note>, String> {
    let notes = out.notes.lock().map_err(|e| e.to_string())?;
    notes.list().map_err(|e| e.to_string())
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

/// Open File Explorer. If `pid` is provided, open the folder containing that
/// process' executable (a real "open file location" action). Otherwise opens
/// the user's home directory. Uses the native Windows `explorer.exe` so it
/// behaves exactly like a real Windows utility.
#[tauri::command]
fn open_explorer(pid: Option<u64>, out: tauri::State<'_, AppState>) -> Result<(), String> {
    let target = match pid {
        Some(p) => {
            // Reuse the shared System instance (never build a fresh one) so we
            // don't disturb the CPU sampling interval.
            let metrics = out.metrics.lock().map_err(|e| e.to_string())?;
            metrics
                .exe_dir_for_pid(p)
                .ok_or_else(|| "Could not locate that process' executable.".to_string())?
        }
        None => directories_home().unwrap_or_else(|| "shell:MyComputerFolder".into()),
    };
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // CREATE_NO_WINDOW (0x08000000) keeps Explorer from flashing a console.
        Command::new("explorer.exe")
            .arg(target)
            .creation_flags(0x0800_0000)
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
        Some(q) => {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                // Strip control characters and double quotes to prevent parameter confusion
                let clean: String = trimmed
                    .chars()
                    .filter(|c| !c.is_control() && *c != '"')
                    .collect();
                format!("search-ms:query={}", clean)
            } else {
                "search-ms:".into()
            }
        }
        None => "search-ms:".into(),
    };
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // CREATE_NO_WINDOW (0x08000000) keeps Search from flashing a console.
        Command::new("explorer.exe")
            .arg(target)
            .creation_flags(0x0800_0000)
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
    // Guard against terminating critical system PIDs or own process
    let current_pid = std::process::id() as u64;
    if pid == 0 || pid == 4 {
        return Err(format!(
            "Process {} is a protected Windows system process and cannot be terminated.",
            pid
        ));
    }
    if pid == current_pid {
        return Err("Cannot terminate Windash process from within itself.".into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // CREATE_NO_WINDOW (0x08000000) avoids flashing a console on every kill.
        let out = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .creation_flags(0x0800_0000)
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
    // Apply the always-on-top side-effect immediately, and keep dock config in sync
    // so a later snap/dock doesn't restore a stale always-on-top flag.
    let _ = window.set_always_on_top(s.always_on_top);
    if let Ok(dock) = out.dock.lock() {
        let mut cfg = dock.get();
        cfg.always_on_top = s.always_on_top;
        let _ = dock.set(&cfg);
    }
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
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // CREATE_NO_WINDOW (0x08000000) avoids flashing a console window.
        let out = Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .creation_flags(0x0800_0000)
            .output();
        if let Ok(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            // Value looks like:  ...    0x1\n
            // Parse the hex value after "0x" robustly.
            if let Some(idx) = text.find("0x") {
                if let Ok(val) = u32::from_str_radix(
                    text[idx + 2..].split_whitespace().next().unwrap_or("0"),
                    16,
                ) {
                    return if val == 0 {
                        "dark".to_string()
                    } else {
                        "light".to_string()
                    };
                }
            }
        }
        // Default to dark if the registry read fails for any reason.
        "dark".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "dark".to_string()
    }
}

/// Remove native window chrome and apply a Windows 11 backdrop when enabled.
/// `dark` should match the effective UI theme so Mica is not forced to dark
/// while the webview is in light mode. If Mica is unavailable, skip blur —
/// CSS surfaces are more reliable than a hard-coded dark acrylic tint.
#[tauri::command]
fn apply_immersive(
    dark: Option<bool>,
    window: tauri::WebviewWindow,
    out: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _ = window.set_decorations(false);
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::apply_mica;
        let mica = out
            .settings
            .lock()
            .map_err(|e| e.to_string())?
            .get()
            .mica_enabled;
        if mica {
            let _ = apply_mica(&window, Some(dark.unwrap_or(true)));
        }
    }
    Ok(())
}

#[tauri::command]
fn set_dock(
    edge: String,
    out: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<dock::DockConfig, String> {
    let always_on_top = out
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .get()
        .always_on_top;
    let store = out.dock.lock().map_err(|e| e.to_string())?;
    let mut cfg = store.get();
    cfg.edge = edge;
    cfg.always_on_top = always_on_top;
    store.set(&cfg)?;
    drop(store);
    dock::apply_dock(&window, &cfg)?;

    // When un-docking, restore the saved floating geometry (size + position)
    // so the window doesn't stay a full-height sidebar shape.
    if cfg.edge() == dock::DockEdge::None {
        let g = out.geom.lock().map_err(|e| e.to_string())?.get("none");
        use tauri::{LogicalPosition, LogicalSize};
        let _ = window.set_size(LogicalSize::new(g.width as f64, g.height as f64));
        let _ = window.set_position(LogicalPosition::new(g.x as f64, g.y as f64));
        let _ = window.emit(
            "windash://resized",
            serde_json::json!({ "width": g.width as f64, "height": g.height as f64 }),
        );
    }
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
