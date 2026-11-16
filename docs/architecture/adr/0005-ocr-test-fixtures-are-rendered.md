---
title: "ADR 0005: OCR test fixtures are rendered, not committed"
discipline: code
status: Accepted
date: 2026-11-16
---

# ADR 0005: OCR test fixtures are rendered, not committed

## Status

Accepted — 2026-11-16

## Context

[M2](../../project/roadmap.md) closes when "a fixture set of real screenshots recognizes correctly, with bounding
boxes that land on the right lines". [testing-strategy.md](../../quality/testing-strategy.md) originally specified
that set as **committed PNGs plus expected-text files in `src-tauri/fixtures/`**. Building M2 made three problems
with that concrete:

1. **A binary fixture can't be reviewed.** A PNG in a diff is an opaque blob. Nobody — the author six months later
   included — can tell from the repository whether a fixture's expected text is still right, whether it was
   re-exported at a different scale, or what changed when it did.
2. **The expected bounding boxes are unknown.** The whole point of the suite is checking that boxes land on the
   right lines (invariant 1, [FR-32](../../product/requirements-functional.md#ocr)). With a screenshot taken
   elsewhere, the position of the ink is something you eyeball and then hard-code — so the assertion encodes a
   guess, and the one coordinate bug it exists to catch would be baked into the expectation.
3. **A screenshot is a screenshot of someone else's machine.** Different font stack, different DPI, different
   ClearType settings. The app's actual input is text rendered by *this* machine's rasterizer; a fixture from
   another one tests a slightly different problem.

Options considered:

**A. Commit PNG screenshots** (the original plan). Real-world fidelity — including the compression noise, scaling
artifacts and theme quirks that a synthetic fixture lacks. But opaque in review, requires hand-derived box
expectations, and the set grows the repository with every case added.

**B. Render fixtures with GDI at test time.** The test draws its own text with `TextOutW` into a DIB, so the exact
ink position is measurable from the pixels and the glyphs come from the machine's own font rasterizer. Nothing
binary enters the repository, and a fixture's content is readable as source. The cost is realism: rendered text is
*cleaner* than a captured screenshot that has been scaled or compressed, so the suite systematically tests the easy
half of the problem.

**C. Both.** Correct in principle. Deferred in practice — the committed half needs a curated, licence-clean set of
real screenshots that nobody has yet, and adding it later costs nothing that doing it now would save.

## Decision

**OCR fixtures are rendered by the test with GDI, in `src-tauri/tests/ocr_fixtures.rs`. No fixture images are
committed, and `src-tauri/fixtures/` is not created.**

Every fixture is placed at a **negative virtual origin**, because that is the layout that catches a dropped
coordinate translation. Each test measures the rendered ink from the buffer and asserts the returned boxes fall
inside it — the bounding-box claim is checked against where the glyphs actually are, not against a transcribed
guess.

The tests talk to the real `Windows.Media.Ocr`. A machine with no OCR language pack is a configuration
[FR-34](../../product/requirements-functional.md#ocr) explicitly supports, so they report and skip rather than fail
the build.

## Consequences

**Easier**

- Fixtures are readable, diffable source. Adding a case is three lines, not an asset-management exercise.
- Bounding-box assertions are grounded in measured ink, which is what makes them worth running at all.
- The repository stays text-only, and CI needs no asset fetching.
- Timings printed per fixture give real numbers against [NFR-P3](../../product/requirements-nfr.md#performance)
  every time the suite runs.

**Harder**

- **The suite tests clean text.** Rendered ClearType glyphs lack the compression artifacts, resampling blur and
  subpixel fringing of a real captured region. The first measurement showed exactly this gap: the pre-processing
  ladder made no difference on these fixtures at 11 px and nothing recognized at 8 px either way, so
  [FR-31](../../product/requirements-functional.md#ocr)'s claim that pre-processing materially helps small text is
  **unproven** by this suite. Tracked as [TD-11](../../project/tech-debt.md).
- **Results vary with the machine.** Fonts, DPI and installed language packs differ, so a fixture can pass here and
  behave differently on a CI runner. Assertions are therefore written as floors ("never worse", "at least N words")
  rather than exact transcriptions.
- **The engine is a dependency of the test suite.** Without a language pack the OCR tests cover nothing, and a
  green build on such a machine proves less than it appears to — the same trap the multi-monitor rule in
  [definition-of-done.md](../../project/definition-of-done.md) already warns about.

**At scale**

Option C remains the end state. When real misses start arriving from users, the right response is a small committed
set of *those* captures — the cases that actually failed — alongside the rendered suite, with the rendered one
staying as the fast, reviewable regression net. That addition needs no change here: it is another test file reading
bytes instead of drawing them.
