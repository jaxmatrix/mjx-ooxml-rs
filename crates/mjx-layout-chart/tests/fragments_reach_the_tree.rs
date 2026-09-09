//! A laid-out chart becomes fragments in a host's own tree, with the host's own handles — and the
//! text metric a host hands in really is what the negotiation uses.
//!
//! # Two seams, both proved live rather than described
//!
//! [`the_metric_a_host_hands_in_changes_the_plot_area`] is the reachability instrument applied to
//! [`mjx_layout_chart::TextMetrics`]: a trait with one implementation is a trait nobody has tested,
//! so this suite carries a second one that reports absurdly wide text and watches the plot area
//! shrink. If the engine ever stops measuring, that test fails and nothing else does.
//!
//! [`the_handles_are_the_hosts_and_resolve_back`] does the same for
//! [`mjx_layout_chart::ChartResources`]: the numbers are allocated from a base the host supplies, so
//! a chart placed on a page that has already issued fifty handles issues its fifty-first.

mod support;

use mjx_layout::{Fragment, FragmentTreeBuilder, LayoutSize, PartId, SourcePath, SourceRef};
use mjx_layout_chart::{
    emit, lay_out, segment, ChartAddress, ChartModel, ChartPalette, ChartResourceTable, ChartText,
    NominalMetrics, TextMetrics,
};
use mjx_ooxml_core::measure::Emu;

/// A metric that reports every string as an inch wide per character and four ems tall — absurd,
/// deterministic, and unlike [`NominalMetrics`] in *both* dimensions, which is what makes the
/// assertions below discriminating.
struct WideMetrics;

impl TextMetrics for WideMetrics {
    fn measure(&mut self, text: ChartText<'_>) -> LayoutSize {
        LayoutSize::new(
            Emu::from_inches(text.text.chars().count() as f64),
            Emu::from_points(text.size_points * 4.0),
        )
    }
}

#[test]
fn the_metric_a_host_hands_in_changes_the_plot_area() {
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("readable");
    let nominal = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let wide = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut WideMetrics,
    );
    assert!(
        wide.plot_area.left > nominal.plot_area.left,
        "wider tick labels must take more of the width; the seam is not being used"
    );
    assert!(
        wide.plot_area.top > nominal.plot_area.top,
        "a taller title must take more of the height"
    );
    // The label rectangles are the metric's own answers, not the engine's.
    let tick = wide
        .axis(mjx_chart::AxisKind::Value)
        .and_then(|axis| axis.ticks.first())
        .and_then(|tick| tick.label.clone())
        .expect("a labelled tick");
    let expected = Emu::from_inches(tick.text.chars().count() as f64);
    assert_eq!(tick.rect.right - tick.rect.left, expected);
}

#[test]
fn the_two_passes_settle_rather_than_running_once() {
    // The plot area's height decides the tick count, and the tick labels' width decides the plot
    // area. A single pass would reserve the gutter for a scale derived from the *whole* frame; the
    // second pass derives it from what the first left. The observable consequence is that laying the
    // same chart out inside its own resulting plot area again is a fixed point.
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("readable");
    let once = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let twice = lay_out(
        &model,
        once.frame,
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    assert_eq!(
        once.plot_area, twice.plot_area,
        "laying the same chart out in the same frame twice must give the same answer"
    );
    assert_eq!(once.value_ticks(), twice.value_ticks());
}

#[test]
fn the_handles_are_the_hosts_and_resolve_back() {
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("readable");
    let geometry = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );

    let mut builder = FragmentTreeBuilder::new();
    // A host that has already issued fifty of each.
    let mut resources = ChartResourceTable::new(50, 70);
    let address = ChartAddress::new(SourceRef::node(PartId::new(7), SourcePath::new(&[3, 1])));
    let root = emit(&geometry, &mut builder, None, &address, &mut resources)
        .expect("the chart fits in an empty tree");
    let tree = builder.finish();

    assert!(
        tree.len() > 20,
        "a decorated chart is more than a handful of fragments"
    );
    let root_node = tree.node(root).expect("the root is in the tree");
    assert_eq!(root_node.rect(), geometry.frame);
    assert_eq!(root_node.source().part(), PartId::new(7));
    assert_eq!(root_node.source().path().segments(), &[3, 1]);

    // Every handle the chart issued is one this table can resolve, and none is below the base.
    let mut decorations = 0usize;
    let mut geometries = 0usize;
    for (_, node) in tree.nodes() {
        match node.fragment() {
            Fragment::Box(box_fragment) => {
                if let Some(handle) = box_fragment.decoration {
                    assert!(handle.number() >= 50, "a handle below the host's base");
                    assert!(
                        resources.paint(handle).is_some(),
                        "an unresolvable paint handle"
                    );
                    decorations += 1;
                }
            }
            Fragment::Shape(shape) => {
                assert!(shape.geometry.number() >= 70);
                assert!(
                    resources.shape(shape.geometry).is_some(),
                    "an unresolvable outline handle"
                );
                geometries += 1;
            }
            _ => {}
        }
    }
    assert!(
        decorations > 0,
        "the bars and the legend swatch are painted"
    );
    assert!(
        geometries > 0,
        "the gridlines and the axis lines are outlines"
    );
    assert!(resources.decoration_count() > 0 && resources.outline_count() > 0);
}

#[test]
fn a_bar_is_addressable_as_its_own_series() {
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("readable");
    let geometry = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let mut builder = FragmentTreeBuilder::new();
    let mut resources = ChartResourceTable::new(0, 0);
    let address = ChartAddress::new(SourceRef::node(PartId::PRIMARY, SourcePath::new(&[2])));
    emit(&geometry, &mut builder, None, &address, &mut resources).expect("emits");
    let tree = builder.finish();

    // A hit test on the first bar must answer with a path that says *this chart, its series, series
    // zero* — which is what makes a click on a bar selectable as a series rather than as "some box".
    let bar = geometry.bar(0, 0).expect("the first bar");
    let found = tree
        .nodes()
        .find(|(_, node)| node.rect() == bar && matches!(node.fragment(), Fragment::Box(_)))
        .map(|(_, node)| node.source().path().segments().to_vec())
        .expect("the bar is in the tree");
    assert_eq!(
        found,
        vec![2, segment::SERIES, 0],
        "a bar's address is the chart's own, then the series segment, then the series index"
    );
}

#[test]
fn an_empty_chart_still_emits_a_frame() {
    let part = support::bar_chart("clustered", &[], "");
    let model = ChartModel::read(&part).expect("readable");
    let geometry = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let mut builder = FragmentTreeBuilder::new();
    let mut resources = ChartResourceTable::new(0, 0);
    let root = emit(
        &geometry,
        &mut builder,
        None,
        &mjx_layout_chart::root_address(PartId::PRIMARY),
        &mut resources,
    )
    .expect("a chart with no series still occupies its frame");
    let tree = builder.finish();
    assert_eq!(
        tree.node(root).map(|node| node.rect()),
        Some(geometry.frame)
    );
}
