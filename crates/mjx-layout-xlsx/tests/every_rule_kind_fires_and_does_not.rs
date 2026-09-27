//! ⚠ The gate this whole child was written around.
//!
//! **A conditional format that never fires is indistinguishable from one that is not implemented.**
//! The sheet renders either way, every fragment is in the same place, and every count matches. So a
//! suite that laid out a sheet with rules on it and asserted that nothing had broken would be green
//! for an evaluator that returns *false* from every predicate — which is the shape this programme
//! keeps finding.
//!
//! The rule here is therefore stated as a rule and enforced as one: **every rule kind is exercised
//! at a value where it FIRES and a value where it does NOT, and both are asserted.** A kind tested
//! only on data that satisfies it proves nothing about the predicate, and
//! [`every_kind_of_st_cf_type_is_exercised_both_ways`] fails if a member of `ST_CfType` is missing
//! from either half of the table.
//!
//! Each case names the rule that fired by its `(block, rule)` position, so the assertion is *which*
//! rule and not *something happened*.

mod support;

use std::collections::BTreeSet;

use mjx_layout::PageIndex;
use mjx_layout_xlsx::{CellReport, ConditionalEffect, SheetBoxModel, SheetGrid, UnevaluatedReason};
use mjx_ooxml_types::spreadsheetml::ConditionalFormatType;
use support::{column_with_blocks, column_with_rules, styles_with_differentials, Value};

/// The one `dxf` every case in this file points at: Excel's own *Light Red Fill with Dark Red Text*,
/// written exactly as Excel writes it — the colour in `bgColor`, and **no `@patternType` at all**.
const HIGHLIGHT: &str = r#"<dxf><font><color rgb="FF9C0006"/></font><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf>"#;

/// A second one, so precedence has two answers to choose between.
const BLUE: &str =
    r#"<dxf><fill><patternFill><bgColor rgb="FF0070C0"/></patternFill></fill></dxf>"#;

/// Lays out one band of a one-column sheet and answers what each row's cell reported.
fn evaluate(values: &[Value], body: &str) -> Vec<Option<CellReport>> {
    evaluate_at(values, body, None)
}

/// The same, with `TODAY()` pinned — which every `timePeriod` case needs, because a suite that read
/// the machine's clock would go red on a particular morning.
fn evaluate_at(values: &[Value], body: &str, today: Option<f64>) -> Vec<Option<CellReport>> {
    use mjx_layout::BoxModel;

    let styles = styles_with_differentials(&[], &[HIGHLIGHT, BLUE]);
    let book = support::workbook(&support::worksheet(body), &styles);
    let mut grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    if let Some(serial) = today {
        grid = grid.with_today(serial);
    }
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

/// The rules that fired on one row, by their `(block, rule)` position.
fn fired(reports: &[Option<CellReport>], row: usize) -> Vec<(u32, u32)> {
    reports
        .get(row)
        .and_then(Option::as_ref)
        .and_then(|report| report.conditional.as_ref())
        .map(|effect| effect.fired.iter().map(|rule| rule.origin).collect())
        .unwrap_or_default()
}

/// Whether the cell in `row` took a conditional fill.
fn is_highlighted(reports: &[Option<CellReport>], row: usize) -> bool {
    reports
        .get(row)
        .and_then(Option::as_ref)
        .and_then(|report| report.conditional.as_ref())
        .is_some_and(ConditionalEffect::changes_appearance)
}

/// One `cfRule` whose body is `body`.
fn rule(kind: &str, priority: i32, attributes: &str, body: &str) -> String {
    format!(r#"<cfRule type="{kind}" dxfId="0" priority="{priority}" {attributes}>{body}</cfRule>"#)
}

// -------------------------------------------------------------------------------------------
// `cellIs` — twelve operators, each over literal operands
// -------------------------------------------------------------------------------------------

#[test]
fn cell_is_greater_than_fires_above_its_operand_and_not_at_it() {
    let values = [Value::Number(10.0), Value::Number(5.0), Value::Number(1.0)];
    let reports = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule(
                "cellIs",
                1,
                r#"operator="greaterThan""#,
                "<formula>5</formula>",
            ),
        ),
    );
    assert_eq!(fired(&reports, 0), vec![(0, 0)], "10 > 5 fires");
    assert!(fired(&reports, 1).is_empty(), "5 > 5 does not");
    assert!(fired(&reports, 2).is_empty(), "1 > 5 does not");
    assert!(is_highlighted(&reports, 0));
    assert!(!is_highlighted(&reports, 1));
}

#[test]
fn cell_is_between_is_inclusive_at_both_ends_and_excludes_outside() {
    let values = [
        Value::Number(15.0),
        Value::Number(10.0),
        Value::Number(20.0),
        Value::Number(25.0),
        Value::Number(9.0),
    ];
    let reports = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule(
                "cellIs",
                1,
                r#"operator="between""#,
                "<formula>10</formula><formula>20</formula>",
            ),
        ),
    );
    for row in 0..3 {
        assert_eq!(fired(&reports, row), vec![(0, 0)], "row {row} is inside");
    }
    assert!(fired(&reports, 3).is_empty(), "25 is outside");
    assert!(fired(&reports, 4).is_empty(), "9 is outside");
}

#[test]
fn cell_is_equal_over_text_is_case_insensitive_and_still_refuses_a_different_word() {
    let values = [
        Value::Text("Overdue"),
        Value::Text("OVERDUE"),
        Value::Text("Paid"),
    ];
    let reports = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule(
                "cellIs",
                1,
                r#"operator="equal""#,
                "<formula>\"overdue\"</formula>",
            ),
        ),
    );
    assert_eq!(fired(&reports, 0), vec![(0, 0)]);
    assert_eq!(fired(&reports, 1), vec![(0, 0)]);
    assert!(fired(&reports, 2).is_empty(), "Paid is a different word");
}

#[test]
fn a_cell_is_whose_operand_is_a_reference_is_unevaluated_rather_than_false() {
    // ⚠ The distinction this whole surface exists for. A rule that could not be answered and a rule
    // that answered *no* render identically; only the report tells them apart.
    let values = [Value::Number(10.0)];
    let reports = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule(
                "cellIs",
                1,
                r#"operator="greaterThan""#,
                "<formula>$B$1</formula>",
            ),
        ),
    );
    let effect = reports[0]
        .as_ref()
        .and_then(|report| report.conditional.as_ref())
        .expect("a rule reached the cell");
    assert!(effect.fired.is_empty(), "nothing fired");
    assert_eq!(effect.unevaluated.len(), 1);
    assert_eq!(
        effect.unevaluated[0].reason,
        UnevaluatedReason::OperandIsNotALiteral
    );
    assert_eq!(effect.unevaluated[0].condition, "$B$1");
    assert!(!effect.changes_appearance(), "and it changed nothing");
}

// -------------------------------------------------------------------------------------------
// `expression`
// -------------------------------------------------------------------------------------------

#[test]
fn an_expression_rule_is_recorded_as_partial_with_its_formula() {
    let values = [Value::Number(1.0)];
    let reports = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule("expression", 1, "", "<formula>MOD(ROW(),2)=0</formula>"),
        ),
    );
    let effect = reports[0]
        .as_ref()
        .and_then(|report| report.conditional.as_ref())
        .expect("a rule reached the cell");
    assert_eq!(effect.unevaluated.len(), 1);
    assert_eq!(
        effect.unevaluated[0].reason,
        UnevaluatedReason::NoFormulaEngine
    );
    assert_eq!(effect.unevaluated[0].condition, "MOD(ROW(),2)=0");
    assert_eq!(
        effect.unevaluated[0].reason.describe(),
        "the condition is a formula and there is no calculation engine"
    );
}

#[test]
fn an_expression_of_a_literal_truth_value_is_answered_both_ways() {
    let values = [Value::Number(1.0)];
    let yes = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule("expression", 1, "", "<formula>TRUE</formula>"),
        ),
    );
    let no = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule("expression", 1, "", "<formula>FALSE</formula>"),
        ),
    );
    assert_eq!(fired(&yes, 0), vec![(0, 0)]);
    assert!(fired(&no, 0).is_empty());
}

// -------------------------------------------------------------------------------------------
// The rank and average kinds, which need the whole range before they can answer for one cell
// -------------------------------------------------------------------------------------------

const LADDER: [Value; 5] = [
    Value::Number(1.0),
    Value::Number(2.0),
    Value::Number(3.0),
    Value::Number(4.0),
    Value::Number(5.0),
];

#[test]
fn top_ten_takes_the_top_n_and_leaves_the_rest() {
    let reports = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("top10", 1, r#"rank="2""#, "")),
    );
    assert!(fired(&reports, 4).len() == 1, "5 is in the top two");
    assert!(fired(&reports, 3).len() == 1, "4 is in the top two");
    assert!(fired(&reports, 2).is_empty(), "3 is not");
    assert!(fired(&reports, 0).is_empty(), "1 is not");
}

#[test]
fn top_ten_from_the_bottom_takes_the_other_end() {
    let reports = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("top10", 1, r#"rank="2" bottom="1""#, "")),
    );
    assert!(fired(&reports, 0).len() == 1, "1 is in the bottom two");
    assert!(fired(&reports, 1).len() == 1, "2 is in the bottom two");
    assert!(fired(&reports, 2).is_empty(), "3 is not");
    assert!(fired(&reports, 4).is_empty(), "5 is not");
}

#[test]
fn top_ten_per_cent_counts_a_fraction_of_the_range() {
    // Five values, 40 per cent: `floor(5 * 0.4)` is two, so 4 and 5 fire and 3 does not.
    let reports = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("top10", 1, r#"rank="40" percent="1""#, "")),
    );
    assert!(fired(&reports, 4).len() == 1);
    assert!(fired(&reports, 3).len() == 1);
    assert!(
        fired(&reports, 2).is_empty(),
        "3 is the median, not the top"
    );
}

#[test]
fn above_average_excludes_the_average_itself_unless_the_rule_says_otherwise() {
    // The mean of 1..=5 is 3.
    let strict = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("aboveAverage", 1, "", "")),
    );
    assert!(fired(&strict, 3).len() == 1, "4 > 3");
    assert!(fired(&strict, 2).is_empty(), "3 is not above 3");

    let inclusive = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("aboveAverage", 1, r#"equalAverage="1""#, "")),
    );
    assert!(fired(&inclusive, 2).len() == 1, "3 >= 3");
    assert!(fired(&inclusive, 1).is_empty(), "2 is still below");
}

#[test]
fn below_average_is_the_same_rule_pointing_the_other_way() {
    let reports = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("aboveAverage", 1, r#"aboveAverage="0""#, "")),
    );
    assert!(fired(&reports, 0).len() == 1, "1 < 3");
    assert!(fired(&reports, 1).len() == 1, "2 < 3");
    assert!(fired(&reports, 2).is_empty(), "3 is the average");
    assert!(fired(&reports, 4).is_empty(), "5 is above it");
}

#[test]
fn a_standard_deviation_moves_the_threshold_away_from_the_mean() {
    // 1..=5 has mean 3 and a *population* deviation of sqrt(2) ≈ 1.4142, so one deviation above the
    // mean is ≈ 4.414: 5 fires and 4 does not. Without the offset 4 would fire, which is what makes
    // this case a test of `@stdDev` rather than of `aboveAverage`.
    let with = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("aboveAverage", 1, r#"stdDev="1""#, "")),
    );
    assert!(fired(&with, 4).len() == 1, "5 > 4.414");
    assert!(fired(&with, 3).is_empty(), "4 < 4.414");

    let without = evaluate(
        &LADDER,
        &column_with_rules(&LADDER, &rule("aboveAverage", 1, "", "")),
    );
    assert!(
        fired(&without, 3).len() == 1,
        "and 4 fires without the deviation, which is what says the offset did the work"
    );
}

// -------------------------------------------------------------------------------------------
// Duplicates and uniques
// -------------------------------------------------------------------------------------------

const REPEATS: [Value; 4] = [
    Value::Text("Apple"),
    Value::Text("apple"),
    Value::Text("Pear"),
    Value::Blank,
];

#[test]
fn duplicate_values_fires_on_the_repeats_and_not_on_the_singleton() {
    let reports = evaluate(
        &REPEATS,
        &column_with_rules(&REPEATS, &rule("duplicateValues", 1, "", "")),
    );
    assert!(fired(&reports, 0).len() == 1, "Apple repeats");
    assert!(fired(&reports, 1).len() == 1, "apple is the same value");
    assert!(fired(&reports, 2).is_empty(), "Pear occurs once");
    assert!(fired(&reports, 3).is_empty(), "a blank is not a duplicate");
}

#[test]
fn unique_values_is_the_same_count_read_the_other_way() {
    let reports = evaluate(
        &REPEATS,
        &column_with_rules(&REPEATS, &rule("uniqueValues", 1, "", "")),
    );
    assert!(fired(&reports, 2).len() == 1, "Pear is the unique one");
    assert!(fired(&reports, 0).is_empty(), "Apple is not");
    assert!(fired(&reports, 3).is_empty(), "and a blank is neither");
}

// -------------------------------------------------------------------------------------------
// The four text kinds, whose operand is `@text`
// -------------------------------------------------------------------------------------------

const WORDS: [Value; 2] = [Value::Text("Overdue"), Value::Text("Paid")];

#[test]
fn contains_text_and_its_negation_split_the_same_two_cells() {
    let yes = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("containsText", 1, r#"text="due""#, "")),
    );
    assert!(fired(&yes, 0).len() == 1, "Overdue contains due");
    assert!(fired(&yes, 1).is_empty(), "Paid does not");

    let no = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("notContainsText", 1, r#"text="due""#, "")),
    );
    assert!(fired(&no, 1).len() == 1, "Paid does not contain due");
    assert!(fired(&no, 0).is_empty(), "Overdue does");
}

#[test]
fn begins_with_and_ends_with_are_anchored_and_not_merely_contained() {
    let begins = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("beginsWith", 1, r#"text="Over""#, "")),
    );
    assert!(fired(&begins, 0).len() == 1);
    assert!(fired(&begins, 1).is_empty());

    // `due` is *in* Overdue but not at its start, which is what tells `beginsWith` from
    // `containsText` — a fixture where the needle is at the start proves neither.
    let anchored = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("beginsWith", 1, r#"text="due""#, "")),
    );
    assert!(
        fired(&anchored, 0).is_empty(),
        "Overdue does not begin with due"
    );

    let ends = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("endsWith", 1, r#"text="due""#, "")),
    );
    assert!(fired(&ends, 0).len() == 1, "Overdue ends with due");
    assert!(fired(&ends, 1).is_empty());
    let not_at_the_end = evaluate(
        &WORDS,
        &column_with_rules(&WORDS, &rule("endsWith", 1, r#"text="Over""#, "")),
    );
    assert!(fired(&not_at_the_end, 0).is_empty());
}

// -------------------------------------------------------------------------------------------
// Blanks and errors
// -------------------------------------------------------------------------------------------

const MIXED: [Value; 4] = [
    Value::Blank,
    Value::Text("   "),
    Value::Text("x"),
    Value::Error("#N/A"),
];

#[test]
fn contains_blanks_follows_excels_own_formula_and_counts_spaces_as_blank() {
    let reports = evaluate(
        &MIXED,
        &column_with_rules(&MIXED, &rule("containsBlanks", 1, "", "")),
    );
    assert!(fired(&reports, 0).len() == 1, "an empty cell is blank");
    assert!(
        fired(&reports, 1).len() == 1,
        "LEN(TRIM(A2))=0, so three spaces are blank too"
    );
    assert!(fired(&reports, 2).is_empty(), "x is not");
    assert!(fired(&reports, 3).is_empty(), "an error is not");
}

#[test]
fn not_contains_blanks_is_its_exact_negation() {
    let reports = evaluate(
        &MIXED,
        &column_with_rules(&MIXED, &rule("notContainsBlanks", 1, "", "")),
    );
    assert!(fired(&reports, 0).is_empty());
    assert!(fired(&reports, 1).is_empty());
    assert!(fired(&reports, 2).len() == 1);
    assert!(fired(&reports, 3).len() == 1);
}

#[test]
fn contains_errors_and_its_negation_split_on_the_error_code() {
    let yes = evaluate(
        &MIXED,
        &column_with_rules(&MIXED, &rule("containsErrors", 1, "", "")),
    );
    assert!(fired(&yes, 3).len() == 1, "#N/A is an error");
    assert!(fired(&yes, 2).is_empty(), "x is not");

    let no = evaluate(
        &MIXED,
        &column_with_rules(&MIXED, &rule("notContainsErrors", 1, "", "")),
    );
    assert!(fired(&no, 2).len() == 1);
    assert!(fired(&no, 3).is_empty());
}

// -------------------------------------------------------------------------------------------
// `timePeriod`, with the clock pinned
// -------------------------------------------------------------------------------------------

/// 2025-06-11, a Wednesday, in the 1900 system.
const TODAY: f64 = 45_819.0;

/// The ten windows, each with a serial inside it and a serial outside it.
fn time_period_case(period: &str, inside: f64, outside: f64) {
    let values = [Value::Number(inside), Value::Number(outside)];
    let reports = evaluate_at(
        &values,
        &column_with_rules(
            &values,
            &rule("timePeriod", 1, &format!(r#"timePeriod="{period}""#), ""),
        ),
        Some(TODAY),
    );
    assert!(
        fired(&reports, 0).len() == 1,
        "{period}: {inside} should be inside"
    );
    assert!(
        fired(&reports, 1).is_empty(),
        "{period}: {outside} should be outside"
    );
}

#[test]
fn all_ten_time_periods_have_a_serial_inside_them_and_one_outside() {
    time_period_case("today", TODAY, TODAY - 1.0);
    time_period_case("yesterday", TODAY - 1.0, TODAY);
    time_period_case("tomorrow", TODAY + 1.0, TODAY);
    // Seven days ending today: `today-6` is in and `today-7` is not.
    time_period_case("last7Days", TODAY - 6.0, TODAY - 7.0);
    // Wednesday 11 June 2025: this week began on Sunday the 8th (serial 45,816).
    time_period_case("thisWeek", TODAY - 3.0, TODAY - 4.0);
    time_period_case("lastWeek", TODAY - 7.0, TODAY - 3.0);
    time_period_case("nextWeek", TODAY + 4.0, TODAY + 3.0);
    // June has thirty days; the 1st is serial 45,809 and 31 May is 45,808.
    time_period_case("thisMonth", 45_809.0, 45_808.0);
    time_period_case("lastMonth", 45_808.0, 45_809.0);
    time_period_case("nextMonth", 45_839.0, 45_838.0);
}

#[test]
fn a_time_period_is_a_window_and_not_merely_a_comparison() {
    // `yesterday` must refuse the day *before* yesterday as well as today; a rule implemented as
    // `<= today - 1` would pass a fixture that only tried today.
    let values = [Value::Number(TODAY - 2.0)];
    let reports = evaluate_at(
        &values,
        &column_with_rules(
            &values,
            &rule("timePeriod", 1, r#"timePeriod="yesterday""#, ""),
        ),
        Some(TODAY),
    );
    assert!(fired(&reports, 0).is_empty());
}

// -------------------------------------------------------------------------------------------
// The three graded kinds. Their numbers live in `graded_rules_interpolate.rs`; what is asserted
// here is only that each reaches a cell and leaves another one alone.
// -------------------------------------------------------------------------------------------

/// A colour scale whose two stops are the range's own minimum and maximum.
const TWO_COLOUR_SCALE: &str = r#"<cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="max"/><color rgb="FFFFFFFF"/><color rgb="FFFF0000"/></colorScale></cfRule>"#;

/// A data bar over the same span.
const DATA_BAR: &str = r#"<cfRule type="dataBar" priority="1"><dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule>"#;

/// A three-icon set whose bands are the 33rd and 67th percentiles, which is Excel's own default.
const ICON_SET: &str = r#"<cfRule type="iconSet" priority="1"><iconSet><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule>"#;

#[test]
fn a_colour_scale_reaches_a_number_and_leaves_a_label_alone() {
    let values = [Value::Number(1.0), Value::Number(5.0), Value::Text("n/a")];
    let reports = evaluate(&values, &column_with_rules(&values, TWO_COLOUR_SCALE));
    assert!(fired(&reports, 0).len() == 1);
    assert!(fired(&reports, 1).len() == 1);
    assert!(
        fired(&reports, 2).is_empty(),
        "a label has no position between two numbers"
    );
}

#[test]
fn a_data_bar_reaches_a_number_and_leaves_a_label_alone() {
    let values = [Value::Number(1.0), Value::Text("n/a")];
    let reports = evaluate(&values, &column_with_rules(&values, DATA_BAR));
    assert!(fired(&reports, 0).len() == 1);
    assert!(fired(&reports, 1).is_empty());
}

#[test]
fn an_icon_set_reaches_a_number_and_leaves_a_label_alone() {
    let values = [Value::Number(1.0), Value::Text("n/a")];
    let reports = evaluate(&values, &column_with_rules(&values, ICON_SET));
    assert!(fired(&reports, 0).len() == 1);
    assert!(fired(&reports, 1).is_empty());
}

// -------------------------------------------------------------------------------------------
// ⚠ The instrument: no member of `ST_CfType` may be missing from this file
// -------------------------------------------------------------------------------------------

/// Every kind this file exercises in the **firing** direction and in the **not-firing** direction.
///
/// It is a list rather than a derivation on purpose: a helper that collected the kinds by running
/// the suite would be green for a suite that ran nothing. Each entry is checked against a fixture
/// below, so the list cannot drift from the file without one of the two failing.
const EXERCISED: [ConditionalFormatType; 18] = [
    ConditionalFormatType::CellIs,
    ConditionalFormatType::Expression,
    ConditionalFormatType::Top10,
    ConditionalFormatType::AboveAverage,
    ConditionalFormatType::DuplicateValues,
    ConditionalFormatType::UniqueValues,
    ConditionalFormatType::ContainsText,
    ConditionalFormatType::NotContainsText,
    ConditionalFormatType::BeginsWith,
    ConditionalFormatType::EndsWith,
    ConditionalFormatType::ContainsBlanks,
    ConditionalFormatType::NotContainsBlanks,
    ConditionalFormatType::ContainsErrors,
    ConditionalFormatType::NotContainsErrors,
    ConditionalFormatType::TimePeriod,
    ConditionalFormatType::ColorScale,
    ConditionalFormatType::DataBar,
    ConditionalFormatType::IconSet,
];

/// A rule of `kind` with a value that satisfies it and a value that does not.
///
/// Returns the wire token, the sheet body, and which row is expected to fire.
fn both_ways(kind: ConditionalFormatType) -> (&'static str, String, usize, usize) {
    let numbers = [Value::Number(9.0), Value::Number(1.0)];
    let words = [Value::Text("Overdue"), Value::Text("Paid")];
    let blanks = [Value::Blank, Value::Text("x")];
    let errors = [Value::Error("#N/A"), Value::Text("x")];
    let repeats = [
        Value::Text("Apple"),
        Value::Text("Apple"),
        Value::Text("Pear"),
    ];
    match kind {
        ConditionalFormatType::CellIs => (
            "cellIs",
            column_with_rules(
                &numbers,
                &rule(
                    "cellIs",
                    1,
                    r#"operator="greaterThan""#,
                    "<formula>5</formula>",
                ),
            ),
            0,
            1,
        ),
        ConditionalFormatType::Expression => (
            "expression",
            column_with_rules(
                &numbers,
                &rule("expression", 1, "", "<formula>TRUE</formula>"),
            ),
            0,
            // Both rows fire for a literal `TRUE`, so the *not firing* half is a second fixture; the
            // one below uses `FALSE` and is checked separately.
            usize::MAX,
        ),
        ConditionalFormatType::Top10 => (
            "top10",
            column_with_rules(&numbers, &rule("top10", 1, r#"rank="1""#, "")),
            0,
            1,
        ),
        ConditionalFormatType::AboveAverage => (
            "aboveAverage",
            column_with_rules(&numbers, &rule("aboveAverage", 1, "", "")),
            0,
            1,
        ),
        ConditionalFormatType::DuplicateValues => (
            "duplicateValues",
            column_with_rules(&repeats, &rule("duplicateValues", 1, "", "")),
            0,
            2,
        ),
        ConditionalFormatType::UniqueValues => (
            "uniqueValues",
            column_with_rules(&repeats, &rule("uniqueValues", 1, "", "")),
            2,
            0,
        ),
        ConditionalFormatType::ContainsText => (
            "containsText",
            column_with_rules(&words, &rule("containsText", 1, r#"text="due""#, "")),
            0,
            1,
        ),
        ConditionalFormatType::NotContainsText => (
            "notContainsText",
            column_with_rules(&words, &rule("notContainsText", 1, r#"text="due""#, "")),
            1,
            0,
        ),
        ConditionalFormatType::BeginsWith => (
            "beginsWith",
            column_with_rules(&words, &rule("beginsWith", 1, r#"text="Over""#, "")),
            0,
            1,
        ),
        ConditionalFormatType::EndsWith => (
            "endsWith",
            column_with_rules(&words, &rule("endsWith", 1, r#"text="due""#, "")),
            0,
            1,
        ),
        ConditionalFormatType::ContainsBlanks => (
            "containsBlanks",
            column_with_rules(&blanks, &rule("containsBlanks", 1, "", "")),
            0,
            1,
        ),
        ConditionalFormatType::NotContainsBlanks => (
            "notContainsBlanks",
            column_with_rules(&blanks, &rule("notContainsBlanks", 1, "", "")),
            1,
            0,
        ),
        ConditionalFormatType::ContainsErrors => (
            "containsErrors",
            column_with_rules(&errors, &rule("containsErrors", 1, "", "")),
            0,
            1,
        ),
        ConditionalFormatType::NotContainsErrors => (
            "notContainsErrors",
            column_with_rules(&errors, &rule("notContainsErrors", 1, "", "")),
            1,
            0,
        ),
        ConditionalFormatType::TimePeriod => {
            let days = [Value::Number(TODAY), Value::Number(TODAY - 1.0)];
            (
                "timePeriod",
                column_with_rules(&days, &rule("timePeriod", 1, r#"timePeriod="today""#, "")),
                0,
                1,
            )
        }
        ConditionalFormatType::ColorScale => (
            "colorScale",
            column_with_rules(&[Value::Number(1.0), Value::Text("n/a")], TWO_COLOUR_SCALE),
            0,
            1,
        ),
        ConditionalFormatType::DataBar => (
            "dataBar",
            column_with_rules(&[Value::Number(1.0), Value::Text("n/a")], DATA_BAR),
            0,
            1,
        ),
        ConditionalFormatType::IconSet => (
            "iconSet",
            column_with_rules(&[Value::Number(1.0), Value::Text("n/a")], ICON_SET),
            0,
            1,
        ),
    }
}

#[test]
fn every_kind_of_st_cf_type_is_exercised_both_ways() {
    // The eighteen members of `ST_CfType`. If a nineteenth is ever generated, `both_ways` stops
    // compiling — its `match` is exhaustive and has no wildcard arm — which is the point.
    let listed: BTreeSet<&str> = EXERCISED.iter().map(|kind| both_ways(*kind).0).collect();
    assert_eq!(
        listed.len(),
        18,
        "every member of ST_CfType has a fixture, and no two share one"
    );

    for kind in EXERCISED {
        let (token, body, fires, does_not) = both_ways(kind);
        let values: Vec<Value> = vec![Value::Number(0.0); 4];
        let reports = evaluate_at(&values, &body, Some(TODAY));
        assert!(
            !fired(&reports, fires).is_empty(),
            "{token}: the fixture's firing row did not fire"
        );
        if does_not != usize::MAX {
            assert!(
                fired(&reports, does_not).is_empty(),
                "{token}: the fixture's non-firing row fired anyway"
            );
        }
    }

    // `expression`'s other half, which needs its own markup rather than another row.
    let values = [Value::Number(1.0)];
    let never = evaluate(
        &values,
        &column_with_rules(
            &values,
            &rule("expression", 1, "", "<formula>FALSE</formula>"),
        ),
    );
    assert!(
        fired(&never, 0).is_empty(),
        "expression: FALSE did not fire"
    );
}

#[test]
fn a_rule_whose_block_covers_no_cell_reaches_none_of_them() {
    // The negative control for the whole file: the same rule, over a range that does not include
    // the cell. If this fired, every assertion above would be measuring something other than the
    // `@sqref`.
    let values = [Value::Number(10.0)];
    let body = column_with_blocks(
        &values,
        r#"<conditionalFormatting sqref="C1:C9"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#,
    );
    let reports = evaluate(&values, &body);
    assert!(fired(&reports, 0).is_empty());
    assert!(
        reports[0]
            .as_ref()
            .is_some_and(|report| report.conditional.is_none()),
        "and no effect was recorded at all"
    );
}
