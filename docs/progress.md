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

**Stage:** early — repository initialized, frontend half of the scaffold built and green.
**Version:** 0.1.0 (unreleased)
**Active milestone:** M1 — Walking skeleton (partially done; the Rust half is blocked)

The frontend builds, type-checks, lints clean, and its 66 unit tests pass. The Rust core does not exist yet because
**the Rust toolchain is not installed on this machine**.

## In Progress

**M1 — Walking skeleton.** Split by what the missing toolchain allows:

| Part                                                      | State                                        |
| --------------------------------------------------------- | ---------------------------------------------- |
| Coordinate model (`src/lib/coords.ts`) + tests             | **Done** — 49 tests incl. mixed-DPI, negative origin |
| Fitting ladder (`src/overlay/fitText.ts`) + tests          | **Done** — 17 tests                            |
| IPC surface (`src/lib/ipc.ts`)                             | **Done** — the full contract, one door         |
| Overlay UI: scrim, drag selection, size readout, `Esc`     | **Done** — awaiting a core to talk to          |
| In-place result layer, result panel, settings window       | **Done** (ahead of M4/M5) — awaiting the core  |
| Tray, single instance, global hotkey, capture, crop        | **Blocked** — needs Rust                       |

## Done (recent)

| Date       | What                                                                                       |
| ---------- | ------------------------------------------------------------------------------------------- |
| 2026-09-22 | Frontend scaffold: Vite 7 + React 19 + TS strict + Tailwind v4, three window entries        |
| 2026-09-22 | `coords.ts` — the single conversion boundary, with 49 tests across a scale/origin grid      |
| 2026-09-22 | `fitText.ts` — the shrink → wrap → clip ladder, pure and injectable, 17 tests               |
| 2026-09-22 | `ipc.ts`, `types.ts`, overlay / panel / settings windows; eslint enforces the IPC boundary  |
| 2026-09-22 | Fixed `clampRectToBounds`: a one-axis overlap now collapses both dimensions, not just one   |
| 2026-09-22 | IPC contract gap closed: added `get_panel_result`, `close_panel`, `otr://result`            |
| 2026-09-22 | `git init`, MIT LICENSE, docs corrected to match the built shape                            |
| 2026-09-10 | Doc set generated; ADR-0001..0004 accepted                                                  |

## Blocked

🔴 **The Rust toolchain is missing.** No `rustc`, `cargo`, or `rustup`. Everything in `src-tauri/` — tray, hotkey,
capture, OCR, translation, the whole core — is blocked until it is installed:

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
winget install Microsoft.VisualStudio.2022.BuildTools   # "Desktop development with C++" workload
```

Present and verified: Node 24.20, npm 11.19, git 2.55, WebView2 153.

Two design decisions remain open but do not block M1–M2:

- **[PRD Q1](product/prd.md#open-questions)** — which keyless provider ships as the zero-config default. Needed by M3.
- **[PRD Q2](product/prd.md#open-questions)** — code signing vs. SmartScreen friction. Needed by M7/M8.

## Next Up

1. **Install the Rust toolchain** (above) — this unblocks everything else.
2. **Scaffold `src-tauri/`** per [engineering/project-structure.md](engineering/project-structure.md): `Cargo.toml`,
   `tauri.conf.json` with the three window definitions and a locked CSP, capabilities, and the module skeleton.
3. **Finish M1** — tray + single instance + hotkey, virtual-desktop capture, the `otr://frame` protocol handler,
   and `capture/geometry.rs` with the same coordinate tests `coords.ts` already has on the TypeScript side.
4. **Verify on a multi-monitor mixed-DPI setup** before calling M1 done — the standing rule in
   [project/definition-of-done.md](project/definition-of-done.md). Nothing written so far can be trusted on that
   front until it has run against a real second screen.

See [project/roadmap.md](project/roadmap.md) for the full milestone sequence.
