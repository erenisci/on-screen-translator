---
title: Technical Debt Log
discipline: project
status: active
updated: 2026-09-10
---

# Technical Debt Log

> **Purpose.** Known shortcuts, accepted risks, and deferred work — recorded so they're decisions, not surprises.
> **Related.** [../product/requirements-nfr.md](../product/requirements-nfr.md) · [../architecture/adr/README.md](../architecture/adr/README.md) · [../../SCRATCH.md](../../SCRATCH.md)

## How this list works

Items enter here when we knowingly accept a cost. Each has: what it is, why it exists, what it costs us, and how
we'd remediate. `/acta:track` drains [SCRATCH.md](../../SCRATCH.md) into this file when a scratch note turns out to
be structural rather than a quick fix.

Nothing below is implementation debt yet — the project is greenfield. These are **accepted risks and planned
shortcuts** identified during design, so they're visible before they bite.

## Debt items

### TD-01 — Single OCR engine (`Windows.Media.Ocr`)

**What.** v1 ships one OCR engine, the Windows built-in one.
**Why it exists.** It adds zero megabytes, needs no bundled model, and is fast — see
[ADR-0002](../architecture/adr/0002-ocr-engine.md). Bundling Tesseract would add ~15 MB per language pack against a
20 MB total install budget ([NFR-P7](../product/requirements-nfr.md#performance)).
**Cost.** Languages Windows doesn't cover are simply unavailable, and accuracy on stylized or game text is whatever
Microsoft gives us. This is our most likely source of user complaints.
**Remediation.** The `OcrEngine` trait already exists for this ([NFR-M1](../product/requirements-nfr.md#maintainability));
add a Tesseract implementation in M8 as an opt-in download rather than a bundled asset.
**Status.** Accepted for v1.

---

### TD-02 — Keyless default provider is unreliable

**What.** The zero-config path depends on a public LibreTranslate instance.
**Why it exists.** [NFR-U1](../product/requirements-nfr.md#usability) demands install → hotkey → translation with no
key and no account, and we have [zero server budget](../product/requirements-nfr.md#constraints).
**Cost.** Public instances rate-limit hard and disappear without notice. A first-run user could hit a failure on
their very first capture — the worst possible moment.
**Remediation.** Open question [PRD Q1](../product/prd.md#open-questions). Options: pick a provider during first run,
ship a curated fallback list, or accept the failure with a very good error message. Decide before M3 ships.
**Status.** Open — decision needed by M3.

---

### TD-03 — Unsigned binaries trip SmartScreen and AV heuristics

**What.** Releases will be unsigned for v1.
**Why it exists.** A code-signing certificate costs real money annually, and this project has no revenue by design.
**Cost.** A tray app that registers a global keyboard hook and captures the screen looks exactly like a keylogger to
a heuristic scanner. Expect SmartScreen warnings, occasional AV quarantine, and users who bounce at the warning —
directly threatening [S5](../product/prd.md#success-metrics) (external adoption).
**Remediation.** Document it honestly in the README with the exact steps to proceed. Revisit signing in M8 if the
project finds users ([PRD Q2](../product/prd.md#open-questions)).
**Status.** Accepted for v1, revisit at M8.

---

### TD-04 — Overlay webview is created per capture

**What.** The overlay window is created on hotkey press and destroyed on close, rather than kept warm.
**Why it exists.** [NFR-P4](../product/requirements-nfr.md#performance) caps idle RAM at 30 MB. A resident WebView2
process would blow that budget for a tool that's idle 99% of the time.
**Cost.** Roughly 100–200 ms of window-creation latency on the first capture, eating into the
[NFR-P1](../product/requirements-nfr.md#performance) 250 ms budget.
**Remediation.** If measurement in M6 shows we miss the budget, keep the overlay created-but-hidden for a few
minutes after use and tear it down on an idle timer — a middle path that costs idle RAM only right after use.
Measure before optimizing.
**Status.** Accepted; verify in M6.

---

### TD-05 — Single detected language per capture

**What.** A mixed-language region is treated as one source language.
**Why it exists.** Per-line language detection multiplies provider calls and complicates the batching that
[FR-42](../product/requirements-functional.md#translation) requires.
**Cost.** A region containing two languages translates one of them badly.
**Remediation.** Per-block detection with grouped batches, if it turns out to matter in real use.
**Status.** Accepted for v1; known limitation, documented in [feature-specs.md](../product/feature-specs.md).

---

### TD-06 — Batched translation depends on the provider preserving segments

**What.** Per-line boxes for the in-place overlay are reconstructed from one batched provider response.
**Why it exists.** One request per capture ([FR-42](../product/requirements-functional.md#translation)) is required
for the latency budget; per-line requests would be 10–30× the round trips.
**Cost.** A provider that merges, splits, or reorders segments breaks the line alignment. We fall back to panel-only
display for that capture ([F7](../product/feature-specs.md)), which silently degrades the headline feature.
**Remediation.** Per-provider segment-integrity tests with a fake provider that misbehaves; consider a
delimiter-free structured request (JSON array) for providers that support it.
**Status.** Accepted with a fallback; test coverage required in M6.

---

### TD-07 — Fullscreen exclusive apps are best-effort

**What.** Capture and overlay over exclusive-fullscreen games or DRM-protected players may fail.
**Why it exists.** A Windows platform limitation, not a design choice.
**Cost.** Gamers are a target persona ([PRD](../product/prd.md#target-users)) and this is exactly where they'd use it.
**Remediation.** Detect the failure and tell the user to switch to borderless windowed mode, rather than showing a
black rectangle. Investigate DXGI Desktop Duplication behaviour per case if it becomes a common complaint.
**Status.** Accepted for v1; needs a good error message in M6.

---

### TD-08 — No release automation beyond building

**What.** M7 produces artifacts via GitHub Actions, but versioning, tagging, and release notes are manual.
**Why it exists.** Right-sized for a solo maintainer with no release cadence yet
([NFR-C5](../product/requirements-nfr.md#constraints)).
**Cost.** Manual steps drift and get skipped; the CHANGELOG rots first.
**Remediation.** Automate once there's a second release. Until then [release-plan.md](release-plan.md) is the checklist.
**Status.** Accepted deliberately — do not automate before it hurts.
