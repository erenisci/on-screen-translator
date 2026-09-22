---
title: "ADR 0002: OCR via Windows.Media.Ocr behind an engine trait"
discipline: code
status: Accepted
date: 2026-09-10
---

# ADR 0002: OCR via Windows.Media.Ocr behind an engine trait

## Status

Accepted — 2026-09-10

## Context

OCR is the accuracy ceiling of this product. Everything downstream — translation quality, the in-place overlay,
whether the user trusts the tool — is bounded by what the engine returns. The requirements that constrain the choice:

- **Local execution is mandatory.** [NFR-S1](../../product/requirements-nfr.md#security) says the captured image
  never leaves the device. That rules out every cloud OCR API outright, regardless of accuracy.
- **Speed.** [NFR-P3](../../product/requirements-nfr.md#performance) budgets under 200 ms p95 for crop +
  pre-process + recognize on a region up to 800×400.
- **Size.** [NFR-P7](../../product/requirements-nfr.md#performance) caps the whole install at 20 MB.
- **Bounding boxes per line are required**, not optional — the in-place overlay
  ([F7](../../product/feature-specs.md)) is the headline feature and it is nothing without positions.
- **Multi-language**, since the source language is arbitrary and auto-detected
  ([FR-33](../../product/requirements-functional.md#ocr)).

Options considered:

**A. Tesseract (bundled).** The obvious open-source default. Mature, ~100 languages, gives word and line boxes,
fully offline. But: each language's `tessdata` is 10–15 MB (`tessdata_best` more), so even three languages blow the
20 MB install budget several times over. Accuracy on small anti-aliased UI text — precisely our use case — is
mediocre without careful pre-processing, and it's slower than the native option on typical regions. Excellent as a
*second* engine, wrong as the only one.

**B. Cloud OCR (Google Vision, Azure CV, AWS Textract).** Best accuracy available. Disqualified by
[NFR-S1](../../product/requirements-nfr.md#security) — it requires uploading the image, which is the one thing this
product promises never to do. Also adds a network round trip inside the latency budget and a per-user cost against
[NFR-C3](../../product/requirements-nfr.md#constraints).

**C. PaddleOCR / EasyOCR / an ONNX model.** Strong accuracy, especially on CJK. But this means shipping a Python
runtime or an ONNX runtime plus model weights — tens to hundreds of megabytes, a heavy cold start, and a packaging
burden a solo maintainer pays for at every release. Wrong shape for a tray utility.

**D. `Windows.Media.Ocr` — the OCR engine built into Windows.** Present since Windows 10 1809. Zero bytes added to
our binary. Hardware-accelerated by the OS. Returns lines with bounding boxes and per-word geometry. Language
support comes from Windows language packs the user already has or can install through Settings. Accuracy on screen
text — clean, rendered, high-DPI glyphs, which is *all* we ever feed it — is genuinely good, because that is the
content class it was tuned for. Weaknesses: no CJK vertical text, fewer languages than Tesseract, no control over
the model, and Windows-only.

The decisive framing: our input is never a photograph. It is always rendered screen text. The engine optimized for
photographs (Tesseract's general model) has no advantage here, while the engine that ships free with the OS costs us
nothing in size, nothing in cold start, and nothing in packaging.

## Decision

**Use `Windows.Media.Ocr` (via the official `windows` crate's WinRT bindings) as the v1 engine, behind an
`OcrEngine` trait that makes a second engine an additive change.**

```rust
pub trait OcrEngine: Send + Sync {
    /// Recognize text in a pre-processed image.
    fn recognize(&self, image: &GrayImage, hint: Option<LangTag>) -> Result<OcrResult, OcrError>;
    /// Languages this engine can currently recognize on this machine.
    fn available_languages(&self) -> Result<Vec<LangTag>, OcrError>;
}
```

`OcrResult` carries `lines: Vec<OcrLine>` where each line has `text`, `bbox` in frame coordinates, and
`confidence` — the contract the in-place overlay depends on
([FR-32](../../product/requirements-functional.md#ocr)).

**Pre-processing lives outside the engine**, in a shared stage: upscale small regions, grayscale, contrast
normalize, adaptive threshold when contrast is low
([FR-31](../../product/requirements-functional.md#ocr), [F5](../../product/feature-specs.md)). Keeping it out of the
engine means both engines benefit from tuning it, and it can be measured independently.

**Missing language packs are a first-class, actionable state**, not an error string: detect via
`available_languages()`, tell the user which pack is missing, and link to the Windows language setting
([FR-34](../../product/requirements-functional.md#ocr)).

## Consequences

**Easier**

- Zero megabytes and zero cold-start cost for OCR — the 20 MB install budget survives.
- Fast enough to hit the 200 ms budget without optimization work.
- No model files to ship, version, license-audit, or update.
- Bounding boxes come for free in exactly the shape the overlay needs.
- Adding Tesseract later is one `impl OcrEngine`, not a refactor.

**Harder**

- **Language coverage is Microsoft's, not ours.** A user wanting a language Windows doesn't support is simply
  stuck in v1. This is our most likely user complaint, tracked as [TD-01](../../project/tech-debt.md) and slated for
  M8.
- **Accuracy is not tunable.** When it misreads stylized game text, we have no knobs — only better pre-processing.
- **Windows-only.** OCR becomes a porting task rather than shared code, though the trait contains the damage
  ([NFR-M2](../../product/requirements-nfr.md#maintainability)).
- **WinRT interop** brings async COM-shaped APIs into an otherwise plain Rust core; the interop is confined to
  `ocr/windows_ocr.rs` so it doesn't leak.
- **Testing needs fixtures, not a screen.** The trait is what makes that possible: pipeline tests run against a
  fake engine, and engine tests run against fixture images
  ([../../quality/testing-strategy.md](../../quality/testing-strategy.md)).

**At scale**

The trait is the escape hatch, and it's deliberately narrow so it stays cheap to implement. Adding Tesseract for
missing languages, or an ONNX model for CJK, means an implementation plus a Settings entry — no downstream change.
If OCR ever needs to run per-frame rather than per-capture (live mode), this decision needs revisiting: an engine
choice tuned for one 200 ms call is not automatically right for 10 calls a second.
