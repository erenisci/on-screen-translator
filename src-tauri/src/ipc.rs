//! Tauri commands — the contract in docs/architecture/api.md.
//!
//! Invariant 8: handlers contain **no logic**. They validate, delegate, and map
//! errors. Logic in a command handler is logic that cannot be tested without
//! Tauri, which is why `pipeline`, `settings` and `capture` hold it instead.
//!
//! Commands for milestones that are not built yet return a typed
//! "not implemented" error naming the milestone, rather than being absent —
//! a missing command surfaces to the user as an opaque failure with no hint
//! that the feature simply doesn't exist yet.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::warn;

use crate::capture::{geometry::Rect, MonitorInfo};
use crate::error::{AppError, CommandResult};
use crate::hotkey::{self, HotkeyResult, HotkeyState};
use crate::protocol;
use crate::session::SessionStore;
use crate::settings::{self, Settings};
use crate::windows_mgr;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFrameInfo {
    pub virtual_bounds: Rect,
    pub monitors: Vec<MonitorInfo>,
    pub frame_url: String,
    pub session_id: String,
    /// The overlay window's own scale factor. The frontend converts CSS to
    /// physical with THIS number and nothing else (ADR-0004).
    pub scale_factor: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub hotkey: String,
    pub degraded_reasons: Vec<String>,
}

pub fn emit_error(app: &AppHandle, error: &AppError) {
    let _ = app.emit("otr://error", error);
}

/* ----------------------------------------------------------- capture ---- */

#[tauri::command]
pub fn get_capture_frame(
    app: AppHandle,
    sessions: State<'_, SessionStore>,
) -> CommandResult<CaptureFrameInfo> {
    // The overlay's own scale factor, not any monitor's: Windows gives a window
    // one DPI even when it spans a 150% and a 100% display.
    let scale_factor = app
        .get_webview_window(windows_mgr::OVERLAY)
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);

    sessions
        .with(|session| CaptureFrameInfo {
            virtual_bounds: session.frame.bounds,
            monitors: session.topology.monitors.clone(),
            frame_url: protocol::frame_url(&session.id),
            session_id: session.id.clone(),
            scale_factor,
        })
        .ok_or_else(|| AppError::capture_failed("get_capture_frame called with no active session"))
}

#[tauri::command]
pub fn translate_region(
    app: AppHandle,
    sessions: State<'_, SessionStore>,
    rect: Rect,
) -> CommandResult<serde_json::Value> {
    crate::pipeline::translate_region(&app, &sessions, rect)
}

#[tauri::command]
pub fn retranslate(_target: String) -> CommandResult<serde_json::Value> {
    Err(AppError::not_implemented(
        "Re-translation",
        "M3 (translation)",
    ))
}

#[tauri::command]
pub fn cancel_capture(app: AppHandle, sessions: State<'_, SessionStore>) {
    crate::pipeline::end_capture(&app, &sessions);
}

#[tauri::command]
pub fn close_overlay(app: AppHandle, sessions: State<'_, SessionStore>) {
    crate::pipeline::end_capture(&app, &sessions);
}

#[tauri::command]
pub fn close_panel(app: AppHandle) {
    windows_mgr::close_panel(&app);
}

#[tauri::command]
pub fn get_panel_result() -> CommandResult<Option<serde_json::Value>> {
    // Nothing to show until the pipeline produces results (M3).
    Ok(None)
}

/* ---------------------------------------------------------- settings ---- */

#[tauri::command]
pub fn get_settings() -> CommandResult<Settings> {
    Ok(settings::load().settings)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, mut settings: Settings) -> CommandResult<Settings> {
    settings.normalize();

    // The hotkey applies live (FR-60). If the rebind fails we keep the working
    // binding and persist that, so the stored value never claims something
    // that isn't actually registered.
    let current = app.state::<HotkeyState>().active();
    if current.as_deref() != Some(settings.hotkey.as_str()) {
        let result = hotkey::register(&app, &settings.hotkey);
        if !result.ok && !result.active.is_empty() {
            settings.hotkey = result.active;
        }
        crate::tray::refresh_tooltip(&app);
    }

    settings
        .save()
        .map_err(|e| AppError::internal(format!("settings save failed: {e}")))?;

    let _ = app.emit("otr://settings-changed", &settings);
    Ok(settings)
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, accelerator: String) -> CommandResult<HotkeyResult> {
    let result = hotkey::register(&app, &accelerator);
    crate::tray::refresh_tooltip(&app);

    if result.ok {
        let mut current = settings::load().settings;
        current.hotkey = result.active.clone();
        if let Err(e) = current.save() {
            warn!(error = %e, "hotkey registered but settings save failed");
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn set_provider_key(_provider: String, _key: String) -> CommandResult<()> {
    Err(AppError::not_implemented(
        "Saving a provider key",
        "M3 (translation)",
    ))
}

#[tauri::command]
pub fn has_provider_key(_provider: String) -> CommandResult<bool> {
    Ok(false)
}

#[tauri::command]
pub fn clear_provider_key(_provider: String) -> CommandResult<()> {
    Err(AppError::not_implemented(
        "Clearing a provider key",
        "M3 (translation)",
    ))
}

#[tauri::command]
pub fn test_provider(_provider: String) -> CommandResult<serde_json::Value> {
    Err(AppError::not_implemented(
        "Testing a provider",
        "M3 (translation)",
    ))
}

#[tauri::command]
pub fn set_autostart(_enabled: bool) -> CommandResult<bool> {
    Err(AppError::not_implemented(
        "Start with Windows",
        "M5 (settings)",
    ))
}

#[tauri::command]
pub fn list_ocr_languages() -> CommandResult<Vec<String>> {
    // Honest empty list rather than a fabricated one: Settings renders this as
    // "no OCR language packs detected", which is true until M2.
    Ok(Vec::new())
}

/* ----------------------------------------------------------- utility ---- */

#[tauri::command]
pub fn copy_to_clipboard(_text: String) -> CommandResult<()> {
    Err(AppError::not_implemented(
        "Copying to the clipboard",
        "M4 (results)",
    ))
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> CommandResult<()> {
    windows_mgr::open_settings(&app)
        .map_err(|e| AppError::internal(format!("settings window failed: {e}")))
}

/// Allowlisted URLs only. An unrestricted "open this URL" command is a
/// phishing primitive (docs/operations/security.md).
#[tauri::command]
pub fn open_external(url: String) -> CommandResult<()> {
    const ALLOWED: &[&str] = &[
        "ms-settings:regionlanguage",
        "https://github.com/erenisci/on-screen-translator",
    ];
    if !ALLOWED.iter().any(|allowed| url.starts_with(allowed)) {
        return Err(AppError::internal(format!("blocked external url: {url}")));
    }
    Err(AppError::not_implemented(
        "Opening external links",
        "M5 (settings)",
    ))
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> CommandResult<AppInfo> {
    let hotkey = app.state::<HotkeyState>().active();
    let mut degraded_reasons = Vec::new();
    if hotkey.is_none() {
        degraded_reasons.push("No global hotkey is registered.".to_string());
    }
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        hotkey: hotkey.unwrap_or_default(),
        degraded_reasons,
    })
}
