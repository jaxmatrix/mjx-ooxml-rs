//! The chart-space spine — `c:chartSpace → c:chart → c:plotArea` — and the chart's furniture.
//!
//! Every chart part (`/ppt/charts/chartN.xml`) is rooted at `c:chartSpace`, whose `c:chart` holds a
//! `c:plotArea`, and the plot area holds the plots and the axes. A plot area may hold **more than
//! one** plot (a combo chart), so a chart is described by a set of kinds.
//!
//! The spine reaches everything a caller asks a chart about: its kind(s), its series and their data,
//! its axes ([`Axis`]), its title and legend, and the chart-level styling — `c:style`,
//! `c:varyColors`, `c:dispBlanksAs`, `c:plotVisOnly` — that decides how a series is drawn. What it
//! does not model — `c:date1904`, `c:lang`, `c:pivotSource`, `c:view3D`, `c:dTable`, `c:printSettings`
//! — rides through the `Raw` bucket byte-for-byte.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::namespaces::DML_MAIN;
use mjx_ooxml_types::support::on_off;

use crate::axis::{chart_local, Axis, AxisKind, BlankDisplay, ChartTitle, Legend, LegendPosition};
use mjx_ooxml_types::child_order::CHART;

use crate::author::ChartDataError;
use crate::build::{
    chart_val_leaf, insert_position, namespace_declaration, raw_child_attr, set_attr,
};
use crate::decoration::{DataLabelSettings, DataLabelSpec, DataLabels};
use crate::plot::{
    Area3DChart, AreaChart, Bar3DChart, BarChart, BubbleChart, ChartKind, DoughnutChart,
    Line3DChart, LineChart, OfPieChart, Pie3DChart, PieChart, RadarChart, ScatterChart, Series,
    SeriesDecoration, StockChart, Surface3DChart, SurfaceChart,
};

/// One ordered child of a [`PlotArea`]: a typed plot, a typed axis, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlotAreaContent {
    /// A bar/column plot (`c:barChart`).
    Bar(BarChart),
    /// A three-dimensional bar/column plot (`c:bar3DChart`).
    Bar3D(Bar3DChart),
    /// A line plot (`c:lineChart`).
    Line(LineChart),
    /// A three-dimensional line plot (`c:line3DChart`).
    Line3D(Line3DChart),
    /// A pie plot (`c:pieChart`).
    Pie(PieChart),
    /// A three-dimensional pie plot (`c:pie3DChart`).
    Pie3D(Pie3DChart),
    /// A pie-of-pie or bar-of-pie plot (`c:ofPieChart`).
    OfPie(OfPieChart),
    /// An area plot (`c:areaChart`).
    Area(AreaChart),
    /// A three-dimensional area plot (`c:area3DChart`).
    Area3D(Area3DChart),
    /// An X/Y scatter plot (`c:scatterChart`).
    Scatter(ScatterChart),
    /// A doughnut plot (`c:doughnutChart`).
    Doughnut(DoughnutChart),
    /// A radar plot (`c:radarChart`).
    Radar(RadarChart),
    /// A bubble plot (`c:bubbleChart`).
    Bubble(BubbleChart),
    /// A stock plot (`c:stockChart`).
    Stock(StockChart),
    /// A surface plot seen from above (`c:surfaceChart`).
    Surface(SurfaceChart),
    /// A three-dimensional surface plot (`c:surface3DChart`).
    Surface3D(Surface3DChart),
    /// A category axis (`c:catAx`).
    CategoryAxis(Axis),
    /// A value axis (`c:valAx`).
    ValueAxis(Axis),
    /// A date axis (`c:dateAx`).
    DateAxis(Axis),
    /// A series (depth) axis (`c:serAx`).
    SeriesAxis(Axis),
    /// Any other child — `c:layout`, `c:dTable`, `c:spPr`, `c:extLst`, unknown — kept verbatim.
    Raw(RawNode),
}

/// Runs `$body` against whichever plot a [`PlotAreaContent`] holds, or evaluates `$fallback` when it
/// holds an axis or an opaque node.
///
/// The sixteen plot types share one API (see the `plot` module), so every walk over a plot area is the
/// same expression sixteen times. This writes it once.
macro_rules! with_plot {
    ($item:expr, |$plot:ident| $body:expr, $fallback:expr) => {
        match $item {
            PlotAreaContent::Bar($plot) => $body,
            PlotAreaContent::Bar3D($plot) => $body,
            PlotAreaContent::Line($plot) => $body,
            PlotAreaContent::Line3D($plot) => $body,
            PlotAreaContent::Pie($plot) => $body,
            PlotAreaContent::Pie3D($plot) => $body,
            PlotAreaContent::OfPie($plot) => $body,
            PlotAreaContent::Area($plot) => $body,
            PlotAreaContent::Area3D($plot) => $body,
            PlotAreaContent::Scatter($plot) => $body,
            PlotAreaContent::Doughnut($plot) => $body,
            PlotAreaContent::Radar($plot) => $body,
            PlotAreaContent::Bubble($plot) => $body,
            PlotAreaContent::Stock($plot) => $body,
            PlotAreaContent::Surface($plot) => $body,
            PlotAreaContent::Surface3D($plot) => $body,
            _ => $fallback,
        }
    };
}

/// `c:plotArea` (`CT_PlotArea`) — the plots and axes a chart draws. A plot area may hold more than
/// one plot (a combo chart); all sixteen plot types and all four axis types are modeled.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct PlotArea {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "barChart", variant = Bar, ty = BarChart),
        child(local = "bar3DChart", variant = Bar3D, ty = Bar3DChart),
        child(local = "lineChart", variant = Line, ty = LineChart),
        child(local = "line3DChart", variant = Line3D, ty = Line3DChart),
        child(local = "pieChart", variant = Pie, ty = PieChart),
        child(local = "pie3DChart", variant = Pie3D, ty = Pie3DChart),
        child(local = "ofPieChart", variant = OfPie, ty = OfPieChart),
        child(local = "areaChart", variant = Area, ty = AreaChart),
        child(local = "area3DChart", variant = Area3D, ty = Area3DChart),
        child(local = "scatterChart", variant = Scatter, ty = ScatterChart),
        child(local = "doughnutChart", variant = Doughnut, ty = DoughnutChart),
        child(local = "radarChart", variant = Radar, ty = RadarChart),
        child(local = "bubbleChart", variant = Bubble, ty = BubbleChart),
        child(local = "stockChart", variant = Stock, ty = StockChart),
        child(local = "surfaceChart", variant = Surface, ty = SurfaceChart),
        child(local = "surface3DChart", variant = Surface3D, ty = Surface3DChart),
        child(local = "catAx", variant = CategoryAxis, ty = Axis),
        child(local = "valAx", variant = ValueAxis, ty = Axis),
        child(local = "dateAx", variant = DateAxis, ty = Axis),
        child(local = "serAx", variant = SeriesAxis, ty = Axis)
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

    /// The first radar plot (`c:radarChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn radar_chart(&self) -> Option<&RadarChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Radar(radar) => Some(radar),
            _ => None,
        })
    }

    /// The first bubble plot (`c:bubbleChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn bubble_chart(&self) -> Option<&BubbleChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Bubble(bubble) => Some(bubble),
            _ => None,
        })
    }

    /// The first surface plot (`c:surfaceChart`), or `None` if the plot area holds none.
    #[must_use]
    pub fn surface_chart(&self) -> Option<&SurfaceChart> {
        self.content.iter().find_map(|item| match item {
            PlotAreaContent::Surface(surface) => Some(surface),
            _ => None,
        })
    }

    /// The kind of the plot area's first plot, or `None` when it holds none. For a combo chart, see
    /// [`chart_kinds`](Self::chart_kinds).
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.chart_kinds().into_iter().next()
    }

    /// The kind of every plot, in document order — one entry per plot element, so a combo chart
    /// yields several (e.g. `[Bar, Line]`).
    #[must_use]
    pub fn chart_kinds(&self) -> Vec<ChartKind> {
        self.content
            .iter()
            .filter_map(|item| with_plot!(item, |plot| Some(plot.kind()), None))
            .collect()
    }

    /// Every series of every plot, in document order — flattened across the plots so a combo chart's
    /// series read as one sequence.
    pub fn all_series(&self) -> impl Iterator<Item = &Series> {
        self.content.iter().flat_map(|item| {
            let series: Box<dyn Iterator<Item = &Series>> = with_plot!(
                item,
                |plot| Box::new(plot.series()),
                Box::new(std::iter::empty())
            );
            series
        })
    }

    /// Every series of every plot, in document order, mutably — the write counterpart of
    /// [`all_series`](Self::all_series), for rewriting a series' data or styling.
    pub fn all_series_mut(&mut self) -> impl Iterator<Item = &mut Series> {
        self.content.iter_mut().flat_map(|item| {
            let series: Box<dyn Iterator<Item = &mut Series>> = with_plot!(
                item,
                |plot| Box::new(plot.series_mut()),
                Box::new(std::iter::empty())
            );
            series
        })
    }

    /// The plot area's axes, in document order, each paired with the kind of axis it is.
    pub fn axes(&self) -> impl Iterator<Item = (AxisKind, &Axis)> {
        self.content.iter().filter_map(|item| match item {
            PlotAreaContent::CategoryAxis(axis) => Some((AxisKind::Category, axis)),
            PlotAreaContent::ValueAxis(axis) => Some((AxisKind::Value, axis)),
            PlotAreaContent::DateAxis(axis) => Some((AxisKind::Date, axis)),
            PlotAreaContent::SeriesAxis(axis) => Some((AxisKind::Series, axis)),
            _ => None,
        })
    }

    /// The plot area's axes, in document order, mutably.
    pub fn axes_mut(&mut self) -> impl Iterator<Item = &mut Axis> {
        self.content.iter_mut().filter_map(|item| match item {
            PlotAreaContent::CategoryAxis(axis)
            | PlotAreaContent::ValueAxis(axis)
            | PlotAreaContent::DateAxis(axis)
            | PlotAreaContent::SeriesAxis(axis) => Some(axis),
            _ => None,
        })
    }

    /// How many axes the plot area declares — the addressing space of
    /// [`axis_mut`](Self::axis_mut).
    #[must_use]
    pub fn axis_count(&self) -> usize {
        self.axes().count()
    }

    /// The `n`-th axis (document order), mutably — `None` when the plot area declares fewer.
    pub fn axis_mut(&mut self, n: usize) -> Option<&mut Axis> {
        self.axes_mut().nth(n)
    }

    /// The `n`-th plot's own data-label settings (`c:dLbls`) — the outermost of the three tiers —
    /// or `None` when the plot area holds fewer plots or that plot states none.
    ///
    /// Plots are numbered as [`chart_kinds`](Self::chart_kinds) numbers them, so a combo chart's
    /// two plots are 0 and 1 and each carries its own label defaults.
    #[must_use]
    pub fn plot_data_labels(&self, plot_idx: usize) -> Option<&DataLabels> {
        let item = self.plots().nth(plot_idx)?;
        with_plot!(item, |plot| plot.data_labels(), None)
    }

    /// Applies `spec` to the `n`-th plot's data labels, creating them at their schema rank if it had
    /// none. Answers `false`, changing nothing, when the plot area holds fewer plots.
    ///
    /// # Errors
    /// [`ChartDataError::DecorationNotAllowed`] when that plot type declares no `c:dLbls` — the two
    /// surface plots.
    pub fn set_plot_data_labels(
        &mut self,
        interner: &mut Interner,
        plot_idx: usize,
        spec: &DataLabelSpec,
    ) -> Result<bool, ChartDataError> {
        let Some(item) = self.plot_content_mut(plot_idx) else {
            return Ok(false);
        };
        with_plot!(
            item,
            |plot| plot.set_data_labels(interner, spec).map(|()| true),
            Ok(false)
        )
    }

    /// Suppresses every label of the `n`-th plot — a `c:dLbls` carrying `c:delete val="1"`.
    /// Answers `false` when the plot area holds fewer plots.
    ///
    /// # Errors
    /// As [`set_plot_data_labels`](Self::set_plot_data_labels).
    pub fn suppress_plot_data_labels(
        &mut self,
        interner: &mut Interner,
        plot_idx: usize,
    ) -> Result<bool, ChartDataError> {
        let Some(item) = self.plot_content_mut(plot_idx) else {
            return Ok(false);
        };
        with_plot!(
            item,
            |plot| {
                plot.data_labels_mut(interner)?.suppress_all(interner);
                Ok(true)
            },
            Ok(false)
        )
    }

    /// Removes the `n`-th plot's `c:dLbls` entirely, answering whether one was there.
    pub fn remove_plot_data_labels(&mut self, plot_idx: usize) -> bool {
        match self.plot_content_mut(plot_idx) {
            Some(item) => with_plot!(item, |plot| plot.remove_data_labels(), false),
            None => false,
        }
    }

    /// Every plot of the plot area, in document order — the numbering
    /// [`chart_kinds`](Self::chart_kinds) uses.
    fn plots(&self) -> impl Iterator<Item = &PlotAreaContent> {
        self.content
            .iter()
            .filter(|item| with_plot!(*item, |_plot| true, false))
    }

    /// The `n`-th plot's content slot, mutably.
    fn plot_content_mut(&mut self, plot_idx: usize) -> Option<&mut PlotAreaContent> {
        self.content
            .iter_mut()
            .filter(|item| with_plot!(&**item, |_plot| true, false))
            .nth(plot_idx)
    }

    /// The `n`-th series across every plot (document order), bound to the kind of plot that holds
    /// it — the write surface for its decoration. `None` when the chart draws fewer.
    ///
    /// A combo chart's plots have different series types, and the decoration a series may carry and
    /// where it is placed both follow from that, so a decoration edit has to be addressed through
    /// the owning plot rather than through the series alone.
    pub fn series_decoration_mut(&mut self, n: usize) -> Option<SeriesDecoration<'_>> {
        let mut remaining = n;
        for item in &mut self.content {
            let count = with_plot!(item, |plot| plot.series_count(), 0);
            if remaining < count {
                return with_plot!(item, |plot| plot.series_decoration_mut(remaining), None);
            }
            remaining -= count;
        }
        None
    }

    /// The data-label settings in force for one point of one series, merged across all three tiers
    /// — the point's `c:dLbl` over the series' `c:dLbls` over the owning plot's.
    ///
    /// `series_index` is global across the plot area's plots, matching
    /// [`all_series`](Self::all_series). `point_index` of `None` stops at the series tier. `None`
    /// when the chart draws fewer series.
    #[must_use]
    pub fn resolved_data_labels(
        &self,
        interner: &Interner,
        series_index: usize,
        point_index: Option<u32>,
    ) -> Option<DataLabelSettings> {
        let mut remaining = series_index;
        for item in &self.content {
            let count = with_plot!(item, |plot| plot.series_count(), 0);
            if remaining < count {
                return with_plot!(
                    item,
                    |plot| Some(plot.resolved_data_labels(interner, remaining, point_index)),
                    None
                );
            }
            remaining -= count;
        }
        None
    }

    /// The kind of plot that holds the `n`-th series across every plot, or `None` when the chart
    /// draws fewer.
    #[must_use]
    pub fn kind_of_series(&self, n: usize) -> Option<ChartKind> {
        let mut remaining = n;
        for item in &self.content {
            let count = with_plot!(item, |plot| plot.series_count(), 0);
            if remaining < count {
                return with_plot!(item, |plot| Some(plot.kind()), None);
            }
            remaining -= count;
        }
        None
    }

    /// The plot area's ordered content — the typed plots and axes interleaved with the nodes this
    /// tier keeps opaque.
    #[must_use]
    pub fn content(&self) -> &[PlotAreaContent] {
        &self.content
    }
}

/// One ordered child of a [`Chart`]: the plot area, the title, the legend, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartContent {
    /// The chart's heading (`c:title`).
    Title(ChartTitle),
    /// The plot area (`c:plotArea`).
    PlotArea(PlotArea),
    /// The key naming each series (`c:legend`).
    Legend(Legend),
    /// Any other child — `c:autoTitleDeleted`, `c:view3D`, `c:dispBlanksAs`, unknown — kept verbatim.
    Raw(RawNode),
}

/// `c:chart` (`CT_Chart`) — a chart's title, plot area and legend.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_CHART)]
pub struct Chart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "title", variant = Title, ty = ChartTitle),
        child(local = "plotArea", variant = PlotArea, ty = PlotArea),
        child(local = "legend", variant = Legend, ty = Legend)
    )]
    content: Vec<ChartContent>,
}

impl Chart {
    /// The chart's plot area (`c:plotArea`), or `None` if it declares none.
    #[must_use]
    pub fn plot_area(&self) -> Option<&PlotArea> {
        self.content.iter().find_map(|item| match item {
            ChartContent::PlotArea(plot_area) => Some(plot_area),
            _ => None,
        })
    }

    /// The chart's plot area, mutably.
    pub fn plot_area_mut(&mut self) -> Option<&mut PlotArea> {
        self.content.iter_mut().find_map(|item| match item {
            ChartContent::PlotArea(plot_area) => Some(plot_area),
            _ => None,
        })
    }

    /// The chart's heading (`c:title`), or `None` when it has none.
    #[must_use]
    pub fn title(&self) -> Option<&ChartTitle> {
        self.content.iter().find_map(|item| match item {
            ChartContent::Title(title) => Some(title),
            _ => None,
        })
    }

    /// The chart's heading text, or `None` when it has no title.
    #[must_use]
    pub fn title_text(&self) -> Option<String> {
        self.title().and_then(ChartTitle::text)
    }

    /// Sets the chart's heading, adding a `c:title` in its schema position if it had none, and
    /// clearing `c:autoTitleDeleted` so Office draws it. `None` removes the title.
    pub fn set_title(&mut self, interner: &mut Interner, text: Option<&str>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, ChartContent::Title(_)));
        match (existing, text) {
            (Some(index), Some(text)) => {
                let ChartContent::Title(title) = &mut self.content[index] else {
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
                    .insert(at, ChartContent::Title(ChartTitle::new(interner, text)));
                self.empty = false;
            }
            (None, None) => {}
        }
        // A chart that declares `c:autoTitleDeleted="1"` draws no title however many it carries, so
        // adding one has to clear the flag; removing one sets it, or Office invents a title of its own.
        self.set_scalar(
            interner,
            "autoTitleDeleted",
            if text.is_some() { "0" } else { "1" },
        );
    }

    /// The chart's legend (`c:legend`), or `None` when it has none.
    #[must_use]
    pub fn legend(&self) -> Option<&Legend> {
        self.content.iter().find_map(|item| match item {
            ChartContent::Legend(legend) => Some(legend),
            _ => None,
        })
    }

    /// The chart's legend, mutably.
    pub fn legend_mut(&mut self) -> Option<&mut Legend> {
        self.content.iter_mut().find_map(|item| match item {
            ChartContent::Legend(legend) => Some(legend),
            _ => None,
        })
    }

    /// Places the legend at `position`, adding a `c:legend` in its schema position if the chart had
    /// none. `None` removes the legend.
    pub fn set_legend(&mut self, interner: &mut Interner, position: Option<LegendPosition>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, ChartContent::Legend(_)));
        match (existing, position) {
            (Some(index), Some(position)) => {
                let ChartContent::Legend(legend) = &mut self.content[index] else {
                    unreachable!("the index was just found by matching this variant")
                };
                legend.set_position(interner, position);
            }
            (Some(index), None) => {
                self.content.remove(index);
            }
            (None, Some(position)) => {
                let at = self.insert_index(interner, "legend");
                self.content
                    .insert(at, ChartContent::Legend(Legend::new(interner, position)));
                self.empty = false;
            }
            (None, None) => {}
        }
    }

    /// What the chart draws in place of a blank value (`c:dispBlanksAs`).
    #[must_use]
    pub fn display_blanks_as(&self, interner: &Interner) -> Option<BlankDisplay> {
        self.scalar(interner, "dispBlanksAs")
            .and_then(BlankDisplay::from_wire)
    }

    /// Whether only visible worksheet cells are plotted (`c:plotVisOnly`).
    #[must_use]
    pub fn plots_visible_cells_only(&self, interner: &Interner) -> Option<bool> {
        self.scalar(interner, "plotVisOnly")
            .and_then(on_off::from_wire)
    }

    /// Whether Office is told **not** to invent a title of its own (`c:autoTitleDeleted`).
    #[must_use]
    pub fn auto_title_suppressed(&self, interner: &Interner) -> Option<bool> {
        self.scalar(interner, "autoTitleDeleted")
            .and_then(on_off::from_wire)
    }

    /// The `@val` of a raw scalar child of the chart.
    fn scalar(&self, interner: &Interner, local: &str) -> Option<&str> {
        let raw = self.content.iter().filter_map(|item| match item {
            ChartContent::Raw(node) => Some(node),
            _ => None,
        });
        raw_child_attr(raw, interner, local, "val")
    }

    /// Sets a raw scalar child's `@val` in place, or inserts the child in its schema position.
    fn set_scalar(&mut self, interner: &mut Interner, local: &str, value: &str) {
        for item in &mut self.content {
            if let ChartContent::Raw(RawNode::Element(element)) = item {
                if chart_local(&RawNode::Element(element.clone()), interner) == Some(local) {
                    set_attr(&mut element.attributes, interner, "val", value);
                    return;
                }
            }
        }
        let at = self.insert_index(interner, local);
        let element = chart_val_leaf(interner, local, value);
        self.content
            .insert(at, ChartContent::Raw(RawNode::Element(element)));
        self.empty = false;
    }

    /// Where a child named `local` belongs among the chart's current children.
    fn insert_index(&self, interner: &Interner, local: &str) -> usize {
        insert_position(
            CHART,
            self.content.iter().map(|item| match item {
                ChartContent::Title(_) => Some("title"),
                ChartContent::PlotArea(_) => Some("plotArea"),
                ChartContent::Legend(_) => Some("legend"),
                ChartContent::Raw(node) => chart_local(node, interner),
            }),
            local,
        )
    }
}

/// One ordered child of a [`ChartSpace`]: the chart, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSpaceContent {
    /// The chart (`c:chart`).
    Chart(Chart),
    /// Any other child — `c:date1904`, `c:lang`, `c:style`, `c:txPr`, `c:externalData`, unknown —
    /// kept verbatim.
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

    /// The chart, mutably.
    pub fn chart_mut(&mut self) -> Option<&mut Chart> {
        self.content.iter_mut().find_map(|item| match item {
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

    /// The kind of this chart's first plot, or `None` when it draws none. For a combo chart, see
    /// [`chart_kinds`](Self::chart_kinds).
    #[must_use]
    pub fn chart_kind(&self) -> Option<ChartKind> {
        self.plot_area().and_then(PlotArea::chart_kind)
    }

    /// The kind of every plot this chart draws, in order — one entry per plot element (a combo chart
    /// yields several), or empty when there is no plot area.
    #[must_use]
    pub fn chart_kinds(&self) -> Vec<ChartKind> {
        self.plot_area()
            .map(PlotArea::chart_kinds)
            .unwrap_or_default()
    }

    /// The chart's plot area, mutably — the write path down the spine.
    pub fn plot_area_mut(&mut self) -> Option<&mut PlotArea> {
        self.chart_mut().and_then(Chart::plot_area_mut)
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

    /// The `n`-th series across every plot, bound to the kind of plot that holds it — the write
    /// surface for its decoration. `None` when the chart draws fewer.
    pub fn series_decoration_mut(&mut self, n: usize) -> Option<SeriesDecoration<'_>> {
        self.plot_area_mut()?.series_decoration_mut(n)
    }

    /// The data-label settings in force for one point of one series, merged across all three tiers.
    /// See [`PlotArea::resolved_data_labels`].
    #[must_use]
    pub fn resolved_data_labels(
        &self,
        interner: &Interner,
        series_index: usize,
        point_index: Option<u32>,
    ) -> Option<DataLabelSettings> {
        self.plot_area()?
            .resolved_data_labels(interner, series_index, point_index)
    }

    /// The built-in chart style the part names (`c:style@val`, `ST_Style` — 1 to 48), or `None` when
    /// it names none. This is the palette and effect set Office draws an unstyled series with.
    #[must_use]
    pub fn style_id(&self, interner: &Interner) -> Option<u32> {
        self.scalar(interner, "style")
            .and_then(|value| value.trim().parse().ok())
    }

    /// Whether the chart area is drawn with rounded corners (`c:roundedCorners`).
    #[must_use]
    pub fn has_rounded_corners(&self, interner: &Interner) -> Option<bool> {
        self.scalar(interner, "roundedCorners")
            .and_then(on_off::from_wire)
    }

    /// The relationship id of the chart's embedded workbook (`c:externalData@r:id`), or `None` when
    /// the chart carries no workbook reference.
    #[must_use]
    pub fn external_data_rel_id(&self, interner: &Interner) -> Option<&str> {
        for item in &self.content {
            let ChartSpaceContent::Raw(RawNode::Element(element)) = item else {
                continue;
            };
            if chart_local(&RawNode::Element(element.clone()), interner) != Some("externalData") {
                continue;
            }
            return element
                .attributes
                .iter()
                .find(|attribute| interner.resolve(attribute.name.local) == "id")
                .and_then(|attribute| std::str::from_utf8(&attribute.value).ok());
        }
        None
    }

    /// Ensures the root declares the DrawingML namespace under the prefix `a`, adding the
    /// declaration if it is missing.
    ///
    /// A chart part's `c:` elements can hold DrawingML — a series' `c:spPr`, a title's `c:rich` — so
    /// anything this crate *writes* into a chart may introduce `a:` elements. Office-written parts
    /// declare the prefix already; a part that does not would otherwise gain unbound-prefix markup
    /// the moment a title or a fill is set. Returns whether a declaration was added.
    pub fn ensure_drawingml_namespace(&mut self, interner: &mut Interner) -> bool {
        let declared = self.attributes.iter().any(|attribute| {
            attribute
                .name
                .prefix
                .is_some_and(|prefix| interner.resolve(prefix) == "xmlns")
                && interner.resolve(attribute.name.local) == "a"
        });
        if declared {
            return false;
        }
        self.attributes
            .push(namespace_declaration(interner, "a", DML_MAIN.transitional));
        true
    }

    /// The `@val` of a raw scalar child of the chart space.
    fn scalar(&self, interner: &Interner, local: &str) -> Option<&str> {
        let raw = self.content.iter().filter_map(|item| match item {
            ChartSpaceContent::Raw(node) => Some(node),
            ChartSpaceContent::Chart(_) => None,
        });
        raw_child_attr(raw, interner, local, "val")
    }
}
