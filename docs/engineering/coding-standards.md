---
title: Coding Standards
discipline: code
status: active
updated: 2026-09-10
---

# Coding Standards

> **Purpose.** How code in this repo should read and behave. Conventions that are settled here don't need an ADR
> and shouldn't be re-argued in review.
> **Related.** [project-structure.md](project-structure.md) · [naming-conventions.md](naming-conventions.md) · [self-review-checklist.md](self-review-checklist.md)

## Language

**Rust (core)** — 2021 edition, stable toolchain. `rustfmt` defaults; `clippy` with `-D warnings` in CI.

**TypeScript (frontend)** — `strict: true`, no exceptions. `any` is banned; use `unknown` and narrow. React function
components with hooks; no class components.

**Both** — the compiler is the first reviewer. If a rule can be enforced by a type or a lint, enforce it there
instead of writing it down here.

## Formatting & Lint

| Tool                        | Scope     | Enforced          |
| --------------------------- | --------- | ----------------- |
| `rustfmt`                   | Rust      | CI + pre-commit   |
| `clippy -D warnings`        | Rust      | CI                |
| Prettier                    | TS/TSX/CSS| CI + pre-commit   |
| ESLint (typescript-eslint)  | TS/TSX    | CI                |
| `tsc --noEmit`              | TS        | CI                |

Formatting is never a review topic. If the formatter did it, it's correct.

## Patterns

### Errors

**Rust.** `thiserror` for typed errors in library modules, mapped to `AppError` at the IPC boundary
([../architecture/api.md](../architecture/api.md)). Never `unwrap()` or `expect()` outside tests and `main.rs`
startup — use `?` and let the error carry context. A `panic!` in a running app is a bug, not error handling.

**TypeScript.** Errors from `ipc.ts` arrive as typed `AppError`. Components render `message` and `action`; they never
parse or reconstruct a message. Adding a new failure means adding an `ErrorCode`, not a new string.

**The rule that matters most here:** every error a user can see must name the cause *and* the next action
([NFR-U5](../product/requirements-nfr.md#usability)). "Translation failed" is not an acceptable message.

### Async

Tokio in the core, for provider calls and OCR interop. Two rules:

- **Nothing is scheduled while idle.** No background task, no interval, no watcher. This is a hard invariant, not a
  preference ([NFR-P5](../product/requirements-nfr.md#performance)).
- **Every pipeline stage is cancellable**, and cancellation is checked between stages. `Esc` must always work
  ([NFR-R1](../product/requirements-nfr.md#reliability)).

### Traits and abstraction

`OcrEngine` and `TranslationProvider` exist because we know a second implementation is coming
([ADR-0002](../architecture/adr/0002-ocr-engine.md), [ADR-0003](../architecture/adr/0003-translation-provider-abstraction.md)).
That is the bar: **abstract when a second implementation is real, not hypothetical.** A trait with one
implementation and no planned second is an indirection tax.

### Purity where it counts

Coordinate math, line assembly, batching, and text fitting are pure functions in dedicated modules. They take data
and return data — no I/O, no globals, no clock. This is deliberate: they carry the project's highest bug risk and
its lowest testing cost ([project-structure.md](project-structure.md)).

### Coordinates

The single most important convention in this codebase:

> Core coordinates are **physical pixels in virtual-desktop space**. Conversion to and from CSS pixels happens in
> `src/lib/coords.ts` and **nowhere else** ([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)).

If you find yourself multiplying by a scale factor outside that file, stop — that's the bug.

### State (frontend)

Local `useState` and props. No Redux, no Zustand, no context gymnastics. Three small windows with shallow trees
don't need a state library, and adding one is complexity without a requirement
([principles](../../CLAUDE.md)).

### Dependencies

Every new dependency needs a reason you'd say out loud in review. Prefer the standard library, then a small focused
crate/package, then a large framework — never the reverse. Weigh it against the ≤ 20 MB install budget
([NFR-P7](../product/requirements-nfr.md#performance)) and the fact that one person maintains this.

## Anti-patterns

Things that are wrong in *this* project specifically:

| Don't                                                        | Because                                                                    |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Call `invoke` outside `src/lib/ipc.ts`                        | The IPC surface must stay greppable and mockable                            |
| Convert pixel coordinates outside `coords.ts`                 | This is the mixed-DPI bug factory ([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)) |
| Put logic in a Tauri command handler                          | It becomes untestable without Tauri                                         |
| Add a timer, interval, or watcher that runs while idle        | Breaks the idle budget and the whole architecture's premise                 |
| Make a network call from the frontend                         | Breaks the single-egress-point privacy guarantee                            |
| Read or write an API key outside `secrets.rs`                 | Keys must never reach the settings file, the logs, or the webview           |
| Log captured text or image data                               | [NFR-S6](../product/requirements-nfr.md#security) — see [../operations/logging.md](../operations/logging.md) |
| Write captured pixels to disk                                 | Same                                                                        |
| Add a per-line translation request                            | 10–30× the latency budget ([FR-42](../product/requirements-functional.md#translation)) |
| `unwrap()` on anything that can fail at runtime               | A panic in a tray app is an invisible death                                 |
| Cache monitor topology across captures                        | Monitors get unplugged ([NFR-R4](../product/requirements-nfr.md#reliability)) |
| Hand-edit `types.gen.ts`                                      | It's generated; your edit disappears and the drift is silent                |
| Add telemetry, analytics, or an update ping                   | [NFR-S4](../product/requirements-nfr.md#security) — a promise, not a preference |
| Introduce a database, ORM, or state library                   | No requirement calls for one                                                |

## Comments

Comment **why**, never **what**. The code says what it does; it cannot say what you rejected.

Worth writing:

```rust
// Upscale before grayscale: thresholding a small image first destroys the
// stroke detail the OCR engine needs on sub-20px UI text.
```

Not worth writing:

```rust
// Convert the image to grayscale
let gray = to_grayscale(&img);
```

Specifically **do** comment: a non-obvious ordering, a workaround for a platform quirk (with a link), a performance
trade-off, and anything where the obvious approach is wrong. If a comment is explaining a *decision*, it probably
wants an [ADR](../architecture/adr/README.md) instead, with the comment linking to it.

Doc comments (`///`, TSDoc) on public traits, commands, and any function crossing a module boundary. Skip them on
obvious private helpers — noise is worse than silence.

## Formatting of the docs themselves

Documentation is code here: it's the source of truth, it's read before writing, and it goes stale the same way.
Update the doc in the same change as the behaviour, not afterwards ([../project/definition-of-done.md](../project/definition-of-done.md)).
