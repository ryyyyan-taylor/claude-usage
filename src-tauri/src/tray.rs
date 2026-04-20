use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

/// Icon state based on usage percentage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Green,  // < 70%
    Yellow, // 70-89%
    Red,    // >= 90%
}

impl IconState {
    /// Get icon state from utilization (0.0-1.0)
    pub fn from_utilization(utilization: f64) -> Self {
        let pct = utilization * 100.0;
        if pct >= 90.0 {
            IconState::Red
        } else if pct >= 70.0 {
            IconState::Yellow
        } else {
            IconState::Green
        }
    }
}

/// Create and set up the system tray with menu
pub fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    // Create menu items
    let refresh = MenuItem::with_id(app, "refresh", "Refresh Now", true, None::<String>)?;
    let open_claude = MenuItem::with_id(app, "open_claude", "Open Claude.ai", true, None::<String>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<String>)?;

    // Create menu
    let menu = Menu::with_items(app, &[&refresh, &open_claude, &quit])?;

    // Create and show tray icon with menu
    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Claude Usage: Initializing...")
        .build(app)?;

    Ok(())
}

/// Update tray tooltip with current usage percentages
/// (For Tauri v2, tooltip updates should happen through menu state if needed)
pub fn format_tooltip(five_h_pct: u8, seven_d_pct: u8) -> String {
    format!("Claude: 5h {}% | 7d {}%", five_h_pct, seven_d_pct)
}
