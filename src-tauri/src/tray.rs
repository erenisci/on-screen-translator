//! The tray icon — the app's entire permanent surface (F1).
//!
//! There is no main window and no taskbar entry. Everything the user can reach
//! without the hotkey, they reach from here.

use tauri::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tracing::{info, warn};

use crate::hotkey::{self, HotkeyState};
use crate::windows_mgr;

const ID_CAPTURE: &str = "capture";
const ID_SETTINGS: &str = "settings";
const ID_QUIT: &str = "quit";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItem::with_id(app, ID_CAPTURE, "Capture", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, ID_SETTINGS, "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&capture, &settings, &separator, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?,
        )
        .menu(&menu)
        // Left-click captures; the menu is right-click only (FR-03).
        .show_menu_on_left_click(false)
        .tooltip(tooltip(app))
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_event)
        .build(app)?;

    Ok(())
}

/// The tooltip carries the current hotkey so the user can always recall the
/// binding without opening Settings (F1).
fn tooltip(app: &AppHandle) -> String {
    match app.try_state::<HotkeyState>().and_then(|s| s.active()) {
        Some(accelerator) => format!("on-screen-translator — {accelerator}"),
        None => "on-screen-translator — no hotkey bound".to_string(),
    }
}

/// Refresh the tooltip after a rebind.
pub fn refresh_tooltip(app: &AppHandle) {
    use tauri::tray::TrayIconId;
    if let Some(tray) = app.tray_by_id(&TrayIconId::new("main")) {
        let _ = tray.set_tooltip(Some(tooltip(app)));
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        ID_CAPTURE => hotkey::trigger(app),
        ID_SETTINGS => {
            if let Err(e) = windows_mgr::open_settings(app) {
                warn!(error = %e, "settings_window_failed");
            }
        }
        ID_QUIT => {
            info!("quit_requested");
            // Cancel anything in flight and tear down every window first, so
            // quitting during a capture cannot leave an orphan process or a
            // full-screen window over the desktop (NFR-R5).
            if let Some(sessions) = app.try_state::<crate::session::SessionStore>() {
                crate::pipeline::end_capture(app, &sessions);
            }
            windows_mgr::close_panel(app);
            app.exit(0);
        }
        other => warn!(id = other, "unknown tray menu item"),
    }
}

fn on_tray_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    // FR-03: left-click triggers a capture.
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        hotkey::trigger(tray.app_handle());
    }
}
