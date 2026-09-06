//! Charts: the description you author, and the structures you read back.
//!
//! [`ChartData`] is the authoring side — a kind, categories, series, a title, a legend — and is what
//! `Deck.add_chart` takes. Everything else here is what the reading side hands back: the series,
//! axes, legend, point formats, trendlines and error bars a chart part already holds.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use mjx_ooxml as ooxml;

use crate::enums::{
    AxisKind, AxisOrientation, AxisPosition, ChartKind, DataLabelPosition, ErrorBarDirection,
    ErrorBarType, ErrorValueType, LegendPosition, TickLabelPosition, TickMark, TrendlineKind,
    WrapText,
};
use crate::errors::to_py_err;
use crate::paint::{FillSpec, LineSpec};

value_class! {
    /// A chart to author: its kind, its categories, its series, and the decoration it starts with.
    ChartData(ooxml::ChartData), derive(PartialEq);

    /// What data labels to show, and where.
    DataLabelSpec(ooxml::DataLabelSpec), derive(PartialEq);

    /// What data labels a chart part already states, at one tier of its hierarchy.
    DataLabelSettings(ooxml::DataLabelSettings), derive(PartialEq);

    /// Which tier of a chart's data-label hierarchy a call is about.
    ChartLabelScope(ooxml::ChartLabelScope), derive(Copy, PartialEq, Eq);

    /// A trendline to add to a series.
    TrendlineSpec(ooxml::TrendlineSpec), derive(PartialEq);

    /// Error bars to add to a series.
    ErrorBarSpec(ooxml::ErrorBarSpec), derive(PartialEq);

    /// One series as the chart part states it: its name, its categories and its values.
    ChartSeriesData(ooxml::ChartSeriesData), derive(PartialEq);

    /// One axis as the chart part states it.
    ChartAxisData(ooxml::ChartAxisData), derive(PartialEq);

    /// The legend as the chart part states it.
    ChartLegendData(ooxml::ChartLegendData), derive(PartialEq);

    /// One point's own formatting, overriding its series'.
    ChartPointFormatData(ooxml::ChartPointFormatData), derive(PartialEq);

    /// One trendline as the chart part states it.
    ChartTrendlineData(ooxml::ChartTrendlineData), derive(PartialEq);

    /// One set of error bars as the chart part states it.
    ChartErrorBarData(ooxml::ChartErrorBarData), derive(PartialEq);

    /// A chart's backing workbook: which shape holds the chart, where the workbook is, and whether
    /// it lies outside the package.
    ChartWorkbook(ooxml::ChartWorkbook), derive(PartialEq, Eq);

    /// A decoration that names a data point the series no longer has.
    DanglingPointReference(ooxml::DanglingPointReference), derive(Copy, PartialEq, Eq);

    /// A Word chart's backing workbook: which drawing holds the chart, where the workbook is, and
    /// whether it lies outside the package. The Word counterpart of [`ChartWorkbook`], which names
    /// a slide shape instead of a drawing id.
    DocumentChartWorkbook(ooxml::DocumentChartWorkbook), derive(PartialEq, Eq);

    /// How text flows around a floating Word chart (`Document.add_floating_chart`).
    ChartWrap(ooxml::ChartWrap), derive(Copy, PartialEq, Eq);

    /// Where one series says its data lives — the formula beside each cache, as the file wrote it.
    ChartSeriesReferences(ooxml::ChartSeriesReferences), derive(PartialEq, Eq);

    /// Where one series of an Excel range chart takes its data from (`Workbook.add_range_chart`).
    ChartRangeSeries(ooxml::ChartRangeSeries), derive(PartialEq, Eq);

    /// An Excel chart's backing workbook: which anchor on which tab holds the chart, where the
    /// workbook is, and whether it lies outside the package.
    SheetChartWorkbookInfo(ooxml::SheetChartWorkbookInfo), derive(PartialEq, Eq);

    /// One series' cache set beside what its cells actually say, with each named.
    ChartSeriesFreshnessInfo(ooxml::ChartSeriesFreshnessInfo), derive(PartialEq);

    /// One cell a chart's formula reaches, and where it sits in that reference.
    RangeCellInfo(ooxml::RangeCellInfo), derive(PartialEq);

    /// A chart's formula, resolved against a workbook's cells.
    ResolvedRangeInfo(ooxml::ResolvedRangeInfo), derive(PartialEq);
}

// ---------------------------------------------------------------------------------------------
// The Excel chart surface (MJXOFF-111)
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl ChartSeriesReferences {
    /// The cell the series' name comes from, or `None` when the name is a literal.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// The cells its category labels come from, or `None` when they are a literal.
    #[getter]
    fn categories(&self) -> Option<&str> {
        self.0.categories.as_deref()
    }

    /// The cells its values come from, or `None` when they are a literal.
    #[getter]
    fn values(&self) -> Option<&str> {
        self.0.values.as_deref()
    }
}

#[pymethods]
impl ChartRangeSeries {
    /// A series taking its values from `values` and its name from the literal `name`.
    #[new]
    fn new(name: &str, values: &str) -> Self {
        Self(ooxml::ChartRangeSeries::new(name, values))
    }

    /// The same series, taking its name from the cell `reference` names instead.
    fn named_by_cell(&self, reference: &str) -> Self {
        Self(self.0.clone().named_by_cell(reference))
    }

    /// The cell the series takes its name from, or `None` for a literal name.
    #[getter]
    fn name_cell(&self) -> Option<&str> {
        self.0.name_cell.as_deref()
    }

    /// The name to use when no cell names it, or when the cell it names holds nothing.
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }

    /// The cells the series' values come from.
    #[getter]
    fn values(&self) -> &str {
        &self.0.values
    }
}

#[pymethods]
impl SheetChartWorkbookInfo {
    /// The tab the chart is anchored on.
    #[getter]
    fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// The anchor that frames it, in the drawing part's paint order.
    #[getter]
    fn anchor(&self) -> u32 {
        self.0.anchor
    }

    /// Where the workbook is — a part name inside the package, or a URI outside it.
    #[getter]
    fn target(&self) -> &str {
        &self.0.target
    }

    /// Whether the workbook lies outside the package.
    #[getter]
    fn external(&self) -> bool {
        self.0.external
    }
}

#[pymethods]
impl ChartSeriesFreshnessInfo {
    /// Which series this is, in the order `Workbook.chart_series` reports them.
    #[getter]
    fn series_index(&self) -> u32 {
        self.0.series_index
    }

    /// What the chart draws today — its caches.
    #[getter]
    fn cached(&self) -> ChartSeriesData {
        ChartSeriesData(self.0.cached.clone())
    }

    /// The formula of each of the series' sources, as the file wrote them.
    #[getter]
    fn references(&self) -> ChartSeriesReferences {
        ChartSeriesReferences(self.0.references.clone())
    }

    /// What the cells say, for the sources that are references and resolved.
    ///
    /// `clippy::wrong_self_convention` reads `from_*` as a constructor; here it is the *identity*
    /// projection of `mjx_ooxml::ChartSeriesFreshnessInfo::from_cells`, and this binding renames
    /// nothing (see the crate's own doc comment). Renaming it to satisfy a lint would put a third
    /// spelling of one field in front of a Python caller.
    #[allow(clippy::wrong_self_convention)]
    #[getter]
    fn from_cells(&self) -> ChartSeriesData {
        ChartSeriesData(self.0.from_cells.clone())
    }

    /// Why the values reference did not resolve, in words, or `None` when it did.
    #[getter]
    fn values_problem(&self) -> Option<&str> {
        self.0.values_problem.as_deref()
    }

    /// Why the categories reference did not resolve, in words, or `None`.
    #[getter]
    fn categories_problem(&self) -> Option<&str> {
        self.0.categories_problem.as_deref()
    }

    /// Whether the cached values and the cells agree. `None` means *cannot say* — the values are a
    /// literal, or the reference did not resolve.
    #[getter]
    fn values_agree(&self) -> Option<bool> {
        self.0.values_agree
    }

    /// Whether the cached category labels and the cells agree. `None` as above.
    #[getter]
    fn categories_agree(&self) -> Option<bool> {
        self.0.categories_agree
    }
}

#[pymethods]
impl RangeCellInfo {
    /// The cell's zero-based position within the whole reference — the index a cache uses.
    #[getter]
    fn offset(&self) -> u64 {
        self.0.offset
    }

    /// The tab the cell is on.
    #[getter]
    fn sheet(&self) -> u32 {
        self.0.sheet
    }

    /// Where on that tab, in A1 text.
    #[getter]
    fn reference(&self) -> &str {
        &self.0.reference
    }

    /// The cell's value as a number, or `None` for one that is not a number.
    #[getter]
    fn number(&self) -> Option<f64> {
        self.0.number
    }

    /// What a category axis would show for the cell.
    #[getter]
    fn label(&self) -> &str {
        &self.0.label
    }
}

#[pymethods]
impl ResolvedRangeInfo {
    /// The reference exactly as the chart wrote it.
    #[getter]
    fn reference(&self) -> &str {
        &self.0.reference
    }

    /// How many cells the reference addresses, blanks included.
    #[getter]
    fn addressed_cells(&self) -> u64 {
        self.0.addressed_cells
    }

    /// Whether every area of the reference resolved.
    #[getter]
    fn fully_resolved(&self) -> bool {
        self.0.fully_resolved
    }

    /// Why the first area that failed did, in words, or `None` when none did.
    #[getter]
    fn problem(&self) -> Option<&str> {
        self.0.problem.as_deref()
    }

    /// The cells that hold something, in reference order. A blank cell is absent rather than zero.
    #[getter]
    fn cells(&self) -> Vec<RangeCellInfo> {
        self.0.cells.iter().cloned().map(RangeCellInfo).collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl ChartData {
    /// A chart of the given kind, with nothing in it yet.
    #[new]
    fn new(kind: ChartKind) -> Self {
        Self(ooxml::ChartData::new(kind.into()))
    }

    /// This chart with the given category labels, replacing any it had.
    fn categories(&self, categories: Vec<String>) -> Self {
        Self(self.0.clone().categories(categories))
    }

    /// This chart with one more series.
    fn series(&self, name: &str, values: Vec<f64>) -> Self {
        Self(self.0.clone().series(name.to_owned(), values))
    }

    /// This chart with the given title.
    fn title(&self, title: &str) -> Self {
        Self(self.0.clone().title(title.to_owned()))
    }

    /// This chart with a legend in the given position.
    fn legend(&self, position: LegendPosition) -> Self {
        Self(self.0.clone().legend(position.into()))
    }

    /// This chart with the given data labels on every series.
    fn data_labels(&self, spec: DataLabelSpec) -> Self {
        Self(self.0.clone().data_labels(spec.0))
    }

    /// Which kind of chart this is.
    #[getter]
    fn kind(&self) -> PyResult<ChartKind> {
        ChartKind::from_model(self.0.kind())
    }

    /// Whether the chart holds no series at all.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The series names, in order.
    #[getter]
    fn series_names(&self) -> Vec<String> {
        self.0.series_names().map(str::to_owned).collect()
    }

    /// The series values, in order.
    #[getter]
    fn series_values(&self) -> Vec<Vec<f64>> {
        self.0.series_values().map(<[f64]>::to_vec).collect()
    }

    /// How many categories the chart states.
    #[getter]
    fn category_count(&self) -> u32 {
        self.0.category_count() as u32
    }

    /// How many values the longest series holds.
    #[getter]
    fn longest_series(&self) -> u32 {
        self.0.longest_series() as u32
    }

    /// One category label, when the chart states one at that index.
    fn category_label(&self, index: u32) -> Option<&str> {
        self.0.category_label(index as usize)
    }

    /// Whether this description is one the chart kind will accept — the number of series it needs,
    /// the decoration its series may carry, and whether every measure is finite.
    ///
    /// Raises `InvalidArgumentError` describing the first problem it finds. `Deck.add_chart` runs
    /// the same check, so calling this first is a way to fail earlier, not a way to skip it.
    fn validate(&self) -> PyResult<()> {
        self.0
            .validate()
            .map_err(|error| to_py_err(ooxml::Error::from(ooxml::PptxError::from(error))))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl DataLabelSpec {
    /// Data labels that state nothing. Add to them with the fluent methods.
    #[new]
    fn new() -> Self {
        Self(ooxml::DataLabelSpec::new())
    }

    /// Show, or hide, each point's value.
    fn value(&self, show: bool) -> Self {
        Self(self.0.clone().value(show))
    }

    /// Show, or hide, each point's category name.
    fn category_name(&self, show: bool) -> Self {
        Self(self.0.clone().category_name(show))
    }

    /// Show, or hide, the series name.
    fn series_name(&self, show: bool) -> Self {
        Self(self.0.clone().series_name(show))
    }

    /// Show, or hide, each point's share of the total.
    fn percentage(&self, show: bool) -> Self {
        Self(self.0.clone().percentage(show))
    }

    /// Show, or hide, each bubble's size.
    fn bubble_size(&self, show: bool) -> Self {
        Self(self.0.clone().bubble_size(show))
    }

    /// Show, or hide, the legend swatch beside each label.
    fn legend_key(&self, show: bool) -> Self {
        Self(self.0.clone().legend_key(show))
    }

    /// Show, or hide, the lines that join a label to its point.
    fn leader_lines(&self, show: bool) -> Self {
        Self(self.0.clone().leader_lines(show))
    }

    /// Put the labels in the given position relative to their points.
    fn position(&self, position: DataLabelPosition) -> Self {
        Self(self.0.clone().position(position.into()))
    }

    /// Separate the parts of a label with the given string.
    fn separator(&self, separator: &str) -> Self {
        Self(self.0.clone().separator(separator.to_owned()))
    }

    /// Format the numbers with the given format code.
    fn number_format(&self, format_code: &str) -> Self {
        Self(self.0.clone().number_format(format_code.to_owned()))
    }

    /// Whether this specification states nothing.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl DataLabelSettings {
    /// Whether the labels are suppressed, when stated.
    #[getter]
    fn suppressed(&self) -> Option<bool> {
        self.0.suppressed
    }

    /// Whether the value is shown, when stated.
    #[getter]
    fn shows_value(&self) -> Option<bool> {
        self.0.shows_value
    }

    /// Whether the category name is shown, when stated.
    #[getter]
    fn shows_category_name(&self) -> Option<bool> {
        self.0.shows_category_name
    }

    /// Whether the series name is shown, when stated.
    #[getter]
    fn shows_series_name(&self) -> Option<bool> {
        self.0.shows_series_name
    }

    /// Whether the percentage is shown, when stated.
    #[getter]
    fn shows_percentage(&self) -> Option<bool> {
        self.0.shows_percentage
    }

    /// Whether the bubble size is shown, when stated.
    #[getter]
    fn shows_bubble_size(&self) -> Option<bool> {
        self.0.shows_bubble_size
    }

    /// Whether the legend key is shown, when stated.
    #[getter]
    fn shows_legend_key(&self) -> Option<bool> {
        self.0.shows_legend_key
    }

    /// Whether leader lines are shown, when stated.
    #[getter]
    fn shows_leader_lines(&self) -> Option<bool> {
        self.0.shows_leader_lines
    }

    /// Where the labels sit, when stated.
    #[getter]
    fn position(&self) -> PyResult<Option<DataLabelPosition>> {
        self.0
            .position
            .map(DataLabelPosition::from_model)
            .transpose()
    }

    /// The separator between the parts of a label, when stated.
    #[getter]
    fn separator(&self) -> Option<&str> {
        self.0.separator.as_deref()
    }

    /// The number format code, when stated.
    #[getter]
    fn number_format(&self) -> Option<&str> {
        self.0.number_format.as_deref()
    }

    /// Whether these settings state nothing at all.
    #[getter]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// These settings laid over `parent`: whatever this tier states wins, and the rest comes from
    /// the tier above. The same walk `chart_data_labels` makes.
    fn inherit(&self, parent: &Self) -> Self {
        Self(self.0.inherit(&parent.0))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartLabelScope {
    /// One plot of the chart — the widest tier.
    #[staticmethod]
    fn plot(plot_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Plot {
            plot_idx: plot_index as usize,
        })
    }

    /// One series.
    #[staticmethod]
    fn series(series_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Series {
            series_idx: series_index as usize,
        })
    }

    /// One data point — the narrowest tier.
    #[staticmethod]
    fn point(series_index: u32, point_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Point {
            series_idx: series_index as usize,
            point_idx: point_index,
        })
    }

    /// Which tier this is: `"plot"`, `"series"` or `"point"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            ooxml::ChartLabelScope::Plot { .. } => "plot",
            ooxml::ChartLabelScope::Series { .. } => "series",
            ooxml::ChartLabelScope::Point { .. } => "point",
        }
    }

    /// The plot index, when this is a plot scope.
    #[getter]
    fn plot_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Plot { plot_idx } => Some(plot_idx as u32),
            _ => None,
        }
    }

    /// The series index, when this scope names one.
    #[getter]
    fn series_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Series { series_idx }
            | ooxml::ChartLabelScope::Point { series_idx, .. } => Some(series_idx as u32),
            ooxml::ChartLabelScope::Plot { .. } => None,
        }
    }

    /// The point index, when this is a point scope.
    #[getter]
    fn point_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Point { point_idx, .. } => Some(point_idx),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl TrendlineSpec {
    /// A trendline of the given kind.
    #[new]
    fn new(kind: TrendlineKind) -> Self {
        Self(ooxml::TrendlineSpec::new(kind.into()))
    }

    /// This trendline with the given name.
    fn name(&self, name: &str) -> Self {
        Self(self.0.clone().name(name.to_owned()))
    }

    /// This trendline as a polynomial of the given order.
    fn polynomial_order(&self, order: u8) -> Self {
        Self(self.0.clone().polynomial_order(order))
    }

    /// This trendline as a moving average over the given number of periods.
    fn moving_average_period(&self, period: u32) -> Self {
        Self(self.0.clone().moving_average_period(period))
    }

    /// This trendline projected forward and backward by the given number of periods.
    fn projection(&self, forward: f64, backward: f64) -> Self {
        Self(self.0.clone().projection(forward, backward))
    }

    /// This trendline forced through the given intercept.
    fn intercept(&self, intercept: f64) -> Self {
        Self(self.0.clone().intercept(intercept))
    }

    /// This trendline showing its equation and its R² on the chart.
    fn display(&self, equation: bool, r_squared: bool) -> Self {
        Self(self.0.clone().display(equation, r_squared))
    }

    /// Which kind of trendline.
    #[getter]
    fn kind(&self) -> PyResult<TrendlineKind> {
        TrendlineKind::from_model(self.0.kind)
    }

    /// Whether this trendline's order and period are in range for its kind.
    ///
    /// Raises `InvalidArgumentError` describing the first problem it finds.
    fn validate(&self) -> PyResult<()> {
        self.0
            .validate()
            .map_err(|error| to_py_err(ooxml::Error::from(ooxml::PptxError::from(error))))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ErrorBarSpec {
    /// Error bars of a fixed size — a value, a percentage, a standard deviation or a standard
    /// error, depending on `value_type`.
    #[staticmethod]
    fn fixed(bar_type: ErrorBarType, value_type: ErrorValueType, value: f64) -> Self {
        Self(ooxml::ErrorBarSpec::fixed(
            bar_type.into(),
            value_type.into(),
            value,
        ))
    }

    /// Error bars whose lengths are given point by point.
    #[staticmethod]
    fn custom(bar_type: ErrorBarType, plus_values: Vec<f64>, minus_values: Vec<f64>) -> Self {
        Self(ooxml::ErrorBarSpec::custom(
            bar_type.into(),
            plus_values,
            minus_values,
        ))
    }

    /// These error bars along the given axis.
    fn direction(&self, direction: ErrorBarDirection) -> Self {
        Self(self.0.clone().direction(direction.into()))
    }

    /// These error bars with, or without, the cap at each end.
    fn no_end_cap(&self, no_end_cap: bool) -> Self {
        Self(self.0.clone().no_end_cap(no_end_cap))
    }

    /// Whether custom error bars carry the values they need.
    ///
    /// Raises `InvalidArgumentError` describing the first problem it finds.
    fn validate(&self) -> PyResult<()> {
        self.0
            .validate()
            .map_err(|error| to_py_err(ooxml::Error::from(ooxml::PptxError::from(error))))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

#[pymethods]
impl ChartSeriesData {
    /// The series name, when the chart states one.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// The category labels, in order.
    #[getter]
    fn categories(&self) -> Vec<String> {
        self.0.categories.clone()
    }

    /// The values, in order.
    #[getter]
    fn values(&self) -> Vec<f64> {
        self.0.values.clone()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartAxisData {
    /// Whether this is the category, value, date or series axis.
    #[getter]
    fn kind(&self) -> PyResult<AxisKind> {
        AxisKind::from_model(self.0.kind)
    }

    /// The axis's own identifier, when stated.
    #[getter]
    fn axis_id(&self) -> Option<u32> {
        self.0.axis_id
    }

    /// The identifier of the axis this one crosses, when stated.
    #[getter]
    fn cross_axis_id(&self) -> Option<u32> {
        self.0.cross_axis_id
    }

    /// Whether the axis is hidden, when stated.
    #[getter]
    fn suppressed(&self) -> Option<bool> {
        self.0.suppressed
    }

    /// Which side of the plot the axis sits on, when stated.
    #[getter]
    fn position(&self) -> PyResult<Option<AxisPosition>> {
        self.0.position.map(AxisPosition::from_model).transpose()
    }

    /// Which way the axis runs, when stated.
    #[getter]
    fn orientation(&self) -> PyResult<Option<AxisOrientation>> {
        self.0
            .orientation
            .map(AxisOrientation::from_model)
            .transpose()
    }

    /// The lower bound of the scale, when stated.
    #[getter]
    fn minimum(&self) -> Option<f64> {
        self.0.minimum
    }

    /// The upper bound of the scale, when stated.
    #[getter]
    fn maximum(&self) -> Option<f64> {
        self.0.maximum
    }

    /// The logarithm base, when the axis is logarithmic.
    #[getter]
    fn logarithm_base(&self) -> Option<f64> {
        self.0.logarithm_base
    }

    /// The axis title, when it has one.
    #[getter]
    fn title(&self) -> Option<&str> {
        self.0.title.as_deref()
    }

    /// Whether major gridlines are drawn.
    #[getter]
    fn major_gridlines(&self) -> bool {
        self.0.major_gridlines
    }

    /// Whether minor gridlines are drawn.
    #[getter]
    fn minor_gridlines(&self) -> bool {
        self.0.minor_gridlines
    }

    /// The major tick mark style, when stated.
    #[getter]
    fn major_tick_mark(&self) -> PyResult<Option<TickMark>> {
        self.0.major_tick_mark.map(TickMark::from_model).transpose()
    }

    /// The minor tick mark style, when stated.
    #[getter]
    fn minor_tick_mark(&self) -> PyResult<Option<TickMark>> {
        self.0.minor_tick_mark.map(TickMark::from_model).transpose()
    }

    /// Where the tick labels sit, when stated.
    #[getter]
    fn tick_label_position(&self) -> PyResult<Option<TickLabelPosition>> {
        self.0
            .tick_label_position
            .map(TickLabelPosition::from_model)
            .transpose()
    }

    /// The tick labels' number format code, when stated.
    #[getter]
    fn number_format(&self) -> Option<&str> {
        self.0.number_format.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartLegendData {
    /// Where the legend sits, when stated.
    #[getter]
    fn position(&self) -> PyResult<Option<LegendPosition>> {
        self.0.position.map(LegendPosition::from_model).transpose()
    }

    /// Whether the legend overlaps the plot rather than reserving space, when stated.
    #[getter]
    fn overlays_plot(&self) -> Option<bool> {
        self.0.overlays_plot
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartPointFormatData {
    /// Which point this formatting belongs to, when it names one.
    #[getter]
    fn index(&self) -> Option<u32> {
        self.0.index
    }

    /// The point's own fill, when it states one.
    #[getter]
    fn fill(&self) -> Option<FillSpec> {
        self.0.fill.clone().map(FillSpec)
    }

    /// The point's own outline, when it states one.
    #[getter]
    fn line(&self) -> Option<LineSpec> {
        self.0.line.clone().map(LineSpec)
    }

    /// How far the slice is pulled out of a pie, when stated.
    #[getter]
    fn explosion(&self) -> Option<u32> {
        self.0.explosion
    }

    /// Whether a negative value inverts the fill, when stated.
    #[getter]
    fn inverts_if_negative(&self) -> Option<bool> {
        self.0.inverts_if_negative
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartTrendlineData {
    /// Which kind of trendline, when stated.
    #[getter]
    fn kind(&self) -> PyResult<Option<TrendlineKind>> {
        self.0.kind.map(TrendlineKind::from_model).transpose()
    }

    /// The trendline's name, when stated.
    #[getter]
    fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// The polynomial order, when stated.
    #[getter]
    fn polynomial_order(&self) -> Option<u32> {
        self.0.polynomial_order
    }

    /// The moving-average period, when stated.
    #[getter]
    fn moving_average_period(&self) -> Option<u32> {
        self.0.moving_average_period
    }

    /// How far the line is projected forward, when stated.
    #[getter]
    fn forward_periods(&self) -> Option<f64> {
        self.0.forward_periods
    }

    /// How far the line is projected backward, when stated.
    #[getter]
    fn backward_periods(&self) -> Option<f64> {
        self.0.backward_periods
    }

    /// The intercept the line is forced through, when stated.
    #[getter]
    fn intercept(&self) -> Option<f64> {
        self.0.intercept
    }

    /// Whether the equation is shown, when stated.
    #[getter]
    fn displays_equation(&self) -> Option<bool> {
        self.0.displays_equation
    }

    /// Whether the R² is shown, when stated.
    #[getter]
    fn displays_r_squared(&self) -> Option<bool> {
        self.0.displays_r_squared
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartErrorBarData {
    /// Which axis the bars run along, when stated.
    #[getter]
    fn direction(&self) -> PyResult<Option<ErrorBarDirection>> {
        self.0
            .direction
            .map(ErrorBarDirection::from_model)
            .transpose()
    }

    /// Whether the bars run up, down or both ways, when stated.
    #[getter]
    fn bar_type(&self) -> PyResult<Option<ErrorBarType>> {
        self.0.bar_type.map(ErrorBarType::from_model).transpose()
    }

    /// How the bar lengths are computed, when stated.
    #[getter]
    fn value_type(&self) -> PyResult<Option<ErrorValueType>> {
        self.0
            .value_type
            .map(ErrorValueType::from_model)
            .transpose()
    }

    /// Whether the end caps are suppressed, when stated.
    #[getter]
    fn no_end_cap(&self) -> Option<bool> {
        self.0.no_end_cap
    }

    /// The fixed value, when the bars use one.
    #[getter]
    fn value(&self) -> Option<f64> {
        self.0.value
    }

    /// The upward lengths, point by point, when the bars are custom.
    #[getter]
    fn plus_values(&self) -> Vec<f64> {
        self.0.plus_values.clone()
    }

    /// The downward lengths, point by point, when the bars are custom.
    #[getter]
    fn minus_values(&self) -> Vec<f64> {
        self.0.minus_values.clone()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartWorkbook {
    /// The top-level index of the graphic frame that holds the chart.
    #[getter]
    fn shape_index(&self) -> u32 {
        self.0.shape_index as u32
    }

    /// Where the workbook is — a part name inside the package, or a URI outside it.
    #[getter]
    fn target(&self) -> &str {
        &self.0.target
    }

    /// Whether the workbook lies outside the package.
    #[getter]
    fn external(&self) -> bool {
        self.0.external
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl DanglingPointReference {
    /// Which element carries the dangling reference — `c:dPt`, `c:dLbl`, and so on.
    #[getter]
    fn element(&self) -> &'static str {
        self.0.element
    }

    /// The point index it names, which the series no longer has.
    #[getter]
    fn index(&self) -> u32 {
        self.0.index
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Adds every class in this module to the extension module.
#[pymethods]
impl DocumentChartWorkbook {
    /// The `wp:docPr` id of the drawing that frames the chart.
    #[getter]
    fn drawing_id(&self) -> u32 {
        self.0.drawing_id
    }

    /// Where the workbook is — a part name inside the package, or a URI outside it.
    #[getter]
    fn target(&self) -> &str {
        &self.0.target
    }

    /// Whether the workbook lies outside the package.
    #[getter]
    fn external(&self) -> bool {
        self.0.external
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[pymethods]
impl ChartWrap {
    /// The chart floats over or under the text and nothing reflows around it (`wp:wrapNone`).
    #[staticmethod]
    fn none() -> Self {
        Self(ooxml::ChartWrap::None)
    }

    /// Text wraps around the chart's bounding box, on the sides `wrap_text` names
    /// (`wp:wrapSquare`).
    #[staticmethod]
    fn square(wrap_text: WrapText) -> Self {
        Self(ooxml::ChartWrap::Square(wrap_text.into()))
    }

    /// Text wraps above and below the chart only, never beside it (`wp:wrapTopAndBottom`).
    #[staticmethod]
    fn top_and_bottom() -> Self {
        Self(ooxml::ChartWrap::TopAndBottom)
    }

    /// Which wrap this is: `"none"`, `"square"` or `"top_and_bottom"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            ooxml::ChartWrap::None => "none",
            ooxml::ChartWrap::Square(_) => "square",
            ooxml::ChartWrap::TopAndBottom => "top_and_bottom",
        }
    }

    /// Which sides text flows down, when this is a square wrap.
    #[getter]
    fn wrap_text(&self) -> PyResult<Option<WrapText>> {
        match self.0 {
            ooxml::ChartWrap::Square(text) => WrapText::from_model(text).map(Some),
            _ => Ok(None),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ChartData>()?;
    module.add_class::<DataLabelSpec>()?;
    module.add_class::<DataLabelSettings>()?;
    module.add_class::<ChartLabelScope>()?;
    module.add_class::<TrendlineSpec>()?;
    module.add_class::<ErrorBarSpec>()?;
    module.add_class::<ChartSeriesData>()?;
    module.add_class::<ChartAxisData>()?;
    module.add_class::<ChartLegendData>()?;
    module.add_class::<ChartPointFormatData>()?;
    module.add_class::<ChartTrendlineData>()?;
    module.add_class::<ChartErrorBarData>()?;
    module.add_class::<ChartWorkbook>()?;
    module.add_class::<DanglingPointReference>()?;
    module.add_class::<DocumentChartWorkbook>()?;
    module.add_class::<ChartWrap>()?;
    module.add_class::<ChartSeriesReferences>()?;
    module.add_class::<ChartRangeSeries>()?;
    module.add_class::<SheetChartWorkbookInfo>()?;
    module.add_class::<ChartSeriesFreshnessInfo>()?;
    module.add_class::<RangeCellInfo>()?;
    module.add_class::<ResolvedRangeInfo>()
}
