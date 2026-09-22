import { useCallback, useRef, useState } from 'react';
import { rectFromCorners } from '../lib/coords';
import type { Point, Rect } from '../lib/types';

interface SelectionLayerProps {
  /** Called on pointer-up with the selection in CSS pixels. */
  onSelected: (rect_css: Rect) => void;
  /** Called on the first pointer-down, so the parent can clear a previous result. */
  onSelectionStart: () => void;
  /** A finished selection to keep highlighted while the pipeline runs. */
  frozenSelection_css: Rect | null;
  enabled: boolean;
}

/** Big enough to cover any virtual desktop; the scrim is drawn as an outset shadow. */
const SCRIM_SPREAD_PX = 99999;

/**
 * The drag surface: dim scrim, selection rectangle, live size readout.
 *
 * Works entirely in CSS pixels. Conversion to physical pixels happens in the
 * parent via src/lib/coords.ts — this component never touches a scale factor
 * (invariant 1).
 */
export function SelectionLayer({
  onSelected,
  onSelectionStart,
  frozenSelection_css,
  enabled,
}: SelectionLayerProps) {
  const [anchor_css, setAnchor] = useState<Point | null>(null);
  const [cursor_css, setCursor] = useState<Point | null>(null);
  const surfaceRef = useRef<HTMLDivElement>(null);

  const dragging_css = anchor_css && cursor_css ? rectFromCorners(anchor_css, cursor_css) : null;
  const shown_css = dragging_css ?? frozenSelection_css;

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!enabled || event.button !== 0) return;
      // Capture so a drag that leaves the window still delivers pointerup.
      event.currentTarget.setPointerCapture(event.pointerId);
      const point = { x: event.clientX, y: event.clientY };
      setAnchor(point);
      setCursor(point);
      onSelectionStart();
    },
    [enabled, onSelectionStart],
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!anchor_css) return;
      setCursor({ x: event.clientX, y: event.clientY });
    },
    [anchor_css],
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (!anchor_css) return;
      const rect_css = rectFromCorners(anchor_css, { x: event.clientX, y: event.clientY });
      setAnchor(null);
      setCursor(null);
      onSelected(rect_css);
    },
    [anchor_css, onSelected],
  );

  return (
    <div
      ref={surfaceRef}
      className="absolute inset-0 cursor-crosshair"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      {shown_css ? (
        <div
          className="absolute border border-[--color-selection]"
          style={{
            left: shown_css.x,
            top: shown_css.y,
            width: shown_css.w,
            height: shown_css.h,
            boxShadow: `0 0 0 ${SCRIM_SPREAD_PX}px var(--color-scrim)`,
          }}
        />
      ) : (
        <div className="absolute inset-0 bg-[--color-scrim]" />
      )}

      {dragging_css ? <SizeReadout rect_css={dragging_css} /> : null}
    </div>
  );
}

const READOUT_OFFSET = 8;
const READOUT_WIDTH = 92;
const READOUT_HEIGHT = 24;

/** FR-23: live size in CSS pixels, flipped so it never runs off the screen edge. */
function SizeReadout({ rect_css }: { rect_css: Rect }) {
  const wantsBelow = rect_css.y < READOUT_HEIGHT + READOUT_OFFSET;
  const top = wantsBelow
    ? rect_css.y + rect_css.h + READOUT_OFFSET
    : rect_css.y - READOUT_HEIGHT - READOUT_OFFSET;

  const overflowsRight = rect_css.x + READOUT_WIDTH > window.innerWidth;
  const left = overflowsRight ? window.innerWidth - READOUT_WIDTH - READOUT_OFFSET : rect_css.x;

  return (
    <div
      className="pointer-events-none absolute rounded bg-black/75 px-2 py-1 text-center font-mono text-xs tabular-nums text-white"
      style={{ left, top, width: READOUT_WIDTH }}
    >
      {Math.round(rect_css.w)} × {Math.round(rect_css.h)}
    </div>
  );
}
