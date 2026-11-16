//! Fixture-image OCR tests — the M2 "done when".
//!
//! The milestone closes when "a fixture set of real screenshots (small UI text,
//! low contrast, monospace, a paragraph of prose) recognizes correctly, with
//! bounding boxes that land on the right lines" (docs/project/roadmap.md).
//!
//! The fixtures are **rendered here with GDI** rather than checked in as PNGs,
//! for three reasons: a binary fixture can't be reviewed in a diff; text drawn
//! by this machine's own font rasterizer at this machine's DPI is a far better
//! match for what the app actually captures than a screenshot taken somewhere
//! else; and the exact ink position is known, so a bounding-box claim can be
//! checked instead of eyeballed.
//!
//! Every fixture is placed at a **negative virtual origin**, because that is
//! the layout that catches a dropped coordinate translation (invariant 1).
//!
//! These tests talk to the real OS engine. A machine with no OCR language pack
//! is a configuration FR-34 explicitly supports, so they report and skip rather
//! than fail the build.

use std::collections::HashSet;
use std::ffi::c_void;
use std::time::Instant;

use otr_lib::capture::{BgraImage, Rect};
use otr_lib::ocr::{self, OcrEngine, OcrError, OcrResult};
use otr_lib::preprocess::{self, Preprocessed};

use windows::core::PCWSTR;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, GdiFlush,
    SelectObject, SetBkMode, SetTextColor, TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DIB_RGB_COLORS, FW_NORMAL, HGDIOBJ,
    OUT_DEFAULT_PRECIS, TRANSPARENT,
};

type Rgb = (u8, u8, u8);

const WHITE: Rgb = (0xFF, 0xFF, 0xFF);
const BLACK: Rgb = (0x11, 0x11, 0x11);
/// A dark-mode editor: light gray text on a near-black panel.
const DARK_PANEL: Rgb = (0x1E, 0x1E, 0x1E);
const DARK_TEXT: Rgb = (0xCC, 0xCC, 0xCC);
/// Washed out: the case the adaptive threshold exists for.
const FAINT_BG: Rgb = (0x8C, 0x8C, 0x8C);
const FAINT_TEXT: Rgb = (0x76, 0x76, 0x76);

/* --------------------------------------------------------------- fixtures */

/// Draw lines of text into a BGRA buffer that lives at `bounds` in
/// virtual-desktop space. `lines` are `(x, y, text)` in buffer-local pixels.
fn render(
    bounds: Rect,
    font_px: i32,
    face: &str,
    fg: Rgb,
    bg: Rgb,
    lines: &[(i32, i32, &str)],
) -> BgraImage {
    unsafe {
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid(), "CreateCompatibleDC");

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bounds.w,
                // Negative height: top-down, matching the capture buffer.
                biHeight: -bounds.h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        let previous_bitmap = SelectObject(dc, HGDIOBJ(bitmap.0));

        let len = (bounds.area() as usize) * 4;
        let buffer = std::slice::from_raw_parts_mut(bits.cast::<u8>(), len);
        for px in buffer.chunks_exact_mut(4) {
            px[0] = bg.2;
            px[1] = bg.1;
            px[2] = bg.0;
            px[3] = 0xFF;
        }

        let face: Vec<u16> = face.encode_utf16().chain(std::iter::once(0)).collect();
        // A negative height asks for a character height rather than a cell
        // height, which is what "11 px text" means to a person.
        let font = CreateFontW(
            -font_px,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            PCWSTR(face.as_ptr()),
        );
        let previous_font = SelectObject(dc, HGDIOBJ(font.0));

        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, colorref(fg));

        for (x, y, text) in lines {
            let wide: Vec<u16> = text.encode_utf16().collect();
            TextOutW(dc, *x, *y, &wide).ok().expect("TextOutW");
        }
        let _ = GdiFlush();

        let mut pixels = buffer.to_vec();
        // GDI leaves alpha at zero wherever it drew — the same trap the real
        // capture path has to work around.
        for px in pixels.chunks_exact_mut(4) {
            px[3] = 0xFF;
        }

        SelectObject(dc, previous_font);
        SelectObject(dc, previous_bitmap);
        let _ = DeleteObject(HGDIOBJ(font.0));
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);

        BgraImage { bounds, pixels }
    }
}

fn colorref((r, g, b): Rgb) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// The rectangle actually covered by ink, in virtual-desktop space.
///
/// Measured from the rendered pixels rather than assumed from the draw calls,
/// so the bounding-box assertions compare against where the glyphs really are.
fn ink_bounds(image: &BgraImage) -> Rect {
    let background = [image.pixels[0], image.pixels[1], image.pixels[2]];
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);

    for y in 0..image.height() {
        for x in 0..image.width() {
            let i = ((y * image.width() + x) as usize) * 4;
            let differs = (0..3)
                .map(|c| (image.pixels[i + c] as i32 - background[c] as i32).abs())
                .max()
                .unwrap_or(0)
                > 40;
            if differs {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    assert!(min_x != i32::MAX, "the fixture rendered no ink at all");
    Rect::new(
        image.bounds.x + min_x,
        image.bounds.y + min_y,
        max_x - min_x + 1,
        max_y - min_y + 1,
    )
}

/* ------------------------------------------------------------------ setup */

fn engine_available() -> bool {
    ocr::default_engine()
        .available_languages()
        .map(|langs| !langs.is_empty())
        .unwrap_or(false)
}

/// Returns false and explains itself when this machine can't run these tests.
fn ready(test: &str) -> bool {
    if engine_available() {
        return true;
    }
    eprintln!("{test}: skipped — no OCR language pack is installed on this machine");
    false
}

/// The full M2 path, with the timings printed so `cargo test -- --nocapture`
/// gives real numbers against the 200 ms budget (NFR-P3).
fn recognize(label: &str, crop: &BgraImage) -> Result<OcrResult, OcrError> {
    let started = Instant::now();
    let prepared = preprocess::prepare(crop);
    let prepare_ms = started.elapsed().as_millis();

    let started = Instant::now();
    let engine = ocr::default_engine();
    let result = ocr::recognize(&engine, &prepared, None);
    let ocr_ms = started.elapsed().as_millis();

    eprintln!(
        "{label}: {}x{} -> [{}] {}x{} | prepare {prepare_ms} ms, ocr {ocr_ms} ms",
        crop.width(),
        crop.height(),
        prepared.steps_label(),
        prepared.image.width,
        prepared.image.height,
    );
    result
}

/// OCR with no pre-processing beyond the grayscale the engine needs — the
/// control case for FR-31.
fn recognize_unprocessed(crop: &BgraImage) -> Result<OcrResult, OcrError> {
    let prepared = Preprocessed {
        image: preprocess::to_gray(crop),
        scale: 1.0,
        source: crop.bounds,
        steps: Vec::new(),
        text_height_px: None,
    };
    let engine = ocr::default_engine();
    ocr::recognize(&engine, &prepared, None)
}

fn words(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn matched(expected: &[&str], result: &OcrResult) -> usize {
    let found = words(&result.full_text);
    expected
        .iter()
        .filter(|w| found.contains(&w.to_lowercase()))
        .count()
}

/* ------------------------------------------------------------------ tests */

#[test]
fn a_paragraph_of_prose_is_recognized_with_boxes_on_the_right_lines() {
    if !ready("prose") {
        return;
    }

    // A selection on a monitor to the LEFT of the primary.
    let crop = render(
        Rect::new(-1500, 240, 520, 120),
        22,
        "Segoe UI",
        BLACK,
        WHITE,
        &[
            (20, 16, "The update failed because the disk"),
            (20, 52, "is full. Free some space and retry."),
        ],
    );
    let ink = ink_bounds(&crop);

    let result = recognize("prose", &crop).expect("prose should recognize");

    let expected = ["update", "failed", "because", "disk", "space", "retry"];
    assert_eq!(
        matched(&expected, &result),
        expected.len(),
        "missed words in {:?}",
        words(&result.full_text)
    );

    assert!(
        result.lines.len() >= 2,
        "two drawn lines came back as {} line(s)",
        result.lines.len()
    );

    // The claim the in-place overlay depends on: every box is where the ink
    // is, in virtual-desktop space. A dropped origin translation would put
    // these near 0,0 instead of -1500,240.
    let tolerance = 8;
    for line in &result.lines {
        assert!(
            line.bbox.x >= ink.x - tolerance
                && line.bbox.y >= ink.y - tolerance
                && line.bbox.right() <= ink.right() + tolerance
                && line.bbox.bottom() <= ink.bottom() + tolerance,
            "line box {:?} is outside the ink {ink:?}",
            line.bbox
        );
    }

    // Two lines 36 px apart at 22 px text is ordinary line spacing, so they
    // belong to one paragraph and get translated as one sentence.
    assert_eq!(result.block_count(), 1, "prose split into paragraphs");
}

#[test]
fn small_ui_text_is_not_made_worse_by_pre_processing() {
    if !ready("small text") {
        return;
    }

    // 11 px is a tooltip or a status bar; 8 px is what the same UI looks like
    // on a 100%-scaled 4K panel. Both are under the 20 px floor that triggers
    // the upscale (FR-31).
    let expected = ["connection", "timed", "out"];
    let mut results = Vec::new();

    for font_px in [11, 8] {
        let crop = render(
            Rect::new(-900, 100, 300, 40),
            font_px,
            "Segoe UI",
            BLACK,
            WHITE,
            &[(10, 12, "Connection timed out")],
        );

        let processed = recognize(&format!("{font_px} px"), &crop)
            .map(|r| matched(&expected, &r))
            .unwrap_or(0);
        let raw = recognize_unprocessed(&crop)
            .map(|r| matched(&expected, &r))
            .unwrap_or(0);

        eprintln!("{font_px} px: {processed}/3 words with the ladder, {raw}/3 without");
        results.push((font_px, processed, raw));

        // The claim that has to hold at every size: the ladder never costs
        // accuracy. Whether it *gains* any depends on the text — recorded
        // above rather than asserted, because asserting a gain on a fixture
        // that doesn't need one turns a real signal into a flaky test.
        assert!(
            processed >= raw,
            "pre-processing made {font_px} px text worse: {processed} vs {raw}"
        );
    }

    let (_, eleven_px, _) = results[0];
    assert!(
        eleven_px >= 1,
        "nothing recognized in 11 px text even with the ladder"
    );
}

#[test]
fn low_contrast_text_survives_the_threshold_step() {
    if !ready("low contrast") {
        return;
    }

    let crop = render(
        Rect::new(-400, -200, 420, 60),
        20,
        "Segoe UI",
        FAINT_TEXT,
        FAINT_BG,
        &[(16, 16, "License expires soon")],
    );

    let result = recognize("low contrast", &crop).expect("low contrast should recognize");
    assert!(
        matched(&["license", "expires", "soon"], &result) >= 2,
        "low contrast recognition returned {:?}",
        words(&result.full_text)
    );
}

#[test]
fn monospace_text_on_a_dark_panel_is_recognized() {
    if !ready("monospace") {
        return;
    }

    // Light text on a dark background: a terminal, an editor, the single most
    // likely thing a developer points this tool at.
    let crop = render(
        Rect::new(-1920, 0, 460, 60),
        18,
        "Consolas",
        DARK_TEXT,
        DARK_PANEL,
        &[(12, 16, "error: cannot find module")],
    );

    let result = recognize("monospace", &crop).expect("monospace should recognize");
    assert!(
        matched(&["error", "cannot", "find", "module"], &result) >= 3,
        "monospace recognition returned {:?}",
        words(&result.full_text)
    );
}

#[test]
fn separate_paragraphs_come_back_as_separate_blocks_in_reading_order() {
    if !ready("blocks") {
        return;
    }

    let crop = render(
        Rect::new(-700, 500, 460, 200),
        20,
        "Segoe UI",
        BLACK,
        WHITE,
        &[
            (16, 10, "First paragraph here"),
            (16, 40, "continues on this line"),
            // A clear paragraph gap, not just line spacing.
            (16, 120, "Second paragraph starts"),
        ],
    );

    let result = recognize("blocks", &crop).expect("blocks should recognize");

    let text = result.full_text.to_lowercase();
    let first = text.find("first").expect("first paragraph missing");
    let second = text.find("second").expect("second paragraph missing");
    assert!(first < second, "lines came back out of reading order");

    assert!(
        result.block_count() >= 2,
        "a paragraph gap did not start a new block: {:?}",
        result.lines.iter().map(|l| l.block_id).collect::<Vec<_>>()
    );

    // Boxes must stay in top-to-bottom order too — the overlay draws them in
    // this order and a scrambled one would stack translations wrongly.
    let tops: Vec<i32> = result.lines.iter().map(|l| l.bbox.y).collect();
    assert!(
        tops.windows(2).all(|w| w[0] <= w[1]),
        "line boxes are not sorted top to bottom: {tops:?}"
    );
}

#[test]
fn a_blank_region_is_no_text_rather_than_an_engine_failure() {
    if !ready("blank") {
        return;
    }

    // FR-35: dragging a box over an empty area is normal use.
    let crop = render(
        Rect::new(-200, -100, 300, 120),
        20,
        "Segoe UI",
        WHITE,
        WHITE,
        &[],
    );

    match recognize("blank", &crop) {
        Err(OcrError::NoText) => {}
        other => panic!("expected NoText, got {other:?}"),
    }
}
