//! A chart's **decoration** — the data labels, the per-point formatting, the trendlines and the
//! error bars that hang off a plot and its series.
//!
//! The data half of a chart says what it draws; this module says what it *tells the reader*. Four
//! element families live here, and until this tier all four rode through the `Raw` bucket:
//! preserved perfectly, readable not at all.
//!
//! | Element | Type | What it is |
//! |---|---|---|
//! | `c:dLbls` | [`DataLabels`] | the label settings for a whole plot, or for one series |
//! | `c:dLbl` | [`DataLabel`] | one point's override of those settings |
//! | `c:dPt` | [`DataPointFormat`] | one point's own fill and outline — the slice coloured differently |
//! | `c:trendline` | [`Trendline`] | a fitted curve through a series |
//! | `c:errBars` | [`ErrorBars`] | the uncertainty drawn around each point |
//!
//! # The three tiers of a data label
//!
//! ECMA-376 §21.2.2.49 says `c:dLbls` "serves as a root element that specifies the settings for the
//! data labels for an entire series **or the entire chart**". Both containers are the same element,
//! and a `c:dLbl` inside a series' container overrides them for one point. So a label's settings are
//! resolved over **three tiers**, most specific first:
//!
//! 1. the point — `c:ser > c:dLbls > c:dLbl` whose `c:idx` names it;
//! 2. the series — `c:ser > c:dLbls`;
//! 3. the plot — `c:barChart > c:dLbls`, which the schema puts after the series.
//!
//! The merge is **per setting**, not per tier: a series that only says `c:showVal` still takes the
//! plot's `c:dLblPos`. [`DataLabelSettings::inherit`] is that merge, and
//! `resolved_data_labels` on each plot type is the walk that feeds it.
//!
//! There is deliberately no fourth tier: `c:chart` declares no `c:dLbls` of its own (see `CT_Chart`),
//! so "the entire chart" in the prose means the plot element — which is what a single-plot chart is.
//!
//! # Fidelity
//!
//! The same ordered-`content` + `Raw` shape as the rest of this crate. Only the children that need a
//! type get one — the `c:dLbl` list, a label's `c:tx`, an error bar's `c:plus`/`c:minus`, and every
//! `c:spPr` — and the scalars are read through accessors off the `Raw` bucket, so an untouched
//! element re-emits byte-for-byte. Every insertion is placed by the generated
//! [`child_order`](mjx_ooxml_types::child_order) tables: these types are sequence-dense (`CT_DLbls`
//! has fifteen ranked children) and a child in the wrong place is invalid, not merely untidy.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::child_order::{
    DATA_LABEL, DATA_LABELS, DATA_POINT_FORMAT, ERROR_BARS, TRENDLINE,
};
use mjx_ooxml_types::support::on_off;

use crate::author::ChartDataError;
use crate::axis::{build_number_format, chart_local, TitleText};
use crate::build::{
    attr_str, chart_name, chart_text_leaf, chart_val_leaf, element_text, f64_wire, insert_position,
    number_literal_source,
};
use crate::data::NumericData;
use crate::plot::SeriesShapeProperties;

// -------------------------------------------------------------------------------------------------
// The enumerations — every name sourced from the ECMA-376 Part 1 prose, never guessed
// -------------------------------------------------------------------------------------------------

/// Where a data label sits relative to the point it names (`c:dLblPos@val`, `ST_DLblPos`,
/// ECMA-376 Part 1 §21.2.3.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLabelPosition {
    /// "Displayed in the best position" — the application chooses (wire `bestFit`).
    BestFit,
    /// Below the data marker (wire `b`).
    Bottom,
    /// Centered on the data marker (wire `ctr`).
    Center,
    /// Inside the base of the data marker (wire `inBase`).
    InsideBase,
    /// Inside the end of the data marker (wire `inEnd`).
    InsideEnd,
    /// To the left of the data marker (wire `l`).
    Left,
    /// Outside the end of the data marker (wire `outEnd`).
    OutsideEnd,
    /// To the right of the data marker (wire `r`).
    Right,
    /// Above the data marker (wire `t`).
    Top,
}

impl DataLabelPosition {
    /// Maps the wire token to a position, or `None` for a token `ST_DLblPos` does not admit.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "bestFit" => Some(Self::BestFit),
            "b" => Some(Self::Bottom),
            "ctr" => Some(Self::Center),
            "inBase" => Some(Self::InsideBase),
            "inEnd" => Some(Self::InsideEnd),
            "l" => Some(Self::Left),
            "outEnd" => Some(Self::OutsideEnd),
            "r" => Some(Self::Right),
            "t" => Some(Self::Top),
            _ => None,
        }
    }

    /// The exact wire token for this position.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::BestFit => "bestFit",
            Self::Bottom => "b",
            Self::Center => "ctr",
            Self::InsideBase => "inBase",
            Self::InsideEnd => "inEnd",
            Self::Left => "l",
            Self::OutsideEnd => "outEnd",
            Self::Right => "r",
            Self::Top => "t",
        }
    }
}

/// The curve a trendline fits through its series (`c:trendlineType@val`, `ST_TrendlineType`,
/// ECMA-376 Part 1 §21.2.3.50).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendlineKind {
    /// An exponential curve, `y = ab^x` (wire `exp`).
    Exponential,
    /// A straight line, `y = mx + b` (wire `linear`).
    Linear,
    /// A logarithmic curve, `y = a log x + b`, with `log` the natural logarithm (wire `log`).
    Logarithmic,
    /// A moving average over [`Trendline::period`] points (wire `movingAvg`).
    MovingAverage,
    /// A polynomial curve of [`Trendline::order`] (wire `poly`).
    Polynomial,
    /// A power curve, `y = ax^b` (wire `power`).
    Power,
}

impl TrendlineKind {
    /// Maps the wire token to a trendline kind.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "exp" => Some(Self::Exponential),
            "linear" => Some(Self::Linear),
            "log" => Some(Self::Logarithmic),
            "movingAvg" => Some(Self::MovingAverage),
            "poly" => Some(Self::Polynomial),
            "power" => Some(Self::Power),
            _ => None,
        }
    }

    /// The exact wire token for this kind.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Exponential => "exp",
            Self::Linear => "linear",
            Self::Logarithmic => "log",
            Self::MovingAverage => "movingAvg",
            Self::Polynomial => "poly",
            Self::Power => "power",
        }
    }
}

/// Which of a point's two axes an error bar runs along (`c:errDir@val`, `ST_ErrDir`,
/// ECMA-376 Part 1 §21.2.3.13). The prose names the two values `X` and `Y`; there is no longer
/// spelling to expand them to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorBarDirection {
    /// Error bars shown in the x direction (wire `x`).
    X,
    /// Error bars shown in the y direction (wire `y`).
    Y,
}

impl ErrorBarDirection {
    /// Maps the wire token to a direction.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            _ => None,
        }
    }

    /// The exact wire token for this direction.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

/// Which side(s) of a point an error bar is drawn on (`c:errBarType@val`, `ST_ErrBarType`,
/// ECMA-376 Part 1 §21.2.3.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorBarType {
    /// Both the positive and the negative direction (wire `both`).
    Both,
    /// The negative direction only (wire `minus`).
    Minus,
    /// The positive direction only (wire `plus`).
    Plus,
}

impl ErrorBarType {
    /// Maps the wire token to a bar type.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "both" => Some(Self::Both),
            "minus" => Some(Self::Minus),
            "plus" => Some(Self::Plus),
            _ => None,
        }
    }

    /// The exact wire token for this bar type.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Minus => "minus",
            Self::Plus => "plus",
        }
    }
}

/// How an error bar's length is arrived at (`c:errValType@val`, `ST_ErrValType`,
/// ECMA-376 Part 1 §21.2.3.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorValueType {
    /// The length is given per point by `c:plus` and `c:minus` (wire `cust`).
    Custom,
    /// The length is the fixed [`ErrorBars::value`] (wire `fixedVal`).
    FixedValue,
    /// The length is [`ErrorBars::value`] percent of the data (wire `percentage`).
    Percentage,
    /// The length is [`ErrorBars::value`] standard deviations of the data (wire `stdDev`).
    StandardDeviation,
    /// The length is [`ErrorBars::value`] standard errors of the data (wire `stdErr`).
    StandardError,
}

impl ErrorValueType {
    /// Maps the wire token to a value type.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "cust" => Some(Self::Custom),
            "fixedVal" => Some(Self::FixedValue),
            "percentage" => Some(Self::Percentage),
            "stdDev" => Some(Self::StandardDeviation),
            "stdErr" => Some(Self::StandardError),
            _ => None,
        }
    }

    /// The exact wire token for this value type.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Custom => "cust",
            Self::FixedValue => "fixedVal",
            Self::Percentage => "percentage",
            Self::StandardDeviation => "stdDev",
            Self::StandardError => "stdErr",
        }
    }
}

// -------------------------------------------------------------------------------------------------
// The shared raw-child access every decoration element needs
// -------------------------------------------------------------------------------------------------

/// Generates the raw-child readers and writers a decoration element needs over its
/// `content: Vec<$content>` field, placing every insertion by `$order`.
///
/// The five elements here differ only in which children they promote to a typed variant; their
/// scalars all live in the `Raw` bucket and are reached the same way. Writing this once is what
/// keeps `c:showVal`, `c:trendlineType` and `c:errBarType` from each growing their own placement
/// code — and what keeps that placement coming from the generated table rather than from a
/// hand-written rank list.
///
/// `$typed` maps each typed variant to the local name it stands for, so placement sees the whole
/// element, not only its raw half.
macro_rules! raw_child_access {
    ($ty:ty, $content:ident, $order:expr, [$($variant:ident),* $(,)?], [$($local:literal),* $(,)?]) => {
        // This is a shared vocabulary, not a per-type API: it gives every decoration element the
        // same way of reaching its raw children, and each element then uses the words its own
        // schema type has. `c:dPt` has no text-bearing child, `c:errBars` has no unsigned one, and
        // `c:dLbls` places its `c:dLbl` run itself — so some words go unused in some expansions.
        // Generating a subset per type would mean five near-identical macros to keep in step.
        #[allow(dead_code)]
        impl $ty {
            /// Each child's local name in document order, or `None` for a node the schema does not
            /// name — what [`insert_position`] needs to place a new child.
            fn content_locals<'a>(
                &'a self,
                interner: &'a Interner,
            ) -> impl Iterator<Item = Option<&'a str>> {
                self.content.iter().map(move |item| match item {
                    $($content::$variant(_) => Some($local),)*
                    $content::Raw(node) => chart_local(node, interner),
                })
            }

            /// The index of the raw child named `local`, if the element carries one.
            fn raw_index(&self, interner: &Interner, local: &str) -> Option<usize> {
                self.content.iter().position(|item| match item {
                    $content::Raw(node) => chart_local(node, interner) == Some(local),
                    _ => false,
                })
            }

            /// The raw child named `local`, as an element.
            fn raw_element(
                &self,
                interner: &Interner,
                local: &str,
            ) -> Option<&::mjx_ooxml_core::RawElement> {
                let index = self.raw_index(interner, local)?;
                match &self.content[index] {
                    $content::Raw(RawNode::Element(element)) => Some(element),
                    _ => None,
                }
            }

            /// The `@val` of the raw scalar child named `local`.
            fn scalar(&self, interner: &Interner, local: &str) -> Option<&str> {
                attr_str(&self.raw_element(interner, local)?.attributes, interner, "val")
            }

            /// A raw scalar child's `@val` read as an `ST_OnOff`-family boolean. `CT_Boolean`
            /// defaults `@val` to `true`, so a bare `<c:showVal/>` reads as `Some(true)`.
            fn flag(&self, interner: &Interner, local: &str) -> Option<bool> {
                let element = self.raw_element(interner, local)?;
                match attr_str(&element.attributes, interner, "val") {
                    Some(value) => on_off::from_wire(value),
                    None => Some(true),
                }
            }

            /// A raw scalar child's `@val` parsed as a finite number.
            fn number(&self, interner: &Interner, local: &str) -> Option<f64> {
                self.scalar(interner, local)
                    .and_then(|value| value.trim().parse::<f64>().ok())
                    .filter(|value| value.is_finite())
            }

            /// A raw scalar child's `@val` parsed as an unsigned integer.
            fn unsigned(&self, interner: &Interner, local: &str) -> Option<u32> {
                self.scalar(interner, local)
                    .and_then(|value| value.trim().parse().ok())
            }

            /// The decoded text of a text-bearing raw child (`c:separator`, `c:name`).
            fn text_child(&self, interner: &Interner, local: &str) -> Option<String> {
                self.raw_element(interner, local)
                    .map(|element| element_text(&element.children))
            }

            /// Replaces the raw child named `local` in place, or inserts it at its schema rank.
            fn put_raw(&mut self, interner: &mut Interner, element: ::mjx_ooxml_core::RawElement) {
                let local = interner.resolve(element.name.local).to_owned();
                if let Some(index) = self.raw_index(interner, &local) {
                    self.content[index] = $content::Raw(RawNode::Element(element));
                    return;
                }
                let at = insert_position($order, self.content_locals(interner), &local);
                self.content
                    .insert(at, $content::Raw(RawNode::Element(element)));
                self.empty = false;
            }

            /// Removes the raw child named `local`, answering whether one was there.
            fn drop_raw(&mut self, interner: &Interner, local: &str) -> bool {
                match self.raw_index(interner, local) {
                    Some(index) => {
                        self.content.remove(index);
                        true
                    }
                    None => false,
                }
            }

            /// Sets, or (for `None`) removes, a `<c:local val="…"/>` scalar child.
            fn set_scalar(&mut self, interner: &mut Interner, local: &str, value: Option<&str>) {
                match value {
                    Some(value) => {
                        let element = chart_val_leaf(interner, local, value);
                        self.put_raw(interner, element);
                    }
                    None => {
                        self.drop_raw(interner, local);
                    }
                }
            }

            /// Sets, or removes, a `CT_Boolean` child, written in the canonical `0`/`1` spelling.
            fn set_flag(&mut self, interner: &mut Interner, local: &str, value: Option<bool>) {
                self.set_scalar(interner, local, value.map(on_off::to_wire));
            }

            /// Sets, or removes, a text-bearing child (`c:separator`, `c:name`).
            fn set_text_child(&mut self, interner: &mut Interner, local: &str, text: Option<&str>) {
                match text {
                    Some(text) => {
                        let element = chart_text_leaf(interner, local, text);
                        self.put_raw(interner, element);
                    }
                    None => {
                        self.drop_raw(interner, local);
                    }
                }
            }

            /// Where a child named `local` belongs among this element's current children.
            fn insert_index(&self, interner: &Interner, local: &str) -> usize {
                insert_position($order, self.content_locals(interner), local)
            }
        }
    };
}

/// The children `EG_DLblShared` and its two wrappers declare as settings — everything a `c:dLbls`
/// or `c:dLbl` carries other than its identity (`c:idx`), its layout, its own text and its
/// extensions. Turning a label's settings *on* has to clear a `c:delete` that stands in their place
/// (`CT_DLbls` and `CT_DLbl` both put the two in one `xsd:choice`), and suppressing a label has to
/// clear all of them.
const LABEL_SETTING_LOCALS: [&str; 11] = [
    "numFmt",
    "spPr",
    "txPr",
    "dLblPos",
    "showLegendKey",
    "showVal",
    "showCatName",
    "showSerName",
    "showPercent",
    "showBubbleSize",
    "separator",
];

/// The two children only the *container* form (`Group_DLbls`) admits — a per-point `c:dLbl` has
/// neither, because leader lines are drawn for a whole series' labels, not for one of them.
const CONTAINER_ONLY_LOCALS: [&str; 2] = ["showLeaderLines", "leaderLines"];

// -------------------------------------------------------------------------------------------------
// The resolved settings and the specs a caller writes
// -------------------------------------------------------------------------------------------------

/// What a data label shows, where it sits and how it is punctuated — the settings of `EG_DLblShared`
/// read back as one value.
///
/// Every field is `Option`: `None` means *this tier says nothing*, which is what makes
/// [`inherit`](Self::inherit) meaningful. A resolved set whose fields are still `None` is one the
/// application fills in from the chart style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DataLabelSettings {
    /// The label is suppressed entirely (`c:delete`). When `Some(true)` nothing else applies.
    pub suppressed: Option<bool>,
    /// The point's value is shown (`c:showVal`).
    pub shows_value: Option<bool>,
    /// The point's category label is shown (`c:showCatName`).
    pub shows_category_name: Option<bool>,
    /// The series' name is shown (`c:showSerName`).
    pub shows_series_name: Option<bool>,
    /// The point's share of the total is shown (`c:showPercent`).
    pub shows_percentage: Option<bool>,
    /// A bubble plot's third value is shown (`c:showBubbleSize`).
    pub shows_bubble_size: Option<bool>,
    /// The series' legend swatch is drawn beside the label (`c:showLegendKey`).
    pub shows_legend_key: Option<bool>,
    /// Lines are drawn from a moved label back to its point (`c:showLeaderLines`). Only a container
    /// tier declares this; a per-point `c:dLbl` never does.
    pub shows_leader_lines: Option<bool>,
    /// Where the label sits relative to its point (`c:dLblPos`).
    pub position: Option<DataLabelPosition>,
    /// What separates the parts of a multi-part label (`c:separator`), e.g. `"; "`.
    pub separator: Option<String>,
    /// The number format the value is written in (`c:numFmt@formatCode`), e.g. `"0.0%"`.
    pub number_format: Option<String>,
}

impl DataLabelSettings {
    /// Whether this tier says nothing at all — every setting unset.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// Merges this (more specific) tier over `parent`, **per setting**: a field this tier leaves
    /// unset takes `parent`'s.
    ///
    /// A `c:delete` short-circuits: a tier that suppresses its label inherits nothing, because
    /// `CT_DLbls`/`CT_DLbl` put `c:delete` and the settings group in one `xsd:choice` and an element
    /// carrying one cannot carry the other.
    #[must_use]
    pub fn inherit(&self, parent: &Self) -> Self {
        if self.suppressed == Some(true) {
            return self.clone();
        }
        Self {
            suppressed: self.suppressed.or(parent.suppressed),
            shows_value: self.shows_value.or(parent.shows_value),
            shows_category_name: self.shows_category_name.or(parent.shows_category_name),
            shows_series_name: self.shows_series_name.or(parent.shows_series_name),
            shows_percentage: self.shows_percentage.or(parent.shows_percentage),
            shows_bubble_size: self.shows_bubble_size.or(parent.shows_bubble_size),
            shows_legend_key: self.shows_legend_key.or(parent.shows_legend_key),
            shows_leader_lines: self.shows_leader_lines.or(parent.shows_leader_lines),
            position: self.position.or(parent.position),
            separator: self.separator.clone().or_else(|| parent.separator.clone()),
            number_format: self
                .number_format
                .clone()
                .or_else(|| parent.number_format.clone()),
        }
    }
}

/// A description of the data labels to write — the settings a caller states, and only those.
///
/// A `None` field is **left alone**: writing a spec whose only `Some` is `shows_value` turns the value
/// on and touches nothing else, so a caller can change one setting of a label Office wrote without
/// flattening the rest. To clear a setting, state it as `Some(false)`; to remove the element
/// entirely, use the tier's `remove` call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataLabelSpec {
    /// Show the point's value (`c:showVal`).
    pub shows_value: Option<bool>,
    /// Show the point's category label (`c:showCatName`).
    pub shows_category_name: Option<bool>,
    /// Show the series' name (`c:showSerName`).
    pub shows_series_name: Option<bool>,
    /// Show the point's share of the total (`c:showPercent`).
    pub shows_percentage: Option<bool>,
    /// Show a bubble plot's third value (`c:showBubbleSize`).
    pub shows_bubble_size: Option<bool>,
    /// Draw the series' legend swatch beside the label (`c:showLegendKey`).
    pub shows_legend_key: Option<bool>,
    /// Draw leader lines back to the point (`c:showLeaderLines`). **Container tiers only** — asking
    /// for it on one point's label is [`ChartDataError::SettingNotAtThisTier`].
    pub shows_leader_lines: Option<bool>,
    /// Where the label sits (`c:dLblPos`).
    pub position: Option<DataLabelPosition>,
    /// What separates the parts of a multi-part label (`c:separator`).
    pub separator: Option<String>,
    /// The number format the value is written in (`c:numFmt@formatCode`).
    pub number_format: Option<String>,
}

impl DataLabelSpec {
    /// A spec that shows nothing — the starting point for a fluent build.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows (or hides) the point's value.
    #[must_use]
    pub fn value(mut self, show: bool) -> Self {
        self.shows_value = Some(show);
        self
    }

    /// Shows (or hides) the point's category label.
    #[must_use]
    pub fn category_name(mut self, show: bool) -> Self {
        self.shows_category_name = Some(show);
        self
    }

    /// Shows (or hides) the series' name.
    #[must_use]
    pub fn series_name(mut self, show: bool) -> Self {
        self.shows_series_name = Some(show);
        self
    }

    /// Shows (or hides) the point's share of the total.
    #[must_use]
    pub fn percentage(mut self, show: bool) -> Self {
        self.shows_percentage = Some(show);
        self
    }

    /// Shows (or hides) a bubble plot's third value.
    #[must_use]
    pub fn bubble_size(mut self, show: bool) -> Self {
        self.shows_bubble_size = Some(show);
        self
    }

    /// Shows (or hides) the series' legend swatch beside the label.
    #[must_use]
    pub fn legend_key(mut self, show: bool) -> Self {
        self.shows_legend_key = Some(show);
        self
    }

    /// Draws (or stops drawing) leader lines back to the point. Container tiers only.
    #[must_use]
    pub fn leader_lines(mut self, show: bool) -> Self {
        self.shows_leader_lines = Some(show);
        self
    }

    /// Places the label relative to its point.
    #[must_use]
    pub fn position(mut self, position: DataLabelPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets what separates the parts of a multi-part label.
    #[must_use]
    pub fn separator<S: Into<String>>(mut self, separator: S) -> Self {
        self.separator = Some(separator.into());
        self
    }

    /// Sets the number format the value is written in.
    #[must_use]
    pub fn number_format<S: Into<String>>(mut self, format_code: S) -> Self {
        self.number_format = Some(format_code.into());
        self
    }

    /// Whether the spec states anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// A description of a trendline to add to a series (`c:trendline`, `CT_Trendline`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrendlineSpec {
    /// The curve to fit (`c:trendlineType`) — the one setting the schema requires.
    pub kind: TrendlineKind,
    /// A name for the trendline, shown in the legend (`c:name`). `None` lets the application derive
    /// one from [`kind`](Self::kind).
    pub name: Option<String>,
    /// The order of a [polynomial](TrendlineKind::Polynomial) curve (`c:order`). `ST_Order` admits
    /// 2 to 6; the schema's default is 2.
    pub polynomial_order: Option<u8>,
    /// The window of a [moving average](TrendlineKind::MovingAverage) (`c:period`). `ST_Period`
    /// admits 2 upwards; the schema's default is 2.
    pub moving_average_period: Option<u32>,
    /// How far past the last point the curve is extended, in categories (`c:forward`).
    pub forward_periods: Option<f64>,
    /// How far before the first point the curve is extended, in categories (`c:backward`).
    pub backward_periods: Option<f64>,
    /// The y value the curve is forced through (`c:intercept`).
    pub intercept: Option<f64>,
    /// Whether the curve's equation is drawn on the chart (`c:dispEq`).
    pub displays_equation: Option<bool>,
    /// Whether the curve's R² is drawn on the chart (`c:dispRSqr`).
    pub displays_r_squared: Option<bool>,
}

impl TrendlineSpec {
    /// A trendline of `kind` with every optional setting left to the application.
    #[must_use]
    pub fn new(kind: TrendlineKind) -> Self {
        Self {
            kind,
            name: None,
            polynomial_order: None,
            moving_average_period: None,
            forward_periods: None,
            backward_periods: None,
            intercept: None,
            displays_equation: None,
            displays_r_squared: None,
        }
    }

    /// Names the trendline.
    #[must_use]
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets a polynomial curve's order (2 to 6).
    #[must_use]
    pub fn polynomial_order(mut self, order: u8) -> Self {
        self.polynomial_order = Some(order);
        self
    }

    /// Sets a moving average's window (2 upwards).
    #[must_use]
    pub fn moving_average_period(mut self, period: u32) -> Self {
        self.moving_average_period = Some(period);
        self
    }

    /// Extends the curve `forward` categories past the last point and `backward` before the first.
    #[must_use]
    pub fn projection(mut self, forward: f64, backward: f64) -> Self {
        self.forward_periods = Some(forward);
        self.backward_periods = Some(backward);
        self
    }

    /// Forces the curve through `intercept` on the value axis.
    #[must_use]
    pub fn intercept(mut self, intercept: f64) -> Self {
        self.intercept = Some(intercept);
        self
    }

    /// Draws the curve's equation, its R², or both, on the chart.
    #[must_use]
    pub fn display(mut self, equation: bool, r_squared: bool) -> Self {
        self.displays_equation = Some(equation);
        self.displays_r_squared = Some(r_squared);
        self
    }

    /// Checks the spec against `ST_Order` and `ST_Period` before anything is written.
    ///
    /// # Errors
    /// [`ChartDataError::TrendlineOrderOutOfRange`] or
    /// [`ChartDataError::TrendlinePeriodOutOfRange`] when a stated order or period is outside the
    /// range its simple type admits, and [`ChartDataError::NonFiniteMeasure`] for a projection or
    /// intercept that has no XML spelling.
    pub fn validate(&self) -> Result<(), ChartDataError> {
        if let Some(order) = self.polynomial_order {
            if !(2..=6).contains(&order) {
                return Err(ChartDataError::TrendlineOrderOutOfRange { order });
            }
        }
        if let Some(period) = self.moving_average_period {
            if period < 2 {
                return Err(ChartDataError::TrendlinePeriodOutOfRange { period });
            }
        }
        for (name, value) in [
            ("forward", self.forward_periods),
            ("backward", self.backward_periods),
            ("intercept", self.intercept),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(ChartDataError::NonFiniteMeasure { element: name });
            }
        }
        Ok(())
    }
}

/// A description of the error bars to give a series (`c:errBars`, `CT_ErrBars`).
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorBarSpec {
    /// Which axis the bars run along (`c:errDir`). `None` leaves it to the application, which is
    /// what Office writes for a plain category/value chart.
    pub direction: Option<ErrorBarDirection>,
    /// Which side(s) of the point the bars are drawn on (`c:errBarType`) — required by the schema.
    pub bar_type: ErrorBarType,
    /// How the bars' length is arrived at (`c:errValType`) — required by the schema.
    pub value_type: ErrorValueType,
    /// Whether the bars are drawn without their end caps (`c:noEndCap`).
    pub no_end_cap: Option<bool>,
    /// The single length every bar takes, read as [`value_type`](Self::value_type) says (`c:val`).
    pub value: Option<f64>,
    /// Per-point lengths in the positive direction (`c:plus`), written as a `c:numLit`. Only a
    /// [`Custom`](ErrorValueType::Custom) bar reads these.
    pub plus_values: Option<Vec<f64>>,
    /// Per-point lengths in the negative direction (`c:minus`).
    pub minus_values: Option<Vec<f64>>,
}

impl ErrorBarSpec {
    /// Bars of `bar_type` whose length is `value`, read as `value_type` says.
    #[must_use]
    pub fn fixed(bar_type: ErrorBarType, value_type: ErrorValueType, value: f64) -> Self {
        Self {
            direction: None,
            bar_type,
            value_type,
            no_end_cap: None,
            value: Some(value),
            plus_values: None,
            minus_values: None,
        }
    }

    /// Bars of `bar_type` whose length is stated per point — the
    /// [`Custom`](ErrorValueType::Custom) form, written as `c:plus` and `c:minus` literals.
    #[must_use]
    pub fn custom(bar_type: ErrorBarType, plus_values: Vec<f64>, minus_values: Vec<f64>) -> Self {
        Self {
            direction: None,
            bar_type,
            value_type: ErrorValueType::Custom,
            no_end_cap: None,
            value: None,
            plus_values: Some(plus_values),
            minus_values: Some(minus_values),
        }
    }

    /// Runs the bars along `direction`.
    #[must_use]
    pub fn direction(mut self, direction: ErrorBarDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Draws the bars without their end caps.
    #[must_use]
    pub fn no_end_cap(mut self, no_end_cap: bool) -> Self {
        self.no_end_cap = Some(no_end_cap);
        self
    }

    /// Checks the spec before anything is written.
    ///
    /// # Errors
    /// [`ChartDataError::CustomErrorBarsNeedValues`] when the value type is
    /// [`Custom`](ErrorValueType::Custom) and neither `c:plus` nor `c:minus` is given — the schema
    /// admits that shape, but it describes bars whose length nothing determines — and
    /// [`ChartDataError::NonFiniteMeasure`] for a value with no XML spelling.
    pub fn validate(&self) -> Result<(), ChartDataError> {
        if self.value_type == ErrorValueType::Custom
            && self.plus_values.is_none()
            && self.minus_values.is_none()
        {
            return Err(ChartDataError::CustomErrorBarsNeedValues);
        }
        if self.value.is_some_and(|value| !value.is_finite()) {
            return Err(ChartDataError::NonFiniteMeasure { element: "val" });
        }
        for (name, values) in [("plus", &self.plus_values), ("minus", &self.minus_values)] {
            if values
                .as_ref()
                .is_some_and(|values| values.iter().any(|value| !value.is_finite()))
            {
                return Err(ChartDataError::NonFiniteMeasure { element: name });
            }
        }
        Ok(())
    }
}

/// Writes the `EG_DLblShared` settings `$spec` states onto `$element`, in the group's own sequence
/// order.
///
/// A macro rather than a function taking closures: writing needs `&mut self` *and* `&mut Interner`
/// for every setting, which no set of closures can hold at once. Both `c:dLbls` and `c:dLbl` carry
/// exactly this group, so this is written once and expanded twice.
macro_rules! write_label_settings {
    ($element:expr, $interner:expr, $spec:expr) => {{
        let element = &mut *$element;
        let interner = &mut *$interner;
        let spec: &DataLabelSpec = $spec;
        if let Some(format_code) = &spec.number_format {
            let number_format = build_number_format(interner, format_code);
            element.put_raw(interner, number_format);
        }
        if let Some(position) = spec.position {
            element.set_scalar(interner, "dLblPos", Some(position.to_wire()));
        }
        for (local, value) in [
            ("showLegendKey", spec.shows_legend_key),
            ("showVal", spec.shows_value),
            ("showCatName", spec.shows_category_name),
            ("showSerName", spec.shows_series_name),
            ("showPercent", spec.shows_percentage),
            ("showBubbleSize", spec.shows_bubble_size),
        ] {
            if let Some(value) = value {
                element.set_flag(interner, local, Some(value));
            }
        }
        if let Some(separator) = &spec.separator {
            element.set_text_child(interner, "separator", Some(separator.as_str()));
        }
    }};
}

/// A piece of per-point decoration whose `c:idx` names a point its series no longer has.
///
/// A `c:dPt` or `c:dLbl` is anchored by index into the series, so an edit that shortens the series
/// can leave one addressing past the end. This crate never renumbers such an element and never
/// silently drops it — it reports it, and removes it only when a caller asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DanglingPointReference {
    /// The element that dangles, without its `c:` prefix — `dPt` or `dLbl`.
    pub element: &'static str,
    /// The 0-based point index it names.
    pub index: u32,
}

// -------------------------------------------------------------------------------------------------
// c:dLbl — one point's override
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`DataLabel`]: its own text, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLabelContent {
    /// The label's own words, replacing the value it would otherwise show (`c:tx`).
    Text(TitleText),
    /// Any other child — `c:idx`, `c:delete`, `c:layout`, the settings group, `c:extLst` — kept
    /// verbatim.
    Raw(RawNode),
}

/// `c:dLbl` (`CT_DLbl`) — one point's override of its series' data-label settings.
///
/// It is anchored by `c:idx`, the point's 0-based position in the series, **not** by its position in
/// the `c:dLbls` list: [`index`](Self::index) is what identifies it, and nothing in this crate
/// renumbers it.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct DataLabel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tx", variant = Text, ty = TitleText))]
    content: Vec<DataLabelContent>,
}

raw_child_access!(DataLabel, DataLabelContent, DATA_LABEL, [Text], ["tx"]);

impl DataLabel {
    /// A fresh `c:dLbl` for the point at `index`, carrying `spec`'s settings.
    fn new(interner: &mut Interner, index: u32, spec: &DataLabelSpec) -> Self {
        let idx = chart_val_leaf(interner, "idx", &index.to_string());
        let mut label = Self {
            name: chart_name(interner, "dLbl"),
            attributes: Vec::new(),
            empty: false,
            content: vec![DataLabelContent::Raw(RawNode::Element(idx))],
        };
        label.apply(interner, spec);
        label
    }

    /// A fresh `c:dLbl` that suppresses the point's label entirely
    /// (`<c:dLbl><c:idx val="n"/><c:delete val="1"/></c:dLbl>`).
    fn suppressed(interner: &mut Interner, index: u32) -> Self {
        let idx = chart_val_leaf(interner, "idx", &index.to_string());
        let delete = chart_val_leaf(interner, "delete", "1");
        Self {
            name: chart_name(interner, "dLbl"),
            attributes: Vec::new(),
            empty: false,
            content: vec![
                DataLabelContent::Raw(RawNode::Element(idx)),
                DataLabelContent::Raw(RawNode::Element(delete)),
            ],
        }
    }

    /// The 0-based index of the point this label belongs to (`c:idx@val`).
    ///
    /// `None` for a label whose `c:idx` is absent or unparsable — which the schema forbids, so it
    /// only happens in a malformed file. Such a label is never matched by a point lookup and never
    /// renumbered; it rides through a round-trip untouched.
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<u32> {
        self.unsigned(interner, "idx")
    }

    /// The label's own words (`c:tx`), which replace the value it would otherwise show, or `None`
    /// when it declares none.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.content.iter().find_map(|item| match item {
            DataLabelContent::Text(text) => text.text(),
            DataLabelContent::Raw(_) => None,
        })
    }

    /// Replaces the label's own words, adding a `c:tx` in its schema position if it had none.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        self.clear_suppression(interner);
        let replacement = TitleText::new(interner, text);
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, DataLabelContent::Text(_)))
        {
            self.content[index] = DataLabelContent::Text(replacement);
            return;
        }
        let at = self.insert_index(interner, "tx");
        self.content.insert(at, DataLabelContent::Text(replacement));
        self.empty = false;
    }

    /// Whether this point's label is suppressed (`c:delete`).
    #[must_use]
    pub fn is_suppressed(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "delete")
    }

    /// The settings this label states in its own right — the point tier of the merge.
    #[must_use]
    pub fn settings(&self, interner: &Interner) -> DataLabelSettings {
        read_label_settings(
            |local| self.flag(interner, local),
            || {
                self.scalar(interner, "dLblPos")
                    .and_then(DataLabelPosition::from_wire)
            },
            || self.text_child(interner, "separator"),
            || {
                self.raw_element(interner, "numFmt")
                    .and_then(|element| attr_str(&element.attributes, interner, "formatCode"))
                    .map(str::to_owned)
            },
        )
    }

    /// Applies `spec` to this label, leaving every setting it does not state alone.
    ///
    /// # Errors
    /// [`ChartDataError::SettingNotAtThisTier`] for `shows_leader_lines`, which `Group_DLbl` does
    /// not declare.
    pub fn apply_spec(
        &mut self,
        interner: &mut Interner,
        spec: &DataLabelSpec,
    ) -> Result<(), ChartDataError> {
        if spec.shows_leader_lines.is_some() {
            return Err(ChartDataError::SettingNotAtThisTier {
                element: "showLeaderLines",
                parent: "dLbl",
            });
        }
        self.apply(interner, spec);
        Ok(())
    }

    /// Writes `spec`'s stated settings, clearing any `c:delete` that stood in their place.
    fn apply(&mut self, interner: &mut Interner, spec: &DataLabelSpec) {
        if spec.is_empty() {
            return;
        }
        self.clear_suppression(interner);
        write_label_settings!(self, interner, spec);
    }

    /// Removes a `c:delete` — `CT_DLbl` puts it in one `xsd:choice` with the settings group, so a
    /// label cannot both be suppressed and say anything.
    fn clear_suppression(&mut self, interner: &mut Interner) {
        self.drop_raw(interner, "delete");
    }
}

// -------------------------------------------------------------------------------------------------
// c:dLbls — a plot's or a series' label settings
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`DataLabels`]: one point's override, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLabelsContent {
    /// One point's override (`c:dLbl`).
    Label(DataLabel),
    /// Any other child — `c:delete`, the settings group, `c:showLeaderLines`, `c:leaderLines`,
    /// `c:extLst` — kept verbatim.
    Raw(RawNode),
}

/// `c:dLbls` (`CT_DLbls`) — the data-label settings for a whole plot, or for one series.
///
/// The same element serves both tiers (ECMA-376 Part 1 §21.2.2.49: "the settings for the data labels
/// for an entire series or the entire chart"); which tier it is depends on where it hangs, and
/// [`DataLabelSettings::inherit`] is how the two combine.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct DataLabels {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "dLbl", variant = Label, ty = DataLabel))]
    content: Vec<DataLabelsContent>,
}

raw_child_access!(
    DataLabels,
    DataLabelsContent,
    DATA_LABELS,
    [Label],
    ["dLbl"]
);

impl DataLabels {
    /// A fresh `c:dLbls` carrying `spec`'s settings.
    pub(crate) fn new(interner: &mut Interner, spec: &DataLabelSpec) -> Self {
        let mut labels = Self {
            name: chart_name(interner, "dLbls"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        labels.apply(interner, spec);
        labels
    }

    /// The settings this element states in its own right, ignoring the tiers above and below it.
    #[must_use]
    pub fn settings(&self, interner: &Interner) -> DataLabelSettings {
        let mut settings = read_label_settings(
            |local| self.flag(interner, local),
            || {
                self.scalar(interner, "dLblPos")
                    .and_then(DataLabelPosition::from_wire)
            },
            || self.text_child(interner, "separator"),
            || {
                self.raw_element(interner, "numFmt")
                    .and_then(|element| attr_str(&element.attributes, interner, "formatCode"))
                    .map(str::to_owned)
            },
        );
        settings.shows_leader_lines = self.flag(interner, "showLeaderLines");
        settings
    }

    /// Whether every label under this element is suppressed (`c:delete`).
    #[must_use]
    pub fn is_suppressed(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "delete")
    }

    /// Every per-point override this element carries, in document order.
    pub fn labels(&self) -> impl Iterator<Item = &DataLabel> {
        self.content.iter().filter_map(|item| match item {
            DataLabelsContent::Label(label) => Some(label),
            DataLabelsContent::Raw(_) => None,
        })
    }

    /// The override for the point at `index`, matched on its `c:idx` — never on its position in the
    /// list. `None` when no override names that point.
    #[must_use]
    pub fn label_for_point(&self, interner: &Interner, index: u32) -> Option<&DataLabel> {
        self.labels()
            .find(|label| label.index(interner) == Some(index))
    }

    /// The override for the point at `index`, mutably.
    pub fn label_for_point_mut(
        &mut self,
        interner: &Interner,
        index: u32,
    ) -> Option<&mut DataLabel> {
        let at = self.label_index_of(interner, index)?;
        match &mut self.content[at] {
            DataLabelsContent::Label(label) => Some(label),
            DataLabelsContent::Raw(_) => None,
        }
    }

    /// Sets the override for the point at `index`, creating one at its schema position if there is
    /// none. Existing overrides are matched by `c:idx` and rewritten in place.
    ///
    /// # Errors
    /// [`ChartDataError::SettingNotAtThisTier`] when `spec` asks for leader lines, which one point's
    /// label cannot declare.
    pub fn set_label_for_point(
        &mut self,
        interner: &mut Interner,
        index: u32,
        spec: &DataLabelSpec,
    ) -> Result<(), ChartDataError> {
        if spec.shows_leader_lines.is_some() {
            return Err(ChartDataError::SettingNotAtThisTier {
                element: "showLeaderLines",
                parent: "dLbl",
            });
        }
        if let Some(at) = self.label_index_of(interner, index) {
            if let DataLabelsContent::Label(label) = &mut self.content[at] {
                label.apply(interner, spec);
            }
            return Ok(());
        }
        let label = DataLabel::new(interner, index, spec);
        self.insert_label(interner, label);
        Ok(())
    }

    /// Suppresses the label of the point at `index` — a `c:dLbl` carrying only `c:delete`, which is
    /// how Office hides one label of a series that shows the rest.
    pub fn suppress_label_for_point(&mut self, interner: &mut Interner, index: u32) {
        if let Some(at) = self.label_index_of(interner, index) {
            let label = DataLabel::suppressed(interner, index);
            self.content[at] = DataLabelsContent::Label(label);
            return;
        }
        let label = DataLabel::suppressed(interner, index);
        self.insert_label(interner, label);
    }

    /// Removes the override for the point at `index`, answering whether one was there. The point
    /// falls back to its series' settings.
    pub fn remove_label_for_point(&mut self, interner: &Interner, index: u32) -> bool {
        match self.label_index_of(interner, index) {
            Some(at) => {
                self.content.remove(at);
                true
            }
            None => false,
        }
    }

    /// Removes every per-point override whose `c:idx` is at or past `point_count`, answering how
    /// many went. A label whose `c:idx` does not parse is left alone — it addresses no point, and
    /// guessing at it would be inventing data.
    pub(crate) fn drop_labels_beyond(&mut self, interner: &Interner, point_count: usize) -> usize {
        let before = self.content.len();
        let limit = u32::try_from(point_count).unwrap_or(u32::MAX);
        self.content.retain(|item| match item {
            DataLabelsContent::Label(label) => {
                label.index(interner).is_none_or(|index| index < limit)
            }
            DataLabelsContent::Raw(_) => true,
        });
        before - self.content.len()
    }

    /// The `c:idx` of every per-point override that names a point at or past `point_count`.
    pub(crate) fn labels_beyond(&self, interner: &Interner, point_count: usize) -> Vec<u32> {
        let limit = u32::try_from(point_count).unwrap_or(u32::MAX);
        self.labels()
            .filter_map(|label| label.index(interner))
            .filter(|index| *index >= limit)
            .collect()
    }

    /// Applies `spec`'s stated settings, clearing any `c:delete` that stood in their place.
    pub(crate) fn apply(&mut self, interner: &mut Interner, spec: &DataLabelSpec) {
        if spec.is_empty() {
            return;
        }
        self.clear_suppression(interner);
        write_label_settings!(self, interner, spec);
        if let Some(show) = spec.shows_leader_lines {
            self.set_flag(interner, "showLeaderLines", Some(show));
        }
    }

    /// Suppresses every label under this element: a `c:delete val="1"` replacing the settings group,
    /// which `CT_DLbls` puts in one `xsd:choice` with it.
    pub(crate) fn suppress_all(&mut self, interner: &mut Interner) {
        for local in LABEL_SETTING_LOCALS.iter().chain(&CONTAINER_ONLY_LOCALS) {
            self.drop_raw(interner, local);
        }
        self.content
            .retain(|item| !matches!(item, DataLabelsContent::Label(_)));
        let element = chart_val_leaf(interner, "delete", "1");
        self.put_raw(interner, element);
    }

    /// Removes a `c:delete`, so the settings group may be written in its place.
    fn clear_suppression(&mut self, interner: &mut Interner) {
        self.drop_raw(interner, "delete");
    }

    /// The position in `content` of the override naming point `index`.
    fn label_index_of(&self, interner: &Interner, index: u32) -> Option<usize> {
        self.content.iter().position(|item| match item {
            DataLabelsContent::Label(label) => label.index(interner) == Some(index),
            DataLabelsContent::Raw(_) => false,
        })
    }

    /// Inserts a new override, keeping the `c:dLbl` run in ascending `c:idx` order — which is how
    /// Office writes them, and which keeps a reader's scan monotonic. `CT_DLbls` gives every
    /// `c:dLbl` one rank, so the schema is satisfied either way; this is about the file being
    /// readable.
    fn insert_label(&mut self, interner: &Interner, label: DataLabel) {
        let index = label.index(interner);
        let at = self
            .content
            .iter()
            .position(|item| match item {
                DataLabelsContent::Label(existing) => match (existing.index(interner), index) {
                    (Some(existing), Some(index)) => existing > index,
                    _ => false,
                },
                DataLabelsContent::Raw(node) => {
                    // The `c:dLbl` run comes first in `CT_DLbls`; the first ranked sibling after it
                    // is where a new one must stop.
                    chart_local(node, interner).is_some_and(|local| local != "dLbl")
                }
            })
            .unwrap_or(self.content.len());
        self.content.insert(at, DataLabelsContent::Label(label));
        self.empty = false;
    }
}

/// Reads the `EG_DLblShared` settings through the accessors a caller supplies — the half of
/// [`DataLabels::settings`] and [`DataLabel::settings`] that is identical for both.
fn read_label_settings(
    flag: impl Fn(&str) -> Option<bool>,
    position: impl Fn() -> Option<DataLabelPosition>,
    separator: impl Fn() -> Option<String>,
    number_format: impl Fn() -> Option<String>,
) -> DataLabelSettings {
    DataLabelSettings {
        suppressed: flag("delete"),
        shows_value: flag("showVal"),
        shows_category_name: flag("showCatName"),
        shows_series_name: flag("showSerName"),
        shows_percentage: flag("showPercent"),
        shows_bubble_size: flag("showBubbleSize"),
        shows_legend_key: flag("showLegendKey"),
        shows_leader_lines: None,
        position: position(),
        separator: separator(),
        number_format: number_format(),
    }
}

// -------------------------------------------------------------------------------------------------
// c:dPt — one point's own formatting
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`DataPointFormat`]: its shape properties, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataPointFormatContent {
    /// The point's fill and outline (`c:spPr`).
    ShapeProperties(SeriesShapeProperties),
    /// Any other child — `c:idx`, `c:invertIfNegative`, `c:marker`, `c:bubble3D`, `c:explosion`,
    /// `c:pictureOptions`, `c:extLst` — kept verbatim.
    Raw(RawNode),
}

/// `c:dPt` (`CT_DPt`) — one point drawn differently from the rest of its series: the slice of a pie
/// in its own colour, the column that is highlighted.
///
/// Like [`DataLabel`], it is anchored by `c:idx` — the point's 0-based position in the series — and
/// **nothing in this crate renumbers it**. An edit that shortens a series therefore leaves every
/// `c:idx` naming exactly the point it named before; the ones that now address past the end are
/// reported by `Series::decoration_beyond_data` and removed only when a caller asks.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct DataPointFormat {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "spPr", variant = ShapeProperties, ty = SeriesShapeProperties))]
    content: Vec<DataPointFormatContent>,
}

raw_child_access!(
    DataPointFormat,
    DataPointFormatContent,
    DATA_POINT_FORMAT,
    [ShapeProperties],
    ["spPr"]
);

impl DataPointFormat {
    /// A fresh `c:dPt` for the point at `index`, with no formatting of its own yet.
    pub(crate) fn new(interner: &mut Interner, index: u32) -> Self {
        let idx = chart_val_leaf(interner, "idx", &index.to_string());
        Self {
            name: chart_name(interner, "dPt"),
            attributes: Vec::new(),
            empty: false,
            content: vec![DataPointFormatContent::Raw(RawNode::Element(idx))],
        }
    }

    /// The 0-based index of the point this formatting belongs to (`c:idx@val`). See the type docs on
    /// what that index survives.
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<u32> {
        self.unsigned(interner, "idx")
    }

    /// Re-anchors this formatting to a different point. The only way a `c:idx` ever changes.
    pub fn set_index(&mut self, interner: &mut Interner, index: u32) {
        self.set_scalar(interner, "idx", Some(&index.to_string()));
    }

    /// How far a pie or doughnut slice is pulled out of the centre, as a percentage of the radius
    /// (`c:explosion`).
    #[must_use]
    pub fn explosion(&self, interner: &Interner) -> Option<u32> {
        self.unsigned(interner, "explosion")
    }

    /// Pulls a slice out of the centre by `percent`, or (for `None`) puts it back.
    pub fn set_explosion(&mut self, interner: &mut Interner, percent: Option<u32>) {
        self.set_scalar(
            interner,
            "explosion",
            percent.map(|p| p.to_string()).as_deref(),
        );
    }

    /// Whether the point is drawn with its fill inverted when its value is negative
    /// (`c:invertIfNegative`).
    #[must_use]
    pub fn inverts_if_negative(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "invertIfNegative")
    }

    /// Sets, or clears, the invert-if-negative flag.
    pub fn set_inverts_if_negative(&mut self, interner: &mut Interner, invert: Option<bool>) {
        self.set_flag(interner, "invertIfNegative", invert);
    }

    /// Whether a bubble is drawn with a three-dimensional effect (`c:bubble3D`).
    #[must_use]
    pub fn is_bubble_3d(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "bubble3D")
    }

    /// The point's shape properties (`c:spPr`), or `None` when it declares none.
    #[must_use]
    pub fn shape_properties(&self) -> Option<&SeriesShapeProperties> {
        self.content.iter().find_map(|item| match item {
            DataPointFormatContent::ShapeProperties(properties) => Some(properties),
            DataPointFormatContent::Raw(_) => None,
        })
    }

    /// The point's fill — the colour that makes it stand out from its series — or `None` when it
    /// declares none and is drawn like the rest.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<mjx_dml::FillSpec> {
        self.shape_properties()
            .and_then(|properties| properties.fill(interner))
    }

    /// Sets the point's fill, creating its `c:spPr` in schema position if it had none.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: &mjx_dml::FillSpec) {
        self.shape_properties_mut(interner).set_fill(interner, fill);
    }

    /// The point's outline (`a:ln`), or `None` when it declares none.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<mjx_dml::LineSpec> {
        self.shape_properties()
            .and_then(|properties| properties.line(interner))
    }

    /// Sets the point's outline, creating its `c:spPr` in schema position if it had none.
    pub fn set_line(&mut self, interner: &mut Interner, line: &mjx_dml::LineSpec) {
        self.shape_properties_mut(interner).set_line(interner, line);
    }

    /// The point's shape properties, creating an empty `c:spPr` at its rank if it has none.
    fn shape_properties_mut(&mut self, interner: &mut Interner) -> &mut SeriesShapeProperties {
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, DataPointFormatContent::ShapeProperties(_)))
        {
            let DataPointFormatContent::ShapeProperties(properties) = &mut self.content[index]
            else {
                unreachable!("the index was just found by matching this variant")
            };
            return properties;
        }
        let at = self.insert_index(interner, "spPr");
        self.content.insert(
            at,
            DataPointFormatContent::ShapeProperties(SeriesShapeProperties::new(interner)),
        );
        self.empty = false;
        let DataPointFormatContent::ShapeProperties(properties) = &mut self.content[at] else {
            unreachable!("the element inserted at `at` was a ShapeProperties")
        };
        properties
    }
}

// -------------------------------------------------------------------------------------------------
// c:trendline
// -------------------------------------------------------------------------------------------------

/// One ordered child of a [`Trendline`]: its shape properties, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrendlineContent {
    /// The curve's own outline (`c:spPr`).
    ShapeProperties(SeriesShapeProperties),
    /// Any other child — `c:name`, `c:trendlineType`, `c:order`, `c:period`, `c:forward`,
    /// `c:backward`, `c:intercept`, `c:dispRSqr`, `c:dispEq`, `c:trendlineLbl`, `c:extLst` — kept
    /// verbatim.
    Raw(RawNode),
}

/// `c:trendline` (`CT_Trendline`) — a curve fitted through a series, optionally extended past its
/// ends and annotated with its equation and R².
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Trendline {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "spPr", variant = ShapeProperties, ty = SeriesShapeProperties))]
    content: Vec<TrendlineContent>,
}

raw_child_access!(
    Trendline,
    TrendlineContent,
    TRENDLINE,
    [ShapeProperties],
    ["spPr"]
);

impl Trendline {
    /// A fresh `c:trendline` from `spec`, whose settings have already been validated.
    pub(crate) fn new(interner: &mut Interner, spec: &TrendlineSpec) -> Self {
        let mut trendline = Self {
            name: chart_name(interner, "trendline"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        // In `CT_Trendline` sequence order; placement still runs on each, so a change to the schema
        // moves the children rather than breaking them.
        if let Some(name) = &spec.name {
            trendline.set_text_child(interner, "name", Some(name));
        }
        trendline.set_scalar(interner, "trendlineType", Some(spec.kind.to_wire()));
        if let Some(order) = spec.polynomial_order {
            trendline.set_scalar(interner, "order", Some(&order.to_string()));
        }
        if let Some(period) = spec.moving_average_period {
            trendline.set_scalar(interner, "period", Some(&period.to_string()));
        }
        for (local, value) in [
            ("forward", spec.forward_periods),
            ("backward", spec.backward_periods),
            ("intercept", spec.intercept),
        ] {
            if let Some(text) = value.and_then(f64_wire) {
                trendline.set_scalar(interner, local, Some(&text));
            }
        }
        if let Some(show) = spec.displays_r_squared {
            trendline.set_flag(interner, "dispRSqr", Some(show));
        }
        if let Some(show) = spec.displays_equation {
            trendline.set_flag(interner, "dispEq", Some(show));
        }
        trendline
    }

    /// The curve this trendline fits (`c:trendlineType`). `CT_TrendlineType` defaults `@val` to
    /// `linear`, so a bare `<c:trendlineType/>` reads as [`Linear`](TrendlineKind::Linear).
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<TrendlineKind> {
        let element = self.raw_element(interner, "trendlineType")?;
        match attr_str(&element.attributes, interner, "val") {
            Some(value) => TrendlineKind::from_wire(value),
            None => Some(TrendlineKind::Linear),
        }
    }

    /// Changes the curve this trendline fits.
    pub fn set_kind(&mut self, interner: &mut Interner, kind: TrendlineKind) {
        self.set_scalar(interner, "trendlineType", Some(kind.to_wire()));
    }

    /// The trendline's name, shown in the legend (`c:name`), or `None` when the application derives
    /// one.
    #[must_use]
    pub fn name(&self, interner: &Interner) -> Option<String> {
        self.text_child(interner, "name")
    }

    /// The order of a polynomial curve (`c:order`, `ST_Order` 2–6). `CT_Order` defaults `@val` to
    /// 2.
    #[must_use]
    pub fn order(&self, interner: &Interner) -> Option<u32> {
        self.raw_element(interner, "order").map(|element| {
            match attr_str(&element.attributes, interner, "val") {
                Some(value) => value.trim().parse().unwrap_or(2),
                None => 2,
            }
        })
    }

    /// The window of a moving average (`c:period`, `ST_Period` 2 upwards). `CT_Period` defaults
    /// `@val` to 2.
    #[must_use]
    pub fn period(&self, interner: &Interner) -> Option<u32> {
        self.raw_element(interner, "period").map(|element| {
            match attr_str(&element.attributes, interner, "val") {
                Some(value) => value.trim().parse().unwrap_or(2),
                None => 2,
            }
        })
    }

    /// How far past the last point the curve is extended, in categories (`c:forward`).
    #[must_use]
    pub fn forward_periods(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "forward")
    }

    /// How far before the first point the curve is extended, in categories (`c:backward`).
    #[must_use]
    pub fn backward_periods(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "backward")
    }

    /// The y value the curve is forced through (`c:intercept`).
    #[must_use]
    pub fn intercept(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "intercept")
    }

    /// Whether the curve's equation is drawn on the chart (`c:dispEq`).
    #[must_use]
    pub fn displays_equation(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "dispEq")
    }

    /// Whether the curve's R² is drawn on the chart (`c:dispRSqr`).
    #[must_use]
    pub fn displays_r_squared(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "dispRSqr")
    }

    /// Draws, or stops drawing, the curve's equation and R².
    pub fn set_display(
        &mut self,
        interner: &mut Interner,
        equation: Option<bool>,
        r_squared: Option<bool>,
    ) {
        self.set_flag(interner, "dispRSqr", r_squared);
        self.set_flag(interner, "dispEq", equation);
    }

    /// Rewrites the trendline from `spec`, replacing every setting it states and clearing every
    /// optional one it leaves unset — an in-place edit of the curve, keeping its `c:spPr` and any
    /// `c:trendlineLbl` it carries.
    ///
    /// # Errors
    /// Whatever [`TrendlineSpec::validate`] answers — checked before anything is written.
    pub fn apply_spec(
        &mut self,
        interner: &mut Interner,
        spec: &TrendlineSpec,
    ) -> Result<(), ChartDataError> {
        spec.validate()?;
        self.set_text_child(interner, "name", spec.name.as_deref());
        self.set_scalar(interner, "trendlineType", Some(spec.kind.to_wire()));
        self.set_scalar(
            interner,
            "order",
            spec.polynomial_order.map(|o| o.to_string()).as_deref(),
        );
        self.set_scalar(
            interner,
            "period",
            spec.moving_average_period.map(|p| p.to_string()).as_deref(),
        );
        for (local, value) in [
            ("forward", spec.forward_periods),
            ("backward", spec.backward_periods),
            ("intercept", spec.intercept),
        ] {
            self.set_scalar(interner, local, value.and_then(f64_wire).as_deref());
        }
        self.set_flag(interner, "dispRSqr", spec.displays_r_squared);
        self.set_flag(interner, "dispEq", spec.displays_equation);
        Ok(())
    }

    /// The curve's own shape properties (`c:spPr`), or `None` when it declares none.
    #[must_use]
    pub fn shape_properties(&self) -> Option<&SeriesShapeProperties> {
        self.content.iter().find_map(|item| match item {
            TrendlineContent::ShapeProperties(properties) => Some(properties),
            TrendlineContent::Raw(_) => None,
        })
    }

    /// The curve's outline (`a:ln`), or `None` when it declares none.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<mjx_dml::LineSpec> {
        self.shape_properties()
            .and_then(|properties| properties.line(interner))
    }

    /// Sets the curve's outline, creating its `c:spPr` at its rank if it had none.
    pub fn set_line(&mut self, interner: &mut Interner, line: &mjx_dml::LineSpec) {
        if let Some(index) = self
            .content
            .iter()
            .position(|item| matches!(item, TrendlineContent::ShapeProperties(_)))
        {
            let TrendlineContent::ShapeProperties(properties) = &mut self.content[index] else {
                unreachable!("the index was just found by matching this variant")
            };
            properties.set_line(interner, line);
            return;
        }
        let at = self.insert_index(interner, "spPr");
        let mut properties = SeriesShapeProperties::new(interner);
        properties.set_line(interner, line);
        self.content
            .insert(at, TrendlineContent::ShapeProperties(properties));
        self.empty = false;
    }
}

// -------------------------------------------------------------------------------------------------
// c:errBars
// -------------------------------------------------------------------------------------------------

/// One ordered child of an [`ErrorBars`]: a custom length source, its shape properties, or an
/// opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorBarsContent {
    /// The per-point lengths in the positive direction (`c:plus`, a `CT_NumDataSource`).
    Plus(NumericData),
    /// The per-point lengths in the negative direction (`c:minus`).
    Minus(NumericData),
    /// The bars' own outline (`c:spPr`).
    ShapeProperties(SeriesShapeProperties),
    /// Any other child — `c:errDir`, `c:errBarType`, `c:errValType`, `c:noEndCap`, `c:val`,
    /// `c:extLst` — kept verbatim.
    Raw(RawNode),
}

/// `c:errBars` (`CT_ErrBars`) — the uncertainty drawn around each point of a series.
///
/// `c:plus` and `c:minus` are the same `CT_NumDataSource` a series' `c:val` is, so they read and
/// write through the same [`NumericData`] — a custom error bar's lengths can come from a workbook
/// reference exactly as its values can.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct ErrorBars {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "plus", variant = Plus, ty = NumericData),
        child(local = "minus", variant = Minus, ty = NumericData),
        child(local = "spPr", variant = ShapeProperties, ty = SeriesShapeProperties)
    )]
    content: Vec<ErrorBarsContent>,
}

raw_child_access!(
    ErrorBars,
    ErrorBarsContent,
    ERROR_BARS,
    [Plus, Minus, ShapeProperties],
    ["plus", "minus", "spPr"]
);

impl ErrorBars {
    /// A fresh `c:errBars` from `spec`, whose settings have already been validated.
    pub(crate) fn new(interner: &mut Interner, spec: &ErrorBarSpec) -> Self {
        let mut bars = Self {
            name: chart_name(interner, "errBars"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        if let Some(direction) = spec.direction {
            bars.set_scalar(interner, "errDir", Some(direction.to_wire()));
        }
        bars.set_scalar(interner, "errBarType", Some(spec.bar_type.to_wire()));
        bars.set_scalar(interner, "errValType", Some(spec.value_type.to_wire()));
        if let Some(no_end_cap) = spec.no_end_cap {
            bars.set_flag(interner, "noEndCap", Some(no_end_cap));
        }
        for (local, values) in [("plus", &spec.plus_values), ("minus", &spec.minus_values)] {
            if let Some(values) = values {
                bars.put_number_source(interner, local, values);
            }
        }
        if let Some(text) = spec.value.and_then(f64_wire) {
            bars.set_scalar(interner, "val", Some(&text));
        }
        bars
    }

    /// Which axis the bars run along (`c:errDir`), or `None` when the chart decides.
    #[must_use]
    pub fn direction(&self, interner: &Interner) -> Option<ErrorBarDirection> {
        self.scalar(interner, "errDir")
            .and_then(ErrorBarDirection::from_wire)
    }

    /// Which side(s) of the point the bars are drawn on (`c:errBarType`). `CT_ErrBarType` defaults
    /// `@val` to `both`.
    #[must_use]
    pub fn bar_type(&self, interner: &Interner) -> Option<ErrorBarType> {
        let element = self.raw_element(interner, "errBarType")?;
        match attr_str(&element.attributes, interner, "val") {
            Some(value) => ErrorBarType::from_wire(value),
            None => Some(ErrorBarType::Both),
        }
    }

    /// How the bars' length is arrived at (`c:errValType`). `CT_ErrValType` defaults `@val` to
    /// `fixedVal`.
    #[must_use]
    pub fn value_type(&self, interner: &Interner) -> Option<ErrorValueType> {
        let element = self.raw_element(interner, "errValType")?;
        match attr_str(&element.attributes, interner, "val") {
            Some(value) => ErrorValueType::from_wire(value),
            None => Some(ErrorValueType::FixedValue),
        }
    }

    /// Whether the bars are drawn without their end caps (`c:noEndCap`).
    #[must_use]
    pub fn no_end_cap(&self, interner: &Interner) -> Option<bool> {
        self.flag(interner, "noEndCap")
    }

    /// The single length every bar takes, read as [`value_type`](Self::value_type) says (`c:val`).
    #[must_use]
    pub fn value(&self, interner: &Interner) -> Option<f64> {
        self.number(interner, "val")
    }

    /// The per-point lengths in the positive direction (`c:plus`), or `None` when the bars are not
    /// custom.
    #[must_use]
    pub fn plus(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            ErrorBarsContent::Plus(data) => Some(data),
            _ => None,
        })
    }

    /// The per-point lengths in the negative direction (`c:minus`).
    #[must_use]
    pub fn minus(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            ErrorBarsContent::Minus(data) => Some(data),
            _ => None,
        })
    }

    /// The positive per-point lengths as numbers — from a `c:numRef`'s cache or a `c:numLit`.
    /// Empty when the bars are not custom.
    #[must_use]
    pub fn plus_values(&self) -> Vec<f64> {
        self.plus().map(NumericData::values).unwrap_or_default()
    }

    /// The negative per-point lengths as numbers.
    #[must_use]
    pub fn minus_values(&self) -> Vec<f64> {
        self.minus().map(NumericData::values).unwrap_or_default()
    }

    /// The bars' own shape properties (`c:spPr`), or `None` when they declare none.
    #[must_use]
    pub fn shape_properties(&self) -> Option<&SeriesShapeProperties> {
        self.content.iter().find_map(|item| match item {
            ErrorBarsContent::ShapeProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The bars' outline (`a:ln`), or `None` when they declare none.
    #[must_use]
    pub fn line(&self, interner: &Interner) -> Option<mjx_dml::LineSpec> {
        self.shape_properties()
            .and_then(|properties| properties.line(interner))
    }

    /// Rewrites the bars from `spec`, replacing every setting it states and every custom source.
    ///
    /// # Errors
    /// Whatever [`ErrorBarSpec::validate`] answers — checked before anything is written.
    pub fn apply_spec(
        &mut self,
        interner: &mut Interner,
        spec: &ErrorBarSpec,
    ) -> Result<(), ChartDataError> {
        spec.validate()?;
        self.set_scalar(interner, "errDir", spec.direction.map(|d| d.to_wire()));
        self.set_scalar(interner, "errBarType", Some(spec.bar_type.to_wire()));
        self.set_scalar(interner, "errValType", Some(spec.value_type.to_wire()));
        self.set_flag(interner, "noEndCap", spec.no_end_cap);
        for (local, values) in [("plus", &spec.plus_values), ("minus", &spec.minus_values)] {
            match values {
                Some(values) => self.put_number_source(interner, local, values),
                None => self.drop_number_source(local),
            }
        }
        self.set_scalar(interner, "val", spec.value.and_then(f64_wire).as_deref());
        Ok(())
    }

    /// Replaces (or inserts, at its rank) `c:plus` or `c:minus`, holding `values` as a `c:numLit`.
    fn put_number_source(&mut self, interner: &mut Interner, local: &str, values: &[f64]) {
        let element = number_literal_source(interner, local, values);
        let Ok(data) = NumericData::from_xml(&element, interner) else {
            // `NumericData`'s reader accepts any element: it keeps the name and buckets every child.
            // There is no shape it rejects, so this arm is unreachable in practice — and dropping
            // the source silently is still better than a panic on a caller's data.
            return;
        };
        let item = if local == "plus" {
            ErrorBarsContent::Plus(data)
        } else {
            ErrorBarsContent::Minus(data)
        };
        if let Some(index) = self.number_source_index(local) {
            self.content[index] = item;
            return;
        }
        let at = self.insert_index(interner, local);
        self.content.insert(at, item);
        self.empty = false;
    }

    /// Removes `c:plus` or `c:minus`, if present.
    fn drop_number_source(&mut self, local: &str) {
        if let Some(index) = self.number_source_index(local) {
            self.content.remove(index);
        }
    }

    /// The position of `c:plus` or `c:minus` in `content`.
    fn number_source_index(&self, local: &str) -> Option<usize> {
        self.content.iter().position(|item| {
            matches!(
                (local, item),
                ("plus", ErrorBarsContent::Plus(_)) | ("minus", ErrorBarsContent::Minus(_))
            )
        })
    }
}
