---
title: QA Checklist
discipline: quality
status: active
updated: 2026-09-10
---

# QA Checklist

> **Purpose.** The manual pass that covers what automated tests deliberately don't — a real screen, real windows,
> and a real mouse. Run per milestone; run in full before a release.
> **Related.** [testing-strategy.md](testing-strategy.md) · [../project/release-plan.md](../project/release-plan.md) · [../product/feature-specs.md](../product/feature-specs.md)

## Environments

A release pass needs **all three**. A milestone pass needs at least the first two.

| # | Environment                                                        | Catches                                            |
| - | ------------------------------------------------------------------ | ---------------------------------------------------- |
| 1 | Dev machine, single monitor                                         | Ordinary regressions                                |
| 2 | **Multi-monitor, mixed DPI** (e.g. 150% laptop + 100% external)     | The project's worst bug class — nothing else finds it |
| 3 | **Clean Windows VM**: no dev tools, no WebView2 pre-installed, **a different display language** | Install, first run, and the default-language path |

Environment 3 is the one that catches "works on the author's machine" — including the target language defaulting
correctly, which is invisible on a machine already set to the author's language.

## Install & first run

- [ ] MSI installs without admin rights
- [ ] Installs cleanly on a machine **without** WebView2 present
- [ ] Portable exe runs from a folder with no install
- [ ] First launch: tray icon appears, **no** taskbar entry, **no** console window, not in `Alt+Tab`
- [ ] First-run autostart prompt appears once, defaults to off, and never reappears after answering
- [ ] Target language defaults to the **Windows display language** (verify on environment 3)
- [ ] A translation works with **zero configuration** — no key, no account
- [ ] Uninstall removes the app, the tray icon, and the autostart entry

## Tray & lifecycle

- [ ] Left-click triggers a capture; right-click opens the menu
- [ ] Menu items all work: Capture, Settings, Quit
- [ ] Tooltip shows the current hotkey
- [ ] Second launch does not create a second icon or a second process
- [ ] Quit terminates fully — check Task Manager for orphans; tray icon disappears
- [ ] Quit **during** an in-flight capture exits cleanly, no hang
- [ ] Restart Explorer → tray icon comes back

## Hotkey

- [ ] Default `Ctrl+Shift+T` triggers from: Explorer, a browser, a fullscreen video, a game
- [ ] Rebinding takes effect **immediately**, without restart
- [ ] Rebinding survives a restart; the old binding no longer works
- [ ] A conflicting binding shows an inline error and **keeps the previous binding** (never leaves the app unbound)
- [ ] Holding the keys does not queue repeat captures
- [ ] Pressing the hotkey while an overlay is open does nothing (no nested capture)

## Capture overlay

- [ ] Screen visibly **freezes** — start a video, trigger, confirm the frame is still
- [ ] Overlay covers **every** monitor
- [ ] **Environment 2:** the frozen image aligns pixel-exactly on each screen — no offset, no scaling
- [ ] **Environment 2:** a box drawn on the secondary monitor crops exactly what was inside it
- [ ] **Environment 2:** a selection crossing the monitor boundary works
- [ ] A monitor left of the primary (negative origin) works
- [ ] Dim scrim covers everything outside the selection; inside is at full brightness
- [ ] Live size readout matches the region actually processed, and flips near screen edges
- [ ] `Esc` cancels and returns focus to the previously active window
- [ ] Right-click cancels
- [ ] A single click (under 8×8 px) is ignored, not sent to OCR
- [ ] After a result appears, dragging again starts a new selection — no hotkey needed
- [ ] Unplug/replug a monitor between two captures; the second capture is correct

## OCR

- [ ] Ordinary paragraph text recognizes accurately
- [ ] Small UI text (11–13 px) recognizes acceptably
- [ ] Low-contrast text recognizes acceptably
- [ ] Dark-mode (light-on-dark) text recognizes acceptably
- [ ] Monospace / code recognizes acceptably
- [ ] Selecting a blank area reports "no text found" and **keeps the overlay open**
- [ ] A language whose pack isn't installed gives an actionable message + a link to Windows settings
- [ ] Detected source language is correct and shown in the panel
- [ ] Low-confidence results are visibly flagged, not presented as certain

## Translation

- [ ] Default provider translates with no configuration
- [ ] DeepL works with a valid key
- [ ] Google works with a valid key
- [ ] LLM endpoint works with a valid key
- [ ] Same source and target language → text shown as-is, **no** network request
- [ ] **Exactly one** outbound request per capture (check the log or a network trace)
- [ ] Network disconnected mid-capture → clear message, **OCR source text still visible and copyable**
- [ ] Wrong key → "provider rejected the key", not a generic failure
- [ ] Rate limited → says so and suggests configuring a personal key
- [ ] Very long selection (a full page) still works

## In-place result overlay

- [ ] Translated lines appear positioned over their original text blocks
- [ ] A translation much longer than its source stays readable — shrinks, then wraps
- [ ] Text never renders below the legibility floor
- [ ] Overlaid text has adequate contrast over light **and** dark backgrounds
- [ ] Boxes don't overlap into unreadability
- [ ] Clipped text shows in full on hover
- [ ] Starting a new selection clears the previous results
- [ ] Segment mismatch → falls back to panel-only rather than mis-aligning

## Result panel

- [ ] Appears near the selection and stays fully on screen (test near all four screen edges)
- [ ] Shows detected source → target language
- [ ] Copy source and copy translation each place exactly the right text on the clipboard, line breaks preserved
- [ ] Stays on top of other windows
- [ ] Re-translate to a different language works **without** re-capturing or re-OCR'ing
- [ ] `Esc` closes the panel; closing the panel doesn't close the overlay, and vice versa
- [ ] Long text scrolls; the panel doesn't grow off-screen
- [ ] Display mode setting: overlay-only, panel-only, and both each behave as named

## Settings

- [ ] Window opens small, no scrolling needed at 900 px height
- [ ] Every field persists across a restart
- [ ] Every field takes effect **without** a restart
- [ ] Test connection: success on a good key, specific failure on a bad one
- [ ] Switching provider and back preserves the first provider's key
- [ ] Autostart toggle adds/removes the run entry; verified after a real reboot
- [ ] Moving the exe and restarting repairs the stale autostart path
- [ ] Theme follows system; Light and Dark overrides work
- [ ] Uninstalled OCR language shown inline with a link, **before** a capture fails

## Privacy & security

Every item is a public promise ([../operations/security.md](../operations/security.md)) — verify, don't assume:

- [ ] Network trace during one capture: **one** request, to the configured provider, **no image payload**
- [ ] No outbound request at startup, at idle, or on quit — no telemetry, no update ping
- [ ] Grep the settings JSON for the API key → **no match**
- [ ] Grep the log files for the API key → **no match**
- [ ] Log files contain **no** captured text and no image data
- [ ] No captured image or text written anywhere on disk
- [ ] Runs without elevation

## Performance

Measure, don't eyeball ([../product/requirements-nfr.md](../product/requirements-nfr.md)):

- [ ] Idle RAM ≤ 30 MB after 5 minutes with no capture
- [ ] Idle CPU ~0% over 10 minutes
- [ ] Hotkey → overlay visible < 250 ms
- [ ] Release → translation visible < 1s for a normal paragraph
- [ ] Peak RAM during a 4K multi-monitor capture ≤ 400 MB
- [ ] **Memory returns to idle levels after the overlay closes** (no leak across 20 consecutive captures)
- [ ] Installed size ≤ 20 MB
- [ ] Cold start → tray icon < 1s

## Robustness

- [ ] Corrupt the settings file → app starts on defaults and says so
- [ ] Delete the settings file → app starts on defaults
- [ ] 20 captures in a row without a restart — no leak, no slowdown, no stuck state
- [ ] Lock/unlock the workstation with an overlay open
- [ ] Change display scaling while the app runs, then capture
- [ ] Change resolution while the app runs, then capture
- [ ] RDP session, then back to a local session
- [ ] Fullscreen exclusive game: either it works, or it **fails with a clear message** — never a black rectangle or a hang
- [ ] At no point is the user trapped behind an unresponsive overlay

## Sign-off

Record for each release: date, version, environments used, and any item that failed with its issue link. A failed
item is either fixed or explicitly accepted in [../project/tech-debt.md](../project/tech-debt.md) — never silently
skipped.
