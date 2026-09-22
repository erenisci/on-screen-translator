---
title: Testing Strategy
discipline: quality
status: active
updated: 2026-09-10
---

# Testing Strategy

> **Purpose.** What we test, what we deliberately don't, and why — so testing effort goes where bugs actually live.
> **Related.** [qa-checklist.md](qa-checklist.md) · [../architecture/system-design.md](../architecture/system-design.md) · [../project/definition-of-done.md](../project/definition-of-done.md)

## Philosophy

**Test what is silent when it breaks.**

This app is a GUI tool driven by a mouse over a frozen screenshot. Most of it is *loud* when it breaks — you press
the hotkey and nothing appears, so you notice immediately. Automating that is expensive and catches things a
five-second manual check already catches.

The dangerous failures are the **silent** ones:

- A coordinate offset that only appears on a second monitor at a different DPI. The app looks fine; the crop is
  wrong; the translation is nonsense; the user blames OCR.
- A segment-alignment mismatch that puts the right translation on the wrong button.
- An OCR regression from a pre-processing tweak that helps one kind of text and quietly ruins another.
- A settings migration that silently discards a user's configuration.

Those get real, automated tests. The rest gets a checklist and a human.

Consequence: this project's test suite is **narrow and deep**, not broad. That's deliberate, and it's why the pure
logic is isolated into its own modules ([project-structure.md](../engineering/project-structure.md)) — they're the
parts worth testing, so they're separated to make testing them free.

## The shape

Not a pyramid — an hourglass, weighted at the ends:

```
   Manual QA        ██████████     the whole GUI loop, per milestone + release
   Integration      ████           pipeline with fakes; OCR against fixtures
   Unit             ██████████     pure logic: coords, assembly, batching, fitting, settings
```

The thin middle is intentional. There is no server, no database, and no multi-service integration to exercise — the
"integration" that matters is the pipeline, and it's small.

## What we test

### Unit — pure logic (the core of the suite)

Four modules carry most of the project's silent-bug risk and cost almost nothing to test:

| Module                          | What's tested                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------- |
| `capture/geometry.rs`           | Rect math, cropping, clamping, **negative virtual-desktop origins**, selections crossing monitors |
| `src/lib/coords.ts`             | CSS ↔ physical conversion at 100%/125%/150%/200%, mixed-DPI pairs, negative origin, round-tripping |
| `ocr/assemble.rs`               | Line sorting, block grouping by vertical gap, segment ↔ line mapping preserved                    |
| `translate/mod.rs` (batching)   | Segment packing, chunking at provider char limits, **mismatch detection** → `alignmentOk: false`   |
| `settings.rs`                   | Defaults, corrupt-file recovery, schema migration — old settings must never be silently dropped   |
| `src/overlay/fitText.ts`        | The shrink → wrap → clip ladder, including the 10 px legibility floor                             |

**Round-trip property tests** on coordinates: convert CSS → physical → CSS across a grid of scale factors and
origins, and assert you get back what you started with. This is a handful of lines and it closes the project's worst
bug class.

### Integration — the pipeline with fakes

The pipeline is testable without a screen or a network because `OcrEngine` and `TranslationProvider` are traits
([ADR-0002](../architecture/adr/0002-ocr-engine.md), [ADR-0003](../architecture/adr/0003-translation-provider-abstraction.md)).
That was a design goal, not a side effect.

- **`FakeOcrEngine`** returns scripted lines and boxes → tests assembly, translation, and result shaping end to end.
- **`FakeProvider`** returns scripted responses, including the ones that break things: fewer segments than sent, more
  segments, reordered, 429, 401, timeout, empty. Each must produce the right `ErrorCode` and the right degradation
  ([TD-06](../project/tech-debt.md), [FR-44](../product/requirements-functional.md#translation)).
- **Cancellation**: cancel between each stage and assert nothing is left running and the frame is dropped.
- **Same-language skip**: assert **zero** provider calls ([FR-43](../product/requirements-functional.md#translation)).
- **One request per capture**: assert the fake provider was called exactly once, whatever the line count
  ([FR-42](../product/requirements-functional.md#translation)).

### OCR accuracy — fixtures, measured not asserted

A committed fixture set of real screenshots in `src-tauri/fixtures/`, each with expected text:

| Fixture class      | Why it's there                                            |
| ------------------ | ----------------------------------------------------------- |
| Small UI text      | 11–13 px — the most common real case and the hardest        |
| Low contrast       | Grey-on-grey dialogs                                        |
| Monospace / code   | Error messages and stack traces                             |
| Prose paragraph    | The multi-line assembly case                                |
| Dark mode          | Light-on-dark inverts the pre-processing assumptions        |
| High-DPI rendering | Should be the easy case; proves nothing regressed           |
| Game-style UI      | Stylized fonts, textured backgrounds — the known weak spot  |

These are a **benchmark, not a pass/fail gate**: they report a character-accuracy score per fixture. A
pre-processing change that raises one class and drops another is exactly the trade this suite exists to make
visible. Treat a drop as a finding to explain, not automatically a failure.

### Manual — the GUI loop

Everything involving a real screen, real windows, and a real mouse: [qa-checklist.md](qa-checklist.md), run per
milestone and in full before a release. **Including at least one pass on a multi-monitor mixed-DPI machine** — the
one environment that cannot be faked and where this project's worst bugs live.

## What we deliberately don't test

Recorded so the gaps are decisions, not oversights:

| Not tested                          | Why                                                                                  |
| ----------------------------------- | -------------------------------------------------------------------------------------- |
| E2E GUI automation (WinAppDriver etc.) | Enormous setup and maintenance cost for a solo project; the failures it would catch are the loud ones a manual pass catches in seconds |
| React component rendering            | Three small windows with shallow trees; snapshot tests here mostly test that CSS didn't change |
| Real provider APIs in CI             | Needs live keys, costs money, fails for reasons unrelated to our code. Contract-shape tests run against recorded responses instead |
| `Windows.Media.Ocr` itself           | It's the OS. We test our use of it, not it                                            |
| Screen capture pixel output          | Requires a real display; covered by the manual multi-monitor check                     |

If any of these becomes a source of repeated bugs, revisit — but don't add them speculatively.

## Tools

| Layer         | Tool                                                              |
| ------------- | ------------------------------------------------------------------- |
| Rust unit     | Built-in `#[test]`; `proptest` for coordinate round-trips           |
| Rust async    | `tokio::test`                                                       |
| Rust fakes    | Hand-written trait implementations — no mocking framework needed    |
| TS unit       | Vitest                                                              |
| Fixtures      | Committed PNGs + expected-text files in `src-tauri/fixtures/`       |
| CI            | GitHub Actions on `windows-latest` ([../operations/ci-cd.md](../operations/ci-cd.md)) |

Hand-written fakes over a mocking framework: with two traits and a handful of scenarios, a fake is shorter, clearer,
and doesn't need anyone to learn a DSL.

## Coverage

**No coverage percentage target.** A number here would push effort toward the easy-to-cover React components and
away from the coordinate math, which is exactly backwards.

The real bar, enforced in [../project/definition-of-done.md](../project/definition-of-done.md):

1. Every pure-logic module listed above has tests.
2. Every fixed bug has a test that fails without the fix.
3. Every failure mode in the pipeline has a fake-driven test.

## When to write a test

| Situation                                | Test?                                                        |
| ---------------------------------------- | -------------------------------------------------------------- |
| New pure function (math, parsing, mapping) | **Yes** — always                                             |
| New pipeline behaviour or failure mode    | **Yes** — with fakes                                          |
| Bug fix                                   | **Yes** — the test must fail without the fix                  |
| Pre-processing change                     | **Yes** — run the fixture benchmark and record the delta      |
| New provider                              | **Yes** — contract shape against a recorded response          |
| New IPC command                           | Test the logic it delegates to, not the handler                |
| CSS / layout tweak                        | No — [qa-checklist.md](qa-checklist.md)                       |
| Copy change                               | No                                                            |
