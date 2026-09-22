/**
 * The IPC contract, mirrored for the frontend.
 *
 * NOTE: docs/architecture/api.md specifies that these types are GENERATED from
 * the Rust definitions (`types.gen.ts`) so the two sides cannot drift. That
 * generation needs the Rust core to exist first (M2). Until then this file is
 * the hand-authored mirror and must be kept in step with docs/architecture/api.md
 * by hand. Tracked in SCRATCH.md — delete this file when generation lands.
 */

/** BCP-47 language tag, e.g. "en", "tr", "pt-BR". */
export type LangTag = string;

/**
 * A rectangle. The unit is NOT implied by this type — it is implied by the
 * variable name. See docs/engineering/naming-conventions.md: every coordinate
 * is named `_physical` (physical pixels, virtual-desktop space, origin may be
 * negative) or `_css` (CSS pixels inside a webview).
 */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface MonitorInfo {
  id: string;
  /** Physical pixels, virtual-desktop space. */
  bounds: Rect;
  /** 1.0 = 100%, 1.5 = 150%. */
  scaleFactor: number;
  isPrimary: boolean;
}

export interface CaptureFrameInfo {
  /** The full virtual desktop, physical pixels. x/y may be negative. */
  virtualBounds: Rect;
  monitors: MonitorInfo[];
  /** "otr://frame/<session-id>" — served by the core, never a base64 payload. */
  frameUrl: string;
  sessionId: string;
  /** The overlay window's own scale factor, used for every CSS<->physical conversion. */
  scaleFactor: number;
}

export interface OcrLine {
  text: string;
  /** Frame coordinates: physical pixels, virtual-desktop space. */
  bbox: Rect;
  /** 0.0 - 1.0 */
  confidence: number;
  /** Groups lines into paragraphs. */
  blockId: number;
}

export interface TranslatedLine {
  bbox: Rect;
  source: string;
  translated: string;
  confidence: number;
}

export interface TranslateResult {
  sourceLang: LangTag;
  targetLang: LangTag;
  lines: TranslatedLine[];
  fullSource: string;
  fullTranslation: string;
  /**
   * false => translated segments could not be matched back onto OCR lines.
   * The overlay MUST NOT render in place; fall back to panel-only.
   * See docs/project/tech-debt.md TD-06.
   */
  alignmentOk: boolean;
  timings: { ocrMs: number; translateMs: number; totalMs: number };
}

export type ResultDisplayMode = 'overlay' | 'panel' | 'both';
export type ThemePreference = 'system' | 'light' | 'dark';
export type ProviderId = 'libretranslate' | 'deepl' | 'google' | 'llm';

export interface Settings {
  schemaVersion: number;
  targetLanguage: LangTag;
  /** null = auto-detect. */
  sourceLanguage: LangTag | null;
  hotkey: string;
  resultDisplayMode: ResultDisplayMode;
  provider: ProviderId;
  llmEndpoint: string | null;
  llmModel: string | null;
  startWithWindows: boolean;
  theme: ThemePreference;
  logLevel: string;
}

export interface ProviderHealth {
  ok: boolean;
  message: string;
}

export interface HotkeyResult {
  ok: boolean;
  /** The binding actually in effect — the previous one if registration failed. */
  active: string;
  message: string | null;
}

export interface AppInfo {
  version: string;
  hotkey: string;
  degradedReasons: string[];
}

/** Stable, machine-readable. The UI matches on these, never on `message`. */
export type ErrorCode =
  | 'CAPTURE_FAILED'
  | 'NO_TEXT_FOUND'
  | 'OCR_LANG_MISSING'
  | 'OCR_FAILED'
  | 'PROVIDER_UNCONFIGURED'
  | 'PROVIDER_AUTH'
  | 'PROVIDER_RATE_LIMIT'
  | 'PROVIDER_UNAVAILABLE'
  | 'HOTKEY_CONFLICT'
  | 'SETTINGS_CORRUPT'
  | 'INTERNAL';

export interface AppError {
  code: ErrorCode;
  /** User-facing: what happened. */
  message: string;
  /** User-facing: what to do about it. */
  action?: string;
  actionUrl?: string;
  /** Technical context for the log. Never shown by default, never contains secrets. */
  detail?: string;
}

export interface ProgressEvent {
  stage: 'ocr' | 'translate';
  elapsedMs: number;
}
