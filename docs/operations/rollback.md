---
title: Rollback Strategy
discipline: ops
status: active
updated: 2026-09-10
---

# Rollback Strategy

> **Purpose.** What to do when a release turns out to be bad. A desktop app can't be rolled back centrally, so the
> strategy is mostly about what we prepare *before* that happens.
> **Related.** [deployment.md](deployment.md) · [../project/release-plan.md](../project/release-plan.md) · [configuration.md](configuration.md)

## The core constraint

**Once a user has installed a version, we cannot take it back.** There is no server to revert, no feature flag to
flip, and no auto-updater to push a fix ([deployment.md](deployment.md#updating)). A bad release stays on every
machine that installed it until each user chooses to act.

That single fact drives everything below: rollback is 90% preparation and 10% reaction.

## When

Pull a release when it's **broken enough that installing it is worse than not having it**:

| Severity                                                    | Action                                         |
| ----------------------------------------------------------- | ------------------------------------------------ |
| Data loss — settings discarded or corrupted on upgrade       | **Pull immediately**, then patch                |
| The app doesn't start, or the hotkey never works             | **Pull immediately**, then patch                |
| Captured content leaked to a log or disk                     | **Pull immediately** — a security fix, treat as urgent |
| Crash on a common configuration (e.g. any multi-monitor setup) | Pull, then patch                              |
| A feature is broken but the core loop works                  | Don't pull; patch soon                          |
| Accuracy regression on some text                             | Don't pull; fix in the next release             |

"Pull" means un-publishing the release so nobody new installs it — not deleting it, which would break anyone who
scripted a download.

## How

### Pulling a bad release

1. **Mark the GitHub Release as a pre-release** (or delete the assets, keeping the tag and notes). New visitors then
   land on the previous good version.
2. **Edit the release notes** to say plainly what's wrong, who is affected, and what to do — the notes are the only
   channel we have to reach someone who already installed it.
3. **Open a pinned issue** describing the problem and the workaround.
4. **Never delete or move the tag.** It's the record of what was built ([../engineering/git-workflow.md](../engineering/git-workflow.md)).

### The user's rollback path

This is the path we owe them, and it must always work:

1. Download the previous version from GitHub Releases — **every past release stays downloadable**, which is why we
   un-publish rather than delete.
2. Install over the bad version, or uninstall and reinstall.
3. **Their settings must still work.** This is the load-bearing requirement of the whole strategy.

### Fixing forward

Usually the better answer. Patch, tag `v<x.y.z+1>`, run the full
[release checklist](../project/release-plan.md), publish. A desktop patch reaches users at the same speed a rollback
does — they have to download something either way — so a fix is generally worth more than a revert.

The exception is a security issue: pull first, patch second, so nobody new installs the vulnerable build while the
fix is being made.

## Data considerations

The only state that survives an install is the user's, which makes it the only thing a rollback can actually damage:

| State                        | Location                    | Rollback risk                                        |
| ---------------------------- | --------------------------- | ------------------------------------------------------ |
| `settings.json`              | `%APPDATA%`                 | **The real risk** — see below                         |
| Credential Manager entries   | Windows                     | None. Format is stable and version-independent         |
| Logs                         | `%APPDATA%`                 | None                                                  |
| Captured content             | Memory only                 | None — nothing is persisted ([security.md](security.md)) |

### The one rule that makes rollback survivable

> **A newer version must never write a settings file that an older version destroys.**

If v0.3 migrates `schemaVersion` 1 → 2 and the user rolls back to v0.2, v0.2 sees a version it doesn't understand.
It must **not** overwrite the file with defaults. Per [configuration.md](configuration.md#validation--recovery), an
unknown-newer schema loads read-only defaults and warns — the user's real configuration stays intact on disk, so
rolling forward again restores it.

Backing this up:

- Migrations write `settings.v<n>.bak` before changing anything, so the pre-migration file is recoverable.
- A corrupt file is renamed to `settings.corrupt.json`, never deleted.
- Unknown fields are preserved on write, so an older version doesn't strip a newer version's settings.

These three behaviours are cheap to implement and they are the entire difference between "roll back and continue" and
"roll back and reconfigure everything".

## Preparation checklist

Verified before each release, because reacting well depends entirely on having done these:

- [ ] Every previous release is still downloadable
- [ ] Settings from the previous version load correctly in the new one
- [ ] Settings from the new version don't break the previous one (**test the downgrade**, not just the upgrade)
- [ ] Migrations write a backup before running
- [ ] Release notes state the version this one replaces

The downgrade test is the one people skip. It's also the one that matters here, because it's the only thing standing
between a bad release and a user losing their configuration.

## After a pulled release

- Add a `CHANGELOG.md` entry for the pulled version explaining what happened and that it was withdrawn — history is
  append-only, so the record stays ([../../CHANGELOG.md](../../CHANGELOG.md)).
- Record the root cause in [../project/tech-debt.md](../project/tech-debt.md) if it points at something structural.
- Add whatever check would have caught it to [../quality/qa-checklist.md](../quality/qa-checklist.md) or the test
  suite. A bad release that doesn't improve the checklist will happen again.
