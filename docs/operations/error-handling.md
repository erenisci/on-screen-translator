---
title: Error Handling Strategy
discipline: ops
status: active
updated: 2026-09-10
---

# Error Handling Strategy

> **Purpose.** How failures move through the system and what the user sees. In a tool that runs invisibly, a
> swallowed error is indistinguishable from a broken app.
> **Related.** [../architecture/api.md](../architecture/api.md) · [logging.md](logging.md) · [../engineering/coding-standards.md](../engineering/coding-standards.md)

## Principles

**1. Never destroy work that succeeded.**
The most important rule in this system. OCR is the expensive, unrepeatable stage — the frame may be gone by the time
the user retries. So a translation failure must leave the extracted source text on screen and copyable
([FR-44](../product/requirements-functional.md#translation)). Partial success beats a clean failure.

**2. Never trap the user.**
This app puts a full-screen always-on-top window over everything. An error that leaves it up and unresponsive is
worse than a crash — the user can't even see what's wrong. `Esc` works from every state, and any failure either
closes the overlay or shows a dismissible error inside it ([NFR-R1](../product/requirements-nfr.md#reliability)).

**3. Every user-facing error names the cause and the next action.**
"Translation failed" is not acceptable. "DeepL rejected the key — check it in Settings" is
([NFR-U5](../product/requirements-nfr.md#usability)). If we can't say what to do, the message says that honestly
rather than inventing advice.

**4. Errors are typed, not strings.**
A stable `ErrorCode` crosses the IPC boundary; the UI matches on the code and never parses a message
([../architecture/api.md](../architecture/api.md)). New failure modes mean new codes, not new prose.

**5. Nothing is swallowed.**
An unhandled failure is logged with context and surfaced. A silent `catch {}` in a tray app means the user presses
the hotkey and nothing happens, forever, with no way to find out why
([NFR-R6](../product/requirements-nfr.md#reliability)).

**6. Errors never leak secrets or content.**
No API key, no captured text, no image data in any message, `detail` field, or log line
([security.md](security.md), [logging.md](logging.md)).

## Error Types

### In the Rust core

Each module defines a typed error with `thiserror`:

| Type             | Module                    | Represents                                             |
| ---------------- | ------------------------- | -------------------------------------------------------- |
| `CaptureError`   | `capture/`                | Grab failed, no monitors, invalid region                 |
| `OcrError`       | `ocr/`                    | Engine failure, missing language pack, empty result      |
| `TranslateError` | `translate/`              | Auth, rate limit, network, bad response, segment mismatch |
| `SettingsError`  | `settings.rs`             | Unreadable, unparseable, failed migration                |
| `SecretsError`   | `secrets.rs`              | Credential Manager read/write failure                    |
| `HotkeyError`    | `hotkey.rs`               | Registration conflict, invalid accelerator               |

These stay module-local. They are mapped to `AppError` **once**, at the IPC boundary in `ipc.rs`, which is where the
user-facing message and suggested action are attached. Keeping the mapping in one place is what stops user-facing
copy from being scattered through the codebase.

### At the boundary

```rust
pub struct AppError {
    pub code: ErrorCode,        // stable, matched on by the UI
    pub message: String,        // user-facing: what happened
    pub action: Option<String>, // user-facing: what to do about it
    pub action_url: Option<String>,
    pub detail: Option<String>, // technical context, for the log — not shown by default
}
```

The full code table is in [../architecture/api.md](../architecture/api.md). Each code has exactly one
message/action pair, defined in one place.

## Propagation

```
module error (typed)
   │  ?  — with context, never unwrap()
   ▼
pipeline.rs  — decides: fatal, or degrade?
   │
   ▼
ipc.rs  — map to AppError, attach message + action, log with context
   │
   ▼
frontend  — match on code, render message + action
```

**The pipeline is where the interesting decision happens** — whether a failure is fatal to the capture or something
we degrade around:

| Stage failure         | Decision      | Result                                                                 |
| --------------------- | ------------- | ------------------------------------------------------------------------ |
| Capture               | Fatal         | Close the overlay; explain; suggest borderless mode if it looks like a fullscreen app ([TD-07](../project/tech-debt.md)) |
| OCR — no text         | **Not an error** | "No text found" in the overlay; stay open to re-select ([FR-35](../product/requirements-functional.md#ocr)) |
| OCR — missing language| Recoverable   | Actionable message + link to Windows language settings ([FR-34](../product/requirements-functional.md#ocr)) |
| OCR — engine failure  | Fatal for this capture | Message; overlay stays open                                     |
| Translate — any failure | **Degrade**  | Show OCR source text, copyable, with the translation error inline       |
| Translate — segment mismatch | **Degrade** | `alignmentOk: false` → panel-only, no in-place overlay ([TD-06](../project/tech-debt.md)) |
| Settings — corrupt    | Recoverable   | Defaults loaded, bad file preserved as `.corrupt.json`, user warned      |
| Hotkey — conflict     | Recoverable   | Previous binding kept; inline error in Settings ([FR-12](../product/requirements-functional.md#hotkey)) |
| Secrets — unavailable | Recoverable   | Provider unconfigured; Settings explains                                |

**"No text found" is not an error** — it's a normal outcome of pointing at a blank area, and treating it as an error
would make the app feel broken during ordinary use. That distinction is worth being explicit about.

### Panics

A panic in the core is a bug, not error handling. But a panic must not be an invisible death in a tray app:

- A panic hook logs the payload and location with full context.
- The user gets a notification that something went wrong, with the log file path.
- Windows are torn down so nothing is left over the screen.
- `unwrap()`/`expect()` are banned outside tests and `main.rs` startup
  ([coding-standards](../engineering/coding-standards.md)).

### Cancellation is not an error

`Esc` mid-pipeline produces a cancellation, not a failure: no error UI, no log noise beyond `debug`. Every stage
checks for it between steps ([../architecture/system-design.md](../architecture/system-design.md)).

## User-Facing vs Internal

| Audience | Sees                                  | Never sees                                          |
| -------- | ------------------------------------- | ----------------------------------------------------- |
| User     | `message` + `action`, plainly worded  | Stack traces, error codes, crate names, raw HTTP bodies |
| Log      | `code`, `detail`, timings, context    | API keys, captured text, image data                   |

### Writing the message

**Do:** name what failed, in the user's terms, and say what to do.

> "The Turkish OCR language pack isn't installed. Add it in Windows language settings." → [Open settings]
> "DeepL rejected the key. Check it in Settings."
> "No internet connection. The text below was still extracted and can be copied."

**Don't:**

> "Error: reqwest::Error { kind: Request, source: hyper::Error(Connect …) }"
> "Something went wrong."
> "OCR_FAILED (code 3)"

The middle one is the worst of the three: it's polite, useless, and tells the user nothing they can act on.

### Where errors appear

| Context                     | Surface                                                        |
| --------------------------- | ---------------------------------------------------------------- |
| During capture              | Inline in the overlay, near the selection — the overlay stays open |
| Translation failed, OCR ok  | In the panel, above the source text, which remains copyable      |
| Settings validation         | Inline next to the field                                         |
| Startup (hotkey conflict)   | Tray notification + a badge on the tray icon                     |
| Panic                       | Tray notification with the log path                              |

## Retries

**No automatic retries in v1.** Deliberate:

- A retry loop against a rate-limited provider makes the rate limit worse.
- Retrying inside a 1s latency budget means the user waits with no explanation.
- The user retrying is one keypress, and they can see whether the situation changed.

The **only** exception considered acceptable later: a single retry on a connection-reset, with no backoff loop. Even
that needs a measurement showing it helps. Anything more elaborate is an [ADR](../architecture/adr/README.md).
