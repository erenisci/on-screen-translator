import { describe, expect, it } from 'vitest';
import { DEFAULT_MIN_FONT_PX, ELLIPSIS, fitText, wrapText, type FitOptions } from './fitText';

/**
 * A deterministic stand-in for canvas measurement: every glyph is half the font
 * size wide. Proportional enough to exercise the ladder, predictable enough to
 * assert on exact numbers.
 */
const measure = (text: string, fontPx: number) => text.length * fontPx * 0.5;

const base: Omit<FitOptions, 'boxWidth' | 'boxHeight' | 'maxGrowHeight'> = {
  maxFontPx: 20,
  minFontPx: DEFAULT_MIN_FONT_PX,
  lineHeight: 1.2,
  measure,
};

function fit(text: string, boxWidth: number, boxHeight: number, maxGrowHeight = boxHeight) {
  return fitText(text, { ...base, boxWidth, boxHeight, maxGrowHeight });
}

describe('wrapText', () => {
  it('returns no lines for empty input', () => {
    expect(wrapText('   ', 12, 100, measure)).toEqual([]);
  });

  it('keeps text on one line when it fits', () => {
    // "merhaba" = 7 chars * 12 * 0.5 = 42px
    expect(wrapText('merhaba', 12, 100, measure)).toEqual(['merhaba']);
  });

  it('wraps greedily at word boundaries', () => {
    // Each word is 4 chars -> 4*10*0.5 = 20px; "aaaa bbbb" = 9 chars -> 45px.
    expect(wrapText('aaaa bbbb cccc', 10, 45, measure)).toEqual(['aaaa bbbb', 'cccc']);
  });

  it('gives an over-wide word its own line rather than breaking it mid-word', () => {
    const lines = wrapText('a supercalifragilistic b', 10, 40, measure);
    expect(lines).toContain('supercalifragilistic');
  });

  it('collapses runs of whitespace', () => {
    expect(wrapText('a   b', 10, 1000, measure)).toEqual(['a b']);
  });
});

describe('fitText — step 1: shrink', () => {
  it('uses the largest size that fits without shrinking when there is room', () => {
    // 4 chars at 20px = 40px wide, one line = 24px tall.
    const result = fit('abcd', 100, 30);
    expect(result.fontPx).toBe(20);
    expect(result.lines).toEqual(['abcd']);
    expect(result.grew).toBe(false);
    expect(result.clipped).toBe(false);
  });

  it('shrinks rather than wrapping when a smaller size fits on one line', () => {
    // 10 chars: needs 100px at 20px, 50px at 10px. Box is 60px wide, 20px tall.
    const result = fit('abcdefghij', 60, 20);
    expect(result.fontPx).toBeLessThan(20);
    expect(result.lines).toHaveLength(1);
    expect(result.height).toBeLessThanOrEqual(20);
    expect(result.clipped).toBe(false);
  });

  it('never shrinks below the legibility floor', () => {
    const result = fit('a very long translated sentence that will not fit at all', 40, 14);
    expect(result.fontPx).toBeGreaterThanOrEqual(DEFAULT_MIN_FONT_PX);
  });

  it('returns an empty result for blank text', () => {
    const result = fit('   ', 100, 30);
    expect(result.lines).toEqual([]);
    expect(result.height).toBe(0);
  });

  it('returns an empty result for a zero-width box', () => {
    expect(fit('anything', 0, 30).lines).toEqual([]);
  });
});

describe('fitText — step 2: grow', () => {
  it('grows downward at the floor when the source box is too short', () => {
    // At 10px: "aaaa bbbb cccc" wraps to 2 lines in a 50px box -> 24px tall.
    const result = fit('aaaa bbbb cccc', 50, 12, 100);
    expect(result.fontPx).toBe(DEFAULT_MIN_FONT_PX);
    expect(result.lines.length).toBeGreaterThan(1);
    expect(result.grew).toBe(true);
    expect(result.clipped).toBe(false);
    expect(result.height).toBeGreaterThan(12);
    expect(result.height).toBeLessThanOrEqual(100);
  });

  it('does not report growth when the wrapped text happens to fit the source box', () => {
    const result = fit('abcd', 100, 30, 200);
    expect(result.grew).toBe(false);
  });
});

describe('fitText — step 3: clip', () => {
  const longText =
    'bu cok uzun bir ceviri metni ve kesinlikle verilen kutuya hicbir sekilde sigmayacak kadar uzundur';

  it('clips and flags when even growth is not enough', () => {
    const result = fit(longText, 50, 12, 36);
    expect(result.clipped).toBe(true);
    expect(result.fontPx).toBe(DEFAULT_MIN_FONT_PX);
    expect(result.height).toBeLessThanOrEqual(36);
  });

  it('marks the truncation with an ellipsis', () => {
    const result = fit(longText, 50, 12, 36);
    expect(result.lines.at(-1)?.endsWith(ELLIPSIS)).toBe(true);
  });

  it('keeps the ellipsis inside the box width', () => {
    const result = fit(longText, 50, 12, 36);
    for (const line of result.lines) {
      expect(measure(line, result.fontPx)).toBeLessThanOrEqual(50);
    }
  });

  it('always keeps at least one line, even in an impossibly short box', () => {
    const result = fit(longText, 50, 2, 1);
    expect(result.lines.length).toBe(1);
    expect(result.lines[0]?.endsWith(ELLIPSIS)).toBe(true);
  });
});

describe('fitText — the ladder is ordered', () => {
  it('prefers shrinking over growing, and growing over clipping', () => {
    const text = 'aaaa bbbb cccc dddd';

    // Plenty of room: full size, no wrap beyond need, no growth, no clipping.
    const roomy = fit(text, 400, 40, 400);
    expect(roomy.grew).toBe(false);
    expect(roomy.clipped).toBe(false);

    // Narrow but free to grow: floor size, grew, still complete.
    const growable = fit(text, 50, 12, 500);
    expect(growable.clipped).toBe(false);
    expect(growable.grew).toBe(true);

    // Narrow and hemmed in: clipped. At the 10px floor the text wraps to two
    // 12px line boxes, so 24px of growth would be exactly enough — 20px is not.
    const cramped = fit(text, 50, 12, 20);
    expect(cramped.clipped).toBe(true);

    // Growing never loses content; clipping is the only lossy step.
    expect(growable.lines.join(' ')).toBe(text);
  });
});
