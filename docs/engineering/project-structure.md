---
title: Project Structure
discipline: code
status: active
updated: 2026-09-10
---

# Project Structure

> **Purpose.** Where everything lives and why, so new code lands in the right place without asking.
> **Related.** [../architecture/overview.md](../architecture/overview.md) · [coding-standards.md](coding-standards.md) · [naming-conventions.md](naming-conventions.md)

> **Status:** the frontend half is **scaffolded and building**; `src-tauri/` does not exist yet. Lines marked
> `[planned]` are the target shape for [M1–M5](../project/roadmap.md).

## Layout

```
on-screen-translator/
├─ overlay.html · panel.html · settings.html   # One Vite entry per window (at root, so
│                                              # dev URLs stay /overlay.html)
├─ src/                          # Frontend — React + TS + Tailwind. Presentation ONLY.
│  ├─ entries/                   #   Mount points: overlay.tsx, panel.tsx, settings.tsx, mount.tsx
│  ├─ overlay/                   #   The capture surface: scrim, selection, in-place results
│  │  ├─ Overlay.tsx             #   Phase machine: loading → selecting → working → result
│  │  ├─ SelectionLayer.tsx      #   Drag rectangle + live size readout
│  │  ├─ ResultLayer.tsx         #   Translated lines positioned at their bboxes
│  │  └─ fitText.ts              #   The shrink → wrap → clip ladder (pure, tested)
│  ├─ panel/                     #   Frameless always-on-top result panel
│  │  └─ Panel.tsx
│  ├─ settings/                  #   The small settings window
│  │  └─ Settings.tsx
│  ├─ lib/
│  │  ├─ ipc.ts                  #   The ONLY module that calls Tauri commands
│  │  ├─ coords.ts               #   CSS px ↔ physical px. The conversion boundary. Pure, tested.
│  │  └─ types.ts                #   The IPC contract. Hand-authored until ts-rs generation
│  │                             #   lands with the Rust types, then becomes types.gen.ts
│  └─ styles/                    #   index.css — Tailwind v4 (@import, no JS config file)
│
├─ src-tauri/                    # [planned] Rust core — everything expensive
│  ├─ src/
│  │  ├─ main.rs                 #   Bootstrap only. No logic.
│  │  ├─ app.rs                  #   Single-instance guard, window lifecycle, shutdown
│  │  ├─ tray.rs                 #   Tray icon, menu, state badges
│  │  ├─ hotkey.rs               #   Global shortcut register / rebind / conflicts
│  │  ├─ capture/
│  │  │  ├─ mod.rs
│  │  │  ├─ monitors.rs          #   Topology, virtual bounds, per-monitor DPI  [WINDOWS]
│  │  │  ├─ grab.rs              #   The single virtual-desktop frame grab      [WINDOWS]
│  │  │  └─ geometry.rs          #   Rect math, crop. Pure — heavily tested.
│  │  ├─ preprocess.rs           #   Upscale, grayscale, contrast, threshold
│  │  ├─ ocr/
│  │  │  ├─ mod.rs               #   `OcrEngine` trait + line assembly
│  │  │  ├─ windows_ocr.rs       #   Windows.Media.Ocr implementation           [WINDOWS]
│  │  │  └─ assemble.rs          #   Lines → blocks → segments. Pure, tested.
│  │  ├─ translate/
│  │  │  ├─ mod.rs               #   `TranslationProvider` trait + batching
│  │  │  ├─ libretranslate.rs
│  │  │  ├─ deepl.rs
│  │  │  ├─ google.rs
│  │  │  └─ llm.rs               #   Any OpenAI-compatible endpoint
│  │  ├─ pipeline.rs             #   crop → preprocess → ocr → translate; cancellation, timings
│  │  ├─ settings.rs             #   JSON load/save, defaults, migration, corrupt recovery
│  │  ├─ secrets.rs              #   Credential Manager. The ONLY module touching keys. [WINDOWS]
│  │  ├─ autostart.rs            #   HKCU run entry                             [WINDOWS]
│  │  ├─ protocol.rs             #   otr:// frame handler
│  │  ├─ ipc.rs                  #   Tauri commands + events — the api.md contract
│  │  ├─ error.rs                #   AppError, ErrorCode, user-facing messages
│  │  └─ logging.rs              #   tracing setup + the no-content rule
│  ├─ tests/                     #   Integration tests (fake provider, fixture images)
│  ├─ fixtures/                  #   Test screenshots: small text, low contrast, monospace, prose
│  ├─ icons/
│  ├─ Cargo.toml
│  └─ tauri.conf.json            #   Version source of truth; window defs; CSP
│
├─ docs/                         # Documentation — the source of truth
├─ .github/workflows/            # [planned] CI: check on push, build+release on tag
├─ CLAUDE.md                     # The brain — read first
├─ SCRATCH.md                    # Working notes (git-ignored)
├─ README.md · CHANGELOG.md · LICENSE
└─ package.json · vite.config.ts · tsconfig.json · eslint.config.js · .prettierrc.json
```

> **Tailwind v4** is configured in CSS (`@import 'tailwindcss'` + `@theme` in `src/styles/index.css`) through the
> `@tailwindcss/vite` plugin. There is no `tailwind.config.ts` — v4 removed the need for one.

## What Goes Where

**Decision table for "where does this code go?"**

| If the code…                                          | It belongs in…                        |
| ------------------------------------------------------ | --------------------------------------- |
| Touches the network                                    | `src-tauri/src/translate/` — nowhere else |
| Touches the filesystem                                 | `settings.rs` or `logging.rs`           |
| Touches an API key                                     | `secrets.rs` — nowhere else             |
| Calls a Windows API                                    | A module marked `[WINDOWS]` above       |
| Does rectangle math                                    | `capture/geometry.rs` or `lib/coords.ts` |
| Converts between CSS and physical pixels               | `lib/coords.ts` — **nowhere else**      |
| Decides what the user sees                             | `src/`                                  |
| Decides what happens                                   | `src-tauri/`                            |
| Is a Tauri command                                     | `ipc.rs`, delegating immediately        |

## Boundaries

Four rules. Breaking any of them is the thing to catch in review:

**1. The frontend is presentation.** No network, no filesystem, no secrets, no OS access. If a component needs one
of those, it needs a command instead ([NFR-S5](../product/requirements-nfr.md#security)).

**2. `ipc.ts` is the only door.** No component calls `invoke` directly. One module means the whole IPC surface is
greppable and mockable in tests.

**3. `ipc.rs` handlers contain no logic.** They validate, delegate to `pipeline`/`settings`/`secrets`, and map errors.
Logic in a command handler is logic that can't be tested without Tauri.

**4. Platform code is confined and marked.** Windows APIs appear only in the `[WINDOWS]` modules, behind traits or
plain functions. This is what keeps [NFR-M2](../product/requirements-nfr.md#maintainability) true — a port becomes
"write new implementations", not "restructure everything".

### The dependency direction

```
ipc.rs  →  pipeline.rs  →  { capture, preprocess, ocr, translate }
                        ↘  settings.rs  →  secrets.rs
```

Nothing lower reaches back up. `pipeline` doesn't know about Tauri; `ocr` and `translate` don't know about
`pipeline`. This is what lets the pipeline be tested with a fake engine and a fake provider, with no screen and no
network ([../quality/testing-strategy.md](../quality/testing-strategy.md)).

## Conventions

- **Folders** lowercase-kebab. **Rust files** `snake_case`. **React components** `PascalCase.tsx`. **Other TS**
  `camelCase.ts`. Details in [naming-conventions.md](naming-conventions.md).
- **One trait per domain, in that domain's `mod.rs`.** Implementations are siblings.
- **The IPC types live in one file.** Today that is `src/lib/types.ts`, hand-authored against
  [../architecture/api.md](../architecture/api.md). Once the Rust types exist it becomes `types.gen.ts`, generated
  via `ts-rs` with a CI drift check — at which point hand-editing it is a bug.
- **Pure logic goes in its own module** (`geometry.rs`, `assemble.rs`, `coords.ts`, `fitText.ts`) precisely so it can
  be tested without a screen, a network, or a webview. These four files carry the highest bug risk in the project
  and the lowest testing cost — that is not a coincidence, it's the reason they're separated.
- **Fixtures live in the repo.** OCR accuracy work is meaningless without a stable set of real screenshots to
  measure against.
- **No barrel `index.ts` files.** They obscure the import graph for no benefit at this size.
