---
title: System Design
discipline: code
status: active
updated: 2026-09-10
---

# System Design

> **Purpose.** The mechanics behind [overview.md](overview.md) — how each budget is actually met, where the tricky
> parts are, and the trade-offs taken.
> **Related.** [overview.md](overview.md) · [api.md](api.md) · [adr/README.md](adr/README.md) · [../product/requirements-nfr.md](../product/requirements-nfr.md)

## Requirements this design must satisfy

| Requirement                                        | Source                                                   |
| -------------------------------------------------- | ---------------------------------------------------------- |
| < 250 ms hotkey → overlay visible                    | [NFR-P1](../product/requirements-nfr.md#performance)       |
| < 1s p50 release → translation visible               | [NFR-P2](../product/requirements-nfr.md#performance)       |
| ≤ 30 MB idle RAM, ~0% idle CPU                       | [NFR-P4/P5](../product/requirements-nfr.md#performance)    |
| Peak capture memory released on close                | [NFR-P6](../product/requirements-nfr.md#performance)       |
| Pixel-exact on multi-monitor mixed DPI               | [FR-21](../product/requirements-functional.md#capture-overlay) |
| Image never leaves the device; one auditable egress  | [NFR-S1/S5](../product/requirements-nfr.md#security)       |
| Never trap the user behind an invisible overlay      | [NFR-R1](../product/requirements-nfr.md#reliability)       |

## High-Level Design

### The idle state is the design

The app spends 99% of its life doing nothing, so "nothing" has to genuinely cost nothing:

- **No windows exist.** All three webviews are created on demand and destroyed after use.
- **No timers, no watchers, no polling.** The only live registrations are the tray icon and the global hotkey — both
  OS-level callbacks that consume nothing until they fire.
- **No async runtime work at rest.** Tokio is present for provider calls but has nothing scheduled while idle.
- **No preloaded models.** OCR is the OS engine; there is nothing to keep warm.

This is why [NFR-P5](../product/requirements-nfr.md#performance) is achievable and why
[DoD](../project/definition-of-done.md) has a standing check that no new work runs while idle. Any feature that
needs a background loop — live mode, auto-update — invalidates this design and needs its own ADR.

### Three windows, three lifetimes

| Window     | Created                | Destroyed                      | Notes                                                     |
| ---------- | ---------------------- | ------------------------------ | ----------------------------------------------------------- |
| `overlay`  | On hotkey / tray click | On `Esc`, cancel, or completion | Spans the virtual desktop, `PerMonitorV2`, always-on-top     |
| `panel`    | On a successful result | On `Esc` or overlay close       | Frameless, always-on-top, positioned near the selection      |
| `settings` | On tray menu           | On close                        | The only ordinary window in the app                          |

The overlay and panel are independent: closing one doesn't close the other, which is what lets a user keep a
translation on screen while re-selecting.

### Stage budget

Where the 1s goes, and what each stage is allowed:

| Stage                        | Budget   | How it's kept                                                        |
| ---------------------------- | -------- | ---------------------------------------------------------------------- |
| Hotkey → frame captured       | ~80 ms   | One GPU-assisted grab of the virtual desktop                          |
| Frame → overlay visible       | ~150 ms  | Frame served via custom protocol, not base64 IPC                      |
| Crop + preprocess             | ~30 ms   | In-memory crop; preprocessing skipped when the region is already clean |
| OCR                           | ~150 ms  | OS engine on a region, not a full screen                              |
| Translate (network)           | ~400 ms  | **One** request per capture; dominated by provider round trip         |
| Render in-place result        | ~50 ms   | Plain DOM positioning; no layout thrash                               |

Translation is the only stage we don't control, which is why it gets the largest share and why batching
([ADR-0003](adr/0003-translation-provider-abstraction.md)) matters more than any micro-optimization elsewhere.

## Components

### Capture and the coordinate model

The load-bearing part of the whole system ([ADR-0004](adr/0004-capture-and-coordinate-model.md)).

**On every hotkey press:**

1. Enumerate monitors and compute virtual-desktop bounds. **Never cached** — topology changes between captures
   ([NFR-R4](../product/requirements-nfr.md#reliability)).
2. Grab all monitors into one frame buffer covering those bounds.
3. Hold the buffer for the overlay session; drop it on close.

**The invariant, stated once so it's never re-derived:**

> Every coordinate in the core is a **physical pixel**, with origin at the **top-left of the virtual desktop** —
> which may be negative when a monitor sits left of the primary.

**Conversion happens in exactly one place.** The overlay frontend converts its CSS-pixel selection to physical
virtual-desktop pixels before sending it, and converts returned OCR boxes back for rendering. Nothing else in the
system rescales anything:

```
CSS px (webview)  ──×devicePixelRatio, +virtualOrigin──▶  physical px (core)
physical px (core) ──−virtualOrigin, ÷devicePixelRatio──▶  CSS px (overlay render)
```

These two conversions are pure functions and they get unit tests with mixed-DPI and negative-origin fixtures — they
are the cheapest possible insurance against the bug class that ruins tools in this category.

### Pre-processing ladder

Applied in order, and **conditionally** — pre-processing a large clean region costs time and can reduce accuracy:

1. **Upscale ×2–×3** (Lanczos) when estimated text height is under ~20 px. This is the single biggest accuracy win
   on UI text.
2. **Grayscale.**
3. **Contrast normalize.**
4. **Adaptive threshold** only when contrast is measured as low.

Kept outside the OCR engine so both current and future engines benefit and so it can be measured on its own
([ADR-0002](adr/0002-ocr-engine.md)).

### Line assembly

The OCR engine returns lines; translation quality needs sentences. Between them:

1. Sort lines top-to-bottom, then left-to-right.
2. Group into blocks by vertical gap relative to median line height.
3. Join each block into flowing text, so the provider sees a paragraph rather than fragments.
4. Keep the line → block → segment mapping, so translated output can be positioned back onto per-line boxes.

That mapping is what makes the in-place overlay possible while still sending context-rich text. It's pure logic over
rectangles and strings, so it's fully unit-testable ([../quality/testing-strategy.md](../quality/testing-strategy.md)).

### Translation batching

One request per capture ([FR-42](../product/requirements-functional.md#translation)). Segments go out as a
structured array where the provider supports it, delimited otherwise. **The response must return the same number of
segments**; if it doesn't, we fall back to panel-only display rather than guessing an alignment
([TD-06](../project/tech-debt.md)) — a translation on the wrong button is worse than no overlay.

### The in-place fitting ladder

For each line, translation is drawn at the source `bbox` with a contrast-guaranteed backing, then fitted:

1. **Shrink** toward a 10 px effective floor.
2. **Wrap** and grow downward into free space.
3. **Clip** with an ellipsis, full text on hover.

Boxes that would collide after growing nudge later boxes down. This ladder is the difference between the headline
feature working and it producing an unreadable pile ([F7](../product/feature-specs.md)).

### Cancellation

Every pipeline run is cancellable, and cancellation is checked between stages. `Esc` at any moment tears down the
overlay and abandons in-flight work. This is what makes [NFR-R1](../product/requirements-nfr.md#reliability)
achievable: there is no state in which a full-screen window stays up swallowing input while something is stuck.

### Failure handling

Every stage returns a typed error that carries a user-facing message and a suggested action
([../operations/error-handling.md](../operations/error-handling.md)). The rule that shapes the design: **a
translation failure must never destroy the OCR result.** The extracted source text stays on screen and copyable
([FR-44](../product/requirements-functional.md#translation)), because the expensive, unrepeatable part of the work
already succeeded.

## Trade-offs

| Choice                                  | Bought                                      | Paid                                                                 |
| --------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------- |
| Capture whole desktop once               | True freeze; free re-selection               | 60–130 MB peak on 4K dual-monitor ([NFR-P6](../product/requirements-nfr.md#performance)) |
| Windows built-in OCR                     | Zero install bytes; fast                     | Language coverage is Microsoft's ([TD-01](../project/tech-debt.md))     |
| One batched translation request          | Fits the latency budget; better context      | Segment-alignment risk ([TD-06](../project/tech-debt.md))              |
| Windows on demand                        | 30 MB idle budget met                        | 100–200 ms first-capture latency ([TD-04](../project/tech-debt.md))    |
| Custom protocol for the frame            | Avoids base64 cost inside the budget         | Extra moving part with its own lifetime management                     |
| No database                              | No migrations, no dependency, no complexity  | History and caching become real work when they arrive                  |
| User-supplied keys, no backend           | Zero cost forever; no shipped secret         | Fragile zero-config path ([TD-02](../project/tech-debt.md))            |

## Scaling

This is a single-user desktop tool: it doesn't scale along the axes a server does. The dimensions that actually
grow, and whether the design absorbs them:

| Growth                            | Absorbed?  | Why                                                                             |
| --------------------------------- | ---------- | --------------------------------------------------------------------------------- |
| More translation providers         | Yes        | One `impl TranslationProvider` + a Settings entry                                |
| More OCR engines                   | Yes        | One `impl OcrEngine`; pre-processing is already shared                           |
| More languages                     | Yes        | Windows language packs; no code change                                           |
| Bigger screens (8K, triple 4K)     | Mostly     | Memory scales linearly; watch [NFR-P6](../product/requirements-nfr.md#performance) |
| Translation history                | Partly     | Needs storage that doesn't exist yet — a real, if small, design addition         |
| **Live mode**                      | **No**     | Breaks the idle invariant and capture-once; needs its own ADR and budget         |
| Cross-platform                     | Partly     | Traits and module isolation help; capture, OCR, credentials, autostart are all Windows-specific |
