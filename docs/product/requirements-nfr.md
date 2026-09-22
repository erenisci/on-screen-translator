---
title: Non-Functional Requirements
discipline: product
status: active
updated: 2026-09-10
---

# Non-Functional Requirements

> **Purpose.** The qualities the app must have to feel right, not just work. These are budgets, not aspirations.
> **Related.** [prd.md](prd.md) · [requirements-functional.md](requirements-functional.md) · [../architecture/system-design.md](../architecture/system-design.md) · [../operations/security.md](../operations/security.md)

## Performance

Latency is the product. A translator that takes three seconds loses to alt-tabbing.

| ID     | Budget                                                        | Target                | Measured how                                       |
| ------ | ------------------------------------------------------------- | --------------------- | -------------------------------------------------- |
| NFR-P1 | Hotkey press → overlay visible with frozen frame               | < 250 ms p95          | Timestamp from hotkey handler to overlay `shown`   |
| NFR-P2 | Mouse-release → translation visible (one paragraph, ~50 words) | < 1 s p50, < 2 s p95  | End-to-end pipeline trace                          |
| NFR-P3 | Crop + pre-process + OCR (region ≤ 800×400)                    | < 200 ms p95          | Stage timer in the pipeline                        |
| NFR-P4 | Idle RAM, tray only, no capture in the last 5 min              | ≤ 30 MB               | Task Manager, private working set                  |
| NFR-P5 | Idle CPU                                                       | ~0% (no polling loop) | Sampled over 10 min idle                           |
| NFR-P6 | Peak RAM during a 4K multi-monitor capture                     | ≤ 400 MB, released after overlay close | Sampled at the peak       |
| NFR-P7 | Installed size                                                 | ≤ 20 MB               | MSI + unpacked footprint                           |
| NFR-P8 | Cold start (launch → tray icon present)                        | < 1 s                 | Manual, on a mid-range machine                     |

**Design consequences.** Capture happens once per session and is cropped in memory (FR-30). No timers, watchers,
or polling exist while idle. Webview windows are created on demand. Provider requests are batched to one per
capture (FR-42). See [../architecture/system-design.md](../architecture/system-design.md) for how each budget is met.

## Reliability

| ID     | Requirement                                                                                                       |
| ------ | ----------------------------------------------------------------------------------------------------------------- |
| NFR-R1 | A failure in any pipeline stage never leaves an invisible full-screen overlay swallowing input. The overlay closes or shows an error; `Esc` always works. |
| NFR-R2 | A provider or network failure degrades gracefully: the OCR source text stays visible and copyable (FR-44).          |
| NFR-R3 | A corrupt settings file falls back to defaults with a warning rather than crashing (FR-61).                          |
| NFR-R4 | The app survives display topology changes (monitor plugged, resolution or scaling changed) — geometry is re-read at each capture, never cached across captures. |
| NFR-R5 | Quit always terminates fully; no orphan process, no leftover tray icon, no stuck global hook.                        |
| NFR-R6 | An unhandled panic in the Rust core is logged with context and shown to the user, not silently swallowed.            |

## Security

Detail in [../operations/security.md](../operations/security.md); the binding requirements:

| ID     | Requirement                                                                                                     |
| ------ | ----------------------------------------------------------------------------------------------------------------- |
| NFR-S1 | The captured image never leaves the device. Only extracted text is transmitted (FR-45).                            |
| NFR-S2 | Text is transmitted only to the provider the user explicitly configured, over TLS, and only on a user-initiated action. |
| NFR-S3 | API keys live in Windows Credential Manager — never in the settings file, never in logs, never in a crash report (FR-62). |
| NFR-S4 | Zero telemetry, analytics, crash reporting, or update pings (FR-64). The app makes no outbound request the user didn't ask for. |
| NFR-S5 | The webview runs under a restrictive CSP with no remote origins; the frontend never talks to the network directly — all outbound traffic goes through the Rust core. |
| NFR-S6 | Captured text and images are never written to disk; they live in memory for the overlay session and are dropped on close. |
| NFR-S7 | No elevated privileges are required to install or run.                                                             |

## Usability

| ID     | Requirement                                                                                                    |
| ------ | ---------------------------------------------------------------------------------------------------------------- |
| NFR-U1 | Zero-configuration first run: install → hotkey → translation, with no key and no account.                        |
| NFR-U2 | The whole product is reachable from one tray icon; there is no main window to find.                              |
| NFR-U3 | Settings fits one small window with no tabs-within-tabs and no scroll on a 900px-tall display.                    |
| NFR-U4 | The target language defaults to the Windows display language — the app is correct for the user before they touch it (FR-40). |
| NFR-U5 | Every error message names the cause and the next action ("DeepL rejected the key — check it in Settings"), never a raw code. |
| NFR-U6 | The overlay is fully keyboard-escapable at all times; the user is never trapped.                                 |
| NFR-U7 | Overlay and panel follow the system light/dark theme by default.                                                 |
| NFR-U8 | Overlaid text respects a legibility floor: never smaller than 10 px effective, always on a contrast-guaranteed backing (FR-51). |

## Maintainability

| ID     | Requirement                                                                                                       |
| ------ | ------------------------------------------------------------------------------------------------------------------- |
| NFR-M1 | OCR and translation each sit behind a trait with a single implementation swap point, so a second engine/provider is an additive change. See [../architecture/overview.md](../architecture/overview.md). |
| NFR-M2 | Windows-specific code is confined to named modules so a future port is a matter of adding implementations, not restructuring. |
| NFR-M3 | Pipeline logic is testable without a screen: capture, OCR, and translation are injectable, so the core is exercised with fixture images and a fake provider. See [../quality/testing-strategy.md](../quality/testing-strategy.md). |
| NFR-M4 | A stranger can go from `git clone` to a running build with the commands in [../onboarding.md](../onboarding.md), on a clean machine. |
| NFR-M5 | Every significant technical decision has an ADR under [../architecture/adr/](../architecture/adr/README.md).       |

## Constraints

| ID     | Constraint                                                                                                     |
| ------ | ---------------------------------------------------------------------------------------------------------------- |
| NFR-C1 | **Windows 10 1809+ / Windows 11 only** for v1. Platform code is isolated, but no cross-platform work is done now. |
| NFR-C2 | **MIT licensed, open source.** No ads, no paywall, no account, no telemetry.                                     |
| NFR-C3 | **Zero server cost.** No backend of our own; translation goes machine → provider directly.                       |
| NFR-C4 | **No key ships with the app.** Anything requiring a key is the user's own key.                                   |
| NFR-C5 | **Solo developer, no deadline.** Right-size everything: no infrastructure a single maintainer can't run for free. |
| NFR-C6 | x64 only for v1. ARM64 is a later build target, not a v1 requirement.                                            |
