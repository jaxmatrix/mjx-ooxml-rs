//! Conditional formatting, **evaluated** — the eighteen rule kinds of `ST_CfType`, the `dxf` layer
//! they impose, and the two of them that cannot be answered without a calculation engine.
//!
//! # What `mjx-sml` does and where it stops
//!
//! `mjx_sml::features::conditional_chain` says it in as many words: *"this crate answers which rules
//! apply and what each would impose. It never answers what the cell looks like."* It merges the
//! blocks, orders them by `@priority`, reports `@stopIfTrue` as a *position* rather than applying
//! it, and hands back a [`ConditionalCellFormat`](mjx_sml::ConditionalCellFormat) whose two layers
//! are deliberately never combined — because combining them would be claiming a condition held.
//!
//! **This module is the consumer that can evaluate**, and combining them is exactly its job.
//!
//! # The trap this module was written around
//!
//! A conditional format that never fires is indistinguishable from one that is not implemented. The
//! sheet renders; every fragment is in the same place; the counts all match. So the surface here is
//! shaped so that a gate can see the difference:
//!
//! * [`ConditionalEffect::fired`] names every rule that fired, by its `(block, rule)` position —
//!   so a test asserts *which* rule, not that *something* happened.
//! * [`ConditionalEffect::unevaluated`] names every rule that could not be answered, with the
//!   reason and the formula. A rule that quietly did not fire and one that could not be evaluated
//!   are different states and are reported as different states.
//! * The three graded kinds answer numbers — a bar's fraction, a scale's position between two
//!   stops, an icon's index — rather than *"a bar appears"*.
//!
//! # The order rules apply in, which is not the order they are written in
//!
//! Lower `@priority` wins. Rules are considered from the highest priority down, and **the first
//! rule to state a member is the one that imposes it**: a priority-1 rule stating only a font colour
//! and a priority-2 rule stating a font colour and a fill produce a cell with the first rule's
//! colour and the second rule's fill. That is what makes a `dxf` a *delta* rather than a format, and
//! it is why [`ConditionalEffect`] fills its slots in and never overwrites one.
//!
//! `@stopIfTrue` ends the walk **after** the rule that carries it has fired, so a lower-priority
//! rule's members are not reached. On a rule that did not fire it does nothing;
//! [`ConditionalEffect::stopped_after`] says whether it ran.
//!
//! # ⚠ Nothing here is parity with Excel
//!
//! Every behaviour chosen rather than read is marked `GUESS:` at its site, in
//! [`predicate`], [`graded`], [`stats`] and [`value`]. Confirmation is a human
//! sitting against real Microsoft Excel on Windows; LibreOffice is a change detector and not a
//! reference, and the user has said its export of shades and gradients is not to be trusted at all
//! — which lands squarely on data bars and colour scales, both of which *are* gradients.

pub mod graded;
pub mod predicate;
pub mod rules;
pub mod stats;
pub mod value;

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::{ConditionalFormatType, PatternType};
use mjx_sml::{DifferentialFormat, FontProperties};

use crate::model::{CellBorders, CellFill};
use crate::sheet::SheetGrid;

pub use graded::{DataBarGeometry, GradedResult, IconChoice, ScaleBlend};
pub use predicate::{Decision, UnevaluatedReason};
pub use rules::{CompiledRule, ConditionalIndex, Graded, Threshold};
pub use stats::{RangeStatistics, StatisticsCache, SCAN_CEILING};
pub use value::RuleValue;

/// One rule that fired, named by where it came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AppliedRule {
    /// `(block, rule)` — the position in the worksheet's markup, which is stable across a reorder
    /// of priorities and is what a test names.
    pub origin: (u32, u32),
    /// `@priority`, as the file wrote it.
    pub priority: i32,
    /// `@type`.
    pub kind: Option<ConditionalFormatType>,
    /// `@dxfId`, when it states one.
    pub differential_format: Option<u32>,
}

/// One rule that could not be answered, and why.
///
/// **Recorded, never faked and never dropped.** The ticket's own constraint: an `expression` rule's
/// condition is a formula, there is no calculation engine in this workspace, and a rule that
/// silently did not fire looks exactly like a rule that was never implemented.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnevaluatedRule {
    /// `(block, rule)`.
    pub origin: (u32, u32),
    /// `@priority`.
    pub priority: i32,
    /// `@type`.
    pub kind: Option<ConditionalFormatType>,
    /// Why it could not be answered.
    pub reason: UnevaluatedReason,
    /// The text of the condition, verbatim, so a report can quote it.
    pub condition: String,
}

/// What conditional formatting made of one cell.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ConditionalEffect {
    /// The fill the winning `dxf` imposes.
    pub fill: Option<CellFill>,
    /// The font members it imposes — a delta, so every slot it leaves absent stays the cell's own.
    pub font: Option<FontProperties>,
    /// The borders it imposes.
    pub borders: Option<CellBorders>,
    /// The number-format id it imposes, if any. **Reported, not applied**: the `dxf`'s `numFmt`
    /// states a `@numFmtId` and optionally a `@formatCode`, and applying it would mean formatting
    /// the cell twice — once to decide which conditional section a `[Red]` came from and again
    /// after the rule fired. GUESS: that leaving the base format in force is closer to Excel than
    /// a second formatting pass would be; it is the one member of a `dxf` this build reports and
    /// does not honour, and it is marked here rather than in a changelog.
    pub number_format: Option<(u32, Option<String>)>,
    /// The colour a scale interpolated, unresolved.
    pub scale: Option<ScaleBlend>,
    /// The bar a data-bar rule sized.
    pub bar: Option<DataBarGeometry>,
    /// The icon an icon-set rule chose.
    pub icon: Option<IconChoice>,
    /// Every rule that fired, in the order they were applied.
    pub fired: Vec<AppliedRule>,
    /// Every rule that could not be answered.
    pub unevaluated: Vec<UnevaluatedRule>,
    /// The rule whose `@stopIfTrue` ended the walk, when one did.
    pub stopped_after: Option<AppliedRule>,
}

impl ConditionalEffect {
    /// Whether any rule reached this cell at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fired.is_empty() && self.unevaluated.is_empty()
    }

    /// Whether anything here changes what the cell looks like.
    ///
    /// A rule can fire and change nothing — a `cellIs` with no `@dxfId` is legal markup — and a
    /// rule that could not be evaluated changes nothing by definition. Both are still reported.
    #[must_use]
    pub fn changes_appearance(&self) -> bool {
        self.fill.is_some()
            || self.font.is_some()
            || self.borders.is_some()
            || self.scale.is_some()
            || self.bar.is_some()
            || self.icon.is_some()
    }

    /// The key two cells must agree on to share a decoration.
    ///
    /// The `dxf` layer is a pure function of *which rules fired*, so two cells that fired the same
    /// rules carry the same fill, font and borders. A colour scale is the one exception — its
    /// answer depends on the value — so its blend joins the key. A data bar does not: the bar is a
    /// fragment of its own with its own decoration, interned by colour.
    #[must_use]
    pub fn signature(&self) -> ConditionalSignature {
        ConditionalSignature {
            fired: self.fired.iter().map(|rule| rule.origin).collect(),
            scale: self.scale.as_ref().map(|blend| {
                (
                    blend.low.clone(),
                    blend.high.clone(),
                    blend.fraction.to_bits(),
                )
            }),
        }
    }
}

/// What two cells must agree on before they may share one decoration handle.
///
/// `Default` is *no conditional formatting at all*, which is what every cell on an ordinary sheet
/// carries — so the sharing R16 measured (a screen of unformatted cells issuing one handle) is
/// unchanged where there is no conditional formatting, which is almost everywhere.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ConditionalSignature {
    fired: Vec<(u32, u32)>,
    /// Raw bits rather than an `f64`, so the key stays comparable without a float `Eq`.
    scale: Option<(mjx_sml::Color, mjx_sml::Color, u64)>,
}

impl ConditionalSignature {
    /// Whether this signature is the one an unconditioned cell carries.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.fired.is_empty() && self.scale.is_none()
    }
}

/// The evaluator: one sheet's rules, decoded, plus the statistics they need.
///
/// Held on the box model beside [`FormatCache`](crate::numfmt::FormatCache) and the grid geometry,
/// and rebuilt on the same trigger they are — a different tab. See [`ConditionalEngine::prepare`].
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ConditionalEngine {
    sheet: Option<usize>,
    index: ConditionalIndex,
    statistics: StatisticsCache,
}

impl ConditionalEngine {
    /// An engine that has read nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads `grid`'s rules, if they have not been read already.
    pub fn prepare(&mut self, grid: &SheetGrid) {
        if self.sheet == Some(grid.index()) {
            return;
        }
        self.sheet = Some(grid.index());
        self.index = ConditionalIndex::read(grid.worksheet());
        self.statistics.clear();
    }

    /// The rules it read.
    #[must_use]
    pub fn index(&self) -> &ConditionalIndex {
        &self.index
    }

    /// How many rules have had their range statistics computed.
    ///
    /// The measurement a gate makes to say the scan is done **once per rule** rather than once per
    /// cell: a band of two hundred cells over one colour scale computes one.
    #[must_use]
    pub fn statistics_computed(&self) -> usize {
        self.statistics.computed()
    }

    /// Drops everything, so the next `prepare` reads again.
    ///
    /// Called when the content changes. Unlike the number-format cache — which is keyed on a code
    /// and a value and stays correct across an edit — every statistic here is a fact about cell
    /// *values*, so an edit anywhere in a rule's range invalidates it.
    pub fn clear(&mut self) {
        self.sheet = None;
        self.index = ConditionalIndex::default();
        self.statistics.clear();
    }

    /// Whether any rule could reach `(row, column)` — one rectangle test, before any work.
    #[must_use]
    pub fn may_cover(&self, row: u32, column: u16) -> bool {
        self.index.may_cover(row, column)
    }

    /// Evaluates every rule that covers `(row, column)` against `value`.
    ///
    /// `differential` resolves a `@dxfId` to its format; it is the caller's because the styles part
    /// belongs to the resolver the box model already holds.
    pub fn evaluate<'a>(
        &mut self,
        grid: &SheetGrid,
        row: u32,
        column: u16,
        value: &RuleValue,
        interner: &Interner,
        differential: impl Fn(u32) -> Option<&'a DifferentialFormat>,
    ) -> ConditionalEffect {
        let mut effect = ConditionalEffect::default();
        // The two fields are borrowed apart rather than through `self`, because the rules are read
        // while the statistics cache is filled. Cloning a rule per cell instead would copy its
        // range list and its formulas onto the frame path.
        let Self {
            index, statistics, ..
        } = self;
        if !index.may_cover(row, column) {
            return effect;
        }
        for rule in index.rules() {
            if !rule.covers(row, column) {
                continue;
            }
            let statistics = statistics.statistics(grid, rule.position, &rule.ranges);
            let applied = AppliedRule {
                origin: rule.origin,
                priority: rule.priority,
                kind: rule.kind,
                differential_format: rule.differential_format,
            };
            if let Some(graded_rule) = rule.graded.as_ref() {
                if !graded::is_resolvable(graded_rule, statistics) {
                    effect.unevaluated.push(UnevaluatedRule {
                        origin: rule.origin,
                        priority: rule.priority,
                        kind: rule.kind,
                        reason: UnevaluatedReason::OperandIsNotALiteral,
                        condition: condition_of(rule),
                    });
                    continue;
                }
                let Some(number) = value.as_number() else {
                    // A graded rule says nothing about a text cell: there is no position between
                    // two numeric thresholds for a label. It has still *applied*, so it is not
                    // reported as having fired and it does not stop anything.
                    continue;
                };
                match graded::apply(graded_rule, number, statistics) {
                    Some(GradedResult::Scale(blend)) => {
                        if effect.scale.is_none() {
                            effect.scale = Some(blend);
                        }
                    }
                    Some(GradedResult::Bar(bar)) => {
                        if effect.bar.is_none() {
                            effect.bar = Some(bar);
                        }
                    }
                    Some(GradedResult::Icon(icon)) => {
                        if effect.icon.is_none() {
                            effect.icon = Some(icon);
                        }
                    }
                    None => continue,
                }
                effect.fired.push(applied);
                if rule.stops_lower_priority_rules {
                    effect.stopped_after = Some(applied);
                    break;
                }
                continue;
            }
            let context = predicate::Context {
                value,
                statistics,
                today: grid.today(),
                dates: grid.date_system(),
            };
            match predicate::decide(rule, &context) {
                Decision::DidNotFire => {}
                Decision::Unevaluated(reason) => effect.unevaluated.push(UnevaluatedRule {
                    origin: rule.origin,
                    priority: rule.priority,
                    kind: rule.kind,
                    reason,
                    condition: condition_of(rule),
                }),
                Decision::Fired => {
                    if let Some(format) = rule.differential_format.and_then(&differential) {
                        absorb(&mut effect, format, interner);
                    }
                    effect.fired.push(applied);
                    if rule.stops_lower_priority_rules {
                        effect.stopped_after = Some(applied);
                        break;
                    }
                }
            }
        }
        effect
    }
}

/// The text a report quotes for a rule that could not be answered.
fn condition_of(rule: &CompiledRule) -> String {
    if let Some(first) = rule.formulas.first() {
        return first.clone();
    }
    rule.text.clone().unwrap_or_default()
}

/// Folds one `dxf` into the effect, **filling empty slots only**.
///
/// A higher-priority rule has already had its say, and §18.8.15 makes a `dxf` a delta: an absent
/// member means *leave what is there*. So this never overwrites, which is what makes two rules on
/// one cell compose in Excel's order rather than in the file's.
fn absorb(effect: &mut ConditionalEffect, format: &DifferentialFormat, interner: &Interner) {
    if effect.fill.is_none() {
        effect.fill = differential_fill(format, interner);
    }
    if let Some(font) = format.font() {
        let properties = font.properties(interner);
        effect.font = Some(match effect.font.take() {
            None => properties,
            Some(existing) => merge_fonts(existing, properties),
        });
    }
    if effect.borders.is_none() {
        effect.borders = format
            .border()
            .map(|border| crate::model::borders_from(border, interner))
            .filter(|borders| !borders.is_empty());
    }
    if effect.number_format.is_none() {
        effect.number_format = format.number_format().and_then(|number| {
            Some((
                number.number_format_id(interner).ok().flatten()?,
                number
                    .format_code(interner)
                    .ok()
                    .flatten()
                    .map(std::borrow::Cow::into_owned),
            ))
        });
    }
}

/// Keeps every slot `winner` states and takes the rest from `loser`.
pub(crate) fn merge_fonts(winner: FontProperties, loser: FontProperties) -> FontProperties {
    FontProperties {
        font_name: winner.font_name.or(loser.font_name),
        character_set: winner.character_set.or(loser.character_set),
        family: winner.family.or(loser.family),
        bold: winner.bold.or(loser.bold),
        italic: winner.italic.or(loser.italic),
        strikethrough: winner.strikethrough.or(loser.strikethrough),
        outline: winner.outline.or(loser.outline),
        shadow: winner.shadow.or(loser.shadow),
        condensed: winner.condensed.or(loser.condensed),
        extended: winner.extended.or(loser.extended),
        color: winner.color.or(loser.color),
        size_in_points: winner.size_in_points.or(loser.size_in_points),
        underline: winner.underline.or(loser.underline),
        vertical_position: winner.vertical_position.or(loser.vertical_position),
        scheme: winner.scheme.or(loser.scheme),
        extra: winner.extra,
    }
}

/// ⚠ A `dxf`'s fill is read differently from a cell's, and this is the difference that decides
/// whether a highlight rule changes a pixel at all.
///
/// Excel writes a conditional highlight as
/// `<dxf><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf>` — **no
/// `@patternType`, and the colour in `bgColor` rather than in `fgColor`**. Read the way a cell's
/// fill is read, that is a pattern of `none` whose foreground is absent, which paints nothing: the
/// rule would fire, the gate would be green, and the sheet would look identical. That is exactly
/// the vacuous outcome this whole child exists to make impossible.
///
/// So: a `dxf` pattern that states no `@patternType` and does state a `bgColor` is a **solid fill of
/// that colour**. DocumentedBehaviour — §18.8.20's `bgColor` is *"the background colour of the cell
/// fill pattern"*, and with no pattern the background is the whole cell.
///
/// A `dxf` that states a real `@patternType` is read exactly as a cell's fill is, foreground and
/// all, because then the two colours mean what they always mean.
fn differential_fill(format: &DifferentialFormat, interner: &Interner) -> Option<CellFill> {
    let fill = format.fill()?;
    if let Some(pattern) = fill.pattern() {
        let kind = pattern.pattern_type(interner).ok().flatten();
        let foreground = pattern.foreground_colour(interner);
        let background = pattern.background_colour(interner);
        return match (kind, &foreground, &background) {
            (None | Some(PatternType::None), None, Some(_)) => Some(CellFill {
                pattern: Some(PatternType::Solid),
                foreground: background,
                background: None,
                gradient: None,
            }),
            (None | Some(PatternType::None), None, None) => None,
            _ => Some(CellFill {
                pattern: kind,
                foreground,
                background,
                gradient: None,
            }),
        };
    }
    // A `dxf` may carry a gradient fill; it is read exactly as a cell's is.
    fill.gradient().map(|gradient| CellFill {
        pattern: None,
        foreground: None,
        background: None,
        gradient: Some(crate::model::gradient_from(gradient, interner)),
    })
}
