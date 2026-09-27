//! A series with no stated fill takes the **document's** accent, and one with a stated fill keeps it.
//!
//! # The identity-value trap this suite is written against
//!
//! Every series in a fixture authored for convenience carries an explicit `c:spPr`, and a chart whose
//! series all state their own colours never exercises theme resolution at all. A suite built on such
//! fixtures passes on an engine that ignores the palette entirely — and the defect it misses is the
//! worst kind, because a hard-coded blue looks perfectly good in isolation and renders **off-palette
//! inside a customer's branded document**.
//!
//! So every fixture here is a series with **no `c:spPr`**, which is what Office writes, and the
//! palette handed in is not the default one: [`branded`] is six colours no constant in this crate
//! contains, so a chart drawn from `ChartPalette::OFFICE` instead would fail rather than pass.

mod support;

use mjx_dml::{ColorSpec, FillSpec};
use mjx_layout_chart::{lay_out, ChartModel, ChartPalette, NominalMetrics};

/// A palette no default in this crate carries, so an engine that ignored it would be caught.
fn branded() -> ChartPalette {
    ChartPalette::from_accents([
        [0x11, 0x22, 0x33],
        [0x44, 0x55, 0x66],
        [0x77, 0x88, 0x99],
        [0xAA, 0xBB, 0xCC],
        [0xDD, 0xEE, 0xFF],
        [0x01, 0x02, 0x03],
    ])
}

/// The hex of a solid fill, or `None` for anything else.
fn solid_hex(fill: Option<&FillSpec>) -> Option<String> {
    match fill? {
        FillSpec::Solid(ColorSpec::Srgb(hex)) => Some(hex.clone()),
        _ => None,
    }
}

#[test]
fn a_series_with_no_fill_takes_the_documents_accent() {
    let part = support::bar_chart(
        "clustered",
        &[
            support::series(0, "A", &["X"], &support::values(&[1.0])),
            support::series(1, "B", &["X"], &support::values(&[2.0])),
            support::series(2, "C", &["X"], &support::values(&[3.0])),
        ],
        "",
    );
    let model = ChartModel::read(&part).expect("readable");
    for (_, series) in model.all_series() {
        assert!(
            series.fill.is_none(),
            "the fixture must state no fill, or this suite proves nothing"
        );
    }

    let chart = lay_out(&model, support::frame(), &branded(), &mut NominalMetrics);
    assert_eq!(
        solid_hex(chart.series(0).unwrap().paint.fill.as_ref()).as_deref(),
        Some("112233"),
        "the first series takes accent1 from the document's own theme"
    );
    assert_eq!(
        solid_hex(chart.series(1).unwrap().paint.fill.as_ref()).as_deref(),
        Some("445566")
    );
    assert_eq!(
        solid_hex(chart.series(2).unwrap().paint.fill.as_ref()).as_deref(),
        Some("778899")
    );
}

#[test]
fn a_seventh_series_wraps_back_to_the_first_accent() {
    let series: Vec<String> = (0..7)
        .map(|index| {
            support::series(
                index,
                &format!("S{index}"),
                &["X"],
                &support::values(&[f64::from(index) + 1.0]),
            )
        })
        .collect();
    let part = support::bar_chart("clustered", &series, "");
    let model = ChartModel::read(&part).expect("readable");
    let chart = lay_out(&model, support::frame(), &branded(), &mut NominalMetrics);
    assert_eq!(
        solid_hex(chart.series(6).unwrap().paint.fill.as_ref()),
        solid_hex(chart.series(0).unwrap().paint.fill.as_ref()),
        "the seventh series draws in accent1 again"
    );
}

#[test]
fn a_stated_fill_is_never_overridden() {
    let part = support::bar_chart(
        "clustered",
        &[
            support::series_filled(0, "A", &["X"], &support::values(&[1.0]), "FF00FF"),
            support::series(1, "B", &["X"], &support::values(&[2.0])),
        ],
        "",
    );
    let model = ChartModel::read(&part).expect("readable");
    let chart = lay_out(&model, support::frame(), &branded(), &mut NominalMetrics);
    assert_eq!(
        solid_hex(chart.series(0).unwrap().paint.fill.as_ref()).as_deref(),
        Some("FF00FF"),
        "a series that states its own colour keeps it — the document wins over the palette too"
    );
    assert_eq!(
        solid_hex(chart.series(1).unwrap().paint.fill.as_ref()).as_deref(),
        Some("445566"),
        "and the series beside it still takes its own accent by its own index"
    );
}

#[test]
fn a_pie_gives_every_slice_its_own_colour() {
    // One series, four points. Without per-point colours a pie is one solid disc — which renders,
    // and is not a pie.
    let part = support::pie_chart(&support::series(
        0,
        "Share",
        &["A", "B", "C", "D"],
        &support::values(&[1.0, 2.0, 3.0, 4.0]),
    ));
    let model = ChartModel::read(&part).expect("readable");
    let chart = lay_out(&model, support::frame(), &branded(), &mut NominalMetrics);
    let series = chart.series(0).expect("the pie's series");
    let colours: Vec<Option<String>> = series
        .points
        .iter()
        .map(|point| solid_hex(point.paint.as_ref().and_then(|paint| paint.fill.as_ref())))
        .collect();
    assert_eq!(
        colours,
        vec![
            Some("112233".to_owned()),
            Some("445566".to_owned()),
            Some("778899".to_owned()),
            Some("AABBCC".to_owned()),
        ],
        "each slice takes the next accent of the document's own palette"
    );
}

#[test]
fn the_office_default_is_only_a_fallback() {
    // The two palettes differ, so a chart drawn with one is not the chart drawn with the other. That
    // is what makes the assertions above discriminating rather than tautological.
    assert_ne!(ChartPalette::OFFICE.accents, branded().accents);
}
