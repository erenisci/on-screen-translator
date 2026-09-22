//! Monitor topology.
//!
//! Read fresh on **every** capture, never cached (NFR-R4): monitors get
//! plugged, unplugged and rescaled between captures, and a stale layout means a
//! crop from the wrong place.
//!
//! Deliberately uses Tauri's monitor API rather than `EnumDisplayMonitors`:
//! Tauri already reports physical position, physical size and scale factor,
//! which is exactly what we need, and it keeps this module free of `unsafe`.
//! The only place we reach for Win32 is the pixel grab itself.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::geometry::{virtual_bounds, Rect};
use crate::error::CaptureError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    /// Physical pixels, virtual-desktop space.
    pub bounds: Rect,
    /// 1.0 = 100%, 1.5 = 150%.
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// The current display layout.
#[derive(Debug, Clone)]
pub struct Topology {
    pub monitors: Vec<MonitorInfo>,
    /// Union of every monitor. Origin may be negative.
    pub virtual_bounds: Rect,
}

/// Enumerate monitors and compute the virtual desktop.
pub fn read_topology(app: &AppHandle) -> Result<Topology, CaptureError> {
    let available = app.available_monitors().map_err(|e| CaptureError::Win32 {
        call: "available_monitors",
        detail: e.to_string(),
    })?;

    if available.is_empty() {
        return Err(CaptureError::NoMonitors);
    }

    let primary = app.primary_monitor().ok().flatten();
    let primary_name = primary.as_ref().and_then(|m| m.name()).cloned();

    let monitors: Vec<MonitorInfo> = available
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let position = monitor.position();
            let size = monitor.size();
            let name = monitor.name().cloned();

            MonitorInfo {
                // Tauri does not always give a name; fall back to an index so
                // the id is still stable within one capture session.
                id: name.clone().unwrap_or_else(|| format!("monitor-{index}")),
                bounds: Rect::new(
                    position.x,
                    position.y,
                    size.width as i32,
                    size.height as i32,
                ),
                scale_factor: monitor.scale_factor(),
                is_primary: match (&name, &primary_name) {
                    (Some(a), Some(b)) => a == b,
                    // No names to compare: the monitor at the origin is the
                    // primary by Windows' own definition.
                    _ => position.x == 0 && position.y == 0,
                },
            }
        })
        .collect();

    let rects: Vec<Rect> = monitors.iter().map(|m| m.bounds).collect();
    let bounds = virtual_bounds(&rects);

    if bounds.is_empty() {
        return Err(CaptureError::EmptyDesktop);
    }

    Ok(Topology {
        monitors,
        virtual_bounds: bounds,
    })
}

impl Topology {
    /// Scale factors present in this layout, for the capture log line.
    ///
    /// Logged on every capture because mixed-DPI is this project's worst bug
    /// class, and knowing the layout is what makes a user's report diagnosable
    /// without asking them to describe their desk (docs/operations/logging.md).
    pub fn scale_factors(&self) -> Vec<f64> {
        self.monitors.iter().map(|m| m.scale_factor).collect()
    }

    pub fn is_mixed_dpi(&self) -> bool {
        let mut factors = self.scale_factors();
        factors.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
        factors.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topology_of(monitors: Vec<(Rect, f64)>) -> Topology {
        let monitors: Vec<MonitorInfo> = monitors
            .into_iter()
            .enumerate()
            .map(|(i, (bounds, scale_factor))| MonitorInfo {
                id: format!("m{i}"),
                bounds,
                scale_factor,
                is_primary: i == 0,
            })
            .collect();
        let rects: Vec<Rect> = monitors.iter().map(|m| m.bounds).collect();
        let virtual_bounds = virtual_bounds(&rects);
        Topology {
            monitors,
            virtual_bounds,
        }
    }

    #[test]
    fn detects_a_mixed_dpi_layout() {
        let topology = topology_of(vec![
            (Rect::new(0, 0, 2560, 1440), 1.5),
            (Rect::new(-1920, 0, 1920, 1080), 1.0),
        ]);
        assert!(topology.is_mixed_dpi());
        assert_eq!(topology.virtual_bounds, Rect::new(-1920, 0, 4480, 1440));
    }

    #[test]
    fn a_uniform_layout_is_not_mixed_dpi() {
        let topology = topology_of(vec![
            (Rect::new(0, 0, 1920, 1080), 1.0),
            (Rect::new(1920, 0, 1920, 1080), 1.0),
        ]);
        assert!(!topology.is_mixed_dpi());
    }

    #[test]
    fn a_single_monitor_is_not_mixed_dpi() {
        let topology = topology_of(vec![(Rect::new(0, 0, 1920, 1080), 1.25)]);
        assert!(!topology.is_mixed_dpi());
    }
}
