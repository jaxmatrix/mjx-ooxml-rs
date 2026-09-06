//! Chart decoration: data labels, per-point formatting, trendlines and error bars — the
//! parts of a chart that describe individual data rather than the plot as a whole.

use mjx_chart::chart_ops;
use mjx_chart::{
    ChartErrorBarData, ChartLabelScope, ChartPointFormatData, ChartTrendlineData,
    DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarSpec, TrendlineSpec,
};
use mjx_dml::{FillSpec, LineSpec};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::surface::Surface;

use super::Presentation;

impl Presentation {
    /// The fill of series `series_idx` of the chart the frame `shape_idx` on `surface` references —
    /// what colour it is drawn in — or `None` when the series declares none and takes its colour from
    /// the chart style. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartSeriesOutOfRange`] if `series_idx` is past the last series, or another
    /// [`PptxError`] if an index is out of range or the chart part is malformed.
    pub fn chart_series_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::series_fill(space, interner, series_idx)?)
        })
    }

    /// Sets the fill of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references, creating its `c:spPr` if it had none. Marks only the chart part dirty.
    ///
    /// A [`FillSpec::Picture`] is **not** accepted here: an image fill names an image relationship, and
    /// a chart part relates to no images — it is rejected with
    /// [`PptxError::ChartFillNotSupported`] rather than silently written as a dangling reference.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus
    /// [`PptxError::ChartFillNotSupported`] for an image fill.
    pub fn set_chart_series_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_series_fill(
                space, interner, series_idx, fill,
            )?)
        })
    }

    /// Sets the outline of series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references — the line a line or radar plot draws, or the border of a bar or area. Marks only
    /// the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn set_chart_series_line(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_series_line(
                space, interner, series_idx, line,
            )?)
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Chart decoration — data labels, per-point formatting, trendlines and error bars (MJX-116)
    // ---------------------------------------------------------------------------------------------

    /// The data-label settings **in force** for one point of series `series_idx` of the chart the
    /// frame `shape_idx` on `surface` references — the point's `c:dLbl` merged over the series'
    /// `c:dLbls` merged over the owning plot's.
    ///
    /// Pass `point_idx = None` to stop at the series tier. The merge is per setting: a series that
    /// only says "show the value" still takes its plot's label position. A field that is still
    /// `None` is one no tier states, which the application fills in from the chart style.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartSeriesOutOfRange`] if `series_idx` is past the last series, or another
    /// [`PptxError`] if an index is out of range or the chart part is malformed.
    pub fn chart_data_labels(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::data_labels(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to
    /// the merge, with everything it leaves unset reported as `None`.
    ///
    /// `None` means the tier carries no `c:dLbls`/`c:dLbl` at all, which is different from one that
    /// carries an empty element. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels), plus
    /// [`PptxError::ChartPlotOutOfRange`] for a [`ChartLabelScope::Plot`] past the last plot.
    pub fn chart_data_label_tier(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::data_label_tier(space, interner, scope)?)
        })
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
    /// states none and shows what the settings say. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn chart_point_label_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<Option<String>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::point_label_text(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    ///
    /// The three scopes are the three tiers: [`Plot`](ChartLabelScope::Plot) is the default every
    /// series takes, [`Series`](ChartLabelScope::Series) overrides it for one series, and
    /// [`Point`](ChartLabelScope::Point) overrides that for one point.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartSeriesOutOfRange`] / [`PptxError::ChartPlotOutOfRange`] for an index past
    /// the end, or [`PptxError::ChartData`] carrying
    /// [`ChartDataError::DecorationNotAllowed`](crate::ChartDataError::DecorationNotAllowed) (a surface plot has no `c:dLbls`),
    /// [`ChartDataError::DataPointOutOfRange`](crate::ChartDataError::DataPointOutOfRange) (the point does not exist) or
    /// [`ChartDataError::SettingNotAtThisTier`](crate::ChartDataError::SettingNotAtThisTier) (leader lines on one point's label).
    pub fn set_chart_data_labels(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_data_labels(space, interner, scope, spec)?)
        })
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is
    /// how one series of a labelled plot, or one point of a labelled series, is silenced without
    /// disturbing the rest. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        scope: ChartLabelScope,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::suppress_data_labels(space, interner, scope)?)
        })
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    ///
    /// This is the opposite of [`suppress_chart_data_labels`](Self::suppress_chart_data_labels):
    /// suppressing says "draw nothing here", removing says "say nothing here".
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        scope: ChartLabelScope,
    ) -> Result<bool, PptxError> {
        let mut removed = false;
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            removed = chart_ops::remove_data_labels(space, interner, scope)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document
    /// order. Reading does not dirty the part.
    ///
    /// Each entry names the point it formats by `c:idx`, not by its position in this list — see
    /// [`ChartPointFormatData::index`].
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn chart_point_formats(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<Vec<ChartPointFormatData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::point_formats(space, interner, series_idx)?)
        })
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    ///
    /// The point is addressed by index into the series, which is what `c:idx` means. An index at or
    /// past the series' point count is refused rather than written as markup that addresses nothing.
    ///
    /// A [`FillSpec::Picture`] is not accepted, for the same reason it is not on a series: an image
    /// fill names an image relationship, and a chart part relates to no images.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels), plus
    /// [`PptxError::ChartFillNotSupported`] for an image fill.
    pub fn set_chart_point_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_point_fill(
                space, interner, series_idx, point_idx, fill,
            )?)
        })
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    /// Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_point_fill`](Self::set_chart_point_fill), minus the image-fill case.
    pub fn set_chart_point_line(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_point_line(
                space, interner, series_idx, point_idx, line,
            )?)
        })
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the chart
    /// part dirty.
    ///
    /// # Errors
    /// As [`set_chart_point_fill`](Self::set_chart_point_fill), minus the image-fill case.
    pub fn set_chart_point_explosion(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_point_explosion(
                space, interner, series_idx, point_idx, percent,
            )?)
        })
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the
    /// rest of its series. Answers whether any was there. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn remove_chart_point_format(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<bool, PptxError> {
        let mut removed = false;
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            removed = chart_ops::remove_point_format(space, interner, series_idx, point_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn chart_trendlines(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<Vec<ChartTrendlineData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::trendlines(space, interner, series_idx)?)
        })
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
    /// series may carry a linear fit and a moving average at once. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels); the plot-type case is
    /// [`ChartDataError::DecorationNotAllowed`](crate::ChartDataError::DecorationNotAllowed) (pie, doughnut, pie-of-pie, radar and surface series
    /// declare no `c:trendline`), and an order or period outside its simple type's range is
    /// [`ChartDataError::TrendlineOrderOutOfRange`](crate::ChartDataError::TrendlineOrderOutOfRange) / [`ChartDataError::TrendlinePeriodOutOfRange`](crate::ChartDataError::TrendlinePeriodOutOfRange).
    pub fn add_chart_trendline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::add_trendline(space, interner, series_idx, spec)?)
        })
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the
    /// curve keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting
    /// `spec` leaves unset is cleared. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`add_chart_trendline`](Self::add_chart_trendline), plus
    /// [`PptxError::ChartTrendlineOutOfRange`] when the series carries fewer trendlines.
    pub fn set_chart_trendline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        trendline_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_trendline(
                space,
                interner,
                series_idx,
                trendline_idx,
                spec,
            )?)
        })
    }

    /// Removes every trendline from series `series_idx`, answering how many went. Marks only the
    /// chart part dirty.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn remove_chart_trendlines(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<usize, PptxError> {
        let mut removed = 0;
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            removed = chart_ops::remove_trendlines(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line
    /// series, up to two (x and y) for scatter, area and bubble. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn chart_error_bars(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<Vec<ChartErrorBarData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::error_bars(space, interner, series_idx)?)
        })
    }

    /// Gives series `series_idx` error bars, replacing an existing set that runs along the same
    /// axis. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels); the plot-type case is
    /// [`ChartDataError::DecorationNotAllowed`](crate::ChartDataError::DecorationNotAllowed) (pie, doughnut, pie-of-pie, radar and surface series
    /// declare no `c:errBars`), and custom bars with neither `c:plus` nor `c:minus` are
    /// [`ChartDataError::CustomErrorBarsNeedValues`](crate::ChartDataError::CustomErrorBarsNeedValues).
    pub fn set_chart_error_bars(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        spec: &ErrorBarSpec,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_error_bars(
                space, interner, series_idx, spec,
            )?)
        })
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went. Marks
    /// only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn remove_chart_error_bars(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<usize, PptxError> {
        let mut removed = 0;
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            removed = chart_ops::remove_error_bars(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading does not dirty the part.
    ///
    /// A `c:dPt` is anchored by index into the series, so an edit that shortens the series can leave
    /// one addressing past the end. This library **never renumbers** such an element — moving one
    /// point's colour silently onto another would be worse than leaving it dangling — and never
    /// drops it on the caller's behalf. This reports them;
    /// [`drop_chart_dangling_decoration`](Self::drop_chart_dangling_decoration) removes them.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn chart_dangling_decoration(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<Vec<DanglingPointReference>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::dangling_decoration(space, interner, series_idx)?)
        })
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of
    /// its data, answering how many went. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_data_labels`](Self::chart_data_labels).
    pub fn drop_chart_dangling_decoration(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
    ) -> Result<usize, PptxError> {
        let mut removed = 0;
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            removed = chart_ops::drop_dangling_decoration(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }
}
