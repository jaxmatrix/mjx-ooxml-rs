//! What-if scenarios: `CT_Scenarios`, `CT_Scenario` and `CT_InputCells`.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_Scenarios` | 2879 | `x:scenarios` (rank 9 of `CT_Worksheet`) |
//! | `CT_Scenario` | 2929 | `x:scenarios/scenario` |
//! | `CT_InputCells` | 2940 | `x:scenarios/scenario/inputCells` |
//!
//! # A scenario is a saved set of cell values, and this crate never applies one
//!
//! Excel's Scenario Manager saves named alternatives for a handful of input cells: *"Best case" puts
//! 0.08 in B4 and 12000 in B5*. Each alternative is one `<scenario>`, and each cell it overrides is
//! one `<inputCells>` carrying the address and the value **as text**.
//!
//! The values are stored as text on the wire (`@val` is an `ST_Xstring`) and stay text here. Nothing
//! in this crate writes a scenario's values into the cells: `@current` and `@show` name which
//! scenario a consumer had selected, and acting on them would rewrite `sheetData` from a part of the
//! file that only *describes* an alternative. That is the same rule
//! [`SheetCalculationProperties`](super::SheetCalculationProperties) follows for `fullCalcOnLoad` —
//! carried from the producer to the next consumer, reported, never acted on.
//!
//! Scenarios are modelled by this child rather than by a later one because they are structurally
//! part of the sheet — three small complex types in one slot, between `protectedRanges` and
//! `autoFilter` — and not a feature with a vocabulary of its own.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::{CellRangeList, CellReference};
use crate::leaf::attribute_bag;

use super::rebuild_element;

attribute_bag! {
    /// `x:inputCells` (`CT_InputCells`, `sml.xsd:2940`) — one cell a scenario overrides, and the
    /// value it puts there.
    ///
    /// `@r` and `@val` are the two `use="required"` attributes. `@val` is an `ST_Xstring` whatever
    /// the cell's own type is — a number, a boolean and an error all arrive here as text, because a
    /// scenario records what to *type into* the cell rather than a typed cell value. Nothing decodes
    /// it.
    ///
    /// `@deleted` and `@undone` are Excel's bookkeeping for a cell that is gone, and both accessor
    /// names are ECMA-376 Part 1 §18.3.1.52's own sentences rather than the wire tokens: *"Input cell
    /// was deleted. This input cell shall be present in the file format, but shall not be presented
    /// to the user as part of the scenario inputs"*, and *"Cell's deletion was undone"*. Both are
    /// preserved and neither is acted on — this crate does not run scenarios.
    #[xml(attribute(local = "r", codec = Enumeration<CellReference>, accessor = cell, required))]
    #[xml(attribute(local = "val", codec = Text, accessor = value, required))]
    #[xml(attribute(local = "deleted", codec = OnOff, accessor = input_cell_was_deleted, default = false))]
    #[xml(attribute(local = "undone", codec = OnOff, accessor = deletion_was_undone, default = false))]
    #[xml(attribute(local = "numFmtId", codec = Number<u32>, accessor = number_format_id))]
    ScenarioInputCells, "inputCells"
}

/// `x:scenario` (`CT_Scenario`, `sml.xsd:2929`) — one named alternative and the cells it changes.
///
/// `@name` is the only `use="required"` attribute. `@count` is a hint over the `inputCells`
/// children, kept in step when the collection is edited and never added to a scenario that wrote
/// none. `@user` and `@comment` are the person who saved it and the note they left.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "name", codec = Text, accessor = name, required))]
#[xml(attribute(local = "locked", codec = OnOff, accessor = is_locked, default = false))]
#[xml(attribute(local = "hidden", codec = OnOff, accessor = is_hidden, default = false))]
#[xml(attribute(local = "count", codec = Number<u32>, accessor = declared_count))]
#[xml(attribute(local = "user", codec = Text, accessor = user))]
#[xml(attribute(local = "comment", codec = Text, accessor = comment))]
pub struct Scenario {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "inputCells", variant = InputCells, ty = ScenarioInputCells))]
    content: Vec<ScenarioContent>,
}

/// One child of [`Scenario`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioContent {
    /// `x:inputCells` — one overridden cell.
    InputCells(ScenarioInputCells),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl Scenario {
    /// Builds an empty `x:scenario`, bound to `prefix` or to the default namespace.
    ///
    /// `@name` is `use="required"` and is not set here: a caller builds the element and then names
    /// it, because inventing a name would be inventing markup.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "scenario"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ScenarioContent] {
        &self.content
    }

    /// Every `x:inputCells`, in document order.
    pub fn input_cells(&self) -> impl Iterator<Item = &ScenarioInputCells> + '_ {
        self.content.iter().filter_map(|item| match item {
            ScenarioContent::InputCells(cells) => Some(cells),
            ScenarioContent::Raw(_) => None,
        })
    }

    /// How many `x:inputCells` children this scenario holds — the number `@count` claims.
    #[must_use]
    pub fn len(&self) -> usize {
        self.input_cells().count()
    }

    /// Whether the scenario overrides no cell at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:inputCells`, mutably.
    pub fn input_cells_mut(&mut self, index: usize) -> Option<&mut ScenarioInputCells> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ScenarioContent::InputCells(cells) => Some(cells),
                ScenarioContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends an overridden cell, updating `@count` when the file declared one.
    pub fn push(&mut self, interner: &mut Interner, cells: ScenarioInputCells) {
        self.content.push(ScenarioContent::InputCells(cells));
        self.empty = false;
        if self.declared_count(interner).ok().flatten().is_some() {
            let count = u32::try_from(self.len()).unwrap_or(u32::MAX);
            self.set_declared_count(interner, Some(count));
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ScenarioContent::InputCells(cells) => RawNode::Element(cells.as_raw_element()),
                ScenarioContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for Scenario {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:scenarios` (`CT_Scenarios`, `sml.xsd:2879`) — every scenario saved for the sheet.
///
/// `@current` and `@show` are indices into this list: the one last applied and the one last shown.
/// `@sqref` is an `ST_Sqref` naming the cells the scenarios collectively change, which is MJXOFF-93's
/// [`CellRangeList`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "current", codec = Number<u32>, accessor = current_index))]
#[xml(attribute(local = "show", codec = Number<u32>, accessor = shown_index))]
#[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = ranges))]
pub struct Scenarios {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "scenario", variant = Scenario, ty = Scenario))]
    content: Vec<ScenariosContent>,
}

/// One child of [`Scenarios`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenariosContent {
    /// `x:scenario` — one named alternative.
    Scenario(Scenario),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl Scenarios {
    /// Builds an empty `x:scenarios`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "scenarios"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ScenariosContent] {
        &self.content
    }

    /// Every `x:scenario`, in document order.
    pub fn scenarios(&self) -> impl Iterator<Item = &Scenario> + '_ {
        self.content.iter().filter_map(|item| match item {
            ScenariosContent::Scenario(scenario) => Some(scenario),
            ScenariosContent::Raw(_) => None,
        })
    }

    /// How many `x:scenario` children this element holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.scenarios().count()
    }

    /// Whether the element holds no scenario at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:scenario`, mutably.
    pub fn scenario_mut(&mut self, index: usize) -> Option<&mut Scenario> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ScenariosContent::Scenario(scenario) => Some(scenario),
                ScenariosContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a scenario after the ones already present.
    ///
    /// `@current` and `@show` are **not** touched: they are the consumer's own selection, and moving
    /// them because a scenario was added would change which alternative Excel opens with.
    pub fn push(&mut self, scenario: Scenario) {
        self.content.push(ScenariosContent::Scenario(scenario));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ScenariosContent::Scenario(scenario) => RawNode::Element(scenario.as_raw_element()),
                ScenariosContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for Scenarios {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
