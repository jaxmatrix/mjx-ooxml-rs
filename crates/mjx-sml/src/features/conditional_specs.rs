//! Plain-data descriptions of the conditional formats a caller can author, and the one place a
//! `dxf` is allocated.
//!
//! # Why a description and not the model
//!
//! [`ConditionalFormattingRule`] is *markup*: it keeps the [`RawName`](mjx_ooxml_core::RawName) it
//! was read with, and every name in it is a symbol interned in the document the part was parsed
//! from. Constructing one therefore needs that exact [`Interner`], which a caller of a package-tier
//! `add_conditional_formatting` does not hold and should not have to. So the authoring vocabulary is
//! these five `…Spec` structs: public fields, no interner, no lifetime, `Default` where it means
//! something, and one `build` method each that turns a description into markup *inside* the part
//! that will hold it.
//!
//! `MJXOFF-105` set this precedent with [`PatternFillSpec`](crate::PatternFillSpec) and its three
//! siblings, and `MJXOFF-97` set it before that with
//! [`RichTextRunSpec`](crate::RichTextRunSpec), each for the same reason.
//!
//! # What is describable here, and what is not
//!
//! [`ConditionalRuleSpecKind`] carries the five rule kinds whose markup is **completely** stated by
//! what the caller passes: `cellIs`, `expression`, `colorScale`, `dataBar` and `iconSet`. The other
//! thirteen members of `ST_CfType` are deliberately absent, and their absence is a decision rather
//! than a gap: a `top10` rule needs `@rank`, `@bottom` and `@percent`, a `timePeriod` rule needs
//! `@timePeriod`, a `containsText` rule needs `@text` *and* a `formula` restating the same test —
//! so a variant that wrote only `type="top10"` would author markup that is knowingly incomplete,
//! which is worse than not offering it. Those kinds are authored through
//! [`ConditionalFormattingRule`] directly, which states every attribute the schema declares.
//!
//! # Appending a `dxf` never renumbers one
//!
//! A rule points at formatting by **index** — `@dxfId` — exactly as an `xf` points at a font. So
//! [`StylesheetPart::append_differential_format`] appends and returns the index it appended at, and
//! there is no call anywhere in this crate that reorders, inserts into, or removes from the `dxfs`
//! table. Every `@dxfId` in every rule, in every table style, in every part of the workbook, still
//! names what it named. That is [`crate::DifferentialFormats`]'s stated rule, and this is the
//! function that has to keep it.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::{
    ConditionalFormatType, ConditionalFormatValueObjectType, ConditionalFormattingOperator,
    IconSetType,
};

use crate::error::SmlError;
use crate::font::{Color, ColorElement, FontProperties};
use crate::styles::differential::{DifferentialFormat, DifferentialFormats};
use crate::styles::fonts::Font;
use crate::styles::stylesheet::StylesheetPart;
use crate::write::style_specs::{BorderSpec, PatternFillSpec};

use super::conditional_rules::{ConditionalFormattingFormula, ConditionalFormattingRule};
use super::conditional_scales::{ColorScale, ConditionalValueObject, DataBar, IconSet};

/// One threshold to author: `x:cfvo`'s `@type`, `@val` and `@gte`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalValueObjectSpec {
    /// `@type` — how `value` is to be read. `use="required"`, so there is no `None` for it.
    pub value_kind: ConditionalFormatValueObjectType,
    /// `@val` — the threshold, as text. `None` writes no attribute, which is what a `min` or `max`
    /// threshold with nothing to say looks like.
    pub value: Option<String>,
    /// `@gte` — icon sets only. `None` writes no attribute, which means the schema default `true`.
    pub is_greater_than_or_equal: Option<bool>,
}

impl ConditionalValueObjectSpec {
    /// A threshold of `value_kind` with no `@val` and no `@gte` — the shape `min` and `max` take.
    #[must_use]
    pub const fn of_kind(value_kind: ConditionalFormatValueObjectType) -> Self {
        Self {
            value_kind,
            value: None,
            is_greater_than_or_equal: None,
        }
    }

    /// A threshold of `value_kind` whose `@val` is `value`.
    #[must_use]
    pub fn with_value(
        value_kind: ConditionalFormatValueObjectType,
        value: impl Into<String>,
    ) -> Self {
        Self {
            value_kind,
            value: Some(value.into()),
            is_greater_than_or_equal: None,
        }
    }

    /// Builds the `x:cfvo` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> ConditionalValueObject {
        let mut object = ConditionalValueObject::new(interner, prefix);
        object.set_value_kind(interner, self.value_kind);
        if let Some(value) = &self.value {
            object.set_value(interner, Some(value.as_str()));
        }
        if let Some(gte) = self.is_greater_than_or_equal {
            object.set_is_greater_than_or_equal(interner, Some(gte));
        }
        object
    }
}

/// A colour scale to author: the thresholds, and one colour for each.
///
/// The schema wants two or more of each and pairs them by position (§18.3.1.11). Nothing here pads
/// or truncates either list — a caller that passes three thresholds and two colours gets exactly
/// that markup, and [`ColorScale::is_balanced`] then reports it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorScaleSpec {
    /// `cfvo` children, in order.
    pub thresholds: Vec<ConditionalValueObjectSpec>,
    /// `color` children, in the same order.
    pub colors: Vec<Color>,
}

impl ColorScaleSpec {
    /// The two-colour scale a caller almost always wants: the range's minimum in `low`, its maximum
    /// in `high`, both six-digit `RRGGBB`.
    #[must_use]
    pub fn two_color(low: &str, high: &str) -> Self {
        Self {
            thresholds: vec![
                ConditionalValueObjectSpec::with_value(
                    ConditionalFormatValueObjectType::Minimum,
                    "0",
                ),
                ConditionalValueObjectSpec::with_value(
                    ConditionalFormatValueObjectType::Maximum,
                    "0",
                ),
            ],
            colors: vec![Color::from_opaque_rgb(low), Color::from_opaque_rgb(high)],
        }
    }

    /// Builds the `x:colorScale` this describes.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> ColorScale {
        let mut scale = ColorScale::new(interner, prefix);
        for threshold in &self.thresholds {
            let built = threshold.build(interner, prefix);
            scale.push_threshold(built);
        }
        for color in &self.colors {
            scale.push_color(ColorElement::named(interner, prefix, "color", color));
        }
        scale
    }
}

/// A data bar to author: two thresholds, one colour, and the three optional attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct DataBarSpec {
    /// The bar's shorter end.
    pub shortest: ConditionalValueObjectSpec,
    /// The bar's longer end.
    pub longest: ConditionalValueObjectSpec,
    /// The one colour the bar is drawn in.
    pub color: Color,
    /// `@minLength`. `None` writes no attribute, which means the schema default `10`.
    pub minimum_length: Option<u32>,
    /// `@maxLength`. `None` writes no attribute, which means the schema default `90`.
    pub maximum_length: Option<u32>,
    /// `@showValue`. `None` writes no attribute, which means the schema default `true`.
    pub shows_cell_value: Option<bool>,
}

impl DataBarSpec {
    /// A bar spanning the range's own minimum to its own maximum, in one opaque colour — the shape
    /// §18.3.1.28's own example takes.
    #[must_use]
    pub fn spanning_the_range(hex: &str) -> Self {
        Self {
            shortest: ConditionalValueObjectSpec::with_value(
                ConditionalFormatValueObjectType::Minimum,
                "0",
            ),
            longest: ConditionalValueObjectSpec::with_value(
                ConditionalFormatValueObjectType::Maximum,
                "0",
            ),
            color: Color::from_opaque_rgb(hex),
            minimum_length: None,
            maximum_length: None,
            shows_cell_value: None,
        }
    }

    /// Builds the `x:dataBar` this describes.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> DataBar {
        let mut bar = DataBar::new(interner, prefix);
        let shortest = self.shortest.build(interner, prefix);
        bar.push_threshold(shortest);
        let longest = self.longest.build(interner, prefix);
        bar.push_threshold(longest);
        bar.set_color(Some(ColorElement::named(
            interner,
            prefix,
            "color",
            &self.color,
        )));
        if let Some(length) = self.minimum_length {
            bar.set_minimum_length(interner, Some(length));
        }
        if let Some(length) = self.maximum_length {
            bar.set_maximum_length(interner, Some(length));
        }
        if let Some(show) = self.shows_cell_value {
            bar.set_shows_cell_value(interner, Some(show));
        }
        bar
    }
}

/// An icon set to author: which icons, and the band boundaries between them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconSetSpec {
    /// `@iconSet`. `None` writes no attribute, which means the schema default `3TrafficLights1` —
    /// [`IconSetType::ThreeTrafficLights`].
    pub icons: Option<IconSetType>,
    /// `cfvo` children, in order.
    pub thresholds: Vec<ConditionalValueObjectSpec>,
    /// `@showValue`. `None` writes no attribute, which means the schema default `true`.
    pub shows_cell_value: Option<bool>,
    /// `@percent`. `None` writes no attribute, which means the schema default `true`.
    pub thresholds_are_percentiles: Option<bool>,
    /// `@reverse`. `None` writes no attribute, which means the schema default `false`.
    pub icons_are_reversed: Option<bool>,
}

impl IconSetSpec {
    /// Builds the `x:iconSet` this describes.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> IconSet {
        let mut icons = IconSet::new(interner, prefix);
        if let Some(kind) = self.icons {
            icons.set_icons(interner, Some(kind));
        }
        for threshold in &self.thresholds {
            let built = threshold.build(interner, prefix);
            icons.push_threshold(built);
        }
        if let Some(show) = self.shows_cell_value {
            icons.set_shows_cell_value(interner, Some(show));
        }
        if let Some(percent) = self.thresholds_are_percentiles {
            icons.set_thresholds_are_percentiles(interner, Some(percent));
        }
        if let Some(reverse) = self.icons_are_reversed {
            icons.set_icons_are_reversed(interner, Some(reverse));
        }
        icons
    }
}

/// Which kind of rule to author, and everything that kind needs to be complete.
///
/// The five kinds whose markup is fully determined by their arguments. See this module's own
/// documentation for why `top10`, `timePeriod`, `containsText` and the rest are not here.
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionalRuleSpecKind {
    /// `type="cellIs"` — an operator and its operands, each a formula as text.
    ///
    /// `between` and `notBetween` take two operands; every other operator takes one. Nothing checks
    /// that, because §18.3.1.10 does not say it and a producer is free to write what it likes.
    CellIs {
        /// `@operator`.
        operator: ConditionalFormattingOperator,
        /// The `formula` children, in order. Carried as text and never parsed.
        operands: Vec<String>,
    },
    /// `type="expression"` — one formula. §18.3.1.10 notes it is the only rule kind whose content
    /// *"support\[s\] formula syntax"*.
    Expression {
        /// The one `formula` child. Carried as text and never parsed.
        formula: String,
    },
    /// `type="colorScale"`.
    ColorScale(ColorScaleSpec),
    /// `type="dataBar"`.
    DataBar(DataBarSpec),
    /// `type="iconSet"`.
    IconSet(IconSetSpec),
}

impl ConditionalRuleSpecKind {
    /// The `@type` this kind writes.
    #[must_use]
    pub const fn rule_type(&self) -> ConditionalFormatType {
        match self {
            Self::CellIs { .. } => ConditionalFormatType::CellIs,
            Self::Expression { .. } => ConditionalFormatType::Expression,
            Self::ColorScale(_) => ConditionalFormatType::ColorScale,
            Self::DataBar(_) => ConditionalFormatType::DataBar,
            Self::IconSet(_) => ConditionalFormatType::IconSet,
        }
    }
}

/// One `x:cfRule` to author.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalRuleSpec {
    /// Which kind of rule, and its own contents.
    pub kind: ConditionalRuleSpecKind,
    /// `@priority`. **Stated by the caller, never derived** — it is what decides which rule wins,
    /// and a number this library chose would be a decision the caller did not make. §18.3.1.10:
    /// lower is higher priority, and `1` is the highest.
    pub priority: i32,
    /// `@stopIfTrue`. `None` writes no attribute, which means the schema default `false`.
    pub stops_lower_priority_rules: Option<bool>,
    /// `@dxfId` — the index into `dxfs` of the formatting the rule imposes.
    ///
    /// Allocate one with [`StylesheetPart::append_differential_format`], which appends and never
    /// renumbers. `None` writes no attribute, which is right for a `colorScale`, a `dataBar` and an
    /// `iconSet`: those draw themselves and impose no `dxf`.
    pub differential_format_index: Option<u32>,
}

impl ConditionalRuleSpec {
    /// A `cellIs` rule at `priority`, comparing with `operator` against `operands`.
    #[must_use]
    pub fn cell_is(
        operator: ConditionalFormattingOperator,
        operands: impl IntoIterator<Item = String>,
        priority: i32,
    ) -> Self {
        Self {
            kind: ConditionalRuleSpecKind::CellIs {
                operator,
                operands: operands.into_iter().collect(),
            },
            priority,
            stops_lower_priority_rules: None,
            differential_format_index: None,
        }
    }

    /// Builds the `x:cfRule` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(
        &self,
        interner: &mut Interner,
        prefix: Option<&str>,
    ) -> ConditionalFormattingRule {
        let mut rule = ConditionalFormattingRule::new(interner, prefix);
        rule.set_kind(interner, Some(self.kind.rule_type()));
        rule.set_priority(interner, self.priority);
        if let Some(stop) = self.stops_lower_priority_rules {
            rule.set_stops_lower_priority_rules(interner, Some(stop));
        }
        if let Some(index) = self.differential_format_index {
            rule.set_differential_format_index(interner, Some(index));
        }
        match &self.kind {
            ConditionalRuleSpecKind::CellIs { operator, operands } => {
                rule.set_operator(interner, Some(*operator));
                for operand in operands {
                    let formula = ConditionalFormattingFormula::new(interner, prefix, operand);
                    rule.push_formula(formula);
                }
            }
            ConditionalRuleSpecKind::Expression { formula } => {
                let formula = ConditionalFormattingFormula::new(interner, prefix, formula);
                rule.push_formula(formula);
            }
            ConditionalRuleSpecKind::ColorScale(spec) => {
                let scale = spec.build(interner, prefix);
                rule.set_color_scale(Some(scale));
            }
            ConditionalRuleSpecKind::DataBar(spec) => {
                let bar = spec.build(interner, prefix);
                rule.set_data_bar(Some(bar));
            }
            ConditionalRuleSpecKind::IconSet(spec) => {
                let icons = spec.build(interner, prefix);
                rule.set_icon_set(Some(icons));
            }
        }
        rule
    }
}

/// A `dxf` to author: the three members a conditional format states in practice.
///
/// # Every member is absent by default, and absent means *inherited*
///
/// A `dxf` is a **delta**, not a format: all seven of its children are `minOccurs="0"`, and an
/// absent one means *"leave whatever the cell already has"* rather than *"use the default"*. So
/// `Default::default()` here describes `<dxf/>` — a format that changes nothing — which is a
/// meaningful value and not a placeholder. See [`crate::DifferentialFormat`].
///
/// # Three of the six members, and why
///
/// `font`, `fill` and `border` are what a conditional format states; a highlight rule is a font
/// colour and a fill, and almost nothing else. The remaining three — `alignment`, `protection` and
/// `numFmt` — are element children shared with `CT_Xf`, and are reached by building a
/// [`DifferentialFormat`] directly and calling its own setters. That is exactly the boundary
/// MJXOFF-105's [`CellFormatSpec`](crate::CellFormatSpec) already draws for the same two children,
/// so this adds no second rule.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DifferentialFormatSpec {
    /// `x:font` — the font properties the rule overrides.
    pub font: Option<FontProperties>,
    /// `x:fill` — the fill the rule overrides. The member set on its own most often.
    pub fill: Option<PatternFillSpec>,
    /// `x:border` — the border the rule overrides.
    pub border: Option<BorderSpec>,
}

impl DifferentialFormatSpec {
    /// The highlight a `cellIs` rule usually carries: a font colour and a solid fill, both
    /// six-digit `RRGGBB`.
    #[must_use]
    pub fn highlight(text_hex: &str, fill_hex: &str) -> Self {
        Self {
            font: Some(FontProperties {
                color: Some(Color::from_opaque_rgb(text_hex)),
                ..FontProperties::default()
            }),
            fill: Some(PatternFillSpec::solid(fill_hex)),
            border: None,
        }
    }

    /// Builds the `x:dxf` this describes, interning its names into `interner`.
    ///
    /// # Errors
    /// [`SmlError::Xml`] if the font markup this re-parses is not well-formed, which is unreachable
    /// for markup [`FontProperties`] serialized — see [`Font::from_properties`].
    pub fn build(
        &self,
        interner: &mut Interner,
        prefix: Option<&str>,
    ) -> Result<DifferentialFormat, SmlError> {
        let mut format = DifferentialFormat::new(interner, prefix);
        if let Some(properties) = &self.font {
            format.set_font(Some(Font::from_properties(interner, prefix, properties)?));
        }
        if let Some(fill) = &self.fill {
            format.set_fill(Some(fill.build(interner, prefix)));
        }
        if let Some(border) = &self.border {
            format.set_border(Some(border.build(interner, prefix)));
        }
        Ok(format)
    }
}

impl StylesheetPart {
    /// Appends `format` to `x:dxfs` and returns the `@dxfId` it can now be named by, creating the
    /// table at its rank in `CT_Stylesheet`'s sequence if the part has none.
    ///
    /// **Appending is the only mutation this table has**, and the reason is the whole of
    /// [`crate::DifferentialFormats`]'s contract: a `dxf` is addressed by position, so inserting,
    /// removing or reordering one would silently repoint every `@dxfId` above it — in this
    /// worksheet's rules, in every other worksheet's, and in every table style. So the index this
    /// returns is `len() - 1` after the append, and every index handed out before it still names
    /// what it named.
    ///
    /// `@count` follows the collection when the file declared one, and is never added to a table
    /// that wrote none.
    pub fn append_differential_format(
        &mut self,
        interner: &mut Interner,
        prefix: Option<&str>,
        format: DifferentialFormat,
    ) -> u32 {
        if self.differential_formats().is_none() {
            let table = DifferentialFormats::new(interner, prefix);
            self.set_differential_formats(interner, Some(table));
        }
        let table = self
            .differential_formats_mut()
            .expect("the dxfs table was just ensured");
        table.push(interner, format);
        u32::try_from(table.len().saturating_sub(1)).unwrap_or(u32::MAX)
    }
}
