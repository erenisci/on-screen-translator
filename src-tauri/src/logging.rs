//! Tracing setup, and the rule that shapes this whole module.
//!
//! > Captured content — recognized text, translated text, and image data —
//! > must never be written to a log, at any level, including `trace`.
//!
//! There is no debug flag that turns that off. People capture banking pages and
//! private messages; a log file sits on disk and gets attached to bug reports.
//! We log **shapes** instead: counts, dimensions, durations, confidences,
//! language tags. See docs/operations/logging.md.

use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Wraps captured text so logging it accidentally produces a safe line instead
/// of a leak. The type carries the rule so a tired developer doesn't have to.
pub struct Redacted<T>(pub T);

impl<T: AsRef<str>> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[redacted: {} chars]", self.0.as_ref().chars().count())
    }
}

impl<T: AsRef<str>> std::fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub fn log_dir() -> PathBuf {
    // Deliberately not tauri's path resolver: logging starts before the app is
    // built, so it cannot depend on an AppHandle existing yet.
    let base = std::env::var("OTR_DEV_LOG_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("APPDATA").ok().map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("on-screen-translator").join("logs")
}

/// Initialize tracing. The returned guard must be held for the process
/// lifetime — dropping it stops the background writer and loses buffered lines.
#[must_use]
pub fn init() -> Option<WorkerGuard> {
    let filter =
        EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new("otr_lib=info,warn"));

    let dir = log_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        // Without a log directory we still want a usable app, so fall back to
        // stderr rather than refusing to start.
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
        return None;
    }

    // Daily rotation. Retention is capped so a logging tool never quietly eats
    // a user's disk (docs/operations/logging.md).
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("otr")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir)
        .ok()?;

    let (writer, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_ansi(false).with_target(true))
        .with(fmt::layer().json().with_writer(writer).with_ansi(false))
        .init();

    Some(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_never_reveals_its_contents() {
        let secret = Redacted("the user's bank balance is 42");
        let debug = format!("{secret:?}");
        let display = format!("{secret}");

        for rendered in [&debug, &display] {
            assert!(!rendered.contains("bank"));
            assert!(!rendered.contains("42"));
            assert!(rendered.contains("redacted"));
        }
    }

    #[test]
    fn redacted_reports_a_useful_shape() {
        // Character count, not byte count: "kaç karakter geldi" is the
        // diagnostic question, and it must be right for non-ASCII text too.
        assert_eq!(format!("{:?}", Redacted("merhaba")), "[redacted: 7 chars]");
        assert_eq!(format!("{:?}", Redacted("şğüöçı")), "[redacted: 6 chars]");
        assert_eq!(format!("{:?}", Redacted("")), "[redacted: 0 chars]");
    }

    #[test]
    fn log_dir_is_under_the_app_namespace() {
        let dir = log_dir();
        assert!(dir.ends_with("logs"));
        assert!(dir.to_string_lossy().contains("on-screen-translator"));
    }
}
