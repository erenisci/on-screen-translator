---
title: "ADR 0001: Initial architecture — Tauri v2 shell with a Rust core"
discipline: code
status: Accepted
date: 2026-09-10
---

# ADR 0001: Initial architecture — Tauri v2 shell with a Rust core

## Status

Accepted — 2026-09-10

## Context

on-screen-translator is a Windows tray utility that must sit idle for hours and then, on a keypress, freeze the
screen, OCR a selected region, translate it, and draw the result back over the original text. The constraints that
actually decide the architecture:

- **Idle cost dominates.** The app is idle 99% of the time. [NFR-P4](../../product/requirements-nfr.md#performance)
  caps idle RAM at 30 MB and [NFR-P5](../../product/requirements-nfr.md#performance) caps idle CPU at ~0%.
- **Latency is the product.** [NFR-P2](../../product/requirements-nfr.md#performance) allows under 1s p50 from
  mouse-release to visible translation.
- **It needs deep OS access.** Global hotkey, virtual-desktop capture, per-monitor DPI geometry, transparent
  always-on-top windows, tray, autostart, Credential Manager.
- **The UI is genuinely UI.** A selection overlay with a dim scrim, per-line positioned translated text with a
  fitting ladder, and a settings window — layout and text-fitting work that is painful in an immediate-mode or
  native-widget toolkit and trivial in HTML/CSS.
- **Solo maintainer, ≤ 20 MB install** ([NFR-P7](../../product/requirements-nfr.md#performance)), MIT, zero server cost.

The brief expressed a preference for React + TypeScript + Tailwind on the UI side, with Python, Go, or Tauri for the
core. Options considered:

**A. Electron + React.** Familiar and fast to build. But a resident Chromium instance costs 150–250 MB idle — it
misses the idle budget by ~8×, for an app whose main job is to not be noticed. Installed size is 100 MB+. Rejected
on the numbers alone.

**B. Python (PyQt/PySide or a tray lib) + a webview.** Fastest prototype, best OCR ecosystem access. But shipping a
Python desktop app on Windows means PyInstaller bundles of 40–80 MB, a slow cold start, and a packaging story that
fights us at every release. Native Windows API access (per-monitor DPI, Credential Manager) is workable through
`pywin32` but brittle. Rejected on distribution and startup cost.

**C. Go + a webview binding.** Small binaries and good concurrency, but Windows API coverage is via cgo or
hand-rolled `syscall` wrappers, and there's no equivalent of the `windows` crate's generated, typed WinRT bindings.
`Windows.Media.Ocr` is a WinRT API — reaching it cleanly from Go is the hard path.

**D. Fully native (C++/WinRT, Win32, or WPF/WinUI).** Best possible resource profile and OS access. But the overlay's
text-fitting and layout work would be hand-rolled, iteration would be slow, and it discards the brief's stated
frontend preference. The performance headroom it buys isn't needed — the budgets are already met by option E.

**E. Tauri v2 — Rust core + system WebView2 for UI.** The webview is the OS-provided WebView2, not a bundled
runtime, and Tauri creates windows on demand rather than keeping a resident renderer. Rust gets first-class WinRT
access through the official `windows` crate, which is exactly what `Windows.Media.Ocr` needs
([ADR-0002](0002-ocr-engine.md)). Tray, global shortcut, and autostart are first-party plugins.

## Decision

**Tauri v2, with a Rust core that owns everything expensive and a React + TypeScript + Tailwind frontend that owns
only presentation.**

The split is strict, and it is the point of this ADR:

**The Rust core owns** the global hotkey, screen capture, monitor topology and DPI geometry, image pre-processing,
OCR, translation calls, settings persistence, credential storage, and autostart. Every network request originates
here — which is what makes the privacy claim in
[NFR-S1/S5](../../product/requirements-nfr.md#security) auditable at a single boundary.

**The frontend owns** the selection interaction, the dim scrim, positioning translated lines at their bounding
boxes, the fitting ladder, the result panel, and the settings form. It never touches the network, the filesystem,
or the screen.

**They communicate** over Tauri IPC with a typed command surface, documented as a real contract in
[../api.md](../api.md). The frozen frame crosses to the frontend via a custom protocol handler, not base64 in an IPC
payload — a 4K multi-monitor PNG through JSON would cost more than the whole latency budget.

**Three windows, all created on demand and destroyed after use:** `overlay`, `panel`, `settings`. No window exists
while idle; that is how the 30 MB idle budget is met.

## Consequences

**Easier**

- The idle budget is met structurally: nothing is resident but a small Rust process and a tray icon.
- WinRT APIs are reachable with typed, generated bindings — no FFI hand-rolling for OCR, DPI, or Credential Manager.
- The overlay's hardest UI problem (fitting translated text into source-shaped boxes) is CSS, where it's tractable.
- One auditable network boundary in Rust makes "the image never leaves the device" a property of the architecture
  rather than a promise.
- Install size lands around 5–15 MB instead of 100 MB+.
- Hot-reloaded UI development, compiled-language guarantees in the core.

**Harder**

- **Two languages, one bug surface.** Every feature crossing the IPC boundary needs types on both sides; drift there
  is a real and recurring cost. Mitigated by treating [../api.md](../api.md) as the contract and generating/checking
  types rather than hand-syncing them.
- **WebView2 is a dependency.** Present on Windows 11 and evergreen on most Windows 10 machines, but not
  guaranteed — the installer must handle a missing runtime, and the clean-machine test in
  [../../project/release-plan.md](../../project/release-plan.md) exists specifically to catch this.
- **Window-creation latency** of roughly 100–200 ms on first capture, against a 250 ms budget
  ([NFR-P1](../../product/requirements-nfr.md#performance)). Accepted and tracked as
  [TD-04](../../project/tech-debt.md); measured in M6 before any optimization.
- **Rust learning curve** for anyone contributing to the core — a real barrier to the external-contribution goal
  ([S5](../../product/prd.md#success-metrics)).
- **Tauri v2 moves fast.** Plugin APIs may shift between minor versions; pin versions and read changelogs on upgrade.

**At scale**

The architecture scales along the axes this product actually grows on. More OCR engines and more translation
providers are additive implementations behind existing traits ([ADR-0002](0002-ocr-engine.md),
[ADR-0003](0003-translation-provider-abstraction.md)). More UI surfaces are more windows. The one axis it does *not*
absorb for free is **live mode** ([Later](../../product/roadmap-vision.md)), which breaks the "nothing runs while
idle" invariant this whole design rests on — that will need its own ADR and its own performance budget, not an
extension of this one.

A cross-platform port is possible but not free: platform-specific code is confined to named modules
([NFR-M2](../../product/requirements-nfr.md#maintainability)), so a port means writing new implementations, not
restructuring — but capture, OCR, DPI, credentials, and autostart are all Windows-specific today.
