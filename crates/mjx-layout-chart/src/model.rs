//! The chart, read once into an owned model with no borrow into the part.
//!
//! # Why the engine parses the part rather than being handed a `ChartSpace`
//!
//! `mjx-chart`'s model borrows its interner, so a host that wanted to hand one over would have to
//! keep the parsed part alive for as long as the layout ran — which every one of the three host
//! surfaces does by handing a `&ChartSpace` to a closure and dropping it on the way out. Three hosts
//! each writing that closure is three chances to read a `c:grouping` differently.
//!
//! So the seam is **bytes**. `chart_part_bytes` is already public on `mjx_pptx::Presentation`,
//! `mjx_docx::Document` and `mjx_xlsx::Workbook`; [`ChartModel::read`] takes the bytes and returns an
//! owned model, and the parse is shared along with everything above it. That is also what lets
//! `tests/the_seam_holds.rs` refuse all three format crates outright: the engine has no reason to
//! open a package.
//!
//! # What is read, and what is deliberately not
//!
//! Everything that changes where something goes: the plots and their grouping, the series and their
//! values *by index*, the axes and their stated scaling, the legend, the data-label settings at all
//! three tiers, the trendlines and the error bars. Not read: `c:view3D` and the rest of the
//! three-dimensional block (this engine draws every plot flat — see [`crate::plot`]), `c:dTable`,
//! `c:pivotFmts`, and the text properties of anything, because chart text is measured through
//! [`crate::text::TextMetrics`] and the metric a host supplies is what decides its size.
//!
//! # Values are read by index, not by position
//!
//! `NumberCache::values` returns the values it found, in the order it found them; a cache is
//! *sparse* — a blank cell writes no `c:pt` at all — so the fifth value of a series is the one whose
//! `c:idx` is 4 and not the fifth element of that vector. Reading positionally is how a chart with
//! one blank cell draws every later point one category to the left, which is a defect that looks
//! exactly like a data error. This module's own readers index instead, and a `c:pt` with no
//! `c:idx` at all — which the schema forbids — falls back to its position, which is the reading that
//! keeps a nearly-right file drawing.

use mjx_chart::{
    Axis, AxisKind, AxisOrientation, AxisPosition, BarDirection, BarGrouping, CategoryData,
    ChartErrorBarData, ChartKind, ChartPointFormatData, ChartSpace, ChartTrendlineData,
    DataLabelSettings, LegendPosition, NumericData, PlotArea, PlotAreaContent, RadarStyle,
    ScatterStyle, Series, SeriesGrouping, TickLabelPosition, TickMark,
};
use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::{FromXml, Interner};

use crate::error::ChartLayoutError;

/// One chart, read out of its part and owning everything it says.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChartModel {
    /// The chart's title, or `None` when it has none or suppresses the automatic one.
    pub title: Option<String>,
    /// The plots the plot area holds, in document order. More than one is a combo chart.
    pub plots: Vec<PlotModel>,
    /// The axes, in document order.
    pub axes: Vec<AxisModel>,
    /// The legend, or `None` when the chart draws none.
    pub legend: Option<LegendModel>,
    /// What `c:dispBlanksAs` says to do with a gap in a series.
    pub blanks: BlankHandling,
    /// The `c:style@val` the file states, kept because a host may want to report it. Nothing in this
    /// engine reads it: a style id names a gallery entry this project does not carry, and choosing
    /// colours from one we invented would override the theme the document actually states.
    pub style_id: Option<u32>,
}

/// What a chart does with a category whose value is missing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BlankHandling {
    /// `c:dispBlanksAs val="gap"` — the default. The line breaks and the bar is absent.
    #[default]
    Gap,
    /// `val="zero"` — a blank plots as zero.
    Zero,
    /// `val="span"` — a line joins the points either side of the gap.
    Span,
}

/// One plot of a plot area — a `c:barChart`, a `c:lineChart`, and so on.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotModel {
    /// Which element it was read from.
    pub kind: ChartKind,
    /// How its series stack, if they do.
    pub grouping: Grouping,
    /// Which way a bar plot's bars run. `Column` for everything that is not a bar plot.
    pub direction: BarDirection,
    /// `c:gapWidth`, as a percentage of the bar width. Office's default is 150.
    pub gap_width: u32,
    /// `c:overlap`, as a percentage: negative separates clustered bars, 100 stacks them.
    pub overlap: i32,
    /// `c:holeSize` of a doughnut, as a percentage of the outer radius.
    pub hole_size: u32,
    /// `c:firstSliceAng`, in degrees clockwise from twelve o'clock.
    pub first_slice_angle: u32,
    /// How a scatter plot joins its points.
    pub scatter_style: ScatterStyle,
    /// How a radar plot draws.
    pub radar_style: RadarStyle,
    /// `c:bubbleScale`, as a percentage.
    pub bubble_scale: u32,
    /// Whether every point takes its own colour rather than its series' (`c:varyColors`), which is
    /// what a pie chart does by default.
    pub vary_colours: bool,
    /// The axes this plot is measured against (`c:axId`), in the order the file names them.
    pub axis_ids: Vec<u32>,
    /// The plot-tier data-label settings every series of it inherits.
    pub labels: DataLabelSettings,
    /// Its series, in document order.
    pub series: Vec<SeriesModel>,
}

/// How a plot's series stack. `c:grouping` is spelled two different ways by the schema — bar plots
/// admit `clustered`, line and area plots do not — and this is the union of both, so that a stacking
/// rule is written once for every family that has one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Grouping {
    /// Side by side (bar) or overlaid (line, area) — the default.
    #[default]
    Standard,
    /// Bars side by side within a category. Bar plots only.
    Clustered,
    /// Each series stacked on the one before.
    Stacked,
    /// Stacked, and each category scaled to fill the axis.
    PercentStacked,
}

impl Grouping {
    /// Whether the plot's marks accumulate onto each other.
    #[must_use]
    pub fn is_stacked(self) -> bool {
        matches!(self, Self::Stacked | Self::PercentStacked)
    }
}

/// One series, with its values addressed by point index.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SeriesModel {
    /// `c:idx` — the identity the per-point overrides are anchored by.
    pub index: u32,
    /// `c:order` — where it sits in the legend and in a stack.
    pub order: u32,
    /// Its name, or `None` when it has none.
    pub name: Option<String>,
    /// The category labels, by point index; an entry is empty for a category the cache does not
    /// name.
    pub categories: Vec<String>,
    /// The values, by point index. `None` is a blank, which [`BlankHandling`] decides the fate of.
    pub values: Vec<Option<f64>>,
    /// A bubble plot's third dimension, by point index.
    pub bubble_sizes: Vec<Option<f64>>,
    /// The fill the file states for the whole series, or `None` when it states none — in which case
    /// the series takes an accent colour from the document's own theme. **Never a literal colour
    /// invented here**; see [`crate::palette`].
    pub fill: Option<FillSpec>,
    /// The outline the file states for the whole series.
    pub line: Option<LineSpec>,
    /// The points that are formatted differently from the rest (`c:dPt`).
    pub point_formats: Vec<ChartPointFormatData>,
    /// The label settings in force for the series, already merged over its plot's.
    pub labels: DataLabelSettings,
    /// The label settings for the points that override the series' — index and settings.
    pub point_labels: Vec<(u32, DataLabelSettings)>,
    /// The curves fitted through it.
    pub trendlines: Vec<ChartTrendlineData>,
    /// The uncertainty drawn around its points.
    pub error_bars: Vec<ChartErrorBarData>,
}

impl SeriesModel {
    /// How many points the series has — the longest of the three parallel runs, so a series whose
    /// categories outrun its values still places every value it has.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.values
            .len()
            .max(self.categories.len())
            .max(self.bubble_sizes.len())
    }

    /// The value at `point`, or `None` for a blank or a point past the end.
    #[must_use]
    pub fn value(&self, point: usize) -> Option<f64> {
        self.values.get(point).copied().flatten()
    }

    /// The value at `point` with `blanks` applied, so a caller that must have a number gets one.
    #[must_use]
    pub fn plotted_value(&self, point: usize, blanks: BlankHandling) -> Option<f64> {
        match (self.value(point), blanks) {
            (Some(value), _) => Some(value),
            (None, BlankHandling::Zero) => Some(0.0),
            (None, _) => None,
        }
    }
}

/// One axis, read.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisModel {
    /// Which element it was read from.
    pub kind: AxisKind,
    /// `c:axId`.
    pub id: Option<u32>,
    /// `c:crossAx` — the axis this one is drawn against.
    pub cross_axis_id: Option<u32>,
    /// Where it sits, or `None` when the file leaves the position to the reader.
    pub position: Option<AxisPosition>,
    /// Whether the file says to draw nothing at all for it (`c:delete`).
    pub suppressed: bool,
    /// Which way it runs.
    pub reversed: bool,
    /// `c:scaling > c:min`.
    pub minimum: Option<f64>,
    /// `c:scaling > c:max`.
    pub maximum: Option<f64>,
    /// `c:scaling > c:logBase`.
    pub logarithm_base: Option<f64>,
    /// `c:majorUnit`.
    pub major_unit: Option<f64>,
    /// `c:minorUnit`.
    pub minor_unit: Option<f64>,
    /// Its title, or `None`.
    pub title: Option<String>,
    /// Whether it rules major gridlines across the plot.
    pub major_gridlines: bool,
    /// Whether it rules minor gridlines.
    pub minor_gridlines: bool,
    /// How its major tick marks are drawn.
    pub major_tick_mark: TickMark,
    /// How its minor tick marks are drawn.
    pub minor_tick_mark: TickMark,
    /// Where its tick labels sit.
    pub tick_label_position: TickLabelPosition,
    /// How many labels are skipped between drawn ones — `1` draws all of them.
    pub tick_label_skip: u32,
    /// Its number format, or `None` when it inherits one.
    pub number_format: Option<String>,
    /// Whether a category axis' ticks fall between categories rather than on them.
    pub crosses_between_categories: bool,
}

/// A chart's legend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegendModel {
    /// Where it sits. Office's default when a `c:legend` states none is the right-hand side.
    pub position: LegendPosition,
    /// Whether it is drawn over the plot area rather than beside it.
    pub overlays_plot: bool,
}

impl ChartModel {
    /// Reads a chart part's bytes into a model.
    ///
    /// # Errors
    /// [`ChartLayoutError::Malformed`] for bytes that are not well-formed XML,
    /// [`ChartLayoutError::NotReadable`] for a root that is not a readable `c:chartSpace`, and
    /// [`ChartLayoutError::NoChart`] for a chart space with no `c:chart` inside it.
    pub fn read(part: &[u8]) -> Result<Self, ChartLayoutError> {
        let document = mjx_xml::fidelity::parse(part)?;
        let space = ChartSpace::from_xml(&document.root, &document.interner)?;
        Self::from_space(&space, &document.interner)
    }

    /// Reads an already-parsed chart space, for a caller that has one.
    ///
    /// # Errors
    /// [`ChartLayoutError::NoChart`] for a chart space with no `c:chart` inside it.
    pub fn from_space(space: &ChartSpace, interner: &Interner) -> Result<Self, ChartLayoutError> {
        let chart = space.chart().ok_or(ChartLayoutError::NoChart)?;
        let title = if chart.auto_title_suppressed(interner) == Some(true) {
            None
        } else {
            chart.title_text().filter(|text| !text.is_empty())
        };
        let legend = chart.legend().map(|legend| LegendModel {
            position: legend.position(interner).unwrap_or(LegendPosition::Right),
            overlays_plot: legend.overlays_plot(interner).unwrap_or(false),
        });
        let blanks = match chart.display_blanks_as(interner) {
            Some(mjx_chart::BlankDisplay::Zero) => BlankHandling::Zero,
            Some(mjx_chart::BlankDisplay::Span) => BlankHandling::Span,
            _ => BlankHandling::Gap,
        };
        let (plots, axes) = match chart.plot_area() {
            Some(area) => (read_plots(space, area, interner), read_axes(area, interner)),
            None => (Vec::new(), Vec::new()),
        };
        Ok(Self {
            title,
            plots,
            axes,
            legend,
            blanks,
            style_id: space.style_id(interner),
        })
    }

    /// Every series of every plot, in the order [`mjx_chart::PlotArea::all_series`] counts them —
    /// which is the order a legend lists them and the order the per-series colours are handed out.
    pub fn all_series(&self) -> impl Iterator<Item = (usize, &SeriesModel)> {
        self.plots
            .iter()
            .enumerate()
            .flat_map(|(plot, model)| model.series.iter().map(move |series| (plot, series)))
    }

    /// How many series the chart draws in total.
    #[must_use]
    pub fn series_count(&self) -> usize {
        self.plots.iter().map(|plot| plot.series.len()).sum()
    }

    /// The first axis of `kind` the chart declares.
    #[must_use]
    pub fn axis_for_kind(&self, kind: AxisKind) -> Option<&AxisModel> {
        self.axes.iter().find(|axis| axis.kind == kind)
    }

    /// The axis of `kind` a plot is measured against, preferring one the plot's own `c:axId` names.
    #[must_use]
    pub fn axis_for(&self, plot: &PlotModel, kind: AxisKind) -> Option<&AxisModel> {
        self.axes
            .iter()
            .find(|axis| axis.kind == kind && axis.id.is_some_and(|id| plot.axis_ids.contains(&id)))
            .or_else(|| self.axes.iter().find(|axis| axis.kind == kind))
    }

    /// The category labels the chart draws along its category axis: the longest run any series of
    /// it carries, so a chart whose second series names more categories than its first still labels
    /// all of them.
    #[must_use]
    pub fn categories(&self) -> Vec<String> {
        let mut longest: &[String] = &[];
        for (_, series) in self.all_series() {
            if series.categories.len() > longest.len() {
                longest = &series.categories;
            }
        }
        if longest.is_empty() {
            // A chart with no category cache still has categories: Office numbers them from one.
            let points = self
                .all_series()
                .map(|(_, series)| series.point_count())
                .max()
                .unwrap_or(0);
            return (1..=points).map(|n| n.to_string()).collect();
        }
        longest.to_vec()
    }
}

/// Reads every plot of a plot area.
fn read_plots(space: &ChartSpace, area: &PlotArea, interner: &Interner) -> Vec<PlotModel> {
    let mut plots = Vec::new();
    let mut global = 0usize;
    for item in area.content() {
        let Some(kind) = plot_kind(item) else {
            continue;
        };
        let mut model = PlotModel {
            kind,
            grouping: Grouping::default(),
            direction: BarDirection::Column,
            gap_width: DEFAULT_GAP_WIDTH,
            overlap: 0,
            hole_size: DEFAULT_HOLE_SIZE,
            first_slice_angle: 0,
            scatter_style: ScatterStyle::LineWithMarkers,
            radar_style: RadarStyle::Markers,
            bubble_scale: 100,
            vary_colours: false,
            axis_ids: Vec::new(),
            labels: DataLabelSettings::default(),
            series: Vec::new(),
        };
        match item {
            PlotAreaContent::Bar(plot) => {
                model.grouping = bar_grouping(plot.grouping(interner));
                model.direction = plot.direction(interner).unwrap_or(BarDirection::Column);
                model.gap_width = plot.gap_width(interner).unwrap_or(DEFAULT_GAP_WIDTH);
                model.overlap = plot.overlap(interner).unwrap_or_else(|| {
                    // `c:overlap` defaults to 0 in the schema, but a stacked plot with no overlap
                    // would draw its stack as a cluster. Office writes 100 for a stacked plot and
                    // this engine assumes it when the file does not say.
                    if model.grouping.is_stacked() {
                        100
                    } else {
                        0
                    }
                });
            }
            PlotAreaContent::Bar3D(plot) => {
                model.grouping = bar_grouping(plot.grouping(interner));
                model.direction = plot.direction(interner).unwrap_or(BarDirection::Column);
                model.overlap = if model.grouping.is_stacked() { 100 } else { 0 };
            }
            PlotAreaContent::Line(plot) => {
                model.grouping = series_grouping(plot.grouping(interner))
            }
            PlotAreaContent::Line3D(plot) => {
                model.grouping = series_grouping(plot.grouping(interner));
            }
            PlotAreaContent::Area(plot) => {
                model.grouping = series_grouping(plot.grouping(interner));
            }
            PlotAreaContent::Area3D(plot) => {
                model.grouping = series_grouping(plot.grouping(interner));
            }
            PlotAreaContent::Scatter(plot) => {
                model.scatter_style = plot
                    .scatter_style(interner)
                    .unwrap_or(ScatterStyle::LineWithMarkers);
            }
            PlotAreaContent::Doughnut(plot) => {
                model.hole_size = plot.hole_size(interner).unwrap_or(DEFAULT_HOLE_SIZE);
                model.first_slice_angle = plot.first_slice_angle(interner).unwrap_or(0);
            }
            PlotAreaContent::Radar(plot) => {
                model.radar_style = plot.radar_style(interner).unwrap_or(RadarStyle::Markers);
            }
            PlotAreaContent::Bubble(plot) => {
                model.bubble_scale = plot.bubble_scale(interner).unwrap_or(100).clamp(0, 300);
            }
            _ => {}
        }
        model.vary_colours = plot_vary_colours(item, interner);
        model.axis_ids = plot_axis_ids(item, interner);
        model.labels = plot_labels(item, interner);
        model.series = plot_series(item)
            .enumerate()
            .map(|(position, series)| {
                let at = global + position;
                read_series(space, series, at, position, kind, &model.labels, interner)
            })
            .collect();
        global += model.series.len();
        plots.push(model);
    }
    plots
}

/// Office's default gap between clustered bars, as a percentage of one bar's width.
const DEFAULT_GAP_WIDTH: u32 = 150;
/// Office's default doughnut hole, as a percentage of the outer radius.
const DEFAULT_HOLE_SIZE: u32 = 50;

/// The kind of plot a plot-area child is, or `None` for an axis or an opaque node.
fn plot_kind(item: &PlotAreaContent) -> Option<ChartKind> {
    Some(match item {
        PlotAreaContent::Bar(_) => ChartKind::Bar,
        PlotAreaContent::Bar3D(_) => ChartKind::Bar3D,
        PlotAreaContent::Line(_) => ChartKind::Line,
        PlotAreaContent::Line3D(_) => ChartKind::Line3D,
        PlotAreaContent::Pie(_) => ChartKind::Pie,
        PlotAreaContent::Pie3D(_) => ChartKind::Pie3D,
        PlotAreaContent::OfPie(_) => ChartKind::OfPie,
        PlotAreaContent::Area(_) => ChartKind::Area,
        PlotAreaContent::Area3D(_) => ChartKind::Area3D,
        PlotAreaContent::Scatter(_) => ChartKind::Scatter,
        PlotAreaContent::Doughnut(_) => ChartKind::Doughnut,
        PlotAreaContent::Radar(_) => ChartKind::Radar,
        PlotAreaContent::Bubble(_) => ChartKind::Bubble,
        PlotAreaContent::Stock(_) => ChartKind::Stock,
        PlotAreaContent::Surface(_) => ChartKind::Surface,
        PlotAreaContent::Surface3D(_) => ChartKind::Surface3D,
        _ => return None,
    })
}

/// Runs one expression against whichever of the sixteen plot types a plot-area child holds.
///
/// The same shape as `mjx-chart`'s own `with_plot!`, which is private to that crate. Writing it
/// again here is better than the alternative — sixteen `match` arms in each of the four walks
/// below — and it is the only thing this module duplicates.
macro_rules! on_plot {
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

/// Whether a plot varies its colours per point.
fn plot_vary_colours(item: &PlotAreaContent, interner: &Interner) -> bool {
    on_plot!(
        item,
        |plot| plot.vary_colors(interner).unwrap_or(false),
        false
    )
}

/// The `c:axId`s a plot names.
fn plot_axis_ids(item: &PlotAreaContent, interner: &Interner) -> Vec<u32> {
    on_plot!(item, |plot| plot.axis_ids(interner), Vec::new())
}

/// A plot's own data-label tier.
fn plot_labels(item: &PlotAreaContent, interner: &Interner) -> DataLabelSettings {
    on_plot!(
        item,
        |plot| plot.plot_label_settings(interner),
        DataLabelSettings::default()
    )
}

/// A plot's series.
fn plot_series(item: &PlotAreaContent) -> Box<dyn Iterator<Item = &Series> + '_> {
    on_plot!(
        item,
        |plot| Box::new(plot.series()) as Box<dyn Iterator<Item = &Series>>,
        Box::new(std::iter::empty())
    )
}

/// Reads one series into its model.
fn read_series(
    space: &ChartSpace,
    series: &Series,
    global_index: usize,
    position: usize,
    kind: ChartKind,
    plot_labels: &DataLabelSettings,
    interner: &Interner,
) -> SeriesModel {
    let position = u32::try_from(position).unwrap_or(u32::MAX);
    let (categories, values) = if kind.uses_xy_data() {
        (
            series
                .x_data()
                .map(CategoryData::labels)
                .unwrap_or_default(),
            series
                .y_data()
                .map(|data| read_numbers(data, interner))
                .unwrap_or_default(),
        )
    } else {
        (
            series
                .categories()
                .map(CategoryData::labels)
                .unwrap_or_default(),
            series
                .values()
                .map(|data| read_numbers(data, interner))
                .unwrap_or_default(),
        )
    };
    let labels = series.resolved_data_labels(interner, plot_labels, None);
    let point_labels = series
        .data_labels()
        .map(|dlbls| {
            dlbls
                .labels()
                .filter_map(|label| {
                    let index = label.index(interner)?;
                    Some((
                        index,
                        series.resolved_data_labels(interner, plot_labels, Some(index)),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    SeriesModel {
        index: series.index(interner).unwrap_or(position),
        order: series.order(interner).unwrap_or(position),
        name: series.name().filter(|name| !name.is_empty()),
        categories,
        values,
        bubble_sizes: series
            .bubble_sizes()
            .map(|data| read_numbers(data, interner))
            .unwrap_or_default(),
        fill: series.fill(interner),
        line: series.line(interner),
        // The three decoration reads go through `mjx-chart`'s own `chart_ops`, which is what both
        // format surfaces already call. `ChartPointFormatData` and its two neighbours are
        // `#[non_exhaustive]` precisely so that they are built beside their declaration and nowhere
        // else — so this crate could not construct one even if a second reading were wanted.
        point_formats: mjx_chart::chart_ops::point_formats(space, interner, global_index)
            .unwrap_or_default(),
        labels,
        point_labels,
        trendlines: mjx_chart::chart_ops::trendlines(space, interner, global_index)
            .unwrap_or_default(),
        error_bars: mjx_chart::chart_ops::error_bars(space, interner, global_index)
            .unwrap_or_default(),
    }
}

/// Reads a numeric source into a vector indexed by `c:idx`, so a blank cell is a hole rather than a
/// shift.
fn read_numbers(data: &NumericData, interner: &Interner) -> Vec<Option<f64>> {
    let points: Vec<_> = data
        .reference()
        .and_then(|reference| reference.cache())
        .map(|cache| cache.points().collect::<Vec<_>>())
        .or_else(|| data.literal().map(|cache| cache.points().collect()))
        .unwrap_or_default();
    indexed(points.iter().enumerate().map(|(position, point)| {
        // A `c:pt` without a `c:idx` is malformed — the attribute is required — and falling back to
        // its position is the reading that keeps a nearly-right file drawing.
        let index = point
            .index(interner)
            .unwrap_or_else(|| u32::try_from(position).unwrap_or(u32::MAX));
        (index, point.value_f64().filter(|value| value.is_finite()))
    }))
}

/// Collects `(index, value)` pairs into a dense vector, dropping an index no chart could address.
fn indexed<T: Clone + Default>(pairs: impl Iterator<Item = (u32, T)>) -> Vec<T> {
    let pairs: Vec<_> = pairs.filter(|(index, _)| *index < MAXIMUM_POINTS).collect();
    let Some(highest) = pairs.iter().map(|(index, _)| *index).max() else {
        return Vec::new();
    };
    let mut out = vec![T::default(); highest as usize + 1];
    for (index, value) in pairs {
        if let Some(slot) = out.get_mut(index as usize) {
            *slot = value;
        }
    }
    out
}

/// The most points this engine will address in one series.
///
/// Excel's own limit is 32,000 points per series (§ the workbook limits table), and a `c:idx` is an
/// unsigned int, so a file may name point four billion. Allocating for one is how a malformed part
/// becomes an out-of-memory kill; refusing to address it is how it becomes a chart missing a point.
const MAXIMUM_POINTS: u32 = 32_000;

/// Reads every axis of a plot area.
fn read_axes(area: &PlotArea, interner: &Interner) -> Vec<AxisModel> {
    area.axes()
        .map(|(kind, axis)| read_axis(kind, axis, interner))
        .collect()
}

/// Reads one axis.
fn read_axis(kind: AxisKind, axis: &Axis, interner: &Interner) -> AxisModel {
    let scaling = axis.scaling();
    AxisModel {
        kind,
        id: axis.axis_id(interner),
        cross_axis_id: axis.cross_axis_id(interner),
        position: axis.position(interner),
        suppressed: axis.is_suppressed(interner).unwrap_or(false),
        reversed: scaling
            .and_then(|scaling| scaling.orientation(interner))
            .is_some_and(|orientation| orientation == AxisOrientation::MaximumToMinimum),
        minimum: scaling.and_then(|scaling| scaling.minimum(interner)),
        maximum: scaling.and_then(|scaling| scaling.maximum(interner)),
        logarithm_base: scaling.and_then(|scaling| scaling.logarithm_base(interner)),
        major_unit: axis.major_unit(interner),
        minor_unit: axis.minor_unit(interner),
        title: axis.title_text().filter(|text| !text.is_empty()),
        major_gridlines: axis.has_major_gridlines(),
        minor_gridlines: axis.has_minor_gridlines(),
        major_tick_mark: axis.major_tick_mark(interner).unwrap_or(TickMark::Outside),
        minor_tick_mark: axis.minor_tick_mark(interner).unwrap_or(TickMark::None),
        tick_label_position: axis
            .tick_label_position(interner)
            .unwrap_or(TickLabelPosition::NextToAxis),
        tick_label_skip: axis.tick_label_skip(interner).unwrap_or(1).max(1),
        number_format: axis.number_format(interner).map(str::to_owned),
        crosses_between_categories: axis
            .crosses_between_categories(interner)
            .unwrap_or(matches!(kind, AxisKind::Category | AxisKind::Date)),
    }
}

/// Maps a bar plot's `c:grouping` onto the shared one.
fn bar_grouping(grouping: Option<BarGrouping>) -> Grouping {
    match grouping {
        Some(BarGrouping::Stacked) => Grouping::Stacked,
        Some(BarGrouping::PercentStacked) => Grouping::PercentStacked,
        Some(BarGrouping::Standard) => Grouping::Standard,
        // `c:grouping` is optional and its schema default is `clustered`.
        Some(BarGrouping::Clustered) | None => Grouping::Clustered,
    }
}

/// Maps a line or area plot's `c:grouping` onto the shared one.
fn series_grouping(grouping: Option<SeriesGrouping>) -> Grouping {
    match grouping {
        Some(SeriesGrouping::Stacked) => Grouping::Stacked,
        Some(SeriesGrouping::PercentStacked) => Grouping::PercentStacked,
        Some(SeriesGrouping::Standard) | None => Grouping::Standard,
    }
}
