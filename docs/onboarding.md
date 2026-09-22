---
title: Onboarding
discipline: knowledge
status: active
updated: 2026-09-10
---

# Onboarding

> **Purpose.** Clone to running build, plus the handful of things that will confuse you in the first hour.
> **Related.** [engineering/project-structure.md](engineering/project-structure.md) · [architecture/overview.md](architecture/overview.md) · [maintenance.md](maintenance.md)

> **Status:** the project is greenfield — the scaffold doesn't exist yet ([M1](project/roadmap.md)). The Setup
> section below is the target; the Scaffolding section is what to run first.

## Prerequisites

| Requirement                 | Notes                                                                  |
| --------------------------- | ------------------------------------------------------------------------ |
| **Windows 10 1809+ / 11**   | Not optional — `Windows.Media.Ocr` and the capture path are Windows-only |
| **Rust** (stable, MSVC)     | `rustup default stable-x86_64-pc-windows-msvc`                          |
| **Visual Studio Build Tools** | "Desktop development with C++" workload — the MSVC linker              |
| **Node.js 20+**             | For the frontend build                                                  |
| **WebView2 runtime**        | Already present on Windows 11 and most Windows 10                       |
| **An OCR language pack**    | Windows Settings → Time & language → add a language with OCR support    |

**Strongly recommended: a second monitor at a different scaling factor.** This project's worst bugs are invisible on
a single-monitor machine ([ADR-0004](architecture/adr/0004-capture-and-coordinate-model.md)). If you can't manage
that, at least change your primary display's scaling between test runs.

## Setup

```bash
git clone https://github.com/<owner>/on-screen-translator
cd on-screen-translator
npm install
npm run tauri dev
```

The app starts into the tray — **there is no window**. That's correct. Press `Ctrl+Shift+T`.

## Scaffolding (first time only)

The repo currently holds docs and no code. To create the project:

```bash
npm create tauri-app@latest -- --template react-ts
# then add Tailwind, configure the three window entries, and lay out the
# modules exactly as in docs/engineering/project-structure.md
```

Follow [engineering/project-structure.md](engineering/project-structure.md) precisely — the module boundaries there
aren't cosmetic, they're what make the pipeline testable without a screen and keep platform code contained.

## First Run

1. `npm run tauri dev` → tray icon appears, no window.
2. `Ctrl+Shift+T` → the screen freezes and dims.
3. Drag a box around some text.
4. The translation appears **over the original text**, and a panel opens with copyable source and translation.
5. Drag again without pressing the hotkey. `Esc` to exit.

If step 2 does nothing, the hotkey is probably claimed by another app — check the tray tooltip and the log.

## Where Things Live

```
src/          Frontend (React + TS + Tailwind) — presentation ONLY
src-tauri/    Rust core — everything expensive: OS, pixels, network, secrets
docs/         The source of truth. Read before writing.
CLAUDE.md     The brain — start here
```

Full map with a "where does this code go?" table: [engineering/project-structure.md](engineering/project-structure.md).

## The five things that will confuse you

Read these before your first change; each one has cost someone an afternoon.

**1. There is no main window.** The app is a tray icon. All three windows (`overlay`, `panel`, `settings`) are
created on demand and destroyed after use — that's how the 30 MB idle budget is met
([NFR-P4](product/requirements-nfr.md#performance)). If you're looking for `App.tsx`, there isn't one.

**2. Two pixel spaces exist, and mixing them is the project's worst bug class.** Core coordinates are **physical
pixels in virtual-desktop space** (the origin can be negative). CSS pixels exist only inside a webview. Conversion
happens in `src/lib/coords.ts` and **nowhere else**. Any variable holding a coordinate is named `_physical` or
`_css` ([ADR-0004](architecture/adr/0004-capture-and-coordinate-model.md),
[naming-conventions.md](engineering/naming-conventions.md)).

**3. The screen is captured once per session.** On hotkey press, the whole virtual desktop is grabbed into one
buffer. Every selection is a crop of that buffer — the screen is never re-captured. That's why the frame is genuinely
frozen and why re-selecting is free.

**4. Nothing runs while idle.** No timer, no watcher, no background task, no window. If you add one, you've changed
the architecture and you owe an [ADR](architecture/adr/README.md). This is checked in
[definition-of-done.md](project/definition-of-done.md) for a reason.

**5. The frontend has no network, no filesystem, no secrets.** Every such action is a Tauri command, and all
`invoke` calls go through `src/lib/ipc.ts`. This isn't style — it's what makes the privacy guarantee auditable at one
boundary ([security.md](operations/security.md)).

## Common Tasks

| Task                       | Command                                                     |
| -------------------------- | ------------------------------------------------------------- |
| Run in dev                 | `npm run tauri dev`                                         |
| Build a release locally    | `npm run tauri build`                                       |
| Rust tests                 | `cargo test` (in `src-tauri/`)                              |
| Frontend tests             | `npm test`                                                  |
| Lint everything            | `cargo clippy -- -D warnings` · `npm run lint`              |
| Format                     | `cargo fmt` · `npm run format`                              |
| Verbose logging            | `RUST_LOG=otr=debug npm run tauri dev`                      |
| Find the logs              | `%APPDATA%\on-screen-translator\logs\`                      |
| Find the settings          | `%APPDATA%\on-screen-translator\settings.json`              |
| Reset to defaults          | Delete `settings.json` and restart                          |

## Before your first PR

1. Read [architecture/overview.md](architecture/overview.md) — 5 minutes, saves hours.
2. Skim the four [ADRs](architecture/adr/README.md) — they explain why things are the way they are.
3. Check [engineering/coding-standards.md](engineering/coding-standards.md#anti-patterns) — the anti-pattern table is
   project-specific and not what you'd guess.
4. Walk [engineering/self-review-checklist.md](engineering/self-review-checklist.md) before opening the PR.

## Getting stuck

| Symptom                              | Likely cause                                                            |
| ------------------------------------ | ------------------------------------------------------------------------- |
| Hotkey does nothing                   | Another app claimed it — check the log and the tray tooltip              |
| Build fails on a linker error         | MSVC Build Tools missing ("Desktop development with C++")                |
| Overlay appears on the wrong monitor  | A coordinate conversion outside `coords.ts` — see confusion #2           |
| Crop doesn't match the selection      | Same. Check the scale factor and the virtual origin                      |
| OCR returns nothing                   | Language pack not installed for that language                            |
| Translation fails immediately         | Default provider rate-limited ([TD-02](project/tech-debt.md)) — set a key |

More in [maintenance.md](maintenance.md).
