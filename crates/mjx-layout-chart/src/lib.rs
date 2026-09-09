//! `mjx-layout-chart` — chart and diagram layout, built once for all three formats.
//!
//! # Why this is one crate and not three modules
//!
//! A chart in a `.pptx`, a chart in a `.docx` and a chart on an `.xlsx` sheet are **the same chart**:
//! the same `c:chartSpace` part, reached through three different frames. Laying it out three times
//! would be the largest duplication in the whole client platform — `TabSetChartTools` alone is
//! 375–410 controls in each of the three applications over identical markup.
//!
//! So the rank is the design. The three box models sit at **3.6** and share it deliberately, so that
//! an edge between any two of them is *sideways* and `xtask/tests/layering.rs` refuses it: a
//! spreadsheet's box model must not know what a slide is. That is exactly why a chart engine cannot
//! sit beside them — at 3.6 it would be reachable from **none** of them. At **3.55** it is reachable
//! from all three, each edge pointing strictly down, and "built once" is a property of the dependency
//! graph rather than a promise in prose.
//!
//! MJXOFF-176 met the other half of this and reported it rather than routing around it: told to
//! consume MJXOFF-170's DrawingML shape layout, it found that layout inside `mjx-layout-pptx` at 3.6,
//! found the edge sideways, and placed the chart's *frame* while leaving the interior here.
//!
//! # The seam is bytes
//!
//! `chart_part_bytes` is already public on `mjx_pptx::Presentation`, `mjx_docx::Document` and
//! `mjx_xlsx::Workbook`. A host hands those bytes to [`ChartModel::read`] and this crate owns
//! everything from the XML inwards — so the **parse** is shared too, and no two hosts can read a
//! `c:grouping` differently. It is also what lets `tests/the_seam_holds.rs` refuse all three format
//! crates in both dependency sections: an engine that never opens a package cannot learn what one is.
//!
//! # How a host uses it
//!
//! ```no_run
//! use mjx_layout::{FragmentTreeBuilder, LayoutRect, PartId};
//! use mjx_layout_chart::{
//!     emit, lay_out, ChartModel, ChartPalette, ChartResourceTable, NominalMetrics,
//! };
//!
//! # fn demo(chart_part: &[u8], frame: LayoutRect) -> Result<(), Box<dyn std::error::Error>> {
//! let model = ChartModel::read(chart_part)?;
//! // The palette is the *document's* own six accents — never one invented here.
//! let palette = ChartPalette::OFFICE;
//! let geometry = lay_out(&model, frame, &palette, &mut NominalMetrics);
//!
//! let mut builder = FragmentTreeBuilder::new();
//! let mut resources = ChartResourceTable::new(0, 0);
//! emit(
//!     &geometry,
//!     &mut builder,
//!     None,
//!     &mjx_layout_chart::root_address(PartId::PRIMARY),
//!     &mut resources,
//! );
//! # Ok(())
//! # }
//! ```
//!
//! # Nothing here panics
//!
//! The inputs are untrusted files, so a chart with no series, values that are all zero, a single
//! point, an axis whose stated maximum is below its stated minimum, a `c:idx` naming point four
//! billion or a `NaN` in a cache produces a defined chart and never a crash. Every division checks
//! its divisor, every length saturates, and every index into anything a *file* sized is a `get`.
//!
//! `tests/no_panic_on_a_layout_path.rs` holds that rather than this paragraph doing so, and it holds
//! the second half in the only honest way: three files do index a slice, always into a vector they
//! allocated a few lines above at a length they computed themselves, and the suite carries an
//! **enumeration of those three with a reason each**. A fourth file appearing there fails the test
//! until somebody argues for it.
//!
//! # What is deliberately not here
//!
//! Each is stated at its own site as well as listed here, because a limitation a reader has to
//! discover is a defect:
//!
//! * **The two surface plots plot no data** — [`plot`]. Their furniture is laid out; a flat
//!   projection of a three-dimensional mesh would be a different chart.
//! * **Every 3-D family is laid out flat** — [`plot`]. `c:view3D` is not read.
//! * **`c:ofPieChart`'s secondary plot is absent** — [`plot`]. Every point is drawn in the primary
//!   pie; nothing is dropped and nothing is aggregated.
//! * **`c:numFmt` is not applied** — [`label`]. The number-format language has exactly one evaluator
//!   in this workspace, in `mjx-layout-xlsx` at rank 3.6, and the edge would be upward. Moving it
//!   down is a ticket of its own; a value is formatted by its axis step until then.
//! * **`c:plotArea > c:layout`'s stated plot rectangle is not read** — [`space`]. A chart whose
//!   reader has dragged its plot area is laid out by the negotiation instead.
//! * **Chart text is measured, not shaped** — [`text`]. All three hosts pass [`NominalMetrics`]
//!   today; the seam for a real font engine is [`TextMetrics`] and it is one `impl` per host.
//! * **SmartArt evaluates three of the ten layout algorithms and none of the constraint language** —
//!   [`diagram`], which also reports what `mjx-dml`'s `diagram/` module already covers.
//! * **Pivot charts are not absorbed here.** `c:pivotFmts` and `c:pivotSource` are read by nothing in
//!   this crate and a pivot chart lays out as the plain chart its cache describes.

pub mod diagram;
pub mod emit;
pub mod error;
pub mod geometry;
pub mod label;
pub mod model;
pub mod palette;
pub mod plot;
pub mod scale;
pub mod space;
pub mod text;

pub use diagram::{lay_out_diagram, DiagramEdge, DiagramLayout, DiagramNode};
pub use emit::{
    emit, emit_into, root_address, segment, ChartAddress, ChartOutline, ChartResourceTable,
    ChartResources,
};
pub use error::ChartLayoutError;
pub use geometry::{
    AxisGeometry, ChartGeometry, ChartPaint, ErrorBarGeometry, Gridline, LegendEntry,
    LegendGeometry, Mark, PointGeometry, Polyline, SeriesGeometry, SliceGeometry, TextPlacement,
    TickGeometry,
};
pub use label::{format_value, point_label};
pub use model::{
    AxisModel, BlankHandling, ChartModel, Grouping, LegendModel, PlotModel, SeriesModel,
};
pub use palette::{ChartPalette, Rgb};
pub use plot::{CrossScale, PlotContext, PlotFrame};
pub use scale::{nice_number, Baseline, Rounding, Scale, StatedScale, Step};
pub use space::lay_out;
pub use text::{ChartText, ChartTextRole, NominalMetrics, TextMetrics};
