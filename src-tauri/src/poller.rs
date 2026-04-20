use crate::AppError;
use crate::claude;
use crate::state::{AppState, UsageSnapshot, save_cache};
use crate::notify::check_thresholds;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, Emitter};

/// Start the background polling loop
///
/// Runs in a separate Tokio task, polls every ~60 seconds for new usage data.
/// Updates shared state, emits events to frontend, triggers notifications.
pub async fn start_poller(state: Arc<Mutex<AppState>>, app: AppHandle) {
    let interval = Duration::from_secs(60);
    let mut rate_limited_count = 0;

    loop {
        // Check if rate limited
        {
            let s = state.lock().unwrap();
            if let Some(until) = s.rate_limited_until {
                if chrono::Utc::now() < until {
                    rate_limited_count += 1;
                    if rate_limited_count < 3 {
                        // Skip this cycle
                        tokio::time::sleep(interval).await;
                        continue;
                    } else {
                        // Reset counter after 3 skips
                        rate_limited_count = 0;
                    }
                }
            }
        }

        // Mark as refreshing
        {
            let mut s = state.lock().unwrap();
            s.is_refreshing = true;
        }

        // Perform the actual fetch
        match claude::refresh().await {
            Ok(usage_response) => {
                let snapshot = UsageSnapshot::from(usage_response);

                // Update state
                {
                    let mut s = state.lock().unwrap();
                    s.snapshot = Some(snapshot.clone());
                    s.is_refreshing = false;
                    s.last_refreshed = Some(chrono::Utc::now());
                    s.auth_error = false;
                    s.rate_limited_until = None;
                    rate_limited_count = 0;

                    // Check thresholds and send notifications
                    check_thresholds(&mut s, &snapshot, &app);
                }

                // Save to cache
                let _ = save_cache(&snapshot);

                // Emit event to main window
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("usage_updated", snapshot.clone());
                }
            }
            Err(AppError::AuthRequired) => {
                let mut s = state.lock().unwrap();
                s.is_refreshing = false;
                s.auth_error = true;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit::<&str>("auth_error", "");
                }
            }
            Err(AppError::RateLimited) => {
                let mut s = state.lock().unwrap();
                s.is_refreshing = false;
                // Back off for 5 minutes
                s.rate_limited_until = Some(chrono::Utc::now() + chrono::Duration::minutes(5));
                tracing::warn!("Rate limited by API, backing off for 5 minutes");
            }
            Err(e) => {
                // Network or other transient error
                let mut s = state.lock().unwrap();
                s.is_refreshing = false;
                // Preserve existing snapshot
                tracing::warn!("Poll error (snapshot preserved): {}", e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}
