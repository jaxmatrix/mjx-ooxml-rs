//! Chart decoration: per-series and per-point fills and lines, data labels, trendlines and error
//! bars.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{
    ChartErrorBarData, ChartLabelScope, ChartPointFormatData, ChartTrendlineData,
    DanglingPointReference, DataLabelSettings, DataLabelSpec, Deck, Error, ErrorBarSpec, FillSpec,
    LineSpec, ShapePath, Surface, TrendlineSpec,
};

impl Deck {
    /// The fill of series `series_idx` of the chart the frame `shape_idx` on `surface` references —
    /// what colour it is drawn in — or `None` when the series declares none and takes its colour from
    /// the chart style. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_series_fill`](mjx_pptx::Presentation::chart_series_fill).
    pub fn chart_series_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self.presentation.chart_series_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?)
    }

    /// Sets the fill of series `series_idx` of the chart the frame `shape_idx` on `surface` references,
    /// creating its `c:spPr` if it had none. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_series_fill`](mjx_pptx::Presentation::set_chart_series_fill).
    pub fn set_chart_series_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_series_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            fill,
        )?)
    }

    /// Sets the outline of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references — the line a line or radar plot draws, or the border of a bar or area. Marks only the
    /// chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_series_line`](mjx_pptx::Presentation::set_chart_series_line).
    pub fn set_chart_series_line(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_series_line(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            line,
        )?)
    }

    /// The data-label settings **in force** for one point of series `series_idx` of the chart the frame
    /// `shape_idx` on `surface` references — the point's `c:dLbl` merged over the series' `c:dLbls`
    /// merged over the owning plot's.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_data_labels`](mjx_pptx::Presentation::chart_data_labels).
    pub fn chart_data_labels(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, Error> {
        Ok(self.presentation.chart_data_labels(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
        )?)
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to the
    /// merge, with everything it leaves unset reported as `None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_data_label_tier`](mjx_pptx::Presentation::chart_data_label_tier).
    pub fn chart_data_label_tier(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, Error> {
        Ok(self.presentation.chart_data_label_tier(
            surface.to_model(),
            shape_idx.to_model(),
            scope,
        )?)
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
    /// states none and shows what the settings say. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_point_label_text`](mjx_pptx::Presentation::chart_point_label_text).
    pub fn chart_point_label_text(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<Option<String>, Error> {
        Ok(self.presentation.chart_point_label_text(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
        )?)
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_data_labels`](mjx_pptx::Presentation::set_chart_data_labels).
    pub fn set_chart_data_labels(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_data_labels(
            surface.to_model(),
            shape_idx.to_model(),
            scope,
            spec,
        )?)
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is how
    /// one series of a labelled plot, or one point of a labelled series, is silenced without disturbing
    /// the rest. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::suppress_chart_data_labels`](mjx_pptx::Presentation::suppress_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        scope: ChartLabelScope,
    ) -> Result<(), Error> {
        Ok(self.presentation.suppress_chart_data_labels(
            surface.to_model(),
            shape_idx.to_model(),
            scope,
        )?)
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_chart_data_labels`](mjx_pptx::Presentation::remove_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        scope: ChartLabelScope,
    ) -> Result<bool, Error> {
        Ok(self.presentation.remove_chart_data_labels(
            surface.to_model(),
            shape_idx.to_model(),
            scope,
        )?)
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document order.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_point_formats`](mjx_pptx::Presentation::chart_point_formats).
    pub fn chart_point_formats(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<Vec<ChartPointFormatData>, Error> {
        Ok(self.presentation.chart_point_formats(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?)
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_point_fill`](mjx_pptx::Presentation::set_chart_point_fill).
    pub fn set_chart_point_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_point_fill(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
            fill,
        )?)
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series. Marks
    /// only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_point_line`](mjx_pptx::Presentation::set_chart_point_line).
    pub fn set_chart_point_line(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_point_line(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
            line,
        )?)
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the chart part
    /// dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_point_explosion`](mjx_pptx::Presentation::set_chart_point_explosion).
    pub fn set_chart_point_explosion(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_point_explosion(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
            percent,
        )?)
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the rest
    /// of its series. Answers whether any was there. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_chart_point_format`](mjx_pptx::Presentation::remove_chart_point_format).
    pub fn remove_chart_point_format(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        point_idx: u32,
    ) -> Result<bool, Error> {
        Ok(self.presentation.remove_chart_point_format(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            point_idx,
        )?)
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order. Reading
    /// does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_trendlines`](mjx_pptx::Presentation::chart_trendlines).
    pub fn chart_trendlines(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<Vec<ChartTrendlineData>, Error> {
        Ok(self.presentation.chart_trendlines(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?)
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
    /// series may carry a linear fit and a moving average at once. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_chart_trendline`](mjx_pptx::Presentation::add_chart_trendline).
    pub fn add_chart_trendline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.add_chart_trendline(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            spec,
        )?)
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the curve
    /// keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting `spec`
    /// leaves unset is cleared. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_trendline`](mjx_pptx::Presentation::set_chart_trendline).
    pub fn set_chart_trendline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        trendline_idx: u32,
        spec: &TrendlineSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_trendline(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            index(trendline_idx),
            spec,
        )?)
    }

    /// Removes every trendline from series `series_idx`, answering how many went. Marks only the chart
    /// part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_chart_trendlines`](mjx_pptx::Presentation::remove_chart_trendlines).
    pub fn remove_chart_trendlines(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.remove_chart_trendlines(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?))
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line
    /// series, up to two (x and y) for scatter, area and bubble. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_error_bars`](mjx_pptx::Presentation::chart_error_bars).
    pub fn chart_error_bars(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<Vec<ChartErrorBarData>, Error> {
        Ok(self.presentation.chart_error_bars(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?)
    }

    /// Gives series `series_idx` error bars, replacing an existing set that runs along the same axis.
    /// Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_chart_error_bars`](mjx_pptx::Presentation::set_chart_error_bars).
    pub fn set_chart_error_bars(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
        spec: &ErrorBarSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_chart_error_bars(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
            spec,
        )?)
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went. Marks only
    /// the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_chart_error_bars`](mjx_pptx::Presentation::remove_chart_error_bars).
    pub fn remove_chart_error_bars(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.remove_chart_error_bars(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?))
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::chart_dangling_decoration`](mjx_pptx::Presentation::chart_dangling_decoration).
    pub fn chart_dangling_decoration(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<Vec<DanglingPointReference>, Error> {
        Ok(self.presentation.chart_dangling_decoration(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?)
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of its
    /// data, answering how many went. Marks only the chart part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::drop_chart_dangling_decoration`](mjx_pptx::Presentation::drop_chart_dangling_decoration).
    pub fn drop_chart_dangling_decoration(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        series_idx: u32,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.drop_chart_dangling_decoration(
            surface.to_model(),
            shape_idx.to_model(),
            index(series_idx),
        )?))
    }
}
