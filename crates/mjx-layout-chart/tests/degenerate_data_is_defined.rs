//! The four degenerate shapes that appear in real files, each with a defined answer.
//!
//! A chart part is untrusted input, so none of these may panic — but "does not panic" is a weak
//! assertion on its own, satisfied by an engine that returned an empty chart for everything. Each
//! test here says what the answer *is* as well as that there is one.

mod support;

use mjx_layout_chart::{lay_out, ChartLayoutError, ChartModel, ChartPalette, Mark, NominalMetrics};

fn geometry(part: &[u8]) -> mjx_layout_chart::ChartGeometry {
    let model = ChartModel::read(part).expect("the fixture is a readable chart part");
    lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    )
}

#[test]
fn a_series_with_no_points_still_exists() {
    let part = support::bar_chart("clustered", &[support::series(0, "Empty", &[], &[])], "");
    let chart = geometry(&part);
    let series = chart.series(0).expect("an empty series is still a series");
    assert!(series.points.is_empty());
    assert_eq!(series.name.as_deref(), Some("Empty"));
    // The axes are still there, and the value axis still has a scale — Office draws 0 to 1.
    let axis = chart
        .axis(mjx_chart::AxisKind::Value)
        .expect("a value axis");
    let scale = axis
        .scale
        .as_ref()
        .expect("even an empty chart has a scale");
    assert_eq!(scale.minimum, 0.0);
    assert!(
        scale.maximum > 0.0,
        "the axis must have an extent to divide by"
    );
}

#[test]
fn every_value_zero_draws_bars_of_no_height_on_a_real_axis() {
    let part = support::bar_chart(
        "clustered",
        &[support::series(
            0,
            "Flat",
            &["A", "B", "C"],
            &support::values(&[0.0, 0.0, 0.0]),
        )],
        "",
    );
    let chart = geometry(&part);
    let scale = chart
        .axis(mjx_chart::AxisKind::Value)
        .and_then(|axis| axis.scale.as_ref())
        .expect("a scale");
    assert!(
        scale.maximum > scale.minimum,
        "a scale whose ends coincide would divide by zero; it gave {} to {}",
        scale.minimum,
        scale.maximum
    );
    for point in 0..3 {
        let bar = chart.bar(0, point).expect("a zero still draws a bar");
        assert_eq!(
            bar.top, bar.bottom,
            "a zero draws a bar of no height, standing on the baseline"
        );
    }
}

#[test]
fn a_single_point_fills_its_band() {
    let part = support::bar_chart(
        "clustered",
        &[support::series(
            0,
            "One",
            &["Only"],
            &support::values(&[97.0]),
        )],
        "",
    );
    let chart = geometry(&part);
    let bar = chart.bar(0, 0).expect("the one bar");
    let band = (chart.plot_area.right - chart.plot_area.left).emu() as f64;
    let width = (bar.right - bar.left).emu() as f64;
    assert!(
        width > band * 0.2 && width < band,
        "one point should occupy a real share of the plot; it took {}",
        width / band
    );
    // 97 against an axis that reaches at least 97.
    let ticks = chart.value_ticks();
    assert!(ticks.last().copied().is_some_and(|last| last >= 97.0));
}

#[test]
fn a_non_finite_value_never_reaches_the_geometry() {
    // `NaN` and `INF` in a `c:v` are what a spreadsheet writes for `#DIV/0!` round-tripped through a
    // careless producer. They are filtered into blanks on the way in.
    let part = support::bar_chart(
        "clustered",
        &[String::from(
            r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:ptCount val="3"/>
            <c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>NaN</c:v></c:pt><c:pt idx="2"><c:v>inf</c:v></c:pt>
            </c:numCache></c:numRef></c:val></c:ser>"#,
        )],
        "",
    );
    let model = ChartModel::read(&part).expect("readable");
    let series = model.all_series().next().expect("one series").1;
    assert_eq!(series.values, vec![Some(10.0), None, None]);
    let chart = geometry(&part);
    assert_eq!(chart.series(0).unwrap().points[1].mark, Mark::Absent);
    assert_eq!(chart.series(0).unwrap().points[2].mark, Mark::Absent);
    for tick in chart.value_ticks() {
        assert!(tick.is_finite(), "a non-finite value has reached the axis");
    }
}

#[test]
fn an_inverted_stated_scale_is_widened_rather_than_divided_by() {
    // `c:max` below `c:min` describes no axis at all. The answer is an axis, not a division by zero.
    let part = support::bar_chart(
        "clustered",
        &[support::series(0, "A", &["X"], &support::values(&[5.0]))],
        "",
    );
    let inverted = String::from_utf8(part)
        .expect("UTF-8")
        .replace(
            r#"<c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/>"#,
            r#"<c:scaling><c:orientation val="minMax"/><c:max val="10"/><c:min val="90"/></c:scaling><c:delete val="0"/><c:axPos val="l"/>"#,
        )
        .into_bytes();
    let chart = geometry(&inverted);
    let scale = chart
        .axis(mjx_chart::AxisKind::Value)
        .and_then(|axis| axis.scale.as_ref())
        .expect("a scale");
    assert!(
        scale.maximum > scale.minimum,
        "an inverted pair still has to produce an axis; it gave {} to {}",
        scale.minimum,
        scale.maximum
    );
}

#[test]
fn a_frame_smaller_than_its_own_furniture_produces_no_negative_rectangle() {
    // A reader can drag a chart down to an inch across, and a chart whose legend needs more room
    // than the frame has must not produce a plot area whose right edge is left of its left one.
    let part = support::titled_bar_chart(
        "A rather long chart title indeed",
        "r",
        &[support::series(
            0,
            "A series with a long name",
            &["Category one"],
            &support::values(&[1.0]),
        )],
    );
    let model = ChartModel::read(&part).expect("readable");
    let tiny = mjx_layout::LayoutRect::from_edges(
        mjx_ooxml_core::measure::Emu::ZERO,
        mjx_ooxml_core::measure::Emu::ZERO,
        mjx_ooxml_core::measure::Emu::from_points(20.0),
        mjx_ooxml_core::measure::Emu::from_points(12.0),
    );
    let chart = lay_out(&model, tiny, &ChartPalette::OFFICE, &mut NominalMetrics);
    assert!(chart.plot_area.right >= chart.plot_area.left);
    assert!(chart.plot_area.bottom >= chart.plot_area.top);
}

#[test]
fn a_chart_part_that_is_not_a_chart_is_an_error_and_not_a_panic() {
    assert!(matches!(
        ChartModel::read(b"not xml at all <<<"),
        Err(ChartLayoutError::Malformed(_))
    ));
    assert!(matches!(
        ChartModel::read(
            br#"<?xml version="1.0"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>"#
        ),
        Err(ChartLayoutError::NoChart)
    ));
}

#[test]
fn a_point_index_past_the_engines_ceiling_is_dropped_rather_than_allocated_for() {
    // A `c:idx` is an unsigned int, so a malformed part may name point four billion. Allocating a
    // vector that long is how an untrusted file becomes an out-of-memory kill.
    let part = support::bar_chart(
        "clustered",
        &[String::from(
            r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:val><c:numRef><c:f>Sheet1!$B$2</c:f><c:numCache><c:ptCount val="2"/>
            <c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="4000000000"><c:v>2</c:v></c:pt>
            </c:numCache></c:numRef></c:val></c:ser>"#,
        )],
        "",
    );
    let model = ChartModel::read(&part).expect("readable");
    let series = model.all_series().next().expect("one series").1;
    assert_eq!(
        series.values.len(),
        1,
        "the out-of-range point is dropped, not allocated for"
    );
}
