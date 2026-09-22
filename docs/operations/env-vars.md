---
title: Environment Variables
discipline: ops
status: active
updated: 2026-09-10
---

# Environment Variables

> **Purpose.** What environment variables this project uses — and, importantly, where it deliberately doesn't.
> **Related.** [configuration.md](configuration.md) · [ci-cd.md](ci-cd.md) · [security.md](security.md)

## The short version

**The shipped app reads no environment variables at runtime.** Not one.

User configuration lives in a JSON file, secrets live in Windows Credential Manager
([configuration.md](configuration.md)). A desktop app launched from a tray icon or a registry Run entry has no
meaningful environment to read from — the shell that started it isn't the user's shell — so environment-based config
would be a source of "why doesn't it pick that up" confusion with no benefit.

The variables below exist only for **development** and **CI**.

## Variables

### Development (optional)

Set these in your own shell only when you want the behaviour. None is required to build or run.

| Name              | Purpose                                                    | Required | Example                    |
| ----------------- | ---------------------------------------------------------- | -------- | -------------------------- |
| `RUST_LOG`        | Overrides the tracing filter for a dev run. Standard `tracing` syntax. | No | `otr=debug,warn`      |
| `RUST_BACKTRACE`  | Rust backtraces on panic. Invaluable for WinRT interop bugs. | No     | `1`                        |
| `OTR_DEV_LOG_DIR` | Redirect logs somewhere other than `%APPDATA%` while developing. | No | `C:\tmp\otr-logs`      |

`RUST_LOG` affects verbosity, never content: the no-captured-content rule in [logging.md](logging.md) holds at every
level, including `trace`. There is no debug flag that dumps recognized text or captured images to disk — that would
undermine [G5](security.md#the-guarantees), and it's why the fixture set exists instead.

### CI / release (GitHub Actions)

Provided as repository secrets, never present on a developer machine and never in the shipped binary.

| Name                          | Purpose                                              | Required           |
| ----------------------------- | ---------------------------------------------------- | ------------------ |
| `GITHUB_TOKEN`                | Publishing release artifacts. Supplied by Actions.   | Yes (automatic)    |
| `TAURI_SIGNING_PRIVATE_KEY`   | Updater artifact signing — **only if** auto-update is ever added. | No (not used in v1) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the above.                       | No (not used in v1) |
| `WINDOWS_CERTIFICATE`         | Code-signing certificate (base64 PFX) — **only if** signing is adopted. | No ([TD-03](../project/tech-debt.md)) |
| `WINDOWS_CERTIFICATE_PASSWORD`| Password for the above.                              | No                 |

The last four are listed because they're the ones a contributor would expect to find and wonder about. v1 ships
**unsigned with no auto-updater** ([TD-03](../project/tech-debt.md), [PRD Q2](../product/prd.md#open-questions)), so
those secrets are not configured. If signing is adopted, they get set in repository secrets and referenced from the
release workflow — never committed, never echoed into a log.

## What is NOT an environment variable

Recorded so nobody adds them:

| Not an env var          | Where it actually lives                                          |
| ----------------------- | ------------------------------------------------------------------ |
| **Translation API keys** | Windows Credential Manager, via `secrets.rs` ([security.md](security.md)) |
| Target / source language | `settings.json`                                                   |
| Hotkey                   | `settings.json`                                                   |
| Provider selection       | `settings.json`                                                   |
| LLM endpoint URL         | `settings.json`                                                   |
| App version              | `tauri.conf.json` — the single source of truth                    |

**An API key must never be read from an environment variable**, not even as a "convenient" dev shortcut. Environment
variables leak into child processes, crash dumps, and `ps` output; that's exactly the exposure Credential Manager
exists to avoid, and a dev-only path has a way of becoming a shipped path.

## Rules

- **No secret is ever committed.** No `.env` file exists in this project, and `.env*` is git-ignored anyway as a
  belt-and-braces measure.
- **Names and purposes are documented here; values never are.**
- If a variable is ever added that the *shipped app* reads, it needs a line in this table and a note in
  [configuration.md](configuration.md) explaining why the settings file wasn't the right place — because it almost
  always is.
