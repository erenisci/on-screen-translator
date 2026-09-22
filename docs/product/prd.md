---
title: Product Requirements Document
discipline: product
status: active
updated: 2026-09-10
---

# Product Requirements Document — on-screen-translator

> **Purpose.** The single source of _what_ we're building and _why_, for v1.
> **Related.** [requirements-functional.md](requirements-functional.md) · [requirements-nfr.md](requirements-nfr.md) · [roadmap-vision.md](roadmap-vision.md) · [feature-specs.md](feature-specs.md)

## Problem

Reading text you can't select is a daily papercut: game UIs, installers, error dialogs, scanned PDFs, video
subtitles, screenshots pasted into chat, remote desktop sessions. The current workaround costs six steps —
screenshot → open a browser → open a translate site → paste/upload → wait → alt-tab back — for something that
should take one keypress.

Existing tools each fail on one axis:

| Tool                            | Reads the screen | Translates | Open source | Free      |
| ------------------------------- | ---------------- | ---------- | ----------- | --------- |
| Browser translators             | No               | Yes        | No          | Yes       |
| PowerToys Text Extractor        | Yes              | **No**     | Yes         | Yes       |
| Google Lens                     | Yes              | Yes        | No          | Mobile-first |
| Commercial screen translators   | Yes              | Yes        | **No**      | Paid/ads  |

Nothing free, open source, and privacy-auditable does the full Lightshot-speed loop on Windows.

## Goals

- **G1 — One-keypress meaning.** Hotkey → drag → translation visible, with no other interaction required.
- **G2 — Under ~1s** from mouse-release to visible translation for a normal paragraph (see [NFR](requirements-nfr.md)).
- **G3 — Negligible idle cost.** ≤ 30 MB RAM and ~0% CPU while sitting in the tray.
- **G4 — In-place reading.** Translation is drawn over the original text using OCR bounding boxes, so the user
  reads it where the words actually were — without leaving the capture overlay.
- **G5 — The user's own language.** Target language is a setting, defaulting to the Windows display language.
  Nothing is hardcoded to a single language pair.
- **G6 — Auditable privacy.** The captured image never leaves the machine; only extracted text is sent, only to
  the provider the user chose. No telemetry, ever.

## Non-Goals

Explicitly out of scope for v1 — see [roadmap-vision.md](roadmap-vision.md) for when these return:

- Live / continuous re-translation of a pinned region (subtitles, streams).
- Translation history, search, pinning.
- Fully offline translation (bundled local model).
- Saving the annotated capture as PNG, or text-to-speech.
- macOS / Linux ports.
- Auto-update.
- A glossary / do-not-translate list.
- Any account, sync, backend, or paid tier.

## Target Users

**Primary — the non-native reader.** Someone who reads a language they don't speak on screen every day and wants
meaning without breaking flow. The author's own case: a Turkish speaker reading English UIs and docs.

**Secondary:**

| Persona    | Trigger                                          | What they need most                |
| ---------- | ------------------------------------------------ | ---------------------------------- |
| Gamer      | Untranslated game UI, quest text                 | Works over fullscreen; fast        |
| Developer  | Foreign-language error dialog, stack trace image | Accurate on small monospace text   |
| Student    | Scanned PDF, slide deck                          | Paragraph accuracy; copyable text  |
| Support    | User-submitted screenshot in another language    | Quick one-off, no setup            |

## Scope (v1)

The must-have core. Each item has a functional requirement in
[requirements-functional.md](requirements-functional.md) and behaviour detail in [feature-specs.md](feature-specs.md).

1. **Tray-only background app** — no taskbar window, no console. One tray icon with: Capture, Settings, Quit.
2. **Start with Windows** — opt-in on first run, toggleable in Settings, starts minimized to tray.
3. **Global hotkey** (default `Ctrl+Shift+T`, rebindable) working from any focused app, including fullscreen.
4. **Lightshot-style capture overlay** — screen frozen to a still image, dimmed, drag a rectangle, live dimension
   readout, `Esc` cancels. Multi-monitor and per-monitor DPI correct.
5. **OCR of the selected region** with automatic source-language detection and pre-processing (upscale, grayscale,
   contrast/threshold) to lift accuracy on small or low-contrast text.
6. **Translation into the user's target language**, defaulting to the Windows display language.
7. **In-place result without leaving the overlay** — translated lines drawn over the original blocks; the user can
   keep selecting further regions without re-pressing the hotkey.
8. **Copyable result panel** — always-on-top, transparent, showing source text and translation with per-side copy
   buttons and a re-translate control.
9. **Small Settings window** — target language, hotkey, provider + API key, OCR languages, start-with-Windows,
   theme, result display mode (overlay / panel / both).

## Success Metrics

| #  | Metric                                       | Target                                  |
| -- | -------------------------------------------- | --------------------------------------- |
| S1 | Hotkey → visible translation (one paragraph) | < 1s p50, < 2s p95                      |
| S2 | Idle RAM / CPU in tray                       | ≤ 30 MB / ~0%                           |
| S3 | Correct capture on multi-monitor mixed-DPI   | 100% — no scale-factor drift            |
| S4 | Clone-to-running-build                       | One documented command, on a clean machine |
| S5 | External adoption                            | ≥ 1 issue or PR from someone who isn't the author |
| S6 | Author's own behaviour                       | Stops opening a browser to translate anything |

## Assumptions

- **A1** — Windows 10 1809+ / Windows 11. `Windows.Media.Ocr` is present on all supported versions.
- **A2** — The user can install OCR language packs through Windows Settings when a language is missing; we detect
  and guide, we don't bundle them.
- **A3** — The user either accepts the keyless default provider or supplies their own API key.
- **A4** — Text on screen is horizontal. Vertical scripts are out of scope for v1 (see Non-Goals).
- **A5** — A solo developer with no deadline. Ship a usable v1, then iterate.

## Open Questions

- **Q1** — Which keyless default provider is reliable enough to ship as the zero-config path? Public
  LibreTranslate instances rate-limit aggressively. Decide before v0.2; may become "pick a provider on first run".
- **Q2** — Code-signing certificate: unsigned builds will trip SmartScreen and AV heuristics (a tray app with a
  global hook + screen capture looks like a keylogger). Cost vs. friction not yet decided — tracked as
  [R-AV in tech-debt](../project/tech-debt.md).
- **Q3** — When translated text overflows its source bounding box, what wins: shrink, wrap-and-grow, or truncate
  with hover? Needs real-usage testing; v1 ships the heuristic in [feature-specs.md](feature-specs.md).
