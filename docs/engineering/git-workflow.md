---
title: Git Workflow
discipline: code
status: active
updated: 2026-09-10
---

# Git Workflow

> **Purpose.** How changes get into `main` and out to a release. Right-sized for one maintainer with occasional
> outside contributors.
> **Related.** [naming-conventions.md](naming-conventions.md) · [self-review-checklist.md](self-review-checklist.md) · [../project/release-plan.md](../project/release-plan.md) · [../operations/ci-cd.md](../operations/ci-cd.md)

> **Status:** the repository is not yet initialized. `git init` + first commit is part of scaffolding
> ([M1](../project/roadmap.md)).

## Branching

**Trunk-based, with short-lived branches.** No `develop`, no release branches, no GitFlow — for a solo project with
no parallel release trains, that ceremony costs more than it returns.

```
main ──●──────●──────●──────●──── (always releasable, always tagged from here)
        \    /        \    /
         ●──●          ●──●        feat/… fix/… — hours to days, never weeks
```

- **`main` is always releasable.** Anything merged has passed CI and the
  [DoD](../project/definition-of-done.md).
- **Branch per change**, named per [naming-conventions.md](naming-conventions.md): `feat/in-place-overlay`,
  `fix/dpi-offset-secondary-monitor`.
- **Short-lived.** If a branch lives longer than a few days, it's too big — split it. Long branches on a
  single-person project are just uncommitted work with extra steps.
- **Never commit directly to `main`**, even solo. The branch is what makes the change reviewable and revertable as
  a unit.

## Commit convention

[Conventional Commits](https://www.conventionalcommits.org/) — `<type>(<scope>): <subject>`. Types, scopes, and
subject rules are in [naming-conventions.md](naming-conventions.md).

**One logical change per commit.** A commit that fixes a bug, renames a module, and bumps a dependency is three
commits pretending to be one, and it makes `git bisect` useless — which matters here, because the bugs this project
will actually have (a coordinate offset, an OCR regression) are exactly the kind you find by bisecting.

**The body explains why.** The diff already shows what changed:

```
fix(capture): use virtual-desktop origin instead of primary-monitor bounds

Selections on a monitor positioned left of the primary produced negative
X, which was being clamped to 0 — so the crop came from the wrong screen.

Virtual-desktop bounds are the correct origin and are already what
grab.rs captures into, so this also removes a coordinate space that
should never have existed.

Refs ADR-0004.
```

Reference an ADR, FR, or issue when one applies. Six months later that reference is the only thing that explains the
change.

**Never commit:** secrets or API keys, `SCRATCH.md`, `.env`, build output, `types.gen.ts` conflicts resolved by
hand, or a commented-out block "just in case" — that's what history is for.

## Merging

**Solo work:** open a PR anyway when the change is non-trivial. The PR is where CI runs and where you re-read your
own diff with fresh eyes — [self-review-checklist.md](self-review-checklist.md) exists for that moment.

**Outside contributions:** always a PR, always reviewed against the checklist, always CI-green before merge.

**Squash merge by default.** One branch becomes one commit on `main`, so history reads as a sequence of complete
changes rather than "wip", "fix typo", "actually fix it". Use a merge commit only when the individual commits are
each meaningful and worth keeping (a large refactor built in reviewable steps).

**Before merging:**

- [ ] CI green — `clippy -D warnings`, `fmt --check`, `tsc`, ESLint, tests
- [ ] [DoD](../project/definition-of-done.md) satisfied
- [ ] Anything touching capture, coordinates, or windows was tried on a **multi-monitor mixed-DPI setup**
- [ ] Docs updated in the same change (`/acta:track`), not deferred
- [ ] `CHANGELOG.md` `[Unreleased]` updated if a user would notice

**Rebase, don't merge, to update a branch.** `git pull --rebase origin main` keeps history linear and bisectable.
Force-push to your own feature branch is fine; force-push to `main` never is.

## Tags & releases

- Tags are `v<semver>` on `main` only: `v0.1.0`.
- **The tag is the release trigger** — pushing it starts the CI build that produces the MSI and portable exe
  ([../operations/ci-cd.md](../operations/ci-cd.md)). Users only ever receive CI-built artifacts.
- The version in `tauri.conf.json` must match the tag exactly. That check is first on the
  [release checklist](../project/release-plan.md).
- Tags are never moved or deleted once pushed. A mistake gets a new patch version, not a rewritten tag.

## History hygiene

- **Never rewrite pushed `main` history.** Once it's public, it's permanent.
- Rewriting your own unpushed or feature-branch history is encouraged — clean up before the PR, not after.
- **Revert, don't delete.** A bad change on `main` gets `git revert`, which keeps the record of what was tried.
- **Commit at phase boundaries.** When a milestone chunk is done and the docs are synced, commit it as a coherent
  phase — history should advance alongside the docs, milestone by milestone.

## What isn't in this workflow

Deliberately absent, so nobody adds them out of habit: no `develop` branch, no release branches, no changesets bot,
no semantic-release automation, no PR templates, no CODEOWNERS. One maintainer and a handful of contributors don't
need any of it, and [NFR-C5](../product/requirements-nfr.md#constraints) says right-size everything. Revisit if the
project ever has regular contributors — not before.
