//! The pre-processing ladder between a raw crop and the OCR engine.
//!
//! ADR-0002 keeps this **outside** the engine on purpose: every engine benefits
//! from the same tuning, and the stage can be measured on its own. The ladder
//! (FR-31, feature-specs F5) is applied **conditionally** — a large, clean,
//! high-contrast region is already what the engine wants, and processing it
//! costs time and can crush the anti-aliasing the recognizer relies on:
//!
//! 1. grayscale (always — it is also the engine's input format),
//! 2. upscale x2-x3 (Lanczos-3) when the estimated text height is under ~20 px,
//! 3. contrast normalize, unless the region already spans the range,
//! 4. adaptive threshold, only when contrast is measured as low.
//!
//! Everything here is pure: a buffer in, a buffer out, no OS calls. That is
//! what makes it testable without a screen (quality/testing-strategy.md).

use std::f64::consts::PI;

use crate::capture::{BgraImage, Rect};

/// Bytes per pixel in the captured BGRA buffer.
const BPP: usize = 4;

/// Under this, Windows OCR starts losing glyphs on UI text — the single
/// biggest accuracy lever we have (feature-specs F5).
const MIN_TEXT_HEIGHT_PX: i32 = 20;

/// Upscaling past 3x buys nothing: the information isn't there, and the cost
/// is quadratic.
const MAX_UPSCALE: u32 = 3;

/// p98 - p02 of the gray histogram. Below this the region is washed out
/// enough that adaptive thresholding earns its cost.
const LOW_CONTRAST_SPREAD: u8 = 60;

/// Above this the region already spans most of the range and normalizing is a
/// no-op with extra steps.
const ALREADY_FULL_RANGE: u8 = 200;

/// How far from the background a pixel must be to count as ink when estimating
/// text height, as a fraction of the region's own contrast and clamped to this
/// range. A fixed threshold would see no text at all in exactly the washed-out
/// regions that need the upscale most.
const INK_DELTA_RANGE: std::ops::RangeInclusive<i32> = 12..=48;

/// Bradley-Roth: a pixel is ink when it sits this many percent below the mean
/// of its neighbourhood.
const THRESHOLD_PERCENT: i32 = 12;

/// 8-bit grayscale, top-down, tightly packed.
///
/// `Debug` is implemented by hand and prints dimensions only: invariant 5 says
/// nothing captured is ever logged, and a derived `Debug` on a pixel buffer is
/// exactly how that invariant gets broken by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct GrayImage {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for GrayImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GrayImage({}x{})", self.width, self.height)
    }
}

impl GrayImage {
    pub fn new(width: i32, height: i32, pixels: Vec<u8>) -> Self {
        debug_assert_eq!(pixels.len(), (width.max(0) * height.max(0)) as usize);
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn at(&self, x: i32, y: i32) -> u8 {
        self.pixels[(y * self.width + x) as usize]
    }

    fn row(&self, y: i32) -> &[u8] {
        let start = (y * self.width) as usize;
        &self.pixels[start..start + self.width as usize]
    }
}

/// One rung of the ladder, kept for logging and measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Grayscale,
    Upscale(u32),
    ContrastNormalize,
    AdaptiveThreshold,
}

impl Step {
    pub fn label(&self) -> String {
        match self {
            Step::Grayscale => "gray".into(),
            Step::Upscale(f) => format!("upscale{f}x"),
            Step::ContrastNormalize => "contrast".into(),
            Step::AdaptiveThreshold => "threshold".into(),
        }
    }
}

/// A crop that has been through the ladder, plus everything needed to map a
/// recognized box back onto the screen.
#[derive(Debug)]
pub struct Preprocessed {
    pub image: GrayImage,
    /// Image pixels per source pixel. 1.0 when nothing was upscaled.
    pub scale: f64,
    /// Where the source crop lives in virtual-desktop space. Origin may be
    /// negative (invariant 1).
    pub source: Rect,
    pub steps: Vec<Step>,
    /// The estimated height of a line of text in the *source*, if any text-like
    /// band was found at all. Drives the upscale decision; also worth logging,
    /// since "OCR found nothing" and "the text was 6 px tall" are the same bug
    /// report.
    pub text_height_px: Option<i32>,
}

impl Preprocessed {
    /// Map a box in processed-image coordinates back to virtual-desktop space.
    ///
    /// This is the second half of invariant 1: the overlay draws translations at
    /// these rectangles, so an error here puts the right text on the wrong
    /// widget. Edges are rounded outward so a box never cuts a glyph, and the
    /// result is clamped to the selection — a line box wider than the selection
    /// is a known OCR artifact (feature-specs F7).
    pub fn to_source(&self, x: f64, y: f64, w: f64, h: f64) -> Rect {
        let left = self.source.x as f64 + x / self.scale;
        let top = self.source.y as f64 + y / self.scale;
        let right = self.source.x as f64 + (x + w) / self.scale;
        let bottom = self.source.y as f64 + (y + h) / self.scale;

        let left = left.floor() as i32;
        let top = top.floor() as i32;
        let right = right.ceil() as i32;
        let bottom = bottom.ceil() as i32;

        Rect::new(left, top, right - left, bottom - top).intersect(&self.source)
    }

    pub fn steps_label(&self) -> String {
        self.steps
            .iter()
            .map(Step::label)
            .collect::<Vec<_>>()
            .join("+")
    }
}

/// Run the ladder over a crop.
pub fn prepare(crop: &BgraImage) -> Preprocessed {
    let mut image = to_gray(crop);
    let mut steps = vec![Step::Grayscale];

    let text_height_px = estimate_text_height(&image);
    let spread = contrast_spread(&image);

    let factor = upscale_factor(text_height_px);
    if factor > 1 {
        image = upscale_lanczos3(&image, factor);
        steps.push(Step::Upscale(factor));
    }

    if spread < ALREADY_FULL_RANGE {
        normalize_contrast(&mut image);
        steps.push(Step::ContrastNormalize);
    }

    if spread < LOW_CONTRAST_SPREAD {
        // Window sized to the text, not the image: a window smaller than a
        // glyph eats the glyph's interior, a window spanning several lines
        // behaves like a global threshold.
        let window = text_height_px.unwrap_or(MIN_TEXT_HEIGHT_PX) * factor as i32 * 2;
        image = adaptive_threshold(&image, window);
        steps.push(Step::AdaptiveThreshold);
    }

    Preprocessed {
        image,
        scale: factor as f64,
        source: crop.bounds,
        steps,
        text_height_px,
    }
}

/// BGRA -> 8-bit luma (Rec. 601, integer weights summing to 256).
pub fn to_gray(crop: &BgraImage) -> GrayImage {
    let mut pixels = Vec::with_capacity(crop.pixels.len() / BPP);
    for px in crop.pixels.chunks_exact(BPP) {
        let (b, g, r) = (px[0] as u32, px[1] as u32, px[2] as u32);
        pixels.push(((77 * r + 150 * g + 29 * b) >> 8) as u8);
    }
    GrayImage::new(crop.width(), crop.height(), pixels)
}

fn upscale_factor(text_height_px: Option<i32>) -> u32 {
    // No text-like band found: upscaling a photograph or a blank region wastes
    // the budget for nothing. Leave it alone and let the engine decide.
    let Some(height) = text_height_px else {
        return 1;
    };
    if height <= 0 || height >= MIN_TEXT_HEIGHT_PX {
        return 1;
    }
    // Ceiling division: 20/10 upscales 2x, 20/11 still upscales 2x.
    let needed = ((MIN_TEXT_HEIGHT_PX + height - 1) / height) as u32;
    needed.clamp(2, MAX_UPSCALE)
}

/// Estimate the height of one line of text by horizontal projection.
///
/// Rows that contain ink form bands; the median band height is a line of text.
/// This is a heuristic and is allowed to say "I don't know" — returning a wrong
/// number would silently skip the upscale that small text depends on.
pub fn estimate_text_height(img: &GrayImage) -> Option<i32> {
    if img.width <= 0 || img.height <= 0 {
        return None;
    }

    let levels = levels(img);
    let background = levels.median as i32;
    let delta =
        (levels.spread() as i32 / 4).clamp(*INK_DELTA_RANGE.start(), *INK_DELTA_RANGE.end());
    // One stray pixel is noise; a handful across a wide row is a glyph.
    let min_ink_per_row = 1.max(img.width / 200);

    let mut bands: Vec<i32> = Vec::new();
    let mut run_start: Option<i32> = None;

    for y in 0..img.height {
        let ink = img
            .row(y)
            .iter()
            .filter(|&&v| (v as i32 - background).abs() > delta)
            .count() as i32;

        match (ink >= min_ink_per_row, run_start) {
            (true, None) => run_start = Some(y),
            (false, Some(start)) => {
                bands.push(y - start);
                run_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = run_start {
        bands.push(img.height - start);
    }

    // A 1 px band is a rule or a border, not text. A band covering nearly the
    // whole region is a picture or a filled panel.
    let ceiling = ((img.height as f64) * 0.9).max(2.0) as i32;
    bands.retain(|&h| h >= 2 && h <= ceiling);

    if bands.is_empty() {
        return None;
    }
    bands.sort_unstable();
    Some(bands[bands.len() / 2])
}

/// The three numbers every decision in this module is made from. Percentiles
/// rather than min/max, so one stray white pixel in a dark region doesn't
/// report full contrast.
struct Levels {
    median: u8,
    p02: u8,
    p98: u8,
}

impl Levels {
    fn spread(&self) -> u8 {
        self.p98.saturating_sub(self.p02)
    }
}

fn levels(img: &GrayImage) -> Levels {
    let mut hist = [0u32; 256];
    for &v in &img.pixels {
        hist[v as usize] += 1;
    }

    let total = img.pixels.len() as u32;
    let percentile = |fraction: f64| -> u8 {
        let target = ((total as f64) * fraction) as u32;
        let mut seen = 0u32;
        for (value, &count) in hist.iter().enumerate() {
            seen += count;
            if seen > target {
                return value as u8;
            }
        }
        255
    };

    Levels {
        median: percentile(0.5),
        p02: percentile(0.02),
        p98: percentile(0.98),
    }
}

/// p98 - p02.
pub fn contrast_spread(img: &GrayImage) -> u8 {
    if img.pixels.is_empty() {
        return 0;
    }
    levels(img).spread()
}

/// Linear stretch between the 2nd and 98th percentile.
pub fn normalize_contrast(img: &mut GrayImage) {
    if img.pixels.is_empty() {
        return;
    }
    let levels = levels(img);
    let (low, high) = (levels.p02 as i32, levels.p98 as i32);

    // A flat region has nothing to stretch, and dividing by the span would be
    // a division by zero.
    if high <= low {
        return;
    }
    let span = (high - low) as f32;
    for v in &mut img.pixels {
        let stretched = (((*v as i32 - low) as f32) * 255.0 / span).round();
        *v = stretched.clamp(0.0, 255.0) as u8;
    }
}

/// Lanczos-3 upscale by an integer factor.
///
/// Separable, with the tap weights precomputed per phase: for an integer
/// factor every destination pixel at the same `index % factor` uses the same
/// weights, so the window function is evaluated `factor * 7` times for the
/// whole image instead of once per pixel.
pub fn upscale_lanczos3(img: &GrayImage, factor: u32) -> GrayImage {
    if factor <= 1 || img.width <= 0 || img.height <= 0 {
        return img.clone();
    }

    let f = factor as i32;
    let taps = phase_taps(factor);
    let dst_w = img.width * f;
    let dst_h = img.height * f;

    // Horizontal pass, kept in f32 so the vertical pass doesn't resample
    // already-rounded values.
    let mut horizontal = vec![0f32; (dst_w * img.height) as usize];
    for y in 0..img.height {
        let src_row = img.row(y);
        for x in 0..dst_w {
            let base = x / f;
            let mut acc = 0f32;
            for &(offset, weight) in &taps[(x % f) as usize] {
                let sx = (base + offset).clamp(0, img.width - 1) as usize;
                acc += src_row[sx] as f32 * weight;
            }
            horizontal[(y * dst_w + x) as usize] = acc;
        }
    }

    let mut pixels = vec![0u8; (dst_w * dst_h) as usize];
    for y in 0..dst_h {
        let base = y / f;
        let phase = &taps[(y % f) as usize];
        for x in 0..dst_w {
            let mut acc = 0f32;
            for &(offset, weight) in phase {
                let sy = (base + offset).clamp(0, img.height - 1);
                acc += horizontal[(sy * dst_w + x) as usize] * weight;
            }
            pixels[(y * dst_w + x) as usize] = acc.round().clamp(0.0, 255.0) as u8;
        }
    }

    GrayImage::new(dst_w, dst_h, pixels)
}

/// Normalized Lanczos-3 taps for each phase of an integer upscale.
fn phase_taps(factor: u32) -> Vec<Vec<(i32, f32)>> {
    const A: i32 = 3;
    let f = factor as f64;

    (0..factor)
        .map(|phase| {
            // Centre of the destination pixel, expressed in source pixels and
            // relative to the source pixel it falls in.
            let frac = (phase as f64 + 0.5) / f - 0.5;

            let mut taps: Vec<(i32, f32)> = (-A..=A)
                .map(|k| (k, lanczos3(k as f64 - frac)))
                .filter(|(_, w)| *w != 0.0)
                .map(|(k, w)| (k, w as f32))
                .collect();

            // Weights must sum to 1 or the image gains or loses brightness,
            // which shifts every threshold downstream.
            let sum: f32 = taps.iter().map(|(_, w)| *w).sum();
            if sum != 0.0 {
                for (_, w) in &mut taps {
                    *w /= sum;
                }
            }
            taps
        })
        .collect()
}

fn lanczos3(x: f64) -> f64 {
    const A: f64 = 3.0;
    if x == 0.0 {
        return 1.0;
    }
    if x.abs() >= A {
        return 0.0;
    }
    let px = PI * x;
    A * px.sin() * (px / A).sin() / (px * px)
}

/// Bradley-Roth adaptive threshold over an integral image.
///
/// Local, not global, because a screenshot routinely contains a bright panel
/// next to a dark one and a single cutoff erases one of them.
pub fn adaptive_threshold(img: &GrayImage, window: i32) -> GrayImage {
    if img.width <= 0 || img.height <= 0 {
        return img.clone();
    }

    let window = window.clamp(15, 101);
    let radius = window / 2;

    // (w+1) x (h+1) so the four-corner lookup needs no bounds special-casing.
    let stride = (img.width + 1) as usize;
    let mut integral = vec![0u64; stride * (img.height + 1) as usize];
    for y in 0..img.height {
        let mut row_sum = 0u64;
        for x in 0..img.width {
            row_sum += img.at(x, y) as u64;
            integral[(y + 1) as usize * stride + (x + 1) as usize] =
                integral[y as usize * stride + (x + 1) as usize] + row_sum;
        }
    }

    let mut pixels = vec![0u8; img.pixels.len()];
    for y in 0..img.height {
        let top = (y - radius).max(0);
        let bottom = (y + radius).min(img.height - 1);
        for x in 0..img.width {
            let left = (x - radius).max(0);
            let right = (x + radius).min(img.width - 1);

            let count = ((right - left + 1) as u64) * ((bottom - top + 1) as u64);
            let sum = integral[(bottom + 1) as usize * stride + (right + 1) as usize]
                + integral[top as usize * stride + left as usize]
                - integral[(bottom + 1) as usize * stride + left as usize]
                - integral[top as usize * stride + (right + 1) as usize];

            let value = img.at(x, y) as u64 * count * 100;
            let cutoff = sum * (100 - THRESHOLD_PERCENT) as u64;
            pixels[(y * img.width + x) as usize] = if value <= cutoff { 0 } else { 255 };
        }
    }

    GrayImage::new(img.width, img.height, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A crop whose pixels are a flat colour, positioned somewhere on a
    /// negative-origin virtual desktop.
    fn flat_crop(bounds: Rect, b: u8, g: u8, r: u8) -> BgraImage {
        let mut pixels = Vec::with_capacity(bounds.area() as usize * BPP);
        for _ in 0..bounds.area() {
            pixels.extend_from_slice(&[b, g, r, 0xFF]);
        }
        BgraImage { bounds, pixels }
    }

    /// A gray image with horizontal dark bands of `band` px every `period` px —
    /// a stand-in for lines of text.
    fn banded(width: i32, height: i32, band: i32, period: i32) -> GrayImage {
        let mut pixels = vec![255u8; (width * height) as usize];
        for y in 0..height {
            if y % period < band {
                for x in 0..width {
                    pixels[(y * width + x) as usize] = 0;
                }
            }
        }
        GrayImage::new(width, height, pixels)
    }

    #[test]
    fn gray_uses_luma_weights_not_a_channel_average() {
        // Pure green is much brighter to the eye than pure blue; an average
        // would make them identical and flatten green-on-blue text to nothing.
        let green = to_gray(&flat_crop(Rect::new(0, 0, 2, 2), 0, 255, 0));
        let blue = to_gray(&flat_crop(Rect::new(0, 0, 2, 2), 255, 0, 0));
        assert!(green.pixels[0] > blue.pixels[0]);
        assert_eq!(green.pixels[0], 149); // 150 * 255 >> 8
        assert_eq!(blue.pixels[0], 28); //  29 * 255 >> 8

        // White has to survive the integer weights intact, or every contrast
        // measurement downstream starts from a slightly gray page.
        let white = to_gray(&flat_crop(Rect::new(0, 0, 2, 2), 255, 255, 255));
        assert_eq!(white.pixels[0], 255);
    }

    #[test]
    fn gray_keeps_the_crop_dimensions() {
        let crop = flat_crop(Rect::new(-300, -50, 7, 5), 10, 20, 30);
        let gray = to_gray(&crop);
        assert_eq!((gray.width, gray.height), (7, 5));
        assert_eq!(gray.pixels.len(), 35);
    }

    #[test]
    fn text_height_is_the_median_band_height() {
        let img = banded(100, 60, 6, 15);
        assert_eq!(estimate_text_height(&img), Some(6));
    }

    #[test]
    fn text_height_is_unknown_for_a_blank_region() {
        let img = GrayImage::new(40, 40, vec![200u8; 1600]);
        assert_eq!(estimate_text_height(&img), None);
    }

    #[test]
    fn a_one_pixel_rule_is_not_mistaken_for_text() {
        let img = banded(100, 60, 1, 15);
        assert_eq!(estimate_text_height(&img), None);
    }

    #[test]
    fn small_text_is_upscaled_and_large_text_is_left_alone() {
        assert_eq!(upscale_factor(Some(6)), 3); // 20/6 -> 4, clamped to 3
        assert_eq!(upscale_factor(Some(10)), 2);
        assert_eq!(upscale_factor(Some(11)), 2);
        assert_eq!(upscale_factor(Some(20)), 1);
        assert_eq!(upscale_factor(Some(40)), 1);
        assert_eq!(upscale_factor(None), 1);
    }

    #[test]
    fn upscale_multiplies_both_dimensions() {
        let img = banded(20, 10, 3, 5);
        let up = upscale_lanczos3(&img, 3);
        assert_eq!((up.width, up.height), (60, 30));
        assert_eq!(up.pixels.len(), 1800);
    }

    #[test]
    fn upscaling_a_flat_image_does_not_change_its_value() {
        // The clearest test of tap normalization: unnormalized weights show up
        // here as ringing or a brightness shift.
        let img = GrayImage::new(8, 8, vec![137u8; 64]);
        let up = upscale_lanczos3(&img, 3);
        assert!(up.pixels.iter().all(|&v| v == 137), "flat image rang");
    }

    #[test]
    fn upscaling_keeps_edges_aligned_with_the_source() {
        // Left half black, right half white. After a 2x upscale the edge must
        // still be at the halfway point, not shifted by half a pixel.
        let mut pixels = vec![0u8; 64];
        for y in 0..8 {
            for x in 4..8 {
                pixels[y * 8 + x] = 255;
            }
        }
        let up = upscale_lanczos3(&GrayImage::new(8, 8, pixels), 2);
        let row: Vec<u8> = (0..16).map(|x| up.at(x, 8)).collect();
        assert!(row[0..6].iter().all(|&v| v < 40), "left half not dark");
        assert!(row[10..16].iter().all(|&v| v > 215), "right half not light");
    }

    #[test]
    fn upscale_by_one_is_a_no_op() {
        let img = banded(12, 12, 2, 4);
        assert_eq!(upscale_lanczos3(&img, 1), img);
    }

    #[test]
    fn contrast_spread_ignores_a_single_outlier_pixel() {
        let mut pixels = vec![120u8; 10_000];
        pixels[0] = 0;
        pixels[1] = 255;
        let img = GrayImage::new(100, 100, pixels);
        assert_eq!(contrast_spread(&img), 0);
    }

    #[test]
    fn normalize_stretches_a_washed_out_region_to_the_full_range() {
        let mut pixels = vec![120u8; 10_000];
        for p in pixels.iter_mut().take(5_000) {
            *p = 140;
        }
        let mut img = GrayImage::new(100, 100, pixels);
        normalize_contrast(&mut img);
        assert_eq!(img.pixels.iter().copied().min(), Some(0));
        assert_eq!(img.pixels.iter().copied().max(), Some(255));
    }

    #[test]
    fn normalize_leaves_a_flat_region_alone_instead_of_dividing_by_zero() {
        let mut img = GrayImage::new(10, 10, vec![77u8; 100]);
        normalize_contrast(&mut img);
        assert!(img.pixels.iter().all(|&v| v == 77));
    }

    #[test]
    fn adaptive_threshold_separates_dark_text_from_a_bright_panel() {
        // A dark band on a light background, next to a lighter band on a
        // darker background: a single global cutoff cannot keep both.
        let mut pixels = vec![255u8; 100 * 40];
        for y in 10..16 {
            for x in 0..50 {
                pixels[y * 100 + x] = 0; // dark text, bright panel
            }
        }
        for y in 0..40 {
            for x in 50..100 {
                pixels[y * 100 + x] = 90; // darker panel
            }
        }
        for y in 20..26 {
            for x in 50..100 {
                pixels[y * 100 + x] = 40; // its own darker text
            }
        }
        let out = adaptive_threshold(&GrayImage::new(100, 40, pixels), 21);

        assert!(out.pixels.iter().all(|&v| v == 0 || v == 255));
        assert_eq!(out.at(10, 12), 0, "text on the bright panel was lost");
        assert_eq!(out.at(60, 22), 0, "text on the dark panel was lost");
        assert_eq!(out.at(10, 30), 255, "bright background turned to ink");
    }

    #[test]
    fn boxes_map_back_to_the_selection_on_a_negative_origin_desktop() {
        // The bug this guards: a selection on a monitor left of the primary,
        // upscaled 2x. Forgetting either the origin or the scale puts the
        // translated line on the wrong screen.
        let prepared = Preprocessed {
            image: GrayImage::new(1, 1, vec![0]),
            scale: 2.0,
            source: Rect::new(-1500, 200, 400, 100),
            steps: vec![],
            text_height_px: Some(9),
        };

        // A box 40 image-px right and 20 image-px down = 20x10 source px.
        let mapped = prepared.to_source(40.0, 20.0, 200.0, 32.0);
        assert_eq!(mapped, Rect::new(-1480, 210, 100, 16));
    }

    #[test]
    fn box_mapping_is_identity_without_an_upscale() {
        let prepared = Preprocessed {
            image: GrayImage::new(1, 1, vec![0]),
            scale: 1.0,
            source: Rect::new(100, 50, 300, 80),
            steps: vec![],
            text_height_px: Some(24),
        };
        assert_eq!(
            prepared.to_source(10.0, 5.0, 60.0, 20.0),
            Rect::new(110, 55, 60, 20)
        );
    }

    #[test]
    fn a_box_wider_than_the_selection_is_clamped_to_it() {
        let prepared = Preprocessed {
            image: GrayImage::new(1, 1, vec![0]),
            scale: 1.0,
            source: Rect::new(0, 0, 100, 50),
            steps: vec![],
            text_height_px: Some(14),
        };
        assert_eq!(
            prepared.to_source(-10.0, -10.0, 500.0, 500.0),
            Rect::new(0, 0, 100, 50)
        );
    }

    #[test]
    fn box_edges_round_outward_so_a_glyph_is_never_cut() {
        let prepared = Preprocessed {
            image: GrayImage::new(1, 1, vec![0]),
            scale: 3.0,
            source: Rect::new(0, 0, 100, 100),
            steps: vec![],
            text_height_px: Some(7),
        };
        // 10/3 = 3.33 -> 3, and (10+10)/3 = 6.67 -> 7.
        assert_eq!(
            prepared.to_source(10.0, 10.0, 10.0, 10.0),
            Rect::new(3, 3, 4, 4)
        );
    }

    #[test]
    fn the_ladder_upscales_small_text_and_records_what_it_did() {
        // 6 px bands on a 60 px tall crop: small text, full contrast.
        let banded_crop = {
            let gray = banded(120, 60, 6, 15);
            let mut pixels = Vec::with_capacity(gray.pixels.len() * BPP);
            for &v in &gray.pixels {
                pixels.extend_from_slice(&[v, v, v, 0xFF]);
            }
            BgraImage {
                bounds: Rect::new(-200, 100, 120, 60),
                pixels,
            }
        };

        let prepared = prepare(&banded_crop);
        assert_eq!(prepared.text_height_px, Some(6));
        assert_eq!(prepared.scale, 3.0);
        assert_eq!(prepared.image.width, 360);
        assert_eq!(prepared.source, Rect::new(-200, 100, 120, 60));
        assert!(prepared.steps.contains(&Step::Upscale(3)));
        // Already full-range black on white: nothing else should fire.
        assert!(!prepared.steps.contains(&Step::ContrastNormalize));
        assert!(!prepared.steps.contains(&Step::AdaptiveThreshold));
        assert_eq!(prepared.steps_label(), "gray+upscale3x");
    }

    #[test]
    fn the_ladder_leaves_a_large_high_contrast_region_alone() {
        let gray = banded(400, 200, 30, 60);
        let mut pixels = Vec::with_capacity(gray.pixels.len() * BPP);
        for &v in &gray.pixels {
            pixels.extend_from_slice(&[v, v, v, 0xFF]);
        }
        let crop = BgraImage {
            bounds: Rect::new(0, 0, 400, 200),
            pixels,
        };

        let prepared = prepare(&crop);
        assert_eq!(prepared.scale, 1.0);
        assert_eq!(prepared.steps, vec![Step::Grayscale]);
    }

    #[test]
    fn the_ladder_thresholds_a_low_contrast_region() {
        // Mid-gray text on a slightly lighter mid-gray background.
        let mut gray = banded(200, 80, 10, 20);
        for v in &mut gray.pixels {
            *v = if *v == 0 { 118 } else { 140 };
        }
        let mut pixels = Vec::with_capacity(gray.pixels.len() * BPP);
        for &v in &gray.pixels {
            pixels.extend_from_slice(&[v, v, v, 0xFF]);
        }
        let crop = BgraImage {
            bounds: Rect::new(0, 0, 200, 80),
            pixels,
        };

        let prepared = prepare(&crop);
        assert!(prepared.steps.contains(&Step::ContrastNormalize));
        assert!(prepared.steps.contains(&Step::AdaptiveThreshold));
        assert!(prepared.image.pixels.iter().all(|&v| v == 0 || v == 255));
    }
}
