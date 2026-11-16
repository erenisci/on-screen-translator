//! OCR: the engine trait, the result types, and the stage that turns engine
//! output into something a translator can use.
//!
//! ADR-0002 picks `Windows.Media.Ocr` as the v1 engine and puts it behind this
//! trait so a second engine (Tesseract for the languages Microsoft doesn't
//! ship, [TD-01]) is an additive change rather than a refactor. Two things stay
//! deliberately *outside* the engine:
//!
//! - **pre-processing** lives in `crate::preprocess`, so every engine gets the
//!   same ladder and it can be measured on its own;
//! - **line assembly** lives in `assemble`, pure and tested, because paragraph
//!   structure is what makes a translation read like prose instead of a pile
//!   of fragments.
//!
//! [TD-01]: ../../../docs/project/tech-debt.md

pub mod assemble;
pub mod windows_ocr;

use serde::{Deserialize, Serialize};

use crate::capture::Rect;
use crate::preprocess::{GrayImage, Preprocessed};

/// A box exactly as an engine reports it: **processed-image** pixels, which
/// are neither screen pixels nor CSS pixels. `Preprocessed::to_source` is the
/// only thing allowed to turn one into a screen rectangle (invariant 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ImageBox {
    pub fn union(self, other: ImageBox) -> ImageBox {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.w).max(other.x + other.w);
        let bottom = (self.y + self.h).max(other.y + other.h);
        ImageBox {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        }
    }
}

/// One line straight out of an engine, before assembly.
#[derive(Debug, Clone)]
pub struct RawLine {
    pub text: String,
    pub bbox: ImageBox,
    /// `None` when the engine doesn't report one — which is the case for the
    /// only engine we currently have (TD-09). Deliberately not defaulted to
    /// 1.0: a fabricated certainty is worse than an admitted unknown.
    pub confidence: Option<f32>,
}

/// What an engine returns, plus which language it actually recognized with.
#[derive(Debug)]
pub struct EngineOutput {
    pub lines: Vec<RawLine>,
    /// BCP-47, as the engine reports it (e.g. `en-US`, `zh-Hans-CN`). Kept
    /// whole rather than reduced to a primary subtag, because the script
    /// matters for some languages and dropping it is lossy.
    pub language: String,
}

/// The seam a second OCR engine plugs into (ADR-0002).
pub trait OcrEngine: Send + Sync {
    /// Recognize text in an already pre-processed image. `language` is the
    /// user's override; `None` means "use what this machine is set up for".
    fn recognize(
        &self,
        image: &GrayImage,
        language: Option<&str>,
    ) -> Result<EngineOutput, OcrError>;

    /// What this machine can actually recognize right now (FR-34).
    fn available_languages(&self) -> Result<Vec<String>, OcrError>;

    /// For logs and bug reports. Never user-facing copy.
    fn name(&self) -> &'static str;
}

/// One recognized line, positioned in virtual-desktop space.
///
/// Mirrors `OcrLine` in `src/lib/types.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrLine {
    pub text: String,
    /// Physical pixels, virtual-desktop space. The overlay draws here.
    pub bbox: Rect,
    /// `None` when the engine doesn't report one (TD-09).
    pub confidence: Option<f32>,
    /// Which paragraph this line belongs to. Lines sharing a `block_id` were
    /// joined into one unit of `full_text`.
    pub block_id: u32,
}

/// The output of the whole OCR stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub lines: Vec<OcrLine>,
    /// Lines joined into paragraphs: blocks separated by a blank line, lines
    /// inside a block joined with a space. This is what gets translated —
    /// translation quality depends on sentences, not fragments.
    pub full_text: String,
    pub source_lang: String,
    pub engine: String,
}

impl OcrResult {
    pub fn block_count(&self) -> usize {
        self.lines
            .last()
            .map(|l| l.block_id as usize + 1)
            .unwrap_or(0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("no OCR language pack installed for {requested}")]
    LanguageUnavailable { requested: String },

    #[error("no OCR language pack is installed for any of the user's languages")]
    NoLanguages,

    #[error("image is {width}x{height}, engine maximum is {max}")]
    ImageTooLarge { width: i32, height: i32, max: i32 },

    #[error("the engine recognized nothing")]
    NoText,

    #[error("winrt {call} failed: {detail}")]
    Engine { call: &'static str, detail: String },
}

/// The engine this build uses.
pub fn default_engine() -> impl OcrEngine {
    windows_ocr::WindowsOcr
}

/// Run an engine over a pre-processed crop and assemble the result.
///
/// Note what this function does *not* do: it never touches the screen, never
/// re-captures, and never logs a character of what it read (invariant 5).
pub fn recognize(
    engine: &dyn OcrEngine,
    prepared: &Preprocessed,
    language: Option<&str>,
) -> Result<OcrResult, OcrError> {
    let output = engine.recognize(&prepared.image, language)?;

    let positioned: Vec<assemble::PositionedLine> = output
        .lines
        .into_iter()
        .filter_map(|line| {
            let text = line.text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            // The one place engine coordinates become screen coordinates.
            let bbox = prepared.to_source(line.bbox.x, line.bbox.y, line.bbox.w, line.bbox.h);
            if bbox.is_empty() {
                return None;
            }
            Some(assemble::PositionedLine {
                text,
                bbox,
                confidence: line.confidence,
            })
        })
        .collect();

    // FR-35: an empty result is a normal thing to do with this tool, not a
    // failure, and the caller turns it into a quiet message.
    if positioned.is_empty() {
        return Err(OcrError::NoText);
    }

    let (lines, full_text) = assemble::assemble(positioned);

    Ok(OcrResult {
        lines,
        full_text,
        source_lang: output.language,
        engine: engine.name().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocess::{GrayImage, Step};

    /// An engine that returns whatever it was handed. The point of the trait:
    /// the whole stage is testable with no screen and no language pack
    /// (quality/testing-strategy.md).
    struct FakeEngine {
        lines: Vec<RawLine>,
        language: &'static str,
    }

    impl OcrEngine for FakeEngine {
        fn recognize(
            &self,
            _image: &GrayImage,
            _language: Option<&str>,
        ) -> Result<EngineOutput, OcrError> {
            Ok(EngineOutput {
                lines: self.lines.clone(),
                language: self.language.to_string(),
            })
        }

        fn available_languages(&self) -> Result<Vec<String>, OcrError> {
            Ok(vec![self.language.to_string()])
        }

        fn name(&self) -> &'static str {
            "fake"
        }
    }

    fn raw(text: &str, x: f64, y: f64, w: f64, h: f64) -> RawLine {
        RawLine {
            text: text.to_string(),
            bbox: ImageBox { x, y, w, h },
            confidence: None,
        }
    }

    /// A 2x upscale of a selection sitting on a monitor left of the primary —
    /// the layout that catches a missing origin or a missing scale.
    fn prepared_2x_on_the_left_monitor() -> Preprocessed {
        Preprocessed {
            image: GrayImage::new(1, 1, vec![0]),
            scale: 2.0,
            source: Rect::new(-1600, 300, 800, 200),
            steps: vec![Step::Grayscale, Step::Upscale(2)],
            text_height_px: Some(10),
        }
    }

    #[test]
    fn boxes_come_back_in_virtual_desktop_space() {
        let engine = FakeEngine {
            lines: vec![raw("Install now", 20.0, 40.0, 300.0, 24.0)],
            language: "en-US",
        };
        let result =
            recognize(&engine, &prepared_2x_on_the_left_monitor(), None).expect("recognize");

        assert_eq!(result.lines.len(), 1);
        // (-1600 + 20/2, 300 + 40/2) with a 150x12 box.
        assert_eq!(result.lines[0].bbox, Rect::new(-1590, 320, 150, 12));
        assert_eq!(result.source_lang, "en-US");
        assert_eq!(result.engine, "fake");
    }

    #[test]
    fn blank_and_off_image_lines_are_dropped() {
        let engine = FakeEngine {
            lines: vec![
                raw("   ", 0.0, 0.0, 100.0, 20.0),
                raw("real", 10.0, 10.0, 80.0, 20.0),
                // Entirely outside the selection: clamping leaves nothing.
                raw("ghost", 10_000.0, 10_000.0, 50.0, 20.0),
            ],
            language: "en-US",
        };
        let result =
            recognize(&engine, &prepared_2x_on_the_left_monitor(), None).expect("recognize");
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].text, "real");
    }

    #[test]
    fn an_empty_recognition_is_no_text_not_an_engine_failure() {
        let engine = FakeEngine {
            lines: vec![],
            language: "en-US",
        };
        assert!(matches!(
            recognize(&engine, &prepared_2x_on_the_left_monitor(), None),
            Err(OcrError::NoText)
        ));
    }

    #[test]
    fn confidence_stays_unknown_rather_than_being_invented() {
        let engine = FakeEngine {
            lines: vec![raw("text", 0.0, 0.0, 100.0, 20.0)],
            language: "en-US",
        };
        let result =
            recognize(&engine, &prepared_2x_on_the_left_monitor(), None).expect("recognize");
        assert_eq!(result.lines[0].confidence, None);
    }

    #[test]
    fn the_wire_shape_matches_the_typescript_contract() {
        // Field names here are a contract with src/lib/types.ts.
        let line = OcrLine {
            text: "hello".into(),
            bbox: Rect::new(-10, 20, 30, 40),
            confidence: None,
            block_id: 2,
        };
        let json = serde_json::to_string(&line).unwrap();
        assert_eq!(
            json,
            r#"{"text":"hello","bbox":{"x":-10,"y":20,"w":30,"h":40},"confidence":null,"blockId":2}"#
        );
    }
}
