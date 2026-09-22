//! The global shortcut.
//!
//! FR-12 is the rule that shapes this module: if a requested binding is already
//! claimed by another process, registration fails **visibly** and the previous
//! binding is kept. The app must never end up silently unbound — a tray app
//! with no working hotkey is indistinguishable from a broken one.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tracing::{info, warn};

use crate::session::SessionStore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyResult {
    pub ok: bool,
    /// The binding actually in effect — the previous one if registration failed.
    pub active: String,
    pub message: Option<String>,
}

/// Remembers what is currently registered so a failed rebind can be reported
/// against the truth rather than against what the user typed.
#[derive(Clone, Default)]
pub struct HotkeyState(Arc<Mutex<Option<String>>>);

impl HotkeyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active(&self) -> Option<String> {
        self.lock().clone()
    }

    fn set(&self, accelerator: Option<String>) {
        *self.lock() = accelerator;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn parse(accelerator: &str) -> Result<Shortcut, String> {
    let trimmed = accelerator.trim();
    if trimmed.is_empty() {
        return Err("Hotkey cannot be empty.".into());
    }
    // A modifier-less binding would hijack every keystroke in every app.
    if !trimmed.contains('+') {
        return Err("A hotkey needs at least one modifier, for example Ctrl+Shift+T.".into());
    }
    Shortcut::from_str(trimmed).map_err(|e| format!("\"{trimmed}\" isn't a valid hotkey ({e})."))
}

/// Register `accelerator`, replacing whatever is currently bound.
///
/// On failure the previous binding is restored, so a bad rebind costs the user
/// nothing.
pub fn register(app: &AppHandle, accelerator: &str) -> HotkeyResult {
    let state = app.state::<HotkeyState>();
    let previous = state.active();

    let shortcut = match parse(accelerator) {
        Ok(shortcut) => shortcut,
        Err(message) => {
            return HotkeyResult {
                ok: false,
                active: previous.unwrap_or_default(),
                message: Some(message),
            }
        }
    };

    // Release the old binding first; holding both would leave a stale global
    // hook if the new one succeeds.
    if let Some(old) = &previous {
        if let Ok(old_shortcut) = parse(old) {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
    }

    let handle = app.clone();
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_, _, event| {
            // Edge-triggered: holding the keys must not queue repeat captures (F3).
            if event.state() != ShortcutState::Pressed {
                return;
            }
            on_triggered(&handle);
        });

    match result {
        Ok(()) => {
            info!(accelerator, "hotkey_registered");
            state.set(Some(accelerator.trim().to_string()));
            HotkeyResult {
                ok: true,
                active: accelerator.trim().to_string(),
                message: None,
            }
        }
        Err(e) => {
            warn!(accelerator, error = %e, "hotkey_registration_failed");

            // Put the old binding back so the app stays usable.
            let mut restored = String::new();
            if let Some(old) = previous {
                if let Ok(old_shortcut) = parse(&old) {
                    let handle = app.clone();
                    if app
                        .global_shortcut()
                        .on_shortcut(old_shortcut, move |_, _, event| {
                            if event.state() == ShortcutState::Pressed {
                                on_triggered(&handle);
                            }
                        })
                        .is_ok()
                    {
                        restored = old.clone();
                        state.set(Some(old));
                    }
                }
            }

            HotkeyResult {
                ok: false,
                active: restored,
                message: Some(format!(
                    "{accelerator} is already used by another app. Pick a different combination."
                )),
            }
        }
    }
}

/// What the hotkey does. Kept here so the tray and the shortcut share one path.
fn on_triggered(app: &AppHandle) {
    let sessions = app.state::<SessionStore>();

    // F3: pressing the hotkey while an overlay is open is a no-op, not a
    // nested capture.
    if sessions.is_active() {
        return;
    }

    if let Err(e) = crate::pipeline::begin_capture(app, &sessions) {
        warn!(error = %e, "capture_failed_to_start");
        crate::ipc::emit_error(app, &e);
    }
}

/// Trigger a capture from the tray, taking the same path as the hotkey.
pub fn trigger(app: &AppHandle) {
    on_triggered(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_binding() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn rejects_a_modifierless_binding() {
        // Binding a bare letter would swallow typing in every application.
        let err = parse("T").unwrap_err();
        assert!(err.contains("modifier"));
        assert!(parse("F1").is_err());
    }

    #[test]
    fn accepts_the_default_binding() {
        assert!(parse("Ctrl+Shift+T").is_ok());
        assert!(parse("  Ctrl+Shift+T  ").is_ok());
    }

    #[test]
    fn accepts_other_reasonable_combinations() {
        assert!(parse("Ctrl+Alt+Q").is_ok());
        assert!(parse("Alt+Shift+1").is_ok());
    }

    #[test]
    fn rejects_nonsense_that_merely_contains_a_plus() {
        assert!(parse("Ctrl+NotARealKey").is_err());
    }

    #[test]
    fn state_starts_unbound_and_remembers_what_was_set() {
        let state = HotkeyState::new();
        assert_eq!(state.active(), None);
        state.set(Some("Ctrl+Shift+T".into()));
        assert_eq!(state.active().as_deref(), Some("Ctrl+Shift+T"));
    }
}
