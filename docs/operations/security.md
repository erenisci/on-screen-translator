---
title: Security Guidelines
discipline: ops
status: active
updated: 2026-09-10
---

# Security Guidelines

> **Purpose.** The security and privacy properties this app promises, and how the code keeps them true.
> **Related.** [../product/requirements-nfr.md](../product/requirements-nfr.md) · [../architecture/overview.md](../architecture/overview.md) · [logging.md](logging.md) · [configuration.md](configuration.md)

## The threat picture

This app does three things that are, in isolation, exactly what malware does: it registers a **global keyboard
hook**, it **captures the screen**, and it **sends data to the internet**. That's not a footnote — it defines the
security posture:

1. **The user must be able to verify the promises**, not just believe them. Every guarantee below is a property of
   the architecture, checkable by reading one module.
2. **The captured content is the most sensitive thing in the system.** People will capture banking pages, private
   messages, and internal documents. It must never be written down, logged, or transmitted as an image.
3. **We are the supply chain.** An open-source Windows binary with these capabilities is an attractive thing to
   compromise. Dependencies and the release pipeline matter as much as our own code.

## The guarantees

| #  | Guarantee                                                     | Enforced by                                                       |
| -- | ------------------------------------------------------------- | ------------------------------------------------------------------- |
| G1 | The captured **image never leaves the device**                 | OCR is local; no code path sends image bytes                       |
| G2 | Only **extracted text** is transmitted, only to the user-chosen provider, only on a user-initiated action | `translate/` is the only network module |
| G3 | **Zero telemetry** — no analytics, no crash reporting, no update ping | No such dependency exists in the build                       |
| G4 | **API keys** live only in Windows Credential Manager           | `secrets.rs` is the only module that touches them                  |
| G5 | **Nothing captured is written to disk** — no image, no text    | No filesystem write in the capture or pipeline path                |
| G6 | **No elevation** required to install or run                    | Per-user install, `HKCU` autostart                                 |
| G7 | The **frontend has no network, filesystem, or OS access**      | Tauri capability config + the IPC boundary                         |

If a change would weaken any of these, it isn't a code review question — it's an
[ADR](../architecture/adr/README.md) and a README change, because these are published claims.

## AuthN / AuthZ

There is no account, no session, no server, and no multi-user model. The only credentials in the system are the
**user's own third-party API keys**, and the app's role is custodian, not authenticator.

**Key handling rules:**

- Written and read only by `secrets.rs`, via Windows Credential Manager.
- Stored per provider: `on-screen-translator/<provider>` — so switching providers doesn't destroy a key.
- **Never** in the settings JSON, a log line, an error `detail`, a panic message, or an IPC response.
- The Settings UI uses `has_provider_key` to show "configured" — the key itself never enters the webview
  ([../architecture/api.md](../architecture/api.md)). That pair of commands exists for exactly this reason.
- Cleared on user request, removing the Credential Manager entry entirely.

## Data protection

### What exists, where, and for how long

| Data                 | Where                              | Lifetime                         | Leaves device? |
| -------------------- | ---------------------------------- | -------------------------------- | -------------- |
| Captured frame       | Process memory                     | The overlay session; dropped on close | **Never**  |
| Extracted text       | Process memory                     | Until the overlay/panel closes   | **Only** to the chosen provider |
| Translated text      | Process memory + clipboard on request | Same                          | No             |
| Settings             | `%APPDATA%` JSON                   | Persistent                       | No             |
| API keys             | Windows Credential Manager         | Persistent until cleared         | **Only** to that provider as auth |
| Logs                 | `%APPDATA%` rolling file           | Bounded rotation                 | No             |

**Two rules make this table true:**

1. **No capture path writes to disk.** Not for caching, not for debugging, not temporarily. If you need a fixture,
   use `src-tauri/fixtures/`, not a runtime dump.
2. **No log line contains captured content.** Log *shapes* — line counts, character counts, durations — never text.
   See [logging.md](logging.md).

### Network egress

**One egress point:** `src-tauri/src/translate/`. Nothing else in the app opens a socket.

- TLS required; no plaintext HTTP, no certificate-validation bypass, ever.
- Requests carry extracted text and the user's key — nothing else. No machine identifier, no version ping, no
  usage counter.
- Requests happen **only** on a user-initiated capture or an explicit "test connection".
- A custom LLM endpoint URL is user-supplied and therefore user-trusted, but must still be `https://`.

This single-module design is what makes G1–G3 auditable: a reviewer reads one directory to verify what the app
sends. Adding a network call anywhere else defeats the entire privacy claim, which is why it's an
[anti-pattern](../engineering/coding-standards.md#anti-patterns), not a style preference.

### Webview hardening

The frontend is treated as untrusted presentation code:

- **Restrictive CSP** in `tauri.conf.json` — no remote origins, no `unsafe-eval`. All assets are bundled.
- **No remote content is ever loaded** into a window. Everything is local.
- **Tauri capabilities are minimal** — only the commands in [../architecture/api.md](../architecture/api.md) are
  exposed, and none of them is a general filesystem or shell primitive.
- `open_external` accepts **allowlisted URLs only** (Windows language settings, the project's own docs). An
  unrestricted "open this URL" command is a phishing primitive.
- The `otr://` protocol handler serves exactly one in-memory frame, scoped to the active session, read-only. It must
  never become a general file server ([../architecture/api.md](../architecture/api.md)).

## Dependencies

The realistic attack path against this project is a compromised dependency, not a flaw in our own logic.

- **Every new dependency needs a stated reason** ([coding-standards](../engineering/coding-standards.md)). Fewer,
  smaller, well-known crates and packages.
- `cargo audit` and `npm audit` run in CI ([ci-cd.md](ci-cd.md)); advisories are addressed, not ignored.
- Dependencies are **pinned**; lockfiles are committed. Upgrades are deliberate and reviewed, not automatic.
- Before adding one, ask: what does it pull in transitively, and does it make network calls of its own? A crate that
  phones home breaks G3 no matter how good our own code is.
- Tauri and its plugins are pinned to exact versions — they sit closest to the OS and to the IPC boundary.

## Distribution

- Releases come **only** from CI, built from a tag ([../project/release-plan.md](../project/release-plan.md)). A
  locally-built binary is never published — this is what makes "what shipped" reproducible from the repo.
- **Binaries are unsigned in v1** ([TD-03](../project/tech-debt.md)). Consequences: SmartScreen warnings and
  occasional AV quarantine, since a tray app with a keyboard hook and screen capture matches every keylogger
  heuristic. This is documented honestly in the README rather than worked around — and "work around AV detection" is
  not something this project will ever do.
- Release artifacts should carry published checksums so users can verify what they downloaded.

## Checklist for any change

Before merging anything that touches the network, the filesystem, secrets, or the webview:

- [ ] Does this add a network call outside `translate/`? → Not allowed without an ADR.
- [ ] Does this write anything captured to disk? → Not allowed.
- [ ] Could a secret reach a log, an error message, or an IPC response? → Fix before merging.
- [ ] Does this expand what the webview can do? → Justify it against G7.
- [ ] Does the new dependency phone home, or pull in something that might?
- [ ] Does this weaken a published guarantee (G1–G7)? → ADR + README update, not a silent change.

## Reporting a vulnerability

Report privately via GitHub's security advisory feature on the repository rather than a public issue. There is no
server to patch and no user data we hold, so the realistic impact of a vulnerability here is on the user's machine —
which makes a coordinated fix and a prompt release the right response.
