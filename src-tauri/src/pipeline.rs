//! Orchestrates a capture: topology -> frame -> overlay, then crop ->
//! pre-process -> OCR -> translate.
//!
//! The chain stops at OCR for now: translation arrives in M3
//! (docs/project/roadmap.md). Rather than fabricate a translation, recognition
//! runs for real and is cached on the session, and the caller gets a typed
//! "not implemented" error naming the milestone.
//!
//! This module knows nothing about Tauri commands and nothing about the
//! webview; that direction is one-way on purpose (project-structure.md).

use std::time::Instant;

use tauri::AppHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::capture::{geometry::Rect, grab, monitors};
use crate::error::{AppError, CaptureError};
use crate::ocr::{self, OcrResult};
use crate::preprocess::{self, Preprocessed};
use crate::session::{Session, SessionStore};
use crate::settings;
use crate::windows_mgr;

/// Start a capture: read the layout, grab one frame, open the overlay.
///
/// Topology is read fresh every time and never cached (NFR-R4) — monitors get
/// unplugged between captures, and a stale layout means capturing the wrong
/// rectangle of a desktop that no longer exists.
pub fn begin_capture(app: &AppHandle, sessions: &SessionStore) -> Result<(), AppError> {
    let started = Instant::now();

    let topology = monitors::read_topology(app)?;
    let bounds = topology.virtual_bounds;

    let topology_ms = started.elapsed().as_millis();
    let grab_started = Instant::now();

    let frame = grab::capture(bounds)?;

    let capture_ms = grab_started.elapsed().as_millis();
    let id = Uuid::new_v4().to_string();

    // Shapes only — dimensions, counts, scale factors, timings. Never content.
    // scale_factors and monitor count are logged on purpose: mixed-DPI is this
    // project's worst bug class and these are what make a user's report
    // diagnosable (docs/operations/logging.md).
    info!(
        session = %id,
        monitors = topology.monitors.len(),
        scale_factors = ?topology.scale_factors(),
        mixed_dpi = topology.is_mixed_dpi(),
        virtual_bounds = ?bounds,
        frame_bytes = frame.pixels.len(),
        topology_ms,
        capture_ms,
        "capture_started"
    );

    sessions.set(Session::new(id, topology, frame));

    if let Err(e) = windows_mgr::open_overlay(app, bounds) {
        // Never leave a session holding a full-desktop buffer with no window
        // able to close it.
        sessions.clear();
        return Err(AppError::internal(format!("overlay window failed: {e}")));
    }

    Ok(())
}

/// End a capture: destroy the overlay and drop the buffers.
pub fn end_capture(app: &AppHandle, sessions: &SessionStore) {
    windows_mgr::close_overlay(app);
    sessions.clear();
}

/// Crop the selection out of the frozen frame and run the pre-processing
/// ladder over it.
///
/// The screen is NOT re-captured (invariant 3): this reads the buffer grabbed
/// at hotkey time, which is what makes the freeze real and re-selection free.
///
/// Both steps happen while the session lock is held, and OCR deliberately does
/// not: cropping and pre-processing are tens of milliseconds, recognition is
/// hundreds, and holding the lock across it would block `Esc` from closing the
/// overlay.
pub fn crop_and_prepare(
    sessions: &SessionStore,
    selection: Rect,
) -> Result<Preprocessed, AppError> {
    let started = Instant::now();

    let outcome = sessions.with(|session| {
        let cropped = session.frame.crop(&selection)?;
        let prepared = preprocess::prepare(&cropped);
        session.last_crop = Some(cropped);
        Ok::<_, CaptureError>(prepared)
    });

    let Some(result) = outcome else {
        warn!("crop requested with no active session");
        return Err(CaptureError::NoSession.into());
    };
    let prepared = result?;

    info!(
        requested = ?selection,
        cropped = ?prepared.source,
        image = ?(prepared.image.width, prepared.image.height),
        steps = %prepared.steps_label(),
        text_height_px = ?prepared.text_height_px,
        prepare_ms = started.elapsed().as_millis(),
        "crop_complete"
    );

    Ok(prepared)
}

/// Crop -> pre-process -> recognize. The M2 half of the pipeline.
///
/// The result is cached on the session so that re-translating into another
/// language costs one request and no second OCR pass (FR-54).
pub fn recognize_region(sessions: &SessionStore, selection: Rect) -> Result<OcrResult, AppError> {
    let prepared = crop_and_prepare(sessions, selection)?;

    // `None` means auto, which for this engine means the user's own profile
    // languages — Windows OCR recognizes *with* a language, it does not detect
    // one (ADR-0002).
    let requested = settings::load().settings.source_language;

    let started = Instant::now();
    let engine = ocr::default_engine();
    let result = ocr::recognize(&engine, &prepared, requested.as_deref())?;
    let ocr_ms = started.elapsed().as_millis();

    // Shapes only, never content (invariant 5): counts, languages and timings
    // are what make a bad recognition diagnosable from a user's log without
    // ever putting their screen in it.
    info!(
        engine = %result.engine,
        requested_lang = ?requested,
        source_lang = %result.source_lang,
        lines = result.lines.len(),
        blocks = result.block_count(),
        chars = result.full_text.chars().count(),
        ocr_ms,
        "ocr_complete"
    );

    sessions.with(|session| session.last_ocr = Some(result.clone()));

    Ok(result)
}

/// The full pipeline. M1 proves capture and coordinates, M2 adds recognition;
/// the last leg is honest about not existing yet.
pub fn translate_region(
    app: &AppHandle,
    sessions: &SessionStore,
    selection: Rect,
) -> Result<serde_json::Value, AppError> {
    let _ = app;
    // Runs for real, and the result is cached on the session: when translation
    // lands it reuses exactly this, and until then a failure here is a genuine
    // capture, coordinate or OCR bug the caller should see rather than a
    // blanket "not implemented".
    let recognized = recognize_region(sessions, selection)?;

    warn!(
        lines = recognized.lines.len(),
        "translate_region: recognition complete, translation not implemented (M3)"
    );

    Err(AppError::not_implemented("Translation", "M3 (translation)"))
}
