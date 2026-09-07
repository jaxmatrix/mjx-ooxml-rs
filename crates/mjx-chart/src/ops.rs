//! Every read and every edit a *host* surface performs on a chart, stated once over a
//! [`ChartSpace`] — the format-neutral half of `Presentation`'s and `Document`'s chart families.
//!
//! # Why this module exists (MJXOFF-103)
//!
//! A chart part is `c:chartSpace` in all three formats. What differs between a presentation and a
//! document is only **how the part is reached** — a `p:graphicFrame` on a slide, or a `w:drawing`
//! in a run — and what happens either side of the read: resolving the relationship, dirtying the
//! part, refreshing the embedded workbook. Everything *between* those two ends is identical, down
//! to which index is out of range.
//!
//! Before this module that middle was written once in `mjx-pptx`. Giving Word the same vocabulary
//! could not reuse it — `mjx-docx` and `mjx-pptx` are both rank 3.0, so an edge between them is
//! sideways and the layering rule forbids it — so the choice was one shared body here or two bodies
//! that must be kept in step by hand. This is the shared body. `mjx-pptx`'s methods and `mjx-docx`'s
//! now differ only in their first three lines (resolve the part) and their error type, which is what
//! makes "the same method names do the same thing on both surfaces" a property of the code rather
//! than of a review.
//!
//! # The error type, and why a host crate does not use it directly
//!
//! Every fallible operation here answers [`ChartAccessError`], which names only failures that are
//! about the *chart*: an index past the end, a series with nothing editable, a part with no
//! `c:chart`. A host crate maps it into its own error enum through an **exhaustive `match`** (A9's
//! rule, no wildcard), so a variant added here breaks both hosts' builds rather than silently
//! collapsing into a catch-all.

use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::Interner;

use crate::author::ChartDataError;
use crate::axis::{Axis, AxisOrientation, LegendPosition};
use crate::decoration::{
    DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarSpec, TrendlineSpec,
};
use crate::embedding::patch::ReferenceProblem;
use crate::plot::{ChartKind, Series, SeriesDecoration};
use crate::space::ChartSpace;
use crate::view::{
    ChartAxisData, ChartErrorBarData, ChartLabelScope, ChartLegendData, ChartPointFormatData,
    ChartSeriesData, ChartSeriesReferences, ChartTrendlineData,
};

/// What can go wrong reading or editing a chart, once its part has already been found and parsed.
///
/// Reaching the part at all is the host's business — "this shape frames no chart", "this drawing id
/// names nothing" — and stays in the host's own error type. Everything here is a statement about
/// the chart itself.
///
/// This enum is deliberately **not** `#[non_exhaustive]`. The attribute would force every host to
/// carry a wildcard arm, which is exactly the catch-all A9's rule exists to forbid: the point of
/// mapping through an exhaustive `match` is that a variant added here stops both hosts compiling
/// until someone decides what each surface should say about it.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChartAccessError {
    /// The addressed series is outside the chart, which draws `count` series across its plots.
    #[error("chart series {index} is outside a chart with {count} series")]
    SeriesOutOfRange {
        /// The series index asked for.
        index: usize,
        /// The number of series the chart draws.
        count: usize,
    },

    /// The addressed series has no cached numeric values (`c:val`/`c:yVal`) — or no string category
    /// cache (`c:cat`) — to rewrite; its data comes from a source this build does not edit.
    #[error("chart series {index} has no {kind} to edit")]
    SeriesNotEditable {
        /// The series index asked for.
        index: usize,
        /// What was being edited — `"values"` or `"categories"`.
        kind: &'static str,
    },

    /// A trendline index is past the last trendline of the series.
    #[error("trendline {index} is out of range: the series carries {count} trendline(s)")]
    TrendlineOutOfRange {
        /// The index that was asked for.
        index: usize,
        /// How many trendlines the series carries.
        count: usize,
    },

    /// A chart plot index is past the last plot of the chart's plot area.
    ///
    /// Plots are numbered as [`kinds`] numbers them, so a combo chart drawing a bar and a line has
    /// plots 0 and 1.
    #[error("plot {index} is out of range: the chart draws {count} plot(s)")]
    PlotOutOfRange {
        /// The index that was asked for.
        index: usize,
        /// How many plots the chart's plot area holds.
        count: usize,
    },

    /// The addressed axis is outside the chart, whose plot area declares `count` axes.
    #[error("chart axis {index} is outside a plot area with {count} axes")]
    AxisOutOfRange {
        /// The axis index asked for.
        index: usize,
        /// The number of axes the plot area declares.
        count: usize,
    },

    /// The chart part declares no `c:chart`, so it has no title, legend or plot area to edit. The
    /// schema requires one, so this means the part is malformed.
    #[error("chart part declares no c:chart element")]
    NoChartElement,

    /// An image fill was asked for on a chart series or point. An image fill names an image
    /// *relationship*, and a chart part relates to no images, so writing one would leave a dangling
    /// reference.
    #[error("a chart series cannot take an image fill: a chart part relates to no images")]
    FillNotSupported,

    /// The edit describes markup the schema does not admit where it was asked for — a trendline on
    /// a pie series, a point past the end of the data, leader lines on one point's label.
    #[error(transparent)]
    Data(#[from] ChartDataError),

    /// A `c:f` names cells this library will not write, so the chart's data cannot be put into the
    /// workbook the chart embeds (MJXOFF-208).
    ///
    /// A data edit **patches** the embedded workbook — it writes the new numbers into the cells the
    /// chart's own formulas name and leaves every other sheet, format and name the producer wrote
    /// exactly as it was. When the formula does not resolve to writable cells the edit is refused
    /// here rather than completed by regenerating the workbook, because regenerating it destroys
    /// content in a part the caller never named. [`ReferenceProblem`] says which shape of reference
    /// it was; `regenerate_chart_workbook` on the host type is the explicit opt-in for a caller who
    /// would rather have a fresh workbook than the producer's one.
    #[error(
        "the chart reference `{reference}` cannot be written into the embedded workbook: {problem}"
    )]
    EmbeddedWorkbookNotWritable {
        /// The `c:f` exactly as the chart wrote it, or the sheet name it qualified itself with when
        /// the workbook is what could not answer.
        reference: String,
        /// What about the reference stopped it.
        problem: ReferenceProblem,
    },
}

// =================================================================================================
// Locating one addressed thing
// =================================================================================================

/// A `u32` index carried by a public [`ChartLabelScope`] as the `usize` this crate's models address
/// with.
///
/// Widening `u32` to `usize` is lossless on every target this library builds for (32- and 64-bit,
/// plus `wasm32`); on a hypothetical 16-bit one it saturates at [`usize::MAX`], which is an index no
/// chart holds, so the caller gets an out-of-range error rather than the wrong plot. This is the
/// same conversion `mjx_ooxml`'s own facade-level `index` helper makes, for the same reason: one
/// width on every host at the public boundary, the model's own width inside.
fn as_index(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// The `n`-th series of a chart being read, or [`ChartAccessError::SeriesOutOfRange`].
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn series_at(space: &ChartSpace, series_idx: usize) -> Result<&Series, ChartAccessError> {
    let count = space.series_count();
    space
        .plot_area()
        .and_then(|area| area.all_series().nth(series_idx))
        .ok_or(ChartAccessError::SeriesOutOfRange {
            index: series_idx,
            count,
        })
}

/// The `n`-th series of a chart being edited, or [`ChartAccessError::SeriesOutOfRange`].
fn series_mut(space: &mut ChartSpace, series_idx: usize) -> Result<&mut Series, ChartAccessError> {
    let count = space.series_count();
    space
        .series_mut(series_idx)
        .ok_or(ChartAccessError::SeriesOutOfRange {
            index: series_idx,
            count,
        })
}

/// The `n`-th axis of a chart being edited, or [`ChartAccessError::AxisOutOfRange`].
fn axis_mut(space: &mut ChartSpace, axis_idx: usize) -> Result<&mut Axis, ChartAccessError> {
    let area = space
        .plot_area_mut()
        .ok_or(ChartAccessError::AxisOutOfRange {
            index: axis_idx,
            count: 0,
        })?;
    let count = area.axis_count();
    area.axis_mut(axis_idx)
        .ok_or(ChartAccessError::AxisOutOfRange {
            index: axis_idx,
            count,
        })
}

/// Runs `edit` against series `series_idx` bound to the kind of plot that holds it — the shared body
/// of every decoration write below.
///
/// `c:spPr` may be written by any of them, so the DrawingML prefix is declared first: a part that
/// never bound `a:` would otherwise gain unbound markup the moment one is set.
fn with_series_decoration<R>(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    edit: impl FnOnce(&mut SeriesDecoration<'_>, &mut Interner) -> Result<R, ChartAccessError>,
) -> Result<R, ChartAccessError> {
    space.ensure_drawingml_namespace(interner);
    let count = space.series_count();
    let mut decoration =
        space
            .series_decoration_mut(series_idx)
            .ok_or(ChartAccessError::SeriesOutOfRange {
                index: series_idx,
                count,
            })?;
    edit(&mut decoration, interner)
}

// =================================================================================================
// Reads
// =================================================================================================

/// Every series the chart draws, flattened across its plots — each with its name, its category
/// labels and its values (for a scatter series, its X labels and Y values).
#[must_use]
pub fn series(space: &ChartSpace) -> Vec<ChartSeriesData> {
    let Some(area) = space.plot_area() else {
        return Vec::new();
    };
    area.all_series()
        .map(|series| ChartSeriesData {
            name: series.name(),
            categories: series
                .categories()
                .map(crate::data::CategoryData::labels)
                .or_else(|| series.x_data().map(crate::data::CategoryData::labels))
                .unwrap_or_default(),
            values: series
                .values()
                .map(crate::data::NumericData::values)
                .or_else(|| series.y_data().map(crate::data::NumericData::values))
                .unwrap_or_default(),
        })
        .collect()
}

/// Where every series says its data lives — one entry per series, in the same order
/// [`series`] reports them (MJXOFF-111).
///
/// This is the companion of [`series`]: that answers what the **caches** hold, this answers what the
/// `c:f` beside each cache **names**. On a worksheet the two can disagree, because the cells are the
/// source and the cache is only what drew last; everywhere else the reference names an embedded
/// workbook the same call would rewrite. Reporting both, and naming which is which, is the honesty
/// this library already applies to a chart whose workbook disagrees with its caches.
#[must_use]
pub fn series_references(space: &ChartSpace) -> Vec<ChartSeriesReferences> {
    let Some(area) = space.plot_area() else {
        return Vec::new();
    };
    area.all_series()
        .map(|series| ChartSeriesReferences {
            name: series
                .name_source()
                .and_then(crate::data::SeriesText::reference)
                .and_then(|reference| reference.formula())
                .map(crate::data::Formula::text),
            categories: series
                .categories()
                .or_else(|| series.x_data())
                .and_then(category_formula),
            values: series
                .values()
                .or_else(|| series.y_data())
                .and_then(|data| data.reference())
                .and_then(|reference| reference.formula())
                .map(crate::data::Formula::text),
        })
        .collect()
}

/// The `c:f` of whichever of the three reference shapes a category source uses, or `None` for a
/// literal one.
///
/// `CT_AxDataSource` admits four children and three of them carry a reference — a string reference,
/// a numeric one, and a multi-level one. A category axis reading its labels out of a two-level range
/// is as much a live range as a flat one, so all three are read rather than only the common one.
fn category_formula(data: &crate::data::CategoryData) -> Option<String> {
    data.string_reference()
        .and_then(crate::data::StringReference::formula)
        .or_else(|| {
            data.number_reference()
                .and_then(crate::data::NumberReference::formula)
        })
        .or_else(|| {
            data.multi_level_reference()
                .and_then(crate::data::MultiLevelStringReference::formula)
        })
        .map(crate::data::Formula::text)
}

/// The kind of every plot the chart draws, in document order — one entry per plot element, so a
/// combo chart yields several.
#[must_use]
pub fn kinds(space: &ChartSpace) -> Vec<ChartKind> {
    space.chart_kinds()
}

/// The chart's axes, in document order.
#[must_use]
pub fn axes(space: &ChartSpace, interner: &Interner) -> Vec<ChartAxisData> {
    let Some(area) = space.plot_area() else {
        return Vec::new();
    };
    area.axes()
        .map(|(kind, axis)| ChartAxisData::read(kind, axis, interner))
        .collect()
}

/// The chart's heading (`c:title`), or `None` when it has none.
#[must_use]
pub fn title(space: &ChartSpace) -> Option<String> {
    space.chart().and_then(crate::space::Chart::title_text)
}

/// The chart's legend, or `None` when it has none.
#[must_use]
pub fn legend(space: &ChartSpace, interner: &Interner) -> Option<ChartLegendData> {
    space
        .chart()
        .and_then(crate::space::Chart::legend)
        .map(|legend| ChartLegendData {
            position: legend.position(interner),
            overlays_plot: legend.overlays_plot(interner),
        })
}

/// The built-in style id the chart names (`c:style@val`, 1 to 48) — the palette and effect set
/// Office draws an unstyled series with — or `None` when it names none.
#[must_use]
pub fn style_id(space: &ChartSpace, interner: &Interner) -> Option<u32> {
    space.style_id(interner)
}

/// The fill of series `series_idx` — what colour it is drawn in — or `None` when the series declares
/// none and takes its colour from the chart style.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn series_fill(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
) -> Result<Option<FillSpec>, ChartAccessError> {
    Ok(series_at(space, series_idx)?.fill(interner))
}

/// The data-label settings **in force** for one point of series `series_idx` — the point's `c:dLbl`
/// merged over the series' `c:dLbls` merged over the owning plot's.
///
/// Pass `point_idx = None` to stop at the series tier.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn data_labels(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
    point_idx: Option<u32>,
) -> Result<DataLabelSettings, ChartAccessError> {
    let count = space.series_count();
    space
        .resolved_data_labels(interner, series_idx, point_idx)
        .ok_or(ChartAccessError::SeriesOutOfRange {
            index: series_idx,
            count,
        })
}

/// The data-label settings one **tier** states in its own right — what that tier contributes to the
/// merge, with everything it leaves unset reported as `None`.
///
/// `None` means the tier carries no `c:dLbls`/`c:dLbl` at all, which is different from one that
/// carries an empty element.
///
/// # Errors
/// [`ChartAccessError::NoChartElement`] when the part declares no plot area,
/// [`ChartAccessError::PlotOutOfRange`] or [`ChartAccessError::SeriesOutOfRange`] for an index past
/// the end.
pub fn data_label_tier(
    space: &ChartSpace,
    interner: &Interner,
    scope: ChartLabelScope,
) -> Result<Option<DataLabelSettings>, ChartAccessError> {
    let area = space.plot_area().ok_or(ChartAccessError::NoChartElement)?;
    match scope {
        ChartLabelScope::Plot { plot_index } => {
            let plot_index = as_index(plot_index);
            let count = area.chart_kinds().len();
            if plot_index >= count {
                return Err(ChartAccessError::PlotOutOfRange {
                    index: plot_index,
                    count,
                });
            }
            Ok(area
                .plot_data_labels(plot_index)
                .map(|labels| labels.settings(interner)))
        }
        ChartLabelScope::Series { series_index } => Ok(series_at(space, as_index(series_index))?
            .data_labels()
            .map(|labels| labels.settings(interner))),
        ChartLabelScope::Point {
            series_index,
            point_index,
        } => Ok(series_at(space, as_index(series_index))?
            .data_labels()
            .and_then(|labels| labels.label_for_point(interner, point_index))
            .map(|label| label.settings(interner))),
    }
}

/// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
/// states none and shows what the settings say.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn point_label_text(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
    point_idx: u32,
) -> Result<Option<String>, ChartAccessError> {
    Ok(series_at(space, series_idx)?
        .data_labels()
        .and_then(|labels| labels.label_for_point(interner, point_idx))
        .and_then(crate::decoration::DataLabel::text))
}

/// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document order.
///
/// Each entry names the point it formats by `c:idx`, not by its position in this list — see
/// [`ChartPointFormatData::index`].
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn point_formats(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
) -> Result<Vec<ChartPointFormatData>, ChartAccessError> {
    Ok(series_at(space, series_idx)?
        .point_formats()
        .map(|format| ChartPointFormatData {
            index: format.index(interner),
            fill: format.fill(interner),
            line: format.line(interner),
            explosion: format.explosion(interner),
            inverts_if_negative: format.inverts_if_negative(interner),
        })
        .collect())
}

/// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn trendlines(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
) -> Result<Vec<ChartTrendlineData>, ChartAccessError> {
    Ok(series_at(space, series_idx)?
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
}

/// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line series,
/// up to two (x and y) for scatter, area and bubble.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn error_bars(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
) -> Result<Vec<ChartErrorBarData>, ChartAccessError> {
    Ok(series_at(space, series_idx)?
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
}

/// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no longer
/// has.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn dangling_decoration(
    space: &ChartSpace,
    interner: &Interner,
    series_idx: usize,
) -> Result<Vec<DanglingPointReference>, ChartAccessError> {
    Ok(series_at(space, series_idx)?.decoration_beyond_data(interner))
}

// =================================================================================================
// Writes
// =================================================================================================

/// Rewrites the values of series `series_idx` — whichever source the series names: a `c:numRef`'s
/// cache or a `c:numLit`. A non-finite value is skipped.
///
/// This does **not** refresh the chart's embedded workbook: that needs the package the chart part
/// lives in, which only the host has. Every host method that calls this refreshes afterwards.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`], or [`ChartAccessError::SeriesNotEditable`] when the
/// series has no numeric values to rewrite.
pub fn set_series_values(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    values: &[f64],
) -> Result<(), ChartAccessError> {
    if series_mut(space, series_idx)?.set_values(interner, values) {
        Ok(())
    } else {
        Err(ChartAccessError::SeriesNotEditable {
            index: series_idx,
            kind: "values",
        })
    }
}

/// Rewrites the category labels of series `series_idx`.
///
/// # Errors
/// As [`set_series_values`], with [`ChartAccessError::SeriesNotEditable`] when the series' category
/// source is numeric or multi-level and so has no string labels to rewrite.
pub fn set_series_categories(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    labels: &[&str],
) -> Result<(), ChartAccessError> {
    if series_mut(space, series_idx)?.set_categories(interner, labels) {
        Ok(())
    } else {
        Err(ChartAccessError::SeriesNotEditable {
            index: series_idx,
            kind: "categories",
        })
    }
}

/// Sets or clears the explicit bounds of axis `axis_idx`. `None` returns that end of the axis to
/// automatic scaling.
///
/// # Errors
/// [`ChartAccessError::AxisOutOfRange`] when `axis_idx` is past the last axis.
pub fn set_axis_scale(
    space: &mut ChartSpace,
    interner: &mut Interner,
    axis_idx: usize,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> Result<(), ChartAccessError> {
    let axis = axis_mut(space, axis_idx)?;
    let scaling = axis.scaling_mut(interner);
    scaling.set_minimum(interner, minimum);
    scaling.set_maximum(interner, maximum);
    Ok(())
}

/// Sets the direction of axis `axis_idx` — smallest value first, or reversed.
///
/// # Errors
/// As [`set_axis_scale`].
pub fn set_axis_orientation(
    space: &mut ChartSpace,
    interner: &mut Interner,
    axis_idx: usize,
    orientation: AxisOrientation,
) -> Result<(), ChartAccessError> {
    axis_mut(space, axis_idx)?
        .scaling_mut(interner)
        .set_orientation(interner, orientation);
    Ok(())
}

/// Sets or removes the title of axis `axis_idx`. `None` removes the title.
///
/// # Errors
/// As [`set_axis_scale`].
pub fn set_axis_title(
    space: &mut ChartSpace,
    interner: &mut Interner,
    axis_idx: usize,
    text: Option<&str>,
) -> Result<(), ChartAccessError> {
    space.ensure_drawingml_namespace(interner);
    axis_mut(space, axis_idx)?.set_title(interner, text);
    Ok(())
}

/// Turns the gridlines of axis `axis_idx` on or off.
///
/// # Errors
/// As [`set_axis_scale`].
pub fn set_axis_gridlines(
    space: &mut ChartSpace,
    interner: &mut Interner,
    axis_idx: usize,
    major: bool,
    minor: bool,
) -> Result<(), ChartAccessError> {
    let axis = axis_mut(space, axis_idx)?;
    axis.set_major_gridlines(interner, major);
    axis.set_minor_gridlines(interner, minor);
    Ok(())
}

/// Sets or removes the chart's heading. `None` removes it.
///
/// Setting a title also clears `c:autoTitleDeleted`, and removing one sets it — otherwise Office
/// either refuses to draw the title given to it or invents one of its own.
///
/// # Errors
/// [`ChartAccessError::NoChartElement`] when the part declares no `c:chart`.
pub fn set_title(
    space: &mut ChartSpace,
    interner: &mut Interner,
    text: Option<&str>,
) -> Result<(), ChartAccessError> {
    space.ensure_drawingml_namespace(interner);
    space
        .chart_mut()
        .ok_or(ChartAccessError::NoChartElement)?
        .set_title(interner, text);
    Ok(())
}

/// Places the chart's legend at `position`, adding one if the chart had none. `None` removes the
/// legend.
///
/// # Errors
/// As [`set_title`].
pub fn set_legend(
    space: &mut ChartSpace,
    interner: &mut Interner,
    position: Option<LegendPosition>,
) -> Result<(), ChartAccessError> {
    space
        .chart_mut()
        .ok_or(ChartAccessError::NoChartElement)?
        .set_legend(interner, position);
    Ok(())
}

/// Sets the fill of series `series_idx`, creating its `c:spPr` if it had none.
///
/// A [`FillSpec::Picture`] is **not** accepted: an image fill names an image relationship, and a
/// chart part relates to no images.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`], or [`ChartAccessError::FillNotSupported`] for an image
/// fill.
pub fn set_series_fill(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    fill: &FillSpec,
) -> Result<(), ChartAccessError> {
    if matches!(fill, FillSpec::Picture { .. }) {
        return Err(ChartAccessError::FillNotSupported);
    }
    space.ensure_drawingml_namespace(interner);
    series_mut(space, series_idx)?.set_fill(interner, fill);
    Ok(())
}

/// Sets the outline of series `series_idx` — the line a line or radar plot draws, or the border of a
/// bar or area.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn set_series_line(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    line: &LineSpec,
) -> Result<(), ChartAccessError> {
    space.ensure_drawingml_namespace(interner);
    series_mut(space, series_idx)?.set_line(interner, line);
    Ok(())
}

/// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had none
/// and leaving every setting `spec` does not state alone.
///
/// # Errors
/// [`ChartAccessError::NoChartElement`], [`ChartAccessError::PlotOutOfRange`],
/// [`ChartAccessError::SeriesOutOfRange`], or [`ChartAccessError::Data`] when the schema does not
/// admit the markup where it was asked for.
pub fn set_data_labels(
    space: &mut ChartSpace,
    interner: &mut Interner,
    scope: ChartLabelScope,
    spec: &DataLabelSpec,
) -> Result<(), ChartAccessError> {
    // A label may carry `c:spPr`/`c:txPr`, which are DrawingML; a part that never declared the
    // prefix would otherwise gain unbound markup the moment one is written.
    space.ensure_drawingml_namespace(interner);
    match scope {
        ChartLabelScope::Plot { plot_index } => {
            let plot_index = as_index(plot_index);
            let area = space
                .plot_area_mut()
                .ok_or(ChartAccessError::NoChartElement)?;
            let count = area.chart_kinds().len();
            if area.set_plot_data_labels(interner, plot_index, spec)? {
                Ok(())
            } else {
                Err(ChartAccessError::PlotOutOfRange {
                    index: plot_index,
                    count,
                })
            }
        }
        ChartLabelScope::Series { series_index } => with_series_decoration(
            space,
            interner,
            as_index(series_index),
            |decoration, interner| {
                decoration.set_data_labels(interner, spec)?;
                Ok(())
            },
        ),
        ChartLabelScope::Point {
            series_index,
            point_index,
        } => with_series_decoration(
            space,
            interner,
            as_index(series_index),
            |decoration, interner| {
                decoration.set_point_label(interner, point_index, spec)?;
                Ok(())
            },
        ),
    }
}

/// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is how
/// one series of a labelled plot, or one point of a labelled series, is silenced without disturbing
/// the rest.
///
/// # Errors
/// As [`set_data_labels`].
pub fn suppress_data_labels(
    space: &mut ChartSpace,
    interner: &mut Interner,
    scope: ChartLabelScope,
) -> Result<(), ChartAccessError> {
    match scope {
        ChartLabelScope::Plot { plot_index } => {
            let plot_index = as_index(plot_index);
            let area = space
                .plot_area_mut()
                .ok_or(ChartAccessError::NoChartElement)?;
            let count = area.chart_kinds().len();
            if area.suppress_plot_data_labels(interner, plot_index)? {
                Ok(())
            } else {
                Err(ChartAccessError::PlotOutOfRange {
                    index: plot_index,
                    count,
                })
            }
        }
        ChartLabelScope::Series { series_index } => with_series_decoration(
            space,
            interner,
            as_index(series_index),
            |decoration, interner| {
                decoration.suppress_data_labels(interner)?;
                Ok(())
            },
        ),
        ChartLabelScope::Point {
            series_index,
            point_index,
        } => with_series_decoration(
            space,
            interner,
            as_index(series_index),
            |decoration, interner| {
                decoration.suppress_point_label(interner, point_index)?;
                Ok(())
            },
        ),
    }
}

/// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
/// again. Answers whether an element was there.
///
/// This is the opposite of [`suppress_data_labels`]: suppressing says "draw nothing here", removing
/// says "say nothing here".
///
/// # Errors
/// As [`set_data_labels`].
pub fn remove_data_labels(
    space: &mut ChartSpace,
    interner: &mut Interner,
    scope: ChartLabelScope,
) -> Result<bool, ChartAccessError> {
    match scope {
        ChartLabelScope::Plot { plot_index } => {
            let plot_index = as_index(plot_index);
            let area = space
                .plot_area_mut()
                .ok_or(ChartAccessError::NoChartElement)?;
            let count = area.chart_kinds().len();
            if plot_index >= count {
                return Err(ChartAccessError::PlotOutOfRange {
                    index: plot_index,
                    count,
                });
            }
            Ok(area.remove_plot_data_labels(plot_index))
        }
        ChartLabelScope::Series { series_index } => {
            with_series_decoration(space, interner, as_index(series_index), |decoration, _| {
                Ok(decoration.remove_data_labels())
            })
        }
        ChartLabelScope::Point {
            series_index,
            point_index,
        } => with_series_decoration(
            space,
            interner,
            as_index(series_index),
            |decoration, interner| Ok(decoration.remove_point_label(interner, point_index)),
        ),
    }
}

/// Colours point `point_idx` of series `series_idx` differently from the rest of its series, creating
/// its `c:dPt` at the schema rank if it had none.
///
/// # Errors
/// As [`set_data_labels`], plus [`ChartAccessError::FillNotSupported`] for an image fill.
pub fn set_point_fill(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    point_idx: u32,
    fill: &FillSpec,
) -> Result<(), ChartAccessError> {
    if matches!(fill, FillSpec::Picture { .. }) {
        return Err(ChartAccessError::FillNotSupported);
    }
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        decoration.set_point_fill(interner, point_idx, fill)?;
        Ok(())
    })
}

/// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
///
/// # Errors
/// As [`set_point_fill`], minus the image-fill case.
pub fn set_point_line(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    point_idx: u32,
    line: &LineSpec,
) -> Result<(), ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        decoration.set_point_line(interner, point_idx, line)?;
        Ok(())
    })
}

/// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
/// `percent` of the radius (`c:explosion`), or (for `None`) puts it back.
///
/// # Errors
/// As [`set_point_fill`], minus the image-fill case.
pub fn set_point_explosion(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    point_idx: u32,
    percent: Option<u32>,
) -> Result<(), ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        decoration
            .point_format_mut(interner, point_idx)?
            .set_explosion(interner, percent);
        Ok(())
    })
}

/// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the rest
/// of its series. Answers whether any was there.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn remove_point_format(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    point_idx: u32,
) -> Result<bool, ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        Ok(decoration.remove_point_format(interner, point_idx))
    })
}

/// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
/// series may carry a linear fit and a moving average at once.
///
/// # Errors
/// As [`set_data_labels`]; the plot-type case is
/// [`ChartDataError::DecorationNotAllowed`] (pie, doughnut, pie-of-pie, radar and surface series
/// declare no `c:trendline`).
pub fn add_trendline(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    spec: &TrendlineSpec,
) -> Result<(), ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        decoration.add_trendline(interner, spec)?;
        Ok(())
    })
}

/// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the curve
/// keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting `spec`
/// leaves unset is cleared.
///
/// # Errors
/// As [`add_trendline`], plus [`ChartAccessError::TrendlineOutOfRange`] when the series carries
/// fewer trendlines.
pub fn set_trendline(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    trendline_idx: usize,
    spec: &TrendlineSpec,
) -> Result<(), ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        let count = decoration.series().trendlines().count();
        if decoration.set_trendline(interner, trendline_idx, spec)? {
            Ok(())
        } else {
            Err(ChartAccessError::TrendlineOutOfRange {
                index: trendline_idx,
                count,
            })
        }
    })
}

/// Removes every trendline from series `series_idx`, answering how many went.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn remove_trendlines(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
) -> Result<usize, ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, _| {
        Ok(decoration.remove_trendlines())
    })
}

/// Gives series `series_idx` error bars, replacing an existing set that runs along the same axis.
///
/// # Errors
/// As [`set_data_labels`]; the plot-type case is [`ChartDataError::DecorationNotAllowed`] (pie,
/// doughnut, pie-of-pie, radar and surface series declare no `c:errBars`).
pub fn set_error_bars(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
    spec: &ErrorBarSpec,
) -> Result<(), ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        decoration.set_error_bars(interner, spec)?;
        Ok(())
    })
}

/// Removes every set of error bars from series `series_idx`, answering how many went.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn remove_error_bars(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
) -> Result<usize, ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, _| {
        Ok(decoration.remove_error_bars())
    })
}

/// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of its
/// data, answering how many went.
///
/// # Errors
/// [`ChartAccessError::SeriesOutOfRange`] when `series_idx` is past the last series.
pub fn drop_dangling_decoration(
    space: &mut ChartSpace,
    interner: &mut Interner,
    series_idx: usize,
) -> Result<usize, ChartAccessError> {
    with_series_decoration(space, interner, series_idx, |decoration, interner| {
        Ok(decoration.drop_decoration_beyond_data(interner))
    })
}
