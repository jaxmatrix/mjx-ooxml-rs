//! The read summaries a *host* surface hands its caller — one plain value per subject, resolved out
//! of the typed model so a caller never holds a borrow into a chart part.
//!
//! # Why these live here and not in a format crate
//!
//! Until MJXOFF-103 every one of these types was declared inside `mjx-pptx`
//! (`presentation/charts.rs` and `presentation/chart_decoration.rs`), because a chart was reachable
//! from exactly one surface — a `p:graphicFrame` — and a type used once belongs where it is used.
//! MJXOFF-103 gives Word the same chart vocabulary, and `mjx-docx` and `mjx-pptx` are **both rank
//! 3.0**: an edge between them is sideways, which the layering rule forbids exactly as firmly as an
//! upward one. A second declaration of `ChartSeriesData` in `mjx-docx` would then be two types with
//! one name, and "the same chart reads the same way from a presentation and a document" would be
//! prose rather than a fact the compiler holds.
//!
//! So they moved **down**, to the crate that already owns `c:chartSpace` and that both format crates
//! already depend on. `mjx-pptx` re-exports every one of them from here, so no caller of the
//! PowerPoint surface sees a change. Three of them are `#[non_exhaustive]`, which is why every one
//! is built by [`crate::ops`] rather than by a struct literal in a host crate: the attribute that
//! keeps a new field from breaking a downstream caller also stops a host crate constructing one, so
//! the construction has to live beside the declaration. That is a constraint worth having — it is
//! what makes both host surfaces read a chart through *one* body of code.

use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::Interner;

use crate::axis::{
    Axis, AxisKind, AxisOrientation, AxisPosition, LegendPosition, TickLabelPosition, TickMark,
};
use crate::decoration::{ErrorBarDirection, ErrorBarType, ErrorValueType, TrendlineKind};

/// One series of a chart: its name and the labels and values it draws (for a scatter series, its X
/// labels and Y values).
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeriesData {
    /// The series name (`c:tx`), or `None` when it has none.
    pub name: Option<String>,
    /// The category labels the series draws (`c:cat`, or a scatter series' `c:xVal`), in order.
    pub categories: Vec<String>,
    /// The values the series draws (`c:val`, or a scatter series' `c:yVal`), in order.
    pub values: Vec<f64>,
}

/// Where one series says its data lives — the `c:f` formulas beside its caches, as written
/// (MJXOFF-111).
///
/// **The text, never a parse.** `c:f` is preserved as opaque wire text by this crate's data model
/// and this report keeps it that way: a `c:f` naming a range, a defined name or another workbook all
/// come back as themselves, and making sense of one is the host's — `mjx_xlsx`'s resolver is what
/// turns it into cells, because only a package knows which sheet is which.
///
/// A field is `None` when that source is a **literal** rather than a reference: a `c:numLit` has no
/// cells behind it, and neither has a `c:tx` holding a bare `c:v`. That distinction is the whole
/// point of the type — a series whose values are literal cannot be refreshed from a sheet, and
/// saying so is better than reporting an empty string.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChartSeriesReferences {
    /// The cell the series' name comes from (`c:tx > c:strRef > c:f`).
    pub name: Option<String>,
    /// The cells its category labels come from (`c:cat`, or a scatter series' `c:xVal`) — whichever
    /// of the three reference shapes the source uses.
    pub categories: Option<String>,
    /// The cells its values come from (`c:val`, or a scatter series' `c:yVal`).
    pub values: Option<String>,
}

/// One axis of a chart — everything `EG_AxShared` says about it, resolved into typed values.
///
/// A field is `None` when the axis does not declare that setting: the axis inherits it, and this
/// says so rather than guessing what Office would draw.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartAxisData {
    /// Which kind of axis this is — the element it was read from.
    pub kind: AxisKind,
    /// The axis' id (`c:axId`), which a plot's `c:axId` and the partner axis' `c:crossAx` name.
    pub axis_id: Option<u32>,
    /// The id of the axis this one crosses (`c:crossAx`).
    pub cross_axis_id: Option<u32>,
    /// Whether the axis is hidden (`c:delete`).
    pub suppressed: Option<bool>,
    /// Where the axis sits against the plot area (`c:axPos`).
    pub position: Option<AxisPosition>,
    /// Which way the axis runs (`c:scaling > c:orientation`).
    pub orientation: Option<AxisOrientation>,
    /// The axis' explicit lower bound (`c:scaling > c:min`), or `None` when it scales automatically.
    pub minimum: Option<f64>,
    /// The axis' explicit upper bound (`c:scaling > c:max`).
    pub maximum: Option<f64>,
    /// The base of a logarithmic scale (`c:scaling > c:logBase`), or `None` for a linear axis.
    pub logarithm_base: Option<f64>,
    /// The axis' title text (`c:title`), or `None` when it has none.
    pub title: Option<String>,
    /// Whether the axis rules major gridlines across the plot area.
    pub major_gridlines: bool,
    /// Whether the axis rules minor gridlines across the plot area.
    pub minor_gridlines: bool,
    /// How the major tick marks are drawn (`c:majorTickMark`).
    pub major_tick_mark: Option<TickMark>,
    /// How the minor tick marks are drawn (`c:minorTickMark`).
    pub minor_tick_mark: Option<TickMark>,
    /// Where the tick labels are placed (`c:tickLblPos`).
    pub tick_label_position: Option<TickLabelPosition>,
    /// The axis' number format (`c:numFmt@formatCode`), or `None` when it inherits one.
    pub number_format: Option<String>,
}

impl ChartAxisData {
    /// Reads one axis into its summary.
    pub(crate) fn read(kind: AxisKind, axis: &Axis, interner: &Interner) -> Self {
        let scaling = axis.scaling();
        Self {
            kind,
            axis_id: axis.axis_id(interner),
            cross_axis_id: axis.cross_axis_id(interner),
            suppressed: axis.is_suppressed(interner),
            position: axis.position(interner),
            orientation: scaling.and_then(|scaling| scaling.orientation(interner)),
            minimum: scaling.and_then(|scaling| scaling.minimum(interner)),
            maximum: scaling.and_then(|scaling| scaling.maximum(interner)),
            logarithm_base: scaling.and_then(|scaling| scaling.logarithm_base(interner)),
            title: axis.title_text(),
            major_gridlines: axis.has_major_gridlines(),
            minor_gridlines: axis.has_minor_gridlines(),
            major_tick_mark: axis.major_tick_mark(interner),
            minor_tick_mark: axis.minor_tick_mark(interner),
            tick_label_position: axis.tick_label_position(interner),
            number_format: axis.number_format(interner).map(str::to_owned),
        }
    }
}

/// A chart's legend.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartLegendData {
    /// Where the legend sits (`c:legendPos`), or `None` when it declares no position.
    pub position: Option<LegendPosition>,
    /// Whether the legend is drawn on top of the plot area rather than beside it (`c:overlay`).
    pub overlays_plot: Option<bool>,
}

/// Which of the three tiers of a chart's data labels an edit or a read addresses.
///
/// `c:dLbls` is the same element at the plot tier and the series tier (ECMA-376 Part 1 §21.2.2.49),
/// and a `c:dLbl` inside a series' container overrides it for one point. Naming the tier explicitly
/// is what keeps "label this series" and "label this point" from being the same call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartLabelScope {
    /// The plot's own settings — the default every series of it takes. Plots are numbered as
    /// `chart_kinds` numbers them, so a combo chart's two plots are 0 and 1.
    Plot {
        /// Which plot of the plot area.
        plot_idx: usize,
    },
    /// One series' settings, overriding its plot's.
    Series {
        /// Which series, counted across every plot.
        series_idx: usize,
    },
    /// One point's settings, overriding its series'.
    Point {
        /// Which series, counted across every plot.
        series_idx: usize,
        /// Which point of that series — the `c:idx` the override is anchored by.
        point_idx: u32,
    },
}

/// One point of a series drawn differently from the rest (`c:dPt`), as read.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ChartPointFormatData {
    /// The 0-based index of the point this formats (`c:idx@val`) — **the anchor**, not this entry's
    /// position in the list. `None` for a `c:idx` the schema requires but the file omits or
    /// mis-spells; such an element addresses no point and is never renumbered.
    pub index: Option<u32>,
    /// The point's fill — the colour that makes it stand out — or `None` when it takes its series'.
    pub fill: Option<FillSpec>,
    /// The point's outline, or `None` when it takes its series'.
    pub line: Option<LineSpec>,
    /// How far a pie or doughnut slice is pulled out of the centre (`c:explosion`), as a percentage.
    pub explosion: Option<u32>,
    /// Whether the point's fill is inverted when its value is negative (`c:invertIfNegative`).
    pub inverts_if_negative: Option<bool>,
}

/// A curve fitted through a series (`c:trendline`), as read.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ChartTrendlineData {
    /// The curve the trendline fits (`c:trendlineType`).
    pub kind: Option<TrendlineKind>,
    /// The trendline's name, shown in the legend (`c:name`).
    pub name: Option<String>,
    /// The order of a polynomial curve (`c:order`), which defaults to 2.
    pub polynomial_order: Option<u32>,
    /// The window of a moving average (`c:period`), which defaults to 2.
    pub moving_average_period: Option<u32>,
    /// How far past the last point the curve is extended, in categories (`c:forward`).
    pub forward_periods: Option<f64>,
    /// How far before the first point the curve is extended (`c:backward`).
    pub backward_periods: Option<f64>,
    /// The value the curve is forced through (`c:intercept`).
    pub intercept: Option<f64>,
    /// Whether the curve's equation is drawn on the chart (`c:dispEq`).
    pub displays_equation: Option<bool>,
    /// Whether the curve's R² is drawn on the chart (`c:dispRSqr`).
    pub displays_r_squared: Option<bool>,
}

/// The uncertainty drawn around a series' points (`c:errBars`), as read.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ChartErrorBarData {
    /// Which axis the bars run along (`c:errDir`).
    pub direction: Option<ErrorBarDirection>,
    /// Which side(s) of the point the bars are drawn on (`c:errBarType`).
    pub bar_type: Option<ErrorBarType>,
    /// How the bars' length is arrived at (`c:errValType`).
    pub value_type: Option<ErrorValueType>,
    /// Whether the bars are drawn without their end caps (`c:noEndCap`).
    pub no_end_cap: Option<bool>,
    /// The single length every bar takes, read as [`value_type`](Self::value_type) says (`c:val`).
    pub value: Option<f64>,
    /// The per-point lengths in the positive direction (`c:plus`), empty when the bars are not
    /// custom.
    pub plus_values: Vec<f64>,
    /// The per-point lengths in the negative direction (`c:minus`).
    pub minus_values: Vec<f64>,
}
