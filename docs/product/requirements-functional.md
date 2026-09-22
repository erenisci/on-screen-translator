---
title: Functional Requirements
discipline: product
status: active
updated: 2026-09-10
---

# Functional Requirements

> **Purpose.** Numbered, testable statements of what the app must do. Every FR maps to an acceptance check.
> **Related.** [prd.md](prd.md) · [requirements-nfr.md](requirements-nfr.md) · [feature-specs.md](feature-specs.md) · [../quality/qa-checklist.md](../quality/qa-checklist.md)

## Overview

Priority: **P0** = v1 cannot ship without it · **P1** = v1 should have it · **P2** = deferred.
IDs are stable; never renumber. Behaviour detail lives in [feature-specs.md](feature-specs.md).

## Functional Requirements

### Lifecycle & tray

| ID    | Requirement                                                                                          | Pri | Acceptance                                                                                     |
| ----- | ---------------------------------------------------------------------------------------------------- | --- | ---------------------------------------------------------------------------------------------- |
| FR-01 | The app runs with no taskbar entry, no console window, and exactly one tray icon.                     | P0  | After launch, taskbar shows nothing; tray shows one icon; `Alt+Tab` does not list the app.       |
| FR-02 | The tray menu offers Capture, Settings, and Quit.                                                     | P0  | Each item performs its action; Quit terminates the process fully (no orphan).                    |
| FR-03 | Left-clicking the tray icon triggers a capture; right-clicking opens the menu.                        | P1  | Both verified manually.                                                                          |
| FR-04 | Only one instance may run. Launching a second focuses/notifies the first and exits.                   | P0  | Second launch leaves exactly one process and one tray icon.                                      |
| FR-05 | The app can register itself to start with Windows, minimized to tray. Off by default; asked on first run. | P0  | Toggling in Settings adds/removes the run entry; after reboot the app is in the tray only.   |

### Hotkey

| ID    | Requirement                                                                                            | Pri | Acceptance                                                                              |
| ----- | ------------------------------------------------------------------------------------------------------ | --- | --------------------------------------------------------------------------------------- |
| FR-10 | A global hotkey (default `Ctrl+Shift+T`) starts a capture from any focused application.                  | P0  | Works with Explorer, a browser, and a fullscreen game focused.                            |
| FR-11 | The hotkey is rebindable in Settings and persists across restarts.                                      | P0  | Rebind, restart, new binding still works; old binding does not.                           |
| FR-12 | If the requested hotkey is already claimed by another process, registration fails **visibly**, keeping the previous binding. | P0 | Settings shows an inline error; the app does not silently end up with no hotkey. |

### Capture overlay

| ID    | Requirement                                                                                                          | Pri | Acceptance                                                                                     |
| ----- | -------------------------------------------------------------------------------------------------------------------- | --- | ------------------------------------------------------------------------------------------------ |
| FR-20 | On trigger, the entire virtual desktop is captured once to a still frame; the overlay displays that frozen frame.      | P0  | Content moving on screen (a playing video) is frozen the instant the overlay appears.             |
| FR-21 | The overlay covers **all** monitors, positioned correctly under mixed per-monitor DPI.                                 | P0  | On a 150%+100% dual setup, the frozen image aligns pixel-exactly with what was on each screen.    |
| FR-22 | The un-selected area is dimmed; the selection rectangle shows the live source content at full brightness.              | P0  | Visual check.                                                                                      |
| FR-23 | A readout shows the live selection size in pixels while dragging.                                                      | P1  | Readout matches the region actually sent to OCR.                                                   |
| FR-24 | `Esc` (or right-click) cancels, closes the overlay, and restores focus to the previously active window.                | P0  | After cancel, typing goes to the app that had focus before the hotkey.                             |
| FR-25 | A selection smaller than 8×8 px is treated as a mis-click and ignored rather than sent to OCR.                          | P1  | A single click does not fire the pipeline.                                                         |
| FR-26 | After a result is shown, the user can drag a **new** selection without re-pressing the hotkey.                          | P0  | Second and third selections work within one overlay session.                                       |

### OCR

| ID    | Requirement                                                                                                        | Pri | Acceptance                                                                              |
| ----- | ------------------------------------------------------------------------------------------------------------------ | --- | ----------------------------------------------------------------------------------------- |
| FR-30 | The selected region is cropped from the already-captured frame — the screen is **not** re-captured.                  | P0  | No second capture call in a trace; result reflects the frozen frame, not live screen.      |
| FR-31 | Before OCR, the crop is pre-processed: upscale small regions, convert to grayscale, normalize contrast.              | P0  | Small (< 20 px tall) text yields materially better recognition with pre-processing on.     |
| FR-32 | OCR returns, per recognized line: text, a bounding box in frame coordinates, and a confidence value.                 | P0  | The overlay can position translated lines from the returned boxes.                         |
| FR-33 | The source language is detected automatically; the user may override it in Settings.                                | P0  | Detected language is shown in the result panel.                                            |
| FR-34 | If the required OCR language pack is not installed, the app says so and links to the Windows setting — it does not fail silently. | P0 | Actionable message, not a generic error.                                        |
| FR-35 | If no text is found, the user is told plainly and stays in the overlay to re-select.                                | P0  | Empty selection on a blank area shows "no text found", overlay stays open.                 |

### Translation

| ID    | Requirement                                                                                                       | Pri | Acceptance                                                                                |
| ----- | ----------------------------------------------------------------------------------------------------------------- | --- | ------------------------------------------------------------------------------------------- |
| FR-40 | Extracted text is translated into the configured target language, defaulting to the Windows display language.      | P0  | On a Turkish Windows with no configuration, output is Turkish.                              |
| FR-41 | The translation provider is selectable: LibreTranslate (keyless default), DeepL, Google, or an LLM endpoint.       | P0  | Each provider translates the same input successfully with a valid key.                      |
| FR-42 | All lines of one capture are translated in a **single** provider request, not one request per line.                | P0  | One outbound request per capture, verified in logs.                                         |
| FR-43 | If source and target language are the same, translation is skipped and the source is shown as-is.                  | P1  | No provider request is made.                                                                |
| FR-44 | Provider failures (network, rate limit, bad key) surface a specific, actionable message and leave the OCR text available and copyable. | P0 | Pulling the network mid-capture still shows the extracted source text.       |
| FR-45 | Only extracted **text** is transmitted. The captured image never leaves the machine.                              | P0  | Traffic inspection during a capture shows no image payload. See [../operations/security.md](../operations/security.md). |

### Results

| ID    | Requirement                                                                                                         | Pri | Acceptance                                                                          |
| ----- | ------------------------------------------------------------------------------------------------------------------- | --- | ------------------------------------------------------------------------------------- |
| FR-50 | Translated lines are drawn over their source bounding boxes inside the overlay, without closing it.                   | P0  | Translation appears in place, aligned to the original text blocks.                    |
| FR-51 | Overlaid text auto-fits its box: shrink to a floor, then wrap, then clip with the full text reachable on hover.        | P0  | A translation 2× longer than its source stays readable.                               |
| FR-52 | The result panel shows source text and translation with a copy button for each.                                       | P0  | Copy places exactly the shown text on the clipboard.                                  |
| FR-53 | The result panel is always-on-top, frameless, and dismissible with `Esc`.                                             | P0  | Stays above other windows; `Esc` closes it.                                           |
| FR-54 | The panel offers re-translate (after changing target language) without a new capture.                                 | P1  | Re-translate reuses the cached OCR text; no re-capture, no re-OCR.                     |
| FR-55 | Result display mode is configurable: overlay only, panel only, or both.                                               | P1  | Each mode behaves as named.                                                           |

### Settings & persistence

| ID    | Requirement                                                                                                | Pri | Acceptance                                                                        |
| ----- | ------------------------------------------------------------------------------------------------------------ | --- | ----------------------------------------------------------------------------------- |
| FR-60 | Settings covers: target language, source override, hotkey, provider + key, OCR languages, autostart, theme, display mode. | P0 | Every field persists and takes effect without restart (hotkey rebinds live). |
| FR-61 | Settings persist as a single JSON file under `%APPDATA%`; a corrupt file falls back to defaults with a warning rather than crashing. | P0 | Hand-corrupt the file → app starts on defaults and says so. |
| FR-62 | API keys are stored in Windows Credential Manager, never in the JSON file and never in logs.                  | P0  | Grep the settings file and logs for the key → no match. See [../operations/security.md](../operations/security.md). |
| FR-63 | A "test connection" action validates the provider configuration and reports success or the specific failure.  | P1  | Wrong key reports "invalid credentials", not a generic error.                       |
| FR-64 | The app ships with zero telemetry, analytics, or update pings.                                                | P0  | No outbound request occurs except a user-initiated translation or connection test.  |

## Deferred (P2)

Tracked here so they are not re-litigated; see [roadmap-vision.md](roadmap-vision.md).

| ID     | Requirement                                                             |
| ------ | ------------------------------------------------------------------------- |
| FR-90  | Live mode — pin a region and re-translate as its content changes.         |
| FR-91  | Translation history with search and pinning.                              |
| FR-92  | Fully offline translation via a bundled local model.                      |
| FR-93  | Save the annotated capture as PNG / copy image to clipboard.              |
| FR-94  | Text-to-speech for source or translation.                                 |
| FR-95  | Glossary / do-not-translate list.                                         |
| FR-96  | Auto-update.                                                              |
