//! Where every mark goes: one geometry generator per chart family, over one series model.
//!
//! # The projection, written once
//!
//! Every rectangular family — bar, column, line, area, scatter, bubble, stock — is the same two
//! mappings: *which category* becomes a distance along one axis, and *what value* becomes a distance
//! along the other. A column chart and a bar chart differ only in which of the two runs across, which
//! is why [`PlotFrame`] carries a `swapped` flag rather than there being two of everything. The polar
//! families — pie, doughnut, radar — do not fit that and say so by having their own functions.
//!
//! # What is laid out, and what is explicitly not
//!
//! Fourteen of `CT_PlotArea`'s sixteen plot elements produce marks here:
//! `c:barChart`, `c:bar3DChart`, `c:lineChart`, `c:line3DChart`, `c:areaChart`, `c:area3DChart`,
//! `c:pieChart`, `c:pie3DChart`, `c:doughnutChart`, `c:ofPieChart`, `c:scatterChart`,
//! `c:bubbleChart`, `c:radarChart` and `c:stockChart`. Three deliberate limits, each stated here
//! rather than discovered:
//!
//! * **The two surface plots (`c:surfaceChart`, `c:surface3DChart`) plot no data.** A surface is a
//!   three-dimensional mesh over a grid of categories and series, and there is no honest flat
//!   projection of one; drawing its rows as lines would be a different chart wearing its name. Their
//!   furniture — axes, title, legend — is laid out, and their series carry no points.
//! * **Every three-dimensional family is laid out flat.** `c:bar3DChart` is drawn exactly as
//!   `c:barChart` and so on, because `c:view3D`'s rotation, perspective and depth axis are a
//!   projection this engine does not have. The bars are in the right places; they are not in
//!   perspective.
//! * **`c:ofPieChart`'s secondary plot is not laid out.** Every point is drawn in the primary pie,
//!   rather than the last few being aggregated into an "Other" slice and exploded into a second one.
//!   Nothing is dropped, and the split is absent rather than approximated.
//!
//! # Degenerate data
//!
//! Four shapes appear in real files and each has one answer here, exercised by
//! `tests/degenerate_data_is_defined.rs`: a series with no points produces no marks; a series
//! whose values are all zero produces marks of zero extent at the baseline (not a division by zero —
//! [`crate::scale`] never returns a span of zero); a series of one point produces one mark that fills
//! its band; and a `NaN` never reaches here at all, because [`crate::model`] filters a non-finite
//! `c:v` into a blank on the way in.

use mjx_chart::ChartKind;
use mjx_layout::{LayoutPoint, LayoutRect};
use mjx_ooxml_core::measure::{Angle, Emu};

use crate::geometry::{
    ChartPaint, ErrorBarGeometry, Mark, PointGeometry, Polyline, SeriesGeometry, SliceGeometry,
};
use crate::model::{BlankHandling, Grouping, PlotModel, SeriesModel};
use crate::palette::ChartPalette;
use crate::scale::Scale;

/// What runs across a plot: named categories, or a second value scale.
#[derive(Clone, Debug, PartialEq)]
pub enum CrossScale {
    /// A category axis: `count` equally wide bands.
    Categories {
        /// How many categories there are.
        count: usize,
        /// Whether a point sits at the centre of its band (`c:crossBetween val="between"`) rather
        /// than on the tick that separates two bands.
        between: bool,
    },
    /// A second value axis — a scatter or bubble plot's x.
    Values(Scale),
}

/// The plot area and the two mappings that put a value inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotFrame {
    /// The rectangle the marks are drawn in.
    pub rect: LayoutRect,
    /// Whether the value axis runs **across** rather than up — which is the whole difference between
    /// a bar chart and a column chart.
    pub swapped: bool,
    /// The value scale.
    pub value: Scale,
    /// What runs the other way.
    pub cross: CrossScale,
    /// Whether the value axis runs from its maximum end.
    pub value_reversed: bool,
    /// Whether the cross axis does.
    pub cross_reversed: bool,
}

impl PlotFrame {
    /// The extent the value axis runs along.
    fn value_extent(&self) -> Emu {
        if self.swapped {
            self.rect.right - self.rect.left
        } else {
            self.rect.bottom - self.rect.top
        }
    }

    /// The extent the cross axis runs along.
    fn cross_extent(&self) -> Emu {
        if self.swapped {
            self.rect.bottom - self.rect.top
        } else {
            self.rect.right - self.rect.left
        }
    }

    /// Where `fraction` of the way along the value axis falls, in page coordinates.
    ///
    /// A value axis runs **up** when it is vertical and **right** when it is horizontal, which is
    /// why the vertical case subtracts: `y` grows downward on a page and a value grows upward on a
    /// chart, and that inversion is written once here rather than at every call site.
    fn value_at(&self, fraction: f64) -> Emu {
        let fraction = if self.value_reversed {
            1.0 - fraction
        } else {
            fraction
        };
        let along = self.value_extent().scaled_by(clamp_fraction(fraction));
        if self.swapped {
            self.rect.left + along
        } else {
            self.rect.bottom - along
        }
    }

    /// Where `value` falls along the value axis, or `None` for one the scale cannot place.
    pub fn value_position(&self, value: f64) -> Option<Emu> {
        self.value.fraction(value).map(|f| self.value_at(f))
    }

    /// Where the axis' own baseline falls — the line a bar grows from.
    pub fn baseline(&self) -> Emu {
        self.value_at(self.value.baseline_fraction())
    }

    /// Where `fraction` of the way along the cross axis falls.
    fn cross_at(&self, fraction: f64) -> Emu {
        let fraction = if self.cross_reversed {
            1.0 - fraction
        } else {
            fraction
        };
        let along = self.cross_extent().scaled_by(clamp_fraction(fraction));
        if self.swapped {
            // A bar chart's categories run **down** from the top, which is the order a reader sees
            // them listed and the opposite of the value axis' direction.
            self.rect.top + along
        } else {
            self.rect.left + along
        }
    }

    /// How many bands the cross axis has, for a category one.
    pub fn category_count(&self) -> usize {
        match &self.cross {
            CrossScale::Categories { count, .. } => *count,
            CrossScale::Values(_) => 0,
        }
    }

    /// The band the `index`-th category occupies, as `(start, end)` along the cross axis.
    ///
    /// A category axis whose points sit *on* its ticks still has bands — half a band wide at each
    /// end — because a bar has to have a width even on an axis a line chart would put its points on.
    pub fn category_band(&self, index: usize) -> (Emu, Emu) {
        let count = self.category_count().max(1);
        let low = index as f64 / count as f64;
        let high = (index + 1) as f64 / count as f64;
        let (a, b) = (self.cross_at(low), self.cross_at(high));
        (a.minimum(b), a.maximum(b))
    }

    /// Where the `index`-th point sits along the cross axis.
    pub fn cross_position(&self, index: usize, value: Option<f64>) -> Option<Emu> {
        match &self.cross {
            CrossScale::Categories { count, between } => {
                let count = (*count).max(1);
                let fraction = if *between || count == 1 {
                    (index as f64 + 0.5) / count as f64
                } else {
                    index as f64 / (count - 1) as f64
                };
                Some(self.cross_at(fraction))
            }
            CrossScale::Values(scale) => scale.fraction(value?).map(|f| self.cross_at(f)),
        }
    }

    /// A page point from a position on each axis.
    pub fn point(&self, cross: Emu, value: Emu) -> LayoutPoint {
        if self.swapped {
            LayoutPoint::new(value, cross)
        } else {
            LayoutPoint::new(cross, value)
        }
    }

    /// The rectangle between two value positions, spanning `low ..= high` on the cross axis.
    pub fn band_rect(&self, cross: (Emu, Emu), value: (Emu, Emu)) -> LayoutRect {
        if self.swapped {
            LayoutRect::from_edges(value.0, cross.0, value.1, cross.1)
        } else {
            LayoutRect::from_edges(cross.0, value.0, cross.1, value.1)
        }
    }
}

/// Clamps a fraction to the plot area, so a value past a stated axis maximum is drawn at the edge
/// rather than outside the chart's own frame.
fn clamp_fraction(fraction: f64) -> f64 {
    if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The lowest and highest value a plot's series reach, with its stacking applied.
///
/// For a percent-stacked plot the answer is always `0 … 1`: every category fills the axis, whatever
/// the numbers are. For a stacked plot it is the running totals, which is what makes a stack of
/// three sixties reach 180 rather than 60. For everything else it is the plain extremes.
#[must_use]
pub fn value_bounds(plot: &PlotModel, blanks: BlankHandling) -> Option<(f64, f64)> {
    if plot.grouping == Grouping::PercentStacked {
        return Some((0.0, 1.0));
    }
    let points = plot
        .series
        .iter()
        .map(SeriesModel::point_count)
        .max()
        .unwrap_or(0);
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut seen = false;
    if plot.grouping == Grouping::Stacked {
        for point in 0..points {
            let (mut positive, mut negative) = (0.0f64, 0.0f64);
            for series in &plot.series {
                let Some(value) = series.plotted_value(point, blanks) else {
                    continue;
                };
                seen = true;
                if value < 0.0 {
                    negative += value;
                } else {
                    positive += value;
                }
            }
            low = low.min(negative);
            high = high.max(positive);
        }
    } else {
        for series in &plot.series {
            for point in 0..series.point_count() {
                let Some(value) = series.plotted_value(point, blanks) else {
                    continue;
                };
                seen = true;
                low = low.min(value);
                high = high.max(value);
            }
        }
    }
    seen.then_some((low, high))
}

/// The lowest and highest x a scatter or bubble plot reaches — its categories read as numbers.
#[must_use]
pub fn cross_value_bounds(plot: &PlotModel) -> Option<(f64, f64)> {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut seen = false;
    for series in &plot.series {
        for (index, label) in series.categories.iter().enumerate() {
            let value = label
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                // A scatter series whose x cache is missing plots against its point index, which is
                // what Office does and what keeps a y-only scatter drawing.
                .unwrap_or(index as f64 + 1.0);
            seen = true;
            low = low.min(value);
            high = high.max(value);
        }
    }
    seen.then_some((low, high))
}

/// One series' x values, read as numbers.
fn cross_values(series: &SeriesModel) -> Vec<f64> {
    series
        .categories
        .iter()
        .enumerate()
        .map(|(index, label)| {
            label
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .unwrap_or(index as f64 + 1.0)
        })
        .collect()
}

/// Everything a family generator needs that is not the frame.
#[derive(Debug)]
pub struct PlotContext<'a> {
    /// Which plot of the chart this is.
    pub plot_index: usize,
    /// The index of this plot's first series, counted across every plot.
    pub first_series: usize,
    /// The palette a series with no stated fill draws from.
    pub palette: &'a ChartPalette,
    /// What to do with a blank.
    pub blanks: BlankHandling,
}

/// Lays out one plot's series, whichever family it is.
#[must_use]
pub fn lay_out_plot(
    plot: &PlotModel,
    frame: &PlotFrame,
    context: &PlotContext<'_>,
) -> Vec<SeriesGeometry> {
    match plot.kind {
        ChartKind::Bar | ChartKind::Bar3D => bars(plot, frame, context),
        ChartKind::Line | ChartKind::Line3D => lines(plot, frame, context, false),
        ChartKind::Area | ChartKind::Area3D => lines(plot, frame, context, true),
        ChartKind::Scatter | ChartKind::Bubble => scatter(plot, frame, context),
        ChartKind::Pie | ChartKind::Pie3D | ChartKind::Doughnut | ChartKind::OfPie => {
            wedges(plot, frame, context)
        }
        ChartKind::Radar => radar(plot, frame, context),
        ChartKind::Stock => stock(plot, frame, context),
        // The two surface plots. See the module docs: their furniture is laid out and their data is
        // not, because a flat projection of a mesh would be a different chart.
        // The two surface plots, and anything later.
        //
        // `ChartKind` is `#[non_exhaustive]`, so a seventeenth plot element added to `mjx-chart`
        // arrives here rather than failing to compile. It lands with the surface plots: a plot whose
        // marks this engine has never been told about is laid out as furniture with no data, which
        // is the same honest answer and not a guess at what the new element means.
        _ => plot
            .series
            .iter()
            .enumerate()
            .map(|(position, series)| empty_series(plot, series, position, context))
            .collect(),
    }
}

/// A series with its identity and paint but no marks.
fn empty_series(
    plot: &PlotModel,
    series: &SeriesModel,
    position: usize,
    context: &PlotContext<'_>,
) -> SeriesGeometry {
    SeriesGeometry {
        plot: context.plot_index,
        series: context.first_series + position,
        kind: plot.kind,
        name: series.name.clone(),
        paint: series_paint(series, context.first_series + position, context.palette),
        points: Vec::new(),
        connector: None,
        area: None,
        trendlines: Vec::new(),
        error_bars: Vec::new(),
    }
}

/// What paints a series: what the file states, or the document's own accent when it states nothing.
#[must_use]
pub fn series_paint(series: &SeriesModel, index: usize, palette: &ChartPalette) -> ChartPaint {
    ChartPaint {
        fill: series
            .fill
            .clone()
            .or_else(|| ChartPaint::solid(palette.accent(index)).fill),
        line: series.line.clone(),
    }
}

/// What paints one point, or `None` when it takes its series'.
fn point_paint(
    series: &SeriesModel,
    point: usize,
    vary: bool,
    palette: &ChartPalette,
) -> Option<ChartPaint> {
    let stated = series
        .point_formats
        .iter()
        .find(|format| format.index == Some(u32::try_from(point).unwrap_or(u32::MAX)));
    match stated {
        Some(format) if format.fill.is_some() || format.line.is_some() => Some(ChartPaint {
            fill: format.fill.clone(),
            line: format.line.clone(),
        }),
        // `c:varyColors` is what makes a pie's slices different colours. Without it a pie is one
        // colour, which is the "it renders" failure this crate is written against.
        _ if vary => Some(ChartPaint::solid(palette.point_accent(point))),
        _ => None,
    }
}

/// How far a point is exploded out of its pie, as a fraction of the radius.
fn explosion(series: &SeriesModel, point: usize) -> f64 {
    series
        .point_formats
        .iter()
        .find(|format| format.index == Some(u32::try_from(point).unwrap_or(u32::MAX)))
        .and_then(|format| format.explosion)
        .map(|percent| f64::from(percent.min(400)) / 100.0)
        .unwrap_or(0.0)
}

// =================================================================================================
// Bars and columns
// =================================================================================================

/// Bar and column plots, clustered or stacked.
fn bars(plot: &PlotModel, frame: &PlotFrame, context: &PlotContext<'_>) -> Vec<SeriesGeometry> {
    let count = plot.series.len().max(1);
    let gap = f64::from(plot.gap_width.min(500)) / 100.0;
    let overlap = f64::from(plot.overlap.clamp(-100, 100)) / 100.0;
    let stacked = plot.grouping.is_stacked();
    // Clustered: `count` bars share a band, each overlapping its neighbour by `overlap`, with a gap
    // of `gap` bar-widths between one category's cluster and the next. Stacked: one bar per band.
    // Both reduce to a share of the band, which is what keeps the two cases one expression.
    let occupied = if stacked {
        1.0
    } else {
        1.0 + (count as f64 - 1.0) * (1.0 - overlap)
    };
    let share = 1.0 / (occupied + gap).max(0.01);

    let mut running_positive = vec![0.0f64; longest_point_run(plot)];
    let mut running_negative = vec![0.0f64; running_positive.len()];
    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        let mut points = Vec::with_capacity(series.point_count());
        for point in 0..series.point_count() {
            let (band_start, band_end) = frame.category_band(point);
            let band = band_end - band_start;
            let width = band.scaled_by(share);
            let (low, high) = if stacked {
                (
                    band_start + band.scaled_by(gap * share / 2.0),
                    band_end - band.scaled_by(gap * share / 2.0),
                )
            } else {
                let start = band_start
                    + band.scaled_by(gap * share / 2.0)
                    + width.scaled_by(position as f64 * (1.0 - overlap));
                (start, start + width)
            };

            let Some(value) = series.plotted_value(point, context.blanks) else {
                points.push(PointGeometry {
                    index: point,
                    value: None,
                    mark: Mark::Absent,
                    paint: point_paint(series, point, plot.vary_colours, context.palette),
                    label: None,
                });
                continue;
            };
            let (from, to) = if stacked {
                let running = if value < 0.0 {
                    &mut running_negative
                } else {
                    &mut running_positive
                };
                let base = running.get(point).copied().unwrap_or(0.0);
                let top = if plot.grouping == Grouping::PercentStacked {
                    base + proportion(plot, point, value, context.blanks)
                } else {
                    base + value
                };
                if let Some(slot) = running.get_mut(point) {
                    *slot = top;
                }
                (base, top)
            } else {
                (0.0, value)
            };
            let mark = match (frame.value_position(from), frame.value_position(to)) {
                (Some(a), Some(b)) => Mark::Bar(frame.band_rect((low, high), (a, b))),
                _ => Mark::Absent,
            };
            points.push(PointGeometry {
                index: point,
                value: Some(value),
                mark,
                paint: point_paint(series, point, plot.vary_colours, context.palette),
                label: None,
            });
        }
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points,
            connector: None,
            area: None,
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

/// The longest run of points any series of a plot has.
fn longest_point_run(plot: &PlotModel) -> usize {
    plot.series
        .iter()
        .map(SeriesModel::point_count)
        .max()
        .unwrap_or(0)
}

/// One value's share of its category's total, for a hundred-percent-stacked plot.
///
/// The total is the sum of the **magnitudes**, which is what keeps a category holding 3 and −1 from
/// scaling by 2 and drawing a bar one and a half times the axis. A category whose values sum to zero
/// has no proportions and every share is zero.
fn proportion(plot: &PlotModel, point: usize, value: f64, blanks: BlankHandling) -> f64 {
    let total: f64 = plot
        .series
        .iter()
        .filter_map(|series| series.plotted_value(point, blanks))
        .map(f64::abs)
        .sum();
    if total > 0.0 {
        value / total
    } else {
        0.0
    }
}

// =================================================================================================
// Lines and areas
// =================================================================================================

/// Line and area plots. `filled` is what makes the difference: an area plot closes its polyline back
/// along the baseline (or along the stack level below it) and fills the region.
fn lines(
    plot: &PlotModel,
    frame: &PlotFrame,
    context: &PlotContext<'_>,
    filled: bool,
) -> Vec<SeriesGeometry> {
    let stacked = plot.grouping.is_stacked();
    let mut running = vec![0.0f64; longest_point_run(plot)];
    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        let mut points = Vec::with_capacity(series.point_count());
        let mut vertices: Vec<LayoutPoint> = Vec::new();
        let mut floor: Vec<LayoutPoint> = Vec::new();
        for point in 0..series.point_count() {
            let Some(cross) = frame.cross_position(point, None) else {
                continue;
            };
            let Some(value) = series.plotted_value(point, context.blanks) else {
                points.push(PointGeometry {
                    index: point,
                    value: None,
                    mark: Mark::Absent,
                    paint: point_paint(series, point, plot.vary_colours, context.palette),
                    label: None,
                });
                // `dispBlanksAs="span"` joins across the gap, which is what *not* breaking the
                // polyline means; the other two break it.
                if context.blanks != BlankHandling::Span {
                    vertices.clear();
                    floor.clear();
                }
                continue;
            };
            let base = running.get(point).copied().unwrap_or(0.0);
            let top = if !stacked {
                value
            } else if plot.grouping == Grouping::PercentStacked {
                base + proportion(plot, point, value, context.blanks)
            } else {
                base + value
            };
            if stacked {
                if let Some(slot) = running.get_mut(point) {
                    *slot = top;
                }
            }
            let Some(at) = frame.value_position(top) else {
                points.push(PointGeometry {
                    index: point,
                    value: Some(value),
                    mark: Mark::Absent,
                    paint: point_paint(series, point, plot.vary_colours, context.palette),
                    label: None,
                });
                continue;
            };
            let centre = frame.point(cross, at);
            vertices.push(centre);
            if filled {
                let base_at = if stacked {
                    frame
                        .value_position(base)
                        .unwrap_or_else(|| frame.baseline())
                } else {
                    frame.baseline()
                };
                floor.push(frame.point(cross, base_at));
            }
            points.push(PointGeometry {
                index: point,
                value: Some(value),
                mark: Mark::Marker {
                    centre,
                    radius: MARKER_RADIUS,
                },
                paint: point_paint(series, point, plot.vary_colours, context.palette),
                label: None,
            });
        }
        let connector = (vertices.len() >= 2).then(|| Polyline {
            points: vertices.clone(),
            closed: false,
        });
        let area = (filled && vertices.len() >= 2).then(|| {
            let mut ring = vertices.clone();
            ring.extend(floor.iter().rev().copied());
            Polyline {
                points: ring,
                closed: true,
            }
        });
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points,
            connector,
            area,
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

/// Half the width of a line or scatter plot's point marker.
///
/// `GUESS:` Office's default marker is about seven points across, which is what this is half of. No
/// schema states a default marker size; `c:marker > c:size` does when a file carries one, and this
/// engine does not yet read it.
const MARKER_RADIUS: Emu = Emu::from_emu(44_450);

// =================================================================================================
// Scatter and bubble
// =================================================================================================

/// Scatter and bubble plots: both axes are values.
fn scatter(plot: &PlotModel, frame: &PlotFrame, context: &PlotContext<'_>) -> Vec<SeriesGeometry> {
    let joined = matches!(
        plot.scatter_style,
        mjx_chart::ScatterStyle::Line
            | mjx_chart::ScatterStyle::LineWithMarkers
            | mjx_chart::ScatterStyle::SmoothLine
            | mjx_chart::ScatterStyle::SmoothLineWithMarkers
    ) && plot.kind == ChartKind::Scatter;
    let largest_bubble = plot
        .series
        .iter()
        .flat_map(|series| series.bubble_sizes.iter().filter_map(|size| *size))
        .map(f64::abs)
        .fold(0.0f64, f64::max);

    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        let xs = cross_values(series);
        let mut points = Vec::with_capacity(series.point_count());
        let mut vertices: Vec<LayoutPoint> = Vec::new();
        for point in 0..series.point_count() {
            let x = xs.get(point).copied();
            let value = series.plotted_value(point, context.blanks);
            let placed = match (x, value) {
                (Some(x), Some(value)) => frame
                    .cross_position(point, Some(x))
                    .zip(frame.value_position(value))
                    .map(|(cross, at)| frame.point(cross, at)),
                _ => None,
            };
            match placed {
                Some(centre) => {
                    vertices.push(centre);
                    points.push(PointGeometry {
                        index: point,
                        value,
                        mark: Mark::Marker {
                            centre,
                            radius: bubble_radius(series, point, largest_bubble, plot, frame),
                        },
                        paint: point_paint(series, point, plot.vary_colours, context.palette),
                        label: None,
                    });
                }
                None => points.push(PointGeometry {
                    index: point,
                    value,
                    mark: Mark::Absent,
                    paint: point_paint(series, point, plot.vary_colours, context.palette),
                    label: None,
                }),
            }
        }
        let connector = (joined && vertices.len() >= 2).then_some(Polyline {
            points: vertices,
            closed: false,
        });
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points,
            connector,
            area: None,
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

/// How big one bubble is.
///
/// **By area, not by radius.** ECMA-376's `c:sizeRepresents` admits `area` and `w`, and Office's
/// default — and the only honest one — is area: a bubble twice the value is twice the ink, so its
/// radius grows by the square root. Scaling the radius linearly makes a bubble of twice the value
/// look four times as big, which is the classic bubble-chart lie.
fn bubble_radius(
    series: &SeriesModel,
    point: usize,
    largest: f64,
    plot: &PlotModel,
    frame: &PlotFrame,
) -> Emu {
    if plot.kind != ChartKind::Bubble {
        return MARKER_RADIUS;
    }
    let size = series
        .bubble_sizes
        .get(point)
        .copied()
        .flatten()
        .map(f64::abs)
        .unwrap_or(0.0);
    if largest <= 0.0 {
        return MARKER_RADIUS;
    }
    // The largest bubble spans a quarter of the shorter side of the plot area at 100% scale, which
    // is `GUESS:` — Office states no such figure and this is what makes a typical bubble chart read.
    let shorter = (frame.rect.right - frame.rect.left).minimum(frame.rect.bottom - frame.rect.top);
    let full = shorter.scaled_by(0.125 * f64::from(plot.bubble_scale.min(300)) / 100.0);
    full.scaled_by((size / largest).sqrt())
}

// =================================================================================================
// Pie, doughnut and pie-of-pie
// =================================================================================================

/// Pie, doughnut and pie-of-pie plots.
///
/// A pie draws **one** series; a doughnut draws each series as a ring, innermost first, which is the
/// only family where the series index changes the radius rather than the colour.
fn wedges(plot: &PlotModel, frame: &PlotFrame, context: &PlotContext<'_>) -> Vec<SeriesGeometry> {
    let centre = LayoutPoint::new(
        (frame.rect.left + frame.rect.right).divided_by(2),
        (frame.rect.top + frame.rect.bottom).divided_by(2),
    );
    let outer = (frame.rect.right - frame.rect.left)
        .minimum(frame.rect.bottom - frame.rect.top)
        .divided_by(2);
    let rings = plot.series.len().max(1);
    let hole = if plot.kind == ChartKind::Doughnut {
        f64::from(plot.hole_size.min(90)) / 100.0
    } else {
        0.0
    };
    let start_of_first = f64::from(plot.first_slice_angle % 360);

    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        // Every ring is the same thickness, and the hole is a fraction of the whole radius.
        let inner_fraction = hole + (1.0 - hole) * position as f64 / rings as f64;
        let outer_fraction = hole + (1.0 - hole) * (position + 1) as f64 / rings as f64;
        let ring_outer = outer.scaled_by(outer_fraction);
        let ring_inner = outer.scaled_by(inner_fraction);

        let total: f64 = (0..series.point_count())
            .filter_map(|point| series.plotted_value(point, context.blanks))
            .map(f64::abs)
            .sum();
        let mut sweep_so_far = start_of_first;
        let mut points = Vec::with_capacity(series.point_count());
        for point in 0..series.point_count() {
            let value = series.plotted_value(point, context.blanks);
            let share = match (value, total > 0.0) {
                (Some(value), true) => value.abs() / total,
                // Every value zero, or all blank: the slices divide the circle equally, which is
                // what Office draws and what stops a pie of zeroes being a blank rectangle.
                (Some(_), false) => 1.0 / series.point_count().max(1) as f64,
                (None, _) => 0.0,
            };
            let sweep = share * 360.0;
            let mark = if value.is_none() {
                Mark::Absent
            } else {
                Mark::Slice(SliceGeometry {
                    centre,
                    radius: ring_outer,
                    inner_radius: ring_inner,
                    start: Angle::from_degrees(sweep_so_far),
                    sweep: Angle::from_degrees(sweep),
                    explosion: ring_outer.scaled_by(explosion(series, point)),
                })
            };
            sweep_so_far += sweep;
            points.push(PointGeometry {
                index: point,
                value,
                mark,
                // A pie varies its colours whether or not `c:varyColors` says so: one colour for a
                // whole pie is not a pie. The file's own `c:dPt` still wins, inside `point_paint`.
                paint: point_paint(series, point, true, context.palette),
                label: None,
            });
        }
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points,
            connector: None,
            area: None,
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

// =================================================================================================
// Radar
// =================================================================================================

/// Radar plots: the category axis is the circle and the value axis is the radius.
fn radar(plot: &PlotModel, frame: &PlotFrame, context: &PlotContext<'_>) -> Vec<SeriesGeometry> {
    let centre = LayoutPoint::new(
        (frame.rect.left + frame.rect.right).divided_by(2),
        (frame.rect.top + frame.rect.bottom).divided_by(2),
    );
    let radius = (frame.rect.right - frame.rect.left)
        .minimum(frame.rect.bottom - frame.rect.top)
        .divided_by(2);
    let spokes = longest_point_run(plot).max(1);
    let filled = plot.radar_style == mjx_chart::RadarStyle::Filled;

    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        let mut points = Vec::with_capacity(series.point_count());
        let mut vertices = Vec::new();
        for point in 0..series.point_count() {
            let value = series.plotted_value(point, context.blanks);
            let fraction = value.and_then(|value| frame.value.fraction(value));
            match fraction {
                Some(fraction) => {
                    // Spoke zero points straight up, and they run clockwise, which is what Office
                    // draws and the same convention a pie's first slice follows.
                    let angle = std::f64::consts::TAU * point as f64 / spokes as f64;
                    let along = radius.scaled_by(clamp_fraction(fraction));
                    let at = LayoutPoint::new(
                        centre.x + along.scaled_by(angle.sin()),
                        centre.y - along.scaled_by(angle.cos()),
                    );
                    vertices.push(at);
                    points.push(PointGeometry {
                        index: point,
                        value,
                        mark: Mark::Marker {
                            centre: at,
                            radius: MARKER_RADIUS,
                        },
                        paint: point_paint(series, point, plot.vary_colours, context.palette),
                        label: None,
                    });
                }
                None => points.push(PointGeometry {
                    index: point,
                    value,
                    mark: Mark::Absent,
                    paint: point_paint(series, point, plot.vary_colours, context.palette),
                    label: None,
                }),
            }
        }
        let ring = (vertices.len() >= 3).then_some(Polyline {
            points: vertices,
            closed: true,
        });
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points,
            connector: if filled { None } else { ring.clone() },
            area: if filled { ring } else { None },
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

// =================================================================================================
// Stock
// =================================================================================================

/// Stock plots: the series are the columns of a price table, not independent lines.
///
/// A `c:stockChart` carries three series (high, low, close) or four (open, high, low, close), and
/// what a reader sees is **not** three lines: it is a vertical line from the low to the high of each
/// category, with the open and close marked on it. So the high-low line is attached to the *first*
/// series as a [`Mark::Span`], and the fourth series carries the open-to-close bar; the rest carry
/// markers. `GUESS:` which series is which is by position, because `c:stockChart` states no roles —
/// the convention is Excel's and is what every producer writes.
fn stock(plot: &PlotModel, frame: &PlotFrame, context: &PlotContext<'_>) -> Vec<SeriesGeometry> {
    let points = longest_point_run(plot);
    let four = plot.series.len() >= 4;
    let mut out = Vec::with_capacity(plot.series.len());
    for (position, series) in plot.series.iter().enumerate() {
        let index = context.first_series + position;
        let mut geometry = Vec::with_capacity(series.point_count());
        for point in 0..points.max(series.point_count()) {
            let value = series.plotted_value(point, context.blanks);
            let Some(cross) = frame.cross_position(point, None) else {
                continue;
            };
            let mark = if position == 0 {
                // The high-low line: the extremes across every series at this category.
                let extremes = plot
                    .series
                    .iter()
                    .filter_map(|other| other.plotted_value(point, context.blanks))
                    .fold(None::<(f64, f64)>, |acc, value| {
                        Some(match acc {
                            None => (value, value),
                            Some((low, high)) => (low.min(value), high.max(value)),
                        })
                    });
                match extremes.and_then(|(low, high)| {
                    frame.value_position(high).zip(frame.value_position(low))
                }) {
                    Some((high, low)) => Mark::Span {
                        from: frame.point(cross, high),
                        to: frame.point(cross, low),
                    },
                    None => Mark::Absent,
                }
            } else if four && position == plot.series.len() - 1 {
                // The open-to-close bar, drawn between the first series (open) and this one (close).
                let open = plot
                    .series
                    .first()
                    .and_then(|first| first.plotted_value(point, context.blanks));
                let (band_start, band_end) = frame.category_band(point);
                let band = band_end - band_start;
                let low = band_start + band.scaled_by(0.3);
                let high = band_end - band.scaled_by(0.3);
                match open.zip(value).and_then(|(open, close)| {
                    frame.value_position(open).zip(frame.value_position(close))
                }) {
                    Some((a, b)) => Mark::Bar(frame.band_rect((low, high), (a, b))),
                    None => Mark::Absent,
                }
            } else {
                match value.and_then(|value| frame.value_position(value)) {
                    Some(at) => Mark::Marker {
                        centre: frame.point(cross, at),
                        radius: MARKER_RADIUS,
                    },
                    None => Mark::Absent,
                }
            };
            geometry.push(PointGeometry {
                index: point,
                value,
                mark,
                paint: point_paint(series, point, plot.vary_colours, context.palette),
                label: None,
            });
        }
        out.push(SeriesGeometry {
            plot: context.plot_index,
            series: index,
            kind: plot.kind,
            name: series.name.clone(),
            paint: series_paint(series, index, context.palette),
            points: geometry,
            connector: None,
            area: None,
            trendlines: Vec::new(),
            error_bars: Vec::new(),
        });
    }
    out
}

// =================================================================================================
// Trendlines and error bars
// =================================================================================================

/// How many points a fitted curve is sampled at when it is not a straight line.
const TRENDLINE_SAMPLES: usize = 33;

/// Fits every trendline a series carries and samples it into a polyline.
///
/// Six of `ST_TrendlineType`'s seven are fitted: linear, polynomial, exponential, logarithmic, power
/// and moving average. The seventh, `movingAvg`, *is* the moving average — the list is six because
/// `linear` and `polynomial` of order one are one fit. Each is a least-squares fit in the space that
/// makes it linear, which is how a spreadsheet does it: an exponential fit is a linear fit of the
/// logarithms.
#[must_use]
pub fn trendlines(series: &SeriesModel, frame: &PlotFrame, blanks: BlankHandling) -> Vec<Polyline> {
    use mjx_chart::TrendlineKind;

    let sample: Vec<(f64, f64)> = (0..series.point_count())
        .filter_map(|point| {
            let value = series.plotted_value(point, blanks)?;
            Some((point as f64 + 1.0, value))
        })
        .collect();
    if sample.len() < 2 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for trendline in &series.trendlines {
        let kind = trendline.kind.unwrap_or(TrendlineKind::Linear);
        let period = trendline.moving_average_period.unwrap_or(2).clamp(2, 64) as usize;
        let vertices: Vec<LayoutPoint> = match kind {
            TrendlineKind::MovingAverage => sample
                .windows(period)
                .enumerate()
                .filter_map(|(start, window)| {
                    let mean = window.iter().map(|(_, y)| *y).sum::<f64>() / window.len() as f64;
                    let point = start + period - 1;
                    let cross = frame.cross_position(point, Some(window[window.len() - 1].0))?;
                    Some(frame.point(cross, frame.value_position(mean)?))
                })
                .collect(),
            other => {
                let Some(fit) = fit_curve(other, &sample, trendline.polynomial_order) else {
                    continue;
                };
                let first = sample.first().map(|(x, _)| *x).unwrap_or(1.0);
                let last = sample.last().map(|(x, _)| *x).unwrap_or(1.0);
                (0..TRENDLINE_SAMPLES)
                    .filter_map(|step| {
                        let t = step as f64 / (TRENDLINE_SAMPLES - 1) as f64;
                        let x = first + (last - first) * t;
                        let y = fit(x);
                        if !y.is_finite() {
                            return None;
                        }
                        let index = (x - 1.0).round().max(0.0) as usize;
                        // A fitted curve is drawn against the same cross axis its points are, which
                        // for a category axis means interpolating between band centres.
                        let cross = interpolate_cross(frame, x, index)?;
                        Some(frame.point(cross, frame.value_position(y)?))
                    })
                    .collect()
            }
        };
        if vertices.len() >= 2 {
            out.push(Polyline {
                points: vertices,
                closed: false,
            });
        }
    }
    out
}

/// A cross-axis position for a fractional category, so a fitted curve is smooth rather than stepped.
fn interpolate_cross(frame: &PlotFrame, x: f64, index: usize) -> Option<Emu> {
    match &frame.cross {
        CrossScale::Values(_) => frame.cross_position(index, Some(x)),
        CrossScale::Categories { .. } => {
            let whole = (x - 1.0).floor().max(0.0) as usize;
            let fraction = (x - 1.0 - whole as f64).clamp(0.0, 1.0);
            let a = frame.cross_position(whole, None)?;
            let b = frame.cross_position(whole + 1, None).unwrap_or(a);
            Some(a + (b - a).scaled_by(fraction))
        }
    }
}

/// A least-squares fit of `sample`, as a function of x.
fn fit_curve(
    kind: mjx_chart::TrendlineKind,
    sample: &[(f64, f64)],
    order: Option<u32>,
) -> Option<Box<dyn Fn(f64) -> f64>> {
    use mjx_chart::TrendlineKind;
    match kind {
        TrendlineKind::Linear => {
            let (slope, intercept) = least_squares(sample.iter().copied())?;
            Some(Box::new(move |x| slope * x + intercept))
        }
        TrendlineKind::Logarithmic => {
            let (slope, intercept) = least_squares(
                sample
                    .iter()
                    .filter(|(x, _)| *x > 0.0)
                    .map(|(x, y)| (x.ln(), *y)),
            )?;
            Some(Box::new(move |x| {
                if x > 0.0 {
                    slope * x.ln() + intercept
                } else {
                    f64::NAN
                }
            }))
        }
        TrendlineKind::Exponential => {
            let (slope, intercept) = least_squares(
                sample
                    .iter()
                    .filter(|(_, y)| *y > 0.0)
                    .map(|(x, y)| (*x, y.ln())),
            )?;
            let scale = intercept.exp();
            Some(Box::new(move |x| scale * (slope * x).exp()))
        }
        TrendlineKind::Power => {
            let (slope, intercept) = least_squares(
                sample
                    .iter()
                    .filter(|(x, y)| *x > 0.0 && *y > 0.0)
                    .map(|(x, y)| (x.ln(), y.ln())),
            )?;
            let scale = intercept.exp();
            Some(Box::new(move |x| {
                if x > 0.0 {
                    scale * x.powf(slope)
                } else {
                    f64::NAN
                }
            }))
        }
        TrendlineKind::Polynomial => {
            // A polynomial of order two through the least-squares line's residuals is not a
            // polynomial fit, so this is the real thing: a Vandermonde normal-equation solve at the
            // stated order, capped at six because `c:order` admits 2..=6.
            let order = order.unwrap_or(2).clamp(2, 6) as usize;
            let coefficients = polynomial_fit(sample, order)?;
            Some(Box::new(move |x| {
                coefficients
                    .iter()
                    .rev()
                    .fold(0.0, |accumulated, c| accumulated * x + c)
            }))
        }
        TrendlineKind::MovingAverage => None,
    }
}

/// The slope and intercept of the least-squares line through `sample`.
fn least_squares(sample: impl Iterator<Item = (f64, f64)>) -> Option<(f64, f64)> {
    let (mut n, mut sx, mut sy, mut sxx, mut sxy) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
    for (x, y) in sample {
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        n += 1.0;
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    if n < 2.0 {
        return None;
    }
    let denominator = n * sxx - sx * sx;
    if denominator.abs() < f64::EPSILON {
        return None;
    }
    let slope = (n * sxy - sx * sy) / denominator;
    Some((slope, (sy - slope * sx) / n))
}

/// A least-squares polynomial fit, returned lowest coefficient first.
fn polynomial_fit(sample: &[(f64, f64)], order: usize) -> Option<Vec<f64>> {
    let width = order + 1;
    if sample.len() < width {
        return None;
    }
    // Normal equations: `A[i][j] = Σ x^(i+j)`, `b[i] = Σ y·x^i`.
    let mut matrix = vec![0.0f64; width * (width + 1)];
    for (x, y) in sample {
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        for row in 0..width {
            for column in 0..width {
                matrix[row * (width + 1) + column] += x.powi((row + column) as i32);
            }
            matrix[row * (width + 1) + width] += y * x.powi(row as i32);
        }
    }
    gaussian_solve(&mut matrix, width)
}

/// Solves an augmented `width × (width + 1)` system in place, with partial pivoting.
fn gaussian_solve(matrix: &mut [f64], width: usize) -> Option<Vec<f64>> {
    let stride = width + 1;
    for column in 0..width {
        let mut pivot = column;
        for row in column + 1..width {
            if matrix[row * stride + column].abs() > matrix[pivot * stride + column].abs() {
                pivot = row;
            }
        }
        if matrix[pivot * stride + column].abs() < 1e-12 {
            return None;
        }
        for offset in 0..stride {
            matrix.swap(column * stride + offset, pivot * stride + offset);
        }
        let lead = matrix[column * stride + column];
        for row in 0..width {
            if row == column {
                continue;
            }
            let factor = matrix[row * stride + column] / lead;
            for offset in column..stride {
                matrix[row * stride + offset] -= factor * matrix[column * stride + offset];
            }
        }
    }
    Some(
        (0..width)
            .map(|row| matrix[row * stride + width] / matrix[row * stride + row])
            .collect(),
    )
}

/// The error bars a series carries, placed.
///
/// Four of `ST_ErrValType`'s values are computed: `fixedVal`, `percentage`, `stdDev` (of the series)
/// and `cust` (the per-point `c:plus`/`c:minus` runs). `stdErr` is the fifth and is computed as the
/// standard deviation over the square root of the count, which is what the statistic *is*.
#[must_use]
pub fn error_bars(
    series: &SeriesModel,
    frame: &PlotFrame,
    blanks: BlankHandling,
) -> Vec<ErrorBarGeometry> {
    use mjx_chart::{ErrorBarType, ErrorValueType};

    let values: Vec<f64> = (0..series.point_count())
        .filter_map(|point| series.plotted_value(point, blanks))
        .collect();
    let mean = if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    };
    let deviation = if values.len() < 2 {
        0.0
    } else {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64).sqrt()
    };

    let mut out = Vec::new();
    for bars in &series.error_bars {
        let bar_type = bars.bar_type.unwrap_or(ErrorBarType::Both);
        for point in 0..series.point_count() {
            let Some(value) = series.plotted_value(point, blanks) else {
                continue;
            };
            let (plus, minus) = match bars.value_type.unwrap_or(ErrorValueType::FixedValue) {
                ErrorValueType::FixedValue => {
                    let amount = bars.value.unwrap_or(0.0);
                    (amount, amount)
                }
                ErrorValueType::Percentage => {
                    let amount = value.abs() * bars.value.unwrap_or(0.0) / 100.0;
                    (amount, amount)
                }
                ErrorValueType::StandardDeviation => {
                    let amount = deviation * bars.value.unwrap_or(1.0);
                    (amount, amount)
                }
                ErrorValueType::StandardError => {
                    let amount = if values.is_empty() {
                        0.0
                    } else {
                        deviation / (values.len() as f64).sqrt()
                    };
                    (amount, amount)
                }
                ErrorValueType::Custom => (
                    bars.plus_values.get(point).copied().unwrap_or(0.0),
                    bars.minus_values.get(point).copied().unwrap_or(0.0),
                ),
            };
            let high = match bar_type {
                ErrorBarType::Minus => value,
                _ => value + plus.abs(),
            };
            let low = match bar_type {
                ErrorBarType::Plus => value,
                _ => value - minus.abs(),
            };
            let Some(cross) = frame.cross_position(point, None) else {
                continue;
            };
            let Some((a, b)) = frame.value_position(high).zip(frame.value_position(low)) else {
                continue;
            };
            out.push(ErrorBarGeometry {
                point,
                from: frame.point(cross, a),
                to: frame.point(cross, b),
                capped: !bars.no_end_cap.unwrap_or(false),
            });
        }
    }
    out
}
