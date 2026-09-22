# Architecture Decision Records

Every significant technical decision in this project is recorded here: why we chose it, what else was considered,
why not those, and what it costs us later. The value of an ADR is the reasoning — a future maintainer (or a future
Claude session) should be able to tell whether a decision still holds without re-deriving it.

## Index

| #                                                | Title                                                      | Status   | Date       |
| ------------------------------------------------ | ---------------------------------------------------------- | -------- | ---------- |
| [0001](0001-initial-architecture.md)             | Initial architecture — Tauri v2 shell with a Rust core     | Accepted | 2026-09-10 |
| [0002](0002-ocr-engine.md)                       | OCR via `Windows.Media.Ocr` behind an engine trait          | Accepted | 2026-09-10 |
| [0003](0003-translation-provider-abstraction.md) | Pluggable translation providers, user keys, batched request | Accepted | 2026-09-10 |
| [0004](0004-capture-and-coordinate-model.md)     | Capture once into virtual-desktop physical pixels           | Accepted | 2026-09-10 |

## Conventions

- **Filename:** `NNNN-kebab-case-title.md`, numbered sequentially from `0001`. Numbers are never reused.
- **Status:** `Proposed` → `Accepted` → `Superseded by ADR-NNNN`.
- **Immutable except status.** To change a decision, write a new ADR that supersedes the old one and update both
  statuses. Never rewrite the reasoning of a past decision — the record of what we believed at the time is the point.
- **Sections:** Status · Context · Decision · Consequences. Consequences must name what gets *harder*, not just what
  gets easier, and say whether the decision survives the project growing.

## When to write one

Write an ADR when a choice would be expensive to reverse, when a reasonable engineer would have chosen differently,
or when you'll otherwise be asked "why is it like this?" in six months. In practice, for this project: anything
touching capture, coordinates, OCR, translation providers, the IPC boundary, persistence, or the performance
invariants.

Do not write one for library version bumps, formatting choices, or anything already settled by
[../../engineering/coding-standards.md](../../engineering/coding-standards.md).

## Decisions that will need one

Flagged now so they aren't made by accident:

- **Live mode** ([Later](../../product/roadmap-vision.md)) — it breaks the "nothing runs while idle" invariant that
  [ADR-0001](0001-initial-architecture.md) and [ADR-0004](0004-capture-and-coordinate-model.md) both rest on.
- **The keyless default provider** ([PRD Q1](../../product/prd.md#open-questions), [TD-02](../../project/tech-debt.md)).
- **A second OCR engine** and how its language data is delivered ([TD-01](../../project/tech-debt.md)).
- **Offline local translation**, if it ever ships — a size and quality trade worth recording.
