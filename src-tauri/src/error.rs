//! The error taxonomy that crosses the IPC boundary.
//!
//! Two rules from docs/operations/error-handling.md shape this file:
//!
//! 1. **Errors are typed, not strings.** The UI matches on `ErrorCode` and
//!    never parses a message, so a new failure mode means a new code.
//! 2. **Every user-facing error names the cause AND the next action.**
//!    "Translation failed" is not acceptable.
//!
//! Module-local error types (`CaptureError`, `OcrError`, ...) stay local and
//! are mapped to `AppError` exactly once, here, so user-facing copy lives in
//! one place instead of scattered through the codebase.

use serde::{Deserialize, Serialize};

/// Stable, machine-readable. Must match `ErrorCode` in `src/lib/types.ts` and
/// the table in docs/architecture/api.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    CaptureFailed,
    NoTextFound,
    OcrLangMissing,
    OcrFailed,
    ProviderUnconfigured,
    ProviderAuth,
    ProviderRateLimit,
    ProviderUnavailable,
    HotkeyConflict,
    SettingsCorrupt,
    Internal,
}

/// What the frontend receives from every rejected command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    /// User-facing: what happened.
    pub message: String,
    /// User-facing: what to do about it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_url: Option<String>,
    /// Technical context for the log. Never shown by default.
    ///
    /// MUST NOT contain an API key, captured text, image data, or a user path
    /// outside the app's own directory (docs/operations/logging.md).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            action: None,
            action_url: None,
            detail: None,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_action_url(mut self, url: impl Into<String>) -> Self {
        self.action_url = Some(url.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, "Something went wrong inside the app.")
            .with_action("Try again. If it keeps happening, open the log from the tray menu.")
            .with_detail(detail)
    }

    pub fn capture_failed(detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::CaptureFailed, "Couldn't capture the screen.")
            // TD-07: exclusive fullscreen is the usual cause, and switching to
            // borderless is the only thing the user can actually do about it.
            .with_action(
                "If a game or video player is running fullscreen, try borderless windowed mode.",
            )
            .with_detail(detail)
    }

    /// Not every empty result is a failure: pointing at a blank area is normal
    /// use, so this stays a quiet, actionable message (FR-35).
    pub fn no_text_found() -> Self {
        Self::new(ErrorCode::NoTextFound, "No text found in that selection.")
            .with_action("Try selecting a slightly larger area.")
    }

    /// Not implemented yet — used while a milestone is still open.
    ///
    /// Better than a missing command, which surfaces to the user as an opaque
    /// "command not found" with no hint that the feature simply isn't built.
    pub fn not_implemented(what: &str, milestone: &str) -> Self {
        Self::new(
            ErrorCode::Internal,
            format!("{what} isn't implemented yet."),
        )
        .with_action(format!(
            "This arrives in {milestone}. See docs/project/roadmap.md."
        ))
        .with_detail(format!("unimplemented: {what} ({milestone})"))
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

/// Commands return this. Tauri serializes the `AppError` into the promise
/// rejection, which `toAppError` in `src/lib/ipc.ts` narrows back to a type.
pub type CommandResult<T> = Result<T, AppError>;

/// Failures inside the capture layer, before they get user-facing copy.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("no monitors reported by the system")]
    NoMonitors,

    #[error("virtual desktop has no area")]
    EmptyDesktop,

    #[error("selection lies outside the captured frame")]
    SelectionOutsideFrame,

    #[error("selection is too small to be intentional")]
    SelectionTooSmall,

    #[error("no capture session is active")]
    NoSession,

    #[error("win32 {call} failed: {detail}")]
    Win32 { call: &'static str, detail: String },

    #[error("frame is {actual} bytes, expected {expected}")]
    FrameSizeMismatch { actual: usize, expected: usize },
}

impl From<CaptureError> for AppError {
    fn from(err: CaptureError) -> Self {
        match err {
            // The user can act on these two, so they get their own copy.
            CaptureError::SelectionTooSmall => AppError::new(
                ErrorCode::CaptureFailed,
                "That selection was too small to read.",
            )
            .with_action("Drag a box around the text you want translated."),

            CaptureError::SelectionOutsideFrame => AppError::new(
                ErrorCode::CaptureFailed,
                "That selection is outside the captured screen.",
            )
            .with_action("Press the hotkey again and reselect."),

            // The rest are environmental or our own bugs: one honest message,
            // with the specifics kept for the log.
            other => AppError::capture_failed(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_to_the_wire_format_the_ui_matches_on() {
        // These strings are a contract with src/lib/types.ts. Changing one
        // without changing the other breaks error handling silently.
        let cases = [
            (ErrorCode::CaptureFailed, "\"CAPTURE_FAILED\""),
            (ErrorCode::NoTextFound, "\"NO_TEXT_FOUND\""),
            (ErrorCode::OcrLangMissing, "\"OCR_LANG_MISSING\""),
            (ErrorCode::OcrFailed, "\"OCR_FAILED\""),
            (ErrorCode::ProviderUnconfigured, "\"PROVIDER_UNCONFIGURED\""),
            (ErrorCode::ProviderAuth, "\"PROVIDER_AUTH\""),
            (ErrorCode::ProviderRateLimit, "\"PROVIDER_RATE_LIMIT\""),
            (ErrorCode::ProviderUnavailable, "\"PROVIDER_UNAVAILABLE\""),
            (ErrorCode::HotkeyConflict, "\"HOTKEY_CONFLICT\""),
            (ErrorCode::SettingsCorrupt, "\"SETTINGS_CORRUPT\""),
            (ErrorCode::Internal, "\"INTERNAL\""),
        ];
        for (code, expected) in cases {
            assert_eq!(serde_json::to_string(&code).unwrap(), expected);
        }
    }

    #[test]
    fn app_error_serializes_in_camel_case_and_omits_empty_fields() {
        let err = AppError::new(ErrorCode::NoTextFound, "nothing here");
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#"{"code":"NO_TEXT_FOUND","message":"nothing here"}"#);

        let err = err.with_action_url("https://example.invalid");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""actionUrl":"https://example.invalid""#));
    }

    #[test]
    fn every_actionable_capture_error_tells_the_user_what_to_do() {
        for err in [
            CaptureError::SelectionTooSmall,
            CaptureError::SelectionOutsideFrame,
            CaptureError::NoMonitors,
            CaptureError::EmptyDesktop,
        ] {
            let app: AppError = err.into();
            assert!(
                app.action.is_some(),
                "user-facing error without an action: {}",
                app.message
            );
        }
    }

    #[test]
    fn internal_details_are_kept_out_of_the_user_message() {
        let err = AppError::capture_failed("BitBlt returned 0, GetLastError=5");
        assert!(!err.message.contains("BitBlt"));
        assert!(err.detail.unwrap().contains("BitBlt"));
    }
}
