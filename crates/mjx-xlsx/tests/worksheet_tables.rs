//! **MJXOFF-125's package gate.** The four things a table is, and the two things creating one must
//! never do.
//!
//! `crates/mjx-sml/tests/worksheet_tables.rs` pins the markup: the columns, the totals-row functions,
//! the formulas-as-text, the resize refusal, and the three-way style lookup. This file pins what only
//! a package can be asked — **which part**, **which relationship**, **which content type**, and
//! whether the four of them still agree after this library has written one.
//!
//! # The fixture, and what each choice makes visible
//!
//! `tests/fixtures/worksheet_tables.xlsx` has two tables on one sheet:
//!
//! * `/xl/tables/table1.xml` is `Sales`, `@id="1"`, wearing the preset `TableStyleMedium2`;
//! * `/xl/tables/table2.xml` is `Regions`, **`@id="4"`**, wearing the workbook's own `AcmeBlue`.
//!
//! The id **gap** is deliberate. A next-free-id derived from the table *count* answers 3, one derived
//! from the highest id answers 5, and only the second is right — so
//! [`the_next_free_table_id_is_one_past_the_highest_and_not_the_table_count`] can tell them apart.
//! With one table both answers are 2 and the case proves nothing, which is why the fixture has two.
//!
//! The sheet also lists the two **in the reverse of the relationship order** (`rId2` then `rId1`), so
//! [`a_sheets_tables_are_listed_in_the_sheets_own_order`] fails for a reader that answered with
//! `WorksheetParts::tables` instead of with `x:tableParts`.
//!
//! # What each mutation would break
//!
//! * Reporting *"style not found"* for a preset name turns
//!   [`a_preset_style_name_is_reported_as_built_in_at_the_package_tier`] red.
//! * Reusing an existing `@id` when creating a table turns
//!   [`a_second_added_table_gets_a_second_id_and_the_workbook_still_validates`] red, through
//!   [`SpreadsheetDefect::DuplicateTableId`].
//! * Leaving out any one of the four things a table is turns
//!   [`adding_a_table_writes_a_part_a_content_type_a_relationship_and_a_tablePart`] red — and the
//!   relationship half turns `Package::validate` red too, which
//!   [`a_tablePart_naming_no_relationship_is_refused_by_the_packaging_layer`] pins independently.

use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;
use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_sml::{
    CellRange, CellReference, CellValue, DifferentialFormatSpec, TableStyleOrigin,
    TableStyleReferenceSpec, WorksheetTableSpec,
};
use mjx_xlsx::{SpreadsheetDefect, Workbook, XlsxError};

/// The fixture this suite is written against.
const FIXTURE: &str = "worksheet_tables.xlsx";

/// The fixture, opened.
fn workbook() -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(FIXTURE)).expect("the fixture opens")
}

/// A part name, or a panic naming it.
fn part(name: &str) -> PartName {
    PartName::new(name).unwrap_or_else(|error| panic!("{name} is a part name: {error}"))
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// A cell reference, or a panic naming it.
fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text} is a reference: {error}"))
}

/// One part's decompressed bytes out of a saved container.
fn saved_part(bytes: &[u8], name: &str) -> Vec<u8> {
    let package = Package::open(bytes).expect("the container opens");
    package
        .part_bytes(&part(name))
        .unwrap_or_else(|| panic!("{name} is in the container"))
        .to_vec()
}

// -------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------

/// Both tables resolve to their parts, and the report carries what the ticket's surface names.
#[test]
fn a_sheets_tables_resolve_to_their_parts_and_report_their_columns() {
    let workbook = workbook();
    let tables = workbook.sheet_tables(0).expect("the tables resolve");
    assert_eq!(tables.len(), 2);

    let sales = tables
        .iter()
        .find(|table| table.display_name == "Sales")
        .expect("the Sales table");
    assert_eq!(sales.part, part("/xl/tables/table1.xml"));
    assert_eq!(sales.relationship_id, "rId1");
    assert_eq!(sales.id, 1);
    assert_eq!(sales.name.as_deref(), Some("Sales"));
    assert_eq!(sales.range, range("A1:D6"));
    assert_eq!(sales.header_row_count, 1);
    assert_eq!(sales.totals_row_count, 1);
    assert_eq!(sales.data_row_count(), Some(4));
    assert_eq!(sales.style_name.as_deref(), Some("TableStyleMedium2"));

    let names: Vec<&str> = sales
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(names, vec!["Region", "Q1", "Q2", "Total"]);

    let functions: Vec<Option<TotalsRowFunction>> = sales
        .columns
        .iter()
        .map(|column| column.totals_row_function)
        .collect();
    assert_eq!(
        functions,
        vec![
            None,
            Some(TotalsRowFunction::Sum),
            Some(TotalsRowFunction::Average),
            Some(TotalsRowFunction::CustomFormula),
        ],
        "two different functions and a custom one; one function repeated would test one path"
    );
    assert_eq!(
        sales.columns[3].calculated_column_formula.as_deref(),
        Some("IF(Sales[[#This Row],[Q1]]>500,Sales[[#This Row],[Q1]]+Sales[[#This Row],[Q2]],0)"),
        "a calculated column is text, and the structured reference in it is never parsed"
    );
    assert_eq!(
        sales.columns[3].totals_row_formula.as_deref(),
        Some("SUBTOTAL(109,Sales[Total])")
    );

    let regions = tables
        .iter()
        .find(|table| table.display_name == "Regions")
        .expect("the Regions table");
    assert_eq!(regions.part, part("/xl/tables/table2.xml"));
    assert_eq!(regions.id, 4);
    assert_eq!(regions.name, None);
    assert_eq!(regions.style_name.as_deref(), Some("AcmeBlue"));
    assert_eq!(regions.data_row_count(), Some(3));
}

/// The list is the **sheet's** order, not the relationship order, and the fixture makes them
/// disagree.
#[test]
fn a_sheets_tables_are_listed_in_the_sheets_own_order() {
    let workbook = workbook();
    let listed: Vec<String> = workbook
        .sheet_tables(0)
        .expect("resolved")
        .iter()
        .map(|table| table.part.as_str().to_owned())
        .collect();
    assert_eq!(
        listed,
        vec![
            "/xl/tables/table2.xml".to_owned(),
            "/xl/tables/table1.xml".to_owned()
        ],
        "`x:tableParts` lists rId2 then rId1, and that is the sheet's own claim"
    );

    let related: Vec<String> = workbook
        .worksheet(0)
        .expect("resolves")
        .expect("a worksheet")
        .parts()
        .tables
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect();
    assert_eq!(
        related,
        vec![
            "/xl/tables/table1.xml".to_owned(),
            "/xl/tables/table2.xml".to_owned()
        ],
        "the relationship order is the other one, which is what makes the case above discriminating"
    );
}

/// The markup door hands back the model and the interner it was parsed with.
#[test]
fn the_markup_door_reaches_everything_the_report_does_not_carry() {
    let workbook = workbook();
    let comment = workbook
        .table_markup(&part("/xl/tables/table1.xml"), |table, interner| {
            (
                table.table_type(interner).unwrap(),
                table.header_row_format_index(interner).unwrap(),
                table.auto_filter().is_some(),
                table.sort_state().is_some(),
            )
        })
        .expect("the part reads")
        .expect("it is an x:table");
    assert_eq!(
        comment,
        (
            mjx_ooxml_types::spreadsheetml::TableType::Worksheet,
            Some(0),
            true,
            true
        )
    );
}

/// **The built-in/local gate at the package tier.** A preset name is a preset, not a missing style.
///
/// Mutating [`Workbook::table_style_origin`] to report `Undefined` for a name the workbook does not
/// define turns this red on its first assertion.
#[test]
fn a_preset_style_name_is_reported_as_built_in_at_the_package_tier() {
    let workbook = workbook();
    assert_eq!(
        workbook.table_style_origin("TableStyleMedium2").unwrap(),
        TableStyleOrigin::BuiltIn,
        "the 144 presets are the application's; `TableStyleMedium2` is in no .xlsx at all"
    );
    assert_eq!(
        workbook.table_style_origin("AcmeBlue").unwrap(),
        TableStyleOrigin::LocallyDefined,
        "and the workbook's own style is defined right there in xl/styles.xml"
    );
    assert_eq!(
        workbook.table_style_origin("AcmeGreen").unwrap(),
        TableStyleOrigin::Undefined,
        "only a name that is neither falls back to a consumer's default"
    );

    // Both tables' style names go through the same call, so the two answers come from two different
    // files rather than from one lookup asked twice.
    let origins: Vec<TableStyleOrigin> = workbook
        .sheet_tables(0)
        .expect("resolved")
        .iter()
        .map(|table| {
            workbook
                .table_style_origin(table.style_name.as_deref().unwrap_or_default())
                .expect("the styles part reads")
        })
        .collect();
    assert_eq!(
        origins,
        vec![TableStyleOrigin::LocallyDefined, TableStyleOrigin::BuiltIn],
        "Regions is listed first and wears AcmeBlue; Sales wears the preset"
    );
}

/// A workbook with no `tableStyles` block at all still tells a preset from a name nothing defines.
///
/// Whether the file carries a table-style block has nothing to do with whether Excel supplies a
/// style. `Workbook::blank` writes no `tableStyles`.
#[test]
fn a_workbook_with_no_table_styles_still_recognises_a_preset() {
    let workbook = Workbook::blank().expect("a blank workbook");
    assert_eq!(
        workbook.table_style_origin("TableStyleLight9").unwrap(),
        TableStyleOrigin::BuiltIn
    );
    assert_eq!(
        workbook.table_style_origin("AcmeBlue").unwrap(),
        TableStyleOrigin::Undefined
    );
}

// -------------------------------------------------------------------------------------------
// Fidelity
// -------------------------------------------------------------------------------------------

/// Opening and saving leaves both table parts byte-identical.
#[test]
fn both_table_parts_survive_an_open_and_save_byte_for_byte() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let saved = workbook().save().expect("the fixture saves");
    for name in [
        "/xl/tables/table1.xml",
        "/xl/tables/table2.xml",
        "/xl/styles.xml",
        "/xl/worksheets/sheet1.xml",
    ] {
        assert_eq!(
            saved_part(&saved, name),
            saved_part(&original, name),
            "{name} changed on a save that edited nothing"
        );
    }
}

/// **Editing a cell inside a table leaves the table part byte-identical.**
///
/// The ticket's own *Done when* clause, and the concrete form of "never silently resize": `B2` is
/// inside `Sales`' `A1:D6`, and setting it neither moves the table's boundary nor rewrites its
/// column list. The table part is not even opened.
#[test]
fn editing_a_cell_inside_a_table_leaves_the_table_part_alone() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .set_cell_value(0, cell("B2"), CellValue::Number(9999.0))
        .expect("the store accepts the value");
    let saved = workbook.save().expect("saves");

    assert_eq!(
        saved_part(&saved, "/xl/tables/table1.xml"),
        saved_part(&original, "/xl/tables/table1.xml"),
        "the table over B2 did not move, resize, or re-flow"
    );
    assert_eq!(
        saved_part(&saved, "/xl/tables/table2.xml"),
        saved_part(&original, "/xl/tables/table2.xml")
    );
    let sheet = String::from_utf8(saved_part(&saved, "/xl/worksheets/sheet1.xml")).expect("UTF-8");
    assert!(
        sheet.contains("9999"),
        "the cell really was written: {sheet}"
    );
    assert!(
        sheet.contains(
            r#"<tableParts count="2"><tablePart r:id="rId2"/><tablePart r:id="rId1"/></tableParts>"#
        ),
        "and the sheet's own table list came back exactly as it was: {sheet}"
    );
}

/// Editing one table part re-flows that part and leaves the other one alone.
#[test]
fn editing_one_table_leaves_the_other_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let mut workbook = workbook();
    workbook
        .edit_table_markup(&part("/xl/tables/table2.xml"), |table, interner| {
            table.set_comment(interner, Some("regional lookup"));
            Ok(())
        })
        .expect("the edit lands");
    let saved = workbook.save().expect("saves");

    assert_eq!(
        saved_part(&saved, "/xl/tables/table1.xml"),
        saved_part(&original, "/xl/tables/table1.xml"),
        "table1 was not touched"
    );
    let edited = String::from_utf8(saved_part(&saved, "/xl/tables/table2.xml")).expect("UTF-8");
    assert!(edited.contains(r#"comment="regional lookup""#), "{edited}");
    assert!(
        edited.contains(
            r#"<xmlColumnPr mapId="1" xpath="/regions/region/@code" xmlDataType="string"/>"#
        ),
        "and everything the edit did not touch came back from the file's own bytes: {edited}"
    );
}

// -------------------------------------------------------------------------------------------
// Creating a table
// -------------------------------------------------------------------------------------------

/// **One past the highest, not the table count.** The fixture's ids are 1 and 4.
#[test]
fn the_next_free_table_id_is_one_past_the_highest_and_not_the_table_count() {
    let workbook = workbook();
    assert_eq!(
        workbook.next_table_id().expect("the tables read"),
        5,
        "the ids are 1 and 4: one past the highest is 5, and the table count plus one would be 3"
    );
    assert_eq!(
        Workbook::blank()
            .expect("blank")
            .next_table_id()
            .expect("no tables to read"),
        1,
        "§18.5.1.2 requires a non-zero id, so a workbook with no table starts at 1"
    );
}

/// **The four things a table is, written together.**
#[test]
fn adding_a_table_writes_a_part_a_content_type_a_relationship_and_a_tablepart() {
    let mut workbook = workbook();
    let mut spec = WorksheetTableSpec::new("Codes", range("F1:G4"), &["Code", "Name"]);
    spec.id = 99; // Ignored: only the package knows which ids are free.
    spec.style = Some(TableStyleReferenceSpec::named("TableStyleLight9"));

    let created = workbook.add_table(0, &spec).expect("the table is created");
    assert_eq!(
        created.id, 5,
        "the id is allocated from the package, never taken from the spec"
    );
    assert_eq!(created.part, part("/xl/tables/table3.xml"));
    assert_eq!(created.display_name, "Codes");
    assert_eq!(created.style_name.as_deref(), Some("TableStyleLight9"));

    workbook.validate().expect("the package is consistent");
    let saved = workbook.save().expect("saves");
    let package = Package::open(&saved).expect("the container opens");

    // 1. the part
    assert!(
        package.part_bytes(&created.part).is_some(),
        "the part is there"
    );
    // 2. the content type
    assert_eq!(
        package.content_type_of(&created.part),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"),
        "the content-type override is registered"
    );
    // 3. the relationship, from the *sheet* part
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    let rels = package
        .relationships_for(Some(&sheet_part))
        .expect("the sheet has relationships");
    let rel = rels
        .by_id(&created.relationship_id)
        .expect("the relationship this table was reached through");
    assert_eq!(
        rel.rel_type,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/table"
    );
    assert_eq!(rel.mode, TargetMode::Internal);
    assert_eq!(
        rel.target, "../tables/table3.xml",
        "the target is relative to the sheet part's own directory"
    );
    // 4. the sheet's claim on it
    let sheet = String::from_utf8(saved_part(&saved, "/xl/worksheets/sheet1.xml")).expect("UTF-8");
    assert!(
        sheet.contains(&format!(
            r#"<tablePart r:id="{}"/>"#,
            created.relationship_id
        )),
        "the sheet lists the new table: {sheet}"
    );
    assert!(
        sheet.contains(r#"<tableParts count="3">"#),
        "and the count it already declared was maintained: {sheet}"
    );

    // …and reading it back finds three tables, the new one last in the sheet's own order.
    let reopened = Workbook::open(&saved).expect("reopens");
    let tables = reopened.sheet_tables(0).expect("resolved");
    assert_eq!(tables.len(), 3);
    assert_eq!(tables[2].display_name, "Codes");
    assert_eq!(tables[2].id, 5);
}

/// Creating a table writes **no cell**.
///
/// The headings go into the table part, not into row 1 of the sheet. Writing them would be this
/// library overwriting values nobody asked to lose.
#[test]
fn adding_a_table_writes_no_cell() {
    let mut workbook = workbook();
    let before = workbook
        .cell_text(0, cell("A1"))
        .expect("readable")
        .expect("populated");
    let spec = WorksheetTableSpec::new("Codes", range("I1:J4"), &["Heading one", "Heading two"]);
    workbook.add_table(0, &spec).expect("created");

    assert_eq!(
        workbook.cell_text(0, cell("A1")).expect("readable"),
        Some(before),
        "the sheet's own headings are untouched"
    );
    assert_eq!(
        workbook.cell_text(0, cell("I1")).expect("readable"),
        None,
        "and the new table's headings were not written into the cells it covers"
    );
}

/// **The id gate.** Two added tables get two ids, and the workbook still validates.
///
/// Making [`Workbook::add_table`] reuse an existing id — answering 1, or 5 twice — turns this red
/// through [`SpreadsheetDefect::DuplicateTableId`], because the second added part is one this
/// library wrote and therefore one it is willing to fault.
#[test]
fn a_second_added_table_gets_a_second_id_and_the_workbook_still_validates() {
    let mut workbook = workbook();
    let first = workbook
        .add_table(
            0,
            &WorksheetTableSpec::new("Codes", range("I1:J4"), &["Code", "Name"]),
        )
        .expect("created");
    let second = workbook
        .add_table(
            0,
            &WorksheetTableSpec::new("Notes", range("L1:M4"), &["Note", "By"]),
        )
        .expect("created");

    // The validation comes **first**, because it is the assertion the ticket's mutation is aimed
    // at: an allocator that reused an existing id produces a package `validate` refuses, and a test
    // that checked the numbers before checking the package would report the wrong thing about why.
    workbook
        .validate()
        .expect("two tables, two ids, two display names");
    assert_eq!((first.id, second.id), (5, 6));
    assert_ne!(first.part, second.part);
    assert_ne!(first.relationship_id, second.relationship_id);

    let ids: Vec<u32> = workbook
        .sheet_tables(0)
        .expect("resolved")
        .iter()
        .map(|table| table.id)
        .collect();
    assert_eq!(
        ids,
        vec![4, 1, 5, 6],
        "and no existing table was renumbered"
    );
}

/// A duplicate `@id` a table this library wrote carries is refused by `validate`.
///
/// The direct form of the gate above: the collision is manufactured rather than waited for, so the
/// check is proved reachable rather than assumed to be.
#[test]
fn a_table_this_library_wrote_may_not_reuse_another_tables_id() {
    let mut workbook = workbook();
    workbook
        .edit_table_markup(&part("/xl/tables/table2.xml"), |table, interner| {
            table.set_id(interner, 1);
            Ok(())
        })
        .expect("the edit lands");

    let error = workbook.validate().expect_err("two tables now claim id 1");
    assert!(
        matches!(
            &error,
            XlsxError::InvalidWorkbook(defect)
                if matches!(
                    &**defect,
                    SpreadsheetDefect::DuplicateTableId { first_part, second_part, table_id }
                        if first_part == "/xl/tables/table1.xml"
                            && second_part == "/xl/tables/table2.xml"
                            && table_id == "1"
                )
        ),
        "got {error:?}"
    );
    workbook
        .save_unchecked()
        .expect("the escape hatch still writes it");
}

/// A duplicate `@displayName` is refused on the same terms.
#[test]
fn a_table_this_library_wrote_may_not_reuse_another_tables_display_name() {
    let mut workbook = workbook();
    workbook
        .edit_table_markup(&part("/xl/tables/table2.xml"), |table, interner| {
            table.set_display_name(interner, "Sales");
            Ok(())
        })
        .expect("the edit lands");

    let error = workbook
        .validate()
        .expect_err("two tables now answer to `Sales`");
    assert!(
        matches!(
            &error,
            XlsxError::InvalidWorkbook(defect)
                if matches!(&**defect, SpreadsheetDefect::DuplicateTableDisplayName { display_name, .. }
                    if display_name == "Sales")
        ),
        "got {error:?}"
    );
}

/// A workbook that **arrived** with two tables sharing an id still saves.
///
/// The asymmetry that makes the check safe: markup this library did not write is not ours to fault,
/// which is the rule `crates/mjx-xlsx/src/validate.rs` states for the sheet list and restates here.
#[test]
fn a_duplicate_id_a_file_arrived_with_is_not_this_librarys_to_fault() {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let mut package = Package::open(&bytes).expect("opens");
    // Replaced through the packaging layer, so the part is *raw* container bytes on the next open —
    // exactly the provenance a file read off disk has.
    let colliding = String::from_utf8(
        package
            .part_bytes(&part("/xl/tables/table2.xml"))
            .expect("there")
            .to_vec(),
    )
    .expect("UTF-8")
    .replace(r#"id="4""#, r#"id="1""#);
    package
        .replace_part_bytes(&part("/xl/tables/table2.xml"), colliding.into_bytes())
        .expect("accepted");
    let broken = package.save_unchecked().expect("written");

    Workbook::open(&broken)
        .expect("it opens")
        .save()
        .expect("and it saves: nothing here wrote that collision");
}

/// A `tablePart@r:id` that resolves to nothing is caught by the **packaging** layer.
///
/// Not restated in `mjx-xlsx`: it is a dangling relationship reference like any other, and
/// `Package::validate` reports it as `UndeclaredRelationshipReference` over exactly the parts this
/// library will write. This is the case that proves it really does cover `tablePart` — which is a
/// claim about `mjx-opc`'s walk reaching an element the sheet's own model put there, not about
/// anything in this crate.
#[test]
fn a_tablepart_naming_no_relationship_is_refused_by_the_packaging_layer() {
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    let broken = String::from_utf8(package.part_bytes(&sheet_part).expect("there").to_vec())
        .expect("UTF-8")
        .replace(r#"<tablePart r:id="rId1"/>"#, r#"<tablePart r:id="rId9"/>"#);
    assert!(
        broken.contains(r#"r:id="rId9""#),
        "the substitution has to have happened for this case to test anything"
    );
    // `replace_part_bytes` makes the part one this library will write, which is the scope every
    // markup check in this workspace is drawn at.
    package
        .replace_part_bytes(&sheet_part, broken.into_bytes())
        .expect("accepted");

    let workbook = Workbook::from_package(package).expect("the workbook still resolves");
    let error = workbook.validate().expect_err("rId9 is declared nowhere");
    let text = error.to_string();
    assert!(
        text.contains("rId9") && text.contains("tablePart"),
        "the defect names the element and the identifier: {text}"
    );
}

/// A `tablePart` whose relationship leads to something that is **not** a table is this crate's to
/// report — the direction packaging cannot see.
#[test]
fn a_tablepart_pointing_at_a_part_that_is_not_a_table_is_reported() {
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    let mut package = Package::open(&mjx_fixtures::fixture(FIXTURE)).expect("opens");
    // A real, reachable part of entirely the wrong kind, related from the sheet under the table
    // relationship type — which is exactly what packaging cannot fault: the edge resolves.
    package
        .add_relationship(
            Some(&sheet_part),
            Relationship {
                id: "rId3".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/table"
                        .to_owned(),
                target: "../styles.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("accepted");
    let broken = String::from_utf8(package.part_bytes(&sheet_part).expect("there").to_vec())
        .expect("UTF-8")
        .replace(r#"<tablePart r:id="rId1"/>"#, r#"<tablePart r:id="rId3"/>"#);
    package
        .replace_part_bytes(&sheet_part, broken.into_bytes())
        .expect("accepted");

    let workbook = Workbook::from_package(package).expect("the workbook still resolves");
    let error = workbook
        .validate()
        .expect_err("rId3 leads to the styles part");
    assert!(
        matches!(
            &error,
            XlsxError::InvalidWorkbook(defect)
                if matches!(
                    &**defect,
                    SpreadsheetDefect::TablePartTargetIsNotATable { target_part, .. }
                        if target_part == "/xl/styles.xml"
                )
        ),
        "got {error:?}"
    );
}

/// Adding a table to a sheet that lists none creates the `x:tableParts` element at its own rank.
#[test]
fn a_sheet_with_no_table_list_grows_one_at_rank_thirty_seven() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let created = workbook
        .add_table(
            0,
            &WorksheetTableSpec::new("Codes", range("A1:B4"), &["Code", "Name"]),
        )
        .expect("created");
    assert_eq!(created.id, 1);

    let saved = workbook.save().expect("saves");
    let sheet = String::from_utf8(saved_part(&saved, "/xl/worksheets/sheet1.xml")).expect("UTF-8");
    assert!(
        sheet.contains("<tableParts><tablePart"),
        "the element was created, and with no `@count` — a cache the file never declared is not \
         one this library adds: {sheet}"
    );
    assert!(
        sheet.find("<sheetData").unwrap() < sheet.find("<tableParts").unwrap(),
        "`sheetData` is rank 5 and `tableParts` rank 37: {sheet}"
    );
    assert!(
        sheet.contains(
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#
        ),
        "an authored worksheet declares only the SpreadsheetML namespace, so the binding a \
         `tablePart@r:id` needs was added — and only because there was none: {sheet}"
    );
}

// -------------------------------------------------------------------------------------------
// The `dxf` table the table styles share
// -------------------------------------------------------------------------------------------

/// Appending a `dxf` equal to one already held still yields a **new** index.
///
/// The corner of `DifferentialFormats`' index-identity contract that no test in the workspace
/// asserted before this one. It is in this file's path because a table style names `@dxfId` in eight
/// places, so a deduplicating allocator would silently repoint whichever region matched.
#[test]
fn appending_a_duplicate_differential_format_still_yields_a_new_index() {
    let mut workbook = workbook();
    let spec = DifferentialFormatSpec::default();

    let first = workbook
        .append_differential_format(&spec)
        .expect("appended");
    let second = workbook
        .append_differential_format(&spec)
        .expect("appended");
    assert_eq!(
        (first, second),
        (2, 3),
        "the fixture holds two dxfs already, and an identical append is still an append"
    );

    let saved = workbook.save().expect("saves");
    let styles = String::from_utf8(saved_part(&saved, "/xl/styles.xml")).expect("UTF-8");
    assert!(styles.contains(r#"<dxfs count="4">"#), "{styles}");
    assert!(
        styles.contains(r#"<tableStyleElement type="wholeTable" dxfId="0"/>"#),
        "and the table style's own indices still name what they named: {styles}"
    );
}
