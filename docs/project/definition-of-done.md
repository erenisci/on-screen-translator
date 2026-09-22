---
title: Definition of Done
discipline: project
status: active
updated: 2026-09-10
---

# Definition of Done

> **Purpose.** What "finished" means, so work doesn't get called done while half of it is still owed.
> **Related.** [definition-of-ready.md](definition-of-ready.md) · [../engineering/self-review-checklist.md](../engineering/self-review-checklist.md) · [../quality/qa-checklist.md](../quality/qa-checklist.md) · [release-plan.md](release-plan.md)

## A task is Done when…

**It works, and you watched it work.**

- [ ] The acceptance criteria from its FR/user story are met — verified by running the app, not by reading the diff.
- [ ] It works on a **multi-monitor, mixed-DPI setup** if it touches capture, coordinates, or any window. This is the
      project's most common source of "works on my machine" ([FR-21](../product/requirements-functional.md#capture-overlay)).
- [ ] The named edge cases from [../product/feature-specs.md](../product/feature-specs.md) were tried by hand.
- [ ] Failure paths were exercised deliberately: pull the network, use a wrong key, select an empty area,
      corrupt the settings file — whichever apply.

**It's tested where testing pays.**

- [ ] Coordinate math, line assembly, batching, and settings parsing have unit tests — these are pure logic and
      they're where silent wrongness hides ([../quality/testing-strategy.md](../quality/testing-strategy.md)).
- [ ] Pipeline changes have a test with a fake provider and a fixture image.
- [ ] A fixed bug has a test that fails without the fix. No exceptions — that's the only way a bug stays fixed.
- [ ] Tests pass: `cargo test` and the frontend suite.

**It's clean.**

- [ ] `cargo clippy -- -D warnings` and `cargo fmt --check` are clean; the frontend lints and type-checks.
- [ ] [../engineering/self-review-checklist.md](../engineering/self-review-checklist.md) has been walked, not skimmed.
- [ ] No commented-out code, no leftover debug printing, no `TODO` without a matching note in
      [../../SCRATCH.md](../../SCRATCH.md) or [tech-debt.md](tech-debt.md).
- [ ] It reads like the code around it ([../engineering/coding-standards.md](../engineering/coding-standards.md)).

**It didn't cost more than it's worth.**

- [ ] Nothing new runs while the app is idle — no timer, no watcher, no background thread
      ([NFR-P5](../product/requirements-nfr.md#performance)).
- [ ] No new allocation of a full-screen buffer per capture beyond the one frame, and everything is released when
      the overlay closes ([NFR-P6](../product/requirements-nfr.md#performance)).
- [ ] No new outbound request per capture ([FR-42](../product/requirements-functional.md#translation)).
- [ ] If it plausibly affects latency, it was measured against the budget, not estimated.

**It kept the promises.**

- [ ] No telemetry, analytics, or update ping was added ([NFR-S4](../product/requirements-nfr.md#security)).
- [ ] The captured image still never leaves the device ([NFR-S1](../product/requirements-nfr.md#security)).
- [ ] No secret in the settings file, the logs, or an error message ([FR-62](../product/requirements-functional.md#settings--persistence)).
- [ ] No captured text or image is written to disk ([NFR-S6](../product/requirements-nfr.md#security)).

**The docs caught up.**

- [ ] The affected docs are updated — run `/acta:track` rather than hand-editing a pile of files.
- [ ] A significant decision made along the way is an [ADR](../architecture/adr/README.md).
- [ ] `CHANGELOG.md` has an `[Unreleased]` entry if a user would notice the change.
- [ ] [../progress.md](../progress.md) reflects reality.
- [ ] Anything deliberately left undone is in [tech-debt.md](tech-debt.md) with its reason — not left to be
      rediscovered later.

## A milestone is Done when…

Every task in it is done, plus:

- [ ] The milestone's own "Done when" line in [roadmap.md](roadmap.md) is satisfied
- [ ] The full loop still works end to end — press, drag, read, copy, `Esc`. Regressions in the core gesture outrank
      whatever the milestone added.
- [ ] [../quality/qa-checklist.md](../quality/qa-checklist.md) passes
- [ ] The work is committed as a coherent phase with a message explaining *why*, not just *what*

## A release is Done when…

See [release-plan.md](release-plan.md) — the checklist there supersedes this section for shipping.

## What "Done" is not

- Not "the happy path worked once."
- Not "it compiles and the types check."
- Not "I'll write the test after." The test is part of the task, not a follow-up.
- Not "I'll update the docs at the end of the milestone." Docs drift the moment they lag.
