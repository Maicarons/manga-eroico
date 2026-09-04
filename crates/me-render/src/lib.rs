//! me-render: pure typesetting logic that turns (text, bubble box, style)
//! into a glyph layout plan. Font rasterization happens in the frontend
//! (Konva/canvas); this crate owns the deterministic, unit-testable rules:
//! line wrapping, CJK handling, vertical layout and font-size fitting.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_size: u32,
    pub vertical: bool,
    /// Extra spacing between lines (horizontal mode) or columns (vertical).
    pub line_gap: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self { font_size: 18, vertical: false, line_gap: 4.0 }
    }
}

/// Mock renderer used by tests / web preview: applies the default style and
/// returns a plan without font fitting.
pub struct MockRender;

impl me_render_provider::RenderProvider for MockRender {
    fn render_plan(&self, input: &LayoutInput) -> LayoutPlan {
        fit_font_size(input, 8, 48)
    }
}

/// Minimal provider surface shared with the pipeline host.
pub mod me_render_provider {
    use super::*;

    pub trait RenderProvider: Send + Sync {
        fn render_plan(&self, input: &LayoutInput) -> LayoutPlan;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutInput {
    pub text: String,
    /// Available box in px.
    pub box_w: u32,
    pub box_h: u32,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphCell {
    pub ch: char,
    /// Relative position in px inside the bubble box.
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutPlan {
    pub font_size: u32,
    pub cells: Vec<GlyphCell>,
}

/// True for CJK ideographs, kana, hangul and fullwidth punctuation — chars
/// that may break anywhere and never need hyphenation.
pub fn is_cjk(c: char) -> bool {
    let u = c as u32;
    matches!(u,
        0x2E80..=0x9FFF    // CJK radicals, kana, han .. Hangul syllables
        | 0xF900..=0xFAFF  // CJK compat ideographs
        | 0xFF00..=0xFFEF  // fullwidth forms
        | 0xAC00..=0xD7AF  // Hangul syllables (belt & braces)
    )
}

/// Greedy line breaking: CJK breaks per character; latin breaks per word,
/// preserving spaces at line ends as best-effort.
pub fn wrap_lines(text: &str, max_chars_per_line: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut current_len = 0usize;
        for word in paragraph.split_inclusive(' ') {
            let first = word.chars().next().unwrap_or(' ');
            if is_cjk(first) {
                for ch in word.chars() {
                    if current_len >= max_chars_per_line {
                        lines.push(std::mem::take(&mut current));
                        current_len = 0;
                    }
                    current.push(ch);
                    current_len += 1;
                }
            } else {
                // normalize the word: single spaces join words, none at line
                // edges; fit decision counts the joining space
                let trimmed = word.trim().trim_start_matches(' ');
                let trimmed_len = trimmed.chars().count();
                let joiner: usize = if current_len > 0 { 1 } else { 0 };
                if current_len + joiner + trimmed_len > max_chars_per_line && current_len > 0 {
                    lines.push(std::mem::take(&mut current));
                    current.push_str(trimmed);
                    current_len = trimmed_len;
                } else {
                    if joiner == 1 && !current.ends_with(' ') {
                        current.push(' ');
                        current_len += 1;
                    }
                    current.push_str(trimmed);
                    current_len += trimmed_len;
                }
            }
        }
        lines.push(current.trim_end().to_string());
    }
    lines
}

/// Max font size (binary-searched) whose wrapped layout fits the box.
/// Assumes glyph advance ≈ font_size for CJK and ≈ 0.55 * font_size for latin.
pub fn fit_font_size(input: &LayoutInput, min_size: u32, max_size: u32) -> LayoutPlan {
    let mut best: Option<(u32, Vec<GlyphCell>)> = None;
    let (mut lo, mut hi) = (min_size, max_size);
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if let Some(cells) = try_layout(input, mid) {
            best = Some((mid, cells));
            lo = mid + 1;
        } else {
            if mid == 0 {
                break;
            }
            hi = mid - 1;
        }
    }
    match best {
        Some((size, cells)) => LayoutPlan { font_size: size, cells },
        None => LayoutPlan { font_size: min_size, cells: try_layout(input, min_size).unwrap_or_default() },
    }
}

fn glyph_advance(ch: char, size: u32) -> f32 {
    if is_cjk(ch) {
        size as f32
    } else {
        size as f32 * 0.55
    }
}

fn try_layout(input: &LayoutInput, size: u32) -> Option<Vec<GlyphCell>> {
    let style = &input.style;
    let box_w = input.box_w as f32;
    let box_h = input.box_h as f32;
    let gap = style.line_gap;

    if style.vertical {
        // vertical: characters stack top-to-bottom, columns right-to-left
        let max_rows = ((box_h + gap) / (size as f32)).floor() as usize;
        if max_rows == 0 {
            return None;
        }
        let columns: Vec<Vec<char>> = {
            let mut cols = vec![Vec::new()];
            for ch in input.text.chars() {
                if cols.last().unwrap().len() >= max_rows {
                    cols.push(Vec::new());
                }
                cols.last_mut().unwrap().push(ch);
            }
            cols
        };
        let col_pitch = size as f32 + gap;
        let total_w = columns.len() as f32 * col_pitch - gap;
        if total_w > box_w {
            return None;
        }
        // columns flow right-to-left: column 0 at the right edge
        let mut cells = Vec::new();
        for (ci, col) in columns.iter().enumerate() {
            let x = box_w - (ci as f32 + 1.0) * col_pitch + gap;
            for (ri, ch) in col.iter().enumerate() {
                cells.push(GlyphCell { ch: *ch, x, y: ri as f32 * (size as f32) });
            }
        }
        Some(cells)
    } else {
        // horizontal: approximate width in "em units"
        let total_advance: f32 = input.text.chars().map(|c| glyph_advance(c, 1)).sum();
