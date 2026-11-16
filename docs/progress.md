---
title: Progress
discipline: project
status: active
updated: 2026-11-16
---

# Progress

> **Purpose.** The current state of the project at a glance. Updated by `/acta:track` after each chunk of work.
> **Related.** [project/roadmap.md](project/roadmap.md) · [../CHANGELOG.md](../CHANGELOG.md) · [project/tech-debt.md](project/tech-debt.md)

## Current Status

**Stage:** the pipeline reads. Hotkey → frozen frame → drag → crop → pre-process → recognized text, all working.
**Version:** 0.1.0 (unreleased)
**Active milestone:** M3 — translation. M2 is done; M1 is code complete but **not** done per its DoD.

173 tests pass (66 TypeScript, 107 Rust — six of them against the real OCR engine); `tsc`, ESLint, Prettier,
`cargo fmt` and `cargo clippy -D warnings` are all clean.

**M1 is still not "done".** Its definition of done requires a box drawn on a 150%-scaled monitor to produce a crop
containing exactly what was inside it, verified on real mixed-DPI hardware. That has not happened. Every
coordinate claim rests on unit tests, fixture tests at a synthetic negative origin, and a single-monitor run —
which is not the same thing as a second screen.

## In Progress

**M3 — translation.** Not started. The OCR result is already cached on the session for it
([FR-54](product/requirements-functional.md#results)), and `translate_region` runs recognition for real before
returning a typed "not implemented" naming the milestone.

**M1 — the verification that closes it.**

| Part                                                        | State                                                      |
| ----------------------------------------------------------- | ------------------------------------------------------------ |
| Coordinate model — `coords.ts` + `geometry.rs`               | **Done** — tested on both sides of the IPC boundary          |
| Tray icon, menu, single instance, clean quit                 | **Done** — single-instance verified by running two copies    |
| Global hotkey registration + live rebinding                  | **Done** — registration verified; rebinding untested by hand |
| Virtual-desktop capture (GDI `BitBlt`) + crop                | **Done** — one real capture, buffer geometry exactly right   |
| `otr://frame` protocol handler                               | **Written, never served a byte**                             |
| Overlay window (scrim, drag selection, size readout, `Esc`)  | **Written, never seen rendering**                            |
| Settings persistence + OS language detection                 | **Done** — `tr` auto-detected from Windows with no config    |
| In-place result layer, panel, settings UI                    | Written ahead of M4/M5; waiting on translation               |
| **Mixed-DPI multi-monitor verification**                     | **Not started — this is what gates M1**                      |

## Measured

Against [the budgets](product/requirements-nfr.md#performance). All debug builds on a single 1920×1080 display.

| Metric                            | Measured              | Budget          | Verdict                           |
| --------------------------------- | --------------------- | --------------- | ----------------------------------- |
| Idle working set                  | 21.3 MB               | ≤ 30 MB         | **Pass** (and it's a debug build)   |
| Idle private memory               | 6.4 MB                | —               | —                                   |
| Idle CPU over 20 s                | ~0.4%                 | ~0%             | **Pass**                            |
| Virtual-desktop grab              | **183 ms**            | part of 250 ms  | **At risk** — see below             |
| Frame buffer size                 | 8,294,400 B           | 1920×1080×4     | Exactly right                       |
| Pre-process (crops 300–520 px wide) | 2–22 ms             | part of 200 ms  | **Pass**                            |
| OCR (same crops)                  | 6–10 ms               | part of 200 ms  | **Pass**, with room to spare        |

**The grab is still the concern.** NFR-P1 allows 250 ms for the entire hotkey→overlay-visible path, and 183 ms of
that went to the GDI capture alone on a single 1080p screen. It scales with pixel count, so a dual-4K desktop would
miss the budget outright. A release build will help; whether that is enough needs measuring on a large desktop
before M6. The OCR half's headroom is where some of that overrun may have to come from.

**What the OCR numbers do not cover:** a region near the 800×400 ceiling NFR-P3 budgets for, and a release build.

## Done (recent)

| Date       | What                                                                                            |
| ---------- | ------------------------------------------------------------------------------------------------- |
| 2026-11-16 | **M2 — OCR.** Pre-processing ladder, `OcrEngine` trait, `Windows.Media.Ocr`, line assembly       |
| 2026-11-16 | Six fixture tests against the real engine, with boxes asserted against measured ink ([ADR-0005](architecture/adr/0005-ocr-test-fixtures-are-rendered.md)) |
| 2026-11-16 | `recognize_region` added to the IPC contract; `confidence` is nullable across it (TD-09)        |
| 2026-11-16 | FR-31/FR-32/FR-33 corrected to what the engine actually does (TD-09, TD-10, TD-11)              |
| 2026-09-22 | Rust core: tray, single instance, hotkey, capture, crop, `otr://` protocol, settings, logging   |
| 2026-09-22 | `geometry.rs` — the Rust twin of `coords.ts`, incl. crops from a negative-origin desktop         |
| 2026-09-22 | Fixed: Quit did nothing. `prevent_exit()` was unconditional and blocked the user's own Quit      |
| 2026-09-22 | Redesigned the app icon — the first one spelled a Turkish word and baked in one language pair    |
| 2026-09-22 | Frontend scaffold: Vite 7 + React 19 + TS strict + Tailwind v4, three window entries             |
| 2026-09-22 | `coords.ts` (49 tests) and `fitText.ts` (17 tests); ESLint enforces the IPC boundary             |

## Blocked

Nothing is blocked. Three decisions are open; none of them blocks M3 starting:

- **[PRD Q1](product/prd.md#open-questions)** — which keyless provider ships as the zero-config default. Needed by
  M3, and it is the one that can derail it.
- **[PRD Q2](product/prd.md#open-questions)** — code signing vs. SmartScreen friction. Needed by M7/M8.
- **[TD-11](project/tech-debt.md)** — whether the pre-processing ladder earns its 13–22 ms. Needs a harder fixture
  before M6.

## Next Up

1. **Close M1.** Run the app, press the hotkey, confirm the overlay renders the frozen frame at 1:1 and the
   scrim/drag/`Esc` behave. Then repeat **on a multi-monitor mixed-DPI setup** and check the crop lands exactly
   where the box was drawn. Nothing else counts ([DoD](project/definition-of-done.md)). Recognition now gives this
   a sharper check than eyeballing a crop: select text on the 150% monitor and see whether the right words come
   back positioned over the right lines.
2. **M3 — translation.** The `TranslationProvider` trait, batched single-request translation with segment
   reconstruction, typed provider errors, and Credential Manager for keys. Decide [PRD Q1](product/prd.md#open-questions) first.
3. **Measure the grab on a large desktop** in a release build, against the 250 ms budget.
4. **Wire `ts-rs`** so `types.ts` becomes generated and the CI drift check from
   [operations/ci-cd.md](operations/ci-cd.md) can catch IPC divergence. The contract grew again this milestone,
   by hand, on both sides.

See [project/roadmap.md](project/roadmap.md) for the full milestone sequence.
