---
title: Logging Strategy
discipline: ops
status: active
updated: 2026-09-10
---

# Logging Strategy

> **Purpose.** What we log, and — more importantly for this app — what we must never log.
> **Related.** [security.md](security.md) · [error-handling.md](error-handling.md) · [configuration.md](configuration.md)

## The constraint that shapes everything

This app sees the user's screen. People will capture banking pages, private messages, medical records, and internal
documents. A log file is a plain-text file that sits on disk indefinitely and gets attached to bug reports.

> **Captured content — recognized text, translated text, and image data — must never be written to a log, at any
> level, including `trace`.**

There is no debug flag that turns this off. Not for development, not for support, not temporarily. A "temporary"
debug dump is exactly how sensitive content ends up in a file someone later emails to a stranger
([G5](security.md#the-guarantees), [NFR-S6](../product/requirements-nfr.md#security)).

**What we log instead: shapes.** Line counts, character counts, box dimensions, durations, confidence scores,
language tags, error codes. Everything needed to diagnose a problem; nothing that reveals what the user was looking
at.

```rust
// Good — shape, not content
info!(lines = 12, chars = 847, source = "en", target = "tr", ocr_ms = 143, "capture complete");

// Never — content
info!(text = %ocr_result.full_text, "recognized text");
```

## Levels

| Level   | Use                                                              | Example                                                    |
| ------- | ---------------------------------------------------------------- | ------------------------------------------------------------ |
| `error` | Something failed that the user saw                                | Provider auth rejected; capture failed; panic               |
| `warn`  | Degraded but working                                              | Settings corrupt → defaults; segment mismatch → panel-only  |
| `info`  | Notable lifecycle events                                          | Startup, hotkey registered, capture complete with timings   |
| `debug` | Stage transitions and decisions                                   | Pre-processing path chosen; cancellation; window created    |
| `trace` | Fine-grained flow, still content-free                             | Per-line box dimensions and confidence — never the text     |

**Default: `info`.** Configurable via `logLevel` in settings, or `RUST_LOG` in development
([env-vars.md](env-vars.md)).

Note that `trace` is bound by the same content rule as every other level. The rule is about *what* is logged, not
*how verbosely*.

## Format

**Development** — human-readable to stderr, with timestamps and module targets.

**Release** — structured (JSON) lines to a rolling file, so a user can attach a log to an issue and the fields are
machine-readable:

```
%APPDATA%\on-screen-translator\logs\otr.log
```

- Daily rotation, **7 files retained**, hard cap on total size. A logging tool must never quietly consume a user's
  disk.
- Written by `logging.rs` — the only module that configures the subscriber.
- `tracing` + `tracing-subscriber` with a rolling file appender.

**Every capture emits one structured summary line** at `info`, which is the single most useful line for diagnosing
almost anything:

```json
{
  "ts": "2026-09-10T14:22:31Z", "level": "INFO", "target": "otr::pipeline",
  "event": "capture_complete",
  "region_px": "820x240", "monitors": 2, "scale_factors": [1.5, 1.0],
  "ocr_lines": 12, "ocr_chars": 847, "mean_confidence": 0.94,
  "source_lang": "en", "target_lang": "tr", "provider": "deepl",
  "alignment_ok": true,
  "capture_ms": 78, "ocr_ms": 143, "translate_ms": 412, "total_ms": 651
}
```

Region size, monitor count, and scale factors are there deliberately: the project's worst bug class is coordinate
handling on mixed-DPI setups ([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)), and those three
fields are what make a user's report diagnosable without asking them to describe their monitor layout.

## What to Log

- **Startup**: version, OS build, monitor topology (count, bounds, scale factors), hotkey registration result
- **Each capture**: the summary line above
- **Stage timings**: always — they're how the [performance budgets](../product/requirements-nfr.md#performance) get
  verified in the field rather than only on the dev machine
- **Errors**: `ErrorCode` + `detail` + the context that led there
- **Degradations**: settings fell back to defaults; alignment mismatch; language pack missing
- **Settings changes**: which field changed — **never** the value if it could be sensitive (the LLM endpoint URL is
  logged as "set"/"unset", not as the URL)
- **Lifecycle**: window created/destroyed, frame buffer allocated/dropped (with size — useful for leak hunting)

## What NOT to Log

Non-negotiable:

| Never                                    | Why                                                       |
| ---------------------------------------- | ----------------------------------------------------------- |
| Recognized text (any part of it)          | It's the user's screen content                             |
| Translated text                           | Same                                                       |
| Image or pixel data, in any encoding      | Same                                                       |
| API keys, or any fragment of one          | [G4](security.md#the-guarantees)                           |
| Raw provider request/response bodies      | They contain both the text and the key                     |
| Clipboard contents                        | Not ours to record                                         |
| Window titles or process names of other apps | Reveals what the user was doing                         |
| Full file paths outside the app's own directory | Contains usernames and directory structure          |

**Provider requests are logged as metadata only**: provider name, endpoint host, status code, duration, character
count. Never the body, never the headers.

## Enforcement

A rule this important shouldn't rely on memory:

- **A wrapper type.** Captured text is held in a `Redacted<String>` whose `Debug`/`Display` prints `[redacted:
  847 chars]`. Logging it accidentally then produces a safe line instead of a leak — the type system carries the
  rule so a tired developer doesn't have to.
- **A `grep` in the release checklist.** Search a fresh log for a known API key and for a distinctive phrase from a
  test capture; both must miss ([../project/release-plan.md](../project/release-plan.md)).
- **A line in the review checklist** ([../engineering/self-review-checklist.md](../engineering/self-review-checklist.md)).

## Access & retention

- Logs stay on the user's machine. Nothing is uploaded, ever
  ([G3](security.md#the-guarantees)) — there is no crash reporter and no telemetry to send them anywhere.
- The user can find them from the About dialog ("Open log folder") and delete them freely.
- 7 days of rotation is enough to diagnose a reported bug and short enough to limit exposure.
- If a user attaches a log to a GitHub issue, it should contain nothing they'd regret sharing. **That is the design
  target for this entire document.**
