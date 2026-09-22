---
title: "ADR 0003: Pluggable translation providers, user-supplied keys, one batched request"
discipline: code
status: Accepted
date: 2026-09-10
---

# ADR 0003: Pluggable translation providers, user-supplied keys, one batched request

## Status

Accepted — 2026-09-10

## Context

Translation is the one part of the pipeline we cannot do well locally in v1, and the one part that costs money. The
constraints pull hard against each other:

- **Zero server cost, zero shipped key** ([NFR-C3](../../product/requirements-nfr.md#constraints),
  [NFR-C4](../../product/requirements-nfr.md#constraints)). No backend of ours, and no key embedded in an
  open-source binary — an embedded key in a public repo is a key that gets scraped and abused within days.
- **Zero-config first run** ([NFR-U1](../../product/requirements-nfr.md#usability)). Install → hotkey → translation,
  with no account and no key. This directly contradicts the point above, and the tension is the whole problem.
- **Latency.** Translation is the only network hop in the pipeline and the largest share of the
  [1s budget](../../product/requirements-nfr.md#performance).
- **Privacy.** Only extracted text is transmitted, only to the provider the user chose
  ([NFR-S1/S2](../../product/requirements-nfr.md#security)).
- **Per-line boxes must survive.** The in-place overlay needs to map translated segments back onto OCR bounding
  boxes ([F7](../../product/feature-specs.md)) — but translating line-by-line destroys the sentence context that
  makes translation good, and multiplies round trips.

Options considered:

**A. One hardcoded provider.** Simplest. But whichever we pick, we either ship a key (unacceptable) or force every
user through that vendor's signup (kills zero-config), and we inherit its outages and language coverage permanently.

**B. Our own proxy backend.** Solves zero-config elegantly — until someone has to pay for it and keep it up. Violates
[NFR-C3](../../product/requirements-nfr.md#constraints) and makes the maintainer a service operator. It also breaks
the privacy story: text would flow through us, and "trust us, we don't log it" is exactly the claim this project
exists to avoid making.

**C. Unofficial free endpoints** (the undocumented Google Translate web endpoints many tools use). Free and
keyless. But they're undocumented, terms-violating, break without notice, and IP-block aggressively. Shipping that
in an open-source tool that asks users to trust it is indefensible.

**D. A provider trait with several implementations, user-supplied keys, and a keyless default.** Users who care
about quality bring a DeepL or Google key; users who just want it to work get the keyless path. No key ships, no
server exists, and provider outages degrade rather than kill.

On request shape, the choice was per-line requests (N round trips, no context, perfect box alignment) versus one
batched request (1 round trip, full context, reconstructed alignment). At 10–30 lines per capture, per-line requests
cost 10–30× the latency budget. Batching wins on the numbers; the alignment risk is real and gets an explicit
fallback rather than being wished away.

## Decision

**A `TranslationProvider` trait with four v1 implementations, keys supplied by the user and stored in Windows
Credential Manager, and exactly one batched request per capture.**

```rust
#[async_trait]
pub trait TranslationProvider: Send + Sync {
    /// Translate segments in ONE request. Returns exactly as many segments as it received.
    async fn translate(
        &self,
        segments: &[String],
        source: Option<LangTag>,
        target: LangTag,
    ) -> Result<TranslationBatch, TranslateError>;

    /// Validate credentials and reachability for the Settings "test connection" action.
    async fn health_check(&self) -> Result<(), TranslateError>;

    fn requires_key(&self) -> bool;
    fn max_chars_per_request(&self) -> usize;
}
```

**v1 implementations:** `LibreTranslate` (keyless default), `DeepL`, `Google Cloud Translate`, `LlmEndpoint`
(any OpenAI-compatible chat endpoint, including Claude via its API — for users who want context-aware translation of
technical text).

**Batching contract.** All lines of one capture go in a single call
([FR-42](../../product/requirements-functional.md#translation)). Segments are sent as a structured array where the
provider supports it, and as a delimited payload where it doesn't. The provider **must** return the same number of
segments. If it doesn't, we do not guess an alignment — we fall back to panel-only display for that capture
([F7](../../product/feature-specs.md), [TD-06](../../project/tech-debt.md)). A wrong translation on the wrong button
is worse than no overlay.

**Keys** go in Windows Credential Manager under a per-provider entry, never in the settings JSON and never in logs
([FR-62](../../product/requirements-functional.md#settings--persistence)). Per-provider entries mean switching
providers and back doesn't require re-entering a key.

**All requests originate in the Rust core.** The frontend never talks to the network
([NFR-S5](../../product/requirements-nfr.md#security)) — one auditable egress point.

**Failures are typed and actionable**, and never destroy work: on any provider failure the OCR source text stays on
screen and copyable ([FR-44](../../product/requirements-functional.md#translation)). Rate limits, invalid keys, and
network failures each get their own message ([../../operations/error-handling.md](../../operations/error-handling.md)).

**Same-language captures skip the network entirely** ([FR-43](../../product/requirements-functional.md#translation)).

## Consequences

**Easier**

- Zero cost to run, forever, with no key in the repository.
- Users who want quality can have it (DeepL, or an LLM for technical text) without us picking for them.
- A provider outage is a degraded capture, not a dead app.
- One egress point makes the privacy claim verifiable by reading one module.
- Adding a provider is one `impl` plus a Settings entry.

**Harder**

- **The zero-config path is the weak link.** Public LibreTranslate instances rate-limit hard and vanish. A first-run
  user could hit failure on their very first capture — the worst possible moment. This is the largest open risk in
  the project: [TD-02](../../project/tech-debt.md), decision owed by M3
  ([PRD Q1](../../product/prd.md#open-questions)).
- **Four implementations, four sets of quirks** — auth shapes, language-code dialects (`pt-BR` vs `pt`), character
  limits, and error formats. Each needs its own tests against a recorded response.
- **Batch alignment is a real failure mode**, not a theoretical one ([TD-06](../../project/tech-debt.md)). The
  fallback protects correctness but silently degrades the headline feature, so it needs tests with a deliberately
  misbehaving fake provider.
- **Settings gains a provider section**, which is the single largest complexity addition to a window that
  [NFR-U3](../../product/requirements-nfr.md#usability) wants small.
- **Credential Manager is Windows-specific**, adding another item to a future port.

**At scale**

The trait absorbs new providers indefinitely, which is the axis this grows on. It also absorbs the endgame: a
**local offline model** ([Later](../../product/roadmap-vision.md)) is just another `TranslationProvider` whose
`requires_key()` is false and whose latency profile is different — the abstraction was chosen partly because it
makes that future arrive without an architectural change. What it does not absorb is live mode, where a request per
capture becomes a request per frame; that needs caching and debouncing that this design deliberately doesn't have.
