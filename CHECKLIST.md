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
- [ ] Define `UsageSnapshot`:
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
- [ ] Implement `From<UsageResponse> for UsageSnapshot` to convert API response
- [ ] All structs derive `Serialize, Deserialize, Clone`

### 3.2 AppState (`state.rs`)
- [ ] Define `AppState`:
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
- [ ] Implement `AppState::new() -> Self` with all defaults
- [ ] Add `AppState::is_stale(&self) -> bool` — true if `last_refreshed` > 10 minutes ago

### 3.3 Snapshot cache (`state.rs`)
- [ ] Implement `cache_path() -> PathBuf`:
  - `dirs::cache_dir()` / `claude-usage` / `snapshot.json`
  - Create parent dir if missing
- [ ] Implement `load_cached() -> Option<UsageSnapshot>`:
  - Read file, deserialize JSON, return None on any error
- [ ] Implement `save_cache(snapshot: &UsageSnapshot)`:
  - Serialize to pretty JSON, write file, ignore errors silently
- [ ] **Test**: Save a dummy snapshot, kill and restart app, verify it loads the cache

---

## Phase 4: Polling Loop (Rust)

### 4.1 Poller setup (`poller.rs`)
- [ ] Implement `pub async fn start_poller(state: Arc<Mutex<AppState>>, app: AppHandle)`:
  - Loop with `tokio::time::sleep(Duration::from_secs(60))`
  - Set `is_refreshing = true` before each fetch
  - On success: update snapshot, save cache, emit `usage_updated` event, call `check_thresholds`
  - On `AuthRequired`: set `auth_error = true`, emit `auth_error` event
  - On `RateLimited`: skip next 3 cycles (track counter), log warning
  - On other error: log error, preserve existing snapshot, set `is_refreshing = false`

### 4.2 Tauri commands (`main.rs`)
- [ ] Add `#[tauri::command] async fn get_snapshot(state: State<...>) -> Option<UsageSnapshot>`
- [ ] Add `#[tauri::command] async fn refresh_now(state: State<...>, app: AppHandle) -> Result<(), String>`
- [ ] Register both commands in `tauri::Builder`
- [ ] **Test**: Open Tauri devtools, call `invoke('get_snapshot')` from JS console, verify data returns

### 4.3 Wire up startup (`main.rs`)
- [ ] On app ready:
  1. Load cached snapshot into `AppState`
  2. Emit `usage_updated` with cached data (instant UI)
  3. Spawn `start_poller` as background Tokio task
- [ ] **Test**: Start app, verify tray appears, check console for first poll completing

---

## Phase 5: System Tray (Rust)

### 5.1 Basic tray
- [ ] Create tray icon SVG assets (three states: green/yellow/red)
- [ ] Register tray in `tauri.conf.json` with default icon
- [ ] Set up `SystemTray` in `main.rs` with initial tooltip `"Claude: loading..."`

### 5.2 Tray menu
- [ ] Add menu items: `"Refresh Now"`, `"Open Claude.ai"`, `"Quit"`
- [ ] Handle `MenuItemClick`:
  - `refresh_now` → invoke poller manually
  - `open_claude` → `tauri::api::shell::open("https://claude.ai")`
  - `quit` → `std::process::exit(0)`
- [ ] Handle `LeftClick` → show and focus main window

### 5.3 Dynamic tray updates
- [ ] Implement `update_tray(app: &AppHandle, snapshot: &UsageSnapshot)`:
  - Format tooltip: `"Claude: 5h 42% | 7d 18%"`
  - Select icon based on `five_hour.utilization`: <0.7 green, <0.9 yellow, else red
  - Call `app.tray_handle().set_tooltip()` and `set_icon()`
- [ ] Call `update_tray` after every successful poll
- [ ] **Test**: Watch tray tooltip update after first successful poll

### 5.4 Hide on close
- [ ] In `on_window_event`:
  - Match `WindowEvent::CloseRequested` → `window.hide()` + `api.prevent_close()`
- [ ] **Test**: Click X button — window hides, tray icon remains, click tray → window reappears

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
**Phase 2** — In progress (2.1–2.7 implemented, manual testing pending)
**Phase 3** — Ready to start (State & cache layer)
