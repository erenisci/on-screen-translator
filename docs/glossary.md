---
title: Glossary
discipline: knowledge
status: active
updated: 2026-09-10
---

# Glossary

> **Purpose.** Shared vocabulary. When a term here appears in code, a doc, or an issue, it means exactly this.
> **Related.** [architecture/overview.md](architecture/overview.md) · [engineering/naming-conventions.md](engineering/naming-conventions.md)

## Terms (A–Z)

**Accelerator** — A hotkey string in Tauri's format, e.g. `Ctrl+Shift+T`. Stored in settings as `hotkey`.

**Alignment (segment alignment)** — The mapping from translated segments back onto OCR line bounding boxes. When a
provider merges, splits, or reorders segments, alignment fails, `alignmentOk` is `false`, and the app falls back to
panel-only display rather than putting the wrong translation on the wrong element
([TD-06](project/tech-debt.md)).

**Block** — A group of OCR lines joined into a paragraph by vertical proximity. Blocks are what get sent for
translation, because sentence context is what makes translation good; **lines** are what get positioned on screen.

**Capture session** — Everything between a hotkey press and the overlay closing. One frame is captured per session;
every selection within it is a crop of that frame.

**Core** — The Rust half of the app (`src-tauri/`). Owns OS access, pixels, network, and secrets. Contrast with
**frontend**.

**Credential Manager** — The Windows secure credential store. The only place API keys are ever written
([security.md](operations/security.md)).

**CSS pixel** — A pixel inside a webview, subject to `devicePixelRatio`. One of the two pixel spaces in this project.
Variables holding one are named `_css`. Contrast with **physical pixel**.

**Degraded (state)** — The app is running but something is wrong: the hotkey failed to register, or no provider is
configured. Shown as a badge on the tray icon.

**Display mode** — The `resultDisplayMode` setting: `overlay`, `panel`, or `both`. Controls where a translation
appears.

**Fitting ladder** — The three-step strategy for making a translation fit its source bounding box: shrink toward a
10 px floor, then wrap and grow downward, then clip with the full text on hover
([F7](product/feature-specs.md)).

**Frame** — The single captured image of the entire virtual desktop, held in memory for one capture session. Never
written to disk.

**Frontend** — The React/TypeScript half (`src/`). Presentation only: no network, no filesystem, no secrets.

**In-place overlay** — The headline feature: translated text drawn over the original text blocks inside the capture
overlay, using OCR bounding boxes, without leaving capture mode.

**IPC** — The Tauri command and event boundary between core and frontend. The project's only cross-language
interface; contract in [architecture/api.md](architecture/api.md).

**Keyless default** — The translation provider that works with no API key and no account, so the app functions on
first run. Currently LibreTranslate; its reliability is the project's largest open risk
([TD-02](project/tech-debt.md)).

**Line** — One recognized row of text from OCR, with `text`, `bbox`, and `confidence`. The unit the in-place overlay
positions. Contrast with **block**.

**Overlay** — The full-virtual-desktop window showing the frozen frame, the dim scrim, the selection, and the
in-place results. The app's main surface.

**Panel** — The frameless, always-on-top result window with copyable source and translation.

**Physical pixel** — An actual device pixel. **The core's only coordinate unit**, with origin at the top-left of the
virtual desktop. Variables holding one are named `_physical`
([ADR-0004](architecture/adr/0004-capture-and-coordinate-model.md)).

**Pre-processing** — The image transformations applied before OCR to raise accuracy: upscale small regions,
grayscale, contrast normalize, adaptive threshold. Applied conditionally — it can hurt on already-clean regions.

**Provider** — A translation service behind the `TranslationProvider` trait: LibreTranslate, DeepL, Google, or an
LLM endpoint. Always uses the **user's** key ([ADR-0003](architecture/adr/0003-translation-provider-abstraction.md)).

**Scrim** — The dimming layer over everything outside the selection rectangle.

**Segment** — One unit of text sent to a provider in a batch. Segments correspond to lines or blocks; the response
must return the same number, or **alignment** fails.

**Session (capture session)** — See **capture session**.

**Target language** — The language the user reads. Defaults to the Windows display language, never hardcoded
([FR-40](product/requirements-functional.md#translation)).

**Tray-only** — The app has no taskbar entry, no `Alt+Tab` entry, and no main window. Its entire permanent surface
is one system-tray icon.

**Virtual desktop** — The combined rectangle spanning all monitors. **Its origin can be negative** when a monitor
sits above or to the left of the primary — the single most commonly mishandled fact in screen-capture code.

**WebView2** — The Microsoft Edge-based webview Tauri renders into. A runtime dependency on the user's machine, not
something we bundle — which is why the install is ~10 MB instead of ~100 MB.

**Windows OCR** — `Windows.Media.Ocr`, the OCR engine built into Windows 10 1809+. v1's only OCR engine, chosen
because it adds zero install bytes and is tuned for rendered screen text
([ADR-0002](architecture/adr/0002-ocr-engine.md)).

## Abbreviations used in docs

| Short   | Means                                                                             |
| ------- | ----------------------------------------------------------------------------------- |
| **ADR** | Architecture Decision Record — [architecture/adr/](architecture/adr/README.md)     |
| **DoD** | Definition of Done — [project/definition-of-done.md](project/definition-of-done.md) |
| **DoR** | Definition of Ready — [project/definition-of-ready.md](project/definition-of-ready.md) |
| **FR**  | Functional Requirement — [product/requirements-functional.md](product/requirements-functional.md) |
| **NFR** | Non-Functional Requirement — [product/requirements-nfr.md](product/requirements-nfr.md) |
| **TD**  | Tech Debt item — [project/tech-debt.md](project/tech-debt.md)                      |
| **M1–M8** | Milestones — [project/roadmap.md](project/roadmap.md)                            |
| **G1–G7** | Security guarantees — [operations/security.md](operations/security.md)           |
