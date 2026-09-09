//! Projecting a resolved paragraph or run into the vocabulary a layout engine and a font engine
//! speak.
//!
//! # Nothing here resolves anything
//!
//! Every value read below was resolved by `mjx-docx`: the whole ladder — `w:docDefaults`, the
//! numbering level's own `w:pPr`/`w:rPr`, the paragraph style's `w:basedOn` chain, the character
//! style's chain, direct formatting, and the twelve toggle properties' XOR recombination — has
//! already run. What happens here is a *projection*: a wire string becomes a length, an
//! `Option<bool>` becomes a decision, and `ST_Jc` becomes something the justifier can switch on.
//!
//! The one thing this module does decide is **what an unstated value means**, and every such
//! decision is either a schema default (stated as one) or a `GUESS:` about Word.

use mjx_docx::{
    EffectiveCharacterProperties, EffectiveParagraphProperties, EffectiveTabStop, LineSpacing,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingml::{Justification, LineSpacingRule};
use mjx_text::{FontSize, FontSlant, FontWeight, TextDirection};

use crate::measure::{half_points, signed_twips_measure, twips_measure};

/// The family a run asks for when nothing anywhere names one.
///
/// `mjx-text`'s resolver treats an unknown family as a substitution and answers from the bundled
/// tier, so this is *a name to record in the manifest* rather than a face this crate picks. Naming a
/// concrete face here would override the branding of whoever opens the file, which is the one thing
/// a renderer must never do.
pub const UNNAMED_FAMILY: &str = "";

/// The size a run renders at when no tier of the ladder states one.
///
/// ECMA-376 gives `w:sz` **no** default. Ten points is what Word writes into a document whose
/// `w:docDefaults` states no size, and it is a **GUESS: about Word** rather than a value from the
/// specification. It is only ever reached by a run that inherits nothing at all, which a document
/// Word authored never has.
pub const ASSUMED_FONT_SIZE_POINTS: f64 = 10.0;

/// How tall a line is, once the paragraph has had its say.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineHeight {
    /// `w:lineRule="auto"` — a multiple of the line's own natural height. `w:line="240"` is single
    /// spacing, `"360"` is one and a half, `"480"` is double; the unit is 240ths of a line.
    Multiple(f64),
    /// `w:lineRule="exact"` — this height, whatever is on the line. Text taller than it is clipped
    /// by the line above, which is what Word does and what a reader sees.
    Exact(Emu),
    /// `w:lineRule="atLeast"` — this height, or the line's natural height if that is greater.
    AtLeast(Emu),
}

impl LineHeight {
    /// Single spacing: what a paragraph that states no `w:spacing/w:line` gets.
    pub const SINGLE: Self = Self::Multiple(1.0);

    /// The height a line of natural height `natural` is drawn at.
    #[must_use]
    pub fn applied_to(self, natural: Emu) -> Emu {
        match self {
            Self::Multiple(factor) => natural.scaled_by(factor.max(0.0)),
            Self::Exact(height) => height.maximum(Emu::ZERO),
            Self::AtLeast(height) => natural.maximum(height),
        }
    }
}

/// Which way a line is pushed against its measure.
///
/// `ST_Jc` has twelve members and four of them are Arabic kashida elongation, which needs a shaper
/// feature this project does not yet drive. They are mapped rather than dropped — see
/// [`Alignment::of`] — and the mapping is marked where it is made.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alignment {
    /// The line starts at the leading margin. `start`, and `left` in a left-to-right paragraph.
    Start,
    /// Centred in the measure.
    Centre,
    /// The line ends at the trailing margin.
    End,
    /// Every line but the last fills the measure by widening the gaps between words.
    Justified,
    /// Every line **including the last** fills the measure, and the gaps widened are between
    /// characters as well as words. `ST_Jc`'s `distribute`, and East Asian typesetting's normal
    /// justification.
    Distributed,
}

impl Alignment {
    /// What `w:jc` means, in a paragraph whose base direction is `direction`.
    ///
    /// `left` and `right` are **not** synonyms for `start` and `end`: ECMA-376 keeps all four
    /// because `left` means the physical left edge whatever the paragraph's direction is. So in a
    /// right-to-left paragraph, `left` is the *trailing* margin and maps to [`Alignment::End`].
    #[must_use]
    pub fn of(value: Option<Justification>, direction: TextDirection) -> Self {
        let right_to_left = direction == TextDirection::RightToLeft;
        match value {
            None | Some(Justification::Start) => Self::Start,
            Some(Justification::End) => Self::End,
            Some(Justification::Center) => Self::Centre,
            Some(Justification::Left) => {
                if right_to_left {
                    Self::End
                } else {
                    Self::Start
                }
            }
            Some(Justification::Right) => {
                if right_to_left {
                    Self::Start
                } else {
                    Self::End
                }
            }
            Some(Justification::Justified) => Self::Justified,
            Some(Justification::Distribute | Justification::ThaiDistribute) => Self::Distributed,
            // GUESS: the three kashida settings and `numTab` are treated as ordinary justification.
            // Kashida elongation lengthens an Arabic glyph rather than a gap, which needs a shaper
            // feature nothing in this project drives yet; `numTab` aligns to a list's own tab, which
            // is numbering and therefore R22's. Both are *justified* in Word, so justifying them is
            // the closest wrong answer rather than an arbitrary one — and it is wrong, visibly, for
            // an Arabic paragraph.
            Some(
                Justification::MediumKashida
                | Justification::WidestKashida
                | Justification::LowKashida
                | Justification::AlignToListTab,
            ) => Self::Justified,
        }
    }

    /// Whether the paragraph's **last** line is stretched to the measure too.
    #[must_use]
    pub fn stretches_the_last_line(self) -> bool {
        matches!(self, Self::Distributed)
    }
}

/// One paragraph's layout, projected out of its effective properties.
#[derive(Clone, PartialEq, Debug)]
pub struct ParagraphStyle {
    /// Where its lines are pushed.
    pub alignment: Alignment,
    /// Its base direction — `w:bidi`.
    pub direction: TextDirection,
    /// `w:ind@start`/`@left` — the leading indent of every line but the first.
    pub indent_start: Emu,
    /// `w:ind@end`/`@right` — the trailing indent of every line.
    pub indent_end: Emu,
    /// How much further in (positive) or out (negative) the **first** line starts, relative to
    /// [`ParagraphStyle::indent_start`]. `w:ind@firstLine` is positive and `w:ind@hanging` negative;
    /// they cannot both apply, and `w:hanging` wins.
    pub first_line_offset: Emu,
    /// `w:spacing@before`.
    pub space_before: Emu,
    /// `w:spacing@after`.
    pub space_after: Emu,
    /// `w:contextualSpacing` — suppress the space between this paragraph and a neighbour of the
    /// same style.
    pub contextual_spacing: bool,
    /// `w:spacing@line` and `@lineRule`, together.
    pub line_height: LineHeight,
    /// `w:keepNext`.
    pub keep_with_next: bool,
    /// `w:keepLines`.
    pub keep_lines_together: bool,
    /// `w:pageBreakBefore`.
    pub page_break_before: bool,
    /// `w:widowControl` — on unless the document turns it off (§17.3.1.44's own default).
    pub widow_control: bool,
    /// `w:suppressLineNumbers` — this paragraph's lines are skipped by the section's line numbering
    /// **and do not advance the count**, which is what "suppress" means: a numbered line after a
    /// suppressed one carries the number it would have carried had the suppressed one not existed.
    pub suppress_line_numbers: bool,
    /// `w:suppressAutoHyphens` — this paragraph opts out of the document's hyphenation.
    pub suppress_auto_hyphens: bool,
    /// `w:kinsoku` — apply the East Asian line-breaking prohibitions.
    pub east_asian_line_breaking: bool,
    /// `w:overflowPunct` — let trailing punctuation hang past the measure.
    pub overflow_punctuation: bool,
    /// `w:tabs`, as the document stated them.
    pub tab_stops: Vec<EffectiveTabStop>,
}

impl ParagraphStyle {
    /// The style of a paragraph whose effective properties are `properties`.
    #[must_use]
    pub fn of(properties: &EffectiveParagraphProperties) -> Self {
        let direction = if properties.right_to_left_layout.unwrap_or(false) {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        };
        let indentation = properties.indentation.as_ref();
        let indent_start = indentation
            .and_then(|indent| {
                indent
                    .start
                    .as_ref()
                    .or(indent.left.as_ref())
                    .and_then(signed_twips_measure)
            })
            .unwrap_or(Emu::ZERO);
        let indent_end = indentation
            .and_then(|indent| {
                indent
                    .end
                    .as_ref()
                    .or(indent.right.as_ref())
                    .and_then(signed_twips_measure)
            })
            .unwrap_or(Emu::ZERO);
        // §17.3.1.12: `w:hanging` and `w:firstLine` are mutually exclusive and `w:hanging` wins when
        // a non-conformant file writes both.
        let first_line_offset = indentation
            .and_then(|indent| {
                indent
                    .hanging
                    .as_ref()
                    .and_then(twips_measure)
                    .map(|hanging| Emu::ZERO - hanging)
                    .or_else(|| indent.first_line.as_ref().and_then(twips_measure))
            })
            .unwrap_or(Emu::ZERO);

        let spacing = properties.spacing.as_ref();
        Self {
            alignment: Alignment::of(properties.alignment, direction),
            direction,
            indent_start,
            indent_end,
            first_line_offset,
            space_before: spacing
                .and_then(|value| value.before.as_ref().and_then(twips_measure))
                .unwrap_or(Emu::ZERO),
            space_after: spacing
                .and_then(|value| value.after.as_ref().and_then(twips_measure))
                .unwrap_or(Emu::ZERO),
            contextual_spacing: properties.contextual_spacing.unwrap_or(false),
            line_height: spacing
                .and_then(|value| value.line.as_ref())
                .map_or(LineHeight::SINGLE, line_height_of),
            keep_with_next: properties.keep_with_next.unwrap_or(false),
            keep_lines_together: properties.keep_lines_together.unwrap_or(false),
            page_break_before: properties.page_break_before.unwrap_or(false),
            // §17.3.1.44: `w:widowControl`'s own default is **on**. Reading an absent element as off
            // would turn widow and orphan control off for every document that does not write it,
            // which is most of them — and the difference is a page assignment, not a pixel.
            widow_control: properties.widow_control.unwrap_or(true),
            suppress_line_numbers: properties.suppress_line_numbers.unwrap_or(false),
            suppress_auto_hyphens: properties.suppress_auto_hyphens.unwrap_or(false),
            east_asian_line_breaking: properties.east_asian_line_breaking_rules.unwrap_or(false),
            overflow_punctuation: properties.overflow_punctuation.unwrap_or(false),
            tab_stops: properties.tab_stops.clone().unwrap_or_default(),
        }
    }

    /// The measure — the width available to text — of line number `line` of this paragraph, inside a
    /// column `column_width` wide.
    ///
    /// Never negative and never zero: a paragraph indented past its own column would otherwise
    /// produce a measure no glyph fits in, and the fitting loop would take one opportunity per
    /// candidate for ever. One EMU is the floor, and a line that does not fit it overflows — which
    /// is what Word shows too.
    #[must_use]
    pub fn measure_of(&self, line: usize, column_width: Emu) -> Emu {
        let first_line = if line == 0 {
            self.first_line_offset
        } else {
            Emu::ZERO
        };
        let available = column_width - self.indent_start - self.indent_end - first_line;
        available.maximum(Emu::from_emu(1))
    }

    /// Where line number `line` starts, measured from the column's leading edge.
    #[must_use]
    pub fn leading_indent_of(&self, line: usize) -> Emu {
        if line == 0 {
            self.indent_start + self.first_line_offset
        } else {
            self.indent_start
        }
    }
}

fn line_height_of(spacing: &LineSpacing) -> LineHeight {
    let Some(value) = signed_twips_measure(&spacing.value) else {
        return LineHeight::SINGLE;
    };
    match spacing.rule {
        // §17.3.1.33: with `auto`, `w:line` is in 240ths of a line rather than in twips. That is the
        // one place in WordprocessingML where a `ST_SignedTwipsMeasure` is not a length, and reading
        // it as one turns single spacing into 240 twips — a twelve-point line, which looks almost
        // right and is not.
        #[allow(clippy::cast_precision_loss)]
        LineSpacingRule::Auto => LineHeight::Multiple(value.twips() as f64 / 240.0),
        LineSpacingRule::Exact => LineHeight::Exact(value),
        LineSpacingRule::AtLeast => LineHeight::AtLeast(value),
    }
}

/// A run of a paragraph as the font engine wants it: a family, a size and two style axes.
#[derive(Clone, PartialEq, Debug)]
pub struct RunStyle {
    /// The bytes of the paragraph's text it covers.
    pub range: std::ops::Range<usize>,
    /// The family the document asked for. Empty when no tier named one.
    pub family: String,
    /// The size it renders at.
    pub size: FontSize,
    /// Its weight.
    pub weight: FontWeight,
    /// Its slant.
    pub slant: FontSlant,
    /// The language tag it declares, for the shaper's language-sensitive features.
    pub language: Option<String>,
    /// `w:vanish`/`w:webHidden` — whether the run is hidden and contributes no glyphs.
    pub hidden: bool,
}

impl RunStyle {
    /// The style of a run whose effective character properties are `properties`.
    #[must_use]
    pub fn of(range: std::ops::Range<usize>, properties: &EffectiveCharacterProperties) -> Self {
        // GUESS: the **ASCII** slot is what is asked for, with the High ANSI slot behind it.
        // `w:rFonts` states four families and `w:hint` says which wins for an ambiguous character;
        // `mjx-text`'s own itemisation already does per-character face fallback from one request, so
        // asking for the East Asian family as well would be a second fallback policy fighting the
        // first. A document whose East Asian slot names a font its ASCII slot does not will
        // therefore fall back rather than use the stated face, and that is visible in a mixed
        // Japanese paragraph.
        let family = properties
            .fonts
            .as_ref()
            .and_then(|fonts| {
                fonts
                    .ascii
                    .clone()
                    .or_else(|| fonts.high_ansi.clone())
                    .or_else(|| fonts.east_asian.clone())
                    .or_else(|| fonts.complex_script.clone())
            })
            .unwrap_or_else(|| UNNAMED_FAMILY.to_owned());
        Self {
            range,
            family,
            size: properties
                .font_size
                .as_ref()
                .and_then(half_points)
                .map_or_else(
                    || FontSize::from_points(ASSUMED_FONT_SIZE_POINTS),
                    FontSize::from_points,
                ),
            weight: if properties.bold.unwrap_or(false) {
                FontWeight::BOLD
            } else {
                FontWeight::REGULAR
            },
            slant: if properties.italic.unwrap_or(false) {
                FontSlant::Italic
            } else {
                FontSlant::Upright
            },
            language: properties
                .languages
                .as_ref()
                .and_then(|languages| languages.latin.as_ref())
                .map(|tag| tag.to_wire().to_owned()),
            hidden: properties.hidden.unwrap_or(false)
                || properties.web_hidden.unwrap_or(false)
                || properties.always_hidden.unwrap_or(false),
        }
    }
}
