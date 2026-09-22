//! Orchestrates a capture: topology -> frame -> overlay, then crop -> OCR ->
//! translate.
//!
//! The second half stops at "crop" for now: OCR arrives in M2 and translation
//! in M3 (docs/project/roadmap.md). Rather than fabricate a result, the crop
//! succeeds — proving the coordinate model end to end — and the caller gets a
//! typed "not implemented" error naming the milestone.

use std::time::Instant;

use tauri::AppHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::capture::{geometry::Rect, grab, monitors};
use crate::error::{AppError, CaptureError};
use crate::session::{Session, SessionStore};
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

/// Crop the selection out of the frozen frame.
///
/// The screen is NOT re-captured (invariant 3): this reads the buffer grabbed
/// at hotkey time, which is what makes the freeze real and re-selection free.
pub fn crop_selection(sessions: &SessionStore, selection: Rect) -> Result<(Rect, usize), AppError> {
    let started = Instant::now();

    let outcome = sessions.with(|session| {
        let cropped = session.frame.crop(&selection)?;
        let bounds = cropped.bounds;
        let bytes = cropped.pixels.len();
        session.last_crop = Some(cropped);
        Ok::<_, CaptureError>((bounds, bytes))
    });

    let Some(result) = outcome else {
        warn!("crop requested with no active session");
        return Err(CaptureError::NoSession.into());
    };

    let (bounds, bytes) = result?;

    info!(
        requested = ?selection,
        cropped = ?bounds,
        crop_bytes = bytes,
        crop_ms = started.elapsed().as_millis(),
        "crop_complete"
    );

    Ok((bounds, bytes))
}

/// The full pipeline. M1 proves the capture and coordinate half; the rest is
/// honest about not existing yet.
pub fn translate_region(
    app: &AppHandle,
    sessions: &SessionStore,
    selection: Rect,
) -> Result<serde_json::Value, AppError> {
    let _ = app;
    // Runs for real: a failure here is a genuine coordinate or capture bug and
    // the caller should see it rather than a blanket "not implemented".
    let (cropped, _bytes) = crop_selection(sessions, selection)?;

    warn!(
        cropped = ?cropped,
        "translate_region: crop succeeded, OCR not implemented (M2)"
    );

    Err(AppError::not_implemented("Text recognition", "M2 (OCR)"))
}
