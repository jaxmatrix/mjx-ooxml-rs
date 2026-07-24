//! The chart-space spine — `c:chartSpace → c:chart → c:plotArea`.
//!
//! Every chart part (`/ppt/charts/chartN.xml`) is rooted at `c:chartSpace`, whose `c:chart` holds a
//! `c:plotArea`, and the plot area holds the plots (this tier: one `c:barChart`). The spine is thin:
//! its job is to reach the plot, so a caller can ask a chart its kind and read its series. Everything
//! it does not model — `c:date1904`, `c:txPr`, `c:externalData` at the space; `c:autoTitleDeleted`,
//! `c:dispBlanksAs` at the chart; `c:catAx`, `c:valAx` at the plot area — rides through the `Raw`
//! bucket byte-for-byte.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

use crate::plot::{BarChart, ChartKind};

/// One ordered child of a [`PlotArea`]: a typed plot, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlotAreaContent {
    /// A bar/column plot (`c:barChart`).
    Bar(BarChart),
    /// Any other child — another plot type, `c:catAx`, `c:valAx`, `c:layout`, unknown — kept verbatim.
    Raw(RawNode),
}

/// `c:plotArea` (`CT_PlotArea`) — the plots and axes a chart draws. This tier reads the bar plot.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct PlotArea {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "barChart", variant = Bar, ty = BarChart))]
    content: Vec<PlotAreaContent>,
}

impl PlotArea {
    /// The bar plot (`c:barChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn bar_chart(&self) -> Option<&BarChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Bar(bar) => Some(bar),
            PlotAreaContent::Raw(_) => None,
        })
    }

    /// The kind of plot this area draws, or `None` for a plot type this tier does not model yet.
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.bar_chart().map(|_| ChartKind::Bar)
    }
}

/// One ordered child of a [`Chart`]: the plot area, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartContent {
    /// The plot area (`c:plotArea`).
    PlotArea(PlotArea),
    /// Any other child — `c:title`, `c:autoTitleDeleted`, `c:dispBlanksAs`, unknown — kept verbatim.
    Raw(RawNode),
}

/// `c:chart` (`CT_Chart`) — a chart's title, plot area and legend. This tier reads the plot area.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Chart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "plotArea", variant = PlotArea, ty = PlotArea))]
    content: Vec<ChartContent>,
}

impl Chart {
    /// The chart's plot area (`c:plotArea`), or `None` if it declares none.
    #[must_use]
    pub fn plot_area(&self) -> Option<&PlotArea> {
        self.content.iter().find_map(|item| match item {
            ChartContent::PlotArea(plot_area) => Some(plot_area),
            ChartContent::Raw(_) => None,
        })
    }
}

/// One ordered child of a [`ChartSpace`]: the chart, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSpaceContent {
    /// The chart (`c:chart`).
    Chart(Chart),
    /// Any other child — `c:date1904`, `c:lang`, `c:txPr`, `c:externalData`, unknown — kept verbatim.
    Raw(RawNode),
}

/// `c:chartSpace` (`CT_ChartSpace`) — the root of a chart part.
///
/// Parse the bytes of a chart part into this with [`FromXml`](mjx_ooxml_core::FromXml); it re-emits
/// byte-for-byte with [`ToXml`](mjx_ooxml_core::ToXml).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct ChartSpace {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "chart", variant = Chart, ty = Chart))]
    content: Vec<ChartSpaceContent>,
}

impl ChartSpace {
    /// The chart (`c:chart`), or `None` if the part declares none.
    #[must_use]
    pub fn chart(&self) -> Option<&Chart> {
        self.content.iter().find_map(|item| match item {
            ChartSpaceContent::Chart(chart) => Some(chart),
            ChartSpaceContent::Raw(_) => None,
        })
    }

    /// The chart's plot area, walking `c:chart → c:plotArea` — `None` if either is absent.
    #[must_use]
    pub fn plot_area(&self) -> Option<&PlotArea> {
        self.chart().and_then(Chart::plot_area)
    }

    /// The chart's bar plot, walking `c:chart → c:plotArea → c:barChart` — `None` if any is absent.
    #[must_use]
    pub fn bar_chart(&self) -> Option<&BarChart> {
        self.plot_area().and_then(PlotArea::bar_chart)
    }

    /// The kind of plot this chart draws, or `None` for a plot type this tier does not model.
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.plot_area().and_then(PlotArea::chart_kind)
    }
}
