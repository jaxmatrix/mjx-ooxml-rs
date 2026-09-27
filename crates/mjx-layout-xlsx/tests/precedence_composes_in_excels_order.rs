//! ⚠ Precedence and `stopIfTrue` are **ordering semantics**, so the fixture has to be one where the
//! wrong order gives a different, plausible-looking answer.
//!
//! A sheet with one rule on it says nothing about ordering. A sheet with two rules that impose the
//! *same* format says nothing either — both orders paint the same cell the same colour. So every
//! case here has two rules whose `dxf`s differ, arranged so that:
//!
//! * **the wrong order is not a crash but a different colour**, which a screenshot would not catch;
//! * **the rules are in different blocks**, because `@priority` is workbook-scoped and a per-block
//!   sort is wrong in a way no single-block fixture can see;
//! * **the file order and the priority order disagree**, so a reader that walked the markup wins on
//!   neither.
//!
//! # Provenance
//!
//! * **SpecCode** — §18.3.1.10: *"Lower numeric values are higher priority than higher numeric
//!   values, where 1 is the highest priority."* And: *"If this flag is 1, no rules with lower
//!   priority shall be applied over this rule, when this rule evaluates to true."*
//! * **DocumentedBehaviour** — §18.8.15 makes a `dxf` a format *"to be applied on top of or in
//!   addition to any formatting already present"*, and every one of its children is
//!   `minOccurs="0"`; so an absent member is inherited and two rules **compose** rather than one
//!   replacing the other outright.
//! * **EngineDerived** — that a tie in `@priority` is broken by document order, block first. Files
//!   Excel writes do contain duplicate priorities; nothing says what it does with them.

mod support;

use mjx_layout::PageIndex;
use mjx_layout_xlsx::{CellReport, SheetBoxModel, SheetGrid};
use mjx_ooxml_types::spreadsheetml::PatternType;
use support::{column_with_blocks, styles_with_differentials, Value};

/// Three differential formats whose effects can be told apart at a glance.
///
/// The first two state a fill and nothing else, so which one wins is visible; the third states only
/// a font colour, so it can *compose* with either rather than replacing it.
const RED_FILL: &str =
    r#"<dxf><fill><patternFill><bgColor rgb="FFFF0000"/></patternFill></fill></dxf>"#;
const BLUE_FILL: &str =
    r#"<dxf><fill><patternFill><bgColor rgb="FF0000FF"/></patternFill></fill></dxf>"#;
const GREEN_TEXT: &str = r#"<dxf><font><color rgb="FF00FF00"/></font></dxf>"#;

/// Lays out one band of a one-column sheet and hands back what each row reported.
fn evaluate(values: &[Value], blocks: &str) -> Vec<Option<CellReport>> {
    use mjx_layout::BoxModel;

    let styles = styles_with_differentials(&[], &[RED_FILL, BLUE_FILL, GREEN_TEXT]);
    let body = column_with_blocks(values, blocks);
    let book = support::workbook(&support::worksheet(&body), &styles);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(8.0, 6.0);
    model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");
    let catalogue = model.catalogue();
    (0..values.len())
        .map(|row| catalogue.cell(u32::try_from(row).unwrap_or(0), 0).cloned())
        .collect()
}

/// The `@rgb` of the fill a row ended up with, or `None`.
fn fill_hex(reports: &[Option<CellReport>], row: usize) -> Option<String> {
    let fill = reports
        .get(row)?
        .as_ref()?
        .conditional
        .as_ref()?
        .fill
        .as_ref()?;
    assert_eq!(
        fill.pattern,
        Some(PatternType::Solid),
        "a dxf that states only a bgColor is a solid fill of it"
    );
    fill.foreground.as_ref()?.rgb.clone()
}

/// The `@rgb` of the font colour a row ended up with.
fn font_hex(reports: &[Option<CellReport>], row: usize) -> Option<String> {
    reports
        .get(row)?
        .as_ref()?
        .conditional
        .as_ref()?
        .font
        .as_ref()?
        .color
        .as_ref()?
        .rgb
        .clone()
}

/// The `(block, rule)` positions that fired on a row, in the order they applied.
fn fired(reports: &[Option<CellReport>], row: usize) -> Vec<(u32, u32)> {
    reports
        .get(row)
        .and_then(Option::as_ref)
        .and_then(|report| report.conditional.as_ref())
        .map(|effect| effect.fired.iter().map(|rule| rule.origin).collect())
        .unwrap_or_default()
}

/// One cell holding 10, which satisfies both `>5` and `>8`.
const TEN: [Value; 1] = [Value::Number(10.0)];

#[test]
fn the_lower_priority_number_wins_even_when_it_is_written_second() {
    // ⚠ The whole case. Block 0 is written first and has priority **2**; block 1 is written second
    // and has priority **1**. Both fire. A reader that took the first rule in the file paints this
    // cell blue; Excel paints it red. Both look like a working implementation.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="2" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>8</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(
        fired(&reports, 0),
        vec![(1, 0), (0, 0)],
        "priority 1 applies before priority 2, and both applied"
    );
    assert_eq!(
        fill_hex(&reports, 0).as_deref(),
        Some("FFFF0000"),
        "the red fill of the higher-priority rule wins; blue is the wrong answer"
    );
}

#[test]
fn reversing_the_priorities_reverses_the_colour_and_nothing_else() {
    // The same two blocks in the same order with the two priorities swapped. If the colour did not
    // move, the case above was measuring document order.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="2" operator="greaterThan"><formula>8</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(fired(&reports, 0), vec![(0, 0), (1, 0)]);
    assert_eq!(fill_hex(&reports, 0).as_deref(), Some("FF0000FF"));
}

#[test]
fn a_dxf_is_a_delta_so_two_rules_compose_rather_than_one_replacing_the_other() {
    // Priority 1 states a **font colour only**; priority 2 states a **fill only**. Excel's answer is
    // green text on a blue fill: the higher-priority rule wins the members it states and leaves the
    // rest to the one below. An implementation where the winner replaces the loser outright loses
    // the fill entirely, and a sheet with one rule on it could never show the difference.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="2" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="2" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(font_hex(&reports, 0).as_deref(), Some("FF00FF00"));
    assert_eq!(fill_hex(&reports, 0).as_deref(), Some("FF0000FF"));
}

#[test]
fn the_first_rule_to_state_a_member_keeps_it() {
    // Both rules state a fill. Composition must not mean *the last one wins*: the priority-1 fill
    // stands and the priority-2 one is never reached, even though the priority-2 rule also fired.
    let blocks = r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule><cfRule type="cellIs" dxfId="1" priority="2" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#;
    let reports = evaluate(&TEN, blocks);
    assert_eq!(fired(&reports, 0), vec![(0, 0), (0, 1)], "both fired");
    assert_eq!(fill_hex(&reports, 0).as_deref(), Some("FFFF0000"));
}

#[test]
fn priorities_interleave_across_blocks_and_a_per_block_sort_would_get_it_wrong() {
    // ⚠ Three blocks holding priorities `1, 4`, `2` and `3`. The order across the sheet is
    // 1, 2, 3, 4 — consecutive rules from *different* blocks. A reader that sorted within each block
    // and then concatenated produces 1, 4, 2, 3, which is a plausible-looking order that puts the
    // wrong rule second.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>0</formula></cfRule><cfRule type="cellIs" dxfId="0" priority="4" operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="2" operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="3" operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(
        fired(&reports, 0),
        vec![(0, 0), (1, 0), (2, 0), (0, 1)],
        "1, 2, 3, 4 — and not 1, 4, 2, 3"
    );
}

#[test]
fn equal_priorities_keep_document_order_block_by_block() {
    // §18.3.1.10 does not say priorities are unique, and files Excel writes have duplicates in them
    // after a range has been copied. The tie is broken by where the rules were read: block, then
    // rule within the block.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="1" operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>0</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(fired(&reports, 0), vec![(0, 0), (1, 0)]);
    assert_eq!(
        fill_hex(&reports, 0).as_deref(),
        Some("FF0000FF"),
        "the first block's rule keeps the fill"
    );
}

// -------------------------------------------------------------------------------------------
// `stopIfTrue`
// -------------------------------------------------------------------------------------------

/// The same two blocks, with `@stopIfTrue` on the first one or not.
fn stop_fixture(stops: bool) -> String {
    let flag = if stops { r#" stopIfTrue="1""# } else { "" };
    format!(
        concat!(
            r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="2" priority="1"{}"#,
            r#" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
            r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="2""#,
            r#" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
        ),
        flag
    )
}

#[test]
fn stop_if_true_ends_the_walk_and_the_lower_rule_never_applies() {
    // ⚠ Both rules would fire and they state **different members**, so the difference is not a
    // missing colour but a different-looking cell: green text on blue without the flag, green text
    // on nothing with it. A reader that treated `@stopIfTrue` as a no-op passes every fixture where
    // the two rules state the same member.
    let without = evaluate(&TEN, &stop_fixture(false));
    assert_eq!(font_hex(&without, 0).as_deref(), Some("FF00FF00"));
    assert_eq!(fill_hex(&without, 0).as_deref(), Some("FF0000FF"));
    assert_eq!(fired(&without, 0).len(), 2);

    let with = evaluate(&TEN, &stop_fixture(true));
    assert_eq!(font_hex(&with, 0).as_deref(), Some("FF00FF00"));
    assert_eq!(
        fill_hex(&with, 0),
        None,
        "the lower-priority fill was never reached"
    );
    assert_eq!(fired(&with, 0), vec![(0, 0)], "and only one rule applied");
}

#[test]
fn stop_if_true_on_a_rule_that_did_not_fire_stops_nothing() {
    // §18.3.1.10's second clause: *"when this rule evaluates to true"*. The flag is on a rule whose
    // condition fails, so the rule below it still applies — which is the half `mjx-sml` explicitly
    // cannot answer, because reporting a stop and applying one are different acts.
    let blocks = concat!(
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="2" priority="1" stopIfTrue="1" operator="greaterThan"><formula>500</formula></cfRule></conditionalFormatting>"#,
        r#"<conditionalFormatting sqref="A1"><cfRule type="cellIs" dxfId="1" priority="2" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&TEN, blocks);
    assert_eq!(fired(&reports, 0), vec![(1, 0)]);
    assert_eq!(fill_hex(&reports, 0).as_deref(), Some("FF0000FF"));
    assert!(
        reports[0]
            .as_ref()
            .and_then(|report| report.conditional.as_ref())
            .is_some_and(|effect| effect.stopped_after.is_none()),
        "and nothing stopped the walk"
    );
}

#[test]
fn a_stop_is_reported_by_the_rule_that_caused_it() {
    let reports = evaluate(&TEN, &stop_fixture(true));
    let stopped = reports[0]
        .as_ref()
        .and_then(|report| report.conditional.as_ref())
        .and_then(|effect| effect.stopped_after)
        .expect("a stop");
    assert_eq!(stopped.origin, (0, 0));
    assert_eq!(stopped.priority, 1);
}

#[test]
fn two_cells_that_fired_different_rules_do_not_share_a_decoration() {
    // The sharing key gained a conditional signature, and this is what it buys: one cell above the
    // threshold and one below it carry the same `xf` and the same number format, so before
    // MJXOFF-173 they would have shared one handle — and the highlight would have painted both, or
    // neither.
    let values = [Value::Number(10.0), Value::Number(1.0)];
    let blocks = r#"<conditionalFormatting sqref="A1:A2"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#;
    let reports = evaluate(&values, blocks);
    let above = reports[0].as_ref().expect("a report").decoration;
    let below = reports[1].as_ref().expect("a report").decoration;
    assert_ne!(above, below, "the highlighted cell has its own decoration");
    assert_eq!(fill_hex(&reports, 0).as_deref(), Some("FFFF0000"));
    assert_eq!(fill_hex(&reports, 1), None);
}

#[test]
fn two_cells_that_fired_the_same_rules_still_share_one() {
    // The other half, and the one that keeps a screen of highlighted cells cheap: the signature is
    // *which rules fired*, so two cells that fired the same rule are one handle rather than two.
    let values = [Value::Number(10.0), Value::Number(11.0)];
    let blocks = r#"<conditionalFormatting sqref="A1:A2"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#;
    let reports = evaluate(&values, blocks);
    assert_eq!(
        reports[0].as_ref().expect("a report").decoration,
        reports[1].as_ref().expect("a report").decoration
    );
}
