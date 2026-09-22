---
title: IPC Contract
discipline: code
status: active
updated: 2026-09-10
---

# IPC Contract

> **Purpose.** The boundary between the Rust core and the webviews. This is the project's only cross-language
> interface, so it is treated as a real API contract — drift here is the main cost of the two-language architecture
> ([ADR-0001](adr/0001-initial-architecture.md)).
> **Related.** [overview.md](overview.md) · [system-design.md](system-design.md) · [../operations/error-handling.md](../operations/error-handling.md)

## Overview

There is no HTTP API and no server. "API" here means **Tauri commands** (frontend → core, request/response) and
**Tauri events** (core → frontend, push).

**Rules that keep this boundary honest:**

1. **The frontend never touches the network, the filesystem, or a secret.** Every such action is a command
   ([NFR-S5](../product/requirements-nfr.md#security)).
2. **All geometry crossing this boundary is physical pixels in virtual-desktop space.** Conversion to and from CSS
   pixels happens in the overlay frontend and nowhere else ([ADR-0004](adr/0004-capture-and-coordinate-model.md)).
3. **Types are defined once in Rust** and generated for TypeScript (`ts-rs` or equivalent), never hand-mirrored.
   Hand-syncing types is how this boundary rots.
4. **Every command returns `Result`**, and errors are the typed shape below — never a bare string.
5. **No command accepts or returns image bytes.** The frame reaches the overlay through the `otr://` protocol
   handler instead, because base64 through JSON would cost more than the whole overlay budget.

## Auth

None. There is no server, no account, and no session. The trust boundary is the process itself: the core is trusted,
the webview is treated as untrusted presentation code and is given only what it needs to render.

## Shared types

```ts
type LangTag = string;              // BCP-47, e.g. "en", "tr", "pt-BR"
type Px = number;                   // physical pixels, virtual-desktop space

interface Rect { x: Px; y: Px; w: Px; h: Px; }   // x/y may be negative

interface MonitorInfo {
  id: string;
  bounds: Rect;                     // physical, virtual-desktop space
  scaleFactor: number;              // 1.0 = 100%, 1.5 = 150%
  isPrimary: boolean;
}

interface OcrLine {
  text: string;
  bbox: Rect;                       // frame coordinates
  confidence: number;               // 0.0–1.0
  blockId: number;                  // groups lines into paragraphs
}

interface TranslatedLine {
  bbox: Rect;
  source: string;
  translated: string;
  confidence: number;               // carried from OCR
}
```

## Commands

### Capture session

| Command             | Input                | Returns              | Notes                                                                 |
| ------------------- | -------------------- | -------------------- | ----------------------------------------------------------------------- |
| `get_capture_frame` | —                    | `CaptureFrameInfo`   | Metadata only. The pixels are fetched from `otr://frame`.                |
| `translate_region`  | `{ rect: Rect }`     | `TranslateResult`    | The main pipeline call. Cancellable. Crops the existing frame.           |
| `retranslate`       | `{ target: LangTag }`| `TranslateResult`    | Reuses cached OCR text — no re-capture, no re-OCR ([FR-54](../product/requirements-functional.md#results)). |
| `cancel_capture`    | —                    | `void`               | Tears down the session and drops the frame buffer.                       |
| `close_overlay`     | —                    | `void`               | Destroys the overlay window; the panel survives.                         |
| `close_panel`       | —                    | `void`               | Destroys the panel window; the overlay survives.                         |
| `get_panel_result`  | —                    | `TranslateResult \| null` | The current result, for the panel (see below).                      |

**Why the panel both pulls and listens.** The core emits `otr://result` when a capture completes, but the panel
window may still be mounting at that moment and would come up blank. So it calls `get_panel_result` once on mount
and subscribes to `otr://result` for subsequent captures in the same session. The core holds the current result for
the lifetime of the session — in memory only, dropped when the overlay closes
([NFR-S6](../product/requirements-nfr.md#security)).

```ts
interface CaptureFrameInfo {
  virtualBounds: Rect;              // the full virtual desktop
  monitors: MonitorInfo[];
  frameUrl: string;                 // "otr://frame/<session-id>"
  sessionId: string;
}

interface TranslateResult {
  sourceLang: LangTag;              // detected, or the configured override
  targetLang: LangTag;
  lines: TranslatedLine[];
  fullSource: string;               // paragraph-joined, for the panel and copying
  fullTranslation: string;
  alignmentOk: boolean;             // false → render panel-only (TD-06)
  timings: { ocrMs: number; translateMs: number; totalMs: number };
}
```

`alignmentOk: false` is the contract for the segment-mismatch fallback: the core detected that translated segments
don't correspond to OCR lines, so the overlay must **not** attempt in-place rendering
([TD-06](../project/tech-debt.md)).

### Settings

| Command              | Input                       | Returns              | Notes                                                        |
| -------------------- | --------------------------- | -------------------- | -------------------------------------------------------------- |
| `get_settings`       | —                           | `Settings`           | Never includes API keys.                                       |
| `save_settings`      | `{ settings: Settings }`    | `Settings`           | Returns the normalized result; applies live, no restart.       |
| `set_provider_key`   | `{ provider, key }`         | `void`               | Writes to Credential Manager. Key is never echoed back.        |
| `has_provider_key`   | `{ provider }`              | `boolean`            | How the UI shows "key set" without ever reading the key.       |
| `clear_provider_key` | `{ provider }`              | `void`               | Removes the Credential Manager entry.                          |
| `test_provider`      | `{ provider }`              | `ProviderHealth`     | Names the specific failure ([FR-63](../product/requirements-functional.md#settings--persistence)). |
| `set_hotkey`         | `{ accelerator: string }`   | `HotkeyResult`       | Re-registers live; reports conflicts without losing the old binding. |
| `set_autostart`      | `{ enabled: boolean }`      | `boolean`            | Returns the state read back from the registry, not the request. |
| `list_ocr_languages` | —                           | `LangTag[]`          | What this machine can actually recognize right now.            |

**`set_provider_key` and `has_provider_key` exist as a pair on purpose:** the UI must be able to show that a key is
configured without ever having the key in the webview's memory
([FR-62](../product/requirements-functional.md#settings--persistence)).

### Utility

| Command             | Input                 | Returns   | Notes                                                    |
| ------------------- | --------------------- | --------- | ---------------------------------------------------------- |
| `copy_to_clipboard` | `{ text: string }`    | `void`    | Clipboard access stays in the core.                        |
| `open_settings`     | —                     | `void`    | Creates or focuses the settings window.                    |
| `open_external`     | `{ url: string }`     | `void`    | Allowlisted URLs only (Windows language settings, docs).   |
| `get_app_info`      | —                     | `AppInfo` | Version, hotkey, degraded-state reasons — for About.       |

## Events

Core → frontend, one-way.

| Event              | Payload                                          | Consumer  |
| ------------------ | ------------------------------------------------ | --------- |
| `otr://progress`   | `{ stage: "ocr" \| "translate", elapsedMs }`      | overlay   |
| `otr://result`     | `TranslateResult`                                | panel     |
| `otr://error`      | `AppError`                                       | all       |
| `otr://cancelled`  | `{ sessionId }`                                  | overlay   |
| `otr://settings-changed` | `Settings`                                 | all open windows |
| `otr://capture-requested` | —                                        | overlay   |

`otr://progress` exists so the overlay can show an indicator only after ~300 ms — a spinner that appears on every
sub-second capture is worse than none.

## Errors

Every command returns `Result<T, AppError>`. The shape is fixed so the UI never has to parse a message:

```ts
interface AppError {
  code: ErrorCode;         // stable, machine-readable
  message: string;         // user-facing, already localized-ready
  action?: string;         // what the user can do about it
  actionUrl?: string;      // e.g. the Windows language settings page
  detail?: string;         // technical context for the log — never shown by default
}
```

| Code                    | Meaning                             | Typical action                                  |
| ----------------------- | ----------------------------------- | ------------------------------------------------- |
| `CAPTURE_FAILED`        | Screen grab failed                  | Suggest borderless windowed mode ([TD-07](../project/tech-debt.md)) |
| `NO_TEXT_FOUND`         | OCR returned nothing                | Stay in the overlay, re-select ([FR-35](../product/requirements-functional.md#ocr)) |
| `OCR_LANG_MISSING`      | Language pack not installed         | Link to Windows language settings ([FR-34](../product/requirements-functional.md#ocr)) |
| `OCR_FAILED`            | Engine error                        | Retry; log detail                                |
| `PROVIDER_UNCONFIGURED` | No provider or key set              | Open Settings                                    |
| `PROVIDER_AUTH`         | Key rejected (401/403)              | Check the key in Settings                        |
| `PROVIDER_RATE_LIMIT`   | 429                                 | Suggest configuring a personal key               |
| `PROVIDER_UNAVAILABLE`  | Network/DNS/5xx                     | Source text stays copyable ([FR-44](../product/requirements-functional.md#translation)) |
| `HOTKEY_CONFLICT`       | Accelerator already claimed         | Choose another; old binding kept ([FR-12](../product/requirements-functional.md#hotkey)) |
| `SETTINGS_CORRUPT`      | Settings file unreadable            | Defaults loaded; warn ([FR-61](../product/requirements-functional.md#settings--persistence)) |
| `INTERNAL`              | Unexpected                          | Log with context; never swallow ([NFR-R6](../product/requirements-nfr.md#reliability)) |

`detail` must never contain an API key, captured text, or a file path from the user's disk beyond the app's own
directory — see [../operations/logging.md](../operations/logging.md).

## The `otr://` protocol handler

A custom protocol serves the captured frame to the overlay:

- `otr://frame/<session-id>` → the frozen frame as PNG.
- Scoped to the active session. Once the session ends the handler returns 404 and the buffer is dropped.
- Read-only, serving exactly one in-memory resource. It is not a general file server, and must never become one.

This exists purely for performance: it keeps a multi-megabyte image out of the JSON IPC path
([ADR-0004](adr/0004-capture-and-coordinate-model.md)).

## Versioning

The core and the frontend ship in the same binary, so this contract has no external consumers and needs no version
negotiation. It still needs discipline:

- Changing a command's shape means changing the generated types — the build fails if they drift, which is the point.
- **The settings JSON is the one thing that outlives a release.** Its schema is versioned and migrated on load;
  a user's settings must never be silently discarded by an upgrade
  ([../project/release-plan.md](../project/release-plan.md)).
