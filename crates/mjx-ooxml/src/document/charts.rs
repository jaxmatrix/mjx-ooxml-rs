//! Charts on the Word surface — the same family, under the same names, as [`crate::deck::charts`]
//! and [`crate::deck::chart_decoration`] carry for a presentation.
//!
//! Every method here delegates to the identically-named method on [`mjx_docx::Document`], which in
//! turn calls the identically-named function in `mjx_chart::chart_ops` — so "the same chart
//! vocabulary reads, authors and edits from a presentation and a document" is one body of code with
//! two ways in, not two implementations that happen to agree.
//!
//! # The one difference from the `Deck` family, and why it is forced
//!
//! A `Deck` addresses a chart as `(surface, shape)`; a `Document` addresses it by the drawing's own
//! `wp:docPr@id`, a single `u32`. A document has no shape tree to index into — what it has is the
//! id every `w:drawing` carries, which is already this surface's drawing address
//! ([`Document::add_inline_picture`](crate::Document::add_inline_picture) returns one and
//! [`Document::remove_drawing`](crate::Document::remove_drawing) takes one). Everything after that
//! first argument — argument order, method names, return shapes — is identical.

use std::borrow::Cow;

use crate::error::Error;
use crate::index::{count, index};
use crate::{
    AxisOrientation, ChartAxisData, ChartData, ChartErrorBarData, ChartKind, ChartLabelScope,
    ChartLegendData, ChartPointFormatData, ChartSeriesData, ChartSeriesReferences,
    ChartTrendlineData, DanglingPointReference, DataLabelSettings, DataLabelSpec,
    DocumentChartWorkbook, ErrorBarSpec, FillSpec, LegendPosition, LineSpec, TrendlineSpec,
};

use super::BlockPath;

impl super::Document {
    // ---------------------------------------------------------------------------------------------
    // Finding, and authoring
    // ---------------------------------------------------------------------------------------------

    /// The `wp:docPr` id of every drawing in the document body that frames a chart, in document
    /// order — the Word counterpart of walking a slide's shapes looking for chart frames.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    ///
    /// See [`Document::chart_drawing_ids`](mjx_docx::Document::chart_drawing_ids).
    pub fn chart_drawing_ids(&mut self) -> Result<Vec<u32>, Error> {
        Ok(self.document.chart_drawing_ids()?)
    }

    /// The relationship id the drawing `drawing_id` names as its chart part, or `None` when that
    /// drawing frames no chart.
    ///
    /// # Errors
    /// As [`chart_drawing_ids`](Self::chart_drawing_ids).
    ///
    /// See [`Document::chart_rel_id`](mjx_docx::Document::chart_rel_id).
    pub fn chart_rel_id(&mut self, drawing_id: u32) -> Result<Option<String>, Error> {
        Ok(self.document.chart_rel_id(drawing_id)?)
    }

    /// The raw XML bytes of the chart part the drawing `drawing_id` references, or `None` when that
    /// drawing frames no chart.
    ///
    /// # Errors
    /// As [`chart_drawing_ids`](Self::chart_drawing_ids).
    ///
    /// See [`Document::chart_part_bytes`](mjx_docx::Document::chart_part_bytes).
    pub fn chart_part_bytes(&mut self, drawing_id: u32) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .document
            .chart_part_bytes(drawing_id)?
            .map(Cow::into_owned))
    }

    /// Adds `chart` to the document as a new **inline** chart, `width_emu` by `height_emu`, appended
    /// as a new run at the end of the paragraph at `paragraph`. Returns the drawing's own `wp:docPr`
    /// id, which every other method here takes.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Document::add_chart`](mjx_docx::Document::add_chart).
    pub fn add_chart(
        &mut self,
        paragraph: BlockPath,
        chart: &ChartData,
        width_emu: i64,
        height_emu: i64,
        name: &str,
    ) -> Result<u32, Error> {
        Ok(self
            .document
            .add_chart(paragraph.to_model(), chart, width_emu, height_emu, name)?)
    }

    /// Adds `chart` as a **floating** chart, offset `offset_x_emu`/`offset_y_emu` from the
    /// paragraph's own origin, with the text wrapping around it as `wrap` says.
    ///
    /// The wrap modes are the three a rectangle needs — `None`, `Square` and `TopAndBottom`; see
    /// [`ChartWrap`](mjx_docx::ChartWrap) for why the two polygon wraps are not among them.
    ///
    /// # Errors
    /// As [`add_chart`](Self::add_chart).
    ///
    /// See [`Document::add_chart_placed`](mjx_docx::Document::add_chart_placed).
    #[allow(clippy::too_many_arguments)]
    pub fn add_floating_chart(
        &mut self,
        paragraph: BlockPath,
        chart: &ChartData,
        offset_x_emu: i64,
        offset_y_emu: i64,
        width_emu: i64,
        height_emu: i64,
        wrap: mjx_docx::ChartWrap,
        name: &str,
    ) -> Result<u32, Error> {
        Ok(self.document.add_chart_placed(
            paragraph.to_model(),
            chart,
            width_emu,
            height_emu,
            name,
            mjx_docx::ChartPlacement::Floating {
                offset_x_emu,
                offset_y_emu,
                wrap,
            },
        )?)
    }

    // ---------------------------------------------------------------------------------------------
    // The embedded workbook
    // ---------------------------------------------------------------------------------------------

    /// Every chart in the document that references a backing workbook, with the drawing that frames
    /// it and whether the reference is external.
    ///
    /// # Errors
    /// As [`chart_drawing_ids`](Self::chart_drawing_ids).
    ///
    /// See [`Document::chart_workbooks`](mjx_docx::Document::chart_workbooks).
    pub fn chart_workbooks(&mut self) -> Result<Vec<DocumentChartWorkbook>, Error> {
        Ok(self.document.chart_workbooks()?)
    }

    /// Writes the chart's data into the workbook the chart the drawing `drawing_id` frames already
    /// embeds — the cells its own `c:f` formulas name, and nothing else — and answers whether it
    /// wrote one.
    ///
    /// Every other sheet, format and name that workbook carried survives; a reference this library
    /// will not write is refused rather than written over. Use
    /// [`regenerate_chart_workbook`](Self::regenerate_chart_workbook) to replace the workbook
    /// wholesale instead.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::refresh_chart_workbook`](mjx_docx::Document::refresh_chart_workbook).
    pub fn refresh_chart_workbook(&mut self, drawing_id: u32) -> Result<bool, Error> {
        Ok(self.document.refresh_chart_workbook(drawing_id)?)
    }

    /// Replaces the embedded workbook of the chart the drawing `drawing_id` frames with a freshly
    /// built one, and answers whether it replaced one.
    ///
    /// **This discards whatever that workbook held** — every extra sheet, cell format, defined name
    /// and macro. It is the explicit opt-in;
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) is the preserving default.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::regenerate_chart_workbook`](mjx_docx::Document::regenerate_chart_workbook).
    pub fn regenerate_chart_workbook(&mut self, drawing_id: u32) -> Result<bool, Error> {
        Ok(self.document.regenerate_chart_workbook(drawing_id)?)
    }

    /// Detaches the backing workbook from the chart the drawing `drawing_id` frames, leaving it to
    /// render from its cached values.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::detach_chart_workbook`](mjx_docx::Document::detach_chart_workbook).
    pub fn detach_chart_workbook(&mut self, drawing_id: u32) -> Result<(), Error> {
        Ok(self.document.detach_chart_workbook(drawing_id)?)
    }

    // ---------------------------------------------------------------------------------------------
    // Reads
    // ---------------------------------------------------------------------------------------------

    /// The series of the chart the drawing `drawing_id` frames.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_series`](mjx_docx::Document::chart_series).
    pub fn chart_series(&mut self, drawing_id: u32) -> Result<Vec<ChartSeriesData>, Error> {
        Ok(self.document.chart_series(drawing_id)?)
    }

    /// The kind of every plot the chart draws, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_kinds`](mjx_docx::Document::chart_kinds).
    pub fn chart_kinds(&mut self, drawing_id: u32) -> Result<Vec<ChartKind>, Error> {
        Ok(self.document.chart_kinds(drawing_id)?)
    }

    /// Where every series of the chart says its data lives — the formula beside each cache, as the
    /// file wrote it.
    ///
    /// The companion of [`chart_series`](Self::chart_series): that answers what the **caches** hold,
    /// this answers what the references **name**.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_series_references`](mjx_docx::Document::chart_series_references).
    pub fn chart_series_references(
        &mut self,
        drawing_id: u32,
    ) -> Result<Vec<ChartSeriesReferences>, Error> {
        Ok(self.document.chart_series_references(drawing_id)?)
    }

    /// The axes of the chart, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_axes`](mjx_docx::Document::chart_axes).
    pub fn chart_axes(&mut self, drawing_id: u32) -> Result<Vec<ChartAxisData>, Error> {
        Ok(self.document.chart_axes(drawing_id)?)
    }

    /// The heading of the chart, or `None` when it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_title`](mjx_docx::Document::chart_title).
    pub fn chart_title(&mut self, drawing_id: u32) -> Result<Option<String>, Error> {
        Ok(self.document.chart_title(drawing_id)?)
    }

    /// The legend of the chart, or `None` when it has none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_legend`](mjx_docx::Document::chart_legend).
    pub fn chart_legend(&mut self, drawing_id: u32) -> Result<Option<ChartLegendData>, Error> {
        Ok(self.document.chart_legend(drawing_id)?)
    }

    /// The built-in style id the chart names (`c:style@val`, 1 to 48), or `None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_style_id`](mjx_docx::Document::chart_style_id).
    pub fn chart_style_id(&mut self, drawing_id: u32) -> Result<Option<u32>, Error> {
        Ok(self.document.chart_style_id(drawing_id)?)
    }

    /// The fill of series `series_idx`, or `None` when it takes its colour from the chart style.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_series_fill`](mjx_docx::Document::chart_series_fill).
    pub fn chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self
            .document
            .chart_series_fill(drawing_id, index(series_idx))?)
    }

    /// The data-label settings **in force** for one point of series `series_idx`. Pass
    /// `point_idx = None` to stop at the series tier.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_data_labels`](mjx_docx::Document::chart_data_labels).
    pub fn chart_data_labels(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, Error> {
        Ok(self
            .document
            .chart_data_labels(drawing_id, index(series_idx), point_idx)?)
    }

    /// The data-label settings one **tier** states in its own right.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_data_label_tier`](mjx_docx::Document::chart_data_label_tier).
    pub fn chart_data_label_tier(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, Error> {
        Ok(self.document.chart_data_label_tier(drawing_id, scope)?)
    }

    /// The words one point's label shows in place of its value, or `None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_point_label_text`](mjx_docx::Document::chart_point_label_text).
    pub fn chart_point_label_text(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self
            .document
            .chart_point_label_text(drawing_id, index(series_idx), point_idx)?)
    }

    /// Every point of series `series_idx` that carries its own formatting, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_point_formats`](mjx_docx::Document::chart_point_formats).
    pub fn chart_point_formats(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartPointFormatData>, Error> {
        Ok(self
            .document
            .chart_point_formats(drawing_id, index(series_idx))?)
    }

    /// Every trendline fitted through series `series_idx`, in document order.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_trendlines`](mjx_docx::Document::chart_trendlines).
    pub fn chart_trendlines(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartTrendlineData>, Error> {
        Ok(self
            .document
            .chart_trendlines(drawing_id, index(series_idx))?)
    }

    /// Every set of error bars series `series_idx` carries.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_error_bars`](mjx_docx::Document::chart_error_bars).
    pub fn chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<Vec<ChartErrorBarData>, Error> {
        Ok(self
            .document
            .chart_error_bars(drawing_id, index(series_idx))?)
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` naming a point the series no longer has.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::chart_dangling_decoration`](mjx_docx::Document::chart_dangling_decoration).
    pub fn chart_dangling_decoration(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<Vec<DanglingPointReference>, Error> {
        Ok(self
            .document
            .chart_dangling_decoration(drawing_id, index(series_idx))?)
    }

    // ---------------------------------------------------------------------------------------------
    // Edits
    // ---------------------------------------------------------------------------------------------

    /// Rewrites the values of series `series_idx`, refreshing the embedded workbook in the same
    /// call.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_series_values`](mjx_docx::Document::set_chart_series_values).
    pub fn set_chart_series_values(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        values: &[f64],
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_series_values(drawing_id, index(series_idx), values)?)
    }

    /// Rewrites the category labels of series `series_idx`, refreshing the embedded workbook in the
    /// same call.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_series_categories`](mjx_docx::Document::set_chart_series_categories).
    pub fn set_chart_series_categories(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        labels: &[&str],
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_series_categories(drawing_id, index(series_idx), labels)?)
    }

    /// Sets or clears the explicit bounds of axis `axis_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_axis_scale`](mjx_docx::Document::set_chart_axis_scale).
    pub fn set_chart_axis_scale(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_axis_scale(drawing_id, index(axis_idx), minimum, maximum)?)
    }

    /// Sets the direction of axis `axis_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_axis_orientation`](mjx_docx::Document::set_chart_axis_orientation).
    pub fn set_chart_axis_orientation(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        orientation: AxisOrientation,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_axis_orientation(drawing_id, index(axis_idx), orientation)?)
    }

    /// Sets or removes the title of axis `axis_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_axis_title`](mjx_docx::Document::set_chart_axis_title).
    pub fn set_chart_axis_title(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        text: Option<&str>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_axis_title(drawing_id, index(axis_idx), text)?)
    }

    /// Turns the gridlines of axis `axis_idx` on or off.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_axis_gridlines`](mjx_docx::Document::set_chart_axis_gridlines).
    pub fn set_chart_axis_gridlines(
        &mut self,
        drawing_id: u32,
        axis_idx: u32,
        major: bool,
        minor: bool,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_axis_gridlines(drawing_id, index(axis_idx), major, minor)?)
    }

    /// Sets or removes the chart's heading. `None` removes it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_title`](mjx_docx::Document::set_chart_title).
    pub fn set_chart_title(&mut self, drawing_id: u32, text: Option<&str>) -> Result<(), Error> {
        Ok(self.document.set_chart_title(drawing_id, text)?)
    }

    /// Places the chart's legend at `position`, or (for `None`) removes it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_legend`](mjx_docx::Document::set_chart_legend).
    pub fn set_chart_legend(
        &mut self,
        drawing_id: u32,
        position: Option<LegendPosition>,
    ) -> Result<(), Error> {
        Ok(self.document.set_chart_legend(drawing_id, position)?)
    }

    /// Sets the fill of series `series_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_series_fill`](mjx_docx::Document::set_chart_series_fill).
    pub fn set_chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_series_fill(drawing_id, index(series_idx), fill)?)
    }

    /// Sets the outline of series `series_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_series_line`](mjx_docx::Document::set_chart_series_line).
    pub fn set_chart_series_line(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_series_line(drawing_id, index(series_idx), line)?)
    }

    /// Applies `spec` at one tier of the chart's data labels.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_data_labels`](mjx_docx::Document::set_chart_data_labels).
    pub fn set_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_data_labels(drawing_id, scope, spec)?)
    }

    /// Suppresses the labels at one tier.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::suppress_chart_data_labels`](mjx_docx::Document::suppress_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .suppress_chart_data_labels(drawing_id, scope)?)
    }

    /// Removes the labels at one tier entirely, so that tier inherits the one above it again.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::remove_chart_data_labels`](mjx_docx::Document::remove_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<bool, Error> {
        Ok(self.document.remove_chart_data_labels(drawing_id, scope)?)
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_point_fill`](mjx_docx::Document::set_chart_point_fill).
    pub fn set_chart_point_fill(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_point_fill(drawing_id, index(series_idx), point_idx, fill)?)
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_point_line`](mjx_docx::Document::set_chart_point_line).
    pub fn set_chart_point_line(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_point_line(drawing_id, index(series_idx), point_idx, line)?)
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius, or (for `None`) puts it back.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_point_explosion`](mjx_docx::Document::set_chart_point_explosion).
    pub fn set_chart_point_explosion(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), Error> {
        Ok(self.document.set_chart_point_explosion(
            drawing_id,
            index(series_idx),
            point_idx,
            percent,
        )?)
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::remove_chart_point_format`](mjx_docx::Document::remove_chart_point_format).
    pub fn remove_chart_point_format(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<bool, Error> {
        Ok(self
            .document
            .remove_chart_point_format(drawing_id, index(series_idx), point_idx)?)
    }

    /// Fits a trendline through series `series_idx`, appending to any it already carries.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::add_chart_trendline`](mjx_docx::Document::add_chart_trendline).
    pub fn add_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .add_chart_trendline(drawing_id, index(series_idx), spec)?)
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, in place.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_trendline`](mjx_docx::Document::set_chart_trendline).
    pub fn set_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self.document.set_chart_trendline(
            drawing_id,
            index(series_idx),
            index(trendline_idx),
            spec,
        )?)
    }

    /// Removes every trendline from series `series_idx`, answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::remove_chart_trendlines`](mjx_docx::Document::remove_chart_trendlines).
    pub fn remove_chart_trendlines(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(
            self.document
                .remove_chart_trendlines(drawing_id, index(series_idx))?,
        ))
    }

    /// Gives series `series_idx` error bars, replacing an existing set along the same axis.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::set_chart_error_bars`](mjx_docx::Document::set_chart_error_bars).
    pub fn set_chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .set_chart_error_bars(drawing_id, index(series_idx), spec)?)
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::remove_chart_error_bars`](mjx_docx::Document::remove_chart_error_bars).
    pub fn remove_chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(
            self.document
                .remove_chart_error_bars(drawing_id, index(series_idx))?,
        ))
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` past the end of its data,
    /// answering how many went.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure.
    ///
    /// See [`Document::drop_chart_dangling_decoration`](mjx_docx::Document::drop_chart_dangling_decoration).
    pub fn drop_chart_dangling_decoration(
        &mut self,
        drawing_id: u32,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.document.drop_chart_dangling_decoration(
            drawing_id,
            index(series_idx),
        )?))
    }
}
