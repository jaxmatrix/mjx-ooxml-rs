//! **MJXOFF-120 at the package tier.** Conditional formatting driven through [`Workbook`] rather
//! than through [`mjx_sml::WorksheetPart`].
//!
//! # Why this tier is not the markup tier under another name
//!
//! `crates/mjx-sml/tests/conditional_formatting.rs` pins the model: the cross-block priority order,
//! the rule kinds, the `x14` extensions. Three things only *this* tier can say, and each is asserted
//! below:
//!
//! * **Conditional formatting spans two parts.** A rule lives in the worksheet and the formatting it
//!   imposes lives in `xl/styles.xml`. Only a package can find both, and only a package can be
//!   wrong about *which* worksheet a `@dxfId` was resolved against.
//! * **Authoring writes two parts and leaves every other one alone.** Adding a highlighted rule
//!   appends a `dxf` and adds a block; every other part of the container must come back byte for
//!   byte, and the two it did touch must still be schema-valid.
//! * **Appending a `dxf` cannot repoint an existing one.** The fixture's rules name `dxfId="0"` and
//!   `dxfId="1"`; after an append they must still resolve to the same two `dxf` elements, compared
//!   against the *original file's* bytes rather than against a second run of this crate's writer.

use mjx_ooxml_types::spreadsheetml::ConditionalFormattingOperator;
use mjx_opc::{Package, PartName};
use mjx_sml::{
    CellRangeList, CellReference, CellValue, ColorScaleSpec, ConditionalRuleSpec,
    ConditionalRuleSpecKind, DataBarSpec, DifferentialFormatSpec,
};
use mjx_xlsx::Workbook;

/// The fixture this suite is written against.
const FIXTURE: &str = "conditional_formatting.xlsx";

/// The fixture, opened.
fn workbook() -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens")
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell reference: {error}"))
}

/// The bytes of one part of a saved package.
fn part_of(bytes: &[u8], part: &str) -> Vec<u8> {
    let package = Package::open(bytes).expect("the package opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// Every part of a package, by name, so that two saves can be compared part by part.
fn parts_of(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(bytes).expect("the package opens");
    package
        .part_names()
        .map(|name| {
            (
                name.as_str().to_owned(),
                package
                    .part_bytes(&name)
                    .expect("a listed part has bytes")
                    .to_vec(),
            )
        })
        .collect()
}

/// The chain the package tier reports for one cell, as `(block, rule, priority)`.
#[test]
fn the_package_tier_reports_the_chain_the_markup_tier_reports() {
    let workbook = workbook();
    let through_package = workbook
        .conditional_rules_for(0, cell("B2"), |chain| {
            chain
                .rules()
                .iter()
                .map(|applied| {
                    (
                        applied.block_index(),
                        applied.rule_index(),
                        applied.priority(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .expect("the sheet resolves")
        .expect("the tab reaches a worksheet part");

    assert_eq!(
        through_package,
        vec![
            (0, 0, 1),
            (1, 0, 2),
            (3, 0, 2),
            (2, 0, 3),
            (0, 1, 4),
            (3, 1, 7)
        ],
        "the package tier must give the markup tier's answer, not a per-block one"
    );
}

/// **Two parts, one answer.** The base format comes from `xl/styles.xml`'s `cellXfs`; the `dxf`
/// each candidate names comes from the *same* part's `dxfs`; and the rules come from the worksheet.
/// Nothing merges them.
#[test]
fn the_base_format_and_the_conditional_layer_are_reported_side_by_side() {
    let workbook = workbook();
    let (base_style, base_number_format, layer) = workbook
        .conditional_cell_format(0, cell("B2"), |resolved| {
            (
                resolved.base().style_index(),
                resolved.base().number_format().resource_index,
                resolved
                    .layer()
                    .iter()
                    .map(|entry| {
                        (
                            entry.priority(),
                            entry.differential_format_index(),
                            entry
                                .differential_format()
                                .is_some_and(|format| format.fill().is_some()),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .expect("both parts resolve")
        .expect("the tab reaches both parts");

    assert_eq!(base_style, 1, "B2 writes s=\"1\"");
    assert_eq!(
        base_number_format,
        Some(2),
        "and cellXfs[1] states numFmtId=2, which no conditional rule may have altered"
    );
    assert_eq!(
        layer,
        vec![
            (1, Some(0), true),
            (2, None, false),
            (2, Some(0), true),
            (3, None, false),
            (4, Some(1), true),
            (7, None, false),
        ]
    );
}

/// **Appending a `dxf` repoints nothing.** The two the fixture already holds must come back
/// identical — compared against the original file's own bytes.
#[test]
fn appending_a_dxf_leaves_every_existing_dxf_id_naming_what_it_named() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let before = Workbook::open(&original)
        .expect("the fixture opens")
        .conditional_cell_format(0, cell("B2"), fills_of_layer)
        .expect("it resolves")
        .expect("both parts are there");

    let mut workbook = workbook();
    let index = workbook
        .append_differential_format(&DifferentialFormatSpec::highlight("006100", "C6EFCE"))
        .expect("the dxf appends");
    assert_eq!(
        index, 2,
        "the fixture holds two dxfs, so the next index is 2"
    );

    let after = workbook
        .conditional_cell_format(0, cell("B2"), fills_of_layer)
        .expect("it still resolves")
        .expect("both parts are still there");
    assert_eq!(
        after, before,
        "every rule's @dxfId must still resolve to the dxf it resolved to before the append"
    );

    // And the third one really is there, at the index the call answered.
    let saved = workbook.save().expect("the workbook saves");
    let styles = String::from_utf8(part_of(&saved, "/xl/styles.xml")).expect("UTF-8");
    assert_eq!(
        styles.matches("<dxf>").count(),
        3,
        "the table grew by exactly one entry"
    );
    assert!(
        styles.contains("<dxfs count=\"3\">"),
        "and @count followed the collection, because the file declared one"
    );
}

/// The layer's `dxf` fills, as the shape both halves of the append test compare.
fn fills_of_layer(
    resolved: &mjx_sml::ConditionalCellFormat<'_>,
) -> Vec<(i32, Option<u32>, String)> {
    resolved
        .layer()
        .iter()
        .map(|entry| {
            (
                entry.priority(),
                entry.differential_format_index(),
                entry
                    .differential_format()
                    .map(|format| format!("{format:?}"))
                    .unwrap_or_default(),
            )
        })
        .collect()
}

/// Authoring writes exactly two parts: the worksheet gains a block, `xl/styles.xml` gains a `dxf`,
/// and every other part of the container comes back byte for byte.
#[test]
fn authoring_a_rule_touches_two_parts_and_leaves_the_rest_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let untouched = parts_of(
        &Workbook::open(&original)
            .expect("the fixture opens")
            .save()
            .expect("a save with no edits"),
    );

    let mut workbook = workbook();
    let dxf = workbook
        .append_differential_format(&DifferentialFormatSpec::highlight("9C0006", "FFC7CE"))
        .expect("the dxf appends");
    workbook
        .add_conditional_formatting(
            0,
            &CellRangeList::parse("A2:A10").expect("a sqref"),
            &[
                ConditionalRuleSpec {
                    differential_format_index: Some(dxf),
                    ..ConditionalRuleSpec::cell_is(
                        ConditionalFormattingOperator::Equal,
                        ["\"North\"".to_owned()],
                        11,
                    )
                },
                ConditionalRuleSpec {
                    kind: ConditionalRuleSpecKind::ColorScale(ColorScaleSpec::two_color(
                        "FFF8696B", "FF63BE7B",
                    )),
                    priority: 12,
                    stops_lower_priority_rules: None,
                    differential_format_index: None,
                },
                ConditionalRuleSpec {
                    kind: ConditionalRuleSpecKind::DataBar(DataBarSpec::spanning_the_range(
                        "638EC6",
                    )),
                    priority: 13,
                    stops_lower_priority_rules: None,
                    differential_format_index: None,
                },
            ],
        )
        .expect("the block is added");

    let saved = workbook.save().expect("the workbook saves");
    let changed: Vec<String> = parts_of(&saved)
        .into_iter()
        .zip(untouched)
        .filter(|((_, after), (_, before))| after != before)
        .map(|((name, _), _)| name)
        .collect();
    assert_eq!(
        changed,
        vec![
            "/xl/worksheets/sheet1.xml".to_owned(),
            "/xl/styles.xml".to_owned(),
        ],
        "exactly the two parts conditional formatting lives in"
    );

    // The new block reads back, and the pre-existing four are still where they were.
    let reopened = Workbook::open(&saved).expect("the saved workbook opens");
    let chain = reopened
        .conditional_rules_for(0, cell("A2"), |chain| {
            chain
                .rules()
                .iter()
                .map(mjx_sml::AppliedConditionalRule::priority)
                .collect::<Vec<_>>()
        })
        .expect("it resolves")
        .expect("the tab is there");
    assert_eq!(
        chain,
        vec![3, 11, 12, 13],
        "A2 is inside block 2's A1:D20 as well as the new block, and the order is across both"
    );

    // And the authored rule's own dxf is the one that was appended, not one of the fixture's.
    let named = reopened
        .conditional_cell_format(0, cell("A2"), |resolved| {
            resolved
                .layer()
                .iter()
                .map(|entry| (entry.priority(), entry.differential_format_index()))
                .collect::<Vec<_>>()
        })
        .expect("it resolves")
        .expect("both parts are there");
    assert_eq!(
        named,
        vec![(3, None), (11, Some(2)), (12, None), (13, None)],
        "the appended dxf is index 2, and the graded rules name none at all"
    );
}

/// A cell edit leaves every conditional-formatting block byte-identical **through a whole save** —
/// the `x14` extensions with them.
#[test]
fn a_cell_edit_leaves_the_blocks_and_the_x14_extensions_byte_identical_through_a_save() {
    let original = part_of(&mjx_fixtures::fixture(FIXTURE), "/xl/worksheets/sheet1.xml");
    let mut workbook = workbook();
    workbook
        .set_cell_value(0, cell("C3"), CellValue::Number(4321.0))
        .expect("the cell writes");
    let saved = workbook.save().expect("the workbook saves");
    let after = part_of(&saved, "/xl/worksheets/sheet1.xml");

    assert_ne!(
        after, original,
        "the edit must actually have changed something"
    );
    let tail = |bytes: &[u8]| {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let at = text
            .find("<conditionalFormatting")
            .expect("the part has a block");
        text[at..].to_owned()
    };
    assert_eq!(
        tail(&after),
        tail(&original),
        "every byte from the first block onward — both x14 extensions included — must be the \
         file's own"
    );
}

/// A tab that reaches no worksheet part is a question, not a failure — the shape every reader on
/// this surface takes.
#[test]
fn a_workbook_with_no_conditional_formatting_answers_an_empty_chain() {
    let workbook =
        Workbook::open(&mjx_fixtures::fixture("sample.xlsx")).expect("sample.xlsx opens");
    let empty = workbook
        .conditional_rules_for(0, cell("A1"), |chain| chain.is_empty())
        .expect("it resolves")
        .expect("the tab is there");
    assert!(empty, "sample.xlsx writes no conditionalFormatting at all");
}
