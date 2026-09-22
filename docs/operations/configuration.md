---
title: Configuration Management
discipline: ops
status: active
updated: 2026-09-10
---

# Configuration Management

> **Purpose.** Every knob in the app: where it lives, what it defaults to, and how it changes.
> **Related.** [env-vars.md](env-vars.md) · [security.md](security.md) · [../product/feature-specs.md](../product/feature-specs.md) · [../architecture/api.md](../architecture/api.md)

## Config Sources

There are exactly three, and they are deliberately few:

| Source                         | Holds                                | Lifetime                  |
| ------------------------------ | ------------------------------------ | ------------------------- |
| **Settings JSON** (`%APPDATA%`) | Everything the user configures       | Persistent                |
| **Windows Credential Manager**  | API keys — **only**                  | Persistent until cleared  |
| **Built-in defaults**           | The fallback for every setting       | Compiled in               |

No `.env` file, no config directory of fragments, no registry settings beyond the autostart entry, and no
environment variables at runtime ([env-vars.md](env-vars.md)). One file the user could hand-edit, plus a secure
store for secrets.

**Path:** `%APPDATA%\on-screen-translator\settings.json`

## Precedence

```
built-in defaults  ←  settings.json  ←  live changes in the Settings window
```

Later wins. There is no environment or machine-level override — a per-user desktop tool doesn't need one, and adding
one would create a class of "why is it behaving differently" bugs with no corresponding benefit.

**First-run resolution** deserves its own note, because it's what makes [NFR-U1](../product/requirements-nfr.md#usability)
work: when no settings file exists, the target language is resolved from the **Windows display language**, not from a
hardcoded default. The app is correct for the user before they open Settings
([FR-40](../product/requirements-functional.md#translation)).

## The settings schema

```jsonc
{
  "schemaVersion": 1,

  // Language
  "targetLanguage": "tr",          // BCP-47. Default: Windows display language
  "sourceLanguage": null,          // null = auto-detect (FR-33)

  // Capture
  "hotkey": "Ctrl+Shift+T",        // Tauri accelerator string
  "resultDisplayMode": "both",     // "overlay" | "panel" | "both"

  // Translation
  "provider": "libretranslate",    // "libretranslate" | "deepl" | "google" | "llm"
  "llmEndpoint": null,             // https:// only, when provider === "llm"
  "llmModel": null,

  // General
  "startWithWindows": false,       // asked once on first run, defaults off
  "theme": "system",               // "system" | "light" | "dark"

  // Diagnostics
  "logLevel": "info"               // see logging.md
}
```

**API keys are conspicuously absent.** They live in Credential Manager under
`on-screen-translator/<provider>`, one entry per provider ([security.md](security.md)). Anyone finding a key in this
file has found a bug.

## Defaults

| Setting             | Default                     | Why                                                             |
| ------------------- | --------------------------- | ----------------------------------------------------------------- |
| `targetLanguage`    | **Windows display language** | The app should be right before it's configured                   |
| `sourceLanguage`    | `null` (auto)               | The user shouldn't have to know what they're looking at           |
| `hotkey`            | `Ctrl+Shift+T`              | "T" for translate; rare enough to avoid common conflicts          |
| `resultDisplayMode` | `both`                      | Shows the headline feature and the copyable text on first capture |
| `provider`          | `libretranslate`            | The only keyless option — required for zero-config first run      |
| `startWithWindows`  | `false`                     | Opt-in. An app that adds itself to startup uninvited is rude      |
| `theme`             | `system`                    | Match the OS                                                      |
| `logLevel`          | `info`                      | Useful for support, never contains captured content               |

## Applying changes

**Everything applies live.** No setting requires a restart ([FR-60](../product/requirements-functional.md#settings--persistence)):

| Change                | Effect                                                                   |
| --------------------- | -------------------------------------------------------------------------- |
| Hotkey                | Unregistered and re-registered immediately; failure keeps the old binding  |
| Target language       | Next capture uses it; the open panel offers re-translate                  |
| Provider / key        | Next capture uses it                                                      |
| Display mode          | Next capture                                                              |
| Theme                 | Open windows update immediately via `otr://settings-changed`              |
| Autostart             | Registry entry written immediately; state read back from the registry     |
| Log level             | Applied to the tracing subscriber immediately                             |

Settings are saved **on field change**, not on window close, so nothing is lost if the window is closed abruptly or
the app quits.

## Validation & recovery

The settings file is user-editable, so it must be treated as untrusted input:

| Situation                       | Behaviour                                                                    |
| ------------------------------- | ------------------------------------------------------------------------------ |
| File missing                    | Create it from defaults, resolving the target language from Windows            |
| File unparseable                | Load defaults, warn the user, **rename the bad file** to `settings.corrupt.json` rather than deleting it ([FR-61](../product/requirements-functional.md#settings--persistence)) |
| Unknown field                   | Ignore and preserve it on write — never silently drop a user's data            |
| Invalid value (bad language tag, unknown provider) | Fall back to the default **for that field only**, and log which field |
| Invalid hotkey string           | Keep the previous working binding; surface the error in Settings               |
| `llmEndpoint` not `https://`    | Rejected at save time, in the UI                                               |
| Newer `schemaVersion` than the app understands | Load read-only defaults and warn — a downgrade must not corrupt a newer config |

**Never crash on bad config, and never silently discard it.** A user who hand-edited the file and made a typo should
get a working app and a clear message, not a reset.

## Migration

`schemaVersion` exists so a settings file outlives an upgrade — it is the one piece of state that survives releases
and therefore the only place a real migration story is needed
([../project/release-plan.md](../project/release-plan.md)).

Rules:

- Bump `schemaVersion` only when a field is renamed, removed, or changes meaning. Adding an optional field with a
  default needs no bump.
- Migrations run on load, in sequence, and are **pure functions** `(json, from_version) -> json` — so they're unit
  tested like any other logic ([../quality/testing-strategy.md](../quality/testing-strategy.md)).
- **Back up before migrating**: copy to `settings.v<n>.bak` so a failed migration is recoverable.
- A user's configuration must never be silently discarded by an upgrade. That's the whole point of this section.

## Build-time configuration

Distinct from runtime settings, and all of it lives in the repo:

| File                | Controls                                                               |
| ------------------- | ------------------------------------------------------------------------ |
| `tauri.conf.json`   | **Version (source of truth)**, window definitions, CSP, bundle targets  |
| `Cargo.toml`        | Rust dependencies, release profile (LTO, opt-level, strip)              |
| `package.json`      | Frontend dependencies, scripts                                          |
| `vite.config.ts`    | Multi-entry build — one per window                                      |
| `tailwind.config.ts`| Design tokens for the three windows                                     |

**No build-time feature flags** in v1. There's one product with one configuration; flags would be complexity without
a requirement.
