//! Lines -> blocks -> paragraphs. Pure, and tested like it (system-design.md).
//!
//! The engine hands back lines. A translation provider given lines returns
//! fragments translated out of context — "Save" and "as" and "PDF" each on
//! their own. Grouping lines into blocks first is what makes the single
//! batched request (invariant 4) carry sentences, while the per-line boxes
//! survive so the in-place overlay still knows where to draw.

use crate::capture::Rect;

use super::OcrLine;

/// A vertical gap wider than this fraction of the median line height starts a
/// new block. Tuned to sit between normal line spacing (~0.2-0.4 of the line
/// height) and paragraph spacing.
const BLOCK_GAP_RATIO: f64 = 0.75;

/// A line that has already been mapped into virtual-desktop space.
#[derive(Debug, Clone)]
pub struct PositionedLine {
    pub text: String,
    pub bbox: Rect,
    pub confidence: Option<f32>,
}

/// Sort, group into blocks, and build the text that gets translated.
///
/// Reading order is top-to-bottom then left-to-right. Side-by-side columns are
/// a known limitation: they interleave, because telling a two-column layout
/// from a wide paragraph needs structure recovery, which feature-specs F5 puts
/// out of scope for v1.
pub fn assemble(mut lines: Vec<PositionedLine>) -> (Vec<OcrLine>, String) {
    if lines.is_empty() {
        return (Vec::new(), String::new());
    }

    lines.sort_by(|a, b| a.bbox.y.cmp(&b.bbox.y).then(a.bbox.x.cmp(&b.bbox.x)));

    let gap_limit = (median_height(&lines) as f64 * BLOCK_GAP_RATIO).max(1.0);

    let mut assembled: Vec<OcrLine> = Vec::with_capacity(lines.len());
    let mut text = String::new();
    let mut block_id = 0u32;
    let mut previous_bottom: Option<i32> = None;

    for line in lines {
        match previous_bottom {
            None => {}
            Some(bottom) if (line.bbox.y - bottom) as f64 > gap_limit => {
                block_id += 1;
                text.push_str("\n\n");
            }
            // Inside a block the lines are one paragraph, so they are joined
            // with a space rather than a newline: a provider translates a
            // sentence far better than it translates its halves.
            Some(_) => text.push(' '),
        }

        text.push_str(&line.text);
        previous_bottom = Some(line.bbox.bottom());

        assembled.push(OcrLine {
            text: line.text,
            bbox: line.bbox,
            confidence: line.confidence,
            block_id,
        });
    }

    (assembled, text)
}

/// Median rather than mean: one tall heading shouldn't move the gap limit for
/// the body text under it.
fn median_height(lines: &[PositionedLine]) -> i32 {
    let mut heights: Vec<i32> = lines.iter().map(|l| l.bbox.h).collect();
    heights.sort_unstable();
    heights[heights.len() / 2].max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, x: i32, y: i32, w: i32, h: i32) -> PositionedLine {
        PositionedLine {
            text: text.to_string(),
            bbox: Rect::new(x, y, w, h),
            confidence: None,
        }
    }

    #[test]
    fn nothing_in_nothing_out() {
        let (lines, text) = assemble(vec![]);
        assert!(lines.is_empty());
        assert_eq!(text, "");
    }

    #[test]
    fn tightly_spaced_lines_become_one_paragraph() {
        // 16 px lines, 4 px apart: ordinary line spacing.
        let (lines, text) = assemble(vec![
            line("The quick brown fox", 0, 0, 200, 16),
            line("jumps over the lazy dog", 0, 20, 220, 16),
        ]);
        assert_eq!(text, "The quick brown fox jumps over the lazy dog");
        assert!(lines.iter().all(|l| l.block_id == 0));
    }

    #[test]
    fn a_paragraph_gap_starts_a_new_block() {
        // 16 px lines; the third sits 24 px below the second.
        let (lines, text) = assemble(vec![
            line("First paragraph", 0, 0, 200, 16),
            line("still first", 0, 20, 180, 16),
            line("Second paragraph", 0, 60, 200, 16),
        ]);
        assert_eq!(text, "First paragraph still first\n\nSecond paragraph");
        assert_eq!(
            lines.iter().map(|l| l.block_id).collect::<Vec<_>>(),
            vec![0, 0, 1]
        );
    }

    #[test]
    fn lines_are_sorted_into_reading_order_whatever_order_they_arrive_in() {
        let (lines, text) = assemble(vec![
            line("third", 0, 60, 100, 16),
            line("second", 300, 0, 100, 16),
            line("first", 0, 0, 100, 16),
        ]);
        assert_eq!(
            lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        assert_eq!(text, "first second\n\nthird");
    }

    #[test]
    fn block_grouping_follows_the_text_size_not_a_fixed_pixel_gap() {
        // The same 20 px gap: a paragraph break for 8 px text, ordinary
        // spacing for 40 px text. A hard-coded gap gets one of these wrong.
        let (small, _) = assemble(vec![line("a", 0, 0, 100, 8), line("b", 0, 28, 100, 8)]);
        assert_eq!(small[1].block_id, 1);

        let (large, _) = assemble(vec![line("a", 0, 0, 100, 40), line("b", 0, 60, 100, 40)]);
        assert_eq!(large[1].block_id, 0);
    }

    #[test]
    fn overlapping_boxes_do_not_start_a_block() {
        // OCR boxes for adjacent lines routinely overlap by a pixel or two,
        // which makes the gap negative.
        let (lines, _) = assemble(vec![
            line("one", 0, 0, 100, 20),
            line("two", 0, 18, 100, 20),
        ]);
        assert!(lines.iter().all(|l| l.block_id == 0));
    }

    #[test]
    fn a_single_line_is_one_block_and_the_text_is_itself() {
        let (lines, text) = assemble(vec![line("Solo", -500, 40, 80, 14)]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].block_id, 0);
        assert_eq!(lines[0].bbox, Rect::new(-500, 40, 80, 14));
        assert_eq!(text, "Solo");
    }

    #[test]
    fn every_line_keeps_its_own_box_so_the_overlay_can_position_it() {
        let (lines, _) = assemble(vec![
            line("one", -1920, 0, 100, 16),
            line("two", -1900, 20, 120, 16),
        ]);
        assert_eq!(lines[0].bbox, Rect::new(-1920, 0, 100, 16));
        assert_eq!(lines[1].bbox, Rect::new(-1900, 20, 120, 16));
    }

    #[test]
    fn block_ids_are_contiguous_and_start_at_zero() {
        let (lines, _) = assemble(vec![
            line("a", 0, 0, 100, 10),
            line("b", 0, 40, 100, 10),
            line("c", 0, 80, 100, 10),
        ]);
        assert_eq!(
            lines.iter().map(|l| l.block_id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
}
