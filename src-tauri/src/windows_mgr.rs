//! Window lifecycle.
//!
//! All three windows are created on demand and destroyed after use. That is not
//! tidiness — it is how the 30 MB idle budget is met (NFR-P4): a resident
//! WebView2 process would blow it several times over for an app that is idle
//! 99% of the time. Cost: ~100-200 ms on the first capture, accepted as TD-04.

use tauri::{AppHandle, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::capture::Rect;

pub const OVERLAY: &str = "overlay";
pub const PANEL: &str = "panel";
pub const SETTINGS: &str = "settings";

/// Create the capture overlay spanning the whole virtual desktop.
///
/// `virtual_bounds` is in physical pixels and its origin may be negative, so
/// position and size are both set in physical units — passing logical units
/// here is the classic way to land the overlay on the wrong monitor.
pub fn open_overlay(app: &AppHandle, virtual_bounds: Rect) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window(OVERLAY) {
        // The hotkey is a no-op while an overlay is already up (F3), so this
        // only happens if a previous one failed to close. Reuse it.
        existing.show()?;
        existing.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App("overlay.html".into()))
        .title("on-screen-translator")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        .visible(false)
        .focused(true)
        .build()?;

    // Physical units throughout: the window must cover the virtual desktop
    // exactly, including a negative origin.
    window.set_position(tauri::PhysicalPosition::new(
        virtual_bounds.x,
        virtual_bounds.y,
    ))?;
    window.set_size(tauri::PhysicalSize::new(
        virtual_bounds.w as u32,
        virtual_bounds.h as u32,
    ))?;

    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// Destroy the overlay. Closing it must always be possible (NFR-R1).
pub fn close_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY) {
        let _ = window.destroy();
    }
}

pub fn open_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window(PANEL) {
        existing.show()?;
        existing.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, PANEL, WebviewUrl::App("panel.html".into()))
        .title("Translation")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .inner_size(420.0, 320.0)
        .min_inner_size(280.0, 180.0)
        .resizable(true)
        .build()?;

    Ok(())
}

pub fn close_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL) {
        let _ = window.destroy();
    }
}

/// NFR-U3: the settings window fits one small surface with no scrolling at a
/// 900px-tall display.
pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window(SETTINGS) {
        existing.show()?;
        existing.unminimize()?;
        existing.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("on-screen-translator — Settings")
        .inner_size(480.0, 620.0)
        .resizable(true)
        .minimizable(true)
        .maximizable(false)
        .center()
        .build()?;

    window.set_min_size(Some(LogicalSize::new(420.0, 480.0)))?;
    Ok(())
}
