---
title: Release Plan
discipline: project
status: active
updated: 2026-09-10
---

# Release Plan

> **Purpose.** How a version gets from the working tree to a user's machine.
> **Related.** [../operations/ci-cd.md](../operations/ci-cd.md) · [../operations/deployment.md](../operations/deployment.md) · [definition-of-done.md](definition-of-done.md) · [../../CHANGELOG.md](../../CHANGELOG.md)

## Versioning

[SemVer](https://semver.org/). For a desktop app with no API, the contract we version is **the user's experience and
their settings file**:

| Bump      | When                                                                                                  |
| --------- | ------------------------------------------------------------------------------------------------------- |
| **Major** | A settings migration the user must notice, a removed feature, or a changed default that alters behaviour |
| **Minor** | A new feature — a provider, an OCR engine, a capture mode                                               |
| **Patch** | Bug fixes, accuracy improvements, dependency bumps, doc-only changes that ship in the binary            |

Pre-1.0 (`0.x.y`), minor bumps may carry breaking changes; the CHANGELOG says so explicitly when they do.

**Single source of version truth:** `src-tauri/tauri.conf.json`. `package.json` and `Cargo.toml` follow it. A release
never happens with these out of sync — it's the first check in the checklist below.

## Release checklist

Run in order. Nothing here is automated yet ([TD-08](tech-debt.md)); this list is the process.

**Prepare**

- [ ] All milestone work merged; working tree clean
- [ ] Version bumped in `tauri.conf.json`, matched in `package.json` and `Cargo.toml`
- [ ] `CHANGELOG.md` — `[Unreleased]` promoted to the new version with today's date; entries are user-facing, not commit dumps
- [ ] `/acta:track` run — docs reflect what actually shipped
- [ ] `/acta:audit` clean — no dead links, no stale TBDs in shipped docs

**Verify**

- [ ] `cargo clippy -- -D warnings` and `cargo fmt --check` clean
- [ ] `cargo test` and frontend tests pass
- [ ] [../quality/qa-checklist.md](../quality/qa-checklist.md) run in full
- [ ] Performance budgets in [../product/requirements-nfr.md](../product/requirements-nfr.md) measured and recorded
- [ ] **Clean-machine install test** — a Windows VM with no dev tools, no WebView2 pre-installed, and a different
      display language than the dev box. Install → hotkey → translation, without reading a doc.
- [ ] Multi-monitor mixed-DPI check on real hardware ([FR-21](../product/requirements-functional.md#capture-overlay))
- [ ] Grep the built artifacts and a fresh log file for any API key — must be empty ([FR-62](../product/requirements-functional.md#settings--persistence))
- [ ] Network trace during one capture: exactly one request, to the configured provider, no image payload, no telemetry ([FR-45](../product/requirements-functional.md#translation), [FR-64](../product/requirements-functional.md#settings--persistence))

**Ship**

- [ ] Tag `v<version>` on `main` and push the tag — this triggers the release build ([../operations/ci-cd.md](../operations/ci-cd.md))
- [ ] CI produces the MSI and the portable exe; download both and verify they launch
- [ ] GitHub Release created with the CHANGELOG section as the body
- [ ] Release notes state the **unsigned binary / SmartScreen** situation and how to proceed ([TD-03](tech-debt.md))
- [ ] README screenshot/GIF still matches what the app looks like

**After**

- [ ] `CHANGELOG.md` gets a fresh empty `[Unreleased]` section
- [ ] [../progress.md](../progress.md) updated — milestone ticked, next one active
- [ ] Install the released artifact on the daily-driver machine and actually use it for a week before starting the next milestone

## Cadence

**Release when a milestone is done, not on a schedule.** A solo project with no users has nothing to gain from a
fixed cadence and everything to lose from shipping half a milestone to meet a date.

- `v0.1.0` — after M7, the first thing a stranger can install
- `v0.x` — one release per meaningful milestone
- `v1.0.0` — when the app has been someone's daily tool for a month without a blocking bug

Patch releases go out as soon as a real user hits a real bug. That's the one exception to milestone-gating.

## Environments

There is no server, so "environments" means build configurations:

| Name           | How it's built                              | Purpose                                                   |
| -------------- | -------------------------------------------- | ---------------------------------------------------------- |
| **Dev**        | `npm run tauri dev`                          | Hot-reloaded frontend, debug Rust, verbose logging          |
| **Local release** | `npm run tauri build` on the dev machine  | Verify optimized behaviour and timing before tagging        |
| **CI release** | GitHub Actions on `windows-latest`, on a tag | The only artifacts users ever get                           |

**Rule:** users never receive a locally-built binary. If it wasn't built by CI from a tag, it isn't a release —
this keeps "what shipped" reproducible from the repository alone.

## Rolling back

A desktop app can't be rolled back centrally — see [../operations/rollback.md](../operations/rollback.md) for what
we actually do: keep previous releases downloadable, never break the settings file, and ship a patch fast.
