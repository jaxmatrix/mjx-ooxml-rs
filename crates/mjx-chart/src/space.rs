//! The chart-space spine — `c:chartSpace → c:chart → c:plotArea`.
//!
//! Every chart part (`/ppt/charts/chartN.xml`) is rooted at `c:chartSpace`, whose `c:chart` holds a
//! `c:plotArea`, and the plot area holds the plots. A plot area may hold **more than one** plot (a
//! combo chart), so a chart is described by a set of kinds. The spine is thin: its job is to reach
//! the plots, so a caller can ask a chart its kind(s) and read each plot's series. Everything it does
//! not model — `c:date1904`, `c:txPr`, `c:externalData` at the space; `c:autoTitleDeleted`,
//! `c:dispBlanksAs` at the chart; `c:catAx`, `c:valAx` at the plot area — rides through the `Raw`
//! bucket byte-for-byte.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

use crate::plot::{
    AreaChart, BarChart, ChartKind, DoughnutChart, LineChart, PieChart, ScatterChart, Series,
};

/// One ordered child of a [`PlotArea`]: a typed plot, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlotAreaContent {
    /// A bar/column plot (`c:barChart`).
    Bar(BarChart),
    /// A line plot (`c:lineChart`).
    Line(LineChart),
    /// A pie plot (`c:pieChart`).
    Pie(PieChart),
    /// An area plot (`c:areaChart`).
    Area(AreaChart),
    /// An X/Y scatter plot (`c:scatterChart`).
    Scatter(ScatterChart),
    /// A doughnut plot (`c:doughnutChart`).
    Doughnut(DoughnutChart),
    /// Any other child — an unmodeled plot type, `c:catAx`, `c:valAx`, `c:layout`, unknown — kept
    /// verbatim.
    Raw(RawNode),
}

/// `c:plotArea` (`CT_PlotArea`) — the plots and axes a chart draws. A plot area may hold more than
/// one plot (a combo chart); unmodeled plot types (radar, bubble, 3-D, …) and the axes ride through
/// the `Raw` bucket.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct PlotArea {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "barChart", variant = Bar, ty = BarChart),
        child(local = "lineChart", variant = Line, ty = LineChart),
        child(local = "pieChart", variant = Pie, ty = PieChart),
        child(local = "areaChart", variant = Area, ty = AreaChart),
        child(local = "scatterChart", variant = Scatter, ty = ScatterChart),
        child(local = "doughnutChart", variant = Doughnut, ty = DoughnutChart)
    )]
    content: Vec<PlotAreaContent>,
}

impl PlotArea {
    /// The first bar plot (`c:barChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn bar_chart(&self) -> Option<&BarChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Bar(bar) => Some(bar),
            _ => None,
        })
    }

    /// The first line plot (`c:lineChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn line_chart(&self) -> Option<&LineChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Line(line) => Some(line),
            _ => None,
        })
    }

    /// The first pie plot (`c:pieChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn pie_chart(&self) -> Option<&PieChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Pie(pie) => Some(pie),
            _ => None,
        })
    }

    /// The first area plot (`c:areaChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn area_chart(&self) -> Option<&AreaChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Area(area) => Some(area),
            _ => None,
        })
    }

    /// The first scatter plot (`c:scatterChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn scatter_chart(&self) -> Option<&ScatterChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Scatter(scatter) => Some(scatter),
            _ => None,
        })
    }

    /// The first doughnut plot (`c:doughnutChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn doughnut_chart(&self) -> Option<&DoughnutChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Doughnut(doughnut) => Some(doughnut),
            _ => None,
        })
    }

    /// The kind of the plot area's first plot, or `None` for a plot type this tier does not model.
    /// For a combo chart, see [`chart_kinds`](Self::chart_kinds).
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.chart_kinds().into_iter().next()
    }

    /// The kind of every modeled plot, in document order — one entry per plot element, so a combo
    /// chart yields several (e.g. `[Bar, Line]`).
    #[must_use]
    pub fn chart_kinds(&self) -> Vec<ChartKind> {
        self.content
            .iter()
            .filter_map(|item| match item {
                PlotAreaContent::Bar(plot) => Some(plot.kind()),
                PlotAreaContent::Line(plot) => Some(plot.kind()),
                PlotAreaContent::Pie(plot) => Some(plot.kind()),
                PlotAreaContent::Area(plot) => Some(plot.kind()),
                PlotAreaContent::Scatter(plot) => Some(plot.kind()),
                PlotAreaContent::Doughnut(plot) => Some(plot.kind()),
                PlotAreaContent::Raw(_) => None,
            })
            .collect()
    }

    /// Every series of every modeled plot, in document order — flattened across the plots so a combo
    /// chart's series read as one sequence.
    pub fn all_series(&self) -> impl Iterator<Item = &Series> {
        self.content.iter().flat_map(|item| {
            let plot: Box<dyn Iterator<Item = &Series>> = match item {
                PlotAreaContent::Bar(plot) => Box::new(plot.series()),
                PlotAreaContent::Line(plot) => Box::new(plot.series()),
                PlotAreaContent::Pie(plot) => Box::new(plot.series()),
                PlotAreaContent::Area(plot) => Box::new(plot.series()),
                PlotAreaContent::Scatter(plot) => Box::new(plot.series()),
                PlotAreaContent::Doughnut(plot) => Box::new(plot.series()),
                PlotAreaContent::Raw(_) => Box::new(std::iter::empty()),
            };
            plot
        })
    }

    /// Every series of every modeled plot, in document order, mutably — the write counterpart of
    /// [`all_series`](Self::all_series), for rewriting a series' cached data.
    pub fn all_series_mut(&mut self) -> impl Iterator<Item = &mut Series> {
        self.content.iter_mut().flat_map(|item| {
            let plot: Box<dyn Iterator<Item = &mut Series>> = match item {
                PlotAreaContent::Bar(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Line(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Pie(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Area(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Scatter(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Doughnut(plot) => Box::new(plot.series_mut()),
                PlotAreaContent::Raw(_) => Box::new(std::iter::empty()),
            };
            plot
        })
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

    /// The chart's plot area, mutably.
    pub fn plot_area_mut(&mut self) -> Option<&mut PlotArea> {
        self.content.iter_mut().find_map(|item| match item {
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

    /// The kind of this chart's first plot, or `None` for a plot type this tier does not model. For
    /// a combo chart, see [`chart_kinds`](Self::chart_kinds).
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.plot_area().and_then(PlotArea::chart_kind)
    }

    /// The kind of every modeled plot this chart draws, in order — one entry per plot element (a
    /// combo chart yields several), or empty when there is no plot area.
    #[must_use]
    pub fn chart_kinds(&self) -> Vec<ChartKind> {
        self.plot_area()
            .map(PlotArea::chart_kinds)
            .unwrap_or_default()
    }

    /// The chart's plot area, mutably — the write path down the spine.
    pub fn plot_area_mut(&mut self) -> Option<&mut PlotArea> {
        self.content
            .iter_mut()
            .find_map(|item| match item {
                ChartSpaceContent::Chart(chart) => Some(chart),
                ChartSpaceContent::Raw(_) => None,
            })
            .and_then(Chart::plot_area_mut)
    }

    /// How many series the chart draws across every plot — the addressing space of
    /// [`series_mut`](Self::series_mut).
    #[must_use]
    pub fn series_count(&self) -> usize {
        self.plot_area().map_or(0, |area| area.all_series().count())
    }

    /// The `n`-th series across every plot (document order), mutably — `None` when the chart draws
    /// fewer. This is what a `set_chart_series_*` edit addresses.
    pub fn series_mut(&mut self, n: usize) -> Option<&mut Series> {
        self.plot_area_mut()?.all_series_mut().nth(n)
    }
}
