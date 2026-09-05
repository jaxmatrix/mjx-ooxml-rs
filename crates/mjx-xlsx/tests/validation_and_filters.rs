//! **MJXOFF-123 at the package tier.** Data validation, autofilters and sort state driven through
//! [`Workbook`] rather than through [`mjx_sml::WorksheetPart`].
//!
//! # Why this tier is not the markup tier under another name
//!
//! `crates/mjx-sml/tests/validation_and_filters.rs` pins the model: the six filter kinds, the
//! extension slot, the formulas that are never resolved, the rows that never move. Three things only
//! *this* tier can say, and each is asserted below:
//!
//! * **A container round-trips.** Opening the fixture and saving it unedited must give back every
//!   part byte for byte — the autofilter and the validations included — and that is a statement about
//!   a ZIP, not about an element.
//! * **Authoring touches one part.** Adding a validation or setting an autofilter rewrites
//!   `/xl/worksheets/sheet1.xml` and **nothing else**: not `xl/styles.xml`, not the workbook, not
//!   `[Content_Types].xml`. The other parts are compared against the *original file's* bytes.
//! * **An edit somewhere else leaves both features alone.** Writing a cell value goes through the
//!   packed store, and the two feature slots must come back out of the container unchanged.
//!
//! # Neither feature is ever applied — restated at the tier that would be tempted
//!
//! [`Workbook::set_auto_filter`] hides no row, [`Workbook::remove_auto_filter`] unhides none, and
//! [`Workbook::add_data_validation`] rejects nothing.
//! [`the_hidden_row_survives_every_call_this_tier_offers`] is the assertion.

use mjx_ooxml_types::spreadsheetml::{DataValidationType, DynamicFilterType, FilterOperator};
use mjx_opc::{Package, PartName};
use mjx_sml::{
    AutoFilterSpec, CellRange, CellRangeList, CellReference, CellValue, CustomFilterSpec,
    DataValidationSpec, FilterColumnSpec, FilterKind, FilterSpecKind, SortConditionSpec,
    SortStateSpec,
};
use mjx_xlsx::Workbook;

/// The fixture this suite is written against.
const FIXTURE: &str = "validation_and_filters.xlsx";

/// The one worksheet part of the fixture.
const SHEET: &str = "/xl/worksheets/sheet1.xml";

/// The fixture, opened.
fn workbook() -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens")
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a cell reference: {error}"))
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A range list, or a panic naming it.
fn ranges(text: &str) -> CellRangeList {
    CellRangeList::parse(text).unwrap_or_else(|error| panic!("{text} is a sqref: {error}"))
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

/// The bytes of one part of a saved package, as text.
fn part_text(bytes: &[u8], part: &str) -> String {
    let package = Package::open(bytes).expect("the package opens");
    let name = PartName::new(part).expect("a part name");
    String::from_utf8(
        package
            .part_bytes(&name)
            .expect("the part is there")
            .to_vec(),
    )
    .expect("UTF-8")
}

// -----------------------------------------------------------------------------------------------
// Reading, through the package
// -----------------------------------------------------------------------------------------------

/// The package tier reports the same six kinds the markup tier does, and the same seventh column
/// with none.
#[test]
fn the_package_tier_reports_every_filter_kind() {
    let workbook = workbook();
    let kinds = workbook
        .auto_filter(0, |_part, filter| {
            filter
                .expect("the sheet has an autoFilter")
                .columns()
                .map(|column| column.filter().and_then(FilterKind::local))
                .collect::<Vec<_>>()
        })
        .expect("the tab reads")
        .expect("the tab reaches a worksheet part");

    assert_eq!(
        kinds,
        vec![
            Some("customFilters"),
            Some("filters"),
            Some("top10"),
            Some("dynamicFilter"),
            Some("colorFilter"),
            Some("iconFilter"),
            None,
        ]
    );
}

/// The package tier reports the sort state and the four validations, with their formulas as text.
#[test]
fn the_package_tier_reports_the_sort_state_and_the_validations() {
    let workbook = workbook();

    let conditions = workbook
        .auto_filter(0, |_part, filter| {
            filter
                .expect("an autoFilter")
                .sort_state()
                .expect("a sortState")
                .len()
        })
        .expect("the tab reads")
        .expect("a worksheet part");
    assert_eq!(conditions, 2);

    let sources = workbook
        .data_validations(0, |part, rules| {
            rules
                .iter()
                .map(|rule| {
                    (
                        rule.kind(part.interner()).expect("@type"),
                        rule.first_formula().map(|f| f.text().to_owned()),
                    )
                })
                .collect::<Vec<_>>()
        })
        .expect("the tab reads")
        .expect("a worksheet part");
    assert_eq!(sources.len(), 4);
    assert_eq!(
        sources.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
        vec![
            DataValidationType::List,
            DataValidationType::List,
            DataValidationType::Decimal,
            DataValidationType::Custom,
        ]
    );
    assert_eq!(
        sources
            .iter()
            .map(|(_, formula)| formula.clone())
            .collect::<Vec<_>>(),
        vec![
            Some("$G$2:$G$4".to_owned()),
            Some("\"Ada,Grace,Alan\"".to_owned()),
            Some("0".to_owned()),
            Some("AND($E2>0,$E2<\"10000\")".to_owned()),
        ],
        "the range source and the literal list both come through unchanged"
    );
}

// -----------------------------------------------------------------------------------------------
// The container
// -----------------------------------------------------------------------------------------------

/// Opening the fixture and saving it unedited gives every part back byte for byte.
#[test]
fn the_fixture_round_trips_through_the_package_untouched() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let saved = workbook().save().expect("saves");
    assert_eq!(
        parts_of(&saved),
        parts_of(&original),
        "an unedited save must give every part back byte for byte"
    );
}

/// **Editing a cell inside the filtered range leaves the autofilter, the sort state, every row's
/// `@hidden` and every other part byte-identical.**
///
/// This is the ticket's fourth *Done when* clause, at the tier where a package could get it wrong.
#[test]
fn a_cell_edit_leaves_the_features_and_every_other_part_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .set_cell_value(0, cell("B3"), CellValue::Number(999.0))
        .expect("the cell writes");
    let saved = workbook.save().expect("saves");

    // Every part but the worksheet is untouched.
    let before: Vec<(String, Vec<u8>)> = parts_of(&original)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    let after: Vec<(String, Vec<u8>)> = parts_of(&saved)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    assert_eq!(before, after, "a cell edit rewrote a part it did not name");

    let sheet = part_text(&saved, SHEET);
    assert_ne!(
        sheet,
        part_text(&original, SHEET),
        "the edit must actually have changed the worksheet"
    );
    for fragment in [
        "<autoFilter ref=\"A1:G20\"><filterColumn colId=\"1\"><customFilters and=\"1\">",
        "<sortState ref=\"A2:G20\" caseSensitive=\"1\">",
        "<dataValidations count=\"9\" xWindow=\"120\" yWindow=\"180\">",
        "<formula1>$G$2:$G$4</formula1>",
        "<row r=\"4\" hidden=\"1\" customHeight=\"1\" ht=\"0\">",
    ] {
        assert!(
            sheet.contains(fragment),
            "a cell edit lost `{fragment}` from the worksheet"
        );
    }
}

// -----------------------------------------------------------------------------------------------
// Authoring, through the package
// -----------------------------------------------------------------------------------------------

/// Authoring a list validation writes **one** part, and what it writes is schema-valid markup that a
/// consumer can open.
///
/// The schema half runs under the shared ECMA-376 gate, which skips without `References/` and which
/// `MJX_REQUIRE_SCHEMA=1` turns into a failure — the flag CI sets.
#[test]
fn authoring_a_list_validation_writes_one_part_and_is_schema_valid() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .add_data_validation(
            0,
            &DataValidationSpec::list(ranges("C2:C20"), "$G$2:$G$4")
                .with_error("Not a grade", "Pick one of the three")
                .with_prompt("Grade", "Low, Medium or High"),
        )
        .expect("the validation writes");
    let saved = workbook.save().expect("saves");

    let before: Vec<(String, Vec<u8>)> = parts_of(&original)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    let after: Vec<(String, Vec<u8>)> = parts_of(&saved)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    assert_eq!(
        before, after,
        "adding a data validation must touch the worksheet and nothing else"
    );

    let sheet = part_text(&saved, SHEET);
    assert!(
        sheet.contains(
            "<dataValidation type=\"list\" showInputMessage=\"true\" showErrorMessage=\"true\" \
             errorTitle=\"Not a grade\" error=\"Pick one of the three\" promptTitle=\"Grade\" \
             prompt=\"Low, Medium or High\" sqref=\"C2:C20\"><formula1>$G$2:$G$4</formula1>\
             </dataValidation>"
        ),
        "the authored rule's markup is not what was expected. Got: {sheet}"
    );
    assert!(
        sheet.contains("<dataValidations count=\"9\""),
        "appending a rule leaves the producer's stale `@count` exactly where it was"
    );

    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "a workbook with an authored list validation",
        &saved,
    );
}

/// Setting an autofilter writes **one** part, replaces whichever filter was there, and is
/// schema-valid.
///
/// `autoFilter` is `maxOccurs="1"`, so this is a setter; the fixture already carries one, which is
/// what makes the replacement observable.
#[test]
fn setting_an_autofilter_replaces_the_one_there_and_is_schema_valid() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .set_auto_filter(
            0,
            &AutoFilterSpec::over(range("A1:G20"))
                .with_column(FilterColumnSpec::new(
                    0,
                    FilterSpecKind::values(["North", "West"]),
                ))
                .with_column(FilterColumnSpec::new(
                    1,
                    FilterSpecKind::Custom {
                        comparisons: vec![
                            CustomFilterSpec::new(FilterOperator::GreaterThanOrEqual, "100"),
                            CustomFilterSpec::new(FilterOperator::LessThan, "5000"),
                        ],
                        requires_both: true,
                    },
                ))
                .with_column(FilterColumnSpec::new(
                    2,
                    FilterSpecKind::Dynamic(DynamicFilterType::ThisYear),
                ))
                .with_sort_state(SortStateSpec::new(
                    range("A2:G20"),
                    vec![SortConditionSpec::descending(range("B2:B20"))],
                )),
        )
        .expect("the autofilter writes");
    let saved = workbook.save().expect("saves");

    let before: Vec<(String, Vec<u8>)> = parts_of(&original)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    let after: Vec<(String, Vec<u8>)> = parts_of(&saved)
        .into_iter()
        .filter(|(name, _)| name != SHEET)
        .collect();
    assert_eq!(
        before, after,
        "setting an autofilter must touch the worksheet and nothing else"
    );

    let sheet = part_text(&saved, SHEET);
    assert_eq!(
        sheet.matches("<autoFilter").count(),
        1,
        "a worksheet carries at most one autoFilter"
    );
    assert!(
        !sheet.contains("iconFilter"),
        "the fixture's autofilter was replaced, not appended to"
    );
    assert!(sheet.contains(
        "<autoFilter ref=\"A1:G20\"><filterColumn colId=\"0\"><filters><filter val=\"North\"/>\
         <filter val=\"West\"/></filters></filterColumn>"
    ));
    assert!(sheet
        .contains("<filterColumn colId=\"2\"><dynamicFilter type=\"thisYear\"/></filterColumn>"));
    // The validations, which the setter never named, are still where they were.
    assert!(sheet.contains("<formula1>$G$2:$G$4</formula1>"));

    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "a workbook with an authored autofilter",
        &saved,
    );
}

/// A workbook authored **from nothing** with both features is schema-valid, which is where an
/// invalid authored part shows up with nothing preserved around it to hide behind.
#[test]
fn a_workbook_authored_from_nothing_with_both_features_is_schema_valid() {
    let mut workbook = Workbook::blank().expect("authored");
    for (address, value) in [
        ("A1", CellValue::InlineString("Region")),
        ("A2", CellValue::InlineString("North")),
        ("B1", CellValue::InlineString("Q1")),
        ("B2", CellValue::Number(1200.0)),
    ] {
        workbook
            .set_cell_value(0, cell(address), value)
            .expect("the store accepts the value");
    }
    workbook
        .set_auto_filter(
            0,
            &AutoFilterSpec::over(range("A1:B2"))
                .with_column(FilterColumnSpec::new(
                    0,
                    FilterSpecKind::values(["North", "South"]),
                ))
                .with_sort_state(SortStateSpec::new(
                    range("A2:B2"),
                    vec![SortConditionSpec::ascending(range("A2:A2"))],
                )),
        )
        .expect("the autofilter writes");
    workbook
        .add_data_validation(
            0,
            &DataValidationSpec::list(ranges("A2:A100"), "\"North,South,East,West\""),
        )
        .expect("the validation writes");
    workbook
        .add_data_validation(
            0,
            &DataValidationSpec::between(
                ranges("B2:B100"),
                DataValidationType::WholeNumber,
                "0",
                "100000",
            ),
        )
        .expect("the validation writes");

    let bytes = workbook.save().expect("saves");
    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "a workbook authored with filters and validations",
        &bytes,
    );

    let sheet = part_text(&bytes, SHEET);
    assert!(
        sheet.contains("<formula1>\"North,South,East,West\"</formula1>"),
        "the literal list is written exactly as stated. Got: {sheet}"
    );
    assert!(
        sheet.contains("<formula1>0</formula1><formula2>100000</formula2>"),
        "both formulas of a `between` rule are written, in order. Got: {sheet}"
    );
}

// -----------------------------------------------------------------------------------------------
// Never apply
// -----------------------------------------------------------------------------------------------

/// **The hidden row survives every call this tier offers.**
///
/// Row 4 is hidden and row 5 is not. Setting a filter that matches neither, and then removing the
/// filter altogether, changes neither flag: a `@hidden` is the file's statement about a row, and
/// nothing here evaluates a filter to decide it.
#[test]
fn the_hidden_row_survives_every_call_this_tier_offers() {
    let hidden_rows = |bytes: &[u8]| -> Vec<(u32, bool)> {
        let sheet = part_text(bytes, SHEET);
        let markup = mjx_sml::WorksheetPart::read_part(sheet.as_bytes())
            .expect("reads")
            .expect("a worksheet");
        markup
            .rows()
            .map(|row| (row.number().expect("a row number"), row.is_hidden()))
            .collect()
    };
    let original = hidden_rows(&mjx_fixtures::fixture(FIXTURE));
    assert_eq!(
        original,
        vec![
            (1, false),
            (2, false),
            (3, false),
            (4, true),
            (5, false),
            (20, false)
        ],
        "the fixture must hold one hidden row beside a visible one"
    );

    let mut workbook = workbook();
    workbook
        .set_auto_filter(
            0,
            &AutoFilterSpec::over(range("A1:G20")).with_column(FilterColumnSpec::new(
                0,
                // Nothing in the sheet is `Atlantis`, so a library that applied the filter would
                // hide every row; one that records it hides none.
                FilterSpecKind::values(["Atlantis"]),
            )),
        )
        .expect("writes");
    assert_eq!(
        hidden_rows(&workbook.save().expect("saves")),
        original,
        "setting a filter that matches no row hid nothing"
    );

    assert!(
        workbook.remove_auto_filter(0).expect("removes"),
        "the sheet had an autofilter to remove"
    );
    let saved = workbook.save().expect("saves");
    assert_eq!(
        hidden_rows(&saved),
        original,
        "removing the filter unhid nothing either"
    );
    assert!(
        !part_text(&saved, SHEET).contains("<autoFilter"),
        "the autofilter really was removed, so this test is not vacuous"
    );
}

/// Removing the last validation removes the enclosing element too, because the schema declares
/// `dataValidation` `minOccurs="1"`.
#[test]
fn removing_the_last_validation_removes_the_element_with_it() {
    let mut workbook = workbook();
    for _ in 0..4 {
        assert!(
            workbook.remove_data_validation(0, 0).expect("removes"),
            "there was a rule to remove"
        );
    }
    assert!(
        !workbook.remove_data_validation(0, 0).expect("asks"),
        "there is nothing left to remove"
    );
    let sheet = part_text(&workbook.save().expect("saves"), SHEET);
    assert!(
        !sheet.contains("<dataValidations"),
        "an empty `dataValidations` is markup no validator accepts, so it goes with the last rule"
    );
    assert!(
        sheet.contains("<x14:dataValidations"),
        "the unmodelled `x14` validations are somebody else's markup and stay exactly where they are"
    );
}
