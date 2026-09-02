//! Charts: the description you author, and the structures you read back.
//!
//! [`ChartData`] is the authoring side — a kind, categories, series, a title, a legend — and is what
//! `Deck.addChart` takes. Everything else here is what the reading side hands back: the series,
//! axes, legend, point formats, trendlines and error bars a chart part already holds.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{
    AxisKind, AxisOrientation, AxisPosition, ChartKind, DataLabelPosition, ErrorBarDirection,
    ErrorBarType, ErrorValueType, LegendPosition, TickLabelPosition, TickMark, TrendlineKind,
};
use crate::errors::map_error;
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
}

// ---------------------------------------------------------------------------------------------
// Authoring
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl ChartData {
    /// A chart of the given kind, with nothing in it yet.
    #[wasm_bindgen(constructor)]
    pub fn new(kind: ChartKind) -> Self {
        Self(ooxml::ChartData::new(kind.into()))
    }

    /// This chart with the given category labels, replacing any it had.
    pub fn categories(&self, categories: Vec<String>) -> Self {
        Self(self.0.clone().categories(categories))
    }

    /// This chart with one more series.
    pub fn series(&self, name: &str, values: Vec<f64>) -> Self {
        Self(self.0.clone().series(name.to_owned(), values))
    }

    /// This chart with the given title.
    pub fn title(&self, title: &str) -> Self {
        Self(self.0.clone().title(title.to_owned()))
    }

    /// This chart with a legend in the given position.
    pub fn legend(&self, position: LegendPosition) -> Self {
        Self(self.0.clone().legend(position.into()))
    }

    /// This chart with the given data labels on every series.
    #[wasm_bindgen(js_name = "dataLabels")]
    pub fn data_labels(&self, spec: &DataLabelSpec) -> Self {
        Self(self.0.clone().data_labels(spec.0.clone()))
    }

    /// Which kind of chart this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<ChartKind, JsValue> {
        ChartKind::from_model(self.0.kind())
    }

    /// Whether the chart holds no series at all.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The series names, in order.
    #[wasm_bindgen(getter, js_name = "seriesNames")]
    pub fn series_names(&self) -> Vec<String> {
        self.0.series_names().map(str::to_owned).collect()
    }

    /// One series' values, in order; an empty array if the chart has no series at that index.
    ///
    /// One series at a time rather than all of them at once: `wasm-bindgen` cannot project a
    /// `Vec<Vec<f64>>`, and an array of `Float64Array` assembled by hand would type as `any[]`.
    #[wasm_bindgen(js_name = "seriesValues")]
    pub fn series_values(&self, index: u32) -> Vec<f64> {
        self.0
            .series_values()
            .nth(index as usize)
            .map(<[f64]>::to_vec)
            .unwrap_or_default()
    }

    /// How many series the chart holds.
    #[wasm_bindgen(getter, js_name = "seriesCount")]
    pub fn series_count(&self) -> u32 {
        u32::try_from(self.0.series_names().count()).unwrap_or(u32::MAX)
    }

    /// How many categories the chart states.
    #[wasm_bindgen(getter, js_name = "categoryCount")]
    pub fn category_count(&self) -> u32 {
        self.0.category_count() as u32
    }

    /// How many values the longest series holds.
    #[wasm_bindgen(getter, js_name = "longestSeries")]
    pub fn longest_series(&self) -> u32 {
        self.0.longest_series() as u32
    }

    /// One category label, when the chart states one at that index.
    #[wasm_bindgen(js_name = "categoryLabel")]
    pub fn category_label(&self, index: u32) -> Option<String> {
        self.0.category_label(index as usize).map(str::to_owned)
    }

    /// Whether this description is one the chart kind will accept — the number of series it needs,
    /// the decoration its series may carry, and whether every measure is finite.
    ///
    /// Throws an `OoxmlError` with code `InvalidArgument` describing the first problem it finds.
    /// `Deck.addChart` runs the same check, so calling this first is a way to fail earlier, not a
    /// way to skip it.
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(
            self.0
                .validate()
                .map_err(|error| ooxml::Error::from(ooxml::PptxError::from(error))),
        )
    }
}

#[wasm_bindgen]
impl DataLabelSpec {
    /// Data labels that state nothing. Add to them with the fluent methods.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self(ooxml::DataLabelSpec::new())
    }

    /// Show, or hide, each point's value.
    pub fn value(&self, show: bool) -> Self {
        Self(self.0.clone().value(show))
    }

    /// Show, or hide, each point's category name.
    #[wasm_bindgen(js_name = "categoryName")]
    pub fn category_name(&self, show: bool) -> Self {
        Self(self.0.clone().category_name(show))
    }

    /// Show, or hide, the series name.
    #[wasm_bindgen(js_name = "seriesName")]
    pub fn series_name(&self, show: bool) -> Self {
        Self(self.0.clone().series_name(show))
    }

    /// Show, or hide, each point's share of the total.
    pub fn percentage(&self, show: bool) -> Self {
        Self(self.0.clone().percentage(show))
    }

    /// Show, or hide, each bubble's size.
    #[wasm_bindgen(js_name = "bubbleSize")]
    pub fn bubble_size(&self, show: bool) -> Self {
        Self(self.0.clone().bubble_size(show))
    }

    /// Show, or hide, the legend swatch beside each label.
    #[wasm_bindgen(js_name = "legendKey")]
    pub fn legend_key(&self, show: bool) -> Self {
        Self(self.0.clone().legend_key(show))
    }

    /// Show, or hide, the lines that join a label to its point.
    #[wasm_bindgen(js_name = "leaderLines")]
    pub fn leader_lines(&self, show: bool) -> Self {
        Self(self.0.clone().leader_lines(show))
    }

    /// Put the labels in the given position relative to their points.
    pub fn position(&self, position: DataLabelPosition) -> Self {
        Self(self.0.clone().position(position.into()))
    }

    /// Separate the parts of a label with the given string.
    pub fn separator(&self, separator: &str) -> Self {
        Self(self.0.clone().separator(separator.to_owned()))
    }

    /// Format the numbers with the given format code.
    #[wasm_bindgen(js_name = "numberFormat")]
    pub fn number_format(&self, format_code: &str) -> Self {
        Self(self.0.clone().number_format(format_code.to_owned()))
    }

    /// Whether this specification states nothing.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[wasm_bindgen]
impl DataLabelSettings {
    /// Whether the labels are suppressed, when stated.
    #[wasm_bindgen(getter, js_name = "deleted")]
    pub fn deleted(&self) -> Option<bool> {
        self.0.deleted
    }

    /// Whether the value is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsValue")]
    pub fn shows_value(&self) -> Option<bool> {
        self.0.shows_value
    }

    /// Whether the category name is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsCategoryName")]
    pub fn shows_category_name(&self) -> Option<bool> {
        self.0.shows_category_name
    }

    /// Whether the series name is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsSeriesName")]
    pub fn shows_series_name(&self) -> Option<bool> {
        self.0.shows_series_name
    }

    /// Whether the percentage is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsPercentage")]
    pub fn shows_percentage(&self) -> Option<bool> {
        self.0.shows_percentage
    }

    /// Whether the bubble size is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsBubbleSize")]
    pub fn shows_bubble_size(&self) -> Option<bool> {
        self.0.shows_bubble_size
    }

    /// Whether the legend key is shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsLegendKey")]
    pub fn shows_legend_key(&self) -> Option<bool> {
        self.0.shows_legend_key
    }

    /// Whether leader lines are shown, when stated.
    #[wasm_bindgen(getter, js_name = "showsLeaderLines")]
    pub fn shows_leader_lines(&self) -> Option<bool> {
        self.0.shows_leader_lines
    }

    /// Where the labels sit, when stated.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Result<Option<DataLabelPosition>, JsValue> {
        self.0
            .position
            .map(DataLabelPosition::from_model)
            .transpose()
    }

    /// The separator between the parts of a label, when stated.
    #[wasm_bindgen(getter, js_name = "separator")]
    pub fn separator(&self) -> Option<String> {
        self.0.separator.clone()
    }

    /// The number format code, when stated.
    #[wasm_bindgen(getter, js_name = "numberFormat")]
    pub fn number_format(&self) -> Option<String> {
        self.0.number_format.clone()
    }

    /// Whether these settings state nothing at all.
    #[wasm_bindgen(getter, js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// These settings laid over `parent`: whatever this tier states wins, and the rest comes from
    /// the tier above. The same walk `chart_data_labels` makes.
    pub fn inherit(&self, parent: &Self) -> Self {
        Self(self.0.inherit(&parent.0))
    }
}

#[wasm_bindgen]
impl ChartLabelScope {
    /// One plot of the chart — the widest tier.
    pub fn plot(plot_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Plot {
            plot_idx: plot_index as usize,
        })
    }

    /// One series.
    pub fn series(series_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Series {
            series_idx: series_index as usize,
        })
    }

    /// One data point — the narrowest tier.
    pub fn point(series_index: u32, point_index: u32) -> Self {
        Self(ooxml::ChartLabelScope::Point {
            series_idx: series_index as usize,
            point_idx: point_index,
        })
    }

    /// Which tier this is: `"plot"`, `"series"` or `"point"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match self.0 {
            ooxml::ChartLabelScope::Plot { .. } => "plot".to_owned(),
            ooxml::ChartLabelScope::Series { .. } => "series".to_owned(),
            ooxml::ChartLabelScope::Point { .. } => "point".to_owned(),
        }
    }

    /// The plot index, when this is a plot scope.
    #[wasm_bindgen(getter, js_name = "plotIndex")]
    pub fn plot_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Plot { plot_idx } => Some(plot_idx as u32),
            _ => None,
        }
    }

    /// The series index, when this scope names one.
    #[wasm_bindgen(getter, js_name = "seriesIndex")]
    pub fn series_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Series { series_idx }
            | ooxml::ChartLabelScope::Point { series_idx, .. } => Some(series_idx as u32),
            ooxml::ChartLabelScope::Plot { .. } => None,
        }
    }

    /// The point index, when this is a point scope.
    #[wasm_bindgen(getter, js_name = "pointIndex")]
    pub fn point_index(&self) -> Option<u32> {
        match self.0 {
            ooxml::ChartLabelScope::Point { point_idx, .. } => Some(point_idx),
            _ => None,
        }
    }
}

#[wasm_bindgen]
impl TrendlineSpec {
    /// A trendline of the given kind.
    #[wasm_bindgen(constructor)]
    pub fn new(kind: TrendlineKind) -> Self {
        Self(ooxml::TrendlineSpec::new(kind.into()))
    }

    /// This trendline with the given name.
    pub fn name(&self, name: &str) -> Self {
        Self(self.0.clone().name(name.to_owned()))
    }

    /// This trendline as a polynomial of the given order.
    #[wasm_bindgen(js_name = "polynomialOrder")]
    pub fn polynomial_order(&self, order: u8) -> Self {
        Self(self.0.clone().polynomial_order(order))
    }

    /// This trendline as a moving average over the given number of periods.
    #[wasm_bindgen(js_name = "movingAveragePeriod")]
    pub fn moving_average_period(&self, period: u32) -> Self {
        Self(self.0.clone().moving_average_period(period))
    }

    /// This trendline projected forward and backward by the given number of periods.
    pub fn projection(&self, forward: f64, backward: f64) -> Self {
        Self(self.0.clone().projection(forward, backward))
    }

    /// This trendline forced through the given intercept.
    pub fn intercept(&self, intercept: f64) -> Self {
        Self(self.0.clone().intercept(intercept))
    }

    /// This trendline showing its equation and its R² on the chart.
    pub fn display(&self, equation: bool, r_squared: bool) -> Self {
        Self(self.0.clone().display(equation, r_squared))
    }

    /// Which kind of trendline.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<TrendlineKind, JsValue> {
        TrendlineKind::from_model(self.0.kind)
    }

    /// Whether this trendline's order and period are in range for its kind.
    ///
    /// Throws an `OoxmlError` with code `InvalidArgument` describing the first problem it finds.
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(
            self.0
                .validate()
                .map_err(|error| ooxml::Error::from(ooxml::PptxError::from(error))),
        )
    }
}

#[wasm_bindgen]
impl ErrorBarSpec {
    /// Error bars of a fixed size — a value, a percentage, a standard deviation or a standard
    /// error, depending on `value_type`.
    pub fn fixed(bar_type: ErrorBarType, value_type: ErrorValueType, value: f64) -> Self {
        Self(ooxml::ErrorBarSpec::fixed(
            bar_type.into(),
            value_type.into(),
            value,
        ))
    }

    /// Error bars whose lengths are given point by point.
    pub fn custom(bar_type: ErrorBarType, plus_values: Vec<f64>, minus_values: Vec<f64>) -> Self {
        Self(ooxml::ErrorBarSpec::custom(
            bar_type.into(),
            plus_values,
            minus_values,
        ))
    }

    /// These error bars along the given axis.
    pub fn direction(&self, direction: ErrorBarDirection) -> Self {
        Self(self.0.clone().direction(direction.into()))
    }

    /// These error bars with, or without, the cap at each end.
    #[wasm_bindgen(js_name = "noEndCap")]
    pub fn no_end_cap(&self, no_end_cap: bool) -> Self {
        Self(self.0.clone().no_end_cap(no_end_cap))
    }

    /// Whether custom error bars carry the values they need.
    ///
    /// Throws an `OoxmlError` with code `InvalidArgument` describing the first problem it finds.
    pub fn validate(&self) -> Result<(), JsValue> {
        map_error(
            self.0
                .validate()
                .map_err(|error| ooxml::Error::from(ooxml::PptxError::from(error))),
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl ChartSeriesData {
    /// The series name, when the chart states one.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// The category labels, in order.
    #[wasm_bindgen(getter, js_name = "categories")]
    pub fn categories(&self) -> Vec<String> {
        self.0.categories.clone()
    }

    /// The values, in order.
    #[wasm_bindgen(getter, js_name = "values")]
    pub fn values(&self) -> Vec<f64> {
        self.0.values.clone()
    }
}

#[wasm_bindgen]
impl ChartAxisData {
    /// Whether this is the category, value, date or series axis.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<AxisKind, JsValue> {
        AxisKind::from_model(self.0.kind)
    }

    /// The axis's own identifier, when stated.
    #[wasm_bindgen(getter, js_name = "axisId")]
    pub fn axis_id(&self) -> Option<u32> {
        self.0.axis_id
    }

    /// The identifier of the axis this one crosses, when stated.
    #[wasm_bindgen(getter, js_name = "crossAxisId")]
    pub fn cross_axis_id(&self) -> Option<u32> {
        self.0.cross_axis_id
    }

    /// Whether the axis is hidden, when stated.
    #[wasm_bindgen(getter, js_name = "deleted")]
    pub fn deleted(&self) -> Option<bool> {
        self.0.deleted
    }

    /// Which side of the plot the axis sits on, when stated.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Result<Option<AxisPosition>, JsValue> {
        self.0.position.map(AxisPosition::from_model).transpose()
    }

    /// Which way the axis runs, when stated.
    #[wasm_bindgen(getter, js_name = "orientation")]
    pub fn orientation(&self) -> Result<Option<AxisOrientation>, JsValue> {
        self.0
            .orientation
            .map(AxisOrientation::from_model)
            .transpose()
    }

    /// The lower bound of the scale, when stated.
    #[wasm_bindgen(getter, js_name = "minimum")]
    pub fn minimum(&self) -> Option<f64> {
        self.0.minimum
    }

    /// The upper bound of the scale, when stated.
    #[wasm_bindgen(getter, js_name = "maximum")]
    pub fn maximum(&self) -> Option<f64> {
        self.0.maximum
    }

    /// The logarithm base, when the axis is logarithmic.
    #[wasm_bindgen(getter, js_name = "logarithmBase")]
    pub fn logarithm_base(&self) -> Option<f64> {
        self.0.logarithm_base
    }

    /// The axis title, when it has one.
    #[wasm_bindgen(getter, js_name = "title")]
    pub fn title(&self) -> Option<String> {
        self.0.title.clone()
    }

    /// Whether major gridlines are drawn.
    #[wasm_bindgen(getter, js_name = "majorGridlines")]
    pub fn major_gridlines(&self) -> bool {
        self.0.major_gridlines
    }

    /// Whether minor gridlines are drawn.
    #[wasm_bindgen(getter, js_name = "minorGridlines")]
    pub fn minor_gridlines(&self) -> bool {
        self.0.minor_gridlines
    }

    /// The major tick mark style, when stated.
    #[wasm_bindgen(getter, js_name = "majorTickMark")]
    pub fn major_tick_mark(&self) -> Result<Option<TickMark>, JsValue> {
        self.0.major_tick_mark.map(TickMark::from_model).transpose()
    }

    /// The minor tick mark style, when stated.
    #[wasm_bindgen(getter, js_name = "minorTickMark")]
    pub fn minor_tick_mark(&self) -> Result<Option<TickMark>, JsValue> {
        self.0.minor_tick_mark.map(TickMark::from_model).transpose()
    }

    /// Where the tick labels sit, when stated.
    #[wasm_bindgen(getter, js_name = "tickLabelPosition")]
    pub fn tick_label_position(&self) -> Result<Option<TickLabelPosition>, JsValue> {
        self.0
            .tick_label_position
            .map(TickLabelPosition::from_model)
            .transpose()
    }

    /// The tick labels' number format code, when stated.
    #[wasm_bindgen(getter, js_name = "numberFormat")]
    pub fn number_format(&self) -> Option<String> {
        self.0.number_format.clone()
    }
}

#[wasm_bindgen]
impl ChartLegendData {
    /// Where the legend sits, when stated.
    #[wasm_bindgen(getter, js_name = "position")]
    pub fn position(&self) -> Result<Option<LegendPosition>, JsValue> {
        self.0.position.map(LegendPosition::from_model).transpose()
    }

    /// Whether the legend overlaps the plot rather than reserving space, when stated.
    #[wasm_bindgen(getter, js_name = "overlaysPlot")]
    pub fn overlays_plot(&self) -> Option<bool> {
        self.0.overlays_plot
    }
}

#[wasm_bindgen]
impl ChartPointFormatData {
    /// Which point this formatting belongs to, when it names one.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> Option<u32> {
        self.0.index
    }

    /// The point's own fill, when it states one.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Option<FillSpec> {
        self.0.fill.clone().map(FillSpec)
    }

    /// The point's own outline, when it states one.
    #[wasm_bindgen(getter, js_name = "line")]
    pub fn line(&self) -> Option<LineSpec> {
        self.0.line.clone().map(LineSpec)
    }

    /// How far the slice is pulled out of a pie, when stated.
    #[wasm_bindgen(getter, js_name = "explosion")]
    pub fn explosion(&self) -> Option<u32> {
        self.0.explosion
    }

    /// Whether a negative value inverts the fill, when stated.
    #[wasm_bindgen(getter, js_name = "invertsIfNegative")]
    pub fn inverts_if_negative(&self) -> Option<bool> {
        self.0.inverts_if_negative
    }
}

#[wasm_bindgen]
impl ChartTrendlineData {
    /// Which kind of trendline, when stated.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<Option<TrendlineKind>, JsValue> {
        self.0.kind.map(TrendlineKind::from_model).transpose()
    }

    /// The trendline's name, when stated.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// The polynomial order, when stated.
    #[wasm_bindgen(getter, js_name = "polynomialOrder")]
    pub fn polynomial_order(&self) -> Option<u32> {
        self.0.polynomial_order
    }

    /// The moving-average period, when stated.
    #[wasm_bindgen(getter, js_name = "movingAveragePeriod")]
    pub fn moving_average_period(&self) -> Option<u32> {
        self.0.moving_average_period
    }

    /// How far the line is projected forward, when stated.
    #[wasm_bindgen(getter, js_name = "forwardPeriods")]
    pub fn forward_periods(&self) -> Option<f64> {
        self.0.forward_periods
    }

    /// How far the line is projected backward, when stated.
    #[wasm_bindgen(getter, js_name = "backwardPeriods")]
    pub fn backward_periods(&self) -> Option<f64> {
        self.0.backward_periods
    }

    /// The intercept the line is forced through, when stated.
    #[wasm_bindgen(getter, js_name = "intercept")]
    pub fn intercept(&self) -> Option<f64> {
        self.0.intercept
    }

    /// Whether the equation is shown, when stated.
    #[wasm_bindgen(getter, js_name = "displaysEquation")]
    pub fn displays_equation(&self) -> Option<bool> {
        self.0.displays_equation
    }

    /// Whether the R² is shown, when stated.
    #[wasm_bindgen(getter, js_name = "displaysRSquared")]
    pub fn displays_r_squared(&self) -> Option<bool> {
        self.0.displays_r_squared
    }
}

#[wasm_bindgen]
impl ChartErrorBarData {
    /// Which axis the bars run along, when stated.
    #[wasm_bindgen(getter, js_name = "direction")]
    pub fn direction(&self) -> Result<Option<ErrorBarDirection>, JsValue> {
        self.0
            .direction
            .map(ErrorBarDirection::from_model)
            .transpose()
    }

    /// Whether the bars run up, down or both ways, when stated.
    #[wasm_bindgen(getter, js_name = "barType")]
    pub fn bar_type(&self) -> Result<Option<ErrorBarType>, JsValue> {
        self.0.bar_type.map(ErrorBarType::from_model).transpose()
    }

    /// How the bar lengths are computed, when stated.
    #[wasm_bindgen(getter, js_name = "valueType")]
    pub fn value_type(&self) -> Result<Option<ErrorValueType>, JsValue> {
        self.0
            .value_type
            .map(ErrorValueType::from_model)
            .transpose()
    }

    /// Whether the end caps are suppressed, when stated.
    #[wasm_bindgen(getter, js_name = "noEndCap")]
    pub fn no_end_cap(&self) -> Option<bool> {
        self.0.no_end_cap
    }

    /// The fixed value, when the bars use one.
    #[wasm_bindgen(getter, js_name = "value")]
    pub fn value(&self) -> Option<f64> {
        self.0.value
    }

    /// The upward lengths, point by point, when the bars are custom.
    #[wasm_bindgen(getter, js_name = "plusValues")]
    pub fn plus_values(&self) -> Vec<f64> {
        self.0.plus_values.clone()
    }

    /// The downward lengths, point by point, when the bars are custom.
    #[wasm_bindgen(getter, js_name = "minusValues")]
    pub fn minus_values(&self) -> Vec<f64> {
        self.0.minus_values.clone()
    }
}

#[wasm_bindgen]
impl ChartWorkbook {
    /// The top-level index of the graphic frame that holds the chart.
    #[wasm_bindgen(getter, js_name = "shapeIndex")]
    pub fn shape_index(&self) -> u32 {
        self.0.shape_index as u32
    }

    /// Where the workbook is — a part name inside the package, or a URI outside it.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> String {
        self.0.target.clone()
    }

    /// Whether the workbook lies outside the package.
    #[wasm_bindgen(getter, js_name = "external")]
    pub fn external(&self) -> bool {
        self.0.external
    }
}

#[wasm_bindgen]
impl DanglingPointReference {
    /// Which element carries the dangling reference — `c:dPt`, `c:dLbl`, and so on.
    #[wasm_bindgen(getter, js_name = "element")]
    pub fn element(&self) -> String {
        self.0.element.to_owned()
    }

    /// The point index it names, which the series no longer has.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index
    }
}

impl Default for DataLabelSpec {
    /// The same value the no-argument constructor builds.
    fn default() -> Self {
        Self::new()
    }
}
