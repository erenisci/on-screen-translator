//! The capture session: the one frame, the current crop, and the current result.
//!
//! Everything here lives in **memory only** and is dropped when the overlay
//! closes (NFR-S6, invariant 5). Nothing captured is ever written to disk —
//! not for caching, not for debugging, not temporarily.

use std::sync::{Arc, Mutex};

use crate::capture::{BgraImage, Rect, Topology};

/// One overlay session, from hotkey press to overlay close.
pub struct Session {
    pub id: String,
    pub topology: Topology,
    pub frame: BgraImage,
    /// The most recent crop, served at `otr://crop/<id>` during development so
    /// the coordinate model can be eyeballed without writing a file.
    pub last_crop: Option<BgraImage>,
}

impl Session {
    pub fn new(id: String, topology: Topology, frame: BgraImage) -> Self {
        Self {
            id,
            topology,
            frame,
            last_crop: None,
        }
    }

    pub fn virtual_bounds(&self) -> Rect {
        self.frame.bounds
    }
}

/// Shared handle to the active session, if any.
///
/// A plain `Mutex<Option<..>>` rather than anything cleverer: there is exactly
/// one session at a time and contention is a handful of calls per capture.
#[derive(Clone, Default)]
pub struct SessionStore(Arc<Mutex<Option<Session>>>);

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, session: Session) {
        *self.lock() = Some(session);
    }

    /// Drop the session and its buffers. Called on overlay close and on cancel.
    pub fn clear(&self) {
        *self.lock() = None;
    }

    pub fn is_active(&self) -> bool {
        self.lock().is_some()
    }

    pub fn id(&self) -> Option<String> {
        self.lock().as_ref().map(|s| s.id.clone())
    }

    /// Run `f` against the active session, if there is one.
    pub fn with<T>(&self, f: impl FnOnce(&mut Session) -> T) -> Option<T> {
        self.lock().as_mut().map(f)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Session>> {
        // A poisoned lock means a panic happened mid-capture. The session is
        // just pixels and is about to be replaced or cleared, so recovering is
        // strictly better than propagating the panic into the tray app.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
