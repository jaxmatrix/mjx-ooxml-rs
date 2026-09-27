//! [`ConditionalIndex`] — every rule on a sheet, decoded once and ordered the way Excel applies
//! them.
//!
//! # Why the blocks are flattened before anything is evaluated
//!
//! `conditionalFormatting` is one of only two children of `CT_Worksheet` declared
//! `maxOccurs="unbounded"`, and `cfRule@priority` is scoped to the **workbook** rather than to the
//! block. `mjx_sml::conditional_chain` says the consequence in as many words: three blocks holding
//! priorities `1, 4`, `2` and `3` govern a cell they all cover in the order `1, 2, 3, 4`, so
//! consecutive rules come from different blocks and a per-block sort is wrong in a way no
//! single-block fixture can see.
//!
//! `mjx-sml` answers that question per cell, through
//! [`WorksheetPart::conditional_rules_for`](mjx_sml::WorksheetPart::conditional_rules_for). This
//! crate cannot ask it per cell: that call re-reads every block's `@sqref` — a string, parsed
//! through the interner, into a range list — for **every visible cell**, which on a band of two
//! hundred cells over ten blocks is two thousand range parses a frame. So the decode happens once
//! per sheet, into a flat list already in priority order, and the per-cell question becomes a
//! containment test against ranges that are already `GridBounds`.
//!
//! The ordering is `mjx-sml`'s and is not re-derived: **lower `@priority` wins**, ties broken by
//! document order (block, then rule within the block), by a stable sort. Priorities are never
//! renumbered — §18.3.1.10 does not say they are dense, unique or one-based, and files Excel writes
//! are none of the three.
//!
//! # What a rule needs that its markup does not carry
//!
//! Nothing. Every attribute of `CT_CfRule` is modelled in `mjx-sml`, including the eleven that
//! §18.3.1.10 describes as *"ignored if type is not equal to…"*. This module reads them and decides
//! nothing about validity: a `@rank` on a `cellIs` rule is decoded and never consulted, exactly as
//! `mjx-sml` reports it and never enforces it.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::{
    ConditionalFormatType, ConditionalFormatValueObjectType, ConditionalFormattingOperator,
    IconSetType, TimePeriod,
};
use mjx_sml::{Color, GridBounds, WorksheetPart};

/// One `cfvo`: what the number means and what the number is, decoded no further.
///
/// `@val` is an `ST_Xstring` and its meaning is `@type`'s, so a `min` carries a `@val` Excel writes
/// and ignores and a `formula` carries an expression nothing here evaluates. Both are kept as the
/// file wrote them.
#[derive(Clone, PartialEq, Debug)]
pub struct Threshold {
    /// `@type` — number, percent, max, min, formula or percentile.
    pub kind: ConditionalFormatValueObjectType,
    /// `@val`, verbatim.
    pub value: Option<String>,
    /// `@gte` — whether the band this bounds includes its own boundary. Defaults to `true`.
    pub inclusive: bool,
}

impl Threshold {
    /// `@val` read as a number, or `None` when it is absent or is not one.
    #[must_use]
    pub fn number(&self) -> Option<f64> {
        self.value
            .as_deref()
            .map(str::trim)
            .and_then(|text| text.parse::<f64>().ok())
            .filter(|value| value.is_finite())
    }
}

/// The graded half of a rule: the three kinds that interpolate rather than decide.
#[derive(Clone, PartialEq, Debug)]
pub enum Graded {
    /// `colorScale` — two or more thresholds, paired by position with two or more colours.
    ColorScale {
        /// The thresholds and their colours, already paired and truncated to the shorter list.
        stops: Vec<(Threshold, Color)>,
    },
    /// `dataBar` — exactly two thresholds and one colour.
    DataBar {
        /// The threshold the shortest bar is drawn at.
        shortest: Threshold,
        /// The threshold the longest bar is drawn at.
        longest: Threshold,
        /// The bar's colour, unresolved.
        colour: Option<Color>,
        /// `@minLength`, *as a percentage of the cell width*. Schema default 10.
        minimum_length: u32,
        /// `@maxLength`, the same. Schema default 90.
        maximum_length: u32,
        /// `@showValue` — whether the cell's own text is drawn over the bar. Schema default true.
        shows_value: bool,
    },
    /// `iconSet` — one icon per band, and the thresholds between them.
    IconSet {
        /// `@iconSet`, whose schema default is the wire token `3TrafficLights1`.
        icons: IconSetType,
        /// The band boundaries, in the order the file wrote them.
        thresholds: Vec<Threshold>,
        /// `@reverse`.
        reversed: bool,
        /// `@showValue`.
        shows_value: bool,
        /// `@percent`, reported and not acted on — see [`crate::condfmt::graded`].
        thresholds_are_percentages: bool,
    },
}

/// One rule, decoded, with its block's ranges attached.
#[derive(Clone, PartialEq, Debug)]
pub struct CompiledRule {
    /// Where this rule sits in [`ConditionalIndex::rules`], which is also its statistics slot.
    pub position: usize,
    /// Which block it came from, and which rule of that block — the stable tiebreak, and the
    /// identity a [`crate::model::CellReport`] names.
    pub origin: (u32, u32),
    /// `@priority`, as the file wrote it. Lower wins.
    pub priority: i32,
    /// `@type`, or `None` for a rule that states none — which the schema permits and which this
    /// evaluates as nothing at all rather than guessing.
    pub kind: Option<ConditionalFormatType>,
    /// `@dxfId` — the differential format this rule imposes when it fires.
    pub differential_format: Option<u32>,
    /// `@stopIfTrue`.
    pub stops_lower_priority_rules: bool,
    /// The block's `@sqref`, normalised.
    pub ranges: Vec<GridBounds>,
    /// `@operator`, for `cellIs`.
    pub operator: Option<ConditionalFormattingOperator>,
    /// `@text`, for the four text kinds.
    pub text: Option<String>,
    /// `@timePeriod`.
    pub time_period: Option<TimePeriod>,
    /// `@rank`, for `top10`.
    pub rank: Option<u32>,
    /// `@bottom`.
    pub ranks_from_bottom: bool,
    /// `@percent`.
    pub ranks_by_percent: bool,
    /// `@aboveAverage`, whose schema default is `true`.
    pub is_above_average: bool,
    /// `@equalAverage`.
    pub includes_the_average: bool,
    /// `@stdDev`.
    pub standard_deviations: Option<i32>,
    /// Every `<formula>`, in document order, as text. Never parsed — MJXOFF-115's contract.
    pub formulas: Vec<String>,
    /// The graded child, when the rule has one.
    pub graded: Option<Graded>,
}

impl CompiledRule {
    /// Whether this rule's block covers `(row, column)`.
    #[must_use]
    pub fn covers(&self, row: u32, column: u16) -> bool {
        super::stats::covers(&self.ranges, row, column)
    }
}

/// Every rule on one worksheet, decoded once, in the order Excel applies them.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ConditionalIndex {
    rules: Vec<CompiledRule>,
    /// The union of every block's ranges, so a cell outside all of them costs one test rather than
    /// one per rule. A sheet with no conditional formatting answers `None` and is free.
    ///
    /// Four numbers rather than a [`GridBounds`]: that type is built by normalising a
    /// [`CellRange`](mjx_sml::CellRange) and has no constructor of its own, which is the right
    /// shape for an address and the wrong one for an accumulator.
    envelope: Option<Envelope>,
}

impl ConditionalIndex {
    /// Decodes every `conditionalFormatting` block of `sheet`.
    #[must_use]
    pub fn read(sheet: &WorksheetPart) -> Self {
        let interner = sheet.interner();
        let mut rules = Vec::new();
        for (block_index, block) in sheet.conditional_formatting_blocks().enumerate() {
            let ranges: Vec<GridBounds> = block
                .ranges(interner)
                .ok()
                .flatten()
                .map(|list| {
                    list.ranges()
                        .iter()
                        .map(|range| range.normalized_bounds())
                        .collect()
                })
                .unwrap_or_default();
            if ranges.is_empty() {
                // A block whose `@sqref` is absent or will not parse governs no cell. Reported by
                // its absence from the index rather than by an error: a malformed range is a fact
                // about the file, and one bad block must not stop the sheet rendering.
                continue;
            }
            for (rule_index, rule) in block.rules().enumerate() {
                let block_number = u32::try_from(block_index).unwrap_or(u32::MAX);
                let rule_number = u32::try_from(rule_index).unwrap_or(u32::MAX);
                rules.push(compile(
                    rule,
                    interner,
                    (block_number, rule_number),
                    &ranges,
                ));
            }
        }
        // Stable, so rules of equal priority keep document order — block by block, and rule by rule
        // within a block. That is `mjx_sml::ConditionalRuleChain`'s rule, restated here rather than
        // re-decided.
        rules.sort_by_key(|rule| rule.priority);
        let mut envelope: Option<Envelope> = None;
        for (position, rule) in rules.iter_mut().enumerate() {
            rule.position = position;
            for bounds in &rule.ranges {
                envelope = Some(match envelope {
                    None => Envelope::of(*bounds),
                    Some(existing) => existing.union(*bounds),
                });
            }
        }
        Self { rules, envelope }
    }

    /// Every rule, in the order they apply.
    #[must_use]
    pub fn rules(&self) -> &[CompiledRule] {
        &self.rules
    }

    /// How many rules the sheet holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the sheet holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Whether any rule could possibly reach `(row, column)`.
    ///
    /// One rectangle test against the union of every block, so the overwhelmingly common case — a
    /// sheet with no conditional formatting, or a screen well away from the block that has it —
    /// costs nothing per cell.
    #[must_use]
    pub fn may_cover(&self, row: u32, column: u16) -> bool {
        self.envelope
            .is_some_and(|envelope| envelope.contains(row, column))
    }
}

/// The union of every block's ranges: one rectangle test before any per-rule work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Envelope {
    first_row: u32,
    last_row: u32,
    first_column: u16,
    last_column: u16,
}

impl Envelope {
    /// The envelope of one range.
    fn of(bounds: GridBounds) -> Self {
        Self {
            first_row: bounds.first_row(),
            last_row: bounds.last_row(),
            first_column: bounds.first_column(),
            last_column: bounds.last_column(),
        }
    }

    /// The smallest envelope containing both.
    fn union(self, bounds: GridBounds) -> Self {
        Self {
            first_row: self.first_row.min(bounds.first_row()),
            last_row: self.last_row.max(bounds.last_row()),
            first_column: self.first_column.min(bounds.first_column()),
            last_column: self.last_column.max(bounds.last_column()),
        }
    }

    /// Whether it reaches this position.
    fn contains(self, row: u32, column: u16) -> bool {
        row >= self.first_row
            && row <= self.last_row
            && column >= self.first_column
            && column <= self.last_column
    }
}

/// Decodes one `cfRule`.
fn compile(
    rule: &mjx_sml::ConditionalFormattingRule,
    interner: &Interner,
    origin: (u32, u32),
    ranges: &[GridBounds],
) -> CompiledRule {
    let graded = rule
        .color_scale()
        .map(|scale| colour_scale(scale, interner))
        .or_else(|| rule.data_bar().map(|bar| data_bar(bar, interner)))
        .or_else(|| rule.icon_set().map(|set| icon_set(set, interner)));
    CompiledRule {
        position: 0,
        origin,
        // `@priority` is `use="required"`; a rule that omits it or writes a number that will not
        // parse is placed last rather than dropped, because the rule still states a format.
        priority: rule.priority(interner).ok().unwrap_or(i32::MAX),
        kind: rule.kind(interner).ok().flatten(),
        differential_format: rule.differential_format_index(interner).ok().flatten(),
        stops_lower_priority_rules: rule.stops_lower_priority_rules(interner).unwrap_or(false),
        ranges: ranges.to_vec(),
        operator: rule.operator(interner).ok().flatten(),
        text: rule
            .text(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned),
        time_period: rule.time_period(interner).ok().flatten(),
        rank: rule.top_or_bottom_count(interner).ok().flatten(),
        ranks_from_bottom: rule.ranks_from_bottom(interner).unwrap_or(false),
        ranks_by_percent: rule.ranks_by_percent(interner).unwrap_or(false),
        is_above_average: rule.is_above_average(interner).unwrap_or(true),
        includes_the_average: rule.includes_the_average(interner).unwrap_or(false),
        standard_deviations: rule.standard_deviations(interner).ok().flatten(),
        formulas: rule
            .formulas()
            .map(|formula| formula.text().to_owned())
            .collect(),
        graded,
    }
}

/// Decodes one `cfvo`.
fn threshold(object: &mjx_sml::ConditionalValueObject, interner: &Interner) -> Threshold {
    Threshold {
        kind: object
            .value_kind(interner)
            .unwrap_or(ConditionalFormatValueObjectType::Number),
        value: object
            .value(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned),
        // `@gte`'s schema default is `true`: a band includes its own lower boundary unless the file
        // says otherwise.
        inclusive: object.is_greater_than_or_equal(interner).unwrap_or(true),
    }
}

/// Decodes an `x:colorScale`, pairing thresholds with colours by position.
fn colour_scale(scale: &mjx_sml::ColorScale, interner: &Interner) -> Graded {
    // `ColorScale::pairs` stops at the shorter of the two lists rather than padding, which is
    // `mjx-sml`'s answer to a file that writes three colours beside two thresholds. Nothing here
    // repairs it either; a scale with one usable stop degrades to its first colour.
    let stops = scale
        .pairs()
        .filter_map(|(object, colour)| {
            let colour = colour.color(interner);
            (!colour.is_empty()).then(|| (threshold(object, interner), colour))
        })
        .collect();
    Graded::ColorScale { stops }
}

/// Decodes an `x:dataBar`.
fn data_bar(bar: &mjx_sml::DataBar, interner: &Interner) -> Graded {
    let mut thresholds = bar.thresholds().map(|object| threshold(object, interner));
    let shortest = thresholds.next().unwrap_or(Threshold {
        kind: ConditionalFormatValueObjectType::Minimum,
        value: None,
        inclusive: true,
    });
    let longest = thresholds.next().unwrap_or(Threshold {
        kind: ConditionalFormatValueObjectType::Maximum,
        value: None,
        inclusive: true,
    });
    Graded::DataBar {
        shortest,
        longest,
        colour: bar
            .color()
            .map(|colour| colour.color(interner))
            .filter(|colour| !colour.is_empty()),
        minimum_length: bar.minimum_length(interner).unwrap_or(10),
        maximum_length: bar.maximum_length(interner).unwrap_or(90),
        shows_value: bar.shows_cell_value(interner).unwrap_or(true),
    }
}

/// Decodes an `x:iconSet`.
fn icon_set(set: &mjx_sml::IconSet, interner: &Interner) -> Graded {
    Graded::IconSet {
        icons: set
            .icons(interner)
            .unwrap_or(IconSetType::ThreeTrafficLights),
        thresholds: set
            .thresholds()
            .map(|object| threshold(object, interner))
            .collect(),
        reversed: set.icons_are_reversed(interner).unwrap_or(false),
        shows_value: set.shows_cell_value(interner).unwrap_or(true),
        thresholds_are_percentages: set.thresholds_are_percentiles(interner).unwrap_or(true),
    }
}
