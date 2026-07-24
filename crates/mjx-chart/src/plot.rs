//! The bar plot and its series — `c:barChart` and `c:ser`.
//!
//! A plot is one chart *type* inside the plot area; a plot holds one or more **series** (`c:ser`),
//! and each series binds a name (`c:tx`), the category labels every series shares (`c:cat`) and its
//! own values (`c:val`) — the data layer in [`crate::data`]. This tier models the bar plot end to
//! end; other plot types (line, pie, …) are later tiers.
//!
//! ```xml
//! <c:barChart>
//!   <c:barDir val="col"/>
//!   <c:grouping val="clustered"/>
//!   <c:ser>
//!     <c:idx val="0"/><c:order val="0"/>
//!     <c:tx>…</c:tx> <c:cat>…</c:cat> <c:val>…</c:val>
//!   </c:ser>
//!   <c:axId val="…"/> <c:axId val="…"/>
//! </c:barChart>
//! ```
//!
//! `c:barDir`, `c:grouping`, `c:axId`, and a series' `c:idx`/`c:order` are kept in the `Raw` bucket
//! and read through small accessors — this tier reads the data, not the axes or styling.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::raw_child_attr;
use crate::data::{CategoryData, NumericData, SeriesText};

/// Which way a bar plot's bars run (`c:barDir@val`, `ST_BarDir`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDirection {
    /// Vertical bars — a column chart (wire `col`).
    Column,
    /// Horizontal bars (wire `bar`).
    Bar,
}

impl BarDirection {
    /// Maps the wire token (`col`/`bar`) to a direction.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "col" => Some(Self::Column),
            "bar" => Some(Self::Bar),
            _ => None,
        }
    }
}

/// How a bar plot's series are combined (`c:grouping@val`, `ST_BarGrouping`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarGrouping {
    /// Bars for each category stand side by side (wire `clustered`).
    Clustered,
    /// Series stacked into one bar per category (wire `stacked`).
    Stacked,
    /// Stacked and normalized to 100% (wire `percentStacked`).
    PercentStacked,
    /// Series overlaid on a shared baseline (wire `standard`).
    Standard,
}

impl BarGrouping {
    /// Maps the wire token to a grouping.
    fn from_wire(wire: &str) -> Option<Self> {
        match wire {
            "clustered" => Some(Self::Clustered),
            "stacked" => Some(Self::Stacked),
            "percentStacked" => Some(Self::PercentStacked),
            "standard" => Some(Self::Standard),
            _ => None,
        }
    }
}

/// The chart type a plot area draws — one variant per modeled plot. Extended as later tiers add
/// line, pie, area and scatter plots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartKind {
    /// A bar/column plot (`c:barChart`).
    Bar,
}

/// One ordered child of a [`Series`]: its name, categories or values, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesContent {
    /// The series name (`c:tx`).
    Text(SeriesText),
    /// The shared category labels (`c:cat`).
    Categories(CategoryData),
    /// The series' values (`c:val`).
    Values(NumericData),
    /// Any other child — `c:idx`, `c:order`, `c:spPr`, whitespace, unknown — preserved verbatim.
    Raw(RawNode),
}

/// `c:ser` (`CT_BarSer`) — one series of a bar plot: a name, the shared categories, and its values.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Series {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tx", variant = Text, ty = SeriesText),
        child(local = "cat", variant = Categories, ty = CategoryData),
        child(local = "val", variant = Values, ty = NumericData)
    )]
    content: Vec<SeriesContent>,
}

impl Series {
    /// The series' 0-based index among its plot's series (`c:idx@val`).
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "idx")
    }

    /// The series' draw order (`c:order@val`) — which series paints over which.
    #[must_use]
    pub fn order(&self, interner: &Interner) -> Option<u32> {
        self.raw_val(interner, "order")
    }

    /// The series name (`c:tx`), or `None` if it declares none.
    #[must_use]
    pub fn name_source(&self) -> Option<&SeriesText> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Text(text) => Some(text),
            _ => None,
        })
    }

    /// The series' resolved name, or `None` when it has no `c:tx` (or the name is unmodeled).
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name_source().and_then(SeriesText::text)
    }

    /// The series' category labels (`c:cat`), or `None` if it declares none.
    #[must_use]
    pub fn categories(&self) -> Option<&CategoryData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Categories(categories) => Some(categories),
            _ => None,
        })
    }

    /// The series' values (`c:val`), or `None` if it declares none.
    #[must_use]
    pub fn values(&self) -> Option<&NumericData> {
        self.content.iter().find_map(|item| match item {
            SeriesContent::Values(values) => Some(values),
            _ => None,
        })
    }

    /// Reads the `@val` of a raw scalar child (`c:idx`, `c:order`) as a `u32`.
    fn raw_val(&self, interner: &Interner, local: &str) -> Option<u32> {
        let raw = self.content.iter().filter_map(|item| match item {
            SeriesContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, local, "val").and_then(|s| s.trim().parse().ok())
    }
}

/// One ordered child of a [`BarChart`]: a typed series (`c:ser`), or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarChartContent {
    /// A series (`c:ser`).
    Series(Series),
    /// Any other child — `c:barDir`, `c:grouping`, `c:axId`, whitespace, unknown — kept verbatim.
    Raw(RawNode),
}

/// `c:barChart` (`CT_BarChart`) — a bar/column plot and its series.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct BarChart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "ser", variant = Series, ty = Series))]
    content: Vec<BarChartContent>,
}

impl BarChart {
    /// The plot's series, in order.
    pub fn series(&self) -> impl Iterator<Item = &Series> {
        self.content.iter().filter_map(|item| match item {
            BarChartContent::Series(series) => Some(series),
            BarChartContent::Raw(_) => None,
        })
    }

    /// The `n`-th series, or `None` if the plot has fewer.
    #[must_use]
    pub fn series_at(&self, n: usize) -> Option<&Series> {
        self.series().nth(n)
    }

    /// How many series the plot draws.
    #[must_use]
    pub fn series_count(&self) -> usize {
        self.series().count()
    }

    /// Which way the bars run (`c:barDir`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn direction(&self, interner: &Interner) -> Option<BarDirection> {
        self.raw_val(interner, "barDir")
            .and_then(BarDirection::from_wire)
    }

    /// How the series are combined (`c:grouping`), or `None` if unset or unrecognized.
    #[must_use]
    pub fn grouping(&self, interner: &Interner) -> Option<BarGrouping> {
        self.raw_val(interner, "grouping")
            .and_then(BarGrouping::from_wire)
    }

    /// Reads the `@val` of a raw scalar child (`c:barDir`, `c:grouping`).
    fn raw_val(&self, interner: &Interner, local: &str) -> Option<&str> {
        let raw = self.content.iter().filter_map(|item| match item {
            BarChartContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, local, "val")
    }
}
