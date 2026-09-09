//! What a laid-out chart *is*, before any of it becomes a fragment.
//!
//! # Why there is a value between the model and the tree
//!
//! Two reasons, and the second is the one that matters.
//!
//! The first is testability: a bar's rectangle and an axis' tick values are numbers with correct
//! answers, and asserting them against a [`ChartGeometry`] is one comparison, where asserting them
//! against a `FragmentTree` means walking a tree to find the fragment that ought to be the third bar
//! of the second series. `tests/series_geometry_is_asserted.rs` reads this type.
//!
//! The second is that **a chart is not laid out once**. The plot area's size depends on how much
//! room the tick labels need, and how many tick labels there are depends on how tall the plot area
//! is; [`crate::space`] resolves that in two passes. A value that can be produced, measured and
//! produced again is what makes a second pass cheap. A fragment tree cannot be edited — deliberately,
//! see `mjx_layout::FragmentTree` — so a design that built fragments directly would have to build
//! the whole chart twice.
//!
//! # Degenerate data has defined answers, not absent ones
//!
//! An empty series produces a [`SeriesGeometry`] with no points, not a missing entry: the legend
//! still lists it, because Office lists it. A blank point produces [`Mark::Absent`], which is a
//! point that is not drawn rather than a point that is not there — its index is still addressable,
//! which is what keeps a `c:dPt` anchored to point 4 on point 4.

use mjx_chart::{AxisKind, AxisPosition, ChartKind};
use mjx_dml::{FillSpec, LineSpec};
use mjx_layout::{LayoutPoint, LayoutRect};
use mjx_ooxml_core::measure::{Angle, Emu};

use crate::scale::Scale;
use crate::text::ChartTextRole;

/// What paints one thing a chart draws.
///
/// Both halves are `mjx-dml`'s own specs, which is what the scene companions above already resolve —
/// so a chart's bar and a slide's rectangle reach a painter through one vocabulary rather than two.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChartPaint {
    /// Its fill, or `None` for nothing.
    pub fill: Option<FillSpec>,
    /// Its outline, or `None` for nothing.
    pub line: Option<LineSpec>,
}

impl ChartPaint {
    /// A solid fill of `rgb` with no outline.
    #[must_use]
    pub fn solid(rgb: [u8; 3]) -> Self {
        Self {
            fill: Some(FillSpec::Solid(hex_color(rgb))),
            line: None,
        }
    }

    /// A stroke of `rgb` `width` wide, with no fill.
    #[must_use]
    pub fn stroke(rgb: [u8; 3], width: Emu) -> Self {
        Self {
            fill: None,
            line: Some(LineSpec::solid(
                mjx_dml::LineWidth::from_emu(width.emu()),
                hex_color(rgb),
            )),
        }
    }

    /// Whether it paints nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.line.is_none()
    }
}

/// A `ColorSpec` for a resolved RGB triple.
fn hex_color(rgb: [u8; 3]) -> mjx_dml::ColorSpec {
    mjx_dml::ColorSpec::Srgb(format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))
}

/// A piece of chart text, placed.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPlacement {
    /// What it says.
    pub text: String,
    /// The box it occupies, already sized by the host's [`crate::text::TextMetrics`].
    pub rect: LayoutRect,
    /// How far it is turned, anticlockwise from horizontal. Zero for everything but a rotated tick
    /// label and a vertical axis title.
    pub rotation: Angle,
    /// What it is for, which is what says how big it is set.
    pub role: ChartTextRole,
    /// Where its baseline sits relative to `rect`'s left edge — `0.0` at the left, `0.5` centred,
    /// `1.0` at the right. The reservation is made from the rect; this is what a painter uses to
    /// place the run inside it.
    pub alignment: f64,
}

/// One axis, placed.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisGeometry {
    /// Which element it came from.
    pub kind: AxisKind,
    /// Which side of the plot area it runs along.
    pub position: AxisPosition,
    /// Whether the file says to draw nothing for it. A suppressed axis still **scales** — the plot
    /// is measured against it — and only its line, ticks and labels are absent.
    pub suppressed: bool,
    /// The axis line, from its minimum end to its maximum end, or `None` when it is suppressed.
    pub line: Option<(LayoutPoint, LayoutPoint)>,
    /// The value scale it imposes, or `None` for a category axis, which numbers rather than
    /// measures.
    pub scale: Option<Scale>,
    /// Its ticks, in order from the minimum end.
    pub ticks: Vec<TickGeometry>,
    /// Its title.
    pub title: Option<TextPlacement>,
}

/// One tick of an axis.
#[derive(Clone, Debug, PartialEq)]
pub struct TickGeometry {
    /// The value it stands at, for a value axis; `None` for a category one.
    pub value: Option<f64>,
    /// Where it meets the axis line.
    pub at: LayoutPoint,
    /// The tick mark itself, or `None` when the axis draws none.
    pub mark: Option<(LayoutPoint, LayoutPoint)>,
    /// Its label, or `None` when the axis draws none there — which a `c:tickLblSkip` of 2 makes true
    /// of every other tick.
    pub label: Option<TextPlacement>,
}

/// A line ruled across the plot area at a tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gridline {
    /// Which axis ruled it, as an index into [`ChartGeometry::axes`].
    pub axis: usize,
    /// One end.
    pub from: LayoutPoint,
    /// The other.
    pub to: LayoutPoint,
    /// Whether it is a minor gridline rather than a major one.
    pub minor: bool,
}

/// A run of points, joined.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline {
    /// The vertices, in order.
    pub points: Vec<LayoutPoint>,
    /// Whether the last vertex joins back to the first — a radar plot's outline does, a line plot's
    /// does not.
    pub closed: bool,
}

/// Where one point of a series is drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// A rectangle: a bar, a column, or the body of a stock plot's up/down bar.
    Bar(LayoutRect),
    /// A point marker: a scatter point, a bubble, a line plot's marker, a radar vertex.
    Marker {
        /// Its centre.
        centre: LayoutPoint,
        /// Half its width. A bubble's varies with its third value; every other marker's is fixed.
        radius: Emu,
    },
    /// A wedge of a pie or a doughnut.
    Slice(SliceGeometry),
    /// A vertical line from a low to a high — a stock plot's high-low line.
    Span {
        /// The upper end.
        from: LayoutPoint,
        /// The lower end.
        to: LayoutPoint,
    },
    /// A point the chart does not draw: a blank under `dispBlanksAs="gap"`, or a value the axis
    /// cannot place (a zero on a logarithmic axis).
    ///
    /// **Present rather than omitted.** The index is what a `c:dPt` and a `c:dLbl` are anchored by,
    /// so dropping the entry would renumber every later point's overrides.
    Absent,
}

/// A wedge of a pie or a doughnut, in the polar terms a path builder wants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliceGeometry {
    /// The centre of the whole pie, before this slice's explosion is applied.
    pub centre: LayoutPoint,
    /// The outer radius.
    pub radius: Emu,
    /// The inner radius — zero for a pie, the hole for a doughnut.
    pub inner_radius: Emu,
    /// Where the wedge starts, clockwise from twelve o'clock.
    pub start: Angle,
    /// How far round it goes, clockwise.
    pub sweep: Angle,
    /// How far the wedge is pulled out along its own bisector.
    pub explosion: Emu,
}

/// The uncertainty drawn around one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorBarGeometry {
    /// Which point it belongs to.
    pub point: usize,
    /// One end of the bar.
    pub from: LayoutPoint,
    /// The other.
    pub to: LayoutPoint,
    /// Whether the ends are capped.
    pub capped: bool,
}

/// One series, laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesGeometry {
    /// Which plot of the chart holds it.
    pub plot: usize,
    /// Which series it is, counted across every plot — the index a legend, a `c:dPt` and this
    /// crate's palette all agree on.
    pub series: usize,
    /// What kind of plot holds it, so a consumer knows what its marks mean without looking them up.
    pub kind: ChartKind,
    /// Its name, or `None`.
    pub name: Option<String>,
    /// What paints its marks by default. A point that overrides it carries its own.
    pub paint: ChartPaint,
    /// Its points, one entry per category, in index order.
    pub points: Vec<PointGeometry>,
    /// The line joining its points — a line plot's line, an area plot's boundary, a radar plot's
    /// outline. `None` for a family that draws no connector.
    pub connector: Option<Polyline>,
    /// The closed region an area plot fills, `None` for every other family.
    pub area: Option<Polyline>,
    /// The curves fitted through it, already sampled into polylines.
    pub trendlines: Vec<Polyline>,
    /// Its error bars.
    pub error_bars: Vec<ErrorBarGeometry>,
}

/// One point of a series, laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct PointGeometry {
    /// Its index within the series — the number a `c:dPt` and a `c:dLbl` are anchored by.
    pub index: usize,
    /// The value it draws, or `None` for a blank.
    pub value: Option<f64>,
    /// Where it is drawn.
    pub mark: Mark,
    /// What paints it, if it differs from its series'; `None` when it takes its series'.
    pub paint: Option<ChartPaint>,
    /// Its data label, if the label tiers say to draw one.
    pub label: Option<TextPlacement>,
}

/// One entry of a legend.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendEntry {
    /// Which series it stands for, counted across every plot.
    pub series: usize,
    /// The colour swatch.
    pub swatch: LayoutRect,
    /// What paints the swatch.
    pub paint: ChartPaint,
    /// The series' name, placed.
    pub label: TextPlacement,
}

/// A chart's legend, laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendGeometry {
    /// The whole legend's box.
    pub rect: LayoutRect,
    /// Its entries, in series order.
    pub entries: Vec<LegendEntry>,
}

/// A whole chart, laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartGeometry {
    /// The frame the chart was given — the rectangle its `p:graphicFrame`, its `wp:anchor` or its
    /// `xdr:twoCellAnchor` put it in.
    pub frame: LayoutRect,
    /// The chart's title.
    pub title: Option<TextPlacement>,
    /// The rectangle the data is drawn in, after the title, legend, tick labels and axis titles have
    /// taken their room.
    pub plot_area: LayoutRect,
    /// Its axes.
    pub axes: Vec<AxisGeometry>,
    /// The gridlines its axes rule.
    pub gridlines: Vec<Gridline>,
    /// Its series, in the order a legend lists them.
    pub series: Vec<SeriesGeometry>,
    /// Its legend.
    pub legend: Option<LegendGeometry>,
}

impl ChartGeometry {
    /// The geometry of the `series`-th series, counted across every plot.
    #[must_use]
    pub fn series(&self, series: usize) -> Option<&SeriesGeometry> {
        self.series.iter().find(|entry| entry.series == series)
    }

    /// The rectangle the `point`-th point of the `series`-th series is drawn in, for a family whose
    /// marks are rectangles. `None` for a point that is absent or is not a bar.
    #[must_use]
    pub fn bar(&self, series: usize, point: usize) -> Option<LayoutRect> {
        match self.point(series, point)?.mark {
            Mark::Bar(rect) => Some(rect),
            _ => None,
        }
    }

    /// Where the `point`-th point of the `series`-th series sits, for a family whose marks are
    /// points. `None` for a point that is absent or is not a marker.
    #[must_use]
    pub fn marker(&self, series: usize, point: usize) -> Option<LayoutPoint> {
        match self.point(series, point)?.mark {
            Mark::Marker { centre, .. } => Some(centre),
            _ => None,
        }
    }

    /// One point of one series.
    #[must_use]
    pub fn point(&self, series: usize, point: usize) -> Option<&PointGeometry> {
        self.series(series)?
            .points
            .iter()
            .find(|entry| entry.index == point)
    }

    /// The first axis of `kind`, for a caller checking a scale.
    #[must_use]
    pub fn axis(&self, kind: AxisKind) -> Option<&AxisGeometry> {
        self.axes.iter().find(|axis| axis.kind == kind)
    }

    /// The major tick values of the first value axis, which is what a tick-selection test asserts.
    #[must_use]
    pub fn value_ticks(&self) -> Vec<f64> {
        self.axis(AxisKind::Value)
            .and_then(|axis| axis.scale.as_ref())
            .map(|scale| scale.ticks().collect())
            .unwrap_or_default()
    }
}
