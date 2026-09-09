//! ⚠ Colour scales, data bars and icon sets are **interpolations**, so the endpoints are not the
//! test.
//!
//! A scale that returned its first colour for every cell passes an endpoint-only fixture perfectly:
//! the minimum is the first colour, and that is the only cell the fixture looked at. Every case here
//! therefore asserts a **midpoint** as well — a value that is neither stop — and the numbers are
//! stated rather than described:
//!
//! * a data bar's width as a **fraction of the cell**, for a known value against a known range;
//! * a colour scale's two stops **and how far between them** the value sits;
//! * an icon's **index**, and the same index again with `@reverse` flipping it.
//!
//! # Provenance
//!
//! * **SpecCode** — `@minLength` 10 and `@maxLength` 90 are the schema's own defaults, and
//!   §18.3.1.28 calls both *"a percentage of the cell width"*. The `cfvo` kinds and what each means
//!   are §18.3.1.11's.
//! * **DocumentedBehaviour** — `PERCENTILE.INC`'s interpolation, which is what a `percentile` `cfvo`
//!   refers to and which is why `percent` and `percentile` give *different* answers on skewed data.
//!   `@gte`'s default of `true` is the schema's, and §18.3.1.11 states what `0` does.
//! * **EngineDerived** — the arithmetic joining them: that the bar's fraction is
//!   `min + position * (max - min)`, that a value below the shortest threshold clamps rather than
//!   inverting, and that a three-stop scale sorts its resolved thresholds. Change detectors, **not
//!   evidence about Excel**.

mod support;

use mjx_layout::{Fragment, FragmentTree, LayoutRect};
use mjx_layout_xlsx::{CellReport, PageCatalogue, SheetBoxModel, SheetGrid};
use support::{column_with_rules, styles_with_differentials, Value};

/// Lays out one band and hands back the tree and the catalogue.
fn lay_out(values: &[Value], rules: &str) -> (FragmentTree, PageCatalogue) {
    let styles = styles_with_differentials(&[], &[]);
    let body = column_with_rules(values, rules);
    let book = support::workbook(&support::worksheet(&body), &styles);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(8.0, 6.0);
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    (tree, model.catalogue().clone())
}

/// What one row reported.
fn report(catalogue: &PageCatalogue, row: u32) -> CellReport {
    catalogue
        .cell(row, 0)
        .cloned()
        .unwrap_or_else(|| panic!("row {row} produced no report"))
}

/// The bar fragment's rectangle for a cell, and the cell's own — the two a fraction relates.
fn bar_and_cell(
    tree: &FragmentTree,
    catalogue: &PageCatalogue,
    row: u32,
) -> (LayoutRect, LayoutRect) {
    let mut cell = None;
    let mut bar = None;
    for (_, node) in tree.nodes() {
        let Fragment::Box(box_fragment) = node.fragment() else {
            continue;
        };
        if node.source().path().segments() != [row, 0] {
            continue;
        }
        match box_fragment.cell {
            Some(_) => cell = Some(node.rect()),
            None => {
                let carries_a_bar = box_fragment
                    .decoration
                    .and_then(|handle| catalogue.decoration(handle))
                    .is_some_and(|decoration| decoration.data_bar.is_some());
                if carries_a_bar {
                    bar = Some(node.rect());
                }
            }
        }
    }
    (
        bar.unwrap_or_else(|| panic!("row {row} drew no data bar")),
        cell.unwrap_or_else(|| panic!("row {row} drew no cell box")),
    )
}

/// Values 0, 25, 50, 75 and 100, so every case has two endpoints and three midpoints.
const SPREAD: [Value; 5] = [
    Value::Number(0.0),
    Value::Number(25.0),
    Value::Number(50.0),
    Value::Number(75.0),
    Value::Number(100.0),
];

// -------------------------------------------------------------------------------------------
// Data bars
// -------------------------------------------------------------------------------------------

/// A bar over the range's own minimum and maximum, at the schema's default lengths.
const BAR: &str = r#"<cfRule type="dataBar" priority="1"><dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule>"#;

#[test]
fn a_data_bars_width_is_a_stated_fraction_of_the_cell_at_both_ends_and_in_the_middle() {
    let (_, catalogue) = lay_out(&SPREAD, BAR);
    // `@minLength` 10 and `@maxLength` 90, so the shortest bar is a tenth of the cell and the
    // longest nine tenths — **not** zero and one. A bar implemented as the bare position would give
    // 0.0 and 1.0 here and pass any assertion phrased as "a bar appears".
    let expected = [0.10, 0.30, 0.50, 0.70, 0.90];
    for (row, want) in expected.into_iter().enumerate() {
        let row = u32::try_from(row).unwrap_or(0);
        let bar = report(&catalogue, row)
            .conditional
            .and_then(|effect| effect.bar)
            .unwrap_or_else(|| panic!("row {row} sized no bar"));
        assert!(
            (bar.fraction - want).abs() < 1e-9,
            "row {row}: fraction {} should be {want}",
            bar.fraction
        );
    }
}

#[test]
fn the_bar_that_reaches_the_tree_is_the_fraction_of_the_cell_it_claims_to_be() {
    let (tree, catalogue) = lay_out(&SPREAD, BAR);
    for row in 0..5_u32 {
        let (bar, cell) = bar_and_cell(&tree, &catalogue, row);
        let fraction = report(&catalogue, row)
            .conditional
            .and_then(|effect| effect.bar)
            .expect("a bar")
            .fraction;
        #[allow(clippy::cast_precision_loss)]
        let want = (cell.width().emu() as f64 * fraction).round() as i64;
        assert_eq!(
            bar.width().emu(),
            want,
            "row {row}: the drawn bar and the reported fraction disagree"
        );
        assert_eq!(bar.left, cell.left, "row {row}: the bar starts at the left");
        assert!(
            bar.right < cell.right,
            "row {row}: and a 90% bar does not fill the cell"
        );
    }
}

#[test]
fn min_and_max_lengths_move_both_ends_of_the_bar() {
    // The same data with the lengths stated: the shortest bar is now a quarter of the cell and the
    // longest half of it. An implementation that ignored the two attributes would answer 0.0 and
    // 1.0 and would have passed the case above only because 10 and 90 are the defaults.
    let stated = r#"<cfRule type="dataBar" priority="1"><dataBar minLength="25" maxLength="50"><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule>"#;
    let (_, catalogue) = lay_out(&SPREAD, stated);
    let fraction = |row: u32| {
        report(&catalogue, row)
            .conditional
            .and_then(|effect| effect.bar)
            .expect("a bar")
            .fraction
    };
    assert!((fraction(0) - 0.25).abs() < 1e-9);
    assert!((fraction(2) - 0.375).abs() < 1e-9, "halfway between them");
    assert!((fraction(4) - 0.50).abs() < 1e-9);
}

#[test]
fn a_value_below_the_shortest_threshold_clamps_rather_than_inverting() {
    // Thresholds stated as numbers rather than as the range's own ends, so there are values outside
    // them. `interpolate` clamps, so the bar stops at `@minLength` instead of going negative.
    let bounded = r#"<cfRule type="dataBar" priority="1"><dataBar><cfvo type="num" val="40"/><cfvo type="num" val="60"/><color rgb="FF638EC6"/></dataBar></cfRule>"#;
    let (_, catalogue) = lay_out(&SPREAD, bounded);
    let fraction = |row: u32| {
        report(&catalogue, row)
            .conditional
            .and_then(|effect| effect.bar)
            .expect("a bar")
            .fraction
    };
    assert!((fraction(0) - 0.10).abs() < 1e-9, "0 is below 40");
    assert!((fraction(2) - 0.50).abs() < 1e-9, "50 is halfway");
    assert!((fraction(4) - 0.90).abs() < 1e-9, "100 is above 60");
}

// -------------------------------------------------------------------------------------------
// Colour scales
// -------------------------------------------------------------------------------------------

/// White at the minimum, yellow halfway, red at the maximum — Excel's own three-colour default.
const THREE_COLOUR: &str = r#"<cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="percent" val="50"/><cfvo type="max"/><color rgb="FFF8696B"/><color rgb="FFFFEB84"/><color rgb="FF63BE7B"/></colorScale></cfRule>"#;

/// The `@rgb` of one stop, as the file spells it.
fn hex(colour: &mjx_sml::Color) -> String {
    colour.rgb.clone().unwrap_or_default()
}

#[test]
fn a_three_stop_scale_names_the_pair_a_midpoint_falls_between_and_how_far_along_it_is() {
    let (_, catalogue) = lay_out(&SPREAD, THREE_COLOUR);
    let blend = |row: u32| {
        report(&catalogue, row)
            .conditional
            .and_then(|effect| effect.scale)
            .unwrap_or_else(|| panic!("row {row} took no scale colour"))
    };

    // The two endpoints, which an implementation that always answered its first stop would also
    // pass. They are here so the midpoints below have something to be different from.
    let low = blend(0);
    assert_eq!(hex(&low.low), "FFF8696B");
    assert!((low.fraction - 0.0).abs() < 1e-9);

    let high = blend(4);
    assert_eq!(hex(&high.low), "FFFFEB84");
    assert_eq!(hex(&high.high), "FF63BE7B");
    assert!((high.fraction - 1.0).abs() < 1e-9);

    // ⚠ The three midpoints. 25 is halfway up the *first* segment and 75 halfway up the *second*,
    // so a scale that returned its first colour, or one that ignored the middle stop, is red here.
    let quarter = blend(1);
    assert_eq!(hex(&quarter.low), "FFF8696B", "25 is in the first segment");
    assert_eq!(hex(&quarter.high), "FFFFEB84");
    assert!(
        (quarter.fraction - 0.5).abs() < 1e-9,
        "and halfway along it"
    );

    let middle = blend(2);
    assert_eq!(
        hex(&middle.high),
        "FFFFEB84",
        "50 is the middle stop itself"
    );
    assert!((middle.fraction - 1.0).abs() < 1e-9);

    let three_quarters = blend(3);
    assert_eq!(
        hex(&three_quarters.low),
        "FFFFEB84",
        "75 is in the second segment"
    );
    assert_eq!(hex(&three_quarters.high), "FF63BE7B");
    assert!((three_quarters.fraction - 0.5).abs() < 1e-9);
}

#[test]
fn percent_and_percentile_are_different_questions_and_answer_differently() {
    // ⚠ The case that tells a `cfvo` reader from a placeholder. On a **skewed** range the halfway
    // *value* and the halfway *rank* are far apart: 1, 2, 3, 100 has a midpoint of 50.5 by
    // `percent` and 2.5 by `percentile`. An implementation that treated the two kinds alike is red
    // on one of the two blocks below, and green on any evenly-spaced fixture.
    let skewed = [
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0),
        Value::Number(100.0),
    ];
    let scale = |kind: &str| {
        format!(
            r#"<cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="{kind}" val="50"/><cfvo type="max"/><color rgb="FF000000"/><color rgb="FF808080"/><color rgb="FFFFFFFF"/></colorScale></cfRule>"#
        )
    };

    // By `percent` the middle stop is at 50.5, so the value 3 is still in the *first* segment.
    let (_, by_percent) = lay_out(&skewed, &scale("percent"));
    let three = report(&by_percent, 2)
        .conditional
        .and_then(|effect| effect.scale)
        .expect("a scale colour");
    assert_eq!(hex(&three.low), "FF000000");
    assert_eq!(hex(&three.high), "FF808080");

    // By `percentile` it is at 2.5, so the same value 3 is in the *second*.
    let (_, by_percentile) = lay_out(&skewed, &scale("percentile"));
    let three = report(&by_percentile, 2)
        .conditional
        .and_then(|effect| effect.scale)
        .expect("a scale colour");
    assert_eq!(hex(&three.low), "FF808080");
    assert_eq!(hex(&three.high), "FFFFFFFF");
}

// -------------------------------------------------------------------------------------------
// Icon sets
// -------------------------------------------------------------------------------------------

/// Three icons with bands at 33 and 67 per cent — Excel's own default for a three-icon set.
const ICONS: &str = r#"<cfRule type="iconSet" priority="1"><iconSet><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule>"#;

#[test]
fn an_icon_sets_index_moves_through_its_bands_and_is_not_always_the_first() {
    let (_, catalogue) = lay_out(&SPREAD, ICONS);
    let index = |row: u32| {
        report(&catalogue, row)
            .icon
            .unwrap_or_else(|| panic!("row {row} chose no icon"))
            .index
    };
    assert_eq!(index(0), 0, "0 is below the 33 per cent band");
    assert_eq!(index(1), 0, "25 still is");
    assert_eq!(index(2), 1, "50 is in the middle band");
    assert_eq!(index(3), 2, "75 is in the top band");
    assert_eq!(index(4), 2, "and so is 100");
}

#[test]
fn reverse_flips_the_index_and_nothing_else() {
    let reversed = ICONS.replace("<iconSet>", r#"<iconSet reverse="1">"#);
    let (_, catalogue) = lay_out(&SPREAD, &reversed);
    let icon = |row: u32| report(&catalogue, row).icon.expect("an icon");
    assert_eq!(icon(0).index, 2, "the bottom band takes the last icon");
    assert_eq!(icon(2).index, 1, "the middle one is its own mirror");
    assert_eq!(icon(4).index, 0);
    assert_eq!(icon(0).count, 3, "and the set is still three icons wide");
}

#[test]
fn gte_zero_moves_a_value_that_sits_exactly_on_a_boundary() {
    // 50 resolves to exactly the second threshold. With `@gte` at its schema default of `true` the
    // band includes it; with `@gte="0"` it does not, and the value falls back a band. That single
    // attribute is the whole difference, and a reader that ignored it would answer the same twice.
    let inclusive = r#"<cfRule type="iconSet" priority="1"><iconSet><cfvo type="num" val="0"/><cfvo type="num" val="50"/><cfvo type="num" val="100"/></iconSet></cfRule>"#;
    let exclusive = r#"<cfRule type="iconSet" priority="1"><iconSet><cfvo type="num" val="0"/><cfvo type="num" val="50" gte="0"/><cfvo type="num" val="100"/></iconSet></cfRule>"#;
    let (_, with) = lay_out(&SPREAD, inclusive);
    let (_, without) = lay_out(&SPREAD, exclusive);
    assert_eq!(report(&with, 2).icon.expect("an icon").index, 1);
    assert_eq!(report(&without, 2).icon.expect("an icon").index, 0);
}

#[test]
fn a_graded_rule_whose_threshold_is_a_formula_is_recorded_as_partial() {
    // `<cfvo type="formula" val="AVERAGE($A$1:$A$5)"/>` cannot be resolved without a calculation
    // engine, so the bar is **not drawn at a guessed length**: it is reported unevaluated, with the
    // reason. A bar at the wrong length is a claim about the data.
    let unresolvable = r#"<cfRule type="dataBar" priority="1"><dataBar><cfvo type="formula" val="AVERAGE($A$1:$A$5)"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule>"#;
    let (_, catalogue) = lay_out(&SPREAD, unresolvable);
    let effect = report(&catalogue, 2)
        .conditional
        .expect("a rule reached it");
    assert!(effect.bar.is_none(), "no bar was drawn");
    assert_eq!(effect.unevaluated.len(), 1);
    assert!(effect.fired.is_empty());
}
