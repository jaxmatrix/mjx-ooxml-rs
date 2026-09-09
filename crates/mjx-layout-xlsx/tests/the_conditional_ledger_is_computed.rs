//! The parity ledger for conditional formatting, **computed rather than written down**.
//!
//! The ticket asks that a rule this build cannot evaluate be recorded as `partial`, with its reason,
//! rather than faked or dropped. A row in a document would say so and then rot; this suite asks the
//! engine itself, one rule kind at a time, and prints the answer on every run — so the ledger and
//! the behaviour cannot drift apart, and a green run does not hide what is in it.
//!
//! Run it with `cargo test -p mjx-layout-xlsx --test the_conditional_ledger_is_computed -- --nocapture`
//! to read the table.
//!
//! # What `partial` means here, exactly
//!
//! **Not** *"the rule is unimplemented"*. Every one of the eighteen kinds is decided, and
//! `every_rule_kind_fires_and_does_not.rs` exercises each at a value where it fires and a value
//! where it does not. `partial` names the two places where a *particular rule* cannot be answered
//! because answering it needs a calculation engine, which `PLAN.md` settles as out of scope:
//!
//! * an `expression` rule, whose whole condition is a formula;
//! * a `cellIs` rule whose operand is a reference, a name or a call rather than a literal;
//! * a graded rule whose `cfvo` states `type="formula"` with an expression in `@val`.
//!
//! In each the rule is reported, its reason is reported, its text is reported, and **nothing is
//! painted**. A rule that quietly did not fire and one that could not be evaluated look identical
//! on a screen, so the only honest difference is the one recorded here.
//!
//! MJX-LEDGER-LIMITATION: a conditional-formatting rule whose condition is a formula — an
//! `expression` rule, a `cellIs` with a reference operand, a `cfvo` of `type="formula"` — is
//! reported unevaluated and painted as nothing, because evaluating it needs a calculation engine.

mod support;

use std::collections::BTreeMap;

use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid, UnevaluatedReason};
use support::{column_with_rules, styles_with_differentials, Value};

/// What the engine did with one fixture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verdict {
    /// The rule was decided, one way or the other.
    Evaluated,
    /// It was reported unevaluated, for this reason.
    Partial(UnevaluatedReason),
}

/// Lays a one-cell sheet out and says what became of the rule on it.
fn verdict(value: Value, rules: &str) -> Verdict {
    let values = [value];
    let styles = styles_with_differentials(
        &[],
        &[r#"<dxf><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf>"#],
    );
    let body = column_with_rules(&values, rules);
    let book = support::workbook(&support::worksheet(&body), &styles);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(8.0, 6.0);
    model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");
    let unevaluated = model.catalogue().unevaluated_rules();
    match unevaluated.first() {
        Some((_, _, rule)) => Verdict::Partial(rule.reason),
        None => Verdict::Evaluated,
    }
}

/// Every case the ledger covers: a wire token, a description, a value and the rule's markup.
const CASES: [(&str, &str, Value, &str); 21] = [
    (
        "cellIs",
        "a literal operand",
        Value::Number(10.0),
        r#"<cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule>"#,
    ),
    (
        "cellIs",
        "a reference operand",
        Value::Number(10.0),
        r#"<cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>$B$1</formula></cfRule>"#,
    ),
    (
        "expression",
        "a formula condition",
        Value::Number(10.0),
        r#"<cfRule type="expression" dxfId="0" priority="1"><formula>MOD(ROW(),2)=0</formula></cfRule>"#,
    ),
    (
        "expression",
        "a literal truth value",
        Value::Number(10.0),
        r#"<cfRule type="expression" dxfId="0" priority="1"><formula>TRUE</formula></cfRule>"#,
    ),
    (
        "top10",
        "",
        Value::Number(10.0),
        r#"<cfRule type="top10" dxfId="0" priority="1" rank="1"/>"#,
    ),
    (
        "aboveAverage",
        "",
        Value::Number(10.0),
        r#"<cfRule type="aboveAverage" dxfId="0" priority="1"/>"#,
    ),
    (
        "duplicateValues",
        "",
        Value::Number(10.0),
        r#"<cfRule type="duplicateValues" dxfId="0" priority="1"/>"#,
    ),
    (
        "uniqueValues",
        "",
        Value::Number(10.0),
        r#"<cfRule type="uniqueValues" dxfId="0" priority="1"/>"#,
    ),
    (
        "containsText",
        "",
        Value::Text("x"),
        r#"<cfRule type="containsText" dxfId="0" priority="1" text="x"/>"#,
    ),
    (
        "notContainsText",
        "",
        Value::Text("x"),
        r#"<cfRule type="notContainsText" dxfId="0" priority="1" text="y"/>"#,
    ),
    (
        "beginsWith",
        "",
        Value::Text("x"),
        r#"<cfRule type="beginsWith" dxfId="0" priority="1" text="x"/>"#,
    ),
    (
        "endsWith",
        "",
        Value::Text("x"),
        r#"<cfRule type="endsWith" dxfId="0" priority="1" text="x"/>"#,
    ),
    (
        "containsBlanks",
        "",
        Value::Blank,
        r#"<cfRule type="containsBlanks" dxfId="0" priority="1"/>"#,
    ),
    (
        "notContainsBlanks",
        "",
        Value::Text("x"),
        r#"<cfRule type="notContainsBlanks" dxfId="0" priority="1"/>"#,
    ),
    (
        "containsErrors",
        "",
        Value::Error("#N/A"),
        r#"<cfRule type="containsErrors" dxfId="0" priority="1"/>"#,
    ),
    (
        "notContainsErrors",
        "",
        Value::Text("x"),
        r#"<cfRule type="notContainsErrors" dxfId="0" priority="1"/>"#,
    ),
    (
        "timePeriod",
        "",
        Value::Number(45_819.0),
        r#"<cfRule type="timePeriod" dxfId="0" priority="1" timePeriod="today"/>"#,
    ),
    (
        "colorScale",
        "numeric thresholds",
        Value::Number(10.0),
        r#"<cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="max"/><color rgb="FFFFFFFF"/><color rgb="FFFF0000"/></colorScale></cfRule>"#,
    ),
    (
        "colorScale",
        "a formula threshold",
        Value::Number(10.0),
        r#"<cfRule type="colorScale" priority="1"><colorScale><cfvo type="formula" val="AVERAGE($A:$A)"/><cfvo type="max"/><color rgb="FFFFFFFF"/><color rgb="FFFF0000"/></colorScale></cfRule>"#,
    ),
    (
        "dataBar",
        "numeric thresholds",
        Value::Number(10.0),
        r#"<cfRule type="dataBar" priority="1"><dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule>"#,
    ),
    (
        "iconSet",
        "percentile bands",
        Value::Number(10.0),
        r#"<cfRule type="iconSet" priority="1"><iconSet><cfvo type="percent" val="0"/><cfvo type="percent" val="50"/></iconSet></cfRule>"#,
    ),
];

#[test]
fn the_ledger_prints_and_only_the_formula_paths_are_partial() {
    let mut evaluated = 0_usize;
    let mut partial: BTreeMap<&str, UnevaluatedReason> = BTreeMap::new();
    let mut rows = Vec::new();
    for (token, note, value, markup) in CASES {
        let verdict = verdict(value, markup);
        let label = if note.is_empty() {
            token.to_owned()
        } else {
            format!("{token} ({note})")
        };
        match verdict {
            Verdict::Evaluated => {
                evaluated += 1;
                rows.push(format!("  full      {label}"));
            }
            Verdict::Partial(reason) => {
                partial.insert(token, reason);
                rows.push(format!("  partial   {label} — {}", reason.describe()));
            }
        }
    }
    println!(
        "conditional-formatting ledger: {} full, {} partial, of {} cases",
        evaluated,
        CASES.len() - evaluated,
        CASES.len()
    );
    for row in &rows {
        println!("{row}");
    }

    // ⚠ The assertion, and it is deliberately in **both** directions. Three cases are partial and
    // eighteen are not; a build that answered `partial` more widely would be hiding an evaluator
    // that had stopped working, and one that answered it less widely would be faking a formula.
    assert_eq!(evaluated, 18, "eighteen of the cases are fully evaluated");
    assert_eq!(
        partial.len(),
        3,
        "and three kinds have a partial path: {partial:?}"
    );
    assert_eq!(
        partial.get("expression"),
        Some(&UnevaluatedReason::NoFormulaEngine)
    );
    assert_eq!(
        partial.get("cellIs"),
        Some(&UnevaluatedReason::OperandIsNotALiteral)
    );
    assert_eq!(
        partial.get("colorScale"),
        Some(&UnevaluatedReason::OperandIsNotALiteral)
    );
}

#[test]
fn a_partial_rule_paints_nothing_and_still_says_what_it_would_have_been() {
    // The two halves that make `partial` different from *dropped*: nothing is painted, and the text
    // of the condition survives so a report can quote it.
    let values = [Value::Number(10.0)];
    let styles = styles_with_differentials(
        &[],
        &[r#"<dxf><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf>"#],
    );
    let body = column_with_rules(
        &values,
        r#"<cfRule type="expression" dxfId="0" priority="1"><formula>INDIRECT("A1")&gt;5</formula></cfRule>"#,
    );
    let book = support::workbook(&support::worksheet(&body), &styles);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(8.0, 6.0);
    model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");

    let report = model.catalogue().cell(0, 0).expect("a report");
    let effect = report
        .conditional
        .as_ref()
        .expect("a rule reached the cell");
    assert!(!effect.changes_appearance(), "nothing was painted");
    assert!(effect.fired.is_empty(), "and nothing fired");
    assert_eq!(effect.unevaluated.len(), 1);
    assert_eq!(
        effect.unevaluated[0].condition, "INDIRECT(\"A1\")>5",
        "the condition survives verbatim, entity-decoded as the parser read it"
    );
    assert_eq!(
        model.catalogue().unevaluated_rules().len(),
        1,
        "and the band's own ledger names it once"
    );
    assert_eq!(model.catalogue().unevaluated_rules()[0].0, 0, "at row 1");
}
