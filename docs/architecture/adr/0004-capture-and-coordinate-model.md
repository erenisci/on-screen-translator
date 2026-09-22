---
title: "ADR 0004: Capture once into virtual-desktop physical pixels"
discipline: code
status: Accepted
date: 2026-09-10
---

# ADR 0004: Capture once into virtual-desktop physical pixels

## Status

Accepted — 2026-09-10

## Context

This ADR exists because coordinate handling is the single most likely thing to quietly ruin this product. Every
competing tool gets multi-monitor mixed-DPI wrong at some point, and the symptom is the worst kind: the app appears
to work, but the crop is offset or scaled, so the OCR reads the wrong region and the user can't tell why the
translation is nonsense.

The forces:

- **Windows has at least three coordinate spaces in play** — physical pixels, DPI-scaled logical pixels (per
  monitor, and different per monitor), and CSS pixels inside a webview that applies its own `devicePixelRatio`.
- **The virtual desktop origin can be negative.** A monitor positioned left of the primary gives negative X. Code
  written against primary-monitor bounds silently breaks on that layout.
- **Per-monitor DPI is not a single number.** A 150% laptop screen next to a 100% external monitor means the scale
  factor changes mid-drag when a selection crosses the boundary.
- **Latency.** [NFR-P2](../../product/requirements-nfr.md#performance) allows under 1s end to end, and a second
  screen capture after the user releases the mouse would cost 50–150 ms for no benefit.
- **The screen must appear frozen** ([FR-20](../../product/requirements-functional.md#capture-overlay)) — a playing
  video must stop the instant the overlay appears, and the crop must match what the user saw when they drew the box.
- **Topology changes.** Monitors get plugged, unplugged, and rescaled between captures
  ([NFR-R4](../../product/requirements-nfr.md#reliability)).

Options considered for **when to capture**:

**A. Capture the region after the user releases the mouse.** Less memory. But the screen isn't actually frozen —
content can change between the overlay appearing and the release, so the user selects one thing and OCRs another. It
also puts a capture squarely inside the latency budget. Rejected: it breaks the core promise that what you see is
what you get.

**B. Capture the whole virtual desktop once at hotkey time, crop in memory.** More peak memory (a 4K dual-monitor
frame is ~60–130 MB as raw RGBA). But the freeze is genuine, the crop is instant, and re-selecting costs nothing —
which is what makes [FR-26](../../product/requirements-functional.md#capture-overlay) (drag again without
re-triggering) cheap rather than expensive.

Options for **the coordinate space**:

**C. Work in logical/DPI-scaled pixels.** Feels natural coming from the webview. But it requires a scale factor at
every boundary, and there is no single correct one on a mixed-DPI setup. This is the bug factory.

**D. Work in physical pixels in virtual-desktop space, converting exactly once.** One space, one origin, one
conversion point.

## Decision

**Capture the entire virtual desktop once per overlay session, and treat physical pixels in virtual-desktop space
as the only coordinate system in the core.**

**Capture.** On hotkey press, the core re-reads the monitor topology (never a cached layout,
[NFR-R4](../../product/requirements-nfr.md#reliability)) and captures every monitor into one frame buffer covering
the virtual desktop bounds. That buffer is the truth for the whole overlay session. The screen is never re-captured;
selections are crops of that buffer ([FR-30](../../product/requirements-functional.md#ocr)).

**Coordinate rule — the invariant every contributor must know:**

> All capture, crop, and OCR coordinates are **physical pixels**, origin at the **top-left of the virtual desktop**,
> which may be negative. Nothing downstream of the overlay rescales anything.

**Conversion happens exactly once**, in the overlay frontend: the selection rectangle in CSS pixels is converted to
physical virtual-desktop pixels using the overlay window's own scale factor and the virtual origin, before it is sent
over IPC. The Rust core receives physical pixels and never sees a logical coordinate. OCR bounding boxes come back
in frame coordinates and are converted back to CSS pixels once, at the same boundary, to position the overlaid text.

**The overlay window is created `PerMonitorV2` DPI-aware** and spans the full virtual-desktop bounds, so the webview
is never auto-scaled by Windows and the frozen frame can be rendered at 1:1 physical pixels.

**The frame reaches the frontend through a custom protocol handler** (`otr://frame`), not as a base64 IPC payload.
Base64-encoding a multi-megabyte PNG through JSON would cost more than the entire
[NFR-P1](../../product/requirements-nfr.md#performance) 250 ms budget.

**The buffer is dropped when the overlay closes**, and captured pixels are never written to disk
([NFR-S6](../../product/requirements-nfr.md#security), [NFR-P6](../../product/requirements-nfr.md#performance)).

## Consequences

**Easier**

- The freeze is real: what the user saw is exactly what gets OCR'd.
- Re-selecting is free — no capture, just another crop — which is what makes the "keep translating" flow work.
- One coordinate space means the mixed-DPI bug class mostly cannot be written: there is no second scale factor to
  get wrong.
- Multi-monitor selections that cross a boundary are unremarkable, because it's one continuous space.
- Testable without a screen: the coordinate math is pure functions over rectangles, and it gets unit tests
  ([../../quality/testing-strategy.md](../../quality/testing-strategy.md)).

**Harder**

- **Peak memory** during capture: ~60–130 MB for a 4K dual-monitor setup as raw RGBA.
  [NFR-P6](../../product/requirements-nfr.md#performance) caps it at 400 MB and requires release on close, so this
  needs to be verified rather than assumed in M6.
- **Capture cost is paid up front**, on every hotkey press, even for a small selection. That's the trade for the
  freeze, and it lands inside the 250 ms overlay budget.
- **The conversion boundary is load-bearing.** It's one place, but a mistake there is invisible on a single-monitor
  dev machine and obvious to every user with two. Hence the standing rule in
  [../../project/definition-of-done.md](../../project/definition-of-done.md): anything touching capture, coordinates,
  or windows is verified on a mixed-DPI multi-monitor setup before it's done.
- **The custom protocol handler** is more moving parts than an IPC payload, and needs its own lifetime management so
  the frame isn't served after the overlay is gone.
- **Exclusive fullscreen apps** may refuse capture entirely — a platform limitation, handled as a reported failure
  rather than a black rectangle ([TD-07](../../project/tech-debt.md)).

**At scale**

Capture-once is correct for the per-capture model and explicitly wrong for **live mode**
([Later](../../product/roadmap-vision.md)), which needs a streaming capture of one region with dirty-rectangle
detection, not a full-desktop snapshot per frame. That is a different design with a different memory profile and
will need its own ADR — this decision should not be stretched to cover it. The coordinate invariant, by contrast,
carries over unchanged and should outlive every other decision here.
