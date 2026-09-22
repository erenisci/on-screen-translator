//! The `otr://` handler that serves the frozen frame to the overlay.
//!
//! This exists purely for speed (ADR-0004): base64-encoding a multi-megabyte
//! frame through the JSON IPC channel would cost more than the entire 250 ms
//! budget for getting the overlay on screen.
//!
//! It is read-only, serves exactly two in-memory resources scoped to the active
//! session, and **must never become a general file server**.

use tauri::http::{Request, Response, StatusCode};

use crate::session::SessionStore;

pub const SCHEME: &str = "otr";

/// The URL the overlay should load for the current frame.
///
/// Windows serves custom schemes over `http://<scheme>.localhost/...` rather
/// than `otr://...`, so the URL is built here instead of being assembled in the
/// frontend from a scheme it would have to know about.
pub fn frame_url(session_id: &str) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/frame/{session_id}")
    } else {
        format!("{SCHEME}://frame/{session_id}")
    }
}

/// The most recent crop. Development aid: it lets the coordinate model be
/// verified visually without writing a file, which invariant 5 forbids.
pub fn crop_url(session_id: &str) -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/crop/{session_id}")
    } else {
        format!("{SCHEME}://crop/{session_id}")
    }
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "text/plain")
        .body(b"not found".to_vec())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn bmp(body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "image/bmp")
        // The frame changes every session and must never be reused across them.
        .header("cache-control", "no-store")
        .body(body)
        .unwrap_or_else(|_| not_found())
}

/// Parse `/frame/<id>` or `/crop/<id>` out of a request path.
fn route(path: &str) -> Option<(&str, &str)> {
    let trimmed = path.trim_start_matches('/');
    let (kind, id) = trimmed.split_once('/')?;
    if id.is_empty() {
        return None;
    }
    Some((kind, id))
}

pub fn handle(sessions: &SessionStore, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().to_string();
    let Some((kind, id)) = route(&path) else {
        return not_found();
    };

    let body = sessions.with(|session| {
        // Scoped to the active session: once it ends, the buffers are gone and
        // this 404s rather than serving a stale screenshot.
        if session.id != id {
            return None;
        }
        match kind {
            "frame" => Some(session.frame.to_bmp()),
            "crop" => session.last_crop.as_ref().map(|c| c.to_bmp()),
            _ => None,
        }
    });

    match body.flatten() {
        Some(bytes) => bmp(bytes),
        None => not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_frame_and_crop_paths() {
        assert_eq!(route("/frame/abc123"), Some(("frame", "abc123")));
        assert_eq!(route("/crop/abc123"), Some(("crop", "abc123")));
        assert_eq!(route("frame/abc123"), Some(("frame", "abc123")));
    }

    #[test]
    fn rejects_paths_without_a_session_id() {
        assert_eq!(route("/frame/"), None);
        assert_eq!(route("/frame"), None);
        assert_eq!(route("/"), None);
        assert_eq!(route(""), None);
    }

    #[test]
    fn urls_carry_the_session_id() {
        let id = "d3adb33f";
        assert!(frame_url(id).ends_with(&format!("/frame/{id}")));
        assert!(crop_url(id).ends_with(&format!("/crop/{id}")));
    }

    #[test]
    fn an_inactive_session_serves_nothing() {
        let sessions = SessionStore::new();
        let request = Request::builder()
            .uri("http://otr.localhost/frame/anything")
            .body(Vec::new())
            .unwrap();
        assert_eq!(handle(&sessions, &request).status(), StatusCode::NOT_FOUND);
    }
}
