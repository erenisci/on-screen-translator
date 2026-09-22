---
title: Maintenance
discipline: knowledge
status: active
updated: 2026-09-10
---

# Maintenance

> **Purpose.** Keeping the project healthy between features, and diagnosing what users report.
> **Related.** [onboarding.md](onboarding.md) · [operations/logging.md](operations/logging.md) · [project/tech-debt.md](project/tech-debt.md) · [operations/security.md](operations/security.md)

## Routine Tasks

Right-sized for one maintainer. Nothing here is on a calendar reminder; they're triggered by events.

| When                          | Do                                                                              |
| ----------------------------- | --------------------------------------------------------------------------------- |
| After finishing a chunk       | `/acta:track` — sync docs before the details fade                               |
| After a milestone             | [qa-checklist.md](quality/qa-checklist.md); commit the phase                    |
| Before a release              | [release-plan.md](project/release-plan.md) in full                               |
| A `cargo audit` / `npm audit` warning appears | Assess it — is the vulnerable path one we actually use?         |
| A new Tauri minor version     | Read the changelog before upgrading; plugin APIs move                            |
| A user reports a bug          | Ask for the log summary line first (see below)                                   |
| Quarterly-ish                 | Re-run the OCR fixture benchmark; check the accuracy trend hasn't drifted        |
| When something feels slow     | Measure against the [stage budget](architecture/system-design.md), don't guess   |

## Dependencies

**Philosophy:** few, small, pinned, deliberate. Every dependency is a supply-chain surface in an app that reads the
screen ([security.md](operations/security.md#dependencies)).

| Dependency group        | Upgrade posture                                                            |
| ----------------------- | ---------------------------------------------------------------------------- |
| **Tauri + plugins**     | Pinned exactly. Upgrade deliberately, read the changelog, re-run QA — it sits on the OS and the IPC boundary |
| **`windows` crate**     | Conservative. WinRT binding changes can break OCR interop in subtle ways     |
| **`image`**             | Routine; re-run the OCR fixture benchmark after — it can shift pre-processing results |
| **HTTP client**         | Routine, security-driven                                                    |
| **React / Vite / Tailwind** | Routine. Major versions on their own branch                             |
| **Dev tooling**         | Freely                                                                      |

**After any upgrade touching `image`, the `windows` crate, or Tauri: run the fixture benchmark and one manual
multi-monitor capture.** These three can change behaviour without changing any of our code, and the failure is
silent — a slightly different crop or a slightly worse recognition rate.

Lockfiles are committed. No automated dependency-update PRs ([ci-cd.md](operations/ci-cd.md)) — a bot opening weekly
PRs on an unstaffed repo trains you to ignore them.

## Upgrades

**Rust toolchain** — stay on stable; upgrade when a dependency needs it or a release is a few months old. Rerun
`clippy` after: new lints appear and they're usually right.

**Node** — LTS. Only matters at build time.

**Tauri major versions** — treat as a project, not a chore. Own branch, full QA pass, likely doc updates. Tauri v2's
plugin architecture is load-bearing here ([ADR-0001](architecture/adr/0001-initial-architecture.md)).

**Windows itself** — a Windows update can change OCR behaviour or DPI handling under us. If accuracy or geometry
suddenly shifts with no code change, check the OS build first; the log's startup line records it.

## Troubleshooting

### Diagnosing a user report

**Ask for the `capture_complete` log line first.** One structured line at `info` carries almost everything needed —
region size, monitor count, scale factors, line and character counts, mean confidence, languages, provider, and
per-stage timings ([logging.md](operations/logging.md)). It contains no captured content, so it's safe for a user to
paste into an issue.

Reading it:

| Field                    | Tells you                                                             |
| ------------------------ | ----------------------------------------------------------------------- |
| `scale_factors`, `monitors` | Whether this is the mixed-DPI class of bug                         |
| `region_px`              | Whether the crop was the size they thought they selected               |
| `ocr_lines`, `ocr_chars` | Whether OCR found anything at all                                      |
| `mean_confidence`        | Whether it's an accuracy problem rather than a plumbing problem        |
| `alignment_ok`           | Whether the in-place overlay fell back to panel-only ([TD-06](project/tech-debt.md)) |
| `*_ms`                   | Which stage is slow                                                    |

### Known issues and their answers

| Symptom                                        | Cause / response                                                        |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| Hotkey doesn't fire                             | Claimed by another app. Rebind ([FR-12](product/requirements-functional.md#hotkey)) |
| Hotkey doesn't fire only over an admin window   | Windows limitation — an unelevated app can't receive input over an elevated window. Not fixable without shipping an elevated app, which we won't |
| Black rectangle or capture failure in a game    | Exclusive fullscreen. Suggest borderless windowed ([TD-07](project/tech-debt.md)) |
| Crop offset on a secondary monitor              | A coordinate conversion outside `coords.ts` — the highest-priority class of bug |
| OCR misses small or stylized text               | Engine limitation ([TD-01](project/tech-debt.md)). Try a larger selection; pre-processing is the only lever |
| "Language pack not installed"                   | Working as intended — link them to Windows language settings            |
| Translation fails on first run                  | Keyless default rate-limited ([TD-02](project/tech-debt.md)). Suggest a personal key |
| Translation lands on the wrong box              | Segment mismatch; should have fallen back to panel-only. If it didn't, that's a bug |
| SmartScreen / AV blocks the download            | Unsigned build ([TD-03](project/tech-debt.md)). Point at the README section |
| Memory grows over a session                     | Frame buffer not released on overlay close. Check against [NFR-P6](product/requirements-nfr.md#performance) |

### Where things live on a user's machine

| What                     | Path                                                        |
| ------------------------ | ------------------------------------------------------------- |
| Settings                 | `%APPDATA%\on-screen-translator\settings.json`              |
| Logs (7-day rotation)    | `%APPDATA%\on-screen-translator\logs\`                      |
| API keys                 | Windows Credential Manager → `on-screen-translator/<provider>` |
| Autostart entry          | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`         |

**Full manual removal** (uninstall leaves the first three deliberately, so a reinstall keeps the user's setup —
[deployment.md](operations/deployment.md)): uninstall, then delete `%APPDATA%\on-screen-translator\` and the
Credential Manager entries.

### Reset paths, least to most destructive

1. Delete `settings.json` → defaults, keys kept
2. Clear the provider key in Settings → that key removed
3. Delete `%APPDATA%\on-screen-translator\` → settings and logs gone, keys kept
4. The above + remove Credential Manager entries → fully clean

## Health signals

No dashboards, no monitoring — there's no server and no telemetry
([G3](operations/security.md#the-guarantees)). Health is checked by hand, deliberately:

- **The fixture benchmark** — the accuracy trend over time. The one number worth watching.
- **The stage timings** in your own logs during normal use — the earliest sign of a performance regression.
- **[tech-debt.md](project/tech-debt.md)** — is it growing faster than it's being paid down?
- **Open issues** — for a project whose success metric is one external contributor
  ([S5](product/prd.md#success-metrics)), an unanswered issue is the most expensive thing on this list.
