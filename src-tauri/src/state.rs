use crate::{AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// A single usage window's data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowData {
    pub utilization: f64,    // percentage 0–100
    pub resets_at: DateTime<Utc>,
}

/// Extra/paid usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraUsageData {
    pub is_enabled: bool,
    pub used_credits: f64,   // e.g. 2.61 USD
    pub monthly_limit: f64,  // e.g. 20.00 USD
    pub utilization: f64,    // percentage 0–100
    pub currency: String,
}

/// Complete usage snapshot from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub five_hour: WindowData,
    pub seven_day: WindowData,
    pub extra_usage: Option<ExtraUsageData>,
    pub fetched_at: DateTime<Utc>,
}

/// Convert API response to UsageSnapshot
impl From<crate::claude::UsageResponse> for UsageSnapshot {
    fn from(resp: crate::claude::UsageResponse) -> Self {
        UsageSnapshot {
            five_hour: WindowData {
                utilization: resp.five_hour.utilization,
                resets_at: parse_iso8601_opt(resp.five_hour.resets_at.as_deref()),
            },
            seven_day: WindowData {
                utilization: resp.seven_day.utilization,
                resets_at: parse_iso8601_opt(resp.seven_day.resets_at.as_deref()),
            },
            extra_usage: resp.extra_usage.map(|e| ExtraUsageData {
                is_enabled: e.is_enabled,
                used_credits: e.used_credits,
                monthly_limit: e.monthly_limit,
                utilization: e.utilization,
                currency: e.currency.unwrap_or_else(|| "USD".to_string()),
            }),
            fetched_at: Utc::now(),
        }
    }
}

/// Parse optional ISO 8601 timestamp, fallback to far future on error/None
fn parse_iso8601_opt(s: Option<&str>) -> DateTime<Utc> {
    s.and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(365))
}

/// Runtime app state
#[derive(Debug, Clone)]
pub struct AppState {
    pub snapshot: Option<UsageSnapshot>,
    pub is_refreshing: bool,
    pub last_refreshed: Option<DateTime<Utc>>,
    pub auth_error: bool,
    pub rate_limited_until: Option<DateTime<Utc>>,
    pub notified_thresholds: HashSet<u8>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            snapshot: None,
            is_refreshing: false,
            last_refreshed: None,
            auth_error: false,
            rate_limited_until: None,
            notified_thresholds: HashSet::new(),
        }
    }

    /// Check if snapshot is stale (> 10 minutes old)
    pub fn is_stale(&self) -> bool {
        match self.last_refreshed {
            Some(time) => {
                let elapsed = Utc::now()
                    .signed_duration_since(time)
                    .num_seconds();
                elapsed > 600 // 10 minutes
            }
            None => true,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get cache directory path: ~/.cache/claude-usage/
fn cache_dir() -> Result<PathBuf> {
    let cache_base = dirs::cache_dir()
        .ok_or_else(|| AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine cache directory",
        )))?;

    let cache_path = cache_base.join("claude-usage");
    std::fs::create_dir_all(&cache_path)?;
    Ok(cache_path)
}

/// Get snapshot cache file path
fn cache_file_path() -> Result<PathBuf> {
    let dir = cache_dir()?;
    Ok(dir.join("snapshot.json"))
}

/// Load cached snapshot from disk
pub fn load_cached() -> Option<UsageSnapshot> {
    let path = cache_file_path().ok()?;

    if !path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<UsageSnapshot>(&content).ok()
}

/// Save snapshot to cache
pub fn save_cache(snapshot: &UsageSnapshot) -> Result<()> {
    let path = cache_file_path()?;
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(state.snapshot.is_none());
        assert!(!state.is_refreshing);
        assert!(state.last_refreshed.is_none());
        assert!(!state.auth_error);
        assert!(state.is_stale());
    }

    #[test]
    fn test_is_stale_with_fresh_timestamp() {
        let mut state = AppState::new();
        state.last_refreshed = Some(Utc::now());
        assert!(!state.is_stale());
    }

    #[test]
    fn test_is_stale_with_old_timestamp() {
        let mut state = AppState::new();
        state.last_refreshed = Some(Utc::now() - chrono::Duration::minutes(15));
        assert!(state.is_stale());
    }

    #[test]
    fn test_parse_iso8601_valid() {
        let timestamp = Some("2025-04-19T12:30:45Z");
        let parsed = parse_iso8601_opt(timestamp);
        assert_eq!(parsed.year(), 2025);
        assert_eq!(parsed.month(), 4);
    }

    #[test]
    fn test_parse_iso8601_invalid_fallback() {
        // Invalid string → falls back to far future (1 year from now)
        let parsed = parse_iso8601_opt(Some("invalid"));
        let now = Utc::now();
        assert!(parsed > now);
    }

    #[test]
    fn test_parse_iso8601_none_fallback() {
        // None → falls back to far future
        let parsed = parse_iso8601_opt(None);
        let now = Utc::now();
        assert!(parsed > now);
    }
}
