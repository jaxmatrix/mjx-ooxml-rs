//! The negotiation: how much of the frame the title, the legend and the tick labels take, and what
//! is left for the data.
//!
//! # It is a fixed point, and it is bounded at two passes
//!
//! The plot area's height decides how many major ticks a value axis carries, which decides how wide
//! the widest tick label is, which decides how much of the width the plot area gets. That is a loop,
//! and an unbounded one would be a chart whose layout time depended on its numbers.
//!
//! So it is run **twice**, exactly: once against the frame with a provisional scale, and once with
//! the reservations the first pass measured. The second pass may choose a different tick count from
//! the first; it does not get a third chance, and the labels it measures are the ones it draws.
//!
//! **This is why a chart cannot break MJXOFF-175's two-assembly bound.** A chart is laid out inside a
//! frame it is *given* and never changes that frame's size — there is no path here that returns a
//! height — so a chart in a Word paragraph is one `U+FFFC` of fixed width and height, exactly as
//! MJXOFF-177's `TextRun::advance` describes, and a chart in a float resolves against the same body
//! height every other float does. The fixed point is inside the frame.
//!
//! # What takes room, in the order it is taken
//!
//! 1. A margin around the whole frame.
//! 2. The chart title, along the top.
//! 3. The legend, on its own side, unless it overlays the plot.
//! 4. The axis titles, on their axes' sides.
//! 5. The tick labels, on their axes' sides.
//!
//! The order matters: a legend on the right takes its width from the full height, and the value
//! axis' labels then take theirs from what is left. Reversing the two would put the legend inside the
//! axis' gutter.

use mjx_chart::{AxisKind, AxisPosition, BarDirection, ChartKind, LegendPosition, TickMark};
use mjx_layout::{LayoutPoint, LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::{Angle, Emu};

use crate::geometry::{
    AxisGeometry, ChartGeometry, ChartPaint, Gridline, LegendEntry, LegendGeometry, TextPlacement,
    TickGeometry,
};
use crate::label::{format_value, point_label};
use crate::model::{AxisModel, ChartModel, Grouping, PlotModel};
use crate::palette::ChartPalette;
use crate::plot::{
    cross_value_bounds, error_bars, lay_out_plot, trendlines, value_bounds, CrossScale,
    PlotContext, PlotFrame,
};
use crate::scale::{Baseline, Scale, StatedScale};
use crate::text::{ChartTextRole, TextMetrics};

/// The margin between a chart's frame and everything it draws.
///
/// `GUESS:` five points on every side. `c:plotArea > c:layout` states an explicit plot rectangle when
/// a reader has dragged one, and this engine does not read it (see the crate docs' limits); this is
/// what a chart with no stated layout is inset by.
const FRAME_MARGIN: Emu = Emu::from_emu(63_500);

/// The gap between one piece of chart furniture and the next.
const GAP: Emu = Emu::from_emu(38_100);

/// How long a tick mark is.
const TICK_LENGTH: Emu = Emu::from_emu(50_800);

/// How far apart major ticks should ideally be, before the nice-number step rounds it.
///
/// `EngineDerived:` 0.45 inch. Office scales its tick count with the plot's size — a tall chart gets
/// more ticks than a short one, which is why a chart resized in PowerPoint re-labels itself — and no
/// figure for it is published. This is the spacing at which a value axis reads without crowding.
const IDEAL_TICK_SPACING: Emu = Emu::from_emu(411_480);

/// The width of a legend's colour swatch.
const SWATCH: Emu = Emu::from_emu(114_300);

/// Lays a chart out inside `frame`.
///
/// `palette` supplies the six accents a series with no stated fill takes — **the host document's
/// own**, never one invented here. `metrics` measures the text; see [`crate::text::TextMetrics`].
#[must_use]
pub fn lay_out(
    model: &ChartModel,
    frame: LayoutRect,
    palette: &ChartPalette,
    metrics: &mut impl TextMetrics,
) -> ChartGeometry {
    let inner = deflate(frame, FRAME_MARGIN);

    // Pass one: reserve against the whole inner rectangle, so the provisional scale is derived from
    // an area at least as large as the real one. Pass two re-derives it from what pass one left.
    let first = reserve(model, inner, palette, metrics, None);
    let second = reserve(model, inner, palette, metrics, Some(&first));
    build(model, frame, palette, metrics, &second)
}

/// What one pass of the negotiation worked out.
struct Reservation {
    /// What is left for the data.
    plot: LayoutRect,
    /// The title, placed.
    title: Option<TextPlacement>,
    /// The legend, placed, with its entries.
    legend: Option<LegendGeometry>,
    /// The value scale, if the chart has a value axis.
    value: Option<Scale>,
    /// A scatter or bubble plot's x scale.
    cross: Option<Scale>,
}

/// Runs one pass of the negotiation. `previous` is the pass before, or `None` for the first.
fn reserve(
    model: &ChartModel,
    inner: LayoutRect,
    palette: &ChartPalette,
    metrics: &mut impl TextMetrics,
    previous: Option<&Reservation>,
) -> Reservation {
    let mut remaining = inner;

    let title = model.title.as_ref().map(|text| {
        let size = metrics.measure(ChartTextRole::ChartTitle.text(text));
        centred_top(remaining, size, text, ChartTextRole::ChartTitle)
    });
    if let Some(title) = &title {
        remaining.top = (title.rect.bottom + GAP).minimum(remaining.bottom);
    }

    let legend = model
        .legend
        .as_ref()
        .map(|legend| lay_out_legend(model, legend, remaining, palette, metrics));
    if let (Some(legend), Some(model_legend)) = (&legend, &model.legend) {
        if !model_legend.overlays_plot {
            match model_legend.position {
                LegendPosition::Bottom => remaining.bottom = legend.rect.top - GAP,
                LegendPosition::Top => remaining.top = legend.rect.bottom + GAP,
                LegendPosition::Left => remaining.left = legend.rect.right + GAP,
                LegendPosition::Right | LegendPosition::TopRight => {
                    remaining.right = legend.rect.left - GAP;
                }
            }
        }
    }
    remaining = normalise(remaining);

    if !has_axes(model) {
        return Reservation {
            plot: remaining,
            title,
            legend,
            value: None,
            cross: None,
        };
    }

    let swapped = is_swapped(model);
    // The extent the value axis runs along, which is what decides how many ticks it carries. On the
    // first pass this is the whole of what is left; on the second it is what the first pass reserved.
    let value_extent = match previous {
        Some(previous) if swapped => previous.plot.right - previous.plot.left,
        Some(previous) => previous.plot.bottom - previous.plot.top,
        None if swapped => remaining.right - remaining.left,
        None => remaining.bottom - remaining.top,
    };
    let value = value_scale(model, value_extent);
    let cross = cross_scale(model);

    // The axis titles.
    let (value_position, category_position) = axis_sides(model);
    if let Some(title) = model
        .axis_for_kind(AxisKind::Value)
        .and_then(|axis| axis.title.as_ref())
    {
        let size = metrics.measure(ChartTextRole::AxisTitle.text(title));
        remaining = take(remaining, value_position, along(value_position, size) + GAP);
    }
    if let Some(title) = category_axis(model).and_then(|axis| axis.title.as_ref()) {
        let size = metrics.measure(ChartTextRole::AxisTitle.text(title));
        remaining = take(
            remaining,
            category_position,
            along(category_position, size) + GAP,
        );
    }

    // The tick labels.
    if let Some(scale) = &value {
        if !suppressed(model, AxisKind::Value) {
            let widest = widest_value_label(scale, model, metrics);
            remaining = take(
                remaining,
                value_position,
                along(value_position, widest) + GAP,
            );
        }
    }
    if !suppressed_category(model) {
        let widest = widest_category_label(model, metrics, cross.as_ref());
        remaining = take(
            remaining,
            category_position,
            along(category_position, widest) + GAP,
        );
    }

    Reservation {
        plot: normalise(remaining),
        title,
        legend,
        value,
        cross,
    }
}

/// Whether the chart has axes at all. Pie, doughnut and pie-of-pie do not.
fn has_axes(model: &ChartModel) -> bool {
    model.plots.iter().any(|plot| {
        !matches!(
            plot.kind,
            ChartKind::Pie | ChartKind::Pie3D | ChartKind::Doughnut | ChartKind::OfPie
        )
    })
}

/// Whether the value axis runs across rather than up — true for a bar plot and nothing else.
fn is_swapped(model: &ChartModel) -> bool {
    model
        .plots
        .iter()
        .any(|plot| matches!(plot.kind, ChartKind::Bar | ChartKind::Bar3D))
        && model
            .plots
            .iter()
            .all(|plot| plot.direction == BarDirection::Bar || !is_bar(plot))
}

/// Whether a plot is one of the two bar families.
fn is_bar(plot: &PlotModel) -> bool {
    matches!(plot.kind, ChartKind::Bar | ChartKind::Bar3D)
}

/// Which side each axis sits on: the value axis, then the category one.
fn axis_sides(model: &ChartModel) -> (AxisPosition, AxisPosition) {
    let stated_value = model
        .axis_for_kind(AxisKind::Value)
        .and_then(|axis| axis.position);
    let stated_category = category_axis(model).and_then(|axis| axis.position);
    if is_swapped(model) {
        (
            stated_value.unwrap_or(AxisPosition::Bottom),
            stated_category.unwrap_or(AxisPosition::Left),
        )
    } else {
        (
            stated_value.unwrap_or(AxisPosition::Left),
            stated_category.unwrap_or(AxisPosition::Bottom),
        )
    }
}

/// The chart's category axis, which is a `c:catAx` or a `c:dateAx`, or for a scatter plot the second
/// `c:valAx`.
fn category_axis(model: &ChartModel) -> Option<&AxisModel> {
    model
        .axes
        .iter()
        .find(|axis| matches!(axis.kind, AxisKind::Category | AxisKind::Date))
        .or_else(|| {
            // A scatter or bubble chart has two value axes; the second one is the horizontal one.
            model
                .axes
                .iter()
                .filter(|axis| axis.kind == AxisKind::Value)
                .nth(1)
        })
}

/// Whether the axis of `kind` is suppressed.
fn suppressed(model: &ChartModel, kind: AxisKind) -> bool {
    model
        .axis_for_kind(kind)
        .map(|axis| axis.suppressed)
        .unwrap_or(true)
}

/// Whether the category axis is suppressed.
fn suppressed_category(model: &ChartModel) -> bool {
    category_axis(model)
        .map(|axis| axis.suppressed)
        .unwrap_or(true)
}

/// The value scale the whole chart is measured against.
fn value_scale(model: &ChartModel, extent: Emu) -> Option<Scale> {
    let plot = model.plots.first()?;
    let axis = model.axis_for_kind(AxisKind::Value);
    let intervals = intervals_for(extent);

    if plot.grouping == Grouping::PercentStacked && axis.is_none_or(|axis| axis.minimum.is_none()) {
        return Some(Scale::proportional());
    }
    if let Some(base) = axis.and_then(|axis| axis.logarithm_base) {
        let (low, high) = combined_bounds(model)?;
        return Some(Scale::logarithmic(low, high, base));
    }
    let (low, high) = combined_bounds(model).unwrap_or((0.0, 1.0));
    let baseline = if model.plots.iter().any(baseline_anchored) {
        Baseline::Anchored
    } else {
        Baseline::Floating
    };
    Some(Scale::resolved(
        low,
        high,
        intervals,
        baseline,
        StatedScale {
            minimum: axis.and_then(|axis| axis.minimum),
            maximum: axis.and_then(|axis| axis.maximum),
            major_unit: axis.and_then(|axis| axis.major_unit),
            minor_unit: axis.and_then(|axis| axis.minor_unit),
            logarithm_base: None,
        },
    ))
}

/// Whether a plot's marks are anchored to zero.
fn baseline_anchored(plot: &PlotModel) -> bool {
    matches!(
        plot.kind,
        ChartKind::Bar | ChartKind::Bar3D | ChartKind::Area | ChartKind::Area3D | ChartKind::Radar
    ) || plot.grouping.is_stacked()
}

/// The bounds every plot of the chart reaches together, which is what a combo chart is measured
/// against.
fn combined_bounds(model: &ChartModel) -> Option<(f64, f64)> {
    let mut bounds: Option<(f64, f64)> = None;
    for plot in &model.plots {
        let Some((low, high)) = value_bounds(plot, model.blanks) else {
            continue;
        };
        bounds = Some(match bounds {
            None => (low, high),
            Some((a, b)) => (a.min(low), b.max(high)),
        });
    }
    bounds
}

/// The x scale of a scatter or bubble chart, or `None` for a chart whose horizontal axis is
/// categorical.
fn cross_scale(model: &ChartModel) -> Option<Scale> {
    let plot = model
        .plots
        .iter()
        .find(|plot| matches!(plot.kind, ChartKind::Scatter | ChartKind::Bubble))?;
    let (low, high) = cross_value_bounds(plot)?;
    let axis = category_axis(model);
    Some(Scale::resolved(
        low,
        high,
        6,
        Baseline::Floating,
        StatedScale {
            minimum: axis.and_then(|axis| axis.minimum),
            maximum: axis.and_then(|axis| axis.maximum),
            major_unit: axis.and_then(|axis| axis.major_unit),
            minor_unit: axis.and_then(|axis| axis.minor_unit),
            logarithm_base: None,
        },
    ))
}

/// How many major intervals an axis of `extent` should carry.
fn intervals_for(extent: Emu) -> usize {
    if extent <= Emu::ZERO {
        return 2;
    }
    let ideal = IDEAL_TICK_SPACING.emu().max(1);
    let count = extent.emu() / ideal;
    usize::try_from(count).unwrap_or(2).clamp(2, 12)
}

/// The widest tick label a value scale will draw.
fn widest_value_label(
    scale: &Scale,
    model: &ChartModel,
    metrics: &mut impl TextMetrics,
) -> LayoutSize {
    let percent = model
        .plots
        .first()
        .is_some_and(|plot| plot.grouping == Grouping::PercentStacked);
    scale
        .ticks()
        .map(|value| {
            let text = format_value(value, scale.major, percent);
            metrics.measure(ChartTextRole::Label.text(&text))
        })
        .fold(LayoutSize::ZERO, widest)
}

/// The widest category label the chart will draw.
fn widest_category_label(
    model: &ChartModel,
    metrics: &mut impl TextMetrics,
    cross: Option<&Scale>,
) -> LayoutSize {
    match cross {
        Some(scale) => scale
            .ticks()
            .map(|value| {
                let text = format_value(value, scale.major, false);
                metrics.measure(ChartTextRole::Label.text(&text))
            })
            .fold(LayoutSize::ZERO, widest),
        None => model
            .categories()
            .iter()
            .map(|label| metrics.measure(ChartTextRole::Label.text(label)))
            .fold(LayoutSize::ZERO, widest),
    }
}

/// The larger of two sizes, in each dimension independently.
fn widest(a: LayoutSize, b: LayoutSize) -> LayoutSize {
    LayoutSize::new(a.width.maximum(b.width), a.height.maximum(b.height))
}

/// How much of an axis' own side a piece of furniture takes.
fn along(position: AxisPosition, size: LayoutSize) -> Emu {
    match position {
        AxisPosition::Left | AxisPosition::Right => size.width,
        AxisPosition::Top | AxisPosition::Bottom => size.height,
    }
}

/// Takes `amount` off `rect`'s `position` side.
fn take(rect: LayoutRect, position: AxisPosition, amount: Emu) -> LayoutRect {
    let mut out = rect;
    match position {
        AxisPosition::Left => out.left += amount,
        AxisPosition::Right => out.right -= amount,
        AxisPosition::Top => out.top += amount,
        AxisPosition::Bottom => out.bottom -= amount,
    }
    normalise(out)
}

/// Shrinks a rectangle by `amount` on every side.
fn deflate(rect: LayoutRect, amount: Emu) -> LayoutRect {
    normalise(LayoutRect {
        left: rect.left + amount,
        top: rect.top + amount,
        right: rect.right - amount,
        bottom: rect.bottom - amount,
    })
}

/// Keeps a rectangle from inverting when the furniture wants more room than the frame has.
///
/// A chart in a frame too small for its own legend is a real thing — a reader can drag a chart to an
/// inch across — and the honest answer is a plot area of no size, drawn inside the frame, rather than
/// a rectangle whose right edge is left of its left one.
fn normalise(rect: LayoutRect) -> LayoutRect {
    LayoutRect {
        left: rect.left,
        top: rect.top,
        right: rect.right.maximum(rect.left),
        bottom: rect.bottom.maximum(rect.top),
    }
}

/// A piece of text centred along the top of `rect`.
fn centred_top(
    rect: LayoutRect,
    size: LayoutSize,
    text: &str,
    role: ChartTextRole,
) -> TextPlacement {
    let centre = (rect.left + rect.right).divided_by(2);
    let half = size.width.divided_by(2);
    TextPlacement {
        text: text.to_owned(),
        rect: LayoutRect::from_edges(
            centre - half,
            rect.top,
            centre + half,
            rect.top + size.height,
        ),
        rotation: Angle::ZERO,
        role,
        alignment: 0.5,
    }
}

/// Lays the legend out on its own side of `rect`.
fn lay_out_legend(
    model: &ChartModel,
    legend: &crate::model::LegendModel,
    rect: LayoutRect,
    palette: &ChartPalette,
    metrics: &mut impl TextMetrics,
) -> LegendGeometry {
    let names: Vec<(usize, String, ChartPaint)> = model
        .all_series()
        .enumerate()
        .map(|(index, (_, series))| {
            (
                index,
                series
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Series {}", index + 1)),
                crate::plot::series_paint(series, index, palette),
            )
        })
        .collect();
    let sizes: Vec<LayoutSize> = names
        .iter()
        .map(|(_, name, _)| metrics.measure(ChartTextRole::Label.text(name)))
        .collect();
    let line_height = sizes
        .iter()
        .map(|size| size.height)
        .fold(Emu::ZERO, Emu::maximum)
        .maximum(Emu::from_points(12.0));
    let widest_entry = sizes
        .iter()
        .map(|size| size.width)
        .fold(Emu::ZERO, Emu::maximum);

    let stacked = matches!(
        legend.position,
        LegendPosition::Left | LegendPosition::Right | LegendPosition::TopRight
    );
    let entry_width = SWATCH + GAP + widest_entry;
    let (width, height) = if stacked {
        (
            entry_width,
            line_height.times(i64::try_from(names.len().max(1)).unwrap_or(1)),
        )
    } else {
        (
            sizes
                .iter()
                .map(|size| SWATCH + GAP + size.width + GAP)
                .fold(Emu::ZERO, |total, one| total + one),
            line_height,
        )
    };

    let area = match legend.position {
        LegendPosition::Bottom => LayoutRect::from_edges(
            rect.left,
            rect.bottom - height,
            rect.left + width.minimum(rect.right - rect.left),
            rect.bottom,
        ),
        LegendPosition::Top => LayoutRect::from_edges(
            rect.left,
            rect.top,
            rect.left + width.minimum(rect.right - rect.left),
            rect.top + height,
        ),
        LegendPosition::Left => LayoutRect::from_edges(
            rect.left,
            rect.top,
            rect.left + width,
            rect.top + height.minimum(rect.bottom - rect.top),
        ),
        LegendPosition::Right => LayoutRect::from_edges(
            rect.right - width,
            rect.top,
            rect.right,
            rect.top + height.minimum(rect.bottom - rect.top),
        ),
        LegendPosition::TopRight => LayoutRect::from_edges(
            rect.right - width,
            rect.top,
            rect.right,
            rect.top + height.minimum(rect.bottom - rect.top),
        ),
    };

    let mut entries = Vec::with_capacity(names.len());
    let mut cursor = if stacked { area.top } else { area.left };
    for ((index, name, paint), size) in names.into_iter().zip(sizes) {
        let (swatch, label) = if stacked {
            let top = cursor;
            cursor += line_height;
            (
                LayoutRect::from_edges(
                    area.left,
                    top + (line_height - SWATCH.divided_by(2)).divided_by(2),
                    area.left + SWATCH,
                    top + (line_height + SWATCH.divided_by(2)).divided_by(2),
                ),
                LayoutRect::from_edges(
                    area.left + SWATCH + GAP,
                    top,
                    area.left + SWATCH + GAP + size.width,
                    top + line_height,
                ),
            )
        } else {
            let left = cursor;
            cursor = cursor + SWATCH + GAP + size.width + GAP;
            (
                LayoutRect::from_edges(
                    left,
                    area.top + (line_height - SWATCH.divided_by(2)).divided_by(2),
                    left + SWATCH,
                    area.top + (line_height + SWATCH.divided_by(2)).divided_by(2),
                ),
                LayoutRect::from_edges(
                    left + SWATCH + GAP,
                    area.top,
                    left + SWATCH + GAP + size.width,
                    area.top + line_height,
                ),
            )
        };
        entries.push(LegendEntry {
            series: index,
            swatch,
            paint,
            label: TextPlacement {
                text: name,
                rect: label,
                rotation: Angle::ZERO,
                role: ChartTextRole::Label,
                alignment: 0.0,
            },
        });
    }
    LegendGeometry {
        rect: area,
        entries,
    }
}

/// Builds the finished geometry from the reservation the second pass produced.
fn build(
    model: &ChartModel,
    frame: LayoutRect,
    palette: &ChartPalette,
    metrics: &mut impl TextMetrics,
    reservation: &Reservation,
) -> ChartGeometry {
    let swapped = is_swapped(model);
    let (value_position, category_position) = axis_sides(model);
    let value = reservation.value;
    let categories = model.categories();

    let mut series = Vec::new();
    let mut first_series = 0usize;
    for (plot_index, plot) in model.plots.iter().enumerate() {
        let cross = match (&reservation.cross, plot.kind) {
            (Some(scale), ChartKind::Scatter | ChartKind::Bubble) => CrossScale::Values(*scale),
            _ => CrossScale::Categories {
                count: categories.len().max(
                    plot.series
                        .iter()
                        .map(crate::model::SeriesModel::point_count)
                        .max()
                        .unwrap_or(0),
                ),
                between: category_axis(model)
                    .map(|axis| axis.crosses_between_categories)
                    .unwrap_or(true)
                    || is_bar(plot),
            },
        };
        let plot_frame = PlotFrame {
            rect: reservation.plot,
            swapped: swapped && is_bar(plot),
            value: value.unwrap_or_else(Scale::proportional),
            cross,
            value_reversed: model
                .axis_for_kind(AxisKind::Value)
                .is_some_and(|axis| axis.reversed),
            cross_reversed: category_axis(model).is_some_and(|axis| axis.reversed),
        };
        let context = PlotContext {
            plot_index,
            first_series,
            palette,
            blanks: model.blanks,
        };
        let mut laid = lay_out_plot(plot, &plot_frame, &context);
        for (position, geometry) in laid.iter_mut().enumerate() {
            let Some(model_series) = plot.series.get(position) else {
                continue;
            };
            geometry.trendlines = trendlines(model_series, &plot_frame, model.blanks);
            geometry.error_bars = error_bars(model_series, &plot_frame, model.blanks);
            for point in &mut geometry.points {
                geometry_label(
                    point,
                    model_series,
                    plot,
                    &categories,
                    value.as_ref(),
                    metrics,
                );
            }
        }
        first_series += plot.series.len();
        series.extend(laid);
    }

    let (axes, gridlines) = if has_axes(model) {
        build_axes(
            model,
            reservation,
            value.as_ref(),
            &categories,
            value_position,
            category_position,
            metrics,
        )
    } else {
        (Vec::new(), Vec::new())
    };

    ChartGeometry {
        frame,
        title: reservation.title.clone(),
        plot_area: reservation.plot,
        axes,
        gridlines,
        series,
        legend: reservation.legend.clone(),
    }
}

/// Attaches a point's data label, when its label tiers say to draw one.
fn geometry_label(
    point: &mut crate::geometry::PointGeometry,
    series: &crate::model::SeriesModel,
    plot: &PlotModel,
    categories: &[String],
    scale: Option<&Scale>,
    metrics: &mut impl TextMetrics,
) {
    let Some(text) = point_label(series, plot, point.index, categories, scale) else {
        return;
    };
    let size = metrics.measure(ChartTextRole::Label.text(&text));
    let anchor = match &point.mark {
        crate::geometry::Mark::Bar(rect) => {
            LayoutPoint::new((rect.left + rect.right).divided_by(2), rect.top)
        }
        crate::geometry::Mark::Marker { centre, radius } => {
            LayoutPoint::new(centre.x, centre.y - *radius)
        }
        crate::geometry::Mark::Slice(slice) => {
            let middle = slice.start.radians() + slice.sweep.radians() / 2.0;
            let along = slice.radius.scaled_by(0.7);
            LayoutPoint::new(
                slice.centre.x + along.scaled_by(middle.sin()),
                slice.centre.y - along.scaled_by(middle.cos()),
            )
        }
        crate::geometry::Mark::Span { from, .. } => *from,
        crate::geometry::Mark::Absent => return,
    };
    let half = size.width.divided_by(2);
    point.label = Some(TextPlacement {
        text,
        rect: LayoutRect::from_edges(
            anchor.x - half,
            anchor.y - size.height,
            anchor.x + half,
            anchor.y,
        ),
        rotation: Angle::ZERO,
        role: ChartTextRole::Label,
        alignment: 0.5,
    });
}

/// Builds the axis geometry and the gridlines the axes rule.
#[allow(clippy::too_many_arguments)]
fn build_axes(
    model: &ChartModel,
    reservation: &Reservation,
    value: Option<&Scale>,
    categories: &[String],
    value_position: AxisPosition,
    category_position: AxisPosition,
    metrics: &mut impl TextMetrics,
) -> (Vec<AxisGeometry>, Vec<Gridline>) {
    let plot = reservation.plot;
    let mut axes = Vec::new();
    let mut gridlines = Vec::new();
    let percent = model
        .plots
        .first()
        .is_some_and(|plot| plot.grouping == Grouping::PercentStacked);

    if let (Some(scale), Some(axis)) = (value, model.axis_for_kind(AxisKind::Value)) {
        let index = axes.len();
        let mut ticks = Vec::with_capacity(scale.tick_count());
        for step in 0..scale.tick_count() {
            let tick = scale.tick(step);
            let Some(fraction) = scale.fraction(tick) else {
                continue;
            };
            let at = edge_point(plot, value_position, fraction);
            let text = format_value(tick, scale.major, percent);
            let size = metrics.measure(ChartTextRole::Label.text(&text));
            ticks.push(TickGeometry {
                value: Some(tick),
                at,
                mark: tick_mark(at, value_position, axis.major_tick_mark),
                label: (!axis.suppressed).then(|| label_at(at, value_position, size, &text)),
            });
            if axis.major_gridlines {
                let (from, to) = across(plot, value_position, at);
                gridlines.push(Gridline {
                    axis: index,
                    from,
                    to,
                    minor: false,
                });
            }
        }
        if axis.minor_gridlines {
            gridlines.extend(minor_gridlines(plot, value_position, scale, index));
        }
        axes.push(AxisGeometry {
            kind: AxisKind::Value,
            position: value_position,
            suppressed: axis.suppressed,
            line: (!axis.suppressed).then(|| axis_line(plot, value_position)),
            scale: Some(*scale),
            ticks,
            title: axis
                .title
                .as_ref()
                .map(|title| axis_title(plot, value_position, title, metrics)),
        });
    }

    if let Some(axis) = category_axis(model) {
        let index = axes.len();
        let between = axis.crosses_between_categories;
        let mut ticks = Vec::new();
        match (&reservation.cross, axis.kind) {
            (Some(scale), AxisKind::Value) => {
                for step in 0..scale.tick_count() {
                    let tick = scale.tick(step);
                    let Some(fraction) = scale.fraction(tick) else {
                        continue;
                    };
                    let at = edge_point(plot, category_position, fraction);
                    let text = format_value(tick, scale.major, false);
                    let size = metrics.measure(ChartTextRole::Label.text(&text));
                    ticks.push(TickGeometry {
                        value: Some(tick),
                        at,
                        mark: tick_mark(at, category_position, axis.major_tick_mark),
                        label: (!axis.suppressed)
                            .then(|| label_at(at, category_position, size, &text)),
                    });
                }
            }
            _ => {
                let count = categories.len().max(1);
                for (step, label) in categories.iter().enumerate() {
                    let fraction = if between {
                        (step as f64 + 0.5) / count as f64
                    } else if count > 1 {
                        step as f64 / (count - 1) as f64
                    } else {
                        0.5
                    };
                    let at = edge_point(plot, category_position, fraction);
                    let drawn = (step as u32).is_multiple_of(axis.tick_label_skip.max(1));
                    let size = metrics.measure(ChartTextRole::Label.text(label));
                    ticks.push(TickGeometry {
                        value: None,
                        at,
                        mark: tick_mark(at, category_position, axis.major_tick_mark),
                        label: (drawn && !axis.suppressed)
                            .then(|| label_at(at, category_position, size, label)),
                    });
                    if axis.major_gridlines {
                        let (from, to) = across(plot, category_position, at);
                        gridlines.push(Gridline {
                            axis: index,
                            from,
                            to,
                            minor: false,
                        });
                    }
                }
            }
        }
        axes.push(AxisGeometry {
            kind: axis.kind,
            position: category_position,
            suppressed: axis.suppressed,
            line: (!axis.suppressed).then(|| axis_line(plot, category_position)),
            scale: reservation.cross,
            ticks,
            title: axis
                .title
                .as_ref()
                .map(|title| axis_title(plot, category_position, title, metrics)),
        });
    }

    (axes, gridlines)
}

/// The minor gridlines between an axis' major ticks.
fn minor_gridlines(
    plot: LayoutRect,
    position: AxisPosition,
    scale: &Scale,
    axis: usize,
) -> Vec<Gridline> {
    let divisions = scale.minor_divisions();
    if divisions == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for step in 0..scale.tick_count().saturating_sub(1) {
        let low = scale.tick(step);
        let high = scale.tick(step + 1);
        for division in 1..=divisions {
            let value = low + (high - low) * division as f64 / (divisions + 1) as f64;
            let Some(fraction) = scale.fraction(value) else {
                continue;
            };
            let at = edge_point(plot, position, fraction);
            let (from, to) = across(plot, position, at);
            out.push(Gridline {
                axis,
                from,
                to,
                minor: true,
            });
        }
    }
    out
}

/// Where `fraction` of the way along an axis on `position`'s side of `plot` falls.
fn edge_point(plot: LayoutRect, position: AxisPosition, fraction: f64) -> LayoutPoint {
    let fraction = fraction.clamp(0.0, 1.0);
    match position {
        AxisPosition::Left => LayoutPoint::new(
            plot.left,
            plot.bottom - (plot.bottom - plot.top).scaled_by(fraction),
        ),
        AxisPosition::Right => LayoutPoint::new(
            plot.right,
            plot.bottom - (plot.bottom - plot.top).scaled_by(fraction),
        ),
        AxisPosition::Bottom => LayoutPoint::new(
            plot.left + (plot.right - plot.left).scaled_by(fraction),
            plot.bottom,
        ),
        AxisPosition::Top => LayoutPoint::new(
            plot.left + (plot.right - plot.left).scaled_by(fraction),
            plot.top,
        ),
    }
}

/// The line that crosses the plot area at `at`, for a gridline.
fn across(plot: LayoutRect, position: AxisPosition, at: LayoutPoint) -> (LayoutPoint, LayoutPoint) {
    match position {
        AxisPosition::Left | AxisPosition::Right => (
            LayoutPoint::new(plot.left, at.y),
            LayoutPoint::new(plot.right, at.y),
        ),
        AxisPosition::Top | AxisPosition::Bottom => (
            LayoutPoint::new(at.x, plot.top),
            LayoutPoint::new(at.x, plot.bottom),
        ),
    }
}

/// The axis line itself, from its minimum end to its maximum end.
fn axis_line(plot: LayoutRect, position: AxisPosition) -> (LayoutPoint, LayoutPoint) {
    match position {
        AxisPosition::Left => (
            LayoutPoint::new(plot.left, plot.bottom),
            LayoutPoint::new(plot.left, plot.top),
        ),
        AxisPosition::Right => (
            LayoutPoint::new(plot.right, plot.bottom),
            LayoutPoint::new(plot.right, plot.top),
        ),
        AxisPosition::Bottom => (
            LayoutPoint::new(plot.left, plot.bottom),
            LayoutPoint::new(plot.right, plot.bottom),
        ),
        AxisPosition::Top => (
            LayoutPoint::new(plot.left, plot.top),
            LayoutPoint::new(plot.right, plot.top),
        ),
    }
}

/// One tick mark, or `None` when the axis draws none.
fn tick_mark(
    at: LayoutPoint,
    position: AxisPosition,
    mark: TickMark,
) -> Option<(LayoutPoint, LayoutPoint)> {
    let (outward, inward) = match mark {
        TickMark::None => return None,
        TickMark::Outside => (TICK_LENGTH, Emu::ZERO),
        TickMark::Inside => (Emu::ZERO, TICK_LENGTH),
        TickMark::Cross => (TICK_LENGTH.divided_by(2), TICK_LENGTH.divided_by(2)),
    };
    Some(match position {
        AxisPosition::Left => (
            LayoutPoint::new(at.x - outward, at.y),
            LayoutPoint::new(at.x + inward, at.y),
        ),
        AxisPosition::Right => (
            LayoutPoint::new(at.x + outward, at.y),
            LayoutPoint::new(at.x - inward, at.y),
        ),
        AxisPosition::Bottom => (
            LayoutPoint::new(at.x, at.y + outward),
            LayoutPoint::new(at.x, at.y - inward),
        ),
        AxisPosition::Top => (
            LayoutPoint::new(at.x, at.y - outward),
            LayoutPoint::new(at.x, at.y + inward),
        ),
    })
}

/// A tick label, placed outside the axis line.
fn label_at(
    at: LayoutPoint,
    position: AxisPosition,
    size: LayoutSize,
    text: &str,
) -> TextPlacement {
    let rect = match position {
        AxisPosition::Left => LayoutRect::from_edges(
            at.x - TICK_LENGTH - size.width,
            at.y - size.height.divided_by(2),
            at.x - TICK_LENGTH,
            at.y + size.height.divided_by(2),
        ),
        AxisPosition::Right => LayoutRect::from_edges(
            at.x + TICK_LENGTH,
            at.y - size.height.divided_by(2),
            at.x + TICK_LENGTH + size.width,
            at.y + size.height.divided_by(2),
        ),
        AxisPosition::Bottom => LayoutRect::from_edges(
            at.x - size.width.divided_by(2),
            at.y + TICK_LENGTH,
            at.x + size.width.divided_by(2),
            at.y + TICK_LENGTH + size.height,
        ),
        AxisPosition::Top => LayoutRect::from_edges(
            at.x - size.width.divided_by(2),
            at.y - TICK_LENGTH - size.height,
            at.x + size.width.divided_by(2),
            at.y - TICK_LENGTH,
        ),
    };
    TextPlacement {
        text: text.to_owned(),
        rect,
        rotation: Angle::ZERO,
        role: ChartTextRole::Label,
        alignment: match position {
            AxisPosition::Left => 1.0,
            AxisPosition::Right => 0.0,
            _ => 0.5,
        },
    }
}

/// An axis title, placed outside its tick labels.
///
/// A vertical axis' title is turned a quarter turn anticlockwise, which is what Office draws and
/// what makes a tall label fit beside a plot area.
fn axis_title(
    plot: LayoutRect,
    position: AxisPosition,
    text: &str,
    metrics: &mut impl TextMetrics,
) -> TextPlacement {
    let size = metrics.measure(ChartTextRole::AxisTitle.text(text));
    let (rect, rotation) = match position {
        AxisPosition::Left | AxisPosition::Right => {
            let centre = (plot.top + plot.bottom).divided_by(2);
            let x = if position == AxisPosition::Left {
                plot.left
            } else {
                plot.right
            };
            (
                LayoutRect::from_edges(
                    x - size.height,
                    centre - size.width.divided_by(2),
                    x,
                    centre + size.width.divided_by(2),
                ),
                Angle::from_degrees(90.0),
            )
        }
        AxisPosition::Bottom | AxisPosition::Top => {
            let centre = (plot.left + plot.right).divided_by(2);
            let y = if position == AxisPosition::Bottom {
                plot.bottom
            } else {
                plot.top - size.height
            };
            (
                LayoutRect::from_edges(
                    centre - size.width.divided_by(2),
                    y,
                    centre + size.width.divided_by(2),
                    y + size.height,
                ),
                Angle::ZERO,
            )
        }
    };
    TextPlacement {
        text: text.to_owned(),
        rect,
        rotation,
        role: ChartTextRole::AxisTitle,
        alignment: 0.5,
    }
}
