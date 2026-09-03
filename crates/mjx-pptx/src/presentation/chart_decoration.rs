//! Chart decoration: data labels, per-point formatting, trendlines and error bars — the
//! parts of a chart that describe individual data rather than the plot as a whole.

use mjx_chart::{
    DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarDirection, ErrorBarSpec,
    ErrorBarType, ErrorValueType, TrendlineKind, TrendlineSpec,
};
use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::Interner;

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::surface::Surface;

use super::charts::chart_series_at;
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
            let count = space.series_count();
            let series = space
                .plot_area()
                .and_then(|area| area.all_series().nth(series_idx))
                .ok_or(PptxError::ChartSeriesOutOfRange {
                    index: series_idx,
                    count,
                })?;
            Ok(series.fill(interner))
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
        if matches!(fill, FillSpec::Picture { .. }) {
            return Err(PptxError::ChartFillNotSupported);
        }
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            space.ensure_drawingml_namespace(interner);
            let count = space.series_count();
            let series = space
                .series_mut(series_idx)
                .ok_or(PptxError::ChartSeriesOutOfRange {
                    index: series_idx,
                    count,
                })?;
            series.set_fill(interner, fill);
            Ok(())
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
            space.ensure_drawingml_namespace(interner);
            let count = space.series_count();
            let series = space
                .series_mut(series_idx)
                .ok_or(PptxError::ChartSeriesOutOfRange {
                    index: series_idx,
                    count,
                })?;
            series.set_line(interner, line);
            Ok(())
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
            let count = space.series_count();
            space
                .resolved_data_labels(interner, series_idx, point_idx)
                .ok_or(PptxError::ChartSeriesOutOfRange {
                    index: series_idx,
                    count,
                })
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
            let area = space.plot_area().ok_or(PptxError::ChartHasNoChartElement)?;
            match scope {
                ChartLabelScope::Plot { plot_idx } => {
                    let count = area.chart_kinds().len();
                    if plot_idx >= count {
                        return Err(PptxError::ChartPlotOutOfRange {
                            index: plot_idx,
                            count,
                        });
                    }
                    Ok(area
                        .plot_data_labels(plot_idx)
                        .map(|labels| labels.settings(interner)))
                }
                ChartLabelScope::Series { series_idx } => {
                    let series = chart_series_at(space, series_idx)?;
                    Ok(series.data_labels().map(|labels| labels.settings(interner)))
                }
                ChartLabelScope::Point {
                    series_idx,
                    point_idx,
                } => {
                    let series = chart_series_at(space, series_idx)?;
                    Ok(series
                        .data_labels()
                        .and_then(|labels| labels.label_for_point(interner, point_idx))
                        .map(|label| label.settings(interner)))
                }
            }
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
            let series = chart_series_at(space, series_idx)?;
            Ok(series
                .data_labels()
                .and_then(|labels| labels.label_for_point(interner, point_idx))
                .and_then(mjx_chart::DataLabel::text))
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
            // A label may carry `c:spPr`/`c:txPr`, which are DrawingML; a part that never declared
            // the prefix would otherwise gain unbound markup the moment one is written.
            space.ensure_drawingml_namespace(interner);
            match scope {
                ChartLabelScope::Plot { plot_idx } => {
                    let area = space
                        .plot_area_mut()
                        .ok_or(PptxError::ChartHasNoChartElement)?;
                    let count = area.chart_kinds().len();
                    if !area.set_plot_data_labels(interner, plot_idx, spec)? {
                        return Err(PptxError::ChartPlotOutOfRange {
                            index: plot_idx,
                            count,
                        });
                    }
                    Ok(())
                }
                ChartLabelScope::Series { series_idx } => {
                    let count = space.series_count();
                    let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                        PptxError::ChartSeriesOutOfRange {
                            index: series_idx,
                            count,
                        },
                    )?;
                    decoration.set_data_labels(interner, spec)?;
                    Ok(())
                }
                ChartLabelScope::Point {
                    series_idx,
                    point_idx,
                } => {
                    let count = space.series_count();
                    let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                        PptxError::ChartSeriesOutOfRange {
                            index: series_idx,
                            count,
                        },
                    )?;
                    decoration.set_point_label(interner, point_idx, spec)?;
                    Ok(())
                }
            }
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
        self.edit_chart(surface.into(), shape_idx, |space, interner| match scope {
            ChartLabelScope::Plot { plot_idx } => {
                let area = space
                    .plot_area_mut()
                    .ok_or(PptxError::ChartHasNoChartElement)?;
                let count = area.chart_kinds().len();
                if !area.suppress_plot_data_labels(interner, plot_idx)? {
                    return Err(PptxError::ChartPlotOutOfRange {
                        index: plot_idx,
                        count,
                    });
                }
                Ok(())
            }
            ChartLabelScope::Series { series_idx } => {
                let count = space.series_count();
                let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                    PptxError::ChartSeriesOutOfRange {
                        index: series_idx,
                        count,
                    },
                )?;
                decoration.suppress_data_labels(interner)?;
                Ok(())
            }
            ChartLabelScope::Point {
                series_idx,
                point_idx,
            } => {
                let count = space.series_count();
                let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                    PptxError::ChartSeriesOutOfRange {
                        index: series_idx,
                        count,
                    },
                )?;
                decoration.suppress_point_label(interner, point_idx)?;
                Ok(())
            }
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
            removed = match scope {
                ChartLabelScope::Plot { plot_idx } => {
                    let area = space
                        .plot_area_mut()
                        .ok_or(PptxError::ChartHasNoChartElement)?;
                    let count = area.chart_kinds().len();
                    if plot_idx >= count {
                        return Err(PptxError::ChartPlotOutOfRange {
                            index: plot_idx,
                            count,
                        });
                    }
                    area.remove_plot_data_labels(plot_idx)
                }
                ChartLabelScope::Series { series_idx } => {
                    let count = space.series_count();
                    let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                        PptxError::ChartSeriesOutOfRange {
                            index: series_idx,
                            count,
                        },
                    )?;
                    decoration.remove_data_labels()
                }
                ChartLabelScope::Point {
                    series_idx,
                    point_idx,
                } => {
                    let count = space.series_count();
                    let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                        PptxError::ChartSeriesOutOfRange {
                            index: series_idx,
                            count,
                        },
                    )?;
                    decoration.remove_point_label(interner, point_idx)
                }
            };
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
            let series = chart_series_at(space, series_idx)?;
            Ok(series
                .point_formats()
                .map(|format| ChartPointFormatData {
                    index: format.index(interner),
                    fill: format.fill(interner),
                    line: format.line(interner),
                    explosion: format.explosion(interner),
                    inverts_if_negative: format.inverts_if_negative(interner),
                })
                .collect())
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
        if matches!(fill, FillSpec::Picture { .. }) {
            return Err(PptxError::ChartFillNotSupported);
        }
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            decoration.set_point_fill(i, point_idx, fill)?;
            Ok(())
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
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            decoration.set_point_line(i, point_idx, line)?;
            Ok(())
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
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            decoration
                .point_format_mut(i, point_idx)?
                .set_explosion(i, percent);
            Ok(())
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
        self.edit_chart_series_decoration(
            surface.into(),
            shape_idx,
            series_idx,
            |decoration, i| {
                removed = decoration.remove_point_format(i, point_idx);
                Ok(())
            },
        )?;
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
            let series = chart_series_at(space, series_idx)?;
            Ok(series
                .trendlines()
                .map(|trendline| ChartTrendlineData {
                    kind: trendline.kind(interner),
                    name: trendline.name(interner),
                    polynomial_order: trendline.order(interner),
                    moving_average_period: trendline.period(interner),
                    forward_periods: trendline.forward_periods(interner),
                    backward_periods: trendline.backward_periods(interner),
                    intercept: trendline.intercept(interner),
                    displays_equation: trendline.displays_equation(interner),
                    displays_r_squared: trendline.displays_r_squared(interner),
                })
                .collect())
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
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            decoration.add_trendline(i, spec)?;
            Ok(())
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
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            let count = decoration.series().trendlines().count();
            if !decoration.set_trendline(i, trendline_idx, spec)? {
                return Err(PptxError::ChartTrendlineOutOfRange {
                    index: trendline_idx,
                    count,
                });
            }
            Ok(())
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
        self.edit_chart_series_decoration(
            surface.into(),
            shape_idx,
            series_idx,
            |decoration, _| {
                removed = decoration.remove_trendlines();
                Ok(())
            },
        )?;
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
            let series = chart_series_at(space, series_idx)?;
            Ok(series
                .error_bars()
                .map(|bars| ChartErrorBarData {
                    direction: bars.direction(interner),
                    bar_type: bars.bar_type(interner),
                    value_type: bars.value_type(interner),
                    no_end_cap: bars.no_end_cap(interner),
                    value: bars.value(interner),
                    plus_values: bars.plus_values(),
                    minus_values: bars.minus_values(),
                })
                .collect())
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
        self.edit_chart_series_decoration(surface.into(), shape_idx, series_idx, |decoration, i| {
            decoration.set_error_bars(i, spec)?;
            Ok(())
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
        self.edit_chart_series_decoration(
            surface.into(),
            shape_idx,
            series_idx,
            |decoration, _| {
                removed = decoration.remove_error_bars();
                Ok(())
            },
        )?;
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
            Ok(chart_series_at(space, series_idx)?.decoration_beyond_data(interner))
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
        self.edit_chart_series_decoration(
            surface.into(),
            shape_idx,
            series_idx,
            |decoration, i| {
                removed = decoration.drop_decoration_beyond_data(i);
                Ok(())
            },
        )?;
        Ok(removed)
    }

    /// Runs `edit` against series `series_idx` of the chart the frame `shape_idx` on `surface`
    /// references, bound to the kind of plot that holds it — the shared body of every decoration
    /// write above.
    fn edit_chart_series_decoration(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        edit: impl FnOnce(&mut mjx_chart::SeriesDecoration<'_>, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface, shape_idx, |space, interner| {
            // Decoration may carry `c:spPr`, which is DrawingML.
            space.ensure_drawingml_namespace(interner);
            let count = space.series_count();
            let mut decoration = space.series_decoration_mut(series_idx).ok_or(
                PptxError::ChartSeriesOutOfRange {
                    index: series_idx,
                    count,
                },
            )?;
            edit(&mut decoration, interner)
        })
    }
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
