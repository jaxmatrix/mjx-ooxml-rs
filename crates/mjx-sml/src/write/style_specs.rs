//! Plain-data descriptions of the four `styles.xml` resources a caller can append.
//!
//! # Why a description and not the model
//!
//! [`crate::Font`], [`crate::Fill`], [`crate::Border`] and [`crate::CellFormat`] are *markup* —
//! each keeps the [`RawName`](mjx_ooxml_core::RawName)
//! it was read with, and every name in one is a symbol interned in the document the part was parsed
//! from. Constructing one therefore needs that exact [`Interner`], which a caller of
//! `Workbook::append_border` does not hold and should not have to.
//!
//! So the authoring vocabulary is these four `…Spec` structs: public fields, no interner, no
//! lifetime, `Default` throughout, and one `build` method each that turns a description into markup
//! *inside* the part that will hold it. `MJXOFF-97` already set the precedent with
//! [`RichTextRunSpec`](crate::RichTextRunSpec), for the same reason — a run's `rPr` is markup and
//! its description is not.
//!
//! Fonts are the exception, and deliberately: [`FontProperties`](crate::FontProperties) is *already*
//! that description — a plain struct with fifteen `Option` fields — and MJXOFF-105 already gave
//! [`Font::from_properties`](crate::Font::from_properties) the build step. A `FontSpec` here would
//! be a sixteenth copy of the same list.
//!
//! # Absent means absent
//!
//! Every field is an `Option` and `None` writes **no attribute and no element**. That is not
//! politeness: `<xf/>` is a meaningful record naming font 0, fill 0, border 0 and `General` by
//! omission, `<patternFill/>` states a fill whose pattern is inherited, and a border edge with no
//! `@style` is `none`. A builder that filled in defaults would author markup the caller did not ask
//! for, on a path whose whole point is that this project can explain every byte it emits.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::{BorderStyle, PatternType};

use crate::font::{Color, ColorElement};
use crate::styles::{Border, BorderEdge, CellFormat, Fill, PatternFill};

/// A `patternFill` to append: the pattern and the two colours it is drawn in.
///
/// `x:patternFill` (`CT_PatternFill`). A gradient fill is not described here — it carries a stop
/// list whose shape is a sequence rather than a record, and
/// [`GradientFill`](crate::GradientFill) is built directly for the callers that need one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PatternFillSpec {
    /// `@patternType`. `None` writes no attribute, which states a pattern the file leaves
    /// inherited — a third state beside `none` and `solid`.
    pub pattern: Option<PatternType>,
    /// `fgColor` — the colour a **solid** fill actually shows.
    pub foreground: Option<Color>,
    /// `bgColor`.
    pub background: Option<Color>,
}

impl PatternFillSpec {
    /// A solid fill in one opaque colour, the shape a caller filling a cell almost always wants.
    ///
    /// `hex` is the six-digit `RRGGBB` form; the opaque alpha is prefixed for it, because `@rgb` is
    /// eight digits and a six-digit value there is read as transparent.
    #[must_use]
    pub fn solid(hex: &str) -> Self {
        Self {
            pattern: Some(PatternType::Solid),
            foreground: Some(Color::from_opaque_rgb(hex)),
            background: None,
        }
    }

    /// Builds the `x:fill` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> Fill {
        let mut pattern_fill = PatternFill::new(interner, prefix);
        if let Some(pattern) = self.pattern {
            pattern_fill.set_pattern_type(interner, Some(pattern));
        }
        if let Some(color) = &self.foreground {
            pattern_fill.set_foreground_color(Some(ColorElement::named(
                interner, prefix, "fgColor", color,
            )));
        }
        if let Some(color) = &self.background {
            pattern_fill.set_background_color(Some(ColorElement::named(
                interner, prefix, "bgColor", color,
            )));
        }
        let mut fill = Fill::new(interner, prefix);
        fill.set_pattern(pattern_fill);
        fill
    }
}

/// One edge of a border to append: its line style and its colour.
///
/// `CT_BorderPr`. An edge with `style: None` writes `<left/>`, which is the schema's `none` — the
/// edge element is present and draws nothing, which is what a `border` with a styled `top` and a
/// plain `left` looks like in every file Excel writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BorderEdgeSpec {
    /// `@style`. `None` writes no attribute; the schema default is [`BorderStyle::None`].
    pub style: Option<BorderStyle>,
    /// `color` — absent means *automatic*, which Part 1 §18.8.4 states explicitly.
    pub color: Option<Color>,
}

impl BorderEdgeSpec {
    /// An edge in `style`, with no colour of its own.
    #[must_use]
    pub fn styled(style: BorderStyle) -> Self {
        Self {
            style: Some(style),
            color: None,
        }
    }

    /// Builds the edge element named `local`, interning its names into `interner`.
    #[must_use]
    fn build(&self, interner: &mut Interner, prefix: Option<&str>, local: &str) -> BorderEdge {
        let mut edge = BorderEdge::named(
            interner,
            prefix,
            local,
            self.style.unwrap_or(BorderStyle::None),
        );
        if self.style.is_none() {
            edge.set_style(interner, None);
        }
        if let Some(color) = &self.color {
            edge.set_color(Some(ColorElement::named(interner, prefix, "color", color)));
        }
        edge
    }
}

/// A `border` to append: **nine** edges, and the two diagonal-direction flags.
///
/// `x:border` (`CT_Border`). Nine, not four: `start` and `end` are the leading and trailing edges in
/// the reading direction (which `left` and `right` are only in a left-to-right sheet), and
/// `vertical` and `horizontal` are the *inner* borders of a range, which a `dxf` uses and a plain
/// cell border does not. A description that offered four would silently drop five slots of any
/// border it round-tripped.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BorderSpec {
    /// `start` — the **leading** edge in the reading direction, which is the left one in a
    /// left-to-right sheet.
    pub leading: Option<BorderEdgeSpec>,
    /// `end` — the **trailing** edge in the reading direction.
    pub trailing: Option<BorderEdgeSpec>,
    /// `left` — the physical left edge (§18.8.26).
    pub left: Option<BorderEdgeSpec>,
    /// `right` — the physical right edge (§18.8.34).
    pub right: Option<BorderEdgeSpec>,
    /// `top` (§18.8.43).
    pub top: Option<BorderEdgeSpec>,
    /// `bottom` (§18.8.6).
    pub bottom: Option<BorderEdgeSpec>,
    /// `diagonal` — the diagonal line's style and colour. **Which** diagonals are drawn is
    /// [`diagonal_up`](Self::diagonal_up) and [`diagonal_down`](Self::diagonal_down).
    pub diagonal: Option<BorderEdgeSpec>,
    /// `vertical` — the inner vertical border of a range (§18.8.44).
    pub vertical_inner: Option<BorderEdgeSpec>,
    /// `horizontal` — the inner horizontal border of a range (§18.8.22).
    pub horizontal_inner: Option<BorderEdgeSpec>,
    /// `@diagonalUp` — draw the bottom-left to top-right diagonal.
    pub diagonal_up: Option<bool>,
    /// `@diagonalDown` — draw the top-left to bottom-right diagonal.
    pub diagonal_down: Option<bool>,
    /// `@outline` — for a `dxf`, whether only the outside edges of the range are drawn.
    pub outline_only: Option<bool>,
}

impl BorderSpec {
    /// The border every workbook's index 0 is: all nine edges present, none of them styled.
    ///
    /// This is what Excel writes as the first `<border>` of every `styles.xml`, minus the four
    /// slots it omits — see [`skeleton_border`](Self::skeleton_border) for the exact five
    /// `mjx-chart`'s writer emits and the parity gate compares against.
    #[must_use]
    pub fn all_edges_plain() -> Self {
        let plain = Some(BorderEdgeSpec::default());
        Self {
            leading: plain.clone(),
            trailing: plain.clone(),
            left: plain.clone(),
            right: plain.clone(),
            top: plain.clone(),
            bottom: plain.clone(),
            diagonal: plain.clone(),
            vertical_inner: plain.clone(),
            horizontal_inner: plain,
            ..Self::default()
        }
    }

    /// The five edges Excel and `mjx-chart` both write for border 0: `left`, `right`, `top`,
    /// `bottom`, `diagonal`, each plain.
    ///
    /// `start`, `end`, `vertical` and `horizontal` are omitted, which is exactly what real files do
    /// — they are the reading-order and inner-range slots, and a cell border states neither.
    #[must_use]
    pub fn skeleton_border() -> Self {
        let plain = Some(BorderEdgeSpec::default());
        Self {
            left: plain.clone(),
            right: plain.clone(),
            top: plain.clone(),
            bottom: plain.clone(),
            diagonal: plain,
            ..Self::default()
        }
    }

    /// Builds the `x:border` this describes, interning its names into `interner`.
    ///
    /// Every edge goes in through the setter that places it at its rank in `CT_Border`'s
    /// `xsd:sequence`, so the nine come out in schema order however this struct was filled in.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> Border {
        let mut border = Border::new(interner, prefix);
        if let Some(flag) = self.diagonal_up {
            border.set_diagonal_up(interner, Some(flag));
        }
        if let Some(flag) = self.diagonal_down {
            border.set_diagonal_down(interner, Some(flag));
        }
        if let Some(flag) = self.outline_only {
            border.set_outline_only(interner, Some(flag));
        }
        let edges: [(&Option<BorderEdgeSpec>, &str); 9] = [
            (&self.leading, "start"),
            (&self.trailing, "end"),
            (&self.left, "left"),
            (&self.right, "right"),
            (&self.top, "top"),
            (&self.bottom, "bottom"),
            (&self.diagonal, "diagonal"),
            (&self.vertical_inner, "vertical"),
            (&self.horizontal_inner, "horizontal"),
        ];
        for (spec, local) in edges {
            let Some(spec) = spec else { continue };
            let edge = spec.build(interner, prefix, local);
            match local {
                "start" => border.set_leading_edge(Some(edge)),
                "end" => border.set_trailing_edge(Some(edge)),
                "left" => border.set_left_edge(Some(edge)),
                "right" => border.set_right_edge(Some(edge)),
                "top" => border.set_top_edge(Some(edge)),
                "bottom" => border.set_bottom_edge(Some(edge)),
                "diagonal" => border.set_diagonal_edge(Some(edge)),
                "vertical" => border.set_vertical_inner_edge(Some(edge)),
                // The array above is exhaustive over the nine locals, so this arm is `horizontal`.
                _ => border.set_horizontal_inner_edge(Some(edge)),
            }
        }
        border
    }
}

/// An `xf` to append to `cellXfs` or to `cellStyleXfs`: four resource indices, the `xfId` beneath
/// it, and the per-aspect apply flags.
///
/// `x:xf` (`CT_Xf`). Alignment and protection are not described here: both are attribute bags of
/// their own ([`CellAlignment`](crate::CellAlignment), [`CellProtection`](crate::CellProtection))
/// and a caller that wants either sets it on the built record through
/// [`CellFormat::set_alignment`](crate::CellFormat::set_alignment).
///
/// # `applies_*` is three-valued and this struct keeps it that way
///
/// `Option<bool>`: `None` writes no attribute, `Some(true)` writes `applyX="1"`, `Some(false)`
/// writes `applyX="0"`. §18.8.9 makes the distinction load-bearing — an absent flag *participates*
/// and a `0` suppresses — which is why [`ApplyFlag`](crate::ApplyFlag) exists on the read side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellFormatSpec {
    /// `@numFmtId` — a row of `numFmts`, or one of §18.8.30's implied codes.
    pub number_format_id: Option<u32>,
    /// `@fontId` — an index into `fonts`.
    pub font_index: Option<u32>,
    /// `@fillId` — an index into `fills`.
    pub fill_index: Option<u32>,
    /// `@borderId` — an index into `borders`.
    pub border_index: Option<u32>,
    /// `@xfId` — the `cellStyleXfs` record beneath this one. Meaningful on a `cellXfs` entry;
    /// `cellStyleXfs` entries do not carry it.
    pub cell_style_format_index: Option<u32>,
    /// `@quotePrefix` — the value is text because it was typed with a leading apostrophe.
    pub text_is_quote_prefixed: Option<bool>,
    /// `@applyNumberFormat`.
    pub applies_number_format: Option<bool>,
    /// `@applyFont`.
    pub applies_font: Option<bool>,
    /// `@applyFill`.
    pub applies_fill: Option<bool>,
    /// `@applyBorder`.
    pub applies_border: Option<bool>,
    /// `@applyAlignment`.
    pub applies_alignment: Option<bool>,
    /// `@applyProtection`.
    pub applies_protection: Option<bool>,
}

impl CellFormatSpec {
    /// The record `mjx-chart`'s writer emits and every workbook's `cellXfs[0]` is:
    /// `numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"`.
    #[must_use]
    pub fn skeleton_cell_format() -> Self {
        Self {
            number_format_id: Some(0),
            font_index: Some(0),
            fill_index: Some(0),
            border_index: Some(0),
            cell_style_format_index: Some(0),
            ..Self::default()
        }
    }

    /// The same record without `@xfId` — a `cellStyleXfs` entry, which has nothing beneath it.
    #[must_use]
    pub fn skeleton_cell_style_format() -> Self {
        Self {
            cell_style_format_index: None,
            ..Self::skeleton_cell_format()
        }
    }

    /// Builds the `x:xf` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> CellFormat {
        let mut format = CellFormat::new(interner, prefix);
        if let Some(value) = self.number_format_id {
            format.set_number_format_id(interner, Some(value));
        }
        if let Some(value) = self.font_index {
            format.set_font_index(interner, Some(value));
        }
        if let Some(value) = self.fill_index {
            format.set_fill_index(interner, Some(value));
        }
        if let Some(value) = self.border_index {
            format.set_border_index(interner, Some(value));
        }
        if let Some(value) = self.cell_style_format_index {
            format.set_cell_style_format_index(interner, Some(value));
        }
        if let Some(value) = self.text_is_quote_prefixed {
            format.set_text_is_quote_prefixed(interner, Some(value));
        }
        if let Some(value) = self.applies_number_format {
            format.set_applies_number_format(interner, Some(value));
        }
        if let Some(value) = self.applies_font {
            format.set_applies_font(interner, Some(value));
        }
        if let Some(value) = self.applies_fill {
            format.set_applies_fill(interner, Some(value));
        }
        if let Some(value) = self.applies_border {
            format.set_applies_border(interner, Some(value));
        }
        if let Some(value) = self.applies_alignment {
            format.set_applies_alignment(interner, Some(value));
        }
        if let Some(value) = self.applies_protection {
            format.set_applies_protection(interner, Some(value));
        }
        format
    }
}
