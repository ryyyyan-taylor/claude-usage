use crate::state::{AppState, UsageSnapshot};
use tauri::AppHandle;

const THRESHOLDS_5H: &[u8] = &[75, 90];

/// Check usage against thresholds and log/queue notifications
///
/// Uses a latch mechanism: notification fires once when crossing threshold,
/// resets when usage drops back below threshold.
/// (Actual notification delivery is deferred to Phase 6)
pub fn check_thresholds(_state: &mut AppState, _snapshot: &UsageSnapshot, _app: &AppHandle) {
    // Placeholder for Phase 6 implementation
    // For now, just manage the threshold latch state

    // This will be replaced with actual notification sending in Phase 6
}
