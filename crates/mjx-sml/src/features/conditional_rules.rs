//! The conditional-formatting block and the rules in it: `CT_ConditionalFormatting`, `CT_CfRule`
//! and the `ST_Formula` element a rule states its condition in.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_ConditionalFormatting` | 2709 | `x:conditionalFormatting` (rank **16** of `CT_Worksheet`) |
//! | `CT_CfRule` | 2717 | `x:conditionalFormatting/cfRule` |
//! | `ST_Formula` | — | `x:cfRule/formula` |
//!
//! # A worksheet carries a *list* of blocks, and that is the whole reason this is its own subject
//!
//! `conditionalFormatting` and `cols` are the **only two** children of `CT_Worksheet` declared
//! `maxOccurs="unbounded"`. A sheet therefore holds an ordered list of blocks, each with its own
//! `@sqref` and its own rules — and `cfRule@priority` is workbook-scoped, so the rules that apply to
//! one cell are drawn from *every* block whose `@sqref` covers it and ordered across all of them at
//! once. [`crate::features::conditional_chain`] is where that merge lives; this file is the markup
//! it merges.
//!
//! # What is reported and what is never decided
//!
//! **Reporting which rules apply to a cell is in scope. Deciding whether a rule's condition is
//! *true* is not**, and never will be: that needs a calculation engine, and `PLAN.md` settles the
//! absence of one as scope rather than as an omission. A `cellIs` rule with
//! `operator="greaterThan"` and `<formula>0.5</formula>` is reported as exactly that — an operator
//! and a piece of text — and the text goes through MJXOFF-115's contract, which is that a formula is
//! carried and never parsed, rewritten or evaluated.
//!
//! # Priorities are as read, and are never renumbered
//!
//! §18.3.1.10 defines `@priority` as *"The priority of this conditional formatting rule … Lower
//! numeric values are higher priority than higher numeric values, where 1 is the highest priority"*.
//! It does **not** say the numbers are dense, unique, or start at 1, and files Excel itself writes
//! have gaps and duplicates in them. Nothing in this crate renumbers a priority on write: doing so
//! would change which rule wins, which is a correction the caller did not ask for and the sixth
//! guise of this phase's recurring hazard.
//!
//! # The `x14` extension namespace
//!
//! The modern conditional formats — data bars with negative fills, icon-set overrides — are carried
//! in an `x14:conditionalFormattings` inside the block's own `extLst`, and are **not modelled here**.
//! They land in the unmodelled bucket of [`ConditionalFormatting`] and of
//! [`ConditionalFormattingRule`], keep their prefix, their attribute order and their bytes, and come
//! back out of an unrelated edit unchanged. Not modelling them is a scope decision; losing them
//! would be a defect.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::{CONDITIONAL_FORMAT_RULE, WORKSHEET_CONDITIONAL_FORMATTING};
use mjx_ooxml_types::spreadsheetml::{
    ConditionalFormatType, ConditionalFormattingOperator, TimePeriod,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRangeList;
use crate::worksheet::rebuild_element;

use super::conditional_scales::{ColorScale, DataBar, IconSet};

/// `x:cfRule/formula` (`ST_Formula`) — one condition of a rule, as text.
///
/// # Why this is not `#[derive(FromXml, ToXml)]`
///
/// The same reason [`DefinedName`](crate::DefinedName) is not: `mjx-derive`'s `#[xml(text)]` grammar
/// re-escapes character data **minimally** on write, so a producer that spelled a comparison
/// `&quot;OK&quot;` would get `"OK"` back — the same string and different bytes. That is invisible
/// while the part is copied verbatim and becomes visible the moment anything *else* in the part
/// changes, because a rebuilt text node denies its element, and every ancestor of it, the verbatim
/// source range it would otherwise keep.
///
/// So the pair is written by hand: [`from_xml`](FromXml::from_xml) keeps the original children as
/// they stood, and the rebuild replays them until [`set_text`](Self::set_text) states otherwise.
///
/// # It is never evaluated, and never rewritten
///
/// MJXOFF-115's contract, restated for the one other place `sml.xsd` puts a formula that is not a
/// cell's. Nothing here parses the expression, translates it between `A1` and `R1C1`, or offsets its
/// references when a rule is copied to another range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalFormattingFormula {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    /// The character data, decoded — what [`text`](Self::text) answers with.
    text: String,
    /// The element's children exactly as the file wrote them, or `None` once the text has been
    /// replaced and there is nothing left to preserve.
    verbatim: Option<Vec<RawNode>>,
}

impl ConditionalFormattingFormula {
    /// Builds an `x:formula` holding `text`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, text: impl Into<String>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "formula"),
            attributes: Vec::new(),
            empty: false,
            text: text.into(),
            verbatim: None,
        }
    }

    /// The condition's text, exactly as the file wrote it (entity references decoded).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text.
    ///
    /// Nothing validates it: a condition is a formula, formulas are text here, and a caller that
    /// writes something Excel will refuse has written what a producer is free to write.
    ///
    /// This is the point at which the preserved character data is given up.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.verbatim = None;
        self.empty = false;
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = match &self.verbatim {
            // Untouched: replay exactly what the file held — entity spellings and CDATA included.
            Some(children) => children.clone(),
            None if self.text.is_empty() => Vec::new(),
            None => vec![RawNode::Text(
                mjx_xml::text::escape_text(&self.text).as_bytes().into(),
            )],
        };
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl FromXml for ConditionalFormattingFormula {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        let mut text = String::new();
        for child in &element.children {
            match child {
                RawNode::Text(bytes) => {
                    let raw = core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                    let decoded = mjx_xml::text::unescape_text(raw)
                        .map_err(|error| FromXmlError::InvalidEntity(error.to_string()))?;
                    text.push_str(&decoded);
                }
                RawNode::CData(bytes) => {
                    text.push_str(
                        core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?,
                    );
                }
                _ => {}
            }
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text,
            verbatim: Some(element.children.clone()),
        })
    }
}

impl ToXml for ConditionalFormattingFormula {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:cfRule` (`CT_CfRule`, `sml.xsd:2717`) — one conditional-formatting rule.
///
/// **`ST_`/`CT_` symbol:** `CT_CfRule`. Wire element: `cfRule`.
///
/// `@priority` is the only `use="required"` attribute, and `@type` is the one that decides which of
/// the other eleven mean anything: §18.3.1.10 says of nearly every one of them *"This attribute is
/// ignored if type is not equal to …"*. Nothing here enforces that — a `top10` attribute on a
/// `cellIs` rule is preserved and reported, because it is what the file says.
///
/// `@dxfId` is *"an index to a `dxf` element in the Styles Part indicating which cell formatting to
/// apply when the conditional formatting rule criteria is met"*. It is an **index**, so
/// [`crate::DifferentialFormats`]'s rule holds: append, never reorder.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "type", codec = Enumeration<ConditionalFormatType>, accessor = kind))]
#[xml(attribute(local = "dxfId", codec = Number<u32>, accessor = differential_format_index))]
#[xml(attribute(local = "priority", codec = Number<i32>, accessor = priority, required))]
#[xml(attribute(
    local = "stopIfTrue",
    codec = OnOff,
    accessor = stops_lower_priority_rules,
    default = false
))]
#[xml(attribute(local = "aboveAverage", codec = OnOff, accessor = is_above_average, default = true))]
#[xml(attribute(local = "percent", codec = OnOff, accessor = ranks_by_percent, default = false))]
#[xml(attribute(local = "bottom", codec = OnOff, accessor = ranks_from_bottom, default = false))]
#[xml(attribute(
    local = "operator",
    codec = Enumeration<ConditionalFormattingOperator>,
    accessor = operator
))]
#[xml(attribute(local = "text", codec = Text, accessor = text))]
#[xml(attribute(local = "timePeriod", codec = Enumeration<TimePeriod>, accessor = time_period))]
#[xml(attribute(local = "rank", codec = Number<u32>, accessor = top_or_bottom_count))]
#[xml(attribute(local = "stdDev", codec = Number<i32>, accessor = standard_deviations))]
#[xml(attribute(
    local = "equalAverage",
    codec = OnOff,
    accessor = includes_the_average,
    default = false
))]
pub struct ConditionalFormattingRule {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "formula", variant = Formula, ty = ConditionalFormattingFormula),
        child(local = "colorScale", variant = ColorScale, ty = ColorScale),
        child(local = "dataBar", variant = DataBar, ty = DataBar),
        child(local = "iconSet", variant = IconSet, ty = IconSet)
    )]
    content: Vec<ConditionalFormattingRuleContent>,
}

/// One child of [`ConditionalFormattingRule`]: four modelled members, and `extLst`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalFormattingRuleContent {
    /// `x:formula` (rank 0) — one of up to three conditions, as text.
    Formula(ConditionalFormattingFormula),
    /// `x:colorScale` (rank 1).
    ColorScale(ColorScale),
    /// `x:dataBar` (rank 2).
    DataBar(DataBar),
    /// `x:iconSet` (rank 3).
    IconSet(IconSet),
    /// `x:extLst` (rank 4) — where the `x14` variants live — and anything else, preserved verbatim
    /// and in position.
    Raw(RawNode),
}

impl ConditionalFormattingRuleContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Formula(_) => "formula",
            Self::ColorScale(_) => "colorScale",
            Self::DataBar(_) => "dataBar",
            Self::IconSet(_) => "iconSet",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_CfRule`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        CONDITIONAL_FORMAT_RULE.rank_of(None, self.local()?)
    }
}

/// Declares one singleton member of a rule: a borrowing getter and a setter that replaces the
/// existing element in place or inserts a new one at its rank in `CT_CfRule`'s sequence.
macro_rules! rule_member {
    ($getter:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                ConditionalFormattingRuleContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some` replaces the existing \
            element **where it is**, or inserts one at its rank in `CT_CfRule`'s `xsd:sequence` — \
            after the formulas, and before `extLst`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            self.replace_or_insert(
                $local,
                |item| matches!(item, ConditionalFormattingRuleContent::$variant(_)),
                value.map(ConditionalFormattingRuleContent::$variant),
            );
        }
    };
}

impl ConditionalFormattingRule {
    /// Builds an `x:cfRule` with every attribute absent, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `@priority` is `use="required"` and is **not** set here, because inventing one would be
    /// inventing the thing that decides which rule wins. A caller builds the rule and then states
    /// its priority.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "cfRule"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[ConditionalFormattingRuleContent] {
        &self.content
    }

    /// Every `x:formula`, in document order — the schema allows up to three.
    ///
    /// A `cellIs` rule with `operator="between"` writes two; every other operator writes one; a
    /// `colorScale`, `dataBar` or `iconSet` rule writes none.
    pub fn formulas(&self) -> impl Iterator<Item = &ConditionalFormattingFormula> + '_ {
        self.content.iter().filter_map(|item| match item {
            ConditionalFormattingRuleContent::Formula(formula) => Some(formula),
            _ => None,
        })
    }

    /// The `index`-th `x:formula`, mutably.
    pub fn formula_mut(&mut self, index: usize) -> Option<&mut ConditionalFormattingFormula> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ConditionalFormattingRuleContent::Formula(formula) => Some(formula),
                _ => None,
            })
            .nth(index)
    }

    /// Appends a condition after the formulas already present, and before whichever of
    /// `colorScale`, `dataBar` and `iconSet` the rule carries.
    pub fn push_formula(&mut self, formula: ConditionalFormattingFormula) {
        let at = self.insert_index("formula");
        self.content
            .insert(at, ConditionalFormattingRuleContent::Formula(formula));
        self.empty = false;
    }

    rule_member!(
        color_scale,
        set_color_scale,
        ColorScale,
        ColorScale,
        "colorScale",
        "`x:colorScale` — the gradated colour scale a `type=\"colorScale\"` rule draws."
    );
    rule_member!(
        data_bar,
        set_data_bar,
        DataBar,
        DataBar,
        "dataBar",
        "`x:dataBar` — the in-cell bar a `type=\"dataBar\"` rule draws."
    );
    rule_member!(
        icon_set,
        set_icon_set,
        IconSet,
        IconSet,
        "iconSet",
        "`x:iconSet` — the icons a `type=\"iconSet\"` rule draws."
    );

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ConditionalFormattingRuleContent::Formula(formula) => {
                    RawNode::Element(formula.as_raw_element())
                }
                ConditionalFormattingRuleContent::ColorScale(scale) => {
                    RawNode::Element(scale.as_raw_element())
                }
                ConditionalFormattingRuleContent::DataBar(bar) => {
                    RawNode::Element(bar.as_raw_element())
                }
                ConditionalFormattingRuleContent::IconSet(icons) => {
                    RawNode::Element(icons.as_raw_element())
                }
                ConditionalFormattingRuleContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }

    /// Where a child named `local` belongs, from the generated table.
    fn insert_index(&self, local: &str) -> usize {
        CONDITIONAL_FORMAT_RULE.insert_index_of_names(
            self.content
                .iter()
                .map(ConditionalFormattingRuleContent::rank),
            local,
        )
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&ConditionalFormattingRuleContent) -> bool,
        value: Option<ConditionalFormattingRuleContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = self.insert_index(local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

impl ToXml for ConditionalFormattingRule {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:conditionalFormatting` (`CT_ConditionalFormatting`, `sml.xsd:2709`) — one block: a range list
/// and the rules that apply over it.
///
/// **`ST_`/`CT_` symbol:** `CT_ConditionalFormatting`. Wire element: `conditionalFormatting`, rank
/// **16** of `CT_Worksheet` and one of only two of its children declared `maxOccurs="unbounded"`.
///
/// `@sqref` is an `ST_Sqref` — MJXOFF-93's [`CellRangeList`], the multi-range form conditional
/// formatting uses more than anything else in the schema, since a user who control-clicks three
/// blocks and applies one rule leaves `sqref="A1:B2 D4 F6:F9"`.
///
/// `@pivot` says the block belongs to a PivotTable (§18.3.1.18). It is reported and never acted on;
/// this crate models no PivotTable.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "pivot", codec = OnOff, accessor = is_for_pivot_table, default = false))]
#[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = ranges))]
pub struct ConditionalFormatting {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cfRule", variant = Rule, ty = ConditionalFormattingRule))]
    content: Vec<ConditionalFormattingContent>,
}

/// One child of [`ConditionalFormatting`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalFormattingContent {
    /// `x:cfRule` (rank 0) — one rule.
    Rule(ConditionalFormattingRule),
    /// `x:extLst` (rank 1) — where the `x14` conditional formats live — and anything else,
    /// preserved verbatim and in position.
    Raw(RawNode),
}

impl ConditionalFormattingContent {
    /// This child's rank in `CT_ConditionalFormatting`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Rule(_) => WORKSHEET_CONDITIONAL_FORMATTING.rank_of(None, "cfRule"),
            Self::Raw(_) => None,
        }
    }
}

impl ConditionalFormatting {
    /// Builds an empty `x:conditionalFormatting`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `cfRule` `minOccurs="1"`, so a block with no rule is invalid markup; it
    /// is still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "conditionalFormatting"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[ConditionalFormattingContent] {
        &self.content
    }

    /// Every `x:cfRule`, in document order.
    ///
    /// **Document order, not priority order.** A block's rules are ordered across *every* block by
    /// `@priority`, which is why [`crate::WorksheetPart::conditional_rules_for`] exists and why this
    /// accessor deliberately does not sort.
    pub fn rules(&self) -> impl Iterator<Item = &ConditionalFormattingRule> + '_ {
        self.content.iter().filter_map(|item| match item {
            ConditionalFormattingContent::Rule(rule) => Some(rule),
            ConditionalFormattingContent::Raw(_) => None,
        })
    }

    /// How many `x:cfRule` children this block holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules().count()
    }

    /// Whether the block holds no rule at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:cfRule`, mutably.
    pub fn rule_mut(&mut self, index: usize) -> Option<&mut ConditionalFormattingRule> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ConditionalFormattingContent::Rule(rule) => Some(rule),
                ConditionalFormattingContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a rule after the ones already present, and **before** an `extLst` if the block has
    /// one.
    ///
    /// The rule keeps whatever `@priority` it carries. Nothing here renumbers, reorders or
    /// deduplicates: see this module's own documentation.
    pub fn push_rule(&mut self, rule: ConditionalFormattingRule) {
        let at = WORKSHEET_CONDITIONAL_FORMATTING.insert_index_of_names(
            self.content.iter().map(ConditionalFormattingContent::rank),
            "cfRule",
        );
        self.content
            .insert(at, ConditionalFormattingContent::Rule(rule));
        self.empty = false;
    }

    /// Removes the `index`-th `x:cfRule` and returns it, or `None` when the block holds fewer.
    ///
    /// Markup between the rules is left where it is: only the rule element itself is taken out.
    pub fn remove_rule(&mut self, index: usize) -> Option<ConditionalFormattingRule> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, ConditionalFormattingContent::Rule(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            ConditionalFormattingContent::Rule(rule) => Some(rule),
            ConditionalFormattingContent::Raw(_) => {
                unreachable!("the position was filtered on `Rule`")
            }
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ConditionalFormattingContent::Rule(rule) => RawNode::Element(rule.as_raw_element()),
                ConditionalFormattingContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for ConditionalFormatting {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
