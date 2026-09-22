import { useMemo } from 'react';
import { physicalToCssRect, type OverlayView } from '../lib/coords';
import { createCanvasMeasurer, DEFAULT_MIN_FONT_PX, fitText } from './fitText';
import type { TranslatedLine } from '../lib/types';

interface ResultLayerProps {
  lines: TranslatedLine[];
  view: OverlayView;
}

const LINE_HEIGHT = 1.2;
const FONT_FAMILY = "'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif";

/** Confidence below this gets a visible hint — a wrong translation must not look authoritative. */
const LOW_CONFIDENCE = 0.6;

/**
 * The headline feature (F7): translated text drawn over the original blocks.
 *
 * Every box is positioned from the OCR bbox, converted once through
 * src/lib/coords.ts, and fitted through the shrink -> wrap -> clip ladder.
 * Boxes that would collide after growing are nudged down so growth never makes
 * two translations unreadable.
 */
export function ResultLayer({ lines, view }: ResultLayerProps) {
  const measure = useMemo(() => createCanvasMeasurer(FONT_FAMILY), []);

  const placed = useMemo(() => {
    // Top-to-bottom, so a box can only ever be pushed past boxes already placed.
    const ordered = [...lines].sort((a, b) => a.bbox.y - b.bbox.y);

    let occupiedUntil = Number.NEGATIVE_INFINITY;

    return ordered.map((line) => {
      const box_css = physicalToCssRect(line.bbox, view);

      // A source line's own height is the most honest starting font size.
      const maxFontPx = Math.max(DEFAULT_MIN_FONT_PX, Math.floor(box_css.h * 0.82));

      const fit = fitText(line.translated, {
        boxWidth: box_css.w,
        boxHeight: box_css.h,
        // Growth is allowed up to three times the source height. Beyond that a
        // single translation would swallow the layout it is annotating.
        maxGrowHeight: box_css.h * 3,
        maxFontPx,
        minFontPx: DEFAULT_MIN_FONT_PX,
        lineHeight: LINE_HEIGHT,
        measure,
      });

      const top = Math.max(box_css.y, occupiedUntil);
      occupiedUntil = top + fit.height;

      return { line, box_css, fit, top };
    });
  }, [lines, view, measure]);

  return (
    <div className="pointer-events-none absolute inset-0">
      {placed.map(({ line, box_css, fit, top }, index) => (
        <div
          key={`${line.bbox.x},${line.bbox.y},${index}`}
          // pointer-events-auto only here, so hovering a clipped translation can
          // reveal the full text without the layer swallowing the next drag.
          className="pointer-events-auto absolute overflow-hidden rounded-sm bg-white/95 px-1 text-black shadow-sm ring-1 ring-black/10"
          style={{
            left: box_css.x,
            top,
            width: box_css.w,
            height: fit.height,
            fontSize: fit.fontPx,
            lineHeight: LINE_HEIGHT,
            fontFamily: FONT_FAMILY,
            fontWeight: 500,
          }}
          title={fit.clipped || line.confidence < LOW_CONFIDENCE ? line.translated : undefined}
        >
          {fit.lines.map((text, lineIndex) => (
            <div key={lineIndex} className="whitespace-pre">
              {text}
            </div>
          ))}
          {line.confidence < LOW_CONFIDENCE ? (
            <span
              className="absolute right-0 top-0 h-1.5 w-1.5 rounded-full bg-amber-500"
              aria-label="low confidence"
            />
          ) : null}
        </div>
      ))}
    </div>
  );
}
