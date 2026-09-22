---
title: Self-Review Checklist
discipline: code
status: active
updated: 2026-09-10
---

# Self-Review Checklist

> **Purpose.** The pass to make on your own diff before calling it done. Solo-light: this replaces a second
> reviewer, so it has to catch what a second reviewer would.
> **Related.** [../project/definition-of-done.md](../project/definition-of-done.md) · [coding-standards.md](coding-standards.md) · [git-workflow.md](git-workflow.md)

## How to use it

Read your own diff top to bottom before opening the PR — not the code in your editor, the **diff**. Different view,
different eyes. The sections below are ordered by how expensive the mistake is to find later.

## Before commit

- [ ] Read the whole diff. Anything you can't justify out loud comes out.
- [ ] No debug printing, no commented-out code, no `dbg!`, no `console.log`.
- [ ] No `TODO` without a matching note in [../../SCRATCH.md](../../SCRATCH.md) or
      [../project/tech-debt.md](../project/tech-debt.md). A `TODO` nobody tracked is a lie.
- [ ] No unrelated changes riding along — split them out ([git-workflow.md](git-workflow.md)).
- [ ] Formatters and linters clean: `cargo fmt --check`, `cargo clippy -- -D warnings`, Prettier, ESLint, `tsc --noEmit`.
- [ ] Commit message explains **why**, references an ADR/FR when one applies.

## Correctness

- [ ] The acceptance criteria from the FR or user story are actually met — you ran the app and watched it.
- [ ] **Coordinates:** every pixel value is named for its space (`_physical` / `_css`), and no conversion happens
      outside `coords.ts` ([ADR-0004](../architecture/adr/0004-capture-and-coordinate-model.md)). This is the
      single highest-value line on this list.
- [ ] **Tried on a multi-monitor mixed-DPI setup** if it touches capture, coordinates, or any window. A
      single-monitor pass proves nothing here.
- [ ] Negative virtual-desktop origin still works (a monitor left of the primary).
- [ ] Edge cases from [../product/feature-specs.md](../product/feature-specs.md) were tried by hand, not assumed.
- [ ] Failure paths were exercised deliberately: network pulled, wrong key, empty selection, corrupt settings.
- [ ] No `unwrap()` / `expect()` on anything that can fail at runtime.
- [ ] Nothing can leave the user behind an invisible full-screen overlay — `Esc` still works from every state
      ([NFR-R1](../product/requirements-nfr.md#reliability)).

## Boundaries

The four rules from [project-structure.md](project-structure.md), because they're the ones that erode quietly:

- [ ] No `invoke` outside `src/lib/ipc.ts`.
- [ ] No logic added to a Tauri command handler — it delegates and maps errors.
- [ ] No network, filesystem, or OS access from the frontend.
- [ ] Windows API calls only in modules marked `[WINDOWS]`.
- [ ] No API key read or written outside `secrets.rs`.
- [ ] `types.gen.ts` was regenerated, not hand-edited.

## Performance

- [ ] **Nothing new runs while idle** — no timer, no interval, no watcher, no background task
      ([NFR-P5](../product/requirements-nfr.md#performance)). If you added one, you changed the architecture and
      owe an ADR.
- [ ] No second screen capture per session; selections still crop the cached frame.
- [ ] No additional outbound request per capture ([FR-42](../product/requirements-functional.md#translation)).
- [ ] Large buffers are dropped when the overlay closes ([NFR-P6](../product/requirements-nfr.md#performance)).
- [ ] If it plausibly affects latency, it was **measured** against the
      [stage budget](../architecture/system-design.md), not estimated.
- [ ] No new dependency you couldn't justify against the 20 MB install budget.

## Tests

- [ ] Pure logic touched → unit tests. Coordinates, line assembly, batching, text fitting, settings parsing.
- [ ] Pipeline touched → a test with the fake engine and fake provider.
- [ ] **A fixed bug has a test that fails without the fix.** No exceptions — that's the only thing that keeps it fixed.
- [ ] Tests would actually fail if the behaviour regressed. A test that passes against a broken implementation is
      worse than no test.
- [ ] `cargo test` and the frontend suite pass.

## Security & privacy

Every item here is a promise the project made publicly ([../operations/security.md](../operations/security.md)):

- [ ] No telemetry, analytics, or update ping was added ([NFR-S4](../product/requirements-nfr.md#security)).
- [ ] The captured image still never leaves the device ([NFR-S1](../product/requirements-nfr.md#security)).
- [ ] No captured text or pixels written to disk ([NFR-S6](../product/requirements-nfr.md#security)).
- [ ] No secret in the settings file, the logs, an error `detail`, or a crash message.
- [ ] Any new outbound request goes to the user-configured provider only, over TLS.
- [ ] New dependencies reviewed — what they pull in, and whether they phone home.

## Docs

- [ ] Docs updated **in this change**, not deferred — `/acta:track` rather than hand-editing a pile of files.
- [ ] A significant decision made along the way became an [ADR](../architecture/adr/README.md).
- [ ] [../architecture/api.md](../architecture/api.md) updated if the IPC surface changed.
- [ ] `CHANGELOG.md` `[Unreleased]` entry if a user would notice.
- [ ] Anything left undone is in [../project/tech-debt.md](../project/tech-debt.md) with its reason.

## The last question

> **If someone else wrote this, what would you object to?**

Ask it honestly. The answer is usually one thing, and it's usually right. Fix it now — it costs a minute here and an
afternoon in three months.
