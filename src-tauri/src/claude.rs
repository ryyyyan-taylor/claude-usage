use crate::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Anthropic OAuth credentials from Claude CLI
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    pub oauth: OAuthBlock,
}

/// OAuth token block
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthBlock {
    pub access_token: String,
    pub expires_at: u64, // milliseconds since epoch
    pub scopes: Vec<String>,
}

/// Get the credentials file path
///
/// Respects $CLAUDE_CONFIG_DIR if set, otherwise falls back to ~/.claude/.credentials.json
pub fn credentials_path() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join(".credentials.json"));
    }

    let home = dirs::home_dir()
        .ok_or_else(|| AppError::CredentialsNotFound("Could not determine home directory".to_string()))?;

    Ok(home.join(".claude").join(".credentials.json"))
}

/// Load credentials from ~/.claude/.credentials.json
pub fn load_credentials() -> Result<Credentials> {
    let path = credentials_path()?;

    if !path.exists() {
        return Err(AppError::CredentialsNotFound(path.display().to_string()));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::Io(e))?;

    serde_json::from_str::<Credentials>(&content)
        .map_err(|e| AppError::SerdeJson(e))
}

/// Check if token needs refreshing (with 5-minute buffer)
pub fn needs_refresh(expires_at_ms: u64) -> bool {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Refresh if expiring within 5 minutes (300,000 ms)
    now_ms + 300_000 >= expires_at_ms
}

/// Refresh token by invoking `claude auth status --json`
///
/// The Claude CLI updates the credentials file as a side effect.
/// We don't parse the output — just trigger the refresh.
pub async fn refresh_token() -> Result<()> {
    let output = tokio::process::Command::new("claude")
        .args(&["auth", "status", "--json"])
        .output()
        .await
        .map_err(|_| AppError::CliNotFound)?;

    if !output.status.success() {
        return Err(AppError::CliNotFound);
    }

    Ok(())
}

/// Usage window from API response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageWindow {
    pub utilization: f64, // 0.0–1.0
    pub resets_at: String, // ISO 8601 timestamp
}

/// Extra/paid usage info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtraUsage {
    pub enabled: bool,
    pub used_credits: u64, // cents
    pub monthly_limit: u64, // cents
    pub utilization: f64,
}

/// Response from api.anthropic.com/api/oauth/usage
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsageResponse {
    pub five_hour: UsageWindow,
    pub seven_day: UsageWindow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_usage: Option<ExtraUsage>,
}

/// Fetch usage from Anthropic API
pub async fn fetch_usage(token: &str) -> Result<UsageResponse> {
    let client = reqwest::Client::new();

    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    match resp.status() {
        reqwest::StatusCode::OK => {
            resp.json::<UsageResponse>()
                .await
                .map_err(|e| AppError::Parse(e.to_string()))
        }
        reqwest::StatusCode::UNAUTHORIZED => Err(AppError::AuthRequired),
        reqwest::StatusCode::TOO_MANY_REQUESTS => Err(AppError::RateLimited),
        status => Err(AppError::Network(format!("API returned {}", status))),
    }
}

/// Full refresh flow: load creds, refresh token if needed, fetch usage
pub async fn refresh() -> Result<UsageResponse> {
    let mut creds = load_credentials()?;

    // Refresh token if expiring soon
    if needs_refresh(creds.oauth.expires_at) {
        refresh_token().await?;
        // Reload credentials after refresh
        creds = load_credentials()?;
    }

    // Fetch fresh usage data
    fetch_usage(&creds.oauth.access_token).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_refresh_past_timestamp() {
        let past = 1000; // way in the past
        assert!(needs_refresh(past));
    }

    #[test]
    fn test_needs_refresh_far_future() {
        let far_future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 + 86_400_000; // 24 hours from now
        assert!(!needs_refresh(far_future));
    }

    #[test]
    fn test_needs_refresh_expiring_soon() {
        let soon = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 + 60_000; // 1 minute from now
        assert!(needs_refresh(soon));
    }
}
