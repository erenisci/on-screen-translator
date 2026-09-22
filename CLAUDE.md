# on-screen-translator — project brain

A Windows tray utility: global hotkey → freeze the screen → drag-select a region → local OCR → translate → draw the
translation back over the original text, in place, without leaving the overlay. Tauri v2, Rust core, React frontend.
MIT, open source, no backend, no telemetry.

<!-- acta:index:start -->
## How I work in this project

I work as a senior engineer wearing every hat this one-person project needs (PM, architect, full-stack, UI/UX,
DevOps, security, QA, tech lead). For any non-trivial task: **analyze → choose the simplest solution that fits the
requirements → check security & performance → define tests → update the docs below → self-review.**

- **Right-size.** Match the approach to the project's scale — never add DDD/CQRS/microservices or heavy process without a real need.
- **Decide, then justify.** For each significant decision, record why / alternatives / why-not / long-term impact / at-scale risk as an ADR.
- **Unknown stays `TBD`** — never fabricate. Docs are the source of truth; read the relevant doc before writing.
- **Recommend the next step.** At the end of a chunk / before a commit, proactively suggest the fitting move — `/acta:track` (syncs docs, and the design & legal layers if present — ticks the roadmap, updates the design-system, raises a lawyer re-review flag when legal exposure changed), `/acta:business` if pricing/cost changed — plus the right engineering practice for the change (simplify, a security review on backend/data paths, tests, an ADR). When a phase wraps in a git repo, suggest committing the phase (never auto-commit; leave pushing to me). Right place and time; never nag.
- **Jot in-flight bugs / needed changes into `SCRATCH.md`** as you notice them (🔴 blocking · 🟡 change · 🔵 minor). `/acta:track` drains it into the right docs — never hand-delete it.
- **Write docs in this project's content language** (English); talk to me in the language I use.

## Project documentation index

Engineering docs live under `docs/`. **Before working in an area, read its doc.** Keep docs current with `/acta:track`.

- **Product — what & why** → [PRD](docs/product/prd.md), [functional requirements](docs/product/requirements-functional.md), [NFRs & budgets](docs/product/requirements-nfr.md), [feature specs](docs/product/feature-specs.md), [vision](docs/product/roadmap-vision.md)
- **Project — plan & status** → [roadmap](docs/project/roadmap.md), [progress](docs/progress.md), [tech debt](docs/project/tech-debt.md), [DoD](docs/project/definition-of-done.md), [release plan](docs/project/release-plan.md)
- **Code & architecture** → [architecture overview](docs/architecture/overview.md), [system design](docs/architecture/system-design.md), [IPC contract](docs/architecture/api.md), [ADRs](docs/architecture/adr/README.md), [project structure](docs/engineering/project-structure.md), [coding standards](docs/engineering/coding-standards.md)
- **Quality & testing** → [testing strategy](docs/quality/testing-strategy.md), [QA checklist](docs/quality/qa-checklist.md), [self-review](docs/engineering/self-review-checklist.md)
- **Ops & security** → [security](docs/operations/security.md), [configuration](docs/operations/configuration.md), [error handling](docs/operations/error-handling.md), [logging](docs/operations/logging.md), [deployment](docs/operations/deployment.md)

Full index: `docs/README.md`.
<!-- acta:index:end -->

## Invariants — do not break these without an ADR

These are the load-bearing decisions. Each one is enforced somewhere in the docs above, and breaking any of them
silently is how this project fails.

1. **One coordinate space.** Core coordinates are **physical pixels in virtual-desktop space** (origin can be
   negative). CSS↔physical conversion happens in `src/lib/coords.ts` and **nowhere else**. Name every coordinate
   variable `_physical` or `_css`. → [ADR-0004](docs/architecture/adr/0004-capture-and-coordinate-model.md)

2. **Nothing runs while idle.** No timer, no watcher, no background task, no window. The process sleeps until the
   hotkey fires. This is what buys the 30 MB idle budget. → [NFR-P5](docs/product/requirements-nfr.md#performance)

3. **Capture once per session.** The whole virtual desktop is grabbed on hotkey press; every selection is a crop of
   that buffer. Never re-capture. → [ADR-0004](docs/architecture/adr/0004-capture-and-coordinate-model.md)

4. **One outbound request per capture**, carrying text only. Never one request per line.
   → [ADR-0003](docs/architecture/adr/0003-translation-provider-abstraction.md)

5. **The image never leaves the device. Nothing captured is written to disk or logged.** Not at `trace`, not for
   debugging, not temporarily. → [security.md](docs/operations/security.md), [logging.md](docs/operations/logging.md)

6. **No telemetry, analytics, or update ping.** The app makes no outbound request the user didn't initiate.

7. **Secrets only in Credential Manager**, only via `secrets.rs`. Never in settings, logs, errors, or the webview.

8. **The frontend is presentation only** — no network, no filesystem, no OS access. All `invoke` goes through
   `src/lib/ipc.ts`; Tauri command handlers contain no logic.
   → [project-structure.md](docs/engineering/project-structure.md)

9. **A translation failure must never destroy the OCR result.** The source text stays visible and copyable.
   → [error-handling.md](docs/operations/error-handling.md)

## Where the risk actually is

Four places. If this project fails technically, it fails in one of them — treat changes here with extra care:

1. **Coordinate math** — invisible on a single-monitor dev machine, broken for everyone with two.
2. **OCR accuracy on small text** — bounded by the engine ([TD-01](docs/project/tech-debt.md)).
3. **Batch segment alignment** — a provider that merges or splits segments silently breaks the headline feature
   ([TD-06](docs/project/tech-debt.md)).
4. **The keyless default provider** — the first-run experience depends on infrastructure nobody owes us
   ([TD-02](docs/project/tech-debt.md)).

## Standing rule for testing

**Anything touching capture, coordinates, or windows must be verified on a multi-monitor mixed-DPI setup before it's
done.** CI runs on a single 100%-scaled virtual display, so a green build proves nothing about this class of bug.
→ [definition-of-done.md](docs/project/definition-of-done.md)

## Current state

**Frontend scaffolded and green** (typecheck, lint, 66 tests, production build). `src-tauri/` does not exist yet:
**the Rust toolchain is not installed on this machine** — see the blocked section in
[progress.md](docs/progress.md) for the install commands. Everything in M1 that doesn't need Rust is done.

Commands that work today: `npm test`, `npm run typecheck`, `npm run lint`, `npm run build`.
`npm run tauri dev` will not work until Rust is installed.
