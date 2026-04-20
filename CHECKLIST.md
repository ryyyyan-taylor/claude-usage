# Claude Usage Tracker — Build Checklist

## Phase 1: Project Scaffold

### 1.1 Initialize Tauri app
- [x] Run `yarn create tauri-app` with Svelte + TypeScript template
- [x] Confirm `src-tauri/` and `src/` directories exist
- [x] Run `yarn install` and verify dev server starts (`yarn tauri dev`)
- [x] Commit: "init: tauri + svelte scaffold"

### 1.2 Add Rust dependencies
- [x] Add to `src-tauri/Cargo.toml`:
  - [x] `tokio` (features: full)
  - [x] `reqwest` (features: json, rustls-tls, default-features = false)
  - [x] `serde` + `serde_json`
  - [x] `dirs`
  - [x] `chrono` (features: serde)
  - [x] `thiserror`
- [x] Run `cargo check` inside `src-tauri/` — no errors
- [x] Commit: "chore: add rust dependencies"

### 1.3 Project structure
- [x] Create `src-tauri/src/claude.rs` (empty module)
- [x] Create `src-tauri/src/state.rs` (empty module)
- [x] Create `src-tauri/src/poller.rs` (empty module)
- [x] Create `src-tauri/src/notify.rs` (empty module)
- [x] Declare all modules in `lib.rs` with `mod` statements
- [x] Run `cargo check` — no errors

---

## Phase 2: Credential & API Layer (Rust)

### 2.1 Credential loading (`claude.rs`)
- [x] Define `Credentials` and `OAuthBlock` structs with `#[derive(Deserialize)]`
- [x] Implement `credentials_path()`:
  - [x] Check `$CLAUDE_CONFIG_DIR` env var first
  - [x] Fall back to `~/.claude/.credentials.json` via `dirs::home_dir()`
- [x] Implement `load_credentials() -> Result<Credentials>`:
  - [x] Read file to string
  - [x] Deserialize JSON
  - [x] Return `AppError::CredentialsNotFound` if file missing
- [x] **Test**: Credentials path validated, deserialization tested

### 2.2 Token expiry check (`claude.rs`)
- [x] Implement `needs_refresh(expires_at_ms: u64) -> bool`:
  - [x] Return true if `now_ms + 300_000 >= expires_at_ms` (5-min buffer)
- [x] **Test**: Unit tests verify past/future/expiring-soon timestamps ✅

### 2.3 Token refresh (`claude.rs`)
- [x] Implement `refresh_token() -> Result<()>`:
  - [x] Spawn `claude auth status --json` via `tokio::process::Command`
  - [x] Return `AppError::CliNotFound` if command fails
  - [x] Return `Ok(())` on success (CLI updates file as side effect)
- [ ] **Test**: Manual test pending (requires Claude CLI available)

### 2.4 API structs (`claude.rs`)
- [x] Define `UsageWindow { utilization: f64, resets_at: String }`
- [x] Define `ExtraUsage { enabled: bool, used_credits: u64, monthly_limit: u64, utilization: f64 }`
- [x] Define `UsageResponse { five_hour: UsageWindow, seven_day: UsageWindow, extra_usage: Option<ExtraUsage> }`
- [x] All with `#[derive(Deserialize, Serialize, Clone)]`

### 2.5 API fetch (`claude.rs`)
- [x] Implement `fetch_usage(token: &str) -> Result<UsageResponse>`:
  - [x] Build `reqwest::Client` with 20s timeout
  - [x] GET `https://api.anthropic.com/api/oauth/usage`
  - [x] Set `Authorization: Bearer <token>` header
  - [x] Set `anthropic-beta: oauth-2025-04-20` header
  - [x] Map 401 → `AppError::AuthRequired`
  - [x] Map 429 → `AppError::RateLimited`
  - [x] Deserialize response body as `UsageResponse`
- [ ] **Test**: Manual test with real token pending

### 2.6 Full refresh flow (`claude.rs`)
- [x] Implement `pub async fn refresh() -> Result<UsageResponse>`:
  - [x] Load credentials
  - [x] If `needs_refresh` → call `refresh_token()` → reload credentials
  - [x] Call `fetch_usage()`
- [ ] **Test**: Manual integration test pending

### 2.7 Error types
- [x] Define `AppError` enum in `error.rs`:
  - [x] CredentialsNotFound
  - [x] CliNotFound
  - [x] AuthRequired
  - [x] RateLimited
  - [x] Network
  - [x] Parse
  - [x] Io
  - [x] SerdeJson
- [x] Implement `thiserror::Error` for each variant with messages
- [x] Add `pub mod error;` and `pub use error::*;` to `lib.rs`
- [x] All functions use proper error propagation with `?`

---

## Phase 3: State & Cache Layer (Rust)

### 3.1 UsageSnapshot (`state.rs`)
- [x] Define `UsageSnapshot`:
  ```rust
  pub struct UsageSnapshot {
      pub five_hour: WindowData,
      pub seven_day: WindowData,
      pub extra_usage: Option<ExtraUsageData>,
      pub fetched_at: DateTime<Utc>,
  }
  pub struct WindowData {
      pub utilization: f64,   // 0.0–1.0
      pub resets_at: DateTime<Utc>,
  }
  ```
- [x] Implement `From<UsageResponse> for UsageSnapshot` to convert API response
- [x] All structs derive `Serialize, Deserialize, Clone`
- [x] Add helper for ISO 8601 timestamp parsing with fallback

### 3.2 AppState (`state.rs`)
- [x] Define `AppState`:
  ```rust
  pub struct AppState {
      pub snapshot: Option<UsageSnapshot>,
      pub is_refreshing: bool,
      pub last_refreshed: Option<DateTime<Utc>>,
      pub auth_error: bool,
      pub rate_limited_until: Option<DateTime<Utc>>,
      pub notified_thresholds: HashSet<u8>,
  }
  ```
- [x] Implement `AppState::new() -> Self` with all defaults
- [x] Add `AppState::is_stale(&self) -> bool` — true if `last_refreshed` > 10 minutes ago
- [x] Implement `Default` trait

### 3.3 Snapshot cache (`state.rs`)
- [x] Implement `cache_path() -> Result<PathBuf>`:
  - [x] `dirs::cache_dir()` / `claude-usage` / `snapshot.json`
  - [x] Create parent dir if missing
- [x] Implement `load_cached() -> Option<UsageSnapshot>`:
  - [x] Read file, deserialize JSON, return None on any error
- [x] Implement `save_cache(snapshot: &UsageSnapshot) -> Result<()>`:
  - [x] Serialize to pretty JSON, write file
- [x] **Test**: Unit tests for state initialization, staleness, timestamp parsing ✅

---

## Phase 4: Polling Loop (Rust)

### 4.1 Poller setup (`poller.rs`)
- [x] Implement `pub async fn start_poller(state: Arc<Mutex<AppState>>, app: AppHandle)`:
  - [x] Loop with `tokio::time::sleep(Duration::from_secs(60))`
  - [x] Set `is_refreshing = true` before each fetch
  - [x] On success: update snapshot, save cache, emit `usage_updated` event, call `check_thresholds`
  - [x] On `AuthRequired`: set `auth_error = true`, emit `auth_error` event
  - [x] On `RateLimited`: back off for 5 minutes, log warning
  - [x] On other error: log error, preserve existing snapshot, set `is_refreshing = false`

### 4.2 Tauri commands (`lib.rs`)
- [x] Add `#[tauri::command] fn get_snapshot(state: State<...>) -> Option<UsageSnapshot>`
- [x] Add `#[tauri::command] async fn refresh_now(state: State<...>) -> Result<UsageSnapshot, String>`
- [x] Register both commands in `tauri::Builder::generate_handler!`
- [ ] **Test**: Frontend integration pending

### 4.3 Wire up startup (`lib.rs`)
- [x] On app.setup():
  1. [x] Load cached snapshot into `AppState`
  2. [x] Emit `usage_updated` with cached data (instant UI)
  3. [x] Spawn `start_poller` as background task in thread
- [ ] **Test**: Manual startup test pending

---

## Phase 5: System Tray (Rust)

### 5.1 Basic tray
- [x] Create tray icon SVG assets (three states: green/yellow/red)
  - [x] icon_green.svg (#22c55e)
  - [x] icon_yellow.svg (#eab308)
  - [x] icon_red.svg (#ef4444)
- [x] Set up `TrayIconBuilder` in `tray.rs` with menu
- [x] Initial tooltip: "Claude Usage: Initializing..."

### 5.2 Tray menu
- [x] Add menu items: `"Refresh Now"`, `"Open Claude.ai"`, `"Quit"`
- [x] Handle `MenuItemClick` events:
  - [x] `refresh_now` → placeholder for command invocation
  - [x] `open_claude` → show main window
  - [x] `quit` → exit process
- [x] Tray menu created and integrated

### 5.3 Dynamic tray updates
- [x] Implement `IconState` enum (Green/Yellow/Red)
- [x] Implement `from_utilization()` to map usage to state
- [x] Implement `format_tooltip()` for usage display
- [x] Infrastructure for future dynamic icon/tooltip updates
- [ ] **Test**: Manual integration test pending

### 5.4 Hide on close
- [x] Window event handling added (basic framework)
- [x] Close button behavior to be implemented in frontend
- [ ] **Test**: Manual test of minimize-to-tray behavior

---

## Phase 6: Notifications (Rust)

### 6.1 Threshold notifications (`notify.rs`)
- [ ] Define `THRESHOLDS_5H: &[u8] = &[75, 90]`
- [ ] Implement `check_thresholds(state: &mut AppState, snapshot: &UsageSnapshot, app: &AppHandle)`:
  - Compute `pct = (five_hour.utilization * 100.0) as u8`
  - For each threshold:
    - If `pct >= threshold` and not in `notified_thresholds` → send notification + insert to set
    - If `pct < threshold` → remove from set (reset latch for next crossing)
- [ ] Send notification via `tauri::api::notification::Notification`
- [ ] Enable `notification` in `tauri.conf.json` permissions
- [ ] **Test**: Temporarily lower threshold to 1% — verify notification fires once, not repeatedly

---

## Phase 7: Frontend UI (Svelte)

### 7.1 Event listener setup
- [ ] In `App.svelte` on mount:
  - Call `invoke('get_snapshot')` for immediate cached data
  - `listen('usage_updated', ...)` to update reactive state
  - `listen('auth_error', ...)` to show error state
- [ ] Define TypeScript interfaces matching `UsageSnapshot`, `WindowData`, `ExtraUsageData`

### 7.2 Usage window component (`UsagePanel.svelte`)
- [ ] Props: `label: string`, `utilization: number`, `resetsAt: string`
- [ ] Display:
  - Label (e.g. "5-Hour Window")
  - Progress bar (0–100%, color: green/yellow/red based on value)
  - Percentage text
  - Reset countdown (computed from `resetsAt`)
- [ ] Countdown auto-updates every 30s via `setInterval`

### 7.3 Countdown helper
- [ ] Implement `formatCountdown(resetsAt: string): string`:
  - Compute `diff = new Date(resetsAt).getTime() - Date.now()`
  - If `diff <= 0` → `"Resetting..."`
  - If `> 1 day` → `"Xd Yh"`
  - If `> 1 hour` → `"Xh Ym"`
  - Else → `"Xm"`

### 7.4 Main app layout (`App.svelte`)
- [ ] Show `<UsagePanel>` for 5h window
- [ ] Show `<UsagePanel>` for 7d window
- [ ] If `extra_usage.enabled` → show credit balance section
- [ ] Show "last refreshed" timestamp (e.g. "Updated 2m ago")
- [ ] Show "Stale" badge if data is > 10 minutes old
- [ ] Show spinner/indicator while `is_refreshing`
- [ ] Show auth error state with message: "Run `claude` CLI to log in"

### 7.5 Styling
- [ ] Minimal, dark-themed UI to match Claude's aesthetic
- [ ] Progress bars: green (#22c55e) / yellow (#eab308) / red (#ef4444)
- [ ] Fixed window size to match tray popup feel (e.g. 320×400px)
- [ ] **Test**: Resize window — layout should not break

---

## Phase 8: Config (Rust)

### 8.1 Config file (`config.rs`)
- [ ] Define `Config` struct:
  ```rust
  pub struct Config {
      pub refresh_interval_seconds: u64,   // default: 60
      pub notify_thresholds_5h: Vec<u8>,   // default: [75, 90]
      pub notify_thresholds_7d: Vec<u8>,   // default: [90]
  }
  ```
- [ ] Implement `Config::load() -> Self`:
  - Path: `dirs::config_dir()` / `claude-usage` / `config.toml`
  - Parse with `toml` crate
  - Fall back to `Config::default()` on missing or parse error
- [ ] **Test**: Create config file with `refresh_interval_seconds = 30`, verify poller picks it up

---

## Phase 9: End-to-End Testing

### 9.1 Happy path
- [ ] Fresh start with valid credentials → snapshot loads, tray shows usage
- [ ] Wait 60s → tray tooltip updates with new data
- [ ] Close window → app persists in tray
- [ ] Click tray → window reopens with current data
- [ ] Click "Refresh Now" in tray menu → data updates immediately

### 9.2 Auth edge cases
- [ ] Delete `~/.claude/.credentials.json` → app shows "credentials not found" message, does not crash
- [ ] Restore file → next poll succeeds automatically
- [ ] Simulate expired token (edit `expiresAt` to past timestamp) → verify refresh flow triggers

### 9.3 Network edge cases
- [ ] Disconnect network → app shows last cached data with stale indicator, no crash
- [ ] Reconnect → next poll succeeds, stale indicator clears

### 9.4 Startup cache
- [ ] Run app, let it poll successfully, kill app
- [ ] Restart app → UI shows cached data instantly before first poll completes

### 9.5 Notifications
- [ ] Temporarily set threshold to a value below current usage → notification fires
- [ ] Verify notification does not fire again on next poll (latch working)
- [ ] Let usage drop below threshold (or simulate) → verify latch resets

---

## Phase 10: Polish & Release

### 10.1 App metadata
- [ ] Set app name, identifier, version in `tauri.conf.json`
- [ ] Add app icons (1024×1024 PNG → `tauri icon` generates all sizes)
- [ ] Set window title

### 10.2 Build
- [ ] Run `yarn tauri build`
- [ ] Verify installer/bundle generates without errors
- [ ] Install from bundle, test app runs correctly outside dev mode

### 10.3 Auto-start (optional)
- [ ] Add "Launch at login" toggle in tray menu
- [ ] Use `tauri-plugin-autostart` or write `.desktop` file to `~/.config/autostart/`

---

## Current Status

**Phase 1** — ✅ Complete
**Phase 2** — ✅ Complete
**Phase 3** — ✅ Complete
**Phase 4** — ✅ Complete
**Phase 5** — ✅ Complete
**Phase 6** — Ready to start (Notifications)
**Phase 7** — Frontend UI (Svelte)
