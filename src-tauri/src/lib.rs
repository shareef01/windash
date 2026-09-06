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
                        if let Ok(geom) = app.state::<AppState>().geom.lock() {
                            geom.flush();
                        }
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
                    .map(|d| d.get().edge)
                    .unwrap_or(dock::DockEdge::None);
                if dock_edge == dock::DockEdge::None {
                    if let Ok(geom_guard) = app.state::<AppState>().geom.lock() {
                        let g = geom_guard.get("none");
                        drop(geom_guard);

                        let scale = window.scale_factor().unwrap_or(1.0);
                        let monitors: Vec<geom::Rect> = window
                            .available_monitors()
                            .unwrap_or_default()
                            .iter()
                            .map(|m| {
                                let wa = m.work_area();
                                geom::Rect {
                                    x: wa.position.x as f64 / scale,
                                    y: wa.position.y as f64 / scale,
                                    width: wa.size.width as f64 / scale,
                                    height: wa.size.height as f64 / scale,
                                }
                            })
                            .collect();

                        let primary_rect = window
                            .primary_monitor()
                            .ok()
                            .flatten()
                            .map(|m| {
                                let wa = m.work_area();
                                geom::Rect {
                                    x: wa.position.x as f64 / scale,
                                    y: wa.position.y as f64 / scale,
                                    width: wa.size.width as f64 / scale,
                                    height: wa.size.height as f64 / scale,
                                }
                            })
                            .unwrap_or(geom::Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 1920.0,
                                height: 1040.0,
                            });

                        let saved_rect = geom::Rect {
                            x: g.x as f64,
                            y: g.y as f64,
                            width: g.width as f64,
                            height: g.height as f64,
                        };

                        let resolved =
                            geom::resolve_window_placement(saved_rect, &monitors, primary_rect);

                        use tauri::{LogicalPosition, LogicalSize};
                        let _ = window.set_size(LogicalSize::new(resolved.width, resolved.height));
                        let _ = window.set_position(LogicalPosition::new(resolved.x, resolved.y));
                        // Tell the frontend the actual window width so the responsive
                        // layout mode matches the restored geometry.
                        let _ = window.emit(
                            "windash://resized",
                            serde_json::json!({
                                "width": resolved.width,
                                "height": resolved.height,
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
            get_system_theme,
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Moved(_pos) => {
                let state = window.state::<AppState>();
                let edge = match state.dock.lock() {
                    Ok(guard) => guard.get().edge,
                    Err(_) => return,
                };
                let wv = match window.get_webview_window("main") {
                    Some(w) => w,
                    None => return,
                };
                if edge != dock::DockEdge::None {
                    // Docked: re-snap to the current monitor if geometry deviated,
                    // but skip if window is already at target rect to avoid feedback loops.
                    if let Ok(guard) = state.dock.lock() {
                        let cfg = guard.get();
                        drop(guard);
                        if let Ok(Some(monitor)) = wv.current_monitor() {
                            let scale = wv.scale_factor().unwrap_or(1.0);
                            let wa = monitor.work_area();
                            let work_area = dock::Rect {
                                x: wa.position.x as f64 / scale,
                                y: wa.position.y as f64 / scale,
                                width: wa.size.width as f64 / scale,
                                height: wa.size.height as f64 / scale,
                            };
                            let target =
                                dock::calculate_docked_rect(work_area, cfg.edge, cfg.width as f64);
                            if let (Ok(curr_pos), Ok(curr_size)) =
                                (wv.outer_position(), wv.outer_size())
                            {
                                let curr_x = curr_pos.x as f64 / scale;
                                let curr_y = curr_pos.y as f64 / scale;
                                let curr_w = curr_size.width as f64 / scale;
                                let curr_h = curr_size.height as f64 / scale;
                                if (curr_x - target.x).abs() < 2.0
                                    && (curr_y - target.y).abs() < 2.0
                                    && (curr_w - target.width).abs() < 2.0
                                    && (curr_h - target.height).abs() < 2.0
                                {
                                    return;
                                }
                            }
                        }
                        let _ = dock::apply_dock(&wv, &cfg);
                    }
                    return;
                }
                // Floating: persist geometry (debounced) on every move.
                if let (Ok(pos), Ok(outer)) = (wv.outer_position(), wv.outer_size()) {
                    let scale = wv.scale_factor().unwrap_or(1.0);
                    let g = geom::WindowGeom {
                        x: (pos.x as f64 / scale).round() as i32,
                        y: (pos.y as f64 / scale).round() as i32,
                        width: (outer.width as f64 / scale).round() as u32,
                        height: (outer.height as f64 / scale).round() as u32,
                    };
                    if let Ok(geom_guard) = state.geom.lock() {
                        let _ = geom_guard.set("none", &g);
                    }
                }
                if let Some(detected) = dock::detect_edge(&wv) {
                    if detected != dock::DockEdge::None {
                        if let Ok(store) = state.dock.lock() {
                            let mut cfg = store.get();
                            cfg.edge = detected;
                            let _ = store.set(&cfg);
                            drop(store);
                            let _ = dock::apply_dock(&wv, &cfg);
                            let _ = window.emit("windash://dock", detected.as_str());
                        }
                    }
                }
            }
            tauri::WindowEvent::Focused(focused) => {
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
                    .map(|d| d.get().edge == dock::DockEdge::None)
                    .unwrap_or(false);
                if is_floating {
                    if let (Ok(pos), Ok(outer)) = (window.outer_position(), window.outer_size()) {
                        let g = geom::WindowGeom {
                            x: (pos.x as f64 / scale).round() as i32,
                            y: (pos.y as f64 / scale).round() as i32,
                            width: (outer.width as f64 / scale).round() as u32,
                            height: (outer.height as f64 / scale).round() as u32,
                        };
                        if let Ok(geom_guard) = st.geom.lock() {
                            let _ = geom_guard.set("none", &g);
                        }
                    }
                }
            }
            tauri::WindowEvent::ScaleFactorChanged { .. } => {
                let state = window.state::<AppState>();
                let is_docked = state
                    .dock
                    .lock()
                    .map(|d| d.get().edge != dock::DockEdge::None)
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
            tauri::WindowEvent::ThemeChanged(theme) => {
                let theme_str = match theme {
                    tauri::Theme::Light => "light",
                    _ => "dark",
                };
                let _ = window.emit("windash://theme", theme_str);
                let state = window.state::<AppState>();
                let is_docked = state
                    .dock
                    .lock()
                    .map(|d| d.get().edge != dock::DockEdge::None)
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
            tauri::WindowEvent::Destroyed | tauri::WindowEvent::CloseRequested { .. } => {
                if let Ok(geom) = window.state::<AppState>().geom.lock() {
                    geom.flush();
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

/// Builds a safe `search-ms:` URI.
/// Strips control characters, quotes, and newlines to avoid parameter confusion,
/// and URL-percent-encodes the query parameter according to RFC 3986.
pub fn build_search_ms_uri(query: Option<&str>) -> String {
    let q = match query {
        Some(s) => s,
        None => return "search-ms:".to_string(),
    };
    // Strip control characters, quotes, and newlines
    let clean: String = q
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != '\'')
        .collect();
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        return "search-ms:".to_string();
    }

    // RFC 3986 percent encoding for query string
    let mut encoded = String::with_capacity(trimmed.len() * 3);
    for b in trimmed.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*b as char);
            }
            other => {
                use std::fmt::Write;
                let _ = write!(&mut encoded, "%{:02X}", other);
            }
        }
    }
    format!("search-ms:query={}", encoded)
}

#[tauri::command]
fn windows_search(query: Option<String>) -> Result<(), String> {
    let target = build_search_ms_uri(query.as_deref());
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

/// Terminate a process by PID using the native Windows task-killer.
/// Re-validates the process identity (PID, start time, name) immediately prior
/// to termination to ensure PID reuse / stale UI state cannot cause an unintended
/// process to be killed.
#[tauri::command]
fn end_process(
    pid: u64,
    expected_start_time: u64,
    expected_name: Option<String>,
    out: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let current_pid = std::process::id() as u64;
    let live_proc = {
        let mut m = out.metrics.lock().map_err(|e| e.to_string())?;
        m.process_identity(pid)
    };

    metrics::validate_process_termination(
        pid,
        expected_start_time,
        expected_name.as_deref(),
        current_pid,
        live_proc.as_ref(),
    )?;

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
/// while the webview is in light mode. If Mica is disabled, clear it cleanly.
#[tauri::command]
fn apply_immersive(
    dark: Option<bool>,
    window: tauri::WebviewWindow,
    out: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _ = window.set_decorations(false);
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_mica, clear_mica};
        let mica = out
            .settings
            .lock()
            .map_err(|e| e.to_string())?
            .get()
            .mica_enabled;
        if mica {
            let _ = apply_mica(&window, Some(dark.unwrap_or(true)));
        } else {
            let _ = clear_mica(&window);
        }
    }
    Ok(())
}

#[tauri::command]
fn set_dock(
    edge: dock::DockEdge,
    out: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<dock::DockConfig, String> {
    let always_on_top = out
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .get()
        .always_on_top;
    let current_cfg = out.dock.lock().map_err(|e| e.to_string())?.get();
    let target_cfg = dock::DockConfig {
        edge,
        width: current_cfg.width,
        always_on_top,
    };

    if edge == dock::DockEdge::None {
        // When un-docking, restore the saved floating geometry (size + position)
        // resolved against currently available monitor work areas.
        let scale = window.scale_factor().unwrap_or(1.0);
        let g = out.geom.lock().map_err(|e| e.to_string())?.get("none");

        let monitors: Vec<geom::Rect> = window
            .available_monitors()
            .unwrap_or_default()
            .iter()
            .map(|m| {
                let wa = m.work_area();
                geom::Rect {
                    x: wa.position.x as f64 / scale,
                    y: wa.position.y as f64 / scale,
                    width: wa.size.width as f64 / scale,
                    height: wa.size.height as f64 / scale,
                }
            })
            .collect();

        let primary_rect = window
            .primary_monitor()
            .ok()
            .flatten()
            .map(|m| {
                let wa = m.work_area();
                geom::Rect {
                    x: wa.position.x as f64 / scale,
                    y: wa.position.y as f64 / scale,
                    width: wa.size.width as f64 / scale,
                    height: wa.size.height as f64 / scale,
                }
            })
            .unwrap_or(geom::Rect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1040.0,
            });

        let saved_rect = geom::Rect {
            x: g.x as f64,
            y: g.y as f64,
            width: g.width as f64,
            height: g.height as f64,
        };

        let resolved = geom::resolve_window_placement(saved_rect, &monitors, primary_rect);

        // Apply native changes first
        dock::apply_dock(&window, &target_cfg)?;
        use tauri::{LogicalPosition, LogicalSize};
        let _ = window.set_size(LogicalSize::new(resolved.width, resolved.height));
        let _ = window.set_position(LogicalPosition::new(resolved.x, resolved.y));
        let _ = window.emit(
            "windash://resized",
            serde_json::json!({ "width": resolved.width, "height": resolved.height }),
        );
    } else {
        // Docking: apply native window positioning first!
        dock::apply_dock(&window, &target_cfg)?;
    }

    // Native action succeeded, persist the new dock config!
    let store = out.dock.lock().map_err(|e| e.to_string())?;
    store.set(&target_cfg)?;

    Ok(target_cfg)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_uri_empty_or_none() {
        assert_eq!(build_search_ms_uri(None), "search-ms:");
        assert_eq!(build_search_ms_uri(Some("")), "search-ms:");
        assert_eq!(build_search_ms_uri(Some("   ")), "search-ms:");
        assert_eq!(build_search_ms_uri(Some("\r\n\t")), "search-ms:");
    }

    #[test]
    fn test_search_uri_alphanumeric_and_spaces() {
        assert_eq!(build_search_ms_uri(Some("rust")), "search-ms:query=rust");
        assert_eq!(
            build_search_ms_uri(Some("rust cargo")),
            "search-ms:query=rust%20cargo"
        );
    }

    #[test]
    fn test_search_uri_quotes_and_control_stripped() {
        assert_eq!(
            build_search_ms_uri(Some("hello \"world\" 'test'")),
            "search-ms:query=hello%20world%20test"
        );
        assert_eq!(
            build_search_ms_uri(Some("cmd\x00\r\n/c calc")),
            "search-ms:query=cmd%2Fc%20calc"
        );
    }

    #[test]
    fn test_search_uri_special_characters_encoded() {
        assert_eq!(
            build_search_ms_uri(Some("c++ & rust=fast")),
            "search-ms:query=c%2B%2B%20%26%20rust%3Dfast"
        );
    }

    #[test]
    fn test_search_uri_cjk_and_emoji() {
        // UTF-8 bytes for '日本語' are E6 97 A5 E6 9C AC E8 AA 9E
        assert_eq!(
            build_search_ms_uri(Some("日本語")),
            "search-ms:query=%E6%97%A5%E6%9C%AC%E8%AA%9E"
        );
        // Emoji 🦀 (F0 9F A6 80)
        assert_eq!(
            build_search_ms_uri(Some("🦀")),
            "search-ms:query=%F0%9F%A6%80"
        );
    }
}
