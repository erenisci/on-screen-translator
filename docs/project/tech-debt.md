---
title: Technical Debt Log
discipline: project
status: active
updated: 2026-11-16
---

# Technical Debt Log

> **Purpose.** Known shortcuts, accepted risks, and deferred work — recorded so they're decisions, not surprises.
> **Related.** [../product/requirements-nfr.md](../product/requirements-nfr.md) · [../architecture/adr/README.md](../architecture/adr/README.md) · [../../SCRATCH.md](../../SCRATCH.md)

## How this list works

Items enter here when we knowingly accept a cost. Each has: what it is, why it exists, what it costs us, and how
we'd remediate. `/acta:track` drains [SCRATCH.md](../../SCRATCH.md) into this file when a scratch note turns out to
be structural rather than a quick fix.

TD-01 … TD-08 are **accepted risks and planned shortcuts** identified during design, recorded before they bite.
TD-09 onward are **implementation debt** — things building the app proved to be different from the plan.

## Debt items

### TD-01 — Single OCR engine (`Windows.Media.Ocr`)

**What.** v1 ships one OCR engine, the Windows built-in one.
**Why it exists.** It adds zero megabytes, needs no bundled model, and is fast — see
[ADR-0002](../architecture/adr/0002-ocr-engine.md). Bundling Tesseract would add ~15 MB per language pack against a
20 MB total install budget ([NFR-P7](../product/requirements-nfr.md#performance)).
**Cost.** Languages Windows doesn't cover are simply unavailable, and accuracy on stylized or game text is whatever
Microsoft gives us. This is our most likely source of user complaints. Measured in M2: **8 px text is a hard floor**
— nothing is recognized at that size even upscaled 3×, which rules out the app for dense UI on an unscaled 4K
panel. Worth stating plainly in the README rather than letting users discover it.
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

---

### TD-09 — The OCR engine reports no confidence

**What.** `Windows.Media.Ocr` exposes no confidence value, per line or per word. Every `OcrLine.confidence` from
the v1 engine is therefore `null`, and the "low confidence" marker in the overlay and the panel can never fire.
**Why it exists.** Discovered while implementing M2 ([ADR-0002](../architecture/adr/0002-ocr-engine.md) assumed the
field would be available). The alternative was to synthesize a number — from glyph geometry, or a flat 1.0 — which
would make the marker lie in both directions, so the field is `Option`/nullable instead
([api.md](../architecture/api.md)).
**Cost.** [feature-specs.md](../product/feature-specs.md) F5 says low-confidence lines are "kept but marked"; with
this engine nothing is ever marked, so a confidently wrong recognition reads as authoritative.
**Remediation.** A second engine (TD-01) would supply real values. A weaker proxy — flagging lines whose box is
much shorter than the block median, or whose text is mostly non-word characters — is possible without one, but is a
heuristic dressed as a measurement and needs real misses to calibrate against.
**Status.** Accepted for v1. The contract carries `null` honestly; the UI treats unknown as unknown, never as low.

---

### TD-10 — "Source language detection" is recognizer selection, not detection

**What.** [FR-33](../product/requirements-functional.md#ocr) says the source language is detected automatically.
What the app actually does is recognize with the user's Windows profile languages and report *which recognizer ran*
as the source language.
**Why it exists.** `Windows.Media.Ocr` has no detection mode: an engine is created *for* a language and reads with
that language's model. Detection would mean recognizing with several engines and scoring the results — several
times the latency budget ([NFR-P3](../product/requirements-nfr.md#performance)) — or adding a text-level language
detector after OCR, which is a second dependency.
**Cost.** On a machine whose profile language differs from the text on screen, the reported source language is
wrong, and a provider told the wrong source language translates worse. The user can override it in Settings, which
is the v1 answer.
**Remediation.** Either a small text-level detector run on `full_text` after recognition (cheap, no extra OCR
pass), or scoring two recognizers when the result looks poor. Decide with real misses, not in the abstract.
**Status.** Accepted for v1; [FR-33](../product/requirements-functional.md#ocr) overstates what ships and should be
reworded at the next requirements pass. Related to [TD-05](#td-05--single-detected-language-per-capture).

---

### TD-11 — The pre-processing ladder's accuracy benefit is unproven

**What.** [FR-31](../product/requirements-functional.md#ocr) claims small text "yields materially better
recognition with pre-processing on". The M2 fixtures show no such gain: at 11 px, 3/3 words were recognized both
with and without the ladder; at 8 px, 0/3 either way. The ladder costs 13–22 ms per capture (debug build) for no
measured return.
**Why it exists.** The ladder was designed from first principles in [ADR-0002](../architecture/adr/0002-ocr-engine.md)
and built before it could be measured. The fixtures that measure it are *rendered*, and rendered ClearType text is
cleaner than a real captured region ([ADR-0005](../architecture/adr/0005-ocr-test-fixtures-are-rendered.md)) — so
this may be a fixture problem rather than a ladder problem. Both readings are still open.
**Cost.** Either the app spends time on nothing every capture, or the test suite is blind to the case the ladder
exists for. Both are bad, and today we cannot tell which one we have.
**Remediation.** Build a harder fixture before M6 — a downscaled screenshot, resampling blur, subpixel fringing —
and then either demonstrate the gain and keep the ladder, or narrow it to the cases that measurably need it. The
existing test asserts only "never worse", which is all the current evidence supports. One untried knob belongs in
the same measurement: the threshold step preserves polarity, so a dark-theme crop stays light-on-dark. Inverting
when the background is dark is the obvious thing to try if dark-mode accuracy turns out to be worse — the
monospace-on-dark fixture passes, so there is no evidence it is, yet.
**Status.** Open — the measurement to make before the M6 performance pass, not a shortcut taken deliberately.

---

### TD-12 — The virtual-desktop grab costs 183 ms of a 250 ms budget

**What.** [NFR-P1](../product/requirements-nfr.md#performance) allows 250 ms for the whole
hotkey→overlay-visible path. The GDI `BitBlt` alone measured **183 ms** on a single 1920×1080 display (debug
build), before window creation and the frame transfer have had their turn.
**Why it exists.** `BitBlt` over the virtual screen is what makes one buffer span every monitor in one call with a
negative origin handled naturally ([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)). The cost
scales with pixel count, so a dual-4K desktop (~8× the pixels) misses the budget outright.
**Cost.** The one latency the user feels directly: the gap between pressing the hotkey and the screen freezing.
Miss it and the tool feels laggy at exactly the moment it is supposed to feel instant.
**Remediation.** In order of cost: measure a release build (`opt-level="s"` + LTO) on a large desktop — that is
the missing number, and the rest may be unnecessary; fold the alpha-forcing pass into the BMP write (it is an O(n)
pass over the whole frame done separately today); capture only the monitor under the cursor and extend lazily; or
move to DXGI Desktop Duplication, which would need its own ADR since it changes the primitive ADR-0004 rests on.
**Status.** Open — **measure before optimizing**, on a big desktop, before M6.

---

### TD-13 — The IPC contract is hand-mirrored in two languages

**What.** `src/lib/types.ts` is written by hand against `docs/architecture/api.md` and the Rust types, instead of
being generated from them. [api.md](../architecture/api.md) rule 3 says types are defined once in Rust and
generated — so the code currently contradicts its own contract.
**Why it exists.** Generation needed the Rust types to exist first; they now do. M2 grew the contract again
(`recognize_region`, `OcrResult`, nullable `confidence`) and every change was made twice, by hand.
**Cost.** The two sides can diverge silently, and a divergence surfaces as a runtime `undefined` in a webview
rather than a compile error — exactly the cost [ADR-0001](../architecture/adr/0001-initial-architecture.md) named
as the price of the two-language split.
**Remediation.** Wire `ts-rs` to emit `types.gen.ts`, delete the hand-authored file, and add the CI drift check
from [ci-cd.md](../operations/ci-cd.md) so a mismatch fails the build.
**Status.** Open — do it before M3 adds the translation half of the contract, not after.
