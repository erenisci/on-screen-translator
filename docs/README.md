# Documentation

Every doc for on-screen-translator, grouped by discipline. Documentation is the source of truth here — read the
relevant doc before writing code in that area.
This index is **regenerated** by the acta skills — do not hand-curate the list.

**New here?** Start with [onboarding.md](onboarding.md), then [architecture/overview.md](architecture/overview.md).
**Want to know why something is the way it is?** [architecture/adr/](architecture/adr/README.md).

## Knowledge

- [Onboarding](onboarding.md) — clone to running build, plus the five things that will confuse you in the first hour
- [Maintenance](maintenance.md) — routine upkeep, dependency posture, and how to diagnose a user's bug report
- [Glossary](glossary.md) — shared vocabulary; what "block", "segment", "physical pixel" mean here

## Product — what & why

- [PRD](product/prd.md) — problem, goals, non-goals, v1 scope, success metrics, open questions
- [Functional Requirements](product/requirements-functional.md) — numbered, testable FRs with acceptance checks
- [Non-Functional Requirements](product/requirements-nfr.md) — the performance, reliability, security, and usability budgets
- [User Stories](product/user-stories.md) — the requirements from the user's side
- [Feature Specs](product/feature-specs.md) — how each v1 feature behaves: states, edge cases, decisions
- [Roadmap — Vision](product/roadmap-vision.md) — themes, Now/Next/Later, and what we will never do

## Project — plan & status

- [Execution Roadmap](project/roadmap.md) — milestones M1–M8, build order, and why that order
- [Progress](progress.md) — current status, in progress, blocked, next up
- [Changelog](../CHANGELOG.md) — user-facing history
- [Release Plan](project/release-plan.md) — versioning and the full release checklist
- [Definition of Ready](project/definition-of-ready.md) — the bar to start work
- [Definition of Done](project/definition-of-done.md) — the bar to call it finished
- [Tech Debt](project/tech-debt.md) — accepted risks and shortcuts, each with its reason

## Code & architecture

- [Architecture Overview](architecture/overview.md) — components, data flow, tech stack, where the risk lives
- [System Design](architecture/system-design.md) — how each budget is met, the mechanics, the trade-offs
- [IPC Contract](architecture/api.md) — the Tauri command and event surface between core and frontend
- [Architecture Decision Records](architecture/adr/README.md) — the four load-bearing decisions and their alternatives
- [Project Structure](engineering/project-structure.md) — the layout and a "where does this code go?" table
- [Coding Standards](engineering/coding-standards.md) — patterns, and the project-specific anti-patterns
- [Naming Conventions](engineering/naming-conventions.md) — files, types, branches, commits, coordinate variables
- [Git Workflow](engineering/git-workflow.md) — branching, commits, merging, tags
- [Self-Review Checklist](engineering/self-review-checklist.md) — the pass to make on your own diff

## Quality & testing

- [Testing Strategy](quality/testing-strategy.md) — what we test, what we deliberately don't, and why
- [QA Checklist](quality/qa-checklist.md) — the manual pass, including the multi-monitor mixed-DPI run

## Ops & security

- [Security Guidelines](operations/security.md) — the seven guarantees and how the code keeps them
- [Configuration](operations/configuration.md) — settings schema, defaults, validation, migration
- [Environment Variables](operations/env-vars.md) — dev and CI only; the shipped app reads none
- [Error Handling](operations/error-handling.md) — how failures propagate and what the user sees
- [Logging](operations/logging.md) — what we log, and what must never be logged
- [CI/CD](operations/ci-cd.md) — the check and release workflows, and what CI can't verify
- [Deployment](operations/deployment.md) — artifacts, prerequisites, install, and updating
- [Rollback](operations/rollback.md) — what to do when a release is bad

## The short version

If you read only four things:

1. **[architecture/overview.md](architecture/overview.md)** — how it fits together
2. **[ADR-0004](architecture/adr/0004-capture-and-coordinate-model.md)** — the coordinate model, which is where the
   project's worst bugs come from
3. **[product/requirements-nfr.md](product/requirements-nfr.md)** — the budgets that constrain every design choice
4. **[../CLAUDE.md](../CLAUDE.md)** — the invariants that must not be broken without an ADR

---

- **Start a project:** `/acta:init` → fill `<project>_brief.md` → `/acta:build`
- **After finishing work:** `/acta:track` (updates docs to current state)
- **Existing codebase without docs:** `/acta:adopt`
