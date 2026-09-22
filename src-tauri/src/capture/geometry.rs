//! Pure rectangle math in the core's one coordinate space.
//!
//! Invariant 1 / ADR-0004: every coordinate here is a **physical pixel in
//! virtual-desktop space**, whose origin may be negative when a monitor sits
//! above or to the left of the primary. Nothing in this module knows what a
//! scale factor is — that conversion happens once, in `src/lib/coords.ts`.
//!
//! This is the Rust twin of `coords.ts`, and it is tested just as hard for the
//! same reason: a one-pixel error here is invisible on a single-monitor dev
//! machine and produces a crop that silently OCRs the wrong content.

use serde::{Deserialize, Serialize};

/// A rectangle in physical pixels. `x`/`y` may be negative.
///
/// Signed on purpose: `u32` here would be a bug waiting for the first user who
/// puts a monitor to the left of their primary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn left(&self) -> i32 {
        self.x
    }
    pub const fn top(&self) -> i32 {
        self.y
    }
    /// Exclusive right edge.
    pub const fn right(&self) -> i32 {
        self.x.saturating_add(self.w)
    }
    /// Exclusive bottom edge.
    pub const fn bottom(&self) -> i32 {
        self.y.saturating_add(self.h)
    }

    pub const fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Area as `u64` — a full virtual desktop overflows `i32` when multiplied.
    pub const fn area(&self) -> u64 {
        if self.is_empty() {
            0
        } else {
            (self.w as u64) * (self.h as u64)
        }
    }

    /// Build a positive-area rect from two corners, in any order.
    pub fn from_corners(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        let x = ax.min(bx);
        let y = ay.min(by);
        Self {
            x,
            y,
            w: (ax.max(bx)) - x,
            h: (ay.max(by)) - y,
        }
    }

    /// Intersection with `bounds`.
    ///
    /// Both dimensions collapse together when the rects do not overlap: rects
    /// that meet on only one axis have an empty intersection, and reporting the
    /// surviving axis invites a caller to treat it as real. Mirrors
    /// `clampRectToBounds` in `coords.ts`.
    pub fn intersect(&self, bounds: &Rect) -> Rect {
        let left = self.left().max(bounds.left());
        let top = self.top().max(bounds.top());
        let right = self.right().min(bounds.right());
        let bottom = self.bottom().min(bounds.bottom());

        let w = right - left;
        let h = bottom - top;
        if w <= 0 || h <= 0 {
            return Rect::new(left, top, 0, 0);
        }
        Rect::new(left, top, w, h)
    }

    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.left() && px < self.right() && py >= self.top() && py < self.bottom()
    }

    /// Re-express this rect relative to `origin`.
    ///
    /// The frame buffer's first pixel is the virtual desktop's top-left, so a
    /// selection in virtual-desktop space becomes a buffer offset by
    /// subtracting the virtual origin. This is the step that turns a possibly
    /// negative coordinate into a real index — and the step most likely to be
    /// skipped by someone who only ever tested on one monitor.
    pub fn to_local(&self, origin_x: i32, origin_y: i32) -> Rect {
        Rect::new(self.x - origin_x, self.y - origin_y, self.w, self.h)
    }

    /// The smallest rect containing both. Used to derive virtual-desktop bounds
    /// from a monitor list.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(left, top, right - left, bottom - top)
    }
}

/// FR-25: a selection under 8x8 physical pixels is a mis-click, not a request.
pub const MIN_SELECTION_PX: i32 = 8;

pub const fn is_usable_selection(rect: &Rect) -> bool {
    rect.w >= MIN_SELECTION_PX && rect.h >= MIN_SELECTION_PX
}

/// Union of all monitor rects — the virtual desktop.
pub fn virtual_bounds(monitors: &[Rect]) -> Rect {
    monitors
        .iter()
        .fold(Rect::new(0, 0, 0, 0), |acc, m| acc.union(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout that breaks naive implementations: a monitor to the LEFT of
    /// the primary, so the virtual desktop origin is negative.
    fn negative_origin_desktop() -> Rect {
        Rect::new(-1920, 0, 1920 + 2560, 1440)
    }

    #[test]
    fn from_corners_normalizes_every_drag_direction() {
        let expected = Rect::new(10, 20, 90, 80);
        assert_eq!(Rect::from_corners(10, 20, 100, 100), expected);
        assert_eq!(Rect::from_corners(100, 100, 10, 20), expected);
        assert_eq!(Rect::from_corners(100, 20, 10, 100), expected);
        assert_eq!(Rect::from_corners(10, 100, 100, 20), expected);
    }

    #[test]
    fn from_corners_handles_negative_coordinates() {
        assert_eq!(
            Rect::from_corners(-100, -50, -20, -10),
            Rect::new(-100, -50, 80, 40)
        );
    }

    #[test]
    fn a_click_is_a_zero_size_rect() {
        assert_eq!(Rect::from_corners(50, 50, 50, 50), Rect::new(50, 50, 0, 0));
        assert!(Rect::from_corners(50, 50, 50, 50).is_empty());
    }

    #[test]
    fn intersect_leaves_a_contained_rect_alone() {
        let bounds = negative_origin_desktop();
        let rect = Rect::new(0, 100, 200, 200);
        assert_eq!(rect.intersect(&bounds), rect);
    }

    #[test]
    fn intersect_trims_an_overhang_past_the_negative_origin() {
        let bounds = negative_origin_desktop();
        assert_eq!(
            Rect::new(-2200, 100, 400, 100).intersect(&bounds),
            Rect::new(-1920, 100, 120, 100)
        );
    }

    #[test]
    fn intersect_collapses_both_dimensions_when_only_one_axis_overlaps() {
        let bounds = negative_origin_desktop();
        // Overlaps on Y but not on X. Reporting h=100 here would let a caller
        // read a real intersection out of an empty one.
        let out = Rect::new(-5000, 0, 100, 100).intersect(&bounds);
        assert_eq!(out.w, 0);
        assert_eq!(out.h, 0);
        assert!(out.is_empty());

        let out = Rect::new(0, 9000, 100, 100).intersect(&bounds);
        assert_eq!(out.w, 0);
        assert_eq!(out.h, 0);
    }

    #[test]
    fn to_local_shifts_a_negative_origin_selection_into_buffer_space() {
        let bounds = negative_origin_desktop();
        // A selection on the left monitor, at virtual x = -1000.
        let selection = Rect::new(-1000, 200, 300, 100);
        let local = selection.to_local(bounds.x, bounds.y);
        assert_eq!(local, Rect::new(920, 200, 300, 100));
        // And it must land inside the buffer.
        assert!(local.x >= 0 && local.y >= 0);
        assert!(local.right() <= bounds.w && local.bottom() <= bounds.h);
    }

    #[test]
    fn to_local_is_identity_when_the_origin_is_zero() {
        let selection = Rect::new(100, 200, 300, 400);
        assert_eq!(selection.to_local(0, 0), selection);
    }

    #[test]
    fn clamp_then_localize_always_yields_in_bounds_buffer_coordinates() {
        let bounds = negative_origin_desktop();
        // Deliberately abusive selections, including ones fully outside.
        let candidates = [
            Rect::new(-5000, -5000, 10000, 10000),
            Rect::new(-1921, -1, 100, 100),
            Rect::new(2500, 1400, 500, 500),
            Rect::new(-1920, 0, 1, 1),
            Rect::new(639, 1439, 2, 2),
        ];
        for candidate in candidates {
            let clamped = candidate.intersect(&bounds);
            if clamped.is_empty() {
                continue;
            }
            let local = clamped.to_local(bounds.x, bounds.y);
            assert!(local.x >= 0, "{candidate:?} -> {local:?}");
            assert!(local.y >= 0, "{candidate:?} -> {local:?}");
            assert!(local.right() <= bounds.w, "{candidate:?} -> {local:?}");
            assert!(local.bottom() <= bounds.h, "{candidate:?} -> {local:?}");
        }
    }

    #[test]
    fn contains_point_treats_the_far_edges_as_exclusive() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        assert!(monitor.contains_point(0, 0));
        assert!(monitor.contains_point(1919, 1079));
        assert!(!monitor.contains_point(1920, 0));
        assert!(!monitor.contains_point(0, 1080));
    }

    #[test]
    fn contains_point_works_left_of_the_primary() {
        let left_monitor = Rect::new(-1920, 0, 1920, 1080);
        assert!(left_monitor.contains_point(-1, 500));
        assert!(left_monitor.contains_point(-1920, 0));
        assert!(!left_monitor.contains_point(0, 500));
    }

    #[test]
    fn virtual_bounds_spans_a_left_hand_monitor() {
        let monitors = [
            Rect::new(0, 0, 2560, 1440),     // primary laptop
            Rect::new(-1920, 0, 1920, 1080), // external, to the left
        ];
        assert_eq!(virtual_bounds(&monitors), Rect::new(-1920, 0, 4480, 1440));
    }

    #[test]
    fn virtual_bounds_spans_a_monitor_above_the_primary() {
        let monitors = [Rect::new(0, 0, 1920, 1080), Rect::new(0, -1080, 1920, 1080)];
        assert_eq!(virtual_bounds(&monitors), Rect::new(0, -1080, 1920, 2160));
    }

    #[test]
    fn virtual_bounds_of_a_single_monitor_is_that_monitor() {
        let monitors = [Rect::new(0, 0, 1920, 1080)];
        assert_eq!(virtual_bounds(&monitors), monitors[0]);
    }

    #[test]
    fn virtual_bounds_of_nothing_is_empty() {
        assert!(virtual_bounds(&[]).is_empty());
    }

    #[test]
    fn usable_selection_rejects_a_misclick() {
        assert!(!is_usable_selection(&Rect::new(0, 0, 0, 0)));
        assert!(!is_usable_selection(&Rect::new(0, 0, 7, 40)));
        assert!(is_usable_selection(&Rect::new(0, 0, 8, 8)));
    }

    #[test]
    fn area_does_not_overflow_on_a_large_desktop() {
        // Triple 4K: 11520 x 2160 = 24_883_200 px. Fine in u64, and the byte
        // count (x4) is what would overflow a careless i32.
        let huge = Rect::new(-3840, 0, 11520, 2160);
        assert_eq!(huge.area(), 24_883_200);
        assert_eq!(huge.area() * 4, 99_532_800);
    }
}
