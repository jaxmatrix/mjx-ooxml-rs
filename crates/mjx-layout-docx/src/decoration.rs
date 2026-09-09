//! What paints a paragraph: its shading and its six rules, behind a handle nothing here resolves.
//!
//! # A handle, not a paint
//!
//! A [`DecorationRef`] is a bare number. This crate issues them and a scene companion turns them
//! into paints — the same division `mjx-layout-pptx` and `mjx-layout-xlsx` make, and for the same
//! reason: a box model that produced a fill would have merged two stages the architecture separates
//! on purpose, and `tests/the_seam_holds.rs` refuses `mjx-scene` by name.
//!
//! **There is no companion for Word yet**, and that is stated rather than implied: the handles here
//! are correct, deduplicated and addressable, and nothing resolves them until `mjx-scene-docx`
//! exists. See the crate documentation.
//!
//! # A rule half a stroke wide
//!
//! A border is drawn **astride** the edge it belongs to, so the geometry a painter strokes is the
//! box's own rectangle moved in by half the stroke. Halving is where a hairline disappears:
//! `mjx-layout-xlsx` found a width floored at one EMU halving to zero and drawing nothing at all,
//! with no error anywhere, and Word has six paragraph borders and a `bar` tab that is nothing but a
//! rule. [`crate::measure::half_of`] is the answer and [`stroke_rect`] is where it is used.

use mjx_docx::{EffectiveBorder, EffectiveParagraphProperties, EffectiveShading};
use mjx_layout::{DecorationRef, LayoutRect};
use mjx_ooxml_core::measure::Emu;

use crate::measure::{border_width, half_of};

/// One of a paragraph's six rules, resolved to a width and a style.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rule {
    /// How wide the stroke is. Never zero — `w:sz="0"` means *the thinnest visible line*, and the
    /// absence of a line is `w:val="none"`, which is a different attribute and produces no [`Rule`]
    /// at all.
    pub width: Emu,
    /// How far the rule sits from the text, `w:space`, in points converted to a length.
    pub space: Emu,
    /// The border as the document stated it, colour and style included.
    pub border: EffectiveBorder,
}

impl Rule {
    /// The rule a `w:pBdr` edge states, or `None` when the edge is absent or explicitly `none`.
    #[must_use]
    pub fn of(border: Option<&EffectiveBorder>) -> Option<Self> {
        let border = border?;
        if border.style == mjx_ooxml_types::wordprocessingml::BorderStyle::None {
            return None;
        }
        Some(Self {
            width: border_width(border.width_eighths_of_a_point.as_ref()),
            space: Emu::from_points(i64::try_from(border.spacing_points).map_or(0.0, |points| {
                #[allow(clippy::cast_precision_loss)]
                {
                    points as f64
                }
            })),
            border: border.clone(),
        })
    }
}

/// Everything that paints one paragraph.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct ParagraphDecoration {
    /// `w:shd`.
    pub shading: Option<EffectiveShading>,
    /// `w:pBdr/w:top`.
    pub top: Option<Rule>,
    /// `w:pBdr/w:left`.
    pub left: Option<Rule>,
    /// `w:pBdr/w:bottom`.
    pub bottom: Option<Rule>,
    /// `w:pBdr/w:right`.
    pub right: Option<Rule>,
    /// `w:pBdr/w:between` — drawn between two consecutive paragraphs that share the border, and
    /// **reported rather than placed** in this child: which pair of paragraphs shares one is a
    /// question about neighbours across a page boundary, and answering it wrongly draws a rule
    /// through the middle of a page.
    pub between: Option<Rule>,
    /// `w:pBdr/w:bar`.
    pub bar: Option<Rule>,
}

impl ParagraphDecoration {
    /// What a paragraph's effective properties paint.
    #[must_use]
    pub fn of(properties: &EffectiveParagraphProperties) -> Self {
        let borders = properties.borders.as_ref();
        Self {
            shading: properties.shading.clone(),
            top: borders.and_then(|edges| Rule::of(edges.top.as_ref())),
            left: borders.and_then(|edges| Rule::of(edges.left.as_ref())),
            bottom: borders.and_then(|edges| Rule::of(edges.bottom.as_ref())),
            right: borders.and_then(|edges| Rule::of(edges.right.as_ref())),
            between: borders.and_then(|edges| Rule::of(edges.between.as_ref())),
            bar: borders.and_then(|edges| Rule::of(edges.bar.as_ref())),
        }
    }

    /// Whether it paints anything at all. A paragraph that paints nothing gets no handle, which is
    /// what keeps a decoration table the size of the paragraphs that are actually decorated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The rectangle a painter strokes `rule` along, for the box whose border box is `rect`.
///
/// Moved in by **half** the stroke on every edge that has one, so the outer half of the stroke lands
/// on the border box's own edge — which is what a border box is. This is the call
/// [`crate::measure::half_of`] exists for; a plain division by two turns a one-EMU rule into a
/// zero-EMU one, which draws nothing.
#[must_use]
pub fn stroke_rect(rect: LayoutRect, decoration: &ParagraphDecoration) -> LayoutRect {
    let inset = |rule: Option<&Rule>| rule.map_or(Emu::ZERO, |rule| half_of(rule.width));
    LayoutRect::from_edges(
        rect.left + inset(decoration.left.as_ref()),
        rect.top + inset(decoration.top.as_ref()),
        rect.right - inset(decoration.right.as_ref()),
        rect.bottom - inset(decoration.bottom.as_ref()),
    )
}

/// Every decoration a page issued a handle for, in the order the handles were issued.
///
/// Deduplicated by value, so a hundred paragraphs of one style share one handle — the same measured
/// sharing `mjx-layout-xlsx` makes for a cell format, and for the same reason: the table is what a
/// companion uploads, and a table with one entry per paragraph would upload the same fill a hundred
/// times.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct DecorationCatalogue {
    entries: Vec<ParagraphDecoration>,
}

impl DecorationCatalogue {
    /// An empty catalogue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The handle for `decoration`, issuing one only if this decoration is new.
    ///
    /// `None` for a decoration that paints nothing.
    pub fn intern(&mut self, decoration: ParagraphDecoration) -> Option<DecorationRef> {
        if decoration.is_empty() {
            return None;
        }
        let index = self
            .entries
            .iter()
            .position(|existing| existing == &decoration)
            .unwrap_or_else(|| {
                self.entries.push(decoration);
                self.entries.len() - 1
            });
        Some(DecorationRef::new(index as u64))
    }

    /// What `handle` means.
    #[must_use]
    pub fn get(&self, handle: DecorationRef) -> Option<&ParagraphDecoration> {
        usize::try_from(handle.number())
            .ok()
            .and_then(|index| self.entries.get(index))
    }

    /// How many distinct decorations the page holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing on the page is decorated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
