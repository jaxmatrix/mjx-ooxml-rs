//! Every piece of a chart that is not a mark: the title, the legend, the gridlines, the tick labels,
//! the axis titles, the data labels, the trendlines and the error bars.
//!
//! # The reachability instrument
//!
//! MJXOFF-176 wrote a function that nothing called, and an inline picture consequently changed
//! neither its line's width nor its height. The check that would have caught it is *would an
//! `abort()` at this site fire in any test?* — so every branch this crate added has a test here that
//! reaches it. The suite is arranged that way on purpose: one fixture carrying all of the furniture
//! at once, and one assertion per piece, so a piece that stops being emitted fails by name.

mod support;

use mjx_layout_chart::{lay_out, ChartModel, ChartPalette, NominalMetrics};

fn decorated() -> mjx_layout_chart::ChartGeometry {
    let part = support::decorated_bar_chart();
    let model = ChartModel::read(&part).expect("the fixture is readable");
    lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    )
}

#[test]
fn the_title_is_placed_above_the_plot_and_takes_room_from_it() {
    let chart = decorated();
    let title = chart.title.as_ref().expect("the chart states a title");
    assert_eq!(title.text, "Quarterly revenue");
    assert!(
        title.rect.bottom <= chart.plot_area.top,
        "the title sits above the plot"
    );
    assert!(title.rect.top >= chart.frame.top);
    assert!(
        title.rect.right > title.rect.left,
        "a title measured to nothing has not been measured"
    );

    // The same chart without a title has a taller plot area — which is what proves the title's
    // reservation is applied rather than computed and dropped.
    let untitled = String::from_utf8(support::decorated_bar_chart())
        .expect("UTF-8")
        .replace("Quarterly revenue", "");
    let model = ChartModel::read(untitled.as_bytes()).expect("readable");
    let without = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    assert!(
        without.plot_area.top < chart.plot_area.top,
        "removing the title must give the plot area its room back"
    );
}

#[test]
fn the_legend_lists_every_series_and_takes_its_own_side() {
    let chart = decorated();
    let legend = chart.legend.as_ref().expect("the chart states a legend");
    assert_eq!(legend.entries.len(), 1);
    let entry = &legend.entries[0];
    assert_eq!(entry.label.text, "Revenue");
    assert_eq!(entry.series, 0);
    assert!(
        entry.swatch.right <= entry.label.rect.left,
        "the swatch sits before the name"
    );
    assert!(!entry.paint.is_empty(), "the swatch is painted");
    assert!(
        legend.rect.left >= chart.plot_area.right,
        "a right-hand legend sits beside the plot, not on it"
    );
}

#[test]
fn both_gridline_kinds_are_ruled_across_the_plot() {
    let chart = decorated();
    let major: Vec<_> = chart.gridlines.iter().filter(|line| !line.minor).collect();
    let minor: Vec<_> = chart.gridlines.iter().filter(|line| line.minor).collect();
    assert!(!major.is_empty(), "`c:majorGridlines` must rule lines");
    assert!(!minor.is_empty(), "`c:minorGridlines` must rule lines");
    assert!(
        minor.len() > major.len(),
        "there are more minor gridlines than major ones: {} against {}",
        minor.len(),
        major.len()
    );
    for line in chart.gridlines.iter() {
        assert_eq!(
            line.from.y, line.to.y,
            "a value axis rules horizontal lines"
        );
        assert_eq!(line.from.x, chart.plot_area.left);
        assert_eq!(line.to.x, chart.plot_area.right);
    }
}

#[test]
fn the_value_axis_labels_its_ticks_and_the_category_axis_skips_every_other_one() {
    let chart = decorated();
    let value = chart
        .axis(mjx_chart::AxisKind::Value)
        .expect("a value axis");
    assert!(value.ticks.len() >= 3);
    for tick in &value.ticks {
        let label = tick.label.as_ref().expect("every value tick is labelled");
        assert!(!label.text.is_empty());
        assert!(
            label.rect.right <= chart.plot_area.left,
            "a left-hand axis' labels sit outside the plot"
        );
        assert!(tick.mark.is_some(), "`majorTickMark=out` draws a mark");
    }

    let category = chart
        .axis(mjx_chart::AxisKind::Category)
        .expect("a category axis");
    assert_eq!(category.ticks.len(), 4);
    let labelled: Vec<bool> = category
        .ticks
        .iter()
        .map(|tick| tick.label.is_some())
        .collect();
    assert_eq!(
        labelled,
        vec![true, false, true, false],
        "`c:tickLblSkip val=\"2\"` draws every other label"
    );
    assert_eq!(
        category.ticks[0]
            .label
            .as_ref()
            .map(|label| label.text.as_str()),
        Some("Q1")
    );
}

#[test]
fn each_axis_title_is_placed_on_its_own_side() {
    let chart = decorated();
    let value = chart.axis(mjx_chart::AxisKind::Value).expect("value axis");
    let category = chart
        .axis(mjx_chart::AxisKind::Category)
        .expect("category axis");
    let value_title = value.title.as_ref().expect("`Pounds`");
    let category_title = category.title.as_ref().expect("`Quarter`");
    assert_eq!(value_title.text, "Pounds");
    assert_eq!(category_title.text, "Quarter");
    assert!(
        value_title.rotation.degrees().abs() > 1.0,
        "a vertical axis' title is turned; it was at {} degrees",
        value_title.rotation.degrees()
    );
    assert_eq!(category_title.rotation.degrees(), 0.0);
    assert!(category_title.rect.top >= chart.plot_area.bottom);
}

#[test]
fn a_data_label_says_the_value_and_sits_by_its_mark() {
    let chart = decorated();
    let series = chart.series(0).expect("the series");
    let labels: Vec<&str> = series
        .points
        .iter()
        .filter_map(|point| point.label.as_ref().map(|label| label.text.as_str()))
        .collect();
    assert_eq!(
        labels,
        vec!["10", "20", "30", "40"],
        "`c:showVal` writes the value, formatted by the axis' own step"
    );
    for point in &series.points {
        let label = point.label.as_ref().expect("every point is labelled");
        let bar = match &point.mark {
            mjx_layout_chart::Mark::Bar(rect) => *rect,
            other => panic!("expected a bar, got {other:?}"),
        };
        assert!(
            label.rect.bottom <= bar.top,
            "a column's label sits above its bar"
        );
    }
}

#[test]
fn a_trendline_is_fitted_and_sampled() {
    let chart = decorated();
    let series = chart.series(0).expect("the series");
    assert_eq!(
        series.trendlines.len(),
        1,
        "the fixture states one trendline"
    );
    let line = &series.trendlines[0];
    assert!(
        line.points.len() > 2,
        "a fitted curve is sampled, not a segment"
    );
    // The data is 10, 20, 30, 40 — a perfect straight line — so the fit runs monotonically down the
    // page from left to right, and its ends are the extremes.
    assert!(line.points.first().unwrap().x < line.points.last().unwrap().x);
    assert!(
        line.points.first().unwrap().y > line.points.last().unwrap().y,
        "a rising series fits a line that rises up the page"
    );
    for window in line.points.windows(2) {
        assert!(
            window[0].y >= window[1].y,
            "the fit through a straight line is monotone"
        );
    }
}

#[test]
fn error_bars_reach_above_and_below_each_point() {
    let chart = decorated();
    let series = chart.series(0).expect("the series");
    assert_eq!(series.error_bars.len(), 4, "one bar per point");
    for bar in &series.error_bars {
        assert!(bar.capped, "`c:noEndCap val=\"0\"` keeps the caps");
        assert!(
            bar.from.y < bar.to.y,
            "the bar runs from its high to its low"
        );
        let mark = chart.bar(0, bar.point).expect("the point's own bar");
        assert!(
            bar.from.y <= mark.top && bar.to.y > mark.top,
            "a ten-percent bar straddles the top of its column"
        );
    }
    // The upper half is *clamped* at the axis' own maximum rather than extending the axis, which is
    // what Office does: the point at 40 sits on a scale that ends at 40, so its bar has nowhere
    // above to go. Every point below the maximum straddles strictly.
    let inner = &series.error_bars[0];
    let first = chart.bar(0, 0).expect("the first column");
    assert!(
        inner.from.y < first.top,
        "a point away from the axis' maximum must have a bar reaching above it"
    );
}

#[test]
fn a_legend_on_the_bottom_takes_height_and_one_on_the_right_takes_width() {
    let series = support::series(0, "One", &["A"], &support::values(&[1.0]));
    let right = support::titled_bar_chart("T", "r", std::slice::from_ref(&series));
    let bottom = support::titled_bar_chart("T", "b", std::slice::from_ref(&series));
    let lay = |part: &[u8]| {
        let model = ChartModel::read(part).expect("readable");
        lay_out(
            &model,
            support::frame(),
            &ChartPalette::OFFICE,
            &mut NominalMetrics,
        )
    };
    let right = lay(&right);
    let bottom = lay(&bottom);
    assert!(
        right.plot_area.right < bottom.plot_area.right,
        "a right legend narrows the plot"
    );
    assert!(
        bottom.plot_area.bottom < right.plot_area.bottom,
        "a bottom legend shortens it"
    );
}
