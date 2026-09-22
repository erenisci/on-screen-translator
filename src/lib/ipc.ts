/**
 * The ONE door between the frontend and the Rust core (invariant 8).
 *
 * No component calls `invoke` directly — eslint enforces this via
 * no-restricted-imports. One module means the whole IPC surface is greppable,
 * mockable in tests, and auditable against docs/architecture/api.md.
 *
 * The frontend has no network, no filesystem, and no OS access. Everything it
 * needs beyond drawing happens here.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AppError,
  AppInfo,
  CaptureFrameInfo,
  ErrorCode,
  LangTag,
  ProgressEvent,
  ProviderHealth,
  ProviderId,
  Rect,
  Settings,
  HotkeyResult,
  TranslateResult,
} from './types';

const KNOWN_CODES: readonly ErrorCode[] = [
  'CAPTURE_FAILED',
  'NO_TEXT_FOUND',
  'OCR_LANG_MISSING',
  'OCR_FAILED',
  'PROVIDER_UNCONFIGURED',
  'PROVIDER_AUTH',
  'PROVIDER_RATE_LIMIT',
  'PROVIDER_UNAVAILABLE',
  'HOTKEY_CONFLICT',
  'SETTINGS_CORRUPT',
  'INTERNAL',
];

function isAppError(value: unknown): value is AppError {
  if (typeof value !== 'object' || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === 'string' &&
    KNOWN_CODES.includes(candidate.code as ErrorCode) &&
    typeof candidate.message === 'string'
  );
}

/**
 * Normalize anything a rejected command can throw into an AppError.
 *
 * The core always rejects with an AppError, but a transport-level failure (the
 * window closing mid-call) can surface as a bare string. Components match on
 * `code` and must never have to guess, so everything becomes typed here.
 */
export function toAppError(thrown: unknown): AppError {
  if (isAppError(thrown)) return thrown;
  return {
    code: 'INTERNAL',
    message: 'Something went wrong inside the app.',
    action: 'Try again. If it keeps happening, check the log from the tray menu.',
    detail: typeof thrown === 'string' ? thrown : String(thrown),
  };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (thrown) {
    throw toAppError(thrown);
  }
}

/* -------------------------------------------------------------- capture ---- */

export const getCaptureFrame = () => call<CaptureFrameInfo>('get_capture_frame');

/**
 * The main pipeline call. `rect_physical` is physical pixels in virtual-desktop
 * space — convert with src/lib/coords.ts and nowhere else (invariant 1).
 */
export const translateRegion = (rect_physical: Rect) =>
  call<TranslateResult>('translate_region', { rect: rect_physical });

/** Re-translate the cached OCR text. No re-capture, no re-OCR (FR-54). */
export const retranslate = (target: LangTag) => call<TranslateResult>('retranslate', { target });

export const cancelCapture = () => call<void>('cancel_capture');
export const closeOverlay = () => call<void>('close_overlay');
export const closePanel = () => call<void>('close_panel');

/**
 * The current result, for the panel.
 *
 * The panel also receives `otr://result`, but a window that finishes mounting
 * after the core emitted it would otherwise come up blank — so it pulls once on
 * mount and listens for subsequent captures.
 */
export const getPanelResult = () => call<TranslateResult | null>('get_panel_result');

/* ------------------------------------------------------------- settings ---- */

export const getSettings = () => call<Settings>('get_settings');
export const saveSettings = (settings: Settings) => call<Settings>('save_settings', { settings });

/**
 * Write a provider key. It goes straight to Windows Credential Manager and is
 * never echoed back — use hasProviderKey() to show "configured" in the UI
 * without the key ever living in the webview (FR-62, invariant 7).
 */
export const setProviderKey = (provider: ProviderId, key: string) =>
  call<void>('set_provider_key', { provider, key });

export const hasProviderKey = (provider: ProviderId) =>
  call<boolean>('has_provider_key', { provider });

export const clearProviderKey = (provider: ProviderId) =>
  call<void>('clear_provider_key', { provider });

export const testProvider = (provider: ProviderId) =>
  call<ProviderHealth>('test_provider', { provider });

export const setHotkey = (accelerator: string) => call<HotkeyResult>('set_hotkey', { accelerator });

export const setAutostart = (enabled: boolean) => call<boolean>('set_autostart', { enabled });

export const listOcrLanguages = () => call<LangTag[]>('list_ocr_languages');

/* -------------------------------------------------------------- utility ---- */

export const copyToClipboard = (text: string) => call<void>('copy_to_clipboard', { text });
export const openSettings = () => call<void>('open_settings');
export const openExternal = (url: string) => call<void>('open_external', { url });
export const getAppInfo = () => call<AppInfo>('get_app_info');

/* --------------------------------------------------------------- events ---- */

export const onProgress = (handler: (event: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ProgressEvent>('otr://progress', (e) => handler(e.payload));

export const onError = (handler: (error: AppError) => void): Promise<UnlistenFn> =>
  listen<AppError>('otr://error', (e) => handler(e.payload));

export const onCancelled = (handler: (sessionId: string) => void): Promise<UnlistenFn> =>
  listen<{ sessionId: string }>('otr://cancelled', (e) => handler(e.payload.sessionId));

export const onSettingsChanged = (handler: (settings: Settings) => void): Promise<UnlistenFn> =>
  listen<Settings>('otr://settings-changed', (e) => handler(e.payload));

export const onCaptureRequested = (handler: () => void): Promise<UnlistenFn> =>
  listen('otr://capture-requested', () => handler());

export const onResult = (handler: (result: TranslateResult) => void): Promise<UnlistenFn> =>
  listen<TranslateResult>('otr://result', (e) => handler(e.payload));
