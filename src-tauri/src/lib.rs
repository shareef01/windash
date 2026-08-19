// Windash — personal Windows system dashboard
// Rust backend: system metrics (sysinfo 0.32), persistent notes (JSON file in
// AppData), quick-launch shell commands, and the Tauri app setup with system tray.

mod metrics;
mod notes;
mod dock;

use metrics::SystemMetrics;
use notes::NotesStore;
use dock::DockStore;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
    image::Image,
};

struct AppState {
    metrics: Mutex<SystemMetrics>,
    notes: Mutex<NotesStore>,
    dock: Mutex<DockStore>,
}

pub fn run() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let metrics = SystemMetrics::new();
            let notes = NotesStore::new(app.handle())?;
            let dock_store = DockStore::new(app.handle())?;
            let state = AppState {
                metrics: Mutex::new(metrics),
                notes: Mutex::new(notes),
                dock: Mutex::new(dock_store),
            };
            app.manage(state);

            // Apply saved dock position to the main window once it exists.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let cfg = app.state::<AppState>().dock.lock().unwrap().get();
                let _ = dock::apply_dock(&window, &cfg);
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
        ])
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

#[tauri::command]
fn get_dock(out: tauri::State<'_, AppState>) -> Result<dock::DockConfig, String> {
    Ok(out.dock.lock().map_err(|e| e.to_string())?.get())
}

/// Remove native window chrome and apply a frosted-glass blur. Must run after
/// the window is fully created (i.e. from the frontend after mount); doing it
/// in setup() does not always persist the style change.
///
/// NOTE: use `apply_blur`, NOT `apply_acrylic`. On Windows 11 `apply_acrylic`
/// forces a `WS_CAPTION` (native titlebar) back onto the window, which fights
/// the borderless custom titlebar. `apply_blur` gives the same frosted look on
/// a decoration-less window.
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

/// Minimize the main window (reliable OS-level call from Rust, avoiding the
/// frontend window-handle API which can fail on a docked/resizable-locked window).
#[tauri::command]
fn window_minimize(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Hide the main window to the tray (recoverable via the tray icon), rather than
/// terminating the app — keeps the system monitor running in the background.
#[tauri::command]
fn window_hide(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
