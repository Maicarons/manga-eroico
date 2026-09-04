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
