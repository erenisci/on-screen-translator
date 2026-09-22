/**
 * THE conversion boundary between the two pixel spaces in this project.
 *
 * Invariant 1 (CLAUDE.md) / ADR-0004: core coordinates are PHYSICAL pixels in
 * virtual-desktop space, whose origin may be negative. CSS pixels exist only
 * inside a webview. Conversion happens here and NOWHERE ELSE — if you find
 * yourself multiplying by a scale factor in another file, that is the bug.
 *
 * Why one scale factor for a window spanning mixed-DPI monitors: Windows gives
 * a window a single DPI, so WebView2 reports one devicePixelRatio for the whole
 * overlay even when it spans a 150% and a 100% display. The frozen frame is
 * therefore rendered at 1:1 physical pixels and the overlay's own scale factor
 * is the only ratio that matters. Per-monitor scale factors are carried in
 * MonitorInfo for display purposes only — never for conversion.
 *
 * Everything here is pure. It is the cheapest insurance against the bug class
 * that ruins every tool in this category, so it is tested exhaustively.
 */

import type { Point, Rect } from './types';

/** What the overlay needs to convert. Comes from CaptureFrameInfo. */
export interface OverlayView {
  /** The virtual desktop in physical pixels. x/y may be negative. */
  virtualBounds: Rect;
  /** The overlay window's own scale factor (devicePixelRatio). */
  scaleFactor: number;
}

function assertUsableScale(scaleFactor: number): void {
  if (!Number.isFinite(scaleFactor) || scaleFactor <= 0) {
    throw new RangeError(`scaleFactor must be a positive finite number, got ${scaleFactor}`);
  }
}

/** CSS point inside the overlay -> physical point in virtual-desktop space. */
export function cssToPhysicalPoint(point_css: Point, view: OverlayView): Point {
  assertUsableScale(view.scaleFactor);
  return {
    x: Math.round(point_css.x * view.scaleFactor) + view.virtualBounds.x,
    y: Math.round(point_css.y * view.scaleFactor) + view.virtualBounds.y,
  };
}

/** Physical point in virtual-desktop space -> CSS point inside the overlay. */
export function physicalToCssPoint(point_physical: Point, view: OverlayView): Point {
  assertUsableScale(view.scaleFactor);
  return {
    x: (point_physical.x - view.virtualBounds.x) / view.scaleFactor,
    y: (point_physical.y - view.virtualBounds.y) / view.scaleFactor,
  };
}

/**
 * CSS rect -> physical rect.
 *
 * Edges are converted and rounded independently, then the size is derived from
 * them. Rounding the origin and the size separately lets width drift by a pixel
 * at fractional scale factors, which is exactly how a crop ends up one pixel
 * short of the text it was supposed to contain.
 */
export function cssToPhysicalRect(rect_css: Rect, view: OverlayView): Rect {
  assertUsableScale(view.scaleFactor);
  const left = Math.round(rect_css.x * view.scaleFactor) + view.virtualBounds.x;
  const top = Math.round(rect_css.y * view.scaleFactor) + view.virtualBounds.y;
  const right = Math.round((rect_css.x + rect_css.w) * view.scaleFactor) + view.virtualBounds.x;
  const bottom = Math.round((rect_css.y + rect_css.h) * view.scaleFactor) + view.virtualBounds.y;
  return { x: left, y: top, w: right - left, h: bottom - top };
}

/** Physical rect -> CSS rect. Not rounded: CSS handles fractional pixels. */
export function physicalToCssRect(rect_physical: Rect, view: OverlayView): Rect {
  assertUsableScale(view.scaleFactor);
  return {
    x: (rect_physical.x - view.virtualBounds.x) / view.scaleFactor,
    y: (rect_physical.y - view.virtualBounds.y) / view.scaleFactor,
    w: rect_physical.w / view.scaleFactor,
    h: rect_physical.h / view.scaleFactor,
  };
}

/**
 * Build a positive-area rect from two drag corners.
 *
 * A drag can start at any corner and move in any direction, so the raw anchor
 * and cursor points routinely produce negative width or height.
 */
export function rectFromCorners(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return { x, y, w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) };
}

/**
 * Intersect a rect with bounds. Returns a zero-SIZE rect when they don't overlap.
 *
 * Note both dimensions collapse together: rects that overlap on one axis only
 * have an empty intersection, and reporting the surviving axis (`w: 0, h: 100`)
 * invites a caller to treat that 100 as meaningful. Empty is empty.
 */
export function clampRectToBounds(rect: Rect, bounds: Rect): Rect {
  const left = Math.max(rect.x, bounds.x);
  const top = Math.max(rect.y, bounds.y);
  const right = Math.min(rect.x + rect.w, bounds.x + bounds.w);
  const bottom = Math.min(rect.y + rect.h, bounds.y + bounds.h);

  const w = right - left;
  const h = bottom - top;
  if (w <= 0 || h <= 0) return { x: left, y: top, w: 0, h: 0 };

  return { x: left, y: top, w, h };
}

/**
 * FR-25: a selection under 8x8 physical pixels is a mis-click, not a request.
 * Measured in physical pixels so the threshold means the same thing on a 150%
 * display as on a 100% one.
 */
export const MIN_SELECTION_PX = 8;

export function isUsableSelection(rect_physical: Rect): boolean {
  return rect_physical.w >= MIN_SELECTION_PX && rect_physical.h >= MIN_SELECTION_PX;
}

/** Which monitor contains a physical point, if any. For diagnostics and display. */
export function monitorAtPoint<T extends { bounds: Rect }>(
  point_physical: Point,
  monitors: readonly T[],
): T | undefined {
  return monitors.find(
    (m) =>
      point_physical.x >= m.bounds.x &&
      point_physical.x < m.bounds.x + m.bounds.w &&
      point_physical.y >= m.bounds.y &&
      point_physical.y < m.bounds.y + m.bounds.h,
  );
}
