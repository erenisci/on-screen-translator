---
title: Deployment Guide
discipline: ops
status: active
updated: 2026-09-10
---

# Deployment Guide

> **Purpose.** How the app gets onto a user's machine. For a desktop tool, "deployment" is distribution and install.
> **Related.** [ci-cd.md](ci-cd.md) · [rollback.md](rollback.md) · [../project/release-plan.md](../project/release-plan.md) · [../maintenance.md](../maintenance.md)

## Environments

There is no server, so there are no runtime environments — only build configurations and one distribution channel.

| Name              | Built by                              | Who gets it            |
| ----------------- | ------------------------------------- | ---------------------- |
| **Dev**           | `npm run tauri dev`                   | The developer          |
| **Local release** | `npm run tauri build` locally         | The developer, for pre-tag verification |
| **CI release**    | GitHub Actions, from a `v*` tag       | **Users — only this**  |

**Rule: users never receive a locally-built binary.** If it wasn't built by CI from a tag, it isn't a release. This
is what makes "what shipped" reproducible from the repository alone, and it's the one distribution rule worth being
strict about in an open-source project where trust in the binary is the whole game
([security.md](security.md#distribution)).

## Artifacts

| Artifact                              | Purpose                                                        |
| ------------------------------------- | ---------------------------------------------------------------- |
| `on-screen-translator_<ver>_x64.msi`  | Standard install. Per-user, no admin rights.                    |
| `on-screen-translator_<ver>_x64.exe`  | Portable. Runs from a folder; settings still in `%APPDATA%`.     |
| `SHA256SUMS.txt`                      | Checksums so users can verify what they downloaded.              |

**The portable build matters more than usual here.** Users in restricted environments — exactly the corporate
machines where an untranslated foreign-language dialog is most likely to appear — often can't install anything.

## Prerequisites

**On the user's machine:**

| Requirement          | Notes                                                                 |
| -------------------- | ----------------------------------------------------------------------- |
| Windows 10 1809+ / 11 | Below 1809 there is no `Windows.Media.Ocr` ([ADR-0002](../architecture/adr/0002-ocr-engine.md)) |
| x64                  | ARM64 is not a v1 target ([NFR-C6](../product/requirements-nfr.md#constraints)) |
| WebView2 runtime     | Present on Windows 11 and most Windows 10; the MSI handles a missing runtime |
| OCR language pack    | For the source language, if not already installed. Detected and guided ([FR-34](../product/requirements-functional.md#ocr)) |
| No admin rights      | Deliberately not required                                              |

**WebView2 is the prerequisite most likely to surprise someone**, which is why the clean-machine test in
[../project/release-plan.md](../project/release-plan.md) specifically requires a VM without it pre-installed.

## Steps

**Releasing** (the maintainer):

1. Complete the [release checklist](../project/release-plan.md) through the Verify section.
2. Tag `v<version>` on `main` and push the tag.
3. CI builds and creates a **draft** release with artifacts and checksums ([ci-cd.md](ci-cd.md)).
4. Download the artifacts and run the clean-machine test — install, hotkey, translate, on a VM with a different
   display language.
5. Publish the draft release.

**Installing** (the user):

1. Download the MSI (or the portable exe) from GitHub Releases.
2. **Expect a SmartScreen warning** — the binary is unsigned in v1 ([TD-03](../project/tech-debt.md)). "More info" →
   "Run anyway". This is documented in the README rather than hidden.
3. Run the installer. No admin prompt.
4. The app starts in the tray. Press `Ctrl+Shift+T`.
5. Optionally: enable start-with-Windows, choose a target language, add a provider key.

## Verification

After publishing, on a machine that is not the dev box:

- [ ] Both artifacts download and their checksums match
- [ ] MSI installs without admin rights
- [ ] Installs on a machine **without** WebView2
- [ ] Tray icon appears; no taskbar entry; no console window
- [ ] Hotkey → capture → translation works with **zero configuration**
- [ ] Target language defaults to that machine's display language
- [ ] Portable exe runs standalone
- [ ] Uninstall removes the app, the tray icon, and the autostart entry
- [ ] Settings from a previous version still load ([configuration.md](configuration.md#migration))

The last item is the one that's easy to forget and expensive to get wrong: silently discarding a user's settings on
upgrade is the kind of bug that loses users permanently.

## Updating

**There is no auto-updater in v1** ([FR-96 deferred](../product/requirements-functional.md#deferred-p2)). Users
update by downloading a new release. The app makes **no update check** — that would be an outbound request the user
didn't ask for, which [G3](security.md#the-guarantees) rules out.

Consequence: users won't know when a version ships. Accepted for v1 — the alternative is either a phone-home or a
notification system, and neither is worth breaking the zero-outbound-request promise for at this stage. If
auto-update is ever added, it needs an ADR covering how the check is disclosed and made optional, plus artifact
signing ([env-vars.md](env-vars.md)).

**Upgrading in place:** the MSI replaces the installation. `settings.json` in `%APPDATA%` is untouched and migrated
on load if the schema changed. Credential Manager entries are untouched.

**Uninstalling** removes the app and the autostart entry. It does **not** remove `%APPDATA%\on-screen-translator\`
(settings and logs) or Credential Manager entries — so a reinstall restores the user's configuration. Both are
documented in [../maintenance.md](../maintenance.md) for anyone who wants a clean removal.

## What isn't here

No app store, no winget/Chocolatey manifest, no MSIX, no enterprise deployment story, no silent-install
documentation. GitHub Releases is the whole distribution channel for v1. Package-manager submissions are worth doing
once the project has users who ask for them — not before, since each one is a manifest to keep in sync with every
release.
