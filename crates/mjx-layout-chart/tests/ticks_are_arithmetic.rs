//! Tick selection, asserted as numbers on ranges a naive implementation cannot survive.
//!
//! # Why the ranges are awkward
//!
//! A chart running 0 to 100 in steps of 10 looks right under **any** implementation, including one
//! that divided the data range into five equal parts and printed the results. Such a test passes on
//! an engine with no nice-number selection at all, which is the trap MJXOFF-178 was written against.
//!
//! So the two ranges here are the ones the ticket names — `0.003 … 0.017` and `−45 … 1203` — and
//! every expected tick is a literal. [`a_naive_linear_division_would_fail_these`] states what the
//! naive answer would have been and asserts it is *not* what comes out, so the discrimination is
//! part of the suite rather than a claim in a comment.

mod support;

use mjx_layout_chart::{
    lay_out, nice_number, Baseline, ChartModel, ChartPalette, NominalMetrics, Rounding, Scale,
    StatedScale, Step,
};

/// Every tick of `scale`, so an expectation is one comparison.
fn ticks(scale: &Scale) -> Vec<f64> {
    scale.ticks().collect()
}

#[test]
fn a_thousandths_range_gets_thousandths_ticks() {
    // 0.003 … 0.017 over six intervals. The span is 0.014; a sixth of it is 0.00233…; the nearest
    // nice number is 0.002; the data is non-negative and sits near zero, so the axis is zero-based
    // and runs to the first multiple of 0.002 at or above 0.017.
    let scale = Scale::automatic(0.003, 0.017, 6, Baseline::Anchored);
    assert_eq!(scale.minimum, 0.0);
    assert_eq!(scale.maximum, 0.018);
    assert_eq!(scale.major.value(), 0.002);
    assert_eq!(
        ticks(&scale),
        vec![0.0, 0.002, 0.004, 0.006, 0.008, 0.010, 0.012, 0.014, 0.016, 0.018],
        "the ticks must be the doubles a reader writes down, not accumulated ones"
    );
}

#[test]
fn a_range_that_crosses_zero_extends_both_ways() {
    // −45 … 1203 over six intervals. The span is 1248; a sixth is 208; the nearest nice number is
    // 200; the data crosses zero so neither end is anchored, and both are snapped outward.
    let scale = Scale::automatic(-45.0, 1203.0, 6, Baseline::Anchored);
    assert_eq!(scale.minimum, -200.0);
    assert_eq!(scale.maximum, 1400.0);
    assert_eq!(scale.major.value(), 200.0);
    assert_eq!(
        ticks(&scale),
        vec![-200.0, 0.0, 200.0, 400.0, 600.0, 800.0, 1000.0, 1200.0, 1400.0]
    );
}

#[test]
fn a_naive_linear_division_would_fail_these() {
    // What a five-way linear division of the data range gives, which is what an engine with no
    // nice-number step would print on its axis.
    let naive_low = 0.003;
    let naive_step = (0.017 - 0.003) / 5.0;
    let naive: Vec<f64> = (0..=5)
        .map(|n| naive_low + naive_step * f64::from(n))
        .collect();

    let scale = Scale::automatic(0.003, 0.017, 6, Baseline::Anchored);
    assert_ne!(
        ticks(&scale),
        naive,
        "a nice-number axis is a specific algorithm, not a look — if these ever agree, the step \
         selection has been replaced by a division"
    );
    assert!(
        naive.iter().any(|value| (value - 0.0058).abs() < 1e-9),
        "the naive division really does produce 0.0058, which is the label this engine must not draw"
    );
}

#[test]
fn the_mantissa_set_is_one_two_five_ten() {
    // Heckbert's `nicenum`, both roundings. `Up` never returns a smaller number than its argument;
    // `Nearest` may.
    assert_eq!(nice_number(1.0, Rounding::Up).value(), 1.0);
    assert_eq!(nice_number(1.4, Rounding::Up).value(), 2.0);
    assert_eq!(nice_number(2.1, Rounding::Up).value(), 5.0);
    assert_eq!(nice_number(5.1, Rounding::Up).value(), 10.0);
    assert_eq!(nice_number(1.4, Rounding::Nearest).value(), 1.0);
    assert_eq!(nice_number(2.9, Rounding::Nearest).value(), 2.0);
    assert_eq!(nice_number(6.9, Rounding::Nearest).value(), 5.0);
    assert_eq!(nice_number(7.1, Rounding::Nearest).value(), 10.0);
    // A step of 3 or 4 is never chosen, which is what "nice" means.
    for rough in [3.0_f64, 3.5, 4.0, 4.4] {
        let chosen = nice_number(rough, Rounding::Nearest).value();
        assert!(
            (chosen - 5.0).abs() < f64::EPSILON || (chosen - 2.0).abs() < f64::EPSILON,
            "{rough} chose {chosen}, which is not in {{1, 2, 5, 10}}"
        );
    }
}

#[test]
fn a_bar_chart_is_zero_based_and_a_line_chart_may_float() {
    // 480 … 520: a bar chart drawn against 480 would make the 520 bar look many times the 490 one.
    let anchored = Scale::automatic(480.0, 520.0, 6, Baseline::Anchored);
    assert_eq!(anchored.minimum, 0.0);
    let floating = Scale::automatic(480.0, 520.0, 6, Baseline::Floating);
    assert!(
        floating.minimum > 0.0,
        "a line chart's axis may float; it gave {}",
        floating.minimum
    );
    // …but a line chart whose data sits near zero still snaps to it.
    let near_zero = Scale::automatic(1.0, 20.0, 6, Baseline::Floating);
    assert_eq!(near_zero.minimum, 0.0);
}

#[test]
fn a_stated_bound_is_used_exactly() {
    // A reader who set the maximum to 97 meant 97, however ugly the axis is.
    let scale = Scale::resolved(
        0.0,
        90.0,
        6,
        Baseline::Anchored,
        StatedScale {
            minimum: Some(3.0),
            maximum: Some(97.0),
            major_unit: Some(7.0),
            ..StatedScale::default()
        },
    );
    assert_eq!(scale.minimum, 3.0);
    assert_eq!(scale.maximum, 97.0);
    assert_eq!(
        scale.major.value(),
        7.0,
        "a stated major unit is not rounded"
    );
    assert_eq!(
        ticks(&scale),
        vec![7.0, 14.0, 21.0, 28.0, 35.0, 42.0, 49.0, 56.0, 63.0, 70.0, 77.0, 84.0, 91.0]
    );
}

#[test]
fn a_hundred_percent_stacked_axis_is_always_a_proportion() {
    let scale = Scale::proportional();
    assert_eq!(scale.minimum, 0.0);
    assert_eq!(scale.maximum, 1.0);
    assert_eq!(ticks(&scale), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
}

#[test]
fn a_logarithmic_axis_ticks_at_whole_powers() {
    let scale = Scale::logarithmic(3.0, 4500.0, 10.0);
    assert_eq!(scale.minimum, 1.0);
    assert_eq!(scale.maximum, 10_000.0);
    assert_eq!(ticks(&scale), vec![1.0, 10.0, 100.0, 1000.0, 10_000.0]);
    // A non-positive value has no logarithm and is not placed at all.
    assert_eq!(scale.fraction(0.0), None);
    assert_eq!(scale.fraction(-1.0), None);
    let half = scale.fraction(100.0).expect("100 is on the axis");
    assert!((half - 0.5).abs() < 1e-12, "100 is halfway from 1 to 10000");
}

#[test]
fn the_step_carries_its_own_decimal_places() {
    assert_eq!(Step::new(2, -3).decimals(), 3);
    assert_eq!(Step::new(5, 2).decimals(), 0);
    // The ninth multiple of 0.002 is the double a reader writes as 0.018, not an accumulation of it.
    assert_eq!(Step::new(2, -3).scaled(9), 0.018);
    let mut accumulated = 0.0f64;
    for _ in 0..9 {
        accumulated += 0.002;
    }
    assert_ne!(
        accumulated, 0.018,
        "the accumulation really does drift, which is why the step is an integer mantissa"
    );
}

#[test]
fn the_tick_count_follows_the_plot_and_the_ticks_reach_the_chart() {
    // The whole pipeline: an awkward series through a real chart part, and the ticks off the
    // finished geometry. This is what proves the scale reaches the axis rather than being computed
    // and dropped.
    let part = support::bar_chart(
        "clustered",
        &[support::series(
            0,
            "Yield",
            &["A", "B", "C"],
            &support::values(&[0.003, 0.011, 0.017]),
        )],
        "",
    );
    let model = ChartModel::read(&part).expect("the fixture is a readable chart part");
    let geometry = lay_out(
        &model,
        support::frame(),
        &ChartPalette::OFFICE,
        &mut NominalMetrics,
    );
    let ticks = geometry.value_ticks();
    assert!(!ticks.is_empty(), "the value axis must carry ticks");
    assert_eq!(ticks.first().copied(), Some(0.0));
    assert!(
        ticks.last().copied().is_some_and(|last| last >= 0.017),
        "the axis must reach the data; it ended at {:?}",
        ticks.last()
    );
    for tick in &ticks {
        let scaled = tick * 1000.0;
        assert!(
            (scaled - scaled.round()).abs() < 1e-9,
            "{tick} is not a whole number of thousandths, so the step is not a nice number"
        );
    }
}
