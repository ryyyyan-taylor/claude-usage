use crate::state::{AppState, UsageSnapshot};
use tauri::AppHandle;

/// Check usage against thresholds and send notifications
///
/// Uses a latch mechanism: notification fires once when crossing a threshold,
/// resets when usage drops back below it — preventing repeat alerts.
/// Thresholds come from Config, not hardcoded constants.
pub fn check_thresholds(
    state: &mut AppState,
    snapshot: &UsageSnapshot,
    _app: &AppHandle,
    thresholds_5h: &[u8],
    thresholds_7d: &[u8],
) {
    let pct_5h = snapshot.five_hour.utilization.round() as u8;
    let pct_7d = snapshot.seven_day.utilization.round() as u8;

    check_window(state, pct_5h, thresholds_5h, "5h");
    check_window(state, pct_7d, thresholds_7d, "7d");
}

/// Check one window's thresholds, send notification and manage latch
fn check_window(state: &mut AppState, pct: u8, thresholds: &[u8], window_label: &str) {
    for &threshold in thresholds {
        // Use a composite key: threshold + window encoded into u8 range
        // 5h uses thresholds as-is (0-100), 7d uses threshold + 100 offset
        let latch_key = if window_label == "7d" {
            threshold.saturating_add(100)
        } else {
            threshold
        };

        if pct >= threshold && !state.notified_thresholds.contains(&latch_key) {
            state.notified_thresholds.insert(latch_key);
            send_notification(pct, threshold, window_label);
        } else if pct < threshold {
            state.notified_thresholds.remove(&latch_key);
        }
    }
}

/// Send a native desktop notification for a threshold crossing
fn send_notification(current_pct: u8, threshold: u8, window_label: &str) {
    let window_name = if window_label == "5h" { "5-hour" } else { "7-day" };
    let body = format!(
        "Claude {window_name} window at {current_pct}% (threshold: {threshold}%)"
    );

    if let Err(e) = notify_rust::Notification::new()
        .summary("Claude Usage Alert")
        .body(&body)
        .timeout(5000) // 5 seconds
        .show()
    {
        tracing::warn!("Failed to send notification: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;

    #[test]
    fn test_latch_fires_once_then_holds() {
        let mut state = AppState::new();
        let thresholds: &[u8] = &[75, 90];

        // First time at 92% — both thresholds should latch
        check_window(&mut state, 92, thresholds, "5h");
        assert!(state.notified_thresholds.contains(&75));
        assert!(state.notified_thresholds.contains(&90));

        // Second time at 92% — already latched, no change
        let prev_len = state.notified_thresholds.len();
        check_window(&mut state, 92, thresholds, "5h");
        assert_eq!(state.notified_thresholds.len(), prev_len);
    }

    #[test]
    fn test_latch_resets_below_threshold() {
        let mut state = AppState::new();
        let thresholds: &[u8] = &[90];

        check_window(&mut state, 92, thresholds, "5h");
        assert!(state.notified_thresholds.contains(&90));

        // Drop below 90% — latch should reset
        check_window(&mut state, 85, thresholds, "5h");
        assert!(!state.notified_thresholds.contains(&90));

        // Rise above 90% again — should re-fire
        check_window(&mut state, 92, thresholds, "5h");
        assert!(state.notified_thresholds.contains(&90));
    }

    #[test]
    fn test_5h_and_7d_use_separate_latches() {
        let mut state = AppState::new();
        let thresholds: &[u8] = &[90];

        check_window(&mut state, 92, thresholds, "5h");
        check_window(&mut state, 92, thresholds, "7d");

        // Both should be latched, with different keys
        assert!(state.notified_thresholds.contains(&90));   // 5h key
        assert!(state.notified_thresholds.contains(&190));  // 7d key (90 + 100)
    }

    #[test]
    fn test_granular_threshold() {
        let mut state = AppState::new();
        let thresholds: &[u8] = &[75, 90];

        // At 78%, only the 75% threshold should fire
        check_window(&mut state, 78, thresholds, "5h");
        assert!(state.notified_thresholds.contains(&75));
        assert!(!state.notified_thresholds.contains(&90));
    }

    #[test]
    fn test_percentage_rounding() {
        // Confirm utilization is correctly rounded to u8
        let pct = 92.4_f64.round() as u8;
        assert_eq!(pct, 92);
        let pct = 92.6_f64.round() as u8;
        assert_eq!(pct, 93);
    }
}
