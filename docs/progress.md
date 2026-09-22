---
title: Progress
discipline: project
status: active
updated: 2026-09-22
---

# Progress

> **Purpose.** The current state of the project at a glance. Updated by `/acta:track` after each chunk of work.
> **Related.** [project/roadmap.md](project/roadmap.md) · [../CHANGELOG.md](../CHANGELOG.md) · [project/tech-debt.md](project/tech-debt.md)

## Current Status

**Stage:** early — both halves of the walking skeleton exist and the app runs.
**Version:** 0.1.0 (unreleased)
**Active milestone:** M1 — Walking skeleton (code complete; **not** done per the DoD)

The app boots into the tray, registers the global hotkey, captures the virtual desktop, and opens the overlay.
122 tests pass (66 TypeScript, 56 Rust); `tsc`, ESLint, Prettier, `cargo fmt` and `cargo clippy -D warnings` are
all clean.

**M1 is not "done".** Its definition of done requires a box drawn on a 150%-scaled monitor to produce a crop
containing exactly what was inside it, verified on real mixed-DPI hardware. That has not happened — every
coordinate claim so far rests on unit tests and a single-monitor run.

## In Progress

**M1 — Walking skeleton.**

| Part                                                        | State                                                      |
| ----------------------------------------------------------- | ------------------------------------------------------------ |
| Coordinate model — `coords.ts` + `geometry.rs`               | **Done** — tested on both sides of the IPC boundary          |
| Tray icon, menu, single instance, clean quit                 | **Done** — single-instance verified by running two copies    |
| Global hotkey registration + live rebinding                  | **Done** — registration verified; rebinding untested by hand |
| Virtual-desktop capture (GDI `BitBlt`) + crop                | **Done** — one real capture, buffer geometry exactly right   |
| `otr://frame` protocol handler                               | **Written, never served a byte**                             |
| Overlay window (scrim, drag selection, size readout, `Esc`)  | **Written, never seen rendering**                            |
| Settings persistence + OS language detection                 | **Done** — `tr` auto-detected from Windows with no config    |
| In-place result layer, panel, settings UI                    | Written ahead of M4/M5; waiting on OCR and translation       |
| **Mixed-DPI multi-monitor verification**                     | **Not started — this is what gates M1**                      |

## Measured (2026-09-22, debug build, single 1920×1080 display)

The project's first real numbers, against [the budgets](product/requirements-nfr.md#performance):

| Metric                    | Measured              | Budget          | Verdict                       |
| ------------------------- | --------------------- | --------------- | ------------------------------- |
| Idle working set          | 21.3 MB               | ≤ 30 MB         | **Pass** (and it's a debug build) |
| Idle private memory       | 6.4 MB                | —               | —                               |
| Idle CPU over 20 s        | ~0.4%                 | ~0%             | **Pass**                        |
| Virtual-desktop grab      | **183 ms**            | part of 250 ms  | **At risk** — see below         |
| Frame buffer size         | 8,294,400 B           | 1920×1080×4     | Exactly right                   |

**The grab is the concern.** NFR-P1 allows 250 ms for the entire hotkey→overlay-visible path, and 183 ms of that
went to the GDI capture alone on a single 1080p screen. It scales with pixel count, so a dual-4K desktop would
miss the budget outright. A release build will help; whether that is enough needs measuring on a large desktop
before M6.

## Done (recent)

| Date       | What                                                                                          |
| ---------- | ----------------------------------------------------------------------------------------------- |
| 2026-09-22 | Rust core: tray, single instance, hotkey, capture, crop, `otr://` protocol, settings, logging  |
| 2026-09-22 | `geometry.rs` — the Rust twin of `coords.ts`, incl. crops from a negative-origin desktop        |
| 2026-09-22 | Fixed: Quit did nothing. `prevent_exit()` was unconditional and blocked the user's own Quit     |
| 2026-09-22 | Redesigned the app icon — the first one spelled a Turkish word and baked in one language pair   |
| 2026-09-22 | Frontend scaffold: Vite 7 + React 19 + TS strict + Tailwind v4, three window entries            |
| 2026-09-22 | `coords.ts` (49 tests) and `fitText.ts` (17 tests); ESLint enforces the IPC boundary            |
| 2026-09-22 | IPC contract gaps closed: `get_panel_result`, `close_panel`, `otr://result`                     |
| 2026-09-10 | Doc set generated; ADR-0001..0004 accepted                                                      |

## Blocked

Nothing is blocked. Two decisions remain open but do not block M2:

- **[PRD Q1](product/prd.md#open-questions)** — which keyless provider ships as the zero-config default. Needed by M3.
- **[PRD Q2](product/prd.md#open-questions)** — code signing vs. SmartScreen friction. Needed by M7/M8.

## Next Up

1. **Close M1 properly.** Run the app, press the hotkey, and confirm: the overlay renders the frozen frame at 1:1,
   the scrim and drag selection behave, and `Esc` returns focus. Then repeat **on a multi-monitor mixed-DPI
   setup** and check the crop lands exactly where the box was drawn. Nothing else counts as M1 being done
   ([DoD](project/definition-of-done.md)).
2. **Measure the grab on a large desktop** in a release build, against the 250 ms budget.
3. **Wire `ts-rs`** so `types.ts` becomes generated and the CI drift check from
   [operations/ci-cd.md](operations/ci-cd.md) can catch IPC divergence.
4. **M2 — OCR**: the `OcrEngine` trait, the `Windows.Media.Ocr` implementation, the pre-processing ladder, and line
   assembly.

See [project/roadmap.md](project/roadmap.md) for the full milestone sequence.
