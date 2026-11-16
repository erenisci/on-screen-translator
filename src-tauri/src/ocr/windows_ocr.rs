//! `Windows.Media.Ocr` — the v1 engine (ADR-0002).
//!
//! All WinRT interop is confined to this file on purpose: the rest of the core
//! is plain Rust over buffers, and keeping COM-shaped APIs behind the
//! `OcrEngine` trait is what makes the pipeline testable without a screen or a
//! language pack.
//!
//! Three Windows-specific facts shape the code below:
//!
//! 1. **The engine is a language, not a detector.** There is no "recognize
//!    whatever this is" mode — you pick a recognizer and it reads with that
//!    language's model. "Auto" therefore means the user's own profile
//!    languages, and the detected source language is whichever recognizer was
//!    actually used (TD-10).
//! 2. **It refuses images under 40x40 px**, so a thin one-line selection has
//!    to be padded.
//! 3. **It reports no confidence at all** — not per line, not per word
//!    (TD-09). We pass `None` rather than inventing a number.

use std::sync::OnceLock;

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine as WinOcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::System::Com::CoIncrementMTAUsage;

use super::{EngineOutput, ImageBox, OcrEngine, OcrError, RawLine};
use crate::preprocess::GrayImage;

/// `Windows.Media.Ocr` rejects anything smaller than this on either axis.
const MIN_ENGINE_DIMENSION: i32 = 40;

/// Used only if the engine refuses to report its own limit.
const FALLBACK_MAX_DIMENSION: i32 = 10_000;

pub struct WindowsOcr;

impl OcrEngine for WindowsOcr {
    fn name(&self) -> &'static str {
        "windows.media.ocr"
    }

    fn available_languages(&self) -> Result<Vec<String>, OcrError> {
        ensure_mta();

        let languages = WinOcrEngine::AvailableRecognizerLanguages()
            .map_err(|e| engine_error("AvailableRecognizerLanguages", e))?;
        let count = languages
            .Size()
            .map_err(|e| engine_error("AvailableRecognizerLanguages.Size", e))?;

        let mut tags = Vec::with_capacity(count as usize);
        for index in 0..count {
            let language = languages
                .GetAt(index)
                .map_err(|e| engine_error("AvailableRecognizerLanguages.GetAt", e))?;
            let tag = language
                .LanguageTag()
                .map_err(|e| engine_error("Language.LanguageTag", e))?;
            tags.push(tag.to_string());
        }
        Ok(tags)
    }

    fn recognize(
        &self,
        image: &GrayImage,
        language: Option<&str>,
    ) -> Result<EngineOutput, OcrError> {
        ensure_mta();

        let engine = create_engine(language)?;

        let max = WinOcrEngine::MaxImageDimension()
            .map(|m| m as i32)
            .unwrap_or(FALLBACK_MAX_DIMENSION);
        if image.width > max || image.height > max {
            return Err(OcrError::ImageTooLarge {
                width: image.width,
                height: image.height,
                max,
            });
        }

        let bitmap = software_bitmap(image)?;

        // Blocking on purpose. Tauri runs a non-async command on a worker
        // thread, and the pipeline is a single call with a 200 ms budget
        // (NFR-P3) — an async hop here would buy nothing and spread COM
        // lifetimes across tasks.
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| engine_error("RecognizeAsync", e))?
            .join()
            .map_err(|e| engine_error("RecognizeAsync.join", e))?;

        let recognizer = engine
            .RecognizerLanguage()
            .and_then(|l| l.LanguageTag())
            .map(|tag| tag.to_string())
            .map_err(|e| engine_error("RecognizerLanguage", e))?;

        let lines = result
            .Lines()
            .map_err(|e| engine_error("OcrResult.Lines", e))?;
        let count = lines
            .Size()
            .map_err(|e| engine_error("OcrResult.Lines.Size", e))?;

        let mut out = Vec::with_capacity(count as usize);
        for index in 0..count {
            let line = lines
                .GetAt(index)
                .map_err(|e| engine_error("OcrResult.Lines.GetAt", e))?;
            let text = line
                .Text()
                .map_err(|e| engine_error("OcrLine.Text", e))?
                .to_string();

            let Some(bbox) = line_bounds(&line)? else {
                // A line with no words has no position, and a line we cannot
                // place is useless to the in-place overlay.
                continue;
            };

            out.push(RawLine {
                text,
                bbox,
                // TD-09: the API exposes none. Inventing one would make the
                // "low confidence" UI lie in both directions.
                confidence: None,
            });
        }

        Ok(EngineOutput {
            lines: out,
            language: recognizer,
        })
    }
}

/// The union of the line's word boxes — WinRT puts `BoundingRect` on words,
/// not on lines.
fn line_bounds(line: &windows::Media::Ocr::OcrLine) -> Result<Option<ImageBox>, OcrError> {
    let words = line.Words().map_err(|e| engine_error("OcrLine.Words", e))?;
    let count = words
        .Size()
        .map_err(|e| engine_error("OcrLine.Words.Size", e))?;

    let mut bounds: Option<ImageBox> = None;
    for index in 0..count {
        let rect = words
            .GetAt(index)
            .map_err(|e| engine_error("OcrLine.Words.GetAt", e))?
            .BoundingRect()
            .map_err(|e| engine_error("OcrWord.BoundingRect", e))?;

        let word = ImageBox {
            x: rect.X as f64,
            y: rect.Y as f64,
            w: rect.Width as f64,
            h: rect.Height as f64,
        };
        bounds = Some(match bounds {
            Some(current) => current.union(word),
            None => word,
        });
    }
    Ok(bounds)
}

/// Pick a recognizer.
///
/// A missing language pack is the single most likely failure here, and FR-34
/// insists it be actionable rather than a generic error — so both the explicit
/// and the automatic path map a null engine onto `LanguageUnavailable`, which
/// `error.rs` turns into a message with a link to the Windows setting.
fn create_engine(language: Option<&str>) -> Result<WinOcrEngine, OcrError> {
    match language {
        Some(tag) => {
            let language = Language::CreateLanguage(&HSTRING::from(tag)).map_err(|_| {
                OcrError::LanguageUnavailable {
                    requested: tag.to_string(),
                }
            })?;
            WinOcrEngine::TryCreateFromLanguage(&language).map_err(|_| {
                OcrError::LanguageUnavailable {
                    requested: tag.to_string(),
                }
            })
        }
        None => {
            WinOcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| OcrError::NoLanguages)
        }
    }
}

/// Wrap the gray buffer as a WinRT `SoftwareBitmap`.
///
/// Padding is added to the right and bottom only, never the top or left, so
/// every box the engine returns is still in the un-padded image's coordinate
/// space and `Preprocessed::to_source` needs no knowledge of this at all.
fn software_bitmap(image: &GrayImage) -> Result<SoftwareBitmap, OcrError> {
    let width = image.width.max(MIN_ENGINE_DIMENSION);
    let height = image.height.max(MIN_ENGINE_DIMENSION);
    let fill = dominant_value(image);

    let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height {
        for x in 0..width {
            let value = if x < image.width && y < image.height {
                image.at(x, y)
            } else {
                // Pad with the region's own background, so the padding never
                // reads as an edge the recognizer has to interpret.
                fill
            };
            let i = ((y as usize) * (width as usize) + (x as usize)) * 4;
            bgra[i] = value;
            bgra[i + 1] = value;
            bgra[i + 2] = value;
            bgra[i + 3] = 0xFF;
        }
    }

    let writer = DataWriter::new().map_err(|e| engine_error("DataWriter::new", e))?;
    writer
        .WriteBytes(&bgra)
        .map_err(|e| engine_error("DataWriter.WriteBytes", e))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| engine_error("DataWriter.DetachBuffer", e))?;

    SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width, height)
        .map_err(|e| engine_error("SoftwareBitmap::CreateCopyFromBuffer", e))
}

/// The most common gray value — the background, for anything but a photograph.
fn dominant_value(image: &GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    for &value in &image.pixels {
        histogram[value as usize] += 1;
    }
    histogram
        .iter()
        .enumerate()
        .max_by_key(|(_, &count)| count)
        .map(|(value, _)| value as u8)
        .unwrap_or(0xFF)
}

/// Make sure this thread can call WinRT at all.
///
/// Tauri runs commands on worker threads that have no COM apartment, and a
/// WinRT call from an uninitialized thread fails with `CO_E_NOTINITIALIZED`.
/// `CoIncrementMTAUsage` keeps a process-wide MTA alive, and the only thing
/// that would tear it down again is an explicit `CoDecrementMTAUsage` with the
/// returned cookie — which is never called, on purpose: the MTA should live as
/// long as the process. The cookie is a plain handle with no destructor, so
/// dropping it here changes nothing.
fn ensure_mta() {
    static MTA: OnceLock<()> = OnceLock::new();
    MTA.get_or_init(|| {
        unsafe {
            let _ = CoIncrementMTAUsage();
        };
    });
}

/// WinRT failures carry an HRESULT and a system message — useful in a log,
/// never shown to the user, and never containing captured content.
fn engine_error(call: &'static str, error: windows::core::Error) -> OcrError {
    OcrError::Engine {
        call,
        detail: format!("{} ({})", error.message(), error.code().0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These tests talk to the real OS engine. On a machine (or a CI runner)
    /// with no OCR language pack there is nothing to talk to, and that is a
    /// valid configuration — FR-34 exists precisely for it — so they report
    /// the state instead of failing the build.
    fn engine_available() -> bool {
        WindowsOcr
            .available_languages()
            .map(|langs| !langs.is_empty())
            .unwrap_or(false)
    }

    #[test]
    fn available_languages_are_well_formed_bcp47_tags() {
        let Ok(languages) = WindowsOcr.available_languages() else {
            eprintln!("skipped: the OCR engine is not available on this machine");
            return;
        };
        for tag in languages {
            assert!(!tag.is_empty());
            assert!(
                tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "not a BCP-47 tag: {tag}"
            );
        }
    }

    #[test]
    fn a_language_with_no_pack_is_reported_as_missing_not_as_a_crash() {
        if !engine_available() {
            eprintln!("skipped: the OCR engine is not available on this machine");
            return;
        }
        // A syntactically valid tag nobody has a recognizer for.
        let image = GrayImage::new(64, 64, vec![255u8; 64 * 64]);
        let result = WindowsOcr.recognize(&image, Some("xh-ZA"));
        assert!(
            matches!(result, Err(OcrError::LanguageUnavailable { .. })),
            "expected a missing-pack error, got {result:?}"
        );
    }

    #[test]
    fn a_blank_image_recognizes_as_no_lines_rather_than_failing() {
        if !engine_available() {
            eprintln!("skipped: the OCR engine is not available on this machine");
            return;
        }
        let image = GrayImage::new(200, 100, vec![255u8; 200 * 100]);
        let output = WindowsOcr
            .recognize(&image, None)
            .expect("a blank image is a normal input");
        assert!(output.lines.is_empty());
        assert!(!output.language.is_empty());
    }

    #[test]
    fn an_image_under_the_engine_minimum_is_padded_instead_of_rejected() {
        if !engine_available() {
            eprintln!("skipped: the OCR engine is not available on this machine");
            return;
        }
        // 300x12 is a perfectly ordinary one-line selection, and under the
        // engine's 40 px floor on one axis.
        let image = GrayImage::new(300, 12, vec![255u8; 300 * 12]);
        assert!(WindowsOcr.recognize(&image, None).is_ok());
    }

    #[test]
    fn padding_uses_the_background_and_leaves_the_origin_alone() {
        // 10x10 of value 7, padded to 40x40: the original pixels must stay at
        // (0,0), or every bounding box comes back shifted.
        let image = GrayImage::new(10, 10, vec![7u8; 100]);
        let fill = dominant_value(&image);
        assert_eq!(fill, 7);

        let padded_at = |x: i32, y: i32| -> u8 {
            if x < image.width && y < image.height {
                image.at(x, y)
            } else {
                fill
            }
        };
        assert_eq!(padded_at(0, 0), 7);
        assert_eq!(padded_at(39, 39), 7);
    }

    #[test]
    fn dominant_value_finds_the_background_not_the_ink() {
        let mut pixels = vec![240u8; 1000];
        for p in pixels.iter_mut().take(200) {
            *p = 20;
        }
        assert_eq!(dominant_value(&GrayImage::new(100, 10, pixels)), 240);
    }
}
