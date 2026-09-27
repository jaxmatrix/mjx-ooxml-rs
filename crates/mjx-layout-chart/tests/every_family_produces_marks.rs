//! All sixteen plot elements `CT_PlotArea` admits, reached one at a time.
//!
//! Fourteen must produce marks; the two surface plots must produce a chart with furniture and no
//! marks, which is this engine's stated deferral rather than an accident. A sweep is the right shape
//! here because the failure it guards against is a family nobody wrote a generator for silently
//! producing an empty chart — which looks exactly like a chart whose data did not load.

mod support;

use mjx_chart::ChartKind;
use mjx_layout_chart::{lay_out, ChartGeometry, ChartModel, ChartPalette, Mark, NominalMetrics};

/// The series body every family in the sweep is given.
fn series() -> String {
    support::series(
        0,
        "A",
        &["P", "Q", "R", "S"],
        &support::values(&[4.0, 8.0, 6.0, 2.0]),
    )
}

/// A second series, for the families that stack or ring.
fn second() -> String {
    support::series(
        1,
        "B",
        &["P", "Q", "R", "S"],
        &support::values(&[2.0, 3.0, 5.0, 7.0]),
    )
}

fn lay(part: &[u8]) -> ChartGeometry {
    let model = ChartModel::read(part).expect("the fixture is a readable chart part");
    lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    )
}

/// How many points of the chart draw something.
fn drawn(chart: &ChartGeometry) -> usize {
    chart
        .series
        .iter()
        .flat_map(|series| series.points.iter())
        .filter(|point| point.mark != Mark::Absent)
        .count()
}

/// Every plot element, its body, and whether it declares axes.
fn families() -> Vec<(&'static str, String, bool, ChartKind)> {
    vec![
        (
            "barChart",
            format!(
                r#"<c:barDir val="col"/><c:grouping val="clustered"/>{}"#,
                series()
            ),
            true,
            ChartKind::Bar,
        ),
        (
            "bar3DChart",
            format!(
                r#"<c:barDir val="col"/><c:grouping val="clustered"/>{}"#,
                series()
            ),
            true,
            ChartKind::Bar3D,
        ),
        (
            "lineChart",
            format!(r#"<c:grouping val="standard"/>{}"#, series()),
            true,
            ChartKind::Line,
        ),
        (
            "line3DChart",
            format!(r#"<c:grouping val="standard"/>{}"#, series()),
            true,
            ChartKind::Line3D,
        ),
        (
            "areaChart",
            format!(r#"<c:grouping val="stacked"/>{}{}"#, series(), second()),
            true,
            ChartKind::Area,
        ),
        (
            "area3DChart",
            format!(r#"<c:grouping val="standard"/>{}"#, series()),
            true,
            ChartKind::Area3D,
        ),
        (
            "pieChart",
            format!(r#"<c:varyColors val="1"/>{}"#, series()),
            false,
            ChartKind::Pie,
        ),
        (
            "pie3DChart",
            format!(r#"<c:varyColors val="1"/>{}"#, series()),
            false,
            ChartKind::Pie3D,
        ),
        (
            "doughnutChart",
            format!(
                r#"<c:varyColors val="1"/>{}{}<c:holeSize val="50"/>"#,
                series(),
                second()
            ),
            false,
            ChartKind::Doughnut,
        ),
        (
            "ofPieChart",
            format!(
                r#"<c:ofPieType val="pie"/><c:varyColors val="1"/>{}"#,
                series()
            ),
            false,
            ChartKind::OfPie,
        ),
        (
            "scatterChart",
            format!(
                r#"<c:scatterStyle val="lineMarker"/>{}"#,
                support::scatter_series(0, "A", &[1.0, 2.0, 3.0], &[4.0, 8.0, 6.0])
            ),
            true,
            ChartKind::Scatter,
        ),
        (
            "bubbleChart",
            support::bubble_series(0, "A", &[1.0, 2.0, 3.0], &[4.0, 8.0, 6.0], &[1.0, 4.0, 9.0]),
            true,
            ChartKind::Bubble,
        ),
        (
            "radarChart",
            format!(r#"<c:radarStyle val="marker"/>{}"#, series()),
            true,
            ChartKind::Radar,
        ),
        (
            "stockChart",
            format!(
                "{}{}{}",
                series(),
                second(),
                support::series(
                    2,
                    "C",
                    &["P", "Q", "R", "S"],
                    &support::values(&[3.0, 5.0, 5.0, 4.0])
                )
            ),
            true,
            ChartKind::Stock,
        ),
        (
            "surfaceChart",
            format!(r#"<c:wireframe val="0"/>{}"#, series()),
            true,
            ChartKind::Surface,
        ),
        (
            "surface3DChart",
            format!(r#"<c:wireframe val="0"/>{}"#, series()),
            true,
            ChartKind::Surface3D,
        ),
    ]
}

#[test]
fn all_sixteen_plot_elements_are_read() {
    for (local, body, axes, kind) in families() {
        let part = support::plot_of(local, &body, axes);
        let model = ChartModel::read(&part)
            .unwrap_or_else(|error| panic!("`c:{local}` did not read: {error}"));
        assert_eq!(
            model.plots.first().map(|plot| plot.kind),
            Some(kind),
            "`c:{local}` read as the wrong kind"
        );
        assert!(
            !model.plots[0].series.is_empty(),
            "`c:{local}` lost its series"
        );
    }
}

#[test]
fn fourteen_families_produce_marks_and_the_two_surfaces_do_not() {
    for (local, body, axes, kind) in families() {
        let part = support::plot_of(local, &body, axes);
        let chart = lay(&part);
        let count = drawn(&chart);
        if matches!(kind, ChartKind::Surface | ChartKind::Surface3D) {
            assert_eq!(
                count, 0,
                "`c:{local}` is a deliberate deferral and must plot nothing"
            );
            assert!(
                !chart.series.is_empty(),
                "`c:{local}` still lists its series, so a legend can name them"
            );
            continue;
        }
        assert!(
            count > 0,
            "`c:{local}` produced no marks at all — which is the failure that looks exactly like \
             a chart whose data did not load"
        );
    }
}

#[test]
fn a_doughnut_makes_one_ring_per_series_with_a_hole_in_the_middle() {
    let (local, body, axes, _) = families()
        .into_iter()
        .find(|(local, ..)| *local == "doughnutChart")
        .expect("the sweep declares a doughnut");
    let chart = lay(&support::plot_of(local, &body, axes));
    let inner = match &chart.series(0).unwrap().points[0].mark {
        Mark::Slice(slice) => *slice,
        other => panic!("expected a slice, got {other:?}"),
    };
    let outer = match &chart.series(1).unwrap().points[0].mark {
        Mark::Slice(slice) => *slice,
        other => panic!("expected a slice, got {other:?}"),
    };
    assert!(
        inner.inner_radius > mjx_ooxml_core::measure::Emu::ZERO,
        "a doughnut has a hole; `c:holeSize val=\"50\"` gave {:?}",
        inner.inner_radius
    );
    assert!(
        inner.radius <= outer.inner_radius,
        "the second series rings the first rather than covering it: {:?} then {:?}",
        inner,
        outer
    );
    assert_eq!(inner.centre, outer.centre, "the rings are concentric");
}

#[test]
fn a_bubbles_radius_follows_the_square_root_of_its_size() {
    let (local, body, axes, _) = families()
        .into_iter()
        .find(|(local, ..)| *local == "bubbleChart")
        .expect("the sweep declares a bubble plot");
    let chart = lay(&support::plot_of(local, &body, axes));
    let radius = |point: usize| match &chart.series(0).unwrap().points[point].mark {
        Mark::Marker { radius, .. } => radius.emu() as f64,
        other => panic!("expected a marker, got {other:?}"),
    };
    // Sizes 1, 4, 9: by **area**, the radii are in the ratio 1 : 2 : 3.
    let (one, four, nine) = (radius(0), radius(1), radius(2));
    assert!(
        (four / one - 2.0).abs() < 0.05,
        "a bubble of four times the value has twice the radius, not four times; got {}",
        four / one
    );
    assert!((nine / one - 3.0).abs() < 0.05, "got {}", nine / one);
}

#[test]
fn a_radar_places_its_first_spoke_straight_up() {
    let (local, body, axes, _) = families()
        .into_iter()
        .find(|(local, ..)| *local == "radarChart")
        .expect("the sweep declares a radar plot");
    let chart = lay(&support::plot_of(local, &body, axes));
    let series = chart.series(0).expect("the radar's series");
    let ring = series
        .connector
        .as_ref()
        .expect("a radar draws a closed outline");
    assert!(
        ring.closed,
        "a radar's outline joins back to its first spoke"
    );
    assert_eq!(ring.points.len(), 4);
    let centre_x = (chart.plot_area.left + chart.plot_area.right).divided_by(2);
    let first = chart.marker(0, 0).expect("the first spoke");
    assert!(
        (first.x - centre_x).emu().abs() < 2,
        "the first spoke points straight up from the centre"
    );
    assert!(first.y < (chart.plot_area.top + chart.plot_area.bottom).divided_by(2));
}

#[test]
fn a_stock_plot_draws_a_high_low_line_on_its_first_series() {
    let (local, body, axes, _) = families()
        .into_iter()
        .find(|(local, ..)| *local == "stockChart")
        .expect("the sweep declares a stock plot");
    let chart = lay(&support::plot_of(local, &body, axes));
    let first = chart.series(0).expect("the first series");
    for point in &first.points {
        match &point.mark {
            Mark::Span { from, to } => {
                assert!(
                    from.y < to.y,
                    "a high-low line runs from the high to the low"
                );
                assert_eq!(from.x, to.x, "and it is vertical");
            }
            other => panic!("a stock plot's first series draws spans, not {other:?}"),
        }
    }
    // The high-low line covers every series' value at that category.
    let others: Vec<_> = (1..3).filter_map(|index| chart.marker(index, 0)).collect();
    let Mark::Span { from, to } = first.points[0].mark else {
        unreachable!("checked above")
    };
    for other in others {
        assert!(other.y >= from.y && other.y <= to.y);
    }
}

#[test]
fn a_stacked_area_fills_between_its_own_level_and_the_one_below() {
    let (local, body, axes, _) = families()
        .into_iter()
        .find(|(local, ..)| *local == "areaChart")
        .expect("the sweep declares an area plot");
    let chart = lay(&support::plot_of(local, &body, axes));
    let lower = chart.series(0).expect("the lower band");
    let upper = chart.series(1).expect("the upper band");
    let lower_region = lower.area.as_ref().expect("an area plot fills a region");
    let upper_region = upper.area.as_ref().expect("and so does the one above it");
    assert!(lower_region.closed && upper_region.closed);
    // Eight vertices each: four along the top and four back along the floor.
    assert_eq!(lower_region.points.len(), 8);
    assert_eq!(upper_region.points.len(), 8);
    // The upper band's top is above the lower band's top at every category, because it is stacked
    // on it.
    for point in 0..4 {
        let below = chart.marker(0, point).expect("the lower band's vertex");
        let above = chart.marker(1, point).expect("the upper band's vertex");
        assert!(
            above.y < below.y,
            "at category {point} the stacked band did not rise above the one under it"
        );
    }
}

#[test]
fn a_three_dimensional_family_lays_out_exactly_as_its_flat_one() {
    // `c:bar3DChart` is laid out as `c:barChart`. Stating that as an assertion is what keeps the
    // deferral honest: it is not that 3-D is unsupported, it is that it is drawn flat.
    let flat = support::plot_of(
        "barChart",
        &format!(
            r#"<c:barDir val="col"/><c:grouping val="clustered"/>{}"#,
            series()
        ),
        true,
    );
    let raised = support::plot_of(
        "bar3DChart",
        &format!(
            r#"<c:barDir val="col"/><c:grouping val="clustered"/>{}"#,
            series()
        ),
        true,
    );
    let flat = lay(&flat);
    let raised = lay(&raised);
    for point in 0..4 {
        assert_eq!(
            flat.bar(0, point),
            raised.bar(0, point),
            "the {point}th bar of a 3-D plot is drawn exactly where the flat one is"
        );
    }
}
