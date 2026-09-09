//! Turning a [`ChartGeometry`] into fragments in a host's own tree.
//!
//! # Why the handles are allocated by the host and not here
//!
//! `mjx_layout::DecorationRef` and `GeometryRef` are opaque numbers whose meaning belongs to
//! *"the layer that issued the handles"* — the box model's own companion, which for PowerPoint is
//! `mjx-scene-pptx` and for Excel `mjx-scene-xlsx`. A chart engine that numbered its own handles
//! from zero would collide with every handle its host had already issued for that page.
//!
//! So this module asks. [`ChartResources`] is the trait a host implements over whatever table it
//! already keeps; [`ChartResourceTable`] is a ready-made one for a host that keeps none and for every
//! test in this crate. A chart's fills reach a painter exactly as a slide's shape's do.
//!
//! # What a chart becomes
//!
//! One `BoxFragment` for the whole chart, and under it, in paint order:
//!
//! 1. the plot area's own box,
//! 2. the gridlines (minor first, so a major rules over a minor),
//! 3. each series' area fill, then its marks, then its connector, then its trendlines and error bars,
//! 4. the axes' lines, tick marks and tick labels,
//! 5. the legend, and the chart title.
//!
//! Text is a `BoxFragment` carrying the label's rectangle rather than a `GlyphRunFragment`, and that
//! is a stated limitation rather than an oversight: a glyph run needs a `ShapedRun`, a shaped run
//! needs a shaper, and this crate measures text through [`crate::text::TextMetrics`] precisely so
//! that it needs neither. A host that hands in a real metric can shape the same strings; the
//! placements this module emits are where they go.

use mjx_layout::{
    BoxFragment, DecorationRef, Fragment, FragmentId, FragmentTreeBuilder, GeometryRef,
    LayoutPoint, LayoutRect, ShapeFragment, SourcePath, SourceRef,
};

use crate::geometry::{ChartGeometry, ChartPaint, Mark, SliceGeometry, TextPlacement};

/// An outline a chart needs drawn that no preset shape names.
///
/// A bar is a rectangle and reaches a painter as a `BoxFragment`, so it is not here. Everything else
/// a chart draws is a path someone has to build, and this is the vocabulary that says which.
#[derive(Clone, Debug, PartialEq)]
pub enum ChartOutline {
    /// A circle inscribed in the fragment's rectangle — a point marker, a bubble.
    Ellipse,
    /// A wedge of a pie or a doughnut.
    Slice(SliceGeometry),
    /// A run of points, open or closed. A closed one is filled; an open one is stroked.
    Polyline {
        /// The vertices.
        points: Vec<LayoutPoint>,
        /// Whether the last joins back to the first.
        closed: bool,
    },
    /// A single straight segment — an axis line, a gridline, a tick mark, an error bar.
    Segment {
        /// One end.
        from: LayoutPoint,
        /// The other.
        to: LayoutPoint,
    },
}

/// Where a chart's paints and outlines are recorded, so that whatever resolves the host's handles can
/// resolve a chart's too.
pub trait ChartResources {
    /// Records `paint` and returns the handle that names it, or `None` when the host cannot take
    /// another — which stops the chart growing rather than failing it.
    fn decoration(&mut self, paint: &ChartPaint) -> Option<DecorationRef>;

    /// Records `outline` and returns the handle that names it.
    fn outline(&mut self, outline: ChartOutline) -> Option<GeometryRef>;
}

/// A resource table for a host that keeps none, and the one every test in this crate uses.
///
/// Handles are numbered from a base the host supplies, so a chart placed on a page that has already
/// issued handles does not collide with them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartResourceTable {
    decoration_base: u64,
    geometry_base: u64,
    decorations: Vec<ChartPaint>,
    outlines: Vec<ChartOutline>,
}

impl ChartResourceTable {
    /// A table numbering its handles from `decoration_base` and `geometry_base`.
    #[must_use]
    pub fn new(decoration_base: u64, geometry_base: u64) -> Self {
        Self {
            decoration_base,
            geometry_base,
            decorations: Vec::new(),
            outlines: Vec::new(),
        }
    }

    /// The paint a handle names, or `None` for one this table did not issue.
    #[must_use]
    pub fn paint(&self, handle: DecorationRef) -> Option<&ChartPaint> {
        self.decorations
            .get(usize::try_from(handle.number().checked_sub(self.decoration_base)?).ok()?)
    }

    /// The outline a handle names.
    #[must_use]
    pub fn shape(&self, handle: GeometryRef) -> Option<&ChartOutline> {
        self.outlines
            .get(usize::try_from(handle.number().checked_sub(self.geometry_base)?).ok()?)
    }

    /// How many paints it holds.
    #[must_use]
    pub fn decoration_count(&self) -> usize {
        self.decorations.len()
    }

    /// How many outlines it holds.
    #[must_use]
    pub fn outline_count(&self) -> usize {
        self.outlines.len()
    }
}

impl ChartResources for ChartResourceTable {
    fn decoration(&mut self, paint: &ChartPaint) -> Option<DecorationRef> {
        let handle = DecorationRef::new(
            self.decoration_base
                .checked_add(u64::try_from(self.decorations.len()).ok()?)?,
        );
        self.decorations.push(paint.clone());
        Some(handle)
    }

    fn outline(&mut self, outline: ChartOutline) -> Option<GeometryRef> {
        let handle = GeometryRef::new(
            self.geometry_base
                .checked_add(u64::try_from(self.outlines.len()).ok()?)?,
        );
        self.outlines.push(outline);
        Some(handle)
    }
}

/// How a chart's fragments are addressed inside their host's document.
///
/// A chart is one object in its host — a `p:graphicFrame`, a `w:drawing`, an `xdr:twoCellAnchor` —
/// and everything inside it is that object's subtree. The host supplies the address of the object;
/// this module extends it, so a hit test on the third bar of the second series answers with a path
/// the host can read back.
#[derive(Clone, Debug)]
pub struct ChartAddress {
    /// The address of the chart object itself.
    pub root: SourceRef,
}

impl ChartAddress {
    /// The address of the chart object.
    #[must_use]
    pub fn new(root: SourceRef) -> Self {
        Self { root }
    }

    /// A child address one segment below the chart's own.
    fn child(&self, segment: u32) -> SourceRef {
        SourceRef::node(self.root.part(), self.root.path().child(segment))
    }

    /// A grandchild address two segments below.
    fn grandchild(&self, segment: u32, sub: u32) -> SourceRef {
        SourceRef::node(self.root.part(), self.root.path().child(segment).child(sub))
    }
}

/// Which child of the chart's own subtree a thing is. Stable, so a hit-test path means the same
/// thing on every host — and public, because a host that hit-tests a chart needs to read one back.
pub mod segment {
    /// The plot area's own box.
    pub const PLOT_AREA: u32 = 0;
    /// The gridlines.
    pub const GRIDLINES: u32 = 1;
    /// The axes.
    pub const AXES: u32 = 2;
    /// The series. Its own children are numbered by series index.
    pub const SERIES: u32 = 3;
    /// The legend.
    pub const LEGEND: u32 = 4;
    /// The chart's title.
    pub const TITLE: u32 = 5;
}

/// Emits a laid-out chart into `builder` under `parent`, and returns the chart's own fragment.
///
/// `None` when the tree is full — which is what `FragmentTreeBuilder::push` reports rather than
/// panicking, and what a chart with four billion points in front of it would reach.
pub fn emit(
    geometry: &ChartGeometry,
    builder: &mut FragmentTreeBuilder,
    parent: Option<FragmentId>,
    address: &ChartAddress,
    resources: &mut impl ChartResources,
) -> Option<FragmentId> {
    let root = builder.push_simple(
        parent,
        address.root.clone(),
        geometry.frame,
        Fragment::Box(BoxFragment {
            decoration: None,
            cell: None,
        }),
    )?;
    emit_into(geometry, builder, root, address, resources)?;
    Some(root)
}

/// Emits a laid-out chart's contents under a fragment the host has already pushed, and returns that
/// fragment.
///
/// **This is what the three box models call**, and [`emit`] is what a caller with no fragment of its
/// own calls. Each host already pushes a box for the object the chart lives in — a
/// `p:graphicFrame`, a `w:drawing`, an `xdr:twoCellAnchor` — and pushing a second one at the same
/// rectangle would put a fragment in the tree that stands for nothing, on every chart, in all three
/// formats.
pub fn emit_into(
    geometry: &ChartGeometry,
    builder: &mut FragmentTreeBuilder,
    root: FragmentId,
    address: &ChartAddress,
    resources: &mut impl ChartResources,
) -> Option<FragmentId> {
    let plot = builder.push_simple(
        Some(root),
        address.child(segment::PLOT_AREA),
        geometry.plot_area,
        Fragment::Box(BoxFragment {
            decoration: None,
            cell: None,
        }),
    )?;

    // Minor gridlines first, so a major rules over a minor where they meet.
    let gridline_paint = ChartPaint::default();
    let gridline_handle = resources.decoration(&gridline_paint);
    for (index, line) in geometry
        .gridlines
        .iter()
        .filter(|line| line.minor)
        .chain(geometry.gridlines.iter().filter(|line| !line.minor))
        .enumerate()
    {
        let outline = resources.outline(ChartOutline::Segment {
            from: line.from,
            to: line.to,
        });
        let Some(outline) = outline else { break };
        builder.push_simple(
            Some(plot),
            address.grandchild(segment::GRIDLINES, u32::try_from(index).unwrap_or(u32::MAX)),
            segment_rect(line.from, line.to),
            Fragment::Shape(ShapeFragment {
                geometry: outline,
                decoration: gridline_handle,
            }),
        )?;
    }

    for series in &geometry.series {
        let index = u32::try_from(series.series).unwrap_or(u32::MAX);
        let series_address = address.grandchild(segment::SERIES, index);
        let bounds = series_bounds(series).unwrap_or(geometry.plot_area);
        let node = builder.push_simple(
            Some(plot),
            series_address.clone(),
            bounds,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        )?;
        let default_paint = resources.decoration(&series.paint);

        if let Some(area) = &series.area {
            let outline = resources.outline(ChartOutline::Polyline {
                points: area.points.clone(),
                closed: true,
            });
            if let Some(outline) = outline {
                builder.push_simple(
                    Some(node),
                    series_address.clone(),
                    polyline_rect(&area.points),
                    Fragment::Shape(ShapeFragment {
                        geometry: outline,
                        decoration: default_paint,
                    }),
                )?;
            }
        }

        for point in &series.points {
            let paint = match &point.paint {
                Some(paint) => resources.decoration(paint),
                None => default_paint,
            };
            match &point.mark {
                Mark::Bar(rect) => {
                    builder.push_simple(
                        Some(node),
                        series_address.clone(),
                        *rect,
                        Fragment::Box(BoxFragment {
                            decoration: paint,
                            cell: None,
                        }),
                    )?;
                }
                Mark::Marker { centre, radius } => {
                    let Some(outline) = resources.outline(ChartOutline::Ellipse) else {
                        break;
                    };
                    builder.push_simple(
                        Some(node),
                        series_address.clone(),
                        LayoutRect::from_edges(
                            centre.x - *radius,
                            centre.y - *radius,
                            centre.x + *radius,
                            centre.y + *radius,
                        ),
                        Fragment::Shape(ShapeFragment {
                            geometry: outline,
                            decoration: paint,
                        }),
                    )?;
                }
                Mark::Slice(slice) => {
                    let Some(outline) = resources.outline(ChartOutline::Slice(*slice)) else {
                        break;
                    };
                    builder.push_simple(
                        Some(node),
                        series_address.clone(),
                        LayoutRect::from_edges(
                            slice.centre.x - slice.radius,
                            slice.centre.y - slice.radius,
                            slice.centre.x + slice.radius,
                            slice.centre.y + slice.radius,
                        ),
                        Fragment::Shape(ShapeFragment {
                            geometry: outline,
                            decoration: paint,
                        }),
                    )?;
                }
                Mark::Span { from, to } => {
                    let Some(outline) = resources.outline(ChartOutline::Segment {
                        from: *from,
                        to: *to,
                    }) else {
                        break;
                    };
                    builder.push_simple(
                        Some(node),
                        series_address.clone(),
                        segment_rect(*from, *to),
                        Fragment::Shape(ShapeFragment {
                            geometry: outline,
                            decoration: paint,
                        }),
                    )?;
                }
                Mark::Absent => {}
            }
            if let Some(label) = &point.label {
                push_text(builder, Some(node), &series_address, label)?;
            }
        }

        if let Some(connector) = &series.connector {
            if let Some(outline) = resources.outline(ChartOutline::Polyline {
                points: connector.points.clone(),
                closed: connector.closed,
            }) {
                builder.push_simple(
                    Some(node),
                    series_address.clone(),
                    polyline_rect(&connector.points),
                    Fragment::Shape(ShapeFragment {
                        geometry: outline,
                        decoration: default_paint,
                    }),
                )?;
            }
        }
        for trendline in &series.trendlines {
            if let Some(outline) = resources.outline(ChartOutline::Polyline {
                points: trendline.points.clone(),
                closed: false,
            }) {
                builder.push_simple(
                    Some(node),
                    series_address.clone(),
                    polyline_rect(&trendline.points),
                    Fragment::Shape(ShapeFragment {
                        geometry: outline,
                        decoration: default_paint,
                    }),
                )?;
            }
        }
        for bar in &series.error_bars {
            if let Some(outline) = resources.outline(ChartOutline::Segment {
                from: bar.from,
                to: bar.to,
            }) {
                builder.push_simple(
                    Some(node),
                    series_address.clone(),
                    segment_rect(bar.from, bar.to),
                    Fragment::Shape(ShapeFragment {
                        geometry: outline,
                        decoration: default_paint,
                    }),
                )?;
            }
        }
    }

    for (index, axis) in geometry.axes.iter().enumerate() {
        let axis_address = address.grandchild(segment::AXES, u32::try_from(index).unwrap_or(0));
        if let Some((from, to)) = axis.line {
            if let Some(outline) = resources.outline(ChartOutline::Segment { from, to }) {
                builder.push_simple(
                    Some(root),
                    axis_address.clone(),
                    segment_rect(from, to),
                    Fragment::Shape(ShapeFragment {
                        geometry: outline,
                        decoration: None,
                    }),
                )?;
            }
        }
        for tick in &axis.ticks {
            if let Some((from, to)) = tick.mark {
                if let Some(outline) = resources.outline(ChartOutline::Segment { from, to }) {
                    builder.push_simple(
                        Some(root),
                        axis_address.clone(),
                        segment_rect(from, to),
                        Fragment::Shape(ShapeFragment {
                            geometry: outline,
                            decoration: None,
                        }),
                    )?;
                }
            }
            if let Some(label) = &tick.label {
                push_text(builder, Some(root), &axis_address, label)?;
            }
        }
        if let Some(title) = &axis.title {
            push_text(builder, Some(root), &axis_address, title)?;
        }
    }

    if let Some(legend) = &geometry.legend {
        let legend_address = address.child(segment::LEGEND);
        let node = builder.push_simple(
            Some(root),
            legend_address.clone(),
            legend.rect,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        )?;
        for entry in &legend.entries {
            let paint = resources.decoration(&entry.paint);
            builder.push_simple(
                Some(node),
                legend_address.clone(),
                entry.swatch,
                Fragment::Box(BoxFragment {
                    decoration: paint,
                    cell: None,
                }),
            )?;
            push_text(builder, Some(node), &legend_address, &entry.label)?;
        }
    }

    if let Some(title) = &geometry.title {
        push_text(builder, Some(root), &address.child(segment::TITLE), title)?;
    }

    Some(root)
}

/// A text placement, as a box carrying its rectangle.
fn push_text(
    builder: &mut FragmentTreeBuilder,
    parent: Option<FragmentId>,
    address: &SourceRef,
    text: &TextPlacement,
) -> Option<FragmentId> {
    builder.push_simple(
        parent,
        address.clone(),
        text.rect,
        Fragment::Box(BoxFragment {
            decoration: None,
            cell: None,
        }),
    )
}

/// The bounding rectangle of a straight segment, which is what a hit test on it answers with.
fn segment_rect(from: LayoutPoint, to: LayoutPoint) -> LayoutRect {
    LayoutRect::from_edges(from.x, from.y, to.x, to.y)
}

/// The bounding rectangle of a polyline.
fn polyline_rect(points: &[LayoutPoint]) -> LayoutRect {
    let Some(first) = points.first() else {
        return LayoutRect::ZERO;
    };
    points.iter().skip(1).fold(
        LayoutRect::from_edges(first.x, first.y, first.x, first.y),
        |rect, point| {
            LayoutRect::from_edges(
                rect.left.minimum(point.x),
                rect.top.minimum(point.y),
                rect.right.maximum(point.x),
                rect.bottom.maximum(point.y),
            )
        },
    )
}

/// The rectangle every mark of a series falls inside.
fn series_bounds(series: &crate::geometry::SeriesGeometry) -> Option<LayoutRect> {
    let mut bounds: Option<LayoutRect> = None;
    let mut absorb = |rect: LayoutRect| {
        bounds = Some(match bounds {
            None => rect,
            Some(current) => LayoutRect::from_edges(
                current.left.minimum(rect.left),
                current.top.minimum(rect.top),
                current.right.maximum(rect.right),
                current.bottom.maximum(rect.bottom),
            ),
        });
    };
    for point in &series.points {
        match &point.mark {
            Mark::Bar(rect) => absorb(*rect),
            Mark::Marker { centre, radius } => absorb(LayoutRect::from_edges(
                centre.x - *radius,
                centre.y - *radius,
                centre.x + *radius,
                centre.y + *radius,
            )),
            Mark::Slice(slice) => absorb(LayoutRect::from_edges(
                slice.centre.x - slice.radius,
                slice.centre.y - slice.radius,
                slice.centre.x + slice.radius,
                slice.centre.y + slice.radius,
            )),
            Mark::Span { from, to } => absorb(segment_rect(*from, *to)),
            Mark::Absent => {}
        }
    }
    if let Some(connector) = &series.connector {
        absorb(polyline_rect(&connector.points));
    }
    bounds
}

/// The root path a host with no addressing of its own can use.
#[must_use]
pub fn root_address(part: mjx_layout::PartId) -> ChartAddress {
    ChartAddress::new(SourceRef::node(part, SourcePath::root()))
}
