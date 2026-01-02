//! Rich text paragraph layout.
//!
//! This module provides Skia Paragraph-style text layout for:
//! - Multi-line text layout
//! - Mixed styles within a paragraph
//! - Line breaking and word wrapping
//! - Hyphenation support
//! - Text alignment and justification

use crate::font::{Font, FontMetrics};
use crate::text_blob::{TextBlob, TextBlobBuilder};
use skia_rs_core::{Point, Rect, Scalar};

/// Text direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextDirection {
    /// Left-to-right text.
    #[default]
    Ltr = 0,
    /// Right-to-left text.
    Rtl,
}

/// Text alignment within a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextAlign {
    /// Left-aligned text.
    #[default]
    Left = 0,
    /// Right-aligned text.
    Right,
    /// Center-aligned text.
    Center,
    /// Justified text.
    Justify,
    /// Start-aligned (based on text direction).
    Start,
    /// End-aligned (based on text direction).
    End,
}

/// Paragraph style settings.
#[derive(Debug, Clone)]
pub struct ParagraphStyle {
    /// Text alignment.
    pub text_align: TextAlign,
    /// Text direction.
    pub text_direction: TextDirection,
    /// Maximum number of lines (0 = unlimited).
    pub max_lines: usize,
    /// Ellipsis string for truncated text.
    pub ellipsis: Option<String>,
    /// Line height multiplier.
    pub height: Scalar,
    /// Whether to use height strictly.
    pub height_override: bool,
    /// Strutting (minimum line height from font metrics).
    pub strut_enabled: bool,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            text_align: TextAlign::Left,
            text_direction: TextDirection::Ltr,
            max_lines: 0,
            ellipsis: None,
            height: 1.0,
            height_override: false,
            strut_enabled: false,
        }
    }
}

/// Text style for a span of text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// Font to use.
    pub font: Font,
    /// Foreground color (ARGB).
    pub color: u32,
    /// Background color (ARGB), or 0 for transparent.
    pub background_color: u32,
    /// Decoration (underline, strikethrough, etc.).
    pub decoration: TextDecoration,
    /// Letter spacing.
    pub letter_spacing: Scalar,
    /// Word spacing.
    pub word_spacing: Scalar,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Font::default(),
            color: 0xFF000000, // Black
            background_color: 0,
            decoration: TextDecoration::default(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

/// Text decoration settings.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextDecoration {
    /// Underline flag.
    pub underline: bool,
    /// Overline flag.
    pub overline: bool,
    /// Line-through (strikethrough) flag.
    pub line_through: bool,
    /// Decoration color (ARGB), or 0 for text color.
    pub color: u32,
    /// Decoration style.
    pub style: DecorationStyle,
    /// Decoration thickness multiplier.
    pub thickness: Scalar,
}

/// Decoration line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum DecorationStyle {
    /// Solid line.
    #[default]
    Solid = 0,
    /// Double line.
    Double,
    /// Dotted line.
    Dotted,
    /// Dashed line.
    Dashed,
    /// Wavy line.
    Wavy,
}

/// A builder for creating paragraphs.
pub struct ParagraphBuilder {
    style: ParagraphStyle,
    runs: Vec<TextRun>,
    current_style: TextStyle,
}

/// A run of text with a single style.
#[derive(Debug, Clone)]
struct TextRun {
    text: String,
    style: TextStyle,
}

impl ParagraphBuilder {
    /// Create a new paragraph builder with the given style.
    pub fn new(style: ParagraphStyle) -> Self {
        Self {
            style,
            runs: Vec::new(),
            current_style: TextStyle::default(),
        }
    }

    /// Push a style onto the style stack.
    pub fn push_style(&mut self, style: &TextStyle) -> &mut Self {
        self.current_style = style.clone();
        self
    }

    /// Pop the current style.
    pub fn pop(&mut self) -> &mut Self {
        self.current_style = TextStyle::default();
        self
    }

    /// Add text with the current style.
    pub fn add_text(&mut self, text: &str) -> &mut Self {
        if !text.is_empty() {
            self.runs.push(TextRun {
                text: text.to_string(),
                style: self.current_style.clone(),
            });
        }
        self
    }

    /// Build the paragraph.
    pub fn build(self) -> Paragraph {
        Paragraph {
            style: self.style,
            runs: self.runs,
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            laid_out: false,
        }
    }
}

/// A laid-out paragraph of text.
pub struct Paragraph {
    style: ParagraphStyle,
    runs: Vec<TextRun>,
    lines: Vec<TextLine>,
    width: Scalar,
    height: Scalar,
    laid_out: bool,
}

/// A line of text in a paragraph.
#[derive(Debug, Clone)]
struct TextLine {
    /// Glyphs and positions for this line.
    glyphs: Vec<(u16, Point)>,
    /// Font for this line (simplified - assumes single font per line).
    font: Font,
    /// Line bounds.
    bounds: Rect,
    /// Baseline Y position.
    baseline: Scalar,
}

impl Paragraph {
    /// Layout the paragraph to fit within the given width.
    pub fn layout(&mut self, width: Scalar) {
        self.width = width;
        self.lines.clear();

        // Collect run data first to avoid borrow issues
        let runs_data: Vec<_> = self
            .runs
            .iter()
            .map(|run| {
                let font = run.style.font.clone();
                let metrics = font.metrics();
                let line_height = metrics.line_height() * self.style.height;
                let char_width = font.size() * 0.5;
                let chars: Vec<char> = run.text.chars().collect();
                (
                    font,
                    line_height,
                    char_width,
                    run.style.letter_spacing,
                    run.style.word_spacing,
                    chars,
                )
            })
            .collect();

        let mut current_line_glyphs: Vec<(u16, Point)> = Vec::new();
        let mut current_x: Scalar = 0.0;
        let mut current_y: Scalar = 0.0;
        let mut current_font = Font::default();
        let mut line_height: Scalar = 0.0;

        for (font, run_line_height, char_width, letter_spacing, word_spacing, chars) in runs_data {
            current_font = font.clone();
            line_height = line_height.max(run_line_height);

            for c in chars {
                // Handle newlines
                if c == '\n' {
                    self.add_line(
                        &mut current_line_glyphs,
                        &current_font,
                        current_y,
                        line_height,
                    );
                    current_x = 0.0;
                    current_y += line_height;
                    line_height = run_line_height;
                    continue;
                }

                // Check for word wrap
                let advance = char_width + letter_spacing;
                if current_x + advance > width && current_x > 0.0 {
                    // Word wrap
                    self.add_line(
                        &mut current_line_glyphs,
                        &current_font,
                        current_y,
                        line_height,
                    );
                    current_x = 0.0;
                    current_y += line_height;

                    // Check max lines
                    if self.style.max_lines > 0 && self.lines.len() >= self.style.max_lines {
                        self.laid_out = true;
                        self.height = current_y;
                        return;
                    }
                }

                let glyph_id = font.char_to_glyph(c);
                current_line_glyphs.push((glyph_id, Point::new(current_x, 0.0)));
                current_x += advance;

                // Extra spacing for space characters
                if c == ' ' {
                    current_x += word_spacing;
                }
            }
        }

        // Finish last line
        if !current_line_glyphs.is_empty() {
            self.add_line(
                &mut current_line_glyphs,
                &current_font,
                current_y,
                line_height,
            );
            current_y += line_height;
        }

        self.height = current_y;
        self.laid_out = true;
    }

    fn add_line(&mut self, glyphs: &mut Vec<(u16, Point)>, font: &Font, y: Scalar, height: Scalar) {
        if glyphs.is_empty() {
            return;
        }

        let metrics = font.metrics();
        let baseline = y - metrics.ascent;

        // Calculate line width
        let line_width = glyphs
            .last()
            .map(|(_, p)| p.x + font.size() * 0.5)
            .unwrap_or(0.0);

        // Apply text alignment
        let x_offset = match self.style.text_align {
            TextAlign::Left | TextAlign::Start => 0.0,
            TextAlign::Right | TextAlign::End => self.width - line_width,
            TextAlign::Center => (self.width - line_width) / 2.0,
            TextAlign::Justify => 0.0, // Would need more complex handling
        };

        // Offset glyphs
        let adjusted_glyphs: Vec<(u16, Point)> = glyphs
            .iter()
            .map(|(g, p)| (*g, Point::new(p.x + x_offset, p.y)))
            .collect();

        self.lines.push(TextLine {
            glyphs: adjusted_glyphs,
            font: font.clone(),
            bounds: Rect::from_xywh(0.0, y, self.width, height),
            baseline,
        });

        glyphs.clear();
    }

    /// Get the laid-out width.
    pub fn max_intrinsic_width(&self) -> Scalar {
        self.width
    }

    /// Get the laid-out height.
    pub fn height(&self) -> Scalar {
        self.height
    }

    /// Get the number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get the line height for a specific line.
    pub fn line_height(&self, line: usize) -> Option<Scalar> {
        self.lines.get(line).map(|l| l.bounds.height())
    }

    /// Get the width of a specific line.
    pub fn line_width(&self, line: usize) -> Option<Scalar> {
        self.lines.get(line).map(|l| {
            l.glyphs
                .last()
                .map(|(_, p)| p.x + l.font.size() * 0.5)
                .unwrap_or(0.0)
        })
    }

    /// Convert the paragraph to a text blob for drawing.
    pub fn to_text_blob(&self) -> Option<TextBlob> {
        if !self.laid_out || self.lines.is_empty() {
            return None;
        }

        let mut builder = TextBlobBuilder::new();

        for line in &self.lines {
            let positions: Vec<Point> = line
                .glyphs
                .iter()
                .map(|(_, p)| Point::new(p.x, line.baseline + p.y))
                .collect();

            let glyphs: Vec<u16> = line.glyphs.iter().map(|(g, _)| *g).collect();

            builder.add_positioned_run(&line.font, &glyphs, &positions);
        }

        builder.build()
    }

    /// Get the bounding box of the laid-out text.
    pub fn bounds(&self) -> Rect {
        Rect::from_xywh(0.0, 0.0, self.width, self.height)
    }
}

// =============================================================================
// Line Breaking
// =============================================================================

/// Line breaker for finding valid break points in text.
pub struct LineBreaker {
    /// Break opportunities (byte offsets).
    breaks: Vec<usize>,
}

impl LineBreaker {
    /// Create a new line breaker for the given text.
    pub fn new(text: &str) -> Self {
        let mut breaks = Vec::new();
        let mut last_was_space = false;

        for (i, c) in text.char_indices() {
            // Simple line breaking: break after spaces and hyphens
            if last_was_space && !c.is_whitespace() {
                breaks.push(i);
            }
            if c == '-' {
                breaks.push(i + c.len_utf8());
            }
            last_was_space = c.is_whitespace();
        }

        // Always allow break at end
        breaks.push(text.len());

        Self { breaks }
    }

    /// Get all break opportunities.
    pub fn breaks(&self) -> &[usize] {
        &self.breaks
    }

    /// Find the best break point before a given position.
    pub fn find_break_before(&self, pos: usize) -> usize {
        self.breaks
            .iter()
            .filter(|&&b| b <= pos)
            .copied()
            .last()
            .unwrap_or(0)
    }
}

// =============================================================================
// Hyphenation
// =============================================================================

/// Simple hyphenation support.
pub struct Hyphenator {
    /// Minimum characters before hyphen.
    min_prefix: usize,
    /// Minimum characters after hyphen.
    min_suffix: usize,
}

impl Default for Hyphenator {
    fn default() -> Self {
        Self {
            min_prefix: 2,
            min_suffix: 3,
        }
    }
}

impl Hyphenator {
    /// Create a new hyphenator.
    pub fn new(min_prefix: usize, min_suffix: usize) -> Self {
        Self {
            min_prefix,
            min_suffix,
        }
    }

    /// Find hyphenation points in a word.
    ///
    /// Returns byte offsets where hyphens can be inserted.
    pub fn hyphenate(&self, word: &str) -> Vec<usize> {
        let chars: Vec<char> = word.chars().collect();
        let char_count = chars.len();

        if char_count < self.min_prefix + self.min_suffix {
            return Vec::new();
        }

        let mut points = Vec::new();
        let mut byte_offset = 0;

        for (i, &c) in chars.iter().enumerate() {
            byte_offset += c.len_utf8();

            // Simple rule: allow hyphenation between vowels and consonants
            if i >= self.min_prefix && char_count - i - 1 >= self.min_suffix {
                if is_vowel(c) != is_vowel(chars.get(i + 1).copied().unwrap_or('x')) {
                    points.push(byte_offset);
                }
            }
        }

        points
    }
}

fn is_vowel(c: char) -> bool {
    matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_builder() {
        let style = ParagraphStyle::default();
        let mut builder = ParagraphBuilder::new(style);
        builder.add_text("Hello, world!");
        let paragraph = builder.build();
        assert!(!paragraph.laid_out);
    }

    #[test]
    fn test_paragraph_layout() {
        let style = ParagraphStyle::default();
        let mut builder = ParagraphBuilder::new(style);
        builder.add_text("Hello, world! This is a test.");
        let mut paragraph = builder.build();
        paragraph.layout(100.0);

        assert!(paragraph.laid_out);
        assert!(paragraph.height() > 0.0);
    }

    #[test]
    fn test_line_breaker() {
        let breaker = LineBreaker::new("Hello world");
        assert!(!breaker.breaks().is_empty());
    }

    #[test]
    fn test_hyphenator() {
        let hyphenator = Hyphenator::default();
        let points = hyphenator.hyphenate("hyphenation");
        // Should find some hyphenation points in a long word
        assert!(!points.is_empty() || "hyphenation".len() < 5);
    }
}
