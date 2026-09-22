/**
 * The fitting ladder (F7, FR-51): shrink -> wrap and grow -> clip with hover.
 *
 * Translated text is routinely 20-60% longer than its source, so it rarely fits
 * the bounding box the OCR engine reported. This is the difference between the
 * headline feature working and it producing an unreadable pile.
 *
 * Pure by construction: text measurement is injected, so the ladder can be
 * tested without a DOM and the same logic drives both the real canvas measurer
 * and the tests. See docs/quality/testing-strategy.md.
 */

/** Measures the rendered width of `text` at `fontPx`, in CSS pixels. */
export type MeasureText = (text: string, fontPx: number) => number;

export interface FitOptions {
  /** Width available, CSS px. The source bbox width — never grows. */
  boxWidth: number;
  /** Height of the source bbox, CSS px. Fitting inside this is step 1. */
  boxHeight: number;
  /** How tall the box may become by growing downward into free space (step 2). */
  maxGrowHeight: number;
  /** Starting font size, normally derived from the source line height. */
  maxFontPx: number;
  /** NFR-U8 legibility floor. Never render smaller than this. */
  minFontPx: number;
  /** Line height as a multiple of the font size. */
  lineHeight: number;
  measure: MeasureText;
}

export interface FitResult {
  fontPx: number;
  lines: string[];
  /** Rendered height, CSS px. */
  height: number;
  /** True when the box had to grow past boxHeight. */
  grew: boolean;
  /** True when text was dropped — the caller must expose the full text on hover. */
  clipped: boolean;
}

export const DEFAULT_MIN_FONT_PX = 10;
export const ELLIPSIS = '…';

/**
 * Greedy word wrap. A single word wider than the box gets its own line and is
 * allowed to overflow: breaking mid-word makes an unfamiliar language harder to
 * read, which is the opposite of the point.
 */
export function wrapText(
  text: string,
  fontPx: number,
  maxWidth: number,
  measure: MeasureText,
): string[] {
  const words = text.split(/\s+/).filter((w) => w.length > 0);
  if (words.length === 0) return [];

  const lines: string[] = [];
  let current = '';

  for (const word of words) {
    const candidate = current === '' ? word : `${current} ${word}`;
    if (current !== '' && measure(candidate, fontPx) > maxWidth) {
      lines.push(current);
      current = word;
    } else {
      current = candidate;
    }
  }
  if (current !== '') lines.push(current);

  return lines;
}

/** Shorten a line so that the text plus an ellipsis fits `maxWidth`. */
function clipLine(line: string, fontPx: number, maxWidth: number, measure: MeasureText): string {
  if (measure(line + ELLIPSIS, fontPx) <= maxWidth) return line + ELLIPSIS;

  let end = line.length;
  while (end > 0 && measure(line.slice(0, end) + ELLIPSIS, fontPx) > maxWidth) {
    end -= 1;
  }
  return line.slice(0, end).trimEnd() + ELLIPSIS;
}

export function fitText(text: string, options: FitOptions): FitResult {
  const { boxWidth, boxHeight, maxGrowHeight, maxFontPx, minFontPx, lineHeight, measure } = options;

  const floor = Math.max(1, Math.min(minFontPx, maxFontPx));
  const empty: FitResult = {
    fontPx: floor,
    lines: [],
    height: 0,
    grew: false,
    clipped: false,
  };
  if (text.trim() === '' || boxWidth <= 0) return empty;

  // Step 1 — shrink. Largest integer size whose wrapped text fits the source box.
  for (let fontPx = Math.round(maxFontPx); fontPx >= floor; fontPx -= 1) {
    const lines = wrapText(text, fontPx, boxWidth, measure);
    const height = lines.length * fontPx * lineHeight;
    if (height <= boxHeight) {
      return { fontPx, lines, height, grew: false, clipped: false };
    }
  }

  // Step 2 — grow. At the floor, wrap and take the height we need.
  const lines = wrapText(text, floor, boxWidth, measure);
  const lineBox = floor * lineHeight;
  const neededHeight = lines.length * lineBox;

  if (neededHeight <= maxGrowHeight) {
    return {
      fontPx: floor,
      lines,
      height: neededHeight,
      grew: neededHeight > boxHeight,
      clipped: false,
    };
  }

  // Step 3 — clip, keeping at least one line so something is always readable.
  const maxLines = Math.max(1, Math.floor(maxGrowHeight / lineBox));
  const kept = lines.slice(0, maxLines);
  const lastIndex = kept.length - 1;
  const last = kept[lastIndex];
  if (last !== undefined) {
    kept[lastIndex] = clipLine(last, floor, boxWidth, measure);
  }

  return {
    fontPx: floor,
    lines: kept,
    height: kept.length * lineBox,
    grew: kept.length * lineBox > boxHeight,
    clipped: true,
  };
}

/**
 * The real measurer, backed by a single reused canvas.
 *
 * `font` must match the CSS applied to the rendered text or every measurement
 * is a confident lie.
 */
export function createCanvasMeasurer(fontFamily: string, fontWeight = '500'): MeasureText {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    // No 2D context (an exotic or locked-down webview). Fall back to a rough
    // average-character-width estimate rather than throwing away the feature.
    return (text, fontPx) => text.length * fontPx * 0.52;
  }
  return (text, fontPx) => {
    ctx.font = `${fontWeight} ${fontPx}px ${fontFamily}`;
    return ctx.measureText(text).width;
  };
}
