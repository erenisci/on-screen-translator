---
title: User Stories
discipline: product
status: active
updated: 2026-09-10
---

# User Stories

> **Purpose.** The requirements from the user's side, with the acceptance criteria that make each one testable.
> **Related.** [prd.md](prd.md) · [requirements-functional.md](requirements-functional.md) · [feature-specs.md](feature-specs.md)

## Stories

### US-01 — Translate what I'm looking at, right now

**As a** person reading a language I don't speak,
**I want** to press one key and drag a box around the text,
**so that** I get the meaning without leaving what I'm doing.

**Acceptance criteria**

- Pressing the hotkey from any focused app freezes the screen and shows a selection cursor.
- Dragging a box and releasing produces a translation without any further click.
- The whole loop takes under a second for a normal paragraph.
- Covers FR-10, FR-20, FR-30, FR-40, FR-50.

### US-02 — Read the translation where the words were

**As a** user translating a dense UI with several labels,
**I want** the translation drawn over the original text blocks,
**so that** I can tell which translation belongs to which button without mentally re-mapping a wall of text.

**Acceptance criteria**

- Each recognized line's translation appears positioned over that line's original location.
- Longer translations stay readable — shrink, then wrap, then clip with the full text on hover.
- The original layout is still discernible underneath.
- Covers FR-32, FR-50, FR-51.

### US-03 — Keep translating without re-triggering

**As a** user working through a page with several separate blocks,
**I want** to draw another box straight after seeing a result,
**so that** I don't press the hotkey again for every paragraph.

**Acceptance criteria**

- After a result renders, a new drag starts a new selection in the same overlay session.
- Previous overlaid results clear when the new selection starts.
- `Esc` still exits the whole overlay.
- Covers FR-26, FR-24.

### US-04 — Copy the text out

**As a** developer who just OCR'd an error message,
**I want** to copy the source text and the translation separately,
**so that** I can paste the original into a search engine and the translation into my notes.

**Acceptance criteria**

- The result panel shows both source and translation with a copy button each.
- Copy places exactly the displayed text on the clipboard, with line breaks preserved.
- Covers FR-52, FR-53.

### US-05 — Get my own language, without configuring anything

**As a** first-time user on a Turkish Windows,
**I want** translations in Turkish immediately after install,
**so that** the app is correct for me before I open Settings.

**Acceptance criteria**

- On first run, target language is read from the Windows display language.
- The default provider works with no API key and no account.
- Changing the target language in Settings takes effect on the next capture, no restart.
- Covers FR-40, FR-41, FR-60, NFR-U1, NFR-U4.

### US-06 — Stay out of my way

**As a** user who leaves the app running all day,
**I want** it invisible and cheap until I need it,
**so that** it never costs me a taskbar slot, RAM, or battery.

**Acceptance criteria**

- No taskbar entry, no `Alt+Tab` entry, one tray icon.
- Idle memory ≤ 30 MB, idle CPU ~0%.
- Nothing runs — no timer, no watcher — until the hotkey fires.
- Covers FR-01, NFR-P4, NFR-P5.

### US-07 — Start with my computer

**As a** daily user,
**I want** the app to be there after a reboot without me launching it,
**so that** the hotkey always works.

**Acceptance criteria**

- First run asks once, defaulting to off; the answer is remembered.
- When enabled, after reboot the app is present in the tray with no window shown.
- Toggling it off removes the run entry completely.
- Covers FR-05.

### US-08 — Use my own translation provider

**As a** privacy-conscious or accuracy-focused user,
**I want** to choose DeepL, Google, or an LLM endpoint with my own key,
**so that** I control quality and who sees my text.

**Acceptance criteria**

- Provider is selectable in Settings; the key field is per-provider.
- A "test connection" action confirms the setup or names the exact failure.
- The key is stored in Windows Credential Manager, never in the settings file or logs.
- Covers FR-41, FR-62, FR-63.

### US-09 — Know that my screen isn't being uploaded

**As a** user capturing something confidential,
**I want** a guarantee that the image stays on my machine,
**so that** I can use this at work.

**Acceptance criteria**

- OCR runs locally; only extracted text is transmitted.
- No telemetry, analytics, or update pings exist in the binary.
- The README states this plainly and the code makes it verifiable.
- Covers FR-45, FR-64, NFR-S1, NFR-S4.

### US-10 — Trust it on my multi-monitor setup

**As a** user with a 150%-scaled laptop screen and a 100% external monitor,
**I want** the selection to land exactly where I drew it,
**so that** I'm not fighting a half-offset capture.

**Acceptance criteria**

- The frozen frame aligns pixel-exactly with each physical display.
- A box drawn on the secondary monitor OCRs the content actually inside it.
- Plugging or unplugging a monitor between captures doesn't break the next one.
- Covers FR-21, NFR-R4.

### US-11 — Understand what went wrong

**As a** user whose capture just failed,
**I want** to be told what happened and what to do,
**so that** I can fix it instead of guessing.

**Acceptance criteria**

- Missing OCR language pack → says so and links to the Windows setting.
- No text found → says so, overlay stays open for a re-selection.
- Provider failure → names the cause, keeps the OCR text copyable.
- Covers FR-34, FR-35, FR-44, NFR-U5.

### US-12 — Build it myself

**As a** contributor who found the repo,
**I want** clone-to-running-build in one documented command,
**so that** I can fix the bug I came to fix.

**Acceptance criteria**

- [../onboarding.md](../onboarding.md) works on a clean Windows machine.
- The commands are copy-pasteable and don't assume the author's environment.
- Covers NFR-M4, S4.
