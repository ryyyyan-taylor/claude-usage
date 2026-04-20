pub mod error;
mod claude;
mod state;
mod poller;
mod notify;
mod tray;

pub use error::{AppError, Result};
pub use state::{AppState, UsageSnapshot};

use std::sync::{Arc, Mutex};
use tauri::{State, Emitter, Manager};

// Tauri command: get current snapshot
#[tauri::command]
fn get_snapshot(app_state: State<'_, Arc<Mutex<AppState>>>) -> Option<UsageSnapshot> {
    let state = app_state.lock().unwrap();
    state.snapshot.clone()
}

// Tauri command: trigger immediate refresh
#[tauri::command]
async fn refresh_now(app_state: State<'_, Arc<Mutex<AppState>>>, _app: tauri::AppHandle) -> std::result::Result<UsageSnapshot, String> {
    // Mark as refreshing
    {
        let mut state = app_state.lock().unwrap();
        state.is_refreshing = true;
    }

    // Perform fetch
    match claude::refresh().await {
        Ok(usage_response) => {
            let snapshot = UsageSnapshot::from(usage_response);

            // Update state
            {
                let mut state = app_state.lock().unwrap();
                state.snapshot = Some(snapshot.clone());
                state.is_refreshing = false;
                state.last_refreshed = Some(chrono::Utc::now());
                state.auth_error = false;
            }

            // Save to cache
            let _ = state::save_cache(&snapshot);

            Ok(snapshot)
        }
        Err(e) => {
            let mut state = app_state.lock().unwrap();
            state.is_refreshing = false;
            Err(e.to_string())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize app state
    let app_state = Arc::new(Mutex::new(AppState::new()));

    // Try to load cached snapshot
    if let Some(cached) = state::load_cached() {
        let mut state = app_state.lock().unwrap();
        state.snapshot = Some(cached);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![get_snapshot, refresh_now])
        .setup(move |app| {
            // Set up system tray
            let _ = tray::setup_tray(app);

            // Load initial cache and emit to frontend
            {
                let state = app_state.lock().unwrap();
                if let Some(snapshot) = &state.snapshot {
                    if let Some(window) = app.get_webview_window("main") {
                        let snap = snapshot.clone();
                        let _ = window.emit::<UsageSnapshot>("usage_updated", snap);
                    }
                }
            }

            // Spawn background poller
            let app_handle = app.handle().clone();
            let state_for_poller = app_state.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    poller::start_poller(state_for_poller, app_handle).await
                });
            });

            Ok(())
        })
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "refresh" => {
                    // Trigger immediate refresh via command
                    // User can also call refresh_now() from frontend
                }
                "open_claude" => {
                    // Open Claude in default browser
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
