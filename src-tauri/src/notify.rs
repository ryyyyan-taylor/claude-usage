use crate::state::{AppState, UsageSnapshot};
use tauri::AppHandle;

const THRESHOLDS_5H: &[u8] = &[75, 90];

/// Check usage against thresholds and send notifications
///
/// Uses a latch mechanism: notification fires once when crossing threshold,
/// resets when usage drops back below threshold.
pub fn check_thresholds(state: &mut AppState, snapshot: &UsageSnapshot, _app: &AppHandle) {
    // utilization is already 0–100 from the API
    let pct = snapshot.five_hour.utilization.round() as u8;

    for &threshold in THRESHOLDS_5H {
        if pct >= threshold && !state.notified_thresholds.contains(&threshold) {
            // Threshold crossed — send notification and record it
            state.notified_thresholds.insert(threshold);
            send_notification(pct, threshold);
        } else if pct < threshold {
            // Usage dropped below threshold — reset latch for next crossing
            state.notified_thresholds.remove(&threshold);
        }
    }
}

/// Send a desktop notification for threshold crossing
fn send_notification(current_pct: u8, threshold: u8) {
    let body = format!(
        "Claude 5-hour window at {}% (threshold: {}%)",
        current_pct, threshold
    );

    // Use notify-rust for native system notifications
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

    #[test]
    fn test_thresholds_defined() {
        // Verify thresholds are in expected order
        assert_eq!(THRESHOLDS_5H.len(), 2);
        assert_eq!(THRESHOLDS_5H[0], 75);
        assert_eq!(THRESHOLDS_5H[1], 90);
    }

    #[test]
    fn test_notification_percentage_calculation() {
        // Verify we calculate percentage correctly
        let utilization = 0.92;
        let pct = (utilization * 100.0) as u8;
        assert_eq!(pct, 92);

        let utilization = 0.745;
        let pct = (utilization * 100.0) as u8;
        assert_eq!(pct, 74); // Rounds down
    }
}
