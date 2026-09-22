---
title: Feature Specs
discipline: product
status: active
updated: 2026-09-10
---

# Feature Specs

> **Purpose.** How each v1 feature actually behaves — states, edge cases, and the decisions a developer would
> otherwise have to invent while coding.
> **Related.** [requirements-functional.md](requirements-functional.md) · [../architecture/system-design.md](../architecture/system-design.md) · [../operations/error-handling.md](../operations/error-handling.md)

---

## F1 — Tray presence & lifecycle

**Summary.** The app's entire permanent surface is one tray icon. There is no main window.

**Behavior**

- Launch registers the tray icon, the global hotkey, and nothing else. No window is created.
- Left-click → start capture. Right-click → menu: `Capture`, `Settings`, `—`, `Quit`.
- The tray tooltip shows the app name and the current hotkey, so the user can always recall the binding.
- Single-instance: a second launch signals the first (which flashes the tray icon) and exits.

**States**

| State     | Icon        | Meaning                                        |
| --------- | ----------- | ---------------------------------------------- |
| Idle      | Normal      | Ready; nothing running                         |
| Working   | Subtle busy | A capture pipeline is in flight                |
| Degraded  | Badged      | Hotkey registration failed, or provider unset  |

**Edge cases**

- Explorer restarts and clears the tray → re-register the icon on `TaskbarCreated`.
- Hotkey registration fails at startup → start anyway in Degraded, with an actionable notification (FR-12).
- Quit while a capture is in flight → cancel the pipeline, close windows, then exit. No orphan process (NFR-R5).

**Out of scope** — jump lists, notification center integration, a main window.

---

## F2 — Autostart

**Summary.** Opt-in start with Windows, minimized to tray.

**Behavior**

- First run shows a one-time prompt, defaulting to **off**. The answer is stored; the prompt never repeats.
- Enabled → a per-user registry Run entry (`HKCU`), pointing at the current executable with a `--autostart` flag.
- With `--autostart`, the app skips the first-run prompt and shows no notification.
- Disabling removes the entry entirely.

**Edge cases**

- The executable is moved → the stale entry is detected at startup and rewritten to the current path.
- Another tool (a security suite) removed the entry → Settings reflects reality by reading the registry, not a cached flag.

**Out of scope** — Task Scheduler entries, per-machine (`HKLM`) install, delayed start.

---

## F3 — Global hotkey

**Summary.** One rebindable global shortcut, default `Ctrl+Shift+T`.

**Behavior**

- Registered at startup; re-registered live when changed in Settings (no restart, FR-60).
- Capture is edge-triggered: holding the keys does not queue repeats.
- While an overlay is already open, pressing the hotkey again is a no-op (not a nested capture).

**Edge cases**

- Binding already claimed → registration fails, the previous binding is kept, Settings shows an inline error naming
  the conflict. The app never ends up silently unbound (FR-12).
- A modifier-less binding (a bare letter) is rejected in the Settings UI — it would hijack all typing.
- Elevated (admin) windows have focus → the hotkey may not deliver; documented as a known Windows limitation
  in [../maintenance.md](../maintenance.md), not a bug to chase.

**Out of scope** — multiple hotkeys, per-provider hotkeys, mouse-button bindings.

---

## F4 — Capture overlay

**Summary.** A Lightshot-style selection surface over a frozen picture of the whole desktop.

**Behavior**

1. On trigger, the Rust core reads the current monitor topology and captures the **entire virtual desktop once**
   into a single frame (FR-20). Topology is re-read every time — never cached across captures (NFR-R4).
2. One borderless, always-on-top, click-through-disabled window is created spanning the virtual desktop bounds.
   It renders the frozen frame at 1:1 physical pixels, with a dim scrim over everything.
3. Drag draws a selection rectangle: the scrim is cleared inside it, a 1 px border marks it, and a readout shows
   `width × height` in physical pixels, flipped to stay on screen near the edges.
4. Release ≥ 8×8 px runs the pipeline (F5). Release under that is treated as a mis-click and ignored (FR-25).
5. `Esc` or right-click cancels: the window is destroyed, the frame buffer dropped, and focus returned to the
   window that had it before the hotkey (FR-24).

**Coordinate model** — the one thing that must not be improvised:

- All capture, crop, and OCR coordinates are **physical pixels in virtual-desktop space**, origin at the top-left
  of the virtual desktop (which can be negative on multi-monitor layouts).
- The overlay window is created with per-monitor DPI awareness (`PerMonitorV2`) so the webview is never auto-scaled.
- The frontend converts CSS pixels → physical pixels once, at the window's own scale factor, before sending a
  region to Rust. Nothing downstream re-scales. See [../architecture/system-design.md](../architecture/system-design.md).

**States** — `Selecting` → `Working` (spinner at the selection) → `Showing result` → `Selecting` (new drag) → closed.

**Edge cases**

| Case                                          | Behavior                                                                   |
| --------------------------------------------- | ---------------------------------------------------------------------------- |
| Mixed per-monitor DPI                          | Frame is composed in physical pixels; alignment verified per monitor (FR-21) |
| Negative virtual-desktop origin (left monitor) | Handled by using virtual bounds, not primary-monitor bounds                  |
| Monitor unplugged between captures             | Topology re-read at each capture; the stale layout is never reused           |
| Fullscreen exclusive game                      | Best-effort. If capture or overlay fails, report it plainly (FR-34-style), don't hang |
| A drag that starts on one monitor, ends on another | Fully supported — it's one coordinate space                             |
| Screen content changes after freeze            | Irrelevant by design; the frozen frame is the truth for this session         |

**Out of scope** — freehand/window/element selection, magnifier loupe, drawing tools, saving the capture.

---

## F5 — OCR pipeline

**Summary.** Crop → pre-process → recognize → structured lines.

**Behavior**

1. **Crop** the selection from the in-memory frame. The screen is never re-captured (FR-30).
2. **Pre-process**, in this order, to lift accuracy on small UI text (FR-31):
   - upscale ×2–×3 (Lanczos) when the estimated text height is under ~20 px,
   - convert to grayscale,
   - normalize contrast; apply adaptive thresholding when contrast is low.
   Pre-processing is skipped when the region is already large and high-contrast — it costs time and can hurt.
3. **Recognize** via the engine, returning per line: `text`, `bbox` (frame coordinates), `confidence` (FR-32).
4. **Assemble**: sort lines top-to-bottom, group into blocks by vertical gaps, and join into `full_text` with the
   paragraph structure preserved — because translation quality depends on sentence context, not stray fragments.
5. **Detect** the source language, unless overridden in Settings (FR-33).

**Edge cases**

| Case                          | Behavior                                                                     |
| ----------------------------- | ------------------------------------------------------------------------------ |
| No text found                  | "No text found" in the overlay; overlay stays open to re-select (FR-35)       |
| OCR language pack missing      | Actionable message + link to the Windows language setting (FR-34)             |
| Very low confidence lines      | Kept but marked; the panel flags a low-confidence result so a bad translation isn't read as authoritative |
| Huge selection (a whole 4K screen) | Allowed; a progress indicator appears past ~300 ms                        |
| Mixed-language region          | One detected language wins for v1; noted as a known limitation                |

**Out of scope** — vertical scripts, handwriting, table structure recovery, per-word bounding boxes.

---

## F6 — Translation

**Summary.** One batched request per capture, to a user-chosen provider.

**Behavior**

- Target language defaults to the Windows display language, overridable in Settings (FR-40).
- All lines go in **one** request (FR-42), sent as a delimited batch so per-line boxes can be reconstructed for the
  in-place overlay while the provider still sees full sentence context.
- If source == target, translation is skipped entirely and the source is shown as-is (FR-43).
- Providers behind one trait: `LibreTranslate` (keyless default), `DeepL`, `Google`, `LlmEndpoint` (FR-41).
- Only text is sent — never the image (FR-45).

**Edge cases**

| Case                                | Behavior                                                                              |
| ----------------------------------- | --------------------------------------------------------------------------------------- |
| Network down / DNS failure           | Named error; OCR source text stays visible and copyable (FR-44)                        |
| Rate limited (429)                   | Says "provider rate-limited", suggests configuring a key; no automatic retry storm     |
| Invalid key (401/403)                | "Provider rejected the key — check it in Settings"                                     |
| Provider drops or merges lines       | Fall back to showing the whole translation in the panel rather than mis-aligning boxes |
| Very long text over provider limits  | Split into as few chunks as the limit allows, still batched; never one request per line |

**Out of scope** — offline models, glossaries, alternative-translation suggestions, per-line re-translation.

---

## F7 — In-place result overlay

**Summary.** The headline feature: read the translation where the words were, without leaving the overlay.

**Behavior**

- For each recognized line, its translation is drawn positioned at that line's `bbox`, over a backing shape that
  guarantees contrast against whatever is beneath (NFR-U8).
- **Fitting ladder** for translations longer than their source box (FR-51):
  1. shrink the font toward a **10 px effective floor**,
  2. wrap onto extra lines, growing the box downward into free space,
  3. if it still doesn't fit, clip with an ellipsis and expose the full text on hover.
- The original frozen frame stays visible around the boxes, so layout context is preserved.
- Starting a new drag clears the overlaid results and returns to `Selecting` (FR-26).

**Edge cases**

- Boxes that would overlap after growing → later boxes are nudged down; overlap is never allowed to make text unreadable.
- A single line box wider than the selection (rare OCR artifact) → clamped to the selection bounds.
- Translation returns fewer segments than there are lines → fall back to panel-only display for that capture, rather
  than guessing an alignment.

**Out of scope** — editing the overlaid text, re-flowing the original layout, exporting the annotated view.

---

## F8 — Result panel

**Summary.** The copyable companion to the overlay.

**Behavior**

- A frameless, always-on-top, translucent window near the selection (clamped to stay fully on-screen).
- Shows: detected source language → target language, the source text, the translation, and a copy button for each (FR-52).
- Controls: change target language + re-translate (reuses cached OCR text, no re-capture and no re-OCR, FR-54), and close.
- `Esc` closes it (FR-53). Closing the panel does not close the overlay, and vice versa.
- Display mode — `overlay` / `panel` / `both` — is a setting; default `both` (FR-55).

**Edge cases**

- Text longer than the panel → the panel scrolls; it never grows past a sane maximum height.
- Selection at a screen edge → the panel flips to the other side of the selection.
- Re-translate with an unreachable provider → the existing translation stays on screen; the error is inline.

**Out of scope** — history, pinning, editing the source, TTS.

---

## F9 — Settings

**Summary.** One small window. Every field must earn its place.

**Behavior**

| Group       | Fields                                                                    |
| ----------- | ------------------------------------------------------------------------- |
| Language    | Target language · Source language (Auto by default) · OCR languages installed |
| Capture     | Hotkey · Result display mode                                              |
| Translation | Provider · API key (per provider) · Test connection                       |
| General     | Start with Windows · Theme (System/Light/Dark)                            |

- Changes apply immediately — the hotkey rebinds live, no restart (FR-60).
- Persisted to a single JSON file under `%APPDATA%`; a corrupt file falls back to defaults with a warning (FR-61).
- API keys go to Windows Credential Manager, never to the JSON or the logs (FR-62).
- "Test connection" reports success or the specific failure (FR-63).
- No "check for updates", no "send diagnostics" — those don't exist here (FR-64).

**Edge cases**

- Selected OCR language not installed on Windows → shown inline with a link to the system setting, before a capture fails.
- Provider changed while a key for the previous one exists → the old key stays in Credential Manager under its own
  entry so switching back doesn't require re-entering it.
- Settings open when the app quits → window closed cleanly, pending edits saved on field change, not on close.

**Out of scope** — import/export of settings, profiles, sync, a settings search box.
