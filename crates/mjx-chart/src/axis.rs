//! The chart's **furniture** — its axes, their scaling and titles, the gridlines, the chart title
//! and the legend.
//!
//! These are the parts of a chart that are not its data: `c:catAx` / `c:valAx` / `c:dateAx` /
//! `c:serAx` (all four share `EG_AxShared`, so they share one model here), `c:scaling`, `c:title`
//! and `c:legend`. Until this tier they rode through the `Raw` bucket with no typed surface at all —
//! preserved perfectly, readable not at all.
//!
//! ```xml
//! <c:valAx>
//!   <c:axId val="222222222"/>
//!   <c:scaling><c:orientation val="minMax"/><c:max val="100"/></c:scaling>
//!   <c:delete val="0"/>
//!   <c:axPos val="l"/>
//!   <c:majorGridlines/>
//!   <c:title>…</c:title>
//!   <c:crossAx val="111111111"/>
//! </c:valAx>
//! ```
//!
//! # Fidelity
//!
//! The same ordered-`content` + `Raw` shape as everything else in this crate: only `c:scaling`,
//! `c:title` and the two gridline elements are typed on an axis; its scalars (`c:axId`, `c:delete`,
//! `c:axPos`, `c:majorTickMark`, `c:tickLblPos`, `c:crossAx`, …) stay opaque and are read through
//! accessors, so an axis re-emits byte-for-byte. A setter that inserts a child places it at the
//! position `EG_AxShared` gives it, so an edited axis stays schema-valid.

use mjx_derive::{FromXml, ToXml};
use mjx_dml::TextBody;
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::support::on_off;

use mjx_ooxml_types::child_order::{
    ChildOrder, CATEGORY_AXIS, DATE_AXIS, SCALING, SERIES_AXIS, VALUE_AXIS,
};

use crate::build::{
    chart_attr, chart_element, chart_name, chart_val_leaf, dml_element, dml_text_leaf, f64_wire,
    insert_position, is_chart, raw_child_attr, set_attr,
};
use crate::data::StringReference;

/// Which kind of axis an [`Axis`] is — the element it was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisKind {
    /// A category axis (`c:catAx`) — discrete labels.
    Category,
    /// A value axis (`c:valAx`) — a continuous numeric scale.
    Value,
    /// A date axis (`c:dateAx`) — a continuous time scale.
    Date,
    /// A series axis (`c:serAx`) — the depth axis of a three-dimensional plot.
    Series,
}

impl AxisKind {
    /// The element name this kind is written as, without its `c:` prefix.
    #[must_use]
    pub fn element_local_name(self) -> &'static str {
        match self {
            Self::Category => "catAx",
            Self::Value => "valAx",
            Self::Date => "dateAx",
            Self::Series => "serAx",
        }
    }
}

/// Where an axis sits against the plot area (`c:axPos@val`, `ST_AxPos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPosition {
    /// Along the bottom (wire `b`).
    Bottom,
    /// Up the left-hand side (wire `l`).
    Left,
    /// Up the right-hand side (wire `r`).
    Right,
    /// Along the top (wire `t`).
    Top,
}

impl AxisPosition {
    /// Maps the wire token to a position.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "b" => Some(Self::Bottom),
            "l" => Some(Self::Left),
            "r" => Some(Self::Right),
            "t" => Some(Self::Top),
            _ => None,
        }
    }

    /// The exact wire token for this position.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Bottom => "b",
            Self::Left => "l",
            Self::Right => "r",
            Self::Top => "t",
        }
    }
}

/// Which way an axis runs (`c:orientation@val`, `ST_Orientation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisOrientation {
    /// Smallest value first — the usual direction (wire `minMax`).
    MinimumToMaximum,
    /// Reversed: largest value first (wire `maxMin`).
    MaximumToMinimum,
}

impl AxisOrientation {
    /// Maps the wire token to an orientation.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "minMax" => Some(Self::MinimumToMaximum),
            "maxMin" => Some(Self::MaximumToMinimum),
            _ => None,
        }
    }

    /// The exact wire token for this orientation.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::MinimumToMaximum => "minMax",
            Self::MaximumToMinimum => "maxMin",
        }
    }
}

/// How an axis draws its tick marks (`c:majorTickMark`/`c:minorTickMark@val`, `ST_TickMark`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickMark {
    /// Crossing the axis line (wire `cross`).
    Cross,
    /// Inside the plot area (wire `in`).
    Inside,
    /// No tick marks (wire `none`).
    None,
    /// Outside the plot area (wire `out`).
    Outside,
}

impl TickMark {
    /// Maps the wire token to a tick-mark style.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "cross" => Some(Self::Cross),
            "in" => Some(Self::Inside),
            "none" => Some(Self::None),
            "out" => Some(Self::Outside),
            _ => None,
        }
    }
}

/// Where an axis puts its tick labels (`c:tickLblPos@val`, `ST_TickLblPos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickLabelPosition {
    /// At the high end of the crossing axis (wire `high`).
    High,
    /// At the low end of the crossing axis (wire `low`).
    Low,
    /// Next to the axis (wire `nextTo`).
    NextToAxis,
    /// No tick labels (wire `none`).
    None,
}

impl TickLabelPosition {
    /// Maps the wire token to a label position.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "high" => Some(Self::High),
            "low" => Some(Self::Low),
            "nextTo" => Some(Self::NextToAxis),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// Where the legend sits (`c:legendPos@val`, `ST_LegendPos`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendPosition {
    /// Below the plot area (wire `b`).
    Bottom,
    /// Left of the plot area (wire `l`).
    Left,
    /// Right of the plot area (wire `r`).
    Right,
    /// Above the plot area (wire `t`).
    Top,
    /// In the top-right corner (wire `tr`).
    TopRight,
}

impl LegendPosition {
    /// Maps the wire token to a position.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "b" => Some(Self::Bottom),
            "l" => Some(Self::Left),
            "r" => Some(Self::Right),
            "t" => Some(Self::Top),
            "tr" => Some(Self::TopRight),
            _ => None,
        }
    }

    /// The exact wire token for this position.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Bottom => "b",
            Self::Left => "l",
            Self::Right => "r",
            Self::Top => "t",
            Self::TopRight => "tr",
        }
    }
}

/// What a chart draws in place of a blank value (`c:dispBlanksAs@val`, `ST_DispBlanksAs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankDisplay {
    /// Bridge the blank, joining the points either side (wire `span`).
    Span,
    /// Leave a gap (wire `gap`).
    Gap,
    /// Plot the blank as zero (wire `zero`).
    Zero,
}

impl BlankDisplay {
    /// Maps the wire token to a display rule.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "span" => Some(Self::Span),
            "gap" => Some(Self::Gap),
            "zero" => Some(Self::Zero),
            _ => None,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// c:majorGridlines / c:minorGridlines
// -------------------------------------------------------------------------------------------------

/// `c:majorGridlines` / `c:minorGridlines` (`CT_ChartLines`) — the lines an axis rules across the
/// plot area. The element's presence *is* the setting; its only child is an optional `a:spPr`
/// styling the line, kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gridlines {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(Gridlines);

impl Gridlines {
    /// A fresh, empty `c:local` gridlines element (`majorGridlines` or `minorGridlines`).
    pub(crate) fn new(interner: &mut Interner, local: &str) -> Self {
        let element = chart_element(interner, local, Vec::new(), Vec::new());
        let (name, empty) = (element.name, element.empty);
        let content = element.into_content();
        Self {
            name,
            attributes: content.attributes,
            children: content.children,
            empty,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// c:title
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`TitleText`]: the rich text, a workbook reference, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleTextContent {
    /// The title's own formatted text (`c:rich`, an `a:CT_TextBody`).
    Rich(TextBody),
    /// A workbook reference naming the cell the title reads from (`c:strRef`).
    Reference(StringReference),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `c:tx` (`CT_Tx`) — where a title's words come from: written into the chart (`c:rich`) or read
/// from a workbook cell (`c:strRef`).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct TitleText {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rich", variant = Rich, ty = TextBody),
        child(local = "strRef", variant = Reference, ty = StringReference)
    )]
    content: Vec<TitleTextContent>,
}

impl TitleText {
    /// A fresh `c:tx` holding `text` as one rich-text paragraph.
    pub(crate) fn new(interner: &mut Interner, text: &str) -> Self {
        let run = {
            let t = dml_text_leaf(interner, "t", text);
            dml_element(interner, "r", Vec::new(), vec![RawNode::Element(t)])
        };
        let paragraph = dml_element(interner, "p", Vec::new(), vec![RawNode::Element(run)]);
        let body_properties = dml_element(interner, "bodyPr", Vec::new(), Vec::new());
        let list_style = dml_element(interner, "lstStyle", Vec::new(), Vec::new());
        let rich = chart_element(
            interner,
            "rich",
            Vec::new(),
            vec![
                RawNode::Element(body_properties),
                RawNode::Element(list_style),
                RawNode::Element(paragraph),
            ],
        );
        Self {
            name: chart_name(interner, "tx"),
            attributes: Vec::new(),
            empty: false,
            content: vec![TitleTextContent::Raw(RawNode::Element(rich))],
        }
    }

    /// The title's words — the rich text's paragraphs joined by newlines, or the first cached label
    /// of its workbook reference. `None` when it declares neither.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        for item in &self.content {
            match item {
                TitleTextContent::Rich(body) => return Some(body.text()),
                TitleTextContent::Reference(reference) => {
                    return reference.labels().into_iter().next();
                }
                TitleTextContent::Raw(_) => {}
            }
        }
        None
    }
}

/// One ordered child of a [`ChartTitle`]: its text, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartTitleContent {
    /// The title's text source (`c:tx`).
    Text(TitleText),
    /// Any other child — `c:layout`, `c:overlay`, `c:spPr`, `c:txPr` — preserved verbatim.
    Raw(RawNode),
}

/// `c:title` (`CT_Title`) — the heading over a chart, or the label beside an axis. The same element
/// serves both.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct ChartTitle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tx", variant = Text, ty = TitleText))]
    content: Vec<ChartTitleContent>,
}

impl ChartTitle {
    /// A fresh `c:title` reading `text`, with `c:overlay val="0"` so it does not sit on the plot.
    pub(crate) fn new(interner: &mut Interner, text: &str) -> Self {
        let title_text = TitleText::new(interner, text);
        let overlay = chart_val_leaf(interner, "overlay", "0");
        Self {
            name: chart_name(interner, "title"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                ChartTitleContent::Text(title_text),
                ChartTitleContent::Raw(RawNode::Element(overlay)),
            ],
        }
    }

    /// The title's text source (`c:tx`), or `None` if it declares none.
    #[must_use]
    pub fn text_source(&self) -> Option<&TitleText> {
        self.content.iter().find_map(|item| match item {
            ChartTitleContent::Text(text) => Some(text),
            ChartTitleContent::Raw(_) => None,
        })
    }

    /// The title's words, or `None` when it names no text source.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.text_source().and_then(TitleText::text)
    }

    /// Replaces the title's words, rewriting its `c:tx` (and adding one if it had none) and leaving
    /// its layout, overlay and styling untouched.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        let replacement = TitleText::new(interner, text);
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, ChartTitleContent::Text(_)))
        {
            self.content[index] = ChartTitleContent::Text(replacement);
        } else {
            self.content.insert(0, ChartTitleContent::Text(replacement));
            self.empty = false;
        }
    }
}

// -------------------------------------------------------------------------------------------------
// c:legend
// -------------------------------------------------------------------------------------------------

/// `c:legend` (`CT_Legend`) — the key naming each series. Its position and overlay flag are single
/// attribute scalars kept in the `Raw` bucket and read through accessors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Legend {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(Legend);

impl Legend {
    /// A fresh `c:legend` at `position`, not overlaying the plot.
    pub(crate) fn new(interner: &mut Interner, position: LegendPosition) -> Self {
        let legend_position = chart_val_leaf(interner, "legendPos", position.to_wire());
        let overlay = chart_val_leaf(interner, "overlay", "0");
        let element = chart_element(
            interner,
            "legend",
            Vec::new(),
            vec![RawNode::Element(legend_position), RawNode::Element(overlay)],
        );
        let (name, empty) = (element.name, element.empty);
        let content = element.into_content();
        Self {
            name,
            attributes: content.attributes,
            children: content.children,
            empty,
        }
    }

    /// Where the legend sits (`c:legendPos`), or `None` when it declares no position (Office then
    /// places it on the right).
    #[must_use]
    pub fn position(&self, interner: &Interner) -> Option<LegendPosition> {
        raw_child_attr(self.children.iter(), interner, "legendPos", "val")
            .and_then(LegendPosition::from_wire)
    }

    /// Whether the legend is drawn on top of the plot area rather than beside it (`c:overlay`).
    #[must_use]
    pub fn overlays_plot(&self, interner: &Interner) -> Option<bool> {
        raw_child_attr(self.children.iter(), interner, "overlay", "val").and_then(on_off::from_wire)
    }

    /// Moves the legend to `position`, rewriting its `c:legendPos` or inserting one first (it is the
    /// first child of `CT_Legend`).
    pub fn set_position(&mut self, interner: &mut Interner, position: LegendPosition) {
        for node in &mut self.children {
            if let RawNode::Element(element) = node {
                if is_chart(&element.name, interner)
                    && interner.resolve(element.name.local) == "legendPos"
                {
                    set_attr(&mut element.attributes, interner, "val", position.to_wire());
                    return;
                }
            }
        }
        let element = chart_val_leaf(interner, "legendPos", position.to_wire());
        self.children.insert(0, RawNode::Element(element));
        self.empty = false;
    }
}

// -------------------------------------------------------------------------------------------------
// c:scaling
// -------------------------------------------------------------------------------------------------

/// `c:scaling` (`CT_Scaling`) — how an axis maps values to distance: its direction, its explicit
/// bounds, and the base of a logarithmic scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaling {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

crate::build::fidelity_element_impls!(Scaling);

impl Scaling {
    /// A fresh `c:scaling` running smallest-value-first.
    pub(crate) fn new(interner: &mut Interner) -> Self {
        let orientation = chart_val_leaf(interner, "orientation", "minMax");
        let element = chart_element(
            interner,
            "scaling",
            Vec::new(),
            vec![RawNode::Element(orientation)],
        );
        let (name, empty) = (element.name, element.empty);
        let content = element.into_content();
        Self {
            name,
            attributes: content.attributes,
            children: content.children,
            empty,
        }
    }

    /// Which way the axis runs (`c:orientation`), or `None` when unset (Office then runs it
    /// smallest-first).
    #[must_use]
    pub fn orientation(&self, interner: &Interner) -> Option<AxisOrientation> {
        self.scalar(interner, "orientation")
            .and_then(AxisOrientation::from_wire)
    }

    /// The axis' explicit lower bound (`c:min`), or `None` when it scales automatically.
    #[must_use]
    pub fn minimum(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "min")
    }

    /// The axis' explicit upper bound (`c:max`), or `None` when it scales automatically.
    #[must_use]
    pub fn maximum(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "max")
    }

    /// The base of a logarithmic scale (`c:logBase`), or `None` for a linear axis.
    #[must_use]
    pub fn logarithm_base(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "logBase")
    }

    /// Sets the axis' direction.
    pub fn set_orientation(&mut self, interner: &mut Interner, orientation: AxisOrientation) {
        self.set_scalar(
            interner,
            "orientation",
            Some(orientation.to_wire().to_owned()),
        );
    }

    /// Sets or clears the axis' explicit lower bound. `None` returns the axis to automatic scaling;
    /// a non-finite value has no wire spelling and is treated as `None`.
    pub fn set_minimum(&mut self, interner: &mut Interner, minimum: Option<f64>) {
        self.set_scalar(interner, "min", minimum.and_then(f64_wire));
    }

    /// Sets or clears the axis' explicit upper bound. See [`set_minimum`](Self::set_minimum).
    pub fn set_maximum(&mut self, interner: &mut Interner, maximum: Option<f64>) {
        self.set_scalar(interner, "max", maximum.and_then(f64_wire));
    }

    /// The `@val` of a scalar child of the scaling.
    fn scalar(&self, interner: &Interner, local: &str) -> Option<&str> {
        raw_child_attr(self.children.iter(), interner, local, "val")
    }

    /// A scalar child's `@val` parsed as a number.
    fn number(&self, interner: &Interner, local: &str) -> Option<f64> {
        self.scalar(interner, local)
            .and_then(|value| value.trim().parse().ok())
    }

    /// Sets a scalar child's `@val` in place, inserts the child in its schema position, or — for
    /// `None` — removes it.
    fn set_scalar(&mut self, interner: &mut Interner, local: &str, value: Option<String>) {
        let existing = self.children.iter().position(|node| match node {
            RawNode::Element(element) => {
                is_chart(&element.name, interner) && interner.resolve(element.name.local) == local
            }
            _ => false,
        });
        match (existing, value) {
            (Some(index), Some(value)) => {
                if let RawNode::Element(element) = &mut self.children[index] {
                    set_attr(&mut element.attributes, interner, "val", &value);
                }
            }
            (Some(index), None) => {
                self.children.remove(index);
            }
            (None, Some(value)) => {
                let at = insert_position(
                    SCALING,
                    self.children.iter().map(|node| chart_local(node, interner)),
                    local,
                );
                let element = chart_val_leaf(interner, local, &value);
                self.children.insert(at, RawNode::Element(element));
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

// -------------------------------------------------------------------------------------------------
// c:catAx / c:valAx / c:dateAx / c:serAx
// -------------------------------------------------------------------------------------------------

/// One ordered child of an [`Axis`]: its scaling, its title, its gridlines, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisContent {
    /// How values map to distance (`c:scaling`).
    Scaling(Scaling),
    /// The lines ruled across the plot at the major ticks (`c:majorGridlines`).
    MajorGridlines(Gridlines),
    /// The lines ruled across the plot at the minor ticks (`c:minorGridlines`).
    MinorGridlines(Gridlines),
    /// The axis' label (`c:title`).
    Title(ChartTitle),
    /// Any other child — `c:axId`, `c:delete`, `c:axPos`, `c:numFmt`, `c:crossAx`, `c:spPr`,
    /// `c:txPr`, the type-specific tail — preserved verbatim.
    Raw(RawNode),
}

/// `c:catAx` / `c:valAx` / `c:dateAx` / `c:serAx` — one axis of a plot area.
///
/// All four elements share `EG_AxShared` — an id, a scaling, a position, optional gridlines, a
/// title, tick and label settings, and the id of the axis they cross — and differ only in the tail
/// each adds. One type models all four; [`kind`](Self::kind) says which it was read from, and the
/// tail rides verbatim through the `Raw` bucket.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Axis {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "scaling", variant = Scaling, ty = Scaling),
        child(local = "majorGridlines", variant = MajorGridlines, ty = Gridlines),
        child(local = "minorGridlines", variant = MinorGridlines, ty = Gridlines),
        child(local = "title", variant = Title, ty = ChartTitle)
    )]
    content: Vec<AxisContent>,
}

impl Axis {
    /// Which kind of axis this is, from the element it was read from — `None` for an element this
    /// type was handed that is not one of the four.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<AxisKind> {
        match interner.resolve(self.name.local) {
            "catAx" => Some(AxisKind::Category),
            "valAx" => Some(AxisKind::Value),
            "dateAx" => Some(AxisKind::Date),
            "serAx" => Some(AxisKind::Series),
            _ => None,
        }
    }

    /// The axis' id (`c:axId`) — what a plot's `c:axId` and the partner axis' `c:crossAx` name.
    #[must_use]
    pub fn axis_id(&self, interner: &Interner) -> Option<u32> {
        self.number(interner, "axId")
    }

    /// The id of the axis this one crosses (`c:crossAx`).
    #[must_use]
    pub fn cross_axis_id(&self, interner: &Interner) -> Option<u32> {
        self.number(interner, "crossAx")
    }

    /// Whether the axis is hidden (`c:delete`) — `Some(true)` means Office draws nothing for it.
    #[must_use]
    pub fn is_suppressed(&self, interner: &Interner) -> Option<bool> {
        self.scalar(interner, "delete").and_then(on_off::from_wire)
    }

    /// Where the axis sits against the plot area (`c:axPos`).
    #[must_use]
    pub fn position(&self, interner: &Interner) -> Option<AxisPosition> {
        self.scalar(interner, "axPos")
            .and_then(AxisPosition::from_wire)
    }

    /// How the major ticks are drawn (`c:majorTickMark`).
    #[must_use]
    pub fn major_tick_mark(&self, interner: &Interner) -> Option<TickMark> {
        self.scalar(interner, "majorTickMark")
            .and_then(TickMark::from_wire)
    }

    /// How the minor ticks are drawn (`c:minorTickMark`).
    #[must_use]
    pub fn minor_tick_mark(&self, interner: &Interner) -> Option<TickMark> {
        self.scalar(interner, "minorTickMark")
            .and_then(TickMark::from_wire)
    }

    /// Where the tick labels are placed (`c:tickLblPos`).
    #[must_use]
    pub fn tick_label_position(&self, interner: &Interner) -> Option<TickLabelPosition> {
        self.scalar(interner, "tickLblPos")
            .and_then(TickLabelPosition::from_wire)
    }

    /// The axis' number format (`c:numFmt@formatCode`, e.g. `0.00%`), or `None` when it inherits.
    #[must_use]
    pub fn number_format(&self, interner: &Interner) -> Option<&str> {
        let raw = self.content.iter().filter_map(|item| match item {
            AxisContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, "numFmt", "formatCode")
    }

    /// The axis' scaling (`c:scaling`), or `None` if it declares none (the schema requires one, so
    /// this is `None` only for malformed input).
    #[must_use]
    pub fn scaling(&self) -> Option<&Scaling> {
        self.content.iter().find_map(|item| match item {
            AxisContent::Scaling(scaling) => Some(scaling),
            _ => None,
        })
    }

    /// The axis' scaling, creating one in its schema position if it declares none.
    pub fn scaling_mut(&mut self, interner: &mut Interner) -> &mut Scaling {
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, AxisContent::Scaling(_)))
        {
            let AxisContent::Scaling(scaling) = &mut self.content[index] else {
                unreachable!("the index was just found by matching this variant")
            };
            return scaling;
        }
        let at = self.insert_index(interner, "scaling");
        self.content
            .insert(at, AxisContent::Scaling(Scaling::new(interner)));
        self.empty = false;
        let AxisContent::Scaling(scaling) = &mut self.content[at] else {
            unreachable!("the element inserted at `at` was a Scaling")
        };
        scaling
    }

    /// Whether the axis rules major gridlines across the plot area.
    #[must_use]
    pub fn has_major_gridlines(&self) -> bool {
        self.content
            .iter()
            .any(|item| matches!(item, AxisContent::MajorGridlines(_)))
    }

    /// Whether the axis rules minor gridlines across the plot area.
    #[must_use]
    pub fn has_minor_gridlines(&self) -> bool {
        self.content
            .iter()
            .any(|item| matches!(item, AxisContent::MinorGridlines(_)))
    }

    /// Turns the axis' major gridlines on or off. Turning them on adds an empty `c:majorGridlines`
    /// in its schema position; turning them off removes the element and the styling it carried.
    pub fn set_major_gridlines(&mut self, interner: &mut Interner, on: bool) {
        self.set_gridlines(interner, "majorGridlines", on);
    }

    /// Turns the axis' minor gridlines on or off. See
    /// [`set_major_gridlines`](Self::set_major_gridlines).
    pub fn set_minor_gridlines(&mut self, interner: &mut Interner, on: bool) {
        self.set_gridlines(interner, "minorGridlines", on);
    }

    /// The axis' title (`c:title`), or `None` when it has none.
    #[must_use]
    pub fn title(&self) -> Option<&ChartTitle> {
        self.content.iter().find_map(|item| match item {
            AxisContent::Title(title) => Some(title),
            _ => None,
        })
    }

    /// The axis' title text, or `None` when it has no title.
    #[must_use]
    pub fn title_text(&self) -> Option<String> {
        self.title().and_then(ChartTitle::text)
    }

    /// Sets the axis' title text, adding a `c:title` in its schema position if it had none. `None`
    /// removes the title entirely.
    pub fn set_title(&mut self, interner: &mut Interner, text: Option<&str>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, AxisContent::Title(_)));
        match (existing, text) {
            (Some(index), Some(text)) => {
                let AxisContent::Title(title) = &mut self.content[index] else {
                    unreachable!("the index was just found by matching this variant")
                };
                title.set_text(interner, text);
            }
            (Some(index), None) => {
                self.content.remove(index);
            }
            (None, Some(text)) => {
                let at = self.insert_index(interner, "title");
                self.content
                    .insert(at, AxisContent::Title(ChartTitle::new(interner, text)));
                self.empty = false;
            }
            (None, None) => {}
        }
    }

    /// Adds or removes one of the two gridline elements.
    fn set_gridlines(&mut self, interner: &mut Interner, local: &str, on: bool) {
        let is_target = |item: &AxisContent| {
            matches!(
                (local, item),
                ("majorGridlines", AxisContent::MajorGridlines(_))
                    | ("minorGridlines", AxisContent::MinorGridlines(_))
            )
        };
        let existing = self.content.iter().position(is_target);
        match (existing, on) {
            (Some(_), true) | (None, false) => {}
            (Some(index), false) => {
                self.content.remove(index);
            }
            (None, true) => {
                let at = self.insert_index(interner, local);
                let lines = Gridlines::new(interner, local);
                let item = if local == "majorGridlines" {
                    AxisContent::MajorGridlines(lines)
                } else {
                    AxisContent::MinorGridlines(lines)
                };
                self.content.insert(at, item);
                self.empty = false;
            }
        }
    }

    /// The generated child order of the complex type this axis element *is*.
    ///
    /// All four axis elements open with the same `EG_AxShared` group and differ only in the tail
    /// each adds, so placing a child by the wrong one would still put the shared members right — but
    /// naming the actual type is what makes a `c:crossBetween` on a value axis land correctly too.
    /// An element that is none of the four is placed by the category axis' order, which is the
    /// shared group alone.
    fn child_order(&self, interner: &Interner) -> &'static ChildOrder {
        match self.kind(interner) {
            Some(AxisKind::Value) => VALUE_AXIS,
            Some(AxisKind::Date) => DATE_AXIS,
            Some(AxisKind::Series) => SERIES_AXIS,
            Some(AxisKind::Category) | None => CATEGORY_AXIS,
        }
    }

    /// Where a child named `local` belongs among the axis' current children.
    fn insert_index(&self, interner: &Interner, local: &str) -> usize {
        insert_position(
            self.child_order(interner),
            self.content.iter().map(|item| match item {
                AxisContent::Scaling(_) => Some("scaling"),
                AxisContent::MajorGridlines(_) => Some("majorGridlines"),
                AxisContent::MinorGridlines(_) => Some("minorGridlines"),
                AxisContent::Title(_) => Some("title"),
                AxisContent::Raw(node) => chart_local(node, interner),
            }),
            local,
        )
    }

    /// The `@val` of a raw scalar child of the axis.
    fn scalar(&self, interner: &Interner, local: &str) -> Option<&str> {
        let raw = self.content.iter().filter_map(|item| match item {
            AxisContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, local, "val")
    }

    /// A raw scalar child's `@val` parsed as an unsigned integer.
    fn number(&self, interner: &Interner, local: &str) -> Option<u32> {
        self.scalar(interner, local)
            .and_then(|value| value.trim().parse().ok())
    }
}

/// Builds a minimal axis that renders: its id, `minMax` scaling, `c:delete="0"`, its position, and
/// the id of the axis it crosses. This is what an authored chart's axes start as.
pub(crate) fn build_axis(
    interner: &mut Interner,
    kind: AxisKind,
    axis_id: u32,
    cross_axis_id: u32,
    position: AxisPosition,
) -> mjx_ooxml_core::RawElement {
    let scaling = {
        let orientation = chart_val_leaf(interner, "orientation", "minMax");
        chart_element(
            interner,
            "scaling",
            Vec::new(),
            vec![RawNode::Element(orientation)],
        )
    };
    let children = vec![
        RawNode::Element(chart_val_leaf(interner, "axId", &axis_id.to_string())),
        RawNode::Element(scaling),
        RawNode::Element(chart_val_leaf(interner, "delete", "0")),
        RawNode::Element(chart_val_leaf(interner, "axPos", position.to_wire())),
        RawNode::Element(chart_val_leaf(
            interner,
            "crossAx",
            &cross_axis_id.to_string(),
        )),
    ];
    chart_element(interner, kind.element_local_name(), Vec::new(), children)
}

/// Builds a `c:numFmt formatCode="…" sourceLinked="0"` element — an axis' or a data label's number
/// format. `sourceLinked="0"` says the format is the one written here rather than the one the
/// workbook cell carries, which is what a caller who states a format means.
pub(crate) fn build_number_format(
    interner: &mut Interner,
    format_code: &str,
) -> mjx_ooxml_core::RawElement {
    let attributes = vec![
        chart_attr(interner, "formatCode", format_code),
        chart_attr(interner, "sourceLinked", "0"),
    ];
    chart_element(interner, "numFmt", attributes, Vec::new())
}

/// The local name of a chart-namespace element node, or `None` for anything else.
pub(crate) fn chart_local<'a>(node: &RawNode, interner: &'a Interner) -> Option<&'a str> {
    match node {
        RawNode::Element(element) if is_chart(&element.name, interner) => {
            Some(interner.resolve(element.name.local))
        }
        _ => None,
    }
}
