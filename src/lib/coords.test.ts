import { describe, expect, it } from 'vitest';
import {
  clampRectToBounds,
  cssToPhysicalPoint,
  cssToPhysicalRect,
  isUsableSelection,
  monitorAtPoint,
  physicalToCssPoint,
  physicalToCssRect,
  rectFromCorners,
  type OverlayView,
} from './coords';
import type { Rect } from './types';

// The layout that breaks naive implementations: a 150%-scaled laptop to the
// RIGHT of a 100% external monitor, so the virtual desktop origin is negative.
const mixedDpiNegativeOrigin: OverlayView = {
  virtualBounds: { x: -1920, y: 0, w: 1920 + 2560, h: 1440 },
  scaleFactor: 1.5,
};

const simple: OverlayView = {
  virtualBounds: { x: 0, y: 0, w: 1920, h: 1080 },
  scaleFactor: 1,
};

describe('cssToPhysicalPoint', () => {
  it('is identity at 100% scaling with a zero origin', () => {
    expect(cssToPhysicalPoint({ x: 100, y: 200 }, simple)).toEqual({ x: 100, y: 200 });
  });

  it('applies the scale factor', () => {
    const view: OverlayView = { virtualBounds: { x: 0, y: 0, w: 2880, h: 1620 }, scaleFactor: 1.5 };
    expect(cssToPhysicalPoint({ x: 100, y: 200 }, view)).toEqual({ x: 150, y: 300 });
  });

  it('offsets by a negative virtual-desktop origin', () => {
    // CSS (0,0) is the top-left of the overlay, which sits at physical x=-1920.
    expect(cssToPhysicalPoint({ x: 0, y: 0 }, mixedDpiNegativeOrigin)).toEqual({ x: -1920, y: 0 });
  });

  it('rejects a non-positive scale factor rather than producing silent nonsense', () => {
    expect(() => cssToPhysicalPoint({ x: 1, y: 1 }, { ...simple, scaleFactor: 0 })).toThrow(
      RangeError,
    );
  });
});

describe('physicalToCssPoint', () => {
  it('inverts cssToPhysicalPoint', () => {
    const point_css = { x: 640, y: 360 };
    const point_physical = cssToPhysicalPoint(point_css, mixedDpiNegativeOrigin);
    expect(physicalToCssPoint(point_physical, mixedDpiNegativeOrigin)).toEqual(point_css);
  });
});

describe('cssToPhysicalRect', () => {
  it('converts a rect on a negative-origin mixed-DPI desktop', () => {
    const rect_css = { x: 10, y: 20, w: 100, h: 50 };
    expect(cssToPhysicalRect(rect_css, mixedDpiNegativeOrigin)).toEqual({
      x: -1920 + 15,
      y: 30,
      w: 150,
      h: 75,
    });
  });

  it('derives size from rounded edges so width cannot drift', () => {
    // At 1.25x, x=10.4 and w=9.2 round differently depending on the approach:
    // round(10.4*1.25)=13, round((10.4+9.2)*1.25)=round(24.5)=25 -> w=12,
    // whereas rounding the size on its own gives round(9.2*1.25)=round(11.5)=12.
    // The invariant that matters is that left+w always equals the rounded right edge.
    const view: OverlayView = {
      virtualBounds: { x: 0, y: 0, w: 4000, h: 4000 },
      scaleFactor: 1.25,
    };
    const out = cssToPhysicalRect({ x: 10.4, y: 0, w: 9.2, h: 10 }, view);
    expect(out.x + out.w).toBe(Math.round((10.4 + 9.2) * 1.25));
  });

  it('round-trips through physicalToCssRect at integer-friendly values', () => {
    const rect_css = { x: 100, y: 200, w: 300, h: 400 };
    const rect_physical = cssToPhysicalRect(rect_css, mixedDpiNegativeOrigin);
    expect(physicalToCssRect(rect_physical, mixedDpiNegativeOrigin)).toEqual(rect_css);
  });
});

describe('round-trip across a grid of scale factors and origins', () => {
  const scaleFactors = [1, 1.25, 1.5, 1.75, 2, 2.5];
  const origins = [
    { x: 0, y: 0 },
    { x: -1920, y: 0 },
    { x: -2560, y: -400 },
    { x: 1920, y: 0 },
    { x: 0, y: -1080 },
  ];

  for (const scaleFactor of scaleFactors) {
    for (const origin of origins) {
      it(`survives scale ${scaleFactor} at origin (${origin.x},${origin.y})`, () => {
        const view: OverlayView = {
          virtualBounds: { x: origin.x, y: origin.y, w: 5000, h: 3000 },
          scaleFactor,
        };
        for (const point_css of [
          { x: 0, y: 0 },
          { x: 1, y: 1 },
          { x: 640, y: 360 },
          { x: 1999, y: 1111 },
        ]) {
          const there = cssToPhysicalPoint(point_css, view);
          const back = physicalToCssPoint(there, view);
          // Physical pixels are integers, so a fractional scale can cost at most
          // half a physical pixel on the way back. The epsilon is for float
          // representation of the bound itself, not for slop in the conversion.
          const tolerance = 0.5 / scaleFactor + 1e-9;
          expect(Math.abs(back.x - point_css.x)).toBeLessThanOrEqual(tolerance);
          expect(Math.abs(back.y - point_css.y)).toBeLessThanOrEqual(tolerance);
        }
      });
    }
  }
});

describe('rectFromCorners', () => {
  it('normalizes a drag in every direction to the same rect', () => {
    const expected: Rect = { x: 10, y: 20, w: 90, h: 80 };
    expect(rectFromCorners({ x: 10, y: 20 }, { x: 100, y: 100 })).toEqual(expected);
    expect(rectFromCorners({ x: 100, y: 100 }, { x: 10, y: 20 })).toEqual(expected);
    expect(rectFromCorners({ x: 100, y: 20 }, { x: 10, y: 100 })).toEqual(expected);
    expect(rectFromCorners({ x: 10, y: 100 }, { x: 100, y: 20 })).toEqual(expected);
  });

  it('produces a zero-size rect for a click', () => {
    expect(rectFromCorners({ x: 50, y: 50 }, { x: 50, y: 50 })).toEqual({
      x: 50,
      y: 50,
      w: 0,
      h: 0,
    });
  });
});

describe('clampRectToBounds', () => {
  const bounds: Rect = { x: -1920, y: 0, w: 4480, h: 1440 };

  it('leaves a contained rect alone', () => {
    const rect: Rect = { x: 0, y: 100, w: 200, h: 200 };
    expect(clampRectToBounds(rect, bounds)).toEqual(rect);
  });

  it('trims a rect that overhangs the negative-origin edge', () => {
    expect(clampRectToBounds({ x: -2200, y: 100, w: 400, h: 100 }, bounds)).toEqual({
      x: -1920,
      y: 100,
      w: 120,
      h: 100,
    });
  });

  it('returns zero size when the rect is entirely off the left of the desktop', () => {
    // Overlaps on Y but not on X. Both dimensions must collapse, or a caller
    // could read the surviving height as a real intersection.
    const out = clampRectToBounds({ x: -5000, y: 0, w: 100, h: 100 }, bounds);
    expect(out.w).toBe(0);
    expect(out.h).toBe(0);
  });

  it('returns zero size when the rect is entirely below the desktop', () => {
    const out = clampRectToBounds({ x: 0, y: 9000, w: 100, h: 100 }, bounds);
    expect(out.w).toBe(0);
    expect(out.h).toBe(0);
  });
});

describe('isUsableSelection', () => {
  it('rejects a mis-click', () => {
    expect(isUsableSelection({ x: 0, y: 0, w: 0, h: 0 })).toBe(false);
    expect(isUsableSelection({ x: 0, y: 0, w: 7, h: 40 })).toBe(false);
  });

  it('accepts a real selection at the threshold', () => {
    expect(isUsableSelection({ x: 0, y: 0, w: 8, h: 8 })).toBe(true);
  });
});

describe('monitorAtPoint', () => {
  const monitors = [
    { id: 'external', bounds: { x: -1920, y: 0, w: 1920, h: 1080 } },
    { id: 'laptop', bounds: { x: 0, y: 0, w: 2560, h: 1440 } },
  ];

  it('finds the monitor left of the primary', () => {
    expect(monitorAtPoint({ x: -100, y: 500 }, monitors)?.id).toBe('external');
  });

  it('treats the right edge as exclusive so adjacent monitors do not both match', () => {
    expect(monitorAtPoint({ x: 0, y: 500 }, monitors)?.id).toBe('laptop');
  });

  it('returns undefined in the gap of a non-rectangular desktop', () => {
    expect(monitorAtPoint({ x: 100, y: 3000 }, monitors)).toBeUndefined();
  });
});
