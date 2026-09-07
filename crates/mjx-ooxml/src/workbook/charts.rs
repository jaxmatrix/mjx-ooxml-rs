//! Charts on the Excel surface — the same family, under the same names, as
//! [`crate::deck::charts`] and [`crate::document::charts`] carry for a presentation and a document.
//!
//! Every method here delegates to the identically-named method on [`mjx_xlsx::Workbook`], which in
//! turn calls the identically-named function in `mjx_chart::chart_ops` — so "the same chart
//! vocabulary reads, authors and edits from a presentation, a document and a workbook" is one body
//! of code with three ways in, not three implementations that happen to agree.
//!
//! # The address, and why it is two numbers
//!
//! A `Deck` addresses a chart as `(surface, shape)`; a `Document` by its drawing's own `wp:docPr`
//! id. A `Workbook` addresses it as `(sheet, anchor)` — the tab, and the anchor's position in that
//! sheet's drawing part, which is also its paint order. That is not a scheme invented here: it is
//! the address [`Workbook::sheet_drawing`](crate::Workbook::sheet_drawing) already reports and
//! [`remove_sheet_drawing_object`](crate::Workbook::remove_sheet_drawing_object) already takes, so
//! `add_chart`'s return value removes a chart exactly as `add_two_cell_anchored_picture`'s removes a
//! picture. Everything after those two arguments — argument order, method names, return shapes — is
//! identical to the other two surfaces.
//!
//! # The two things only this surface has
//!
//! A chart on a worksheet can take its data from **cells in the same workbook** rather than from an
//! embedded copy, which is a case that exists nowhere else in this library.
//! [`add_range_chart`](Workbook::add_range_chart) writes one and
//! [`chart_series_freshness`](Workbook::chart_series_freshness) is how a caller learns when the
//! cache and the cells have drifted apart — the cache is what draws until a consumer recalculates,
//! and both are reported with each named.

use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;

use crate::error::Error;
use crate::index::{count, index};
use crate::{
    AxisOrientation, ChartAxisData, ChartData, ChartErrorBarData, ChartKind, ChartLabelScope,
    ChartLegendData, ChartPointFormatData, ChartSeriesData, ChartSeriesReferences,
    ChartTrendlineData, DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarSpec,
    FillSpec, LegendPosition, LineSpec, TrendlineSpec,
};

use super::Workbook;

/// One series of a chart on a sheet: what its caches hold, what its cells say, and whether the two
/// agree.
///
/// The **honest** answer to *"what happens when the cache and the cells disagree"*: the cache is
/// what draws until a consumer recalculates, the cells are what a consumer would recalculate from,
/// and neither is silently preferred. `mjx_xlsx::ChartSeriesFreshness` restated with its
/// `RangeProblem`s as text, because a binding cannot carry an opaque enum across the boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeriesFreshnessInfo {
    /// Which series this is, in the order [`Workbook::chart_series`] reports them.
    pub series_index: u32,
    /// What the chart draws today — its caches.
    pub cached: ChartSeriesData,
    /// The `c:f` of each of the series' sources, as the file wrote them. A field is `None` where the
    /// source is a literal and so has no cells behind it at all.
    pub references: ChartSeriesReferences,
    /// What the cells say, for the sources that are references *and* resolved.
    pub from_cells: ChartSeriesData,
    /// Why the values reference did not resolve, in words, or `None` when it did — or when there is
    /// none.
    pub values_problem: Option<String>,
    /// Why the categories reference did not resolve, in words, or `None`.
    pub categories_problem: Option<String>,
    /// Whether the cached values and the cells agree.
    ///
    /// `None` means **cannot say**: the series' values are a literal, or its reference did not
    /// resolve. A third answer rather than a `false`, because "the cells disagree" and "there are no
    /// cells" are different things.
    pub values_agree: Option<bool>,
    /// Whether the cached category labels and the cells agree. `None` as above.
    pub categories_agree: Option<bool>,
}

/// One cell a chart's `c:f` reaches, and where it sits in that reference.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeCellInfo {
    /// The cell's zero-based position **within the whole reference** — the index a cache's
    /// `c:pt@idx` uses, which is what lets a cache and a resolution be compared without padding
    /// either with blanks.
    pub offset: u64,
    /// The tab the cell is on.
    pub sheet: u32,
    /// Where on that tab, in A1 text.
    pub reference: String,
    /// The cell's value as a number, or `None` for one that is not a number.
    pub number: Option<f64>,
    /// What a category axis would show for the cell — the text of a string, the shortest
    /// round-tripping spelling of a number, `TRUE`/`FALSE` for a boolean, the code of an error.
    pub label: String,
}

/// A chart's `c:f`, resolved against this workbook's cells.
///
/// A **blank cell is absent** from [`cells`](Self::cells) rather than present-and-zero, which is the
/// shape the caches themselves have: a `c:numCache` writes a `c:pt` for the points it has and omits
/// the rest. So a range naming a million addressable cells costs what its populated cells cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRangeInfo {
    /// The reference exactly as the chart wrote it.
    pub reference: String,
    /// How many cells the reference **addresses**, blanks included.
    pub addressed_cells: u64,
    /// Whether every area of the reference resolved.
    pub fully_resolved: bool,
    /// Why the first area that failed did, in words, or `None` when none did.
    pub problem: Option<String>,
    /// The cells that hold something, in reference order.
    pub cells: Vec<RangeCellInfo>,
}

/// Where one series of a chart authored by [`Workbook::add_range_chart`] takes its data from.
///
/// Every field is reference **text**, exactly as it will be written into the chart's `c:f` —
/// `"Data!$B$2:$B$4"`. Each is resolved before anything is written, so a reference naming a sheet
/// this workbook does not have is refused rather than written as a `c:f` pointing at nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartRangeSeries {
    /// The cell the series takes its name from, or `None` for a literal name.
    pub name_cell: Option<String>,
    /// The name to use when `name_cell` is `None`, or when the cell it names holds nothing.
    pub name: String,
    /// The cells the series' values come from.
    pub values: String,
}

impl ChartRangeSeries {
    /// A series taking its values from `values` and its name from the literal `name`.
    #[must_use]
    pub fn new(name: impl Into<String>, values: impl Into<String>) -> Self {
        Self {
            name_cell: None,
            name: name.into(),
            values: values.into(),
        }
    }

    /// The same series, taking its name from the cell `reference` names instead.
    #[must_use]
    pub fn named_by_cell(mut self, reference: impl Into<String>) -> Self {
        self.name_cell = Some(reference.into());
        self
    }
}

impl Workbook {
    // ---------------------------------------------------------------------------------------------
    // Finding
    // ---------------------------------------------------------------------------------------------

    /// The index of every anchor on `sheet` that frames a chart, in paint order — the Excel
    /// counterpart of walking a slide's shapes looking for chart frames.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab.
    ///
    /// See [`Workbook::chart_anchor_indices`](mjx_xlsx::Workbook::chart_anchor_indices).
    pub fn chart_anchor_indices(&mut self, sheet: u32) -> Result<Vec<u32>, Error> {
        Ok(self
            .workbook
            .chart_anchor_indices(index(sheet))?
            .into_iter()
            .map(count)
            .collect())
    }

    /// The relationship id the anchor names as its chart part, or `None` when that anchor frames no
    /// chart.
    ///
    /// # Errors
    /// As [`chart_anchor_indices`](Self::chart_anchor_indices).
    ///
    /// See [`Workbook::chart_rel_id`](mjx_xlsx::Workbook::chart_rel_id).
    pub fn chart_rel_id(&mut self, sheet: u32, anchor: u32) -> Result<Option<String>, Error> {
        Ok(self.workbook.chart_rel_id(index(sheet), index(anchor))?)
    }

    /// The raw XML bytes of the chart part the anchor frames, or `None` when it frames no chart.
    ///
    /// # Errors
    /// As [`chart_anchor_indices`](Self::chart_anchor_indices).
    ///
    /// See [`Workbook::chart_part_bytes`](mjx_xlsx::Workbook::chart_part_bytes).
    pub fn chart_part_bytes(&mut self, sheet: u32, anchor: u32) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .workbook
            .chart_part_bytes(index(sheet), index(anchor))?
            .map(<[u8]>::to_vec))
    }

    /// The series of the chart, from its **caches**.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_series`](mjx_xlsx::Workbook::chart_series).
    pub fn chart_series(&mut self, sheet: u32, anchor: u32) -> Result<Vec<ChartSeriesData>, Error> {
        Ok(self.workbook.chart_series(index(sheet), index(anchor))?)
    }

    /// The kind of every plot the chart draws, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_kinds`](mjx_xlsx::Workbook::chart_kinds).
    pub fn chart_kinds(&mut self, sheet: u32, anchor: u32) -> Result<Vec<ChartKind>, Error> {
        Ok(self.workbook.chart_kinds(index(sheet), index(anchor))?)
    }

    /// The axes of the chart, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_axes`](mjx_xlsx::Workbook::chart_axes).
    pub fn chart_axes(&mut self, sheet: u32, anchor: u32) -> Result<Vec<ChartAxisData>, Error> {
        Ok(self.workbook.chart_axes(index(sheet), index(anchor))?)
    }

    /// The heading of the chart, or `None` when it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_title`](mjx_xlsx::Workbook::chart_title).
    pub fn chart_title(&mut self, sheet: u32, anchor: u32) -> Result<Option<String>, Error> {
        Ok(self.workbook.chart_title(index(sheet), index(anchor))?)
    }

    /// The legend of the chart, or `None` when it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_legend`](mjx_xlsx::Workbook::chart_legend).
    pub fn chart_legend(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Option<ChartLegendData>, Error> {
        Ok(self.workbook.chart_legend(index(sheet), index(anchor))?)
    }

    /// The built-in style id the chart names (1 to 48), or `None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_style_id`](mjx_xlsx::Workbook::chart_style_id).
    pub fn chart_style_id(&mut self, sheet: u32, anchor: u32) -> Result<Option<u32>, Error> {
        Ok(self.workbook.chart_style_id(index(sheet), index(anchor))?)
    }

    /// The fill of series `series` — what colour it is drawn in — or `None` when it declares none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_series_fill`](mjx_xlsx::Workbook::chart_series_fill).
    pub fn chart_series_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self
            .workbook
            .chart_series_fill(index(sheet), index(anchor), index(series))?)
    }

    /// Every point of series `series` that carries its own formatting, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_point_formats`](mjx_xlsx::Workbook::chart_point_formats).
    pub fn chart_point_formats(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<Vec<ChartPointFormatData>, Error> {
        Ok(self
            .workbook
            .chart_point_formats(index(sheet), index(anchor), index(series))?)
    }

    /// Every trendline fitted through series `series`, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_trendlines`](mjx_xlsx::Workbook::chart_trendlines).
    pub fn chart_trendlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<Vec<ChartTrendlineData>, Error> {
        Ok(self
            .workbook
            .chart_trendlines(index(sheet), index(anchor), index(series))?)
    }

    /// Every set of error bars series `series` carries.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_error_bars`](mjx_xlsx::Workbook::chart_error_bars).
    pub fn chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<Vec<ChartErrorBarData>, Error> {
        Ok(self
            .workbook
            .chart_error_bars(index(sheet), index(anchor), index(series))?)
    }

    /// Every `c:dPt` and `c:dLbl` of series `series` naming a point the series no longer has.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_dangling_decoration`](mjx_xlsx::Workbook::chart_dangling_decoration).
    pub fn chart_dangling_decoration(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<Vec<DanglingPointReference>, Error> {
        Ok(self
            .workbook
            .chart_dangling_decoration(index(sheet), index(anchor), index(series))?)
    }

    // ---------------------------------------------------------------------------------------------
    // Data labels
    // ---------------------------------------------------------------------------------------------

    /// The data-label settings **in force** for one point of series `series` — the point's own merged over the series' merged over the owning plot's. Pass `None` for `point` to stop at the series tier.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_data_labels`](mjx_xlsx::Workbook::chart_data_labels).
    pub fn chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: Option<u32>,
    ) -> Result<DataLabelSettings, Error> {
        Ok(self
            .workbook
            .chart_data_labels(index(sheet), index(anchor), index(series), point)?)
    }

    /// The data-label settings one **tier** states in its own right, with everything it leaves unset reported as `None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_data_label_tier`](mjx_xlsx::Workbook::chart_data_label_tier).
    pub fn chart_data_label_tier(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, Error> {
        Ok(self
            .workbook
            .chart_data_label_tier(index(sheet), index(anchor), scope)?)
    }

    /// The words one point's label shows in place of its value, or `None` when it states none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::chart_point_label_text`](mjx_xlsx::Workbook::chart_point_label_text).
    pub fn chart_point_label_text(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self.workbook.chart_point_label_text(
            index(sheet),
            index(anchor),
            index(series),
            point,
        )?)
    }

    // ---------------------------------------------------------------------------------------------
    // Edits
    // ---------------------------------------------------------------------------------------------

    /// Rewrites the values of series `series`, refreshing the chart's embedded workbook alongside it **when there is one** — a chart whose data is a live range has none, and none is fabricated.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_series_values`](mjx_xlsx::Workbook::set_chart_series_values).
    pub fn set_chart_series_values(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        values: &[f64],
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_series_values(
            index(sheet),
            index(anchor),
            index(series),
            values,
        )?)
    }

    /// Rewrites the category labels of series `series`, refreshing the embedded workbook alongside it when there is one.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_series_categories`](mjx_xlsx::Workbook::set_chart_series_categories).
    pub fn set_chart_series_categories(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        labels: &[&str],
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_series_categories(
            index(sheet),
            index(anchor),
            index(series),
            labels,
        )?)
    }

    /// Sets or clears the explicit bounds of axis `axis`. `None` returns that end to automatic scaling.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_axis_scale`](mjx_xlsx::Workbook::set_chart_axis_scale).
    pub fn set_chart_axis_scale(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_axis_scale(
            index(sheet),
            index(anchor),
            index(axis),
            minimum,
            maximum,
        )?)
    }

    /// Sets the direction of axis `axis` — smallest value first, or reversed.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_axis_orientation`](mjx_xlsx::Workbook::set_chart_axis_orientation).
    pub fn set_chart_axis_orientation(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis: u32,
        orientation: AxisOrientation,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_axis_orientation(
            index(sheet),
            index(anchor),
            index(axis),
            orientation,
        )?)
    }

    /// Sets or removes the title of axis `axis`. `None` removes it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_axis_title`](mjx_xlsx::Workbook::set_chart_axis_title).
    pub fn set_chart_axis_title(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis: u32,
        text: Option<&str>,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_axis_title(index(sheet), index(anchor), index(axis), text)?)
    }

    /// Turns the gridlines of axis `axis` on or off.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_axis_gridlines`](mjx_xlsx::Workbook::set_chart_axis_gridlines).
    pub fn set_chart_axis_gridlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        axis: u32,
        major: bool,
        minor: bool,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_axis_gridlines(
            index(sheet),
            index(anchor),
            index(axis),
            major,
            minor,
        )?)
    }

    /// Sets or removes the chart's heading. `None` removes it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_title`](mjx_xlsx::Workbook::set_chart_title).
    pub fn set_chart_title(
        &mut self,
        sheet: u32,
        anchor: u32,
        text: Option<&str>,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_title(index(sheet), index(anchor), text)?)
    }

    /// Places the chart's legend at `position`, adding one if it had none. `None` removes it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_legend`](mjx_xlsx::Workbook::set_chart_legend).
    pub fn set_chart_legend(
        &mut self,
        sheet: u32,
        anchor: u32,
        position: Option<LegendPosition>,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_legend(index(sheet), index(anchor), position)?)
    }

    /// Sets the fill of series `series`. A picture fill is refused: a chart part relates to no images.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_series_fill`](mjx_xlsx::Workbook::set_chart_series_fill).
    pub fn set_chart_series_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_series_fill(index(sheet), index(anchor), index(series), fill)?)
    }

    /// Sets the outline of series `series` — the line a line or radar plot draws, or the border of a bar or area.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_series_line`](mjx_xlsx::Workbook::set_chart_series_line).
    pub fn set_chart_series_line(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_series_line(index(sheet), index(anchor), index(series), line)?)
    }

    /// Applies `spec` at one tier of the chart's data labels, leaving every setting it does not state alone.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_data_labels`](mjx_xlsx::Workbook::set_chart_data_labels).
    pub fn set_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_data_labels(index(sheet), index(anchor), scope, spec)?)
    }

    /// Suppresses the labels at one tier — *draw nothing here*, as against *say nothing here*.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::suppress_chart_data_labels`](mjx_xlsx::Workbook::suppress_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: ChartLabelScope,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .suppress_chart_data_labels(index(sheet), index(anchor), scope)?)
    }

    /// Removes the label settings at one tier entirely, so that tier inherits the one above it again. Answers whether an element was there.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::remove_chart_data_labels`](mjx_xlsx::Workbook::remove_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        sheet: u32,
        anchor: u32,
        scope: ChartLabelScope,
    ) -> Result<bool, Error> {
        Ok(self
            .workbook
            .remove_chart_data_labels(index(sheet), index(anchor), scope)?)
    }

    /// Colours one point of series `series` differently from the rest of its series.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_point_fill`](mjx_xlsx::Workbook::set_chart_point_fill).
    pub fn set_chart_point_fill(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_point_fill(
            index(sheet),
            index(anchor),
            index(series),
            point,
            fill,
        )?)
    }

    /// Outlines one point of series `series` differently from the rest of its series.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_point_line`](mjx_xlsx::Workbook::set_chart_point_line).
    pub fn set_chart_point_line(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_point_line(
            index(sheet),
            index(anchor),
            index(series),
            point,
            line,
        )?)
    }

    /// Pulls one slice of a pie or doughnut out of the centre by `percent` of the radius, or puts it back.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_point_explosion`](mjx_xlsx::Workbook::set_chart_point_explosion).
    pub fn set_chart_point_explosion(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: u32,
        percent: Option<u32>,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_point_explosion(
            index(sheet),
            index(anchor),
            index(series),
            point,
            percent,
        )?)
    }

    /// Removes the formatting of one point, so it is drawn like the rest of its series. Answers whether any was there.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::remove_chart_point_format`](mjx_xlsx::Workbook::remove_chart_point_format).
    pub fn remove_chart_point_format(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        point: u32,
    ) -> Result<bool, Error> {
        Ok(self.workbook.remove_chart_point_format(
            index(sheet),
            index(anchor),
            index(series),
            point,
        )?)
    }

    /// Fits a trendline through series `series`. Trendlines repeat, so this **appends**.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::add_chart_trendline`](mjx_xlsx::Workbook::add_chart_trendline).
    pub fn add_chart_trendline(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .add_chart_trendline(index(sheet), index(anchor), index(series), spec)?)
    }

    /// Rewrites one trendline of series `series` from `spec`, **in place**.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_trendline`](mjx_xlsx::Workbook::set_chart_trendline).
    pub fn set_chart_trendline(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        trendline: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self.workbook.set_chart_trendline(
            index(sheet),
            index(anchor),
            index(series),
            index(trendline),
            spec,
        )?)
    }

    /// Gives series `series` error bars, replacing an existing set that runs along the same axis.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::set_chart_error_bars`](mjx_xlsx::Workbook::set_chart_error_bars).
    pub fn set_chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
        spec: &ErrorBarSpec,
    ) -> Result<(), Error> {
        Ok(self
            .workbook
            .set_chart_error_bars(index(sheet), index(anchor), index(series), spec)?)
    }

    /// Removes every trendline from series `series`, answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::remove_chart_trendlines`](mjx_xlsx::Workbook::remove_chart_trendlines).
    pub fn remove_chart_trendlines(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.remove_chart_trendlines(
            index(sheet),
            index(anchor),
            index(series),
        )?))
    }

    /// Removes every set of error bars from series `series`, answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::remove_chart_error_bars`](mjx_xlsx::Workbook::remove_chart_error_bars).
    pub fn remove_chart_error_bars(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.remove_chart_error_bars(
            index(sheet),
            index(anchor),
            index(series),
        )?))
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series` that names a point past the end of its data, answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::drop_chart_dangling_decoration`](mjx_xlsx::Workbook::drop_chart_dangling_decoration).
    pub fn drop_chart_dangling_decoration(
        &mut self,
        sheet: u32,
        anchor: u32,
        series: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.drop_chart_dangling_decoration(
            index(sheet),
            index(anchor),
            index(series),
        )?))
    }

    // ---------------------------------------------------------------------------------------------
    // Authoring
    // ---------------------------------------------------------------------------------------------

    /// Anchors `chart` between two cells on `sheet`, **with the embedded workbook** that Office's
    /// *Edit Data* opens, and answers the anchor's position in the drawing's paint order.
    ///
    /// The cross-format door: `chart` is the same [`ChartData`] a slide and a document take, so a
    /// caller who built one for a deck adds it to a workbook unchanged. Because a `ChartData` carries
    /// *values* and no cells to point at, the chart's formulas name the workbook this call writes
    /// beside it — exactly as they do in a `.pptx` and a `.docx`. For the case Excel itself writes,
    /// use [`add_range_chart`](Self::add_range_chart).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Workbook::add_chart`](mjx_xlsx::Workbook::add_chart).
    #[allow(clippy::too_many_arguments)]
    pub fn add_chart(
        &mut self,
        sheet: u32,
        chart: &ChartData,
        from_column: u32,
        from_row: u32,
        to_column: u32,
        to_row: u32,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.add_chart(
            index(sheet),
            chart,
            marker(from_column, from_row),
            marker(to_column, to_row),
            name,
            resizing,
        )?))
    }

    /// Anchors a chart of `kind` between two cells on `sheet`, taking its data from **cells in this
    /// workbook**, and answers the anchor's position in the drawing's paint order.
    ///
    /// The case that exists nowhere else in this library: the chart's formulas name the ranges
    /// `series` and `categories` give, its caches are seeded from what those cells say **right now**,
    /// and **no embedded workbook is written** — so
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) answers `false` for the result and
    /// [`chart_series_freshness`](Self::chart_series_freshness) can tell a caller when the sheet has
    /// moved on.
    ///
    /// Every reference is resolved before anything is written, so a range naming a sheet this
    /// workbook does not have is refused rather than written as a formula pointing at nothing.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure — a reference that
    /// names nothing is [`ErrorCode::NotFound`](crate::ErrorCode::NotFound).
    ///
    /// See [`Workbook::add_range_chart`](mjx_xlsx::Workbook::add_range_chart).
    #[allow(clippy::too_many_arguments)]
    pub fn add_range_chart(
        &mut self,
        sheet: u32,
        kind: ChartKind,
        categories: Option<&str>,
        series: &[ChartRangeSeries],
        from_column: u32,
        from_row: u32,
        to_column: u32,
        to_row: u32,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<u32, Error> {
        let source = mjx_xlsx::SheetChartSource {
            categories: categories.map(str::to_owned),
            series: series
                .iter()
                .map(|entry| mjx_xlsx::SheetChartSeries {
                    name_cell: entry.name_cell.clone(),
                    name: entry.name.clone(),
                    values: entry.values.clone(),
                })
                .collect(),
        };
        Ok(count(self.workbook.add_range_chart(
            index(sheet),
            kind,
            &source,
            marker(from_column, from_row),
            marker(to_column, to_row),
            name,
            resizing,
        )?))
    }

    // ---------------------------------------------------------------------------------------------
    // The embedded workbook — and the chart that has none
    // ---------------------------------------------------------------------------------------------

    /// Every chart in the workbook that references a backing workbook, with the anchor that frames it
    /// and whether the reference is external.
    ///
    /// A chart on a sheet usually has none, and is absent from this list rather than present with an
    /// empty target.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::chart_workbooks`](mjx_xlsx::Workbook::chart_workbooks).
    pub fn chart_workbooks(&mut self) -> Result<Vec<SheetChartWorkbookInfo>, Error> {
        Ok(self
            .workbook
            .chart_workbooks()?
            .into_iter()
            .map(|entry| SheetChartWorkbookInfo {
                sheet: count(entry.sheet_index),
                anchor: count(entry.anchor_index),
                target: entry.target,
                external: entry.external,
            })
            .collect())
    }

    /// Rewrites the embedded workbook of the chart so its cells hold exactly what the chart now
    /// draws, and answers whether it rewrote one.
    ///
    /// Answers `false`, changing nothing, when there is nothing to refresh — which includes the
    /// **ordinary** state of a chart on a sheet, whose data is a live range and which has no embedded
    /// copy at all. No workbook is ever fabricated.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::refresh_chart_workbook`](mjx_xlsx::Workbook::refresh_chart_workbook).
    pub fn refresh_chart_workbook(&mut self, sheet: u32, anchor: u32) -> Result<bool, Error> {
        Ok(self
            .workbook
            .refresh_chart_workbook(index(sheet), index(anchor))?)
    }

    /// Detaches the backing workbook from the chart, leaving it to render from its cached values.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure —
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) when the chart references no
    /// workbook, which is the ordinary state of a chart on a sheet.
    ///
    /// See [`Workbook::detach_chart_workbook`](mjx_xlsx::Workbook::detach_chart_workbook).
    pub fn detach_chart_workbook(&mut self, sheet: u32, anchor: u32) -> Result<(), Error> {
        Ok(self
            .workbook
            .detach_chart_workbook(index(sheet), index(anchor))?)
    }

    // ---------------------------------------------------------------------------------------------
    // The live range
    // ---------------------------------------------------------------------------------------------

    /// Where every series of the chart says its data lives — the formula beside each cache, as the
    /// file wrote it.
    ///
    /// The companion of [`chart_series`](Self::chart_series): that answers what the **caches** hold,
    /// this answers what the references **name**.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::chart_series_references`](mjx_xlsx::Workbook::chart_series_references).
    pub fn chart_series_references(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesReferences>, Error> {
        Ok(self
            .workbook
            .chart_series_references(index(sheet), index(anchor))?)
    }

    /// Every series of the chart, read **from the cells its formulas name** rather than from its
    /// caches.
    ///
    /// The same shape [`chart_series`](Self::chart_series) answers, so the two can be compared
    /// directly — which is what [`chart_series_freshness`](Self::chart_series_freshness) does.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::chart_series_from_cells`](mjx_xlsx::Workbook::chart_series_from_cells).
    pub fn chart_series_from_cells(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesData>, Error> {
        Ok(self
            .workbook
            .chart_series_from_cells(index(sheet), index(anchor))?)
    }

    /// Every series' cache set beside what its cells actually say, with each named.
    ///
    /// **The cache is what draws until a consumer recalculates**, and the cells are what a consumer
    /// would recalculate from. Neither is silently preferred: a caller that wants the sheet's answer
    /// to win calls [`refresh_chart_cache_from_cells`](Self::refresh_chart_cache_from_cells), and one
    /// that wants the drawn answer to win does nothing.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::chart_series_freshness`](mjx_xlsx::Workbook::chart_series_freshness).
    pub fn chart_series_freshness(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<Vec<ChartSeriesFreshnessInfo>, Error> {
        Ok(self
            .workbook
            .chart_series_freshness(index(sheet), index(anchor))?
            .into_iter()
            .map(|series| ChartSeriesFreshnessInfo {
                series_index: count(series.series_index),
                cached: series.cached,
                references: series.references,
                from_cells: series.from_cells,
                values_problem: series.values_problem.as_ref().map(ToString::to_string),
                categories_problem: series.categories_problem.as_ref().map(ToString::to_string),
                values_agree: series.values_agree,
                categories_agree: series.categories_agree,
            })
            .collect())
    }

    /// Rewrites the chart's caches from the cells its formulas name, and answers how many series it
    /// changed.
    ///
    /// The **opt-in** repair, and the exact counterpart of
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) pointing the other way: that one
    /// makes the workbook say what the chart draws, this one makes the chart draw what the sheet
    /// says. Nothing calls it for you — writing a cell deliberately leaves the caches alone.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Workbook::refresh_chart_cache_from_cells`](mjx_xlsx::Workbook::refresh_chart_cache_from_cells).
    pub fn refresh_chart_cache_from_cells(
        &mut self,
        sheet: u32,
        anchor: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.workbook.refresh_chart_cache_from_cells(
            index(sheet),
            index(anchor),
        )?))
    }

    /// Resolves `reference` — a chart's formula, a defined name, anything of that shape — against
    /// this workbook's cells, with `sheet` as the tab an area that names none means.
    ///
    /// It resolves a **reference**, and does not evaluate a formula: working out that
    /// `Data!$B$2:$B$4` names three cells is address arithmetic, and working out what `=SUM(B2:B4)`
    /// comes to is a calculation engine this library deliberately does not have. A cell holding a
    /// formula answers with its cached value.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `sheet` names no tab, or
    /// another [`Error`] if a part this had to open is malformed. A reference that cannot be
    /// *resolved* is not an error: it comes back with [`fully_resolved`](ResolvedRangeInfo::fully_resolved)
    /// `false` and the reason in [`problem`](ResolvedRangeInfo::problem).
    ///
    /// See [`Workbook::resolve_range_reference`](mjx_xlsx::Workbook::resolve_range_reference).
    pub fn resolve_range_reference(
        &mut self,
        sheet: u32,
        reference: &str,
    ) -> Result<ResolvedRangeInfo, Error> {
        let resolved = self
            .workbook
            .resolve_range_reference(index(sheet), reference)?;
        Ok(ResolvedRangeInfo {
            reference: resolved.reference.clone(),
            addressed_cells: resolved.addressed_cells,
            fully_resolved: resolved.is_fully_resolved(),
            problem: resolved.problem().map(ToString::to_string),
            cells: resolved
                .cells
                .iter()
                .map(|cell| RangeCellInfo {
                    offset: cell.offset,
                    sheet: count(cell.sheet_index),
                    reference: cell.reference.text().as_str().to_owned(),
                    number: cell.value.number(),
                    label: cell.value.label(),
                })
                .collect(),
        })
    }
}

/// A `xdr:from`/`xdr:to` marker at a cell's own top-left corner.
///
/// The facade takes an anchor as two numbers rather than as a four-field marker tree, the same rule
/// [`crate::workbook::drawings`] follows — a caller that wants an offset *into* a cell reaches for
/// the drawing surface, where a chart frame is an anchor like any other.
fn marker(column: u32, row: u32) -> mjx_dml::CellMarker {
    mjx_dml::CellMarker::new(axis(column), 0, axis(row), 0)
}

/// A row or column index as the wire states it, clamped to the non-negative range the schema allows.
fn axis(index: u32) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// Where a chart on a sheet keeps its backing workbook, and whether that reference points outside the
/// package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetChartWorkbookInfo {
    /// The tab the chart is anchored on.
    pub sheet: u32,
    /// The anchor that frames it, in the drawing part's paint order.
    pub anchor: u32,
    /// The relationship target, as written.
    pub target: String,
    /// Whether the reference points outside the package, in which case the workbook is not this
    /// package's to rewrite.
    pub external: bool,
}
