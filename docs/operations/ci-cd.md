---
title: CI/CD Pipeline
discipline: ops
status: active
updated: 2026-09-10
---

# CI/CD Pipeline

> **Purpose.** What runs automatically, when, and what it gates.
> **Related.** [deployment.md](deployment.md) · [../project/release-plan.md](../project/release-plan.md) · [../engineering/git-workflow.md](../engineering/git-workflow.md) · [env-vars.md](env-vars.md)

> **Status:** not yet implemented — part of [M7](../project/roadmap.md). This is the target design.

## Pipeline Stages

Two workflows. There is no staging environment to deploy to and no server to roll out — "CD" here means **producing
the artifacts a user downloads**.

### `check.yml` — on every push and PR

Runs on `windows-latest`, because Windows is the only target and the OCR path can't be exercised anywhere else.

| Stage        | Command                                   | Gate                    |
| ------------ | ----------------------------------------- | ----------------------- |
| Format       | `cargo fmt --check` · `prettier --check`  | Blocks merge            |
| Lint (Rust)  | `cargo clippy -- -D warnings`             | Blocks merge            |
| Lint (TS)    | `eslint` · `tsc --noEmit`                 | Blocks merge            |
| Test (Rust)  | `cargo test`                              | Blocks merge            |
| Test (TS)    | `vitest run`                              | Blocks merge            |
| Audit        | `cargo audit` · `npm audit`               | **Warns**, doesn't block |
| Build        | `cargo build --release` (compile only)    | Blocks merge            |
| Type drift   | Regenerate `types.gen.ts`, fail if it differs | Blocks merge        |

**Two of these deserve explanation:**

*Audit warns rather than blocks* — a new advisory in a transitive dependency shouldn't stop an unrelated bug fix from
merging. It should be visible and addressed deliberately ([security.md](security.md)).

*Type drift blocks* — the generated TypeScript types must match the Rust source. This check is the mechanical
mitigation for the main cost of the two-language architecture
([ADR-0001](../architecture/adr/0001-initial-architecture.md)): drift at the IPC boundary is silent, and this makes
it loud.

**Caching:** `~/.cargo`, `target/`, and `node_modules` are cached — a cold Rust build on a Windows runner is slow
enough to discourage running CI often, which is the wrong incentive.

### `release.yml` — on a `v*` tag

| Stage                | Action                                                    |
| -------------------- | ----------------------------------------------------------- |
| Verify version       | Tag must match `tauri.conf.json` exactly, or fail          |
| Full check           | Everything from `check.yml`                                |
| Build                | `tauri build` → MSI + portable exe                         |
| Checksums            | SHA-256 for each artifact                                  |
| Draft release        | Create a **draft** GitHub Release with the CHANGELOG section |
| Upload               | Artifacts + checksums attached                             |

**The release is created as a draft on purpose.** The maintainer downloads the artifacts, runs the clean-machine
test from [../project/release-plan.md](../project/release-plan.md), and publishes by hand. CI can prove the build
compiles; it cannot prove the app installs and translates on a machine that has never seen a dev tool — and that's
the check that actually matters for a desktop app.

## Triggers

| Event                  | Workflow      |
| ---------------------- | ------------- |
| Push to any branch     | `check.yml`   |
| PR opened / updated    | `check.yml`   |
| Push tag `v*`          | `release.yml` |
| Manual dispatch        | Either        |

No scheduled runs. A nightly build of a project with no users is noise, and a cron job that fails for two weeks
before anyone looks trains you to ignore CI.

## Gates

**To merge:** all `check.yml` stages green. Plus the human gates that CI cannot enforce, from
[../project/definition-of-done.md](../project/definition-of-done.md) — most importantly the **multi-monitor
mixed-DPI verification**. A GitHub runner has one virtual display at 100% scaling, so the environment where this
project's worst bugs live is exactly the one CI can never test. Keeping that explicit prevents a false sense of
safety from a green checkmark.

**To release:** `release.yml` green, plus the full [release checklist](../project/release-plan.md) done by hand
before the draft is published.

## Secrets

| Secret                     | Used by       | Status in v1                                    |
| -------------------------- | ------------- | ------------------------------------------------- |
| `GITHUB_TOKEN`             | `release.yml` | Automatic, provided by Actions                   |
| `WINDOWS_CERTIFICATE`      | `release.yml` | **Not configured** — v1 ships unsigned ([TD-03](../project/tech-debt.md)) |
| `WINDOWS_CERTIFICATE_PASSWORD` | `release.yml` | Not configured                              |

Rules: repository secrets only, never in the repo, never echoed into a log, never exposed to workflows triggered by
a fork's pull request. See [env-vars.md](env-vars.md).

**No translation-provider keys exist in CI.** Provider code is tested against recorded responses, not live APIs
([../quality/testing-strategy.md](../quality/testing-strategy.md)) — which keeps CI free, deterministic, and
independent of a third party's uptime.

## What we deliberately don't automate

For a solo project, each of these costs more than it returns today ([TD-08](../project/tech-debt.md)):

| Not automated              | Why                                                                   |
| -------------------------- | ----------------------------------------------------------------------- |
| Version bumping            | One command, a few times a year; automation here breaks in confusing ways |
| CHANGELOG generation       | Generated changelogs read like commit dumps. Users need prose            |
| Publishing the release     | The clean-machine test is a human step by nature                        |
| Dependency update PRs      | A bot opening weekly PRs on an unstaffed repo is just noise             |
| Coverage reporting         | No coverage target exists, by design                                    |
| E2E GUI tests              | Not feasible on a headless runner; covered manually                     |

Revisit when there's a second maintainer or a regular release cadence — not before.
