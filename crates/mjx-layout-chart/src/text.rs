//! How wide a tick label is — the one input to plot-area negotiation this crate cannot compute.
//!
//! # Why a trait and not `mjx-text`
//!
//! Reserving room for a value axis' labels means knowing how wide `"1,203"` is, and knowing that
//! properly means a face, a shaper and a font database — all of which each box model already holds
//! and none of which this crate should acquire a second copy of. So the measurement is a **seam**:
//! [`TextMetrics`] is what a host implements over whatever text engine it already has.
//!
//! # What the three hosts pass today, and why that is a limitation rather than a design
//!
//! All three pass [`NominalMetrics`], which measures a string from a per-character advance table
//! rather than from shaped glyphs. It is deterministic, needs no font, and is therefore what makes
//! `tests/one_engine_three_formats.rs` an *equality* rather than an approximate comparison — but it
//! is an estimate, and a chart whose longest tick label is a long currency string will reserve a few
//! percent too much or too little of its width. Handing in a real metric is one `impl` per host and
//! changes nothing else here; `tests/fragments_reach_the_tree.rs` proves the seam is live by passing
//! a deliberately wide metric and watching the plot area shrink.

use mjx_layout::LayoutSize;
use mjx_ooxml_core::measure::Emu;

/// A run of chart text and the size it is set at.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChartText<'a> {
    /// The text itself.
    pub text: &'a str,
    /// Its size, in points. Office's default for chart text is ten points, scaled by `c:autoTitleDeleted`
    /// and the chart's own text properties — neither of which this engine reads, so every caller
    /// passes one of [`ChartTextRole`]'s sizes.
    pub size_points: f64,
    /// Whether it is drawn bold, which widens it.
    pub bold: bool,
}

/// What a piece of chart text is for, which is what decides how big it is.
///
/// The three sizes are Office's defaults for a chart at its natural size. `GUESS:` they are what a
/// chart authored by this workspace's own `add_chart` renders at in PowerPoint; ECMA-376 states no
/// default size for chart text, because the size lives in a `c:txPr` the file may or may not carry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChartTextRole {
    /// The chart's own title.
    ChartTitle,
    /// An axis' title.
    AxisTitle,
    /// A tick label, a legend entry, or a data label.
    Label,
}

impl ChartTextRole {
    /// The point size text in this role is set at.
    #[must_use]
    pub fn size_points(self) -> f64 {
        match self {
            Self::ChartTitle => 14.0,
            Self::AxisTitle => 10.0,
            Self::Label => 10.0,
        }
    }

    /// Whether text in this role is bold.
    #[must_use]
    pub fn is_bold(self) -> bool {
        matches!(self, Self::ChartTitle)
    }

    /// The text of `content` in this role.
    #[must_use]
    pub fn text(self, content: &str) -> ChartText<'_> {
        ChartText {
            text: content,
            size_points: self.size_points(),
            bold: self.is_bold(),
        }
    }
}

/// How wide and tall a piece of chart text is.
///
/// Implemented by whatever holds a text engine. A host that has none passes [`NominalMetrics`].
pub trait TextMetrics {
    /// The advance width and line height of `text`, on one line, unwrapped.
    ///
    /// It is measured unwrapped because chart text is: a tick label that does not fit is rotated or
    /// skipped rather than broken, and a title that does not fit is what makes the plot area
    /// shorter.
    fn measure(&mut self, text: ChartText<'_>) -> LayoutSize;
}

impl<T: TextMetrics + ?Sized> TextMetrics for &mut T {
    fn measure(&mut self, text: ChartText<'_>) -> LayoutSize {
        (**self).measure(text)
    }
}

/// A font-free metric: each character contributes a fraction of the em, from a small table.
///
/// # What the table is
///
/// The ratios below are the advance widths of a handful of representative characters in Calibri,
/// rounded to two places, grouped so that the classes a chart's text actually contains — digits, a
/// decimal point, a thousands separator, a percent sign, upper case, lower case, a space — each get
/// their own. Everything else takes the lower-case ratio, which is the middle of the distribution.
/// `GUESS:` the grouping is this engine's, not a published table.
///
/// Digits are given **one** ratio rather than per-digit ones, which is not an approximation: every
/// digit in a text face intended for numbers is the same width, so that a column of figures lines
/// up. That is why a tick label's width is the one thing this estimate gets close to exactly right,
/// and tick labels are what the plot-area reservation is mostly made of.
#[derive(Clone, Copy, Debug, Default)]
pub struct NominalMetrics;

impl NominalMetrics {
    /// The fraction of the em one character advances.
    fn advance(character: char) -> f64 {
        match character {
            '0'..='9' => 0.51,
            ' ' | '\u{00a0}' => 0.23,
            '.' | ',' | '\'' | '!' | 'i' | 'j' | 'l' | 'I' | '|' => 0.23,
            '%' => 0.79,
            '-' | '\u{2212}' => 0.31,
            'W' | 'M' | 'm' | 'w' | '@' => 0.83,
            'A'..='Z' => 0.60,
            _ => 0.48,
        }
    }
}

impl TextMetrics for NominalMetrics {
    fn measure(&mut self, text: ChartText<'_>) -> LayoutSize {
        if text.text.is_empty() {
            return LayoutSize::ZERO;
        }
        let em = text.size_points.clamp(1.0, 400.0);
        let widening = if text.bold { 1.06 } else { 1.0 };
        let advance: f64 = text.text.chars().take(512).map(Self::advance).sum();
        LayoutSize::new(
            Emu::from_points(advance * em * widening),
            // 1.2 em is the line height a text engine reports for a face with no explicit line gap,
            // which is what chart text is set with.
            Emu::from_points(em * 1.2),
        )
    }
}
