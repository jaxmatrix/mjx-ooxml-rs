//! Where the marks actually go, asserted as rectangles and points.
//!
//! # Why "it renders" is not the assertion
//!
//! A chart with axes, gridlines, a legend and **no plotted data at all** looks entirely plausible in
//! a thumbnail, and a suite that only asked whether the fragment tree was non-empty would pass on
//! one. So every test here names a specific mark — the third bar of the first series, the second
//! point of a scatter series, the first slice of a pie — and asserts where it is.
//!
//! # The invariants, rather than the exact EMU
//!
//! A bar's rectangle depends on the frame margin, the gap width, the tick-label gutter and the text
//! metric, and asserting all four as one number would make this suite a change detector for every
//! constant in the engine. What it asserts instead are the relations that would break under a real
//! defect: bars stand **on** the baseline, a taller value is a taller bar, bars of one category do
//! not overlap the next category's, a stack's segments meet exactly, and a pie's slices sweep a whole
//! turn in proportion to their values.

mod support;

use mjx_layout_chart::{lay_out, ChartModel, ChartPalette, Mark, NominalMetrics};

/// Lays a chart part out in the standard frame.
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
fn a_column_stands_on_the_baseline_and_its_height_follows_its_value() {
    let part = support::bar_chart(
        "clustered",
        &[support::series(
            0,
            "Revenue",
            &["Q1", "Q2", "Q3", "Q4"],
            &support::values(&[10.0, 40.0, 20.0, 30.0]),
        )],
        "",
    );
    let chart = geometry(&part);

    let bars: Vec<_> = (0..4)
        .map(|point| chart.bar(0, point).expect("every point draws a bar"))
        .collect();

    // Every bar's bottom is the axis' zero, and they all agree on it to the EMU.
    let baseline = bars[0].bottom;
    for (index, bar) in bars.iter().enumerate() {
        assert_eq!(
            bar.bottom, baseline,
            "bar {index} does not stand on the same baseline as the first"
        );
    }
    // Height follows value: 40 is the tallest, 10 the shortest, and 40 is four times 10.
    let heights: Vec<i64> = bars
        .iter()
        .map(|bar| (bar.bottom - bar.top).emu())
        .collect();
    assert!(heights[1] > heights[3] && heights[3] > heights[2] && heights[2] > heights[0]);
    let ratio = heights[1] as f64 / heights[0] as f64;
    assert!(
        (ratio - 4.0).abs() < 0.02,
        "a value of 40 must draw four times the height of a value of 10; the ratio was {ratio}"
    );

    // The bars run left to right in category order, and none reaches into the next category's band.
    for window in bars.windows(2) {
        assert!(
            window[0].right <= window[1].left,
            "two clustered bars of different categories overlap: {:?} and {:?}",
            window[0],
            window[1]
        );
    }
    // And they all sit inside the plot area.
    for bar in &bars {
        assert!(bar.left >= chart.plot_area.left && bar.right <= chart.plot_area.right);
        assert!(bar.top >= chart.plot_area.top && bar.bottom <= chart.plot_area.bottom);
    }
}

#[test]
fn two_clustered_series_share_a_category_band_without_overlapping() {
    let part = support::bar_chart(
        "clustered",
        &[
            support::series(0, "A", &["X", "Y"], &support::values(&[10.0, 20.0])),
            support::series(1, "B", &["X", "Y"], &support::values(&[15.0, 5.0])),
        ],
        "",
    );
    let chart = geometry(&part);
    let first = chart.bar(0, 0).expect("series 0, point 0");
    let second = chart.bar(1, 0).expect("series 1, point 0");
    assert!(
        first.right <= second.left,
        "the second series' bar must sit beside the first, not on it: {first:?} then {second:?}"
    );
    assert_eq!(
        (first.right - first.left).emu(),
        (second.right - second.left).emu(),
        "two clustered series draw bars of the same width"
    );
    // The second category's bars are entirely to the right of the first category's.
    let first_next = chart.bar(0, 1).expect("series 0, point 1");
    assert!(second.right <= first_next.left);
}

#[test]
fn a_stack_meets_exactly_and_reaches_the_total() {
    let part = support::bar_chart(
        "stacked",
        &[
            support::series(0, "A", &["X"], &support::values(&[30.0])),
            support::series(1, "B", &["X"], &support::values(&[20.0])),
        ],
        "",
    );
    let chart = geometry(&part);
    let lower = chart.bar(0, 0).expect("the lower segment");
    let upper = chart.bar(1, 0).expect("the upper segment");
    assert_eq!(
        lower.top, upper.bottom,
        "a stack's segments must meet exactly, with no seam and no overlap"
    );
    assert_eq!(lower.left, upper.left, "a stack is one column, not two");
    assert_eq!(lower.right, upper.right);
    // Thirty and twenty: the lower segment is half again the upper.
    let ratio = (lower.bottom - lower.top).emu() as f64 / (upper.bottom - upper.top).emu() as f64;
    assert!((ratio - 1.5).abs() < 0.02, "the ratio was {ratio}");
}

#[test]
fn a_hundred_percent_stack_fills_the_axis_whatever_the_numbers_are() {
    let part = support::bar_chart(
        "percentStacked",
        &[
            support::series(0, "A", &["X"], &support::values(&[3.0])),
            support::series(1, "B", &["X"], &support::values(&[1.0])),
        ],
        "",
    );
    let chart = geometry(&part);
    let lower = chart.bar(0, 0).expect("the lower segment");
    let upper = chart.bar(1, 0).expect("the upper segment");
    assert_eq!(lower.top, upper.bottom);
    // Three quarters and one quarter of the plot's height, whatever three and one are.
    let total = (lower.bottom - upper.top).emu() as f64;
    let lower_share = (lower.bottom - lower.top).emu() as f64 / total;
    assert!(
        (lower_share - 0.75).abs() < 0.01,
        "three of four should fill three quarters; it filled {lower_share}"
    );
    assert_eq!(
        chart.value_ticks(),
        vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0],
        "a hundred-percent-stacked plot's axis is a proportion whatever the data is"
    );
}

#[test]
fn a_scatter_point_lands_where_both_of_its_values_say() {
    let part = support::scatter_chart(&[support::scatter_series(
        0,
        "Trial",
        &[1.0, 5.0, 9.0],
        &[10.0, 50.0, 90.0],
    )]);
    let chart = geometry(&part);
    let first = chart.marker(0, 0).expect("point 0");
    let middle = chart.marker(0, 1).expect("point 1");
    let last = chart.marker(0, 2).expect("point 2");

    assert!(
        first.x < middle.x && middle.x < last.x,
        "x increases with x"
    );
    assert!(
        first.y > middle.y && middle.y > last.y,
        "y decreases as the value rises"
    );
    // The three points are collinear, because the data is.
    let run = (last.x - first.x).emu() as f64;
    let rise = (last.y - first.y).emu() as f64;
    let expected_y = first.y.emu() as f64 + rise * ((middle.x - first.x).emu() as f64 / run);
    assert!(
        (middle.y.emu() as f64 - expected_y).abs() < 5000.0,
        "the middle point of a straight line is off it by {} EMU",
        (middle.y.emu() as f64 - expected_y).abs()
    );
}

#[test]
fn a_pie_sweeps_a_whole_turn_in_proportion() {
    let part = support::pie_chart(&support::series(
        0,
        "Share",
        &["A", "B", "C", "D"],
        &support::values(&[50.0, 25.0, 15.0, 10.0]),
    ));
    let chart = geometry(&part);
    let series = chart.series(0).expect("the pie's one series");
    let sweeps: Vec<f64> = series
        .points
        .iter()
        .map(|point| match &point.mark {
            Mark::Slice(slice) => slice.sweep.degrees(),
            other => panic!("a pie point must be a slice, not {other:?}"),
        })
        .collect();
    assert!(
        (sweeps[0] - 180.0).abs() < 1e-9,
        "half the total is half the circle"
    );
    assert!((sweeps[1] - 90.0).abs() < 1e-9);
    assert!((sweeps[2] - 54.0).abs() < 1e-9);
    assert!((sweeps[3] - 36.0).abs() < 1e-9);
    let total: f64 = sweeps.iter().sum();
    assert!(
        (total - 360.0).abs() < 1e-9,
        "the slices summed to {total} degrees"
    );

    // Each slice starts where the previous one ended, beginning at twelve o'clock.
    let starts: Vec<f64> = series
        .points
        .iter()
        .map(|point| match &point.mark {
            Mark::Slice(slice) => slice.start.degrees(),
            _ => unreachable!("checked above"),
        })
        .collect();
    assert!(starts[0].abs() < 1e-9);
    for index in 1..starts.len() {
        assert!((starts[index] - (starts[index - 1] + sweeps[index - 1])).abs() < 1e-9);
    }
}

#[test]
fn a_line_joins_its_points_and_the_polyline_is_the_markers() {
    let part = support::line_chart(&[support::series(
        0,
        "Trend",
        &["A", "B", "C"],
        &support::values(&[5.0, 9.0, 3.0]),
    )]);
    let chart = geometry(&part);
    let series = chart.series(0).expect("the line's series");
    let connector = series.connector.as_ref().expect("a line plot draws a line");
    assert_eq!(connector.points.len(), 3);
    assert!(!connector.closed, "a line plot's polyline is open");
    for (index, vertex) in connector.points.iter().enumerate() {
        assert_eq!(
            Some(*vertex),
            chart.marker(0, index),
            "the polyline's vertex {index} must be exactly the point's marker centre — if these \
             ever differ, the line and the markers are two different readings of the data"
        );
    }
}

#[test]
fn a_bar_chart_runs_across_and_a_column_chart_runs_up() {
    // The same numbers and the same categories; `c:barDir` and the two axis positions are the only
    // difference, which is exactly what Excel writes when a column chart is changed to a bar chart.
    let series = support::series(0, "A", &["X", "Y"], &support::values(&[10.0, 20.0]));
    let column = support::bar_chart("clustered", std::slice::from_ref(&series), "");
    let bar = support::horizontal_bar_chart(std::slice::from_ref(&series));

    let columns = geometry(&column);
    let bars = geometry(&bar);
    let first_column = columns.bar(0, 0).expect("column 0");
    let second_column = columns.bar(0, 1).expect("column 1");
    let first_bar = bars.bar(0, 0).expect("bar 0");
    let second_bar = bars.bar(0, 1).expect("bar 1");

    assert!(
        first_column.right <= second_column.left,
        "columns run across the plot"
    );
    assert!(
        first_bar.bottom <= second_bar.top,
        "bars run down the plot, one category under the next"
    );
    // A bar's *length* is horizontal, and the 20 bar is twice the 10 bar.
    let short = (first_bar.right - first_bar.left).emu() as f64;
    let long = (second_bar.right - second_bar.left).emu() as f64;
    assert!(
        (long / short - 2.0).abs() < 0.02,
        "a bar's length must follow its value; the ratio was {}",
        long / short
    );
}

#[test]
fn a_gap_in_the_data_is_a_hole_and_not_a_shift() {
    // The `c:idx` run skips 1, which is what a blank cell writes. Point 2 must still be the third
    // category, not the second.
    let part = support::bar_chart(
        "clustered",
        &[support::series(
            0,
            "Sparse",
            &["A", "B", "C"],
            &[Some(10.0), None, Some(30.0)],
        )],
        "",
    );
    let chart = geometry(&part);
    let series = chart.series(0).expect("the series");
    assert_eq!(series.points.len(), 3, "a blank still occupies its index");
    assert_eq!(series.points[1].mark, Mark::Absent);
    assert_eq!(series.points[1].value, None);

    let first = chart.bar(0, 0).expect("point 0 draws");
    let third = chart.bar(0, 2).expect("point 2 draws");
    // The third bar is in the third of three bands, not the second.
    let band = (chart.plot_area.right - chart.plot_area.left).emu() as f64 / 3.0;
    let centre = (third.left + third.right).emu() as f64 / 2.0;
    let expected = chart.plot_area.left.emu() as f64 + band * 2.5;
    assert!(
        (centre - expected).abs() < band * 0.2,
        "the third point drew at {centre}, but its band's centre is {expected} — a blank has \
         shifted the later points"
    );
    assert!(first.left < third.left);
}
