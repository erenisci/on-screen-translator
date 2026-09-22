---
title: Definition of Ready
discipline: project
status: active
updated: 2026-09-10
---

# Definition of Ready

> **Purpose.** The bar a piece of work must clear before it's worth starting. Solo-light: this is a thinking prompt,
> not a gate ceremony.
> **Related.** [definition-of-done.md](definition-of-done.md) · [roadmap.md](roadmap.md) · [../product/feature-specs.md](../product/feature-specs.md)

## A task is Ready when…

**It's grounded in a documented requirement.**

- [ ] It traces to an FR/NFR in [../product/requirements-functional.md](../product/requirements-functional.md) or
      [../product/requirements-nfr.md](../product/requirements-nfr.md), or it's an explicit new requirement added there first.
- [ ] It belongs to a milestone in [roadmap.md](roadmap.md) — or it's a bug fix, which needs no milestone.
- [ ] If it's out of v1 scope, it goes to [../product/roadmap-vision.md](../product/roadmap-vision.md) → Later, not into the sprint.

**The behaviour is decided, not left to the implementer.**

- [ ] Success is described concretely enough to test: given this input, this happens.
- [ ] The edge cases are named. For anything touching capture or OCR, that means at minimum: multi-monitor,
      mixed DPI, empty result, and provider failure.
- [ ] Error behaviour is specified — what the user sees and what they can do about it ([NFR-U5](../product/requirements-nfr.md#usability)).
- [ ] If the feature has states, they're listed. [../product/feature-specs.md](../product/feature-specs.md) is where they live.

**The unknowns are unknowns, not undiscovered.**

- [ ] No open question blocks it. If one does, the question is the task — go answer it first.
- [ ] Any spike needed to size it has been done. "I'll figure out DPI while implementing" is not Ready.
- [ ] Third-party behaviour it depends on has been verified, not assumed. Read the API docs before, not during.

**Its cost is understood.**

- [ ] It fits in one sitting, or it's split. A task that can't be described in a paragraph is two tasks.
- [ ] Its effect on the [performance budgets](../product/requirements-nfr.md#performance) is considered — especially
      anything that runs while idle, allocates per capture, or adds a network round trip.
- [ ] If it requires a significant architectural choice, that's an [ADR](../architecture/adr/README.md) first —
      the decision is the deliverable, the code follows.

**It won't quietly break a promise.**

- [ ] It doesn't add telemetry, an outbound call, or anything that touches the image data
      ([NFR-S1](../product/requirements-nfr.md#security), [NFR-S4](../product/requirements-nfr.md#security)).
- [ ] It doesn't put a secret anywhere but Windows Credential Manager.
- [ ] It doesn't add a dependency without a reason worth stating out loud.

## When to skip this list

Typo fixes, dependency bumps, doc edits, and one-line bug fixes don't need a readiness review. The list is for work
that changes behaviour. Applying it to everything is exactly the heavyweight process this project is sized against
([NFR-C5](../product/requirements-nfr.md#constraints)).

## If it isn't Ready

Don't start it. Put what's missing in [../../SCRATCH.md](../../SCRATCH.md) or as an open question in
[../product/prd.md](../product/prd.md), and pick up something that is ready. Starting an unready task is how a solo
project acquires debt it never writes down.
