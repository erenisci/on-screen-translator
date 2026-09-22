---
title: Product Roadmap (Vision)
discipline: product
status: active
updated: 2026-09-10
---

# Product Roadmap — Vision

> **Purpose.** Where this product is going and why, at the theme level. Dated execution milestones live in
> [../project/roadmap.md](../project/roadmap.md).
> **Related.** [prd.md](prd.md) · [requirements-functional.md](requirements-functional.md) · [../project/roadmap.md](../project/roadmap.md)

## Vision

**Anything readable on your screen should be readable in your language, in one keypress.**

The screen is full of text you can't select: games, installers, dialogs, scanned documents, video, remote sessions.
Every existing answer makes you leave what you're doing. on-screen-translator collapses that into one gesture —
press, drag, read — and does it as an open, auditable, free tool that never sees your screen.

The long-term shape is a **local-first reading aid**: OCR on your machine, translation through a provider you
choose (or eventually none at all), no account, no server, no telemetry. Success is the app becoming invisible
infrastructure — the thing you press without thinking.

## Themes

### T1 — Speed is the product

A translator that takes three seconds loses to alt-tabbing. Every design choice is subordinate to the sub-second
loop: capture once and crop in memory, batch provider calls, create windows on demand, and never poll while idle.
Budgets are enforced in [requirements-nfr.md](requirements-nfr.md), not left to intent.

### T2 — Read it where it was

Text has a place on screen, and that place carries meaning — which button, which column, which label. Dumping OCR
output into a text box throws that away. Drawing the translation back over the original blocks is the feature that
separates this from a screenshot-plus-paste workflow, and it stays the centre of the product.

### T3 — Your screen stays yours

The image never leaves the device; only extracted text is sent, only to the provider the user picked, only when the
user asks. No telemetry, no analytics, no update pings. This is a promise the architecture must make verifiable, not
a line in a privacy policy — which is why OCR is local and all network calls funnel through one auditable layer.

### T4 — Your language, not English

The target language is a setting that defaults to the user's own Windows display language. The app is built by a
Turkish speaker for the general case of "I don't read this language", not as an English-learning tool.

### T5 — Free, open, and cheap to maintain

MIT, no backend, no paid tier, no key shipped in the binary. A solo maintainer must be able to run this forever at
zero cost — which rules out anything needing hosting, and makes "the user brings their own key" a design principle
rather than a limitation.

### T6 — Honest when it fails

OCR misreads and providers go down. The app must say what happened and what to do, keep the source text copyable
when translation fails, and flag low-confidence recognition instead of presenting a confident wrong answer.

## Now / Next / Later

### Now — v1: the loop works

The complete press-drag-read cycle, correct on multi-monitor mixed-DPI setups.

- Tray-only app, autostart, global hotkey
- Frozen-screen capture overlay with drag selection
- Local OCR with pre-processing and language detection
- Batched translation through a pluggable provider
- **In-place translated overlay** + copyable result panel
- Small settings window; keys in Credential Manager

Out: history, live mode, offline translation, other platforms.

### Next — v1.x: make it trustworthy

Once the loop is real, remove the reasons people would abandon it.

- **Second OCR engine** (Tesseract behind the existing trait) for languages Windows OCR lacks — the biggest
  accuracy complaint we expect
- **Provider reliability**: better failure messages, sane fallback when the keyless default is rate-limited,
  resolving [PRD Q1](prd.md#open-questions)
- **Translation history** with search — the most-requested feature in every tool of this shape
- **Save / copy the annotated capture** as an image
- **Code signing** to stop SmartScreen and AV false positives ([PRD Q2](prd.md#open-questions))
- Accuracy work on small and low-contrast text, driven by real failures

### Later — v2: beyond the single capture

Directional, not committed. Each needs its own spec before it starts.

- **Live mode** — pin a region and re-translate as its content changes (subtitles, streams, chat). The single
  biggest jump in usefulness, and the biggest jump in complexity: it breaks the "nothing runs while idle" rule and
  needs its own performance budget.
- **Fully offline translation** via a bundled local model — closes the privacy story completely, at a real cost in
  binary size and quality.
- **Text-to-speech** for source or translation.
- **Glossary / do-not-translate list** for names and technical terms.
- **Auto-update.**
- **macOS / Linux ports** — only worth it if the Windows version finds real users. The architecture keeps platform
  code isolated ([NFR-M2](requirements-nfr.md#maintainability)) so this stays possible without a rewrite.

## What we will not do

Recorded so they aren't re-argued:

- No account, no cloud sync, no backend of our own.
- No telemetry or analytics, not even "anonymous" usage counts.
- No ads, no paid tier, no bundled offers in the installer.
- No shipped API key — anything requiring a key uses the user's own.
- No general screenshot/annotation tool. Lightshot exists; we borrow its interaction, not its scope.
