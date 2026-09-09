//! Every name this crate exports is reachable from outside it, and each does the one thing its
//! documentation says.
//!
//! # Why a crate needs this at all
//!
//! A `pub` item inside a `pub mod` that the crate root does not re-export is reachable in principle
//! and invisible in practice, and the three box models each reach this crate through a handful of
//! names. A suite that only tested the engine through `lay_out` would leave the rest of the surface
//! unexercised — including [`ChartResources`], which no test inside this crate would otherwise
//! implement, and which is the one trait a host has to.

mod support;

use mjx_layout::{DecorationRef, GeometryRef, PartId};
use mjx_layout_chart::{
    diagram, emit, error, geometry, label, model, palette, plot, scale, space, text, AxisModel,
    Baseline, BlankHandling, ChartAddress, ChartGeometry, ChartLayoutError, ChartModel,
    ChartOutline, ChartPaint, ChartPalette, ChartResourceTable, ChartResources, ChartText,
    ChartTextRole, CrossScale, DiagramLayout, ErrorBarGeometry, Gridline, Grouping, LegendEntry,
    LegendGeometry, LegendModel, Mark, NominalMetrics, PlotFrame, PlotModel, PointGeometry,
    Polyline, Rgb, Scale, SeriesGeometry, SeriesModel, SliceGeometry, StatedScale, Step,
    TextMetrics, TextPlacement, TickGeometry,
};

/// A host's own resource table, written against the trait rather than against the shipped
/// implementation — which is the only way to find out whether the trait is usable.
#[derive(Default)]
struct HostTable {
    paints: Vec<ChartPaint>,
    outlines: Vec<ChartOutline>,
}

impl ChartResources for HostTable {
    fn decoration(&mut self, paint: &ChartPaint) -> Option<DecorationRef> {
        self.paints.push(paint.clone());
        Some(DecorationRef::new(self.paints.len() as u64 - 1))
    }

    fn outline(&mut self, outline: ChartOutline) -> Option<GeometryRef> {
        self.outlines.push(outline);
        Some(GeometryRef::new(self.outlines.len() as u64 - 1))
    }
}

#[test]
fn a_host_can_implement_the_resource_trait_and_emit_through_it() {
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("readable");
    let geometry: ChartGeometry = mjx_layout_chart::lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let mut builder = mjx_layout::FragmentTreeBuilder::new();
    let mut table = HostTable::default();
    let address: ChartAddress = mjx_layout_chart::root_address(PartId::PRIMARY);
    let root = emit::emit(&geometry, &mut builder, None, &address, &mut table)
        .expect("a host's own table is enough to emit through");
    assert!(builder.len() > 1);
    assert!(!table.paints.is_empty() && !table.outlines.is_empty());
    assert!(builder.finish().node(root).is_some());
}

#[test]
fn every_module_is_reachable_by_path() {
    // Naming each module keeps `pub mod` from silently becoming private, which would take every
    // item that is not also re-exported at the root with it.
    let _ = diagram::lay_out_diagram;
    let _ = emit::root_address;
    let _: fn(
        &ChartGeometry,
        &mut mjx_layout::FragmentTreeBuilder,
        mjx_layout::FragmentId,
        &ChartAddress,
        &mut HostTable,
    ) -> Option<mjx_layout::FragmentId> = emit::emit_into;
    let _: Option<error::ChartLayoutError> = None;
    let _: Option<geometry::Mark> = None;
    let _ = label::format_value;
    let _: Option<model::BlankHandling> = None;
    let _: Option<palette::ChartPalette> = None;
    let _ = plot::value_bounds;
    let _ = scale::nice_number;
    let _: fn(
        &ChartModel,
        mjx_layout::LayoutRect,
        &ChartPalette,
        &mut NominalMetrics,
    ) -> ChartGeometry = space::lay_out;
    let _: Option<text::ChartTextRole> = None;
}

#[test]
fn every_re_exported_type_is_nameable_and_does_its_job() {
    // Scale and its neighbours.
    let scale: Scale = Scale::automatic(0.0, 10.0, 5, Baseline::Anchored);
    assert!(scale.tick_count() >= 2);
    let _: StatedScale = StatedScale::default();
    let step: Step = Step::new(5, -1);
    assert_eq!(step.value(), 0.5);

    // The palette.
    let palette: ChartPalette = ChartPalette::from_accents([[1, 2, 3]; 6]);
    let colour: Rgb = palette.accent(3);
    assert_eq!(colour, [1, 2, 3]);

    // The text seam.
    let mut metrics = NominalMetrics;
    let sized = metrics.measure(ChartTextRole::Label.text("1,234"));
    assert!(sized.width > mjx_ooxml_core::measure::Emu::ZERO);
    let _: ChartText<'_> = ChartTextRole::ChartTitle.text("t");

    // The model.
    let part = support::pie_chart(&support::series(
        0,
        "A",
        &["x", "y"],
        &support::values(&[1.0, 2.0]),
    ));
    let model: ChartModel = ChartModel::read(&part).expect("readable");
    assert_eq!(model.series_count(), 1);
    let (_, series): (usize, &SeriesModel) = model.all_series().next().expect("one series");
    assert_eq!(series.point_count(), 2);
    let _: &[PlotModel] = &model.plots;
    let _: &[AxisModel] = &model.axes;
    let _: Option<&LegendModel> = model.legend.as_ref();
    assert_eq!(model.blanks, BlankHandling::Gap);
    assert_eq!(model.plots[0].grouping, Grouping::Standard);

    // The geometry.
    let laid = mjx_layout_chart::lay_out(&model, support::frame(), &palette, &mut NominalMetrics);
    let drawn: &SeriesGeometry = laid.series(0).expect("the series");
    let point: &PointGeometry = &drawn.points[0];
    match &point.mark {
        Mark::Slice(slice) => {
            let _: SliceGeometry = *slice;
        }
        other => panic!("a pie draws slices, not {other:?}"),
    }
    let _: Option<&Polyline> = drawn.connector.as_ref();
    let _: &[ErrorBarGeometry] = &drawn.error_bars;
    let _: &[Gridline] = &laid.gridlines;
    let _: Option<&LegendGeometry> = laid.legend.as_ref();
    let _: Option<&TextPlacement> = laid.title.as_ref();
    let _: Option<&TickGeometry> = laid.axes.first().and_then(|axis| axis.ticks.first());
    let _: Option<&LegendEntry> = laid
        .legend
        .as_ref()
        .and_then(|legend| legend.entries.first());

    // The projection, which a host never builds but a caller reading the geometry may want.
    let frame = PlotFrame {
        rect: support::frame(),
        swapped: false,
        value: scale,
        cross: CrossScale::Categories {
            count: 2,
            between: true,
        },
        value_reversed: false,
        cross_reversed: false,
    };
    assert!(frame.value_position(5.0).is_some());

    // The tables and the errors.
    let _: ChartResourceTable = ChartResourceTable::new(0, 0);
    assert!(matches!(
        ChartModel::read(b"<not-xml"),
        Err(ChartLayoutError::Malformed(_))
    ));
    let _: fn(&[u8]) -> Result<ChartModel, ChartLayoutError> = ChartModel::read;
    let _: Option<DiagramLayout> = None;
}
