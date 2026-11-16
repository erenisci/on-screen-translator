# acta registry

> Agent state for the acta doc pipeline. Owned by acta:build / acta:adopt / acta:track.
> Regenerated on each run — safe to read, avoid hand-editing.

## Profile

- project: on-screen-translator
- type: desktop-app (Windows tray utility — screen OCR + translation)
- stack: Tauri v2 · Rust · React · TypeScript · Tailwind CSS · Vite · Windows.Media.Ocr
- stage: active — M1 code complete (DoD open), M2 done, M3 next
- brief: archived
- depth: standard
- language: English

## Selected disciplines

product, project, code, quality, ops

<!-- ai discipline not selected: no LLM/ML product surface. Domain packs: none — desktop-app implies no pack. -->

## Document registry

<!-- one row per generated doc; status: active | tbd | external(pre-existing, untouched) -->

| id               | path                                                        | status | updated    |
| ---------------- | ----------------------------------------------------------- | ------ | ---------- |
| readme           | README.md                                                   | active | 2026-09-10 |
| brain            | CLAUDE.md                                                   | active | 2026-11-16 |
| registry         | .claude/acta.md                                             | active | 2026-11-16 |
| docs-index       | docs/README.md                                              | active | 2026-11-16 |
| glossary         | docs/glossary.md                                            | active | 2026-09-10 |
| onboarding       | docs/onboarding.md                                          | active | 2026-09-10 |
| maintenance      | docs/maintenance.md                                         | active | 2026-09-10 |
| scratch          | SCRATCH.md                                                  | active | 2026-11-16 |
| prd              | docs/product/prd.md                                         | active | 2026-09-10 |
| req-func         | docs/product/requirements-functional.md                     | active | 2026-11-16 |
| req-nfr          | docs/product/requirements-nfr.md                            | active | 2026-09-10 |
| user-stories     | docs/product/user-stories.md                                | active | 2026-09-10 |
| feature-specs    | docs/product/feature-specs.md                               | active | 2026-11-16 |
| roadmap-vision   | docs/product/roadmap-vision.md                              | active | 2026-09-10 |
| roadmap          | docs/project/roadmap.md                                     | active | 2026-11-16 |
| progress         | docs/progress.md                                            | active | 2026-11-16 |
| changelog        | CHANGELOG.md                                                | active | 2026-11-16 |
| release-plan     | docs/project/release-plan.md                                | active | 2026-09-10 |
| dor              | docs/project/definition-of-ready.md                         | active | 2026-09-10 |
| dod              | docs/project/definition-of-done.md                          | active | 2026-09-10 |
| tech-debt        | docs/project/tech-debt.md                                   | active | 2026-11-16 |
| structure        | docs/engineering/project-structure.md                       | active | 2026-11-16 |
| coding-standards | docs/engineering/coding-standards.md                        | active | 2026-09-10 |
| naming           | docs/engineering/naming-conventions.md                      | active | 2026-09-10 |
| git-workflow     | docs/engineering/git-workflow.md                            | active | 2026-09-10 |
| self-review      | docs/engineering/self-review-checklist.md                   | active | 2026-09-10 |
| arch-overview    | docs/architecture/overview.md                               | active | 2026-09-10 |
| system-design    | docs/architecture/system-design.md                          | active | 2026-11-16 |
| api              | docs/architecture/api.md                                    | active | 2026-11-16 |
| adr              | docs/architecture/adr/README.md                             | active | 2026-11-16 |
| adr-0001         | docs/architecture/adr/0001-initial-architecture.md          | active | 2026-09-10 |
| adr-0002         | docs/architecture/adr/0002-ocr-engine.md                    | active | 2026-11-16 |
| adr-0003         | docs/architecture/adr/0003-translation-provider-abstraction.md | active | 2026-09-10 |
| adr-0004         | docs/architecture/adr/0004-capture-and-coordinate-model.md  | active | 2026-09-10 |
| adr-0005         | docs/architecture/adr/0005-ocr-test-fixtures-are-rendered.md | active | 2026-11-16 |
| testing-strategy | docs/quality/testing-strategy.md                            | active | 2026-11-16 |
| qa-checklist     | docs/quality/qa-checklist.md                                | active | 2026-11-16 |
| env-vars         | docs/operations/env-vars.md                                 | active | 2026-09-10 |
| configuration    | docs/operations/configuration.md                            | active | 2026-09-10 |
| error-handling   | docs/operations/error-handling.md                           | active | 2026-09-10 |
| logging          | docs/operations/logging.md                                  | active | 2026-11-16 |
| ci-cd            | docs/operations/ci-cd.md                                    | active | 2026-09-10 |
| deployment       | docs/operations/deployment.md                               | active | 2026-09-10 |
| rollback         | docs/operations/rollback.md                                 | active | 2026-09-10 |
| security         | docs/operations/security.md                                 | active | 2026-11-16 |

## Not generated (out of tier at depth `standard`)

<!-- Recorded so a later depth change knows what's missing, and so audit doesn't treat these as orphans. -->

- product: use-cases, business-rules, domain-model, analytics (full)
- project: risk-register (full) — risks captured in prd.md Open Questions + tech-debt.md instead
- code: rfc, diagrams (full); db-design, erd (standard, **N/A** — no database by design, see ADR-0001)
- quality: unit-testing, integration-testing, e2e-testing (full)
- ops: error-catalog, monitoring, backup, disaster-recovery, runbook, performance, scalability, caching, threat-model (full)
  — performance targets live in requirements-nfr.md; monitoring/backup/DR are N/A with no server and no persisted user data
- knowledge: contributing (full) — contribution guidance lives in README.md + onboarding.md

## Notes

- Four ADRs were seeded rather than one: the capture/coordinate model, the OCR engine, and the translation
  abstraction are each load-bearing and were decided during the build. ADR-0005 was added in M2, when the test
  fixtures turned out to need a different shape than testing-strategy.md had assumed.
- Open questions carried into the docs: keyless default provider reliability (PRD Q1 / TD-02) and code signing
  (PRD Q2 / TD-03).
- M2 moved three requirements from aspiration to measured fact, and the requirement text was corrected rather than
  the code: FR-31 (TD-11), FR-32 (TD-09), FR-33 (TD-10). If a future run finds these reworded back, the code is
  what to check first.
