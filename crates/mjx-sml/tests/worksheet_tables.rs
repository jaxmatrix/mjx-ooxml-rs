//! **MJXOFF-125's markup gate.** `CT_Table` and everything under it, the `tableStyles` block in
//! `xl/styles.xml`, and the three things this cluster must never do.
//!
//! # The fixture is authored to make specific wrong answers visible
//!
//! `tests/fixtures/worksheet_tables.xlsx` carries **two** tables on one sheet, because the ticket
//! names the trap directly: *"One table tests neither the id allocation nor the built-in/local
//! distinction."* The two differ in every way that matters:
//!
//! | | `/xl/tables/table1.xml` | `/xl/tables/table2.xml` |
//! |---|---|---|
//! | `@id` | **1** | **4** — a gap, so the next free id is 5 and not 3 |
//! | `@name` | `Sales` | *absent* — a different statement from `displayName` |
//! | style | `TableStyleMedium2` — **a preset, defined nowhere** | `AcmeBlue` — **defined in `styles.xml`** |
//! | totals row | one, with `sum`, `average` and `custom` | none |
//! | `tableColumns@count` | **9, against four columns** | 2, correct |
//! | own `autoFilter`/`sortState` | both | neither |
//! | `xmlColumnPr` | none | on the first column |
//! | `extLst` | on the table *and* on a column | none |
//!
//! Four further things are in it on purpose:
//!
//! * **the sheet lists the two tables in the reverse of the relationship order** — `rId2` then
//!   `rId1` — so a reader that answered with the relationship order rather than with the sheet's own
//!   `x:tableParts` produces a different list. `crates/mjx-xlsx/tests/worksheet_tables.rs` pins that
//!   half, which is where relationships exist at all;
//! * **the calculated-column formula spells `>` as `&gt;`**, which XML character data does not
//!   require. A writer that re-escaped the decoded text would emit `>` — the same string and
//!   different bytes — so the entity spelling is what says the stored children were replayed;
//! * **that formula is a structured reference** (`Sales[[#This Row],[Q1]]`), the notation this
//!   library carries and never parses;
//! * **`tableColumns@count` is stale**, so any write path that "corrected" the cache would change the
//!   file.
//!
//! # Why the re-emission tests are not byte-identity tests
//!
//! MJXOFF-120 and MJXOFF-123 both established it and it holds here: a part nobody edited is one
//! `extend_from_slice` of its own buffer, and after an edit *elsewhere* every other slot still writes
//! from its own stored bytes. **No byte-identity gate ever reaches this model's writer.** So
//! [`a_calculated_column_formula_read_from_a_file_replays_its_own_bytes`] forces the whole table to
//! be rebuilt from the model — `ToXml::to_xml` produces an element carrying no verbatim range
//! anywhere — and asserts on what comes out. That is the door a defect in
//! `TableFormula::as_raw_element` can be seen through, and the byte-identity suites cannot.
//!
//! MJXOFF-123 found that the **authored** path and the **read** path are two different code paths
//! through the same type, and that mutating one leaves the other's test green. Both are covered:
//! [`an_authored_table_formula_escapes_the_text_it_was_given`] is the first,
//! [`a_calculated_column_formula_read_from_a_file_replays_its_own_bytes`] the second.
//!
//! # Nothing here expands a calculated column, computes a total, filters, or sorts
//!
//! Every assertion below is about what the file *says*. None is about what a calculated column would
//! evaluate to, what a totals row would sum, which rows a filter would hide, or what order a sort
//! would produce — no call in this workspace can answer any of the five.

use mjx_ooxml_core::{Interner, ToXml};
use mjx_ooxml_types::spreadsheetml::{TableStyleType, TableType, TotalsRowFunction};
use mjx_opc::{Package, PartName};
use mjx_sml::{
    builtin_table_style_name, BuiltInTableStyleFamily, CellRange, SmlError, StylesheetPart,
    TableColumnSpec, TableFormula, TableStyleLookup, TableStyleOrigin, TableStyleReferenceSpec,
    WorksheetPart, WorksheetTable, WorksheetTableSpec,
};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "worksheet_tables.xlsx";

/// The bytes of one part of the fixture.
fn part_bytes(part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// One of the fixture's two table parts, read.
fn table(part: &str) -> (Interner, WorksheetTable) {
    let bytes = part_bytes(part);
    let document = mjx_xml::fidelity::parse(&bytes).expect("the part parses");
    let table = WorksheetTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an x:table");
    (document.interner, table)
}

/// The `Sales` table — `/xl/tables/table1.xml`.
fn sales() -> (Interner, WorksheetTable) {
    table("/xl/tables/table1.xml")
}

/// The `Regions` table — `/xl/tables/table2.xml`.
fn regions() -> (Interner, WorksheetTable) {
    table("/xl/tables/table2.xml")
}

/// The fixture's styles part, read.
fn styles() -> (Interner, StylesheetPart) {
    let bytes = part_bytes("/xl/styles.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the part parses");
    let part = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    (document.interner, part)
}

/// The fixture's worksheet, read.
fn sheet() -> WorksheetPart {
    WorksheetPart::read_part(&part_bytes("/xl/worksheets/sheet1.xml"))
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// A range, or a panic naming it.
fn range(text: &str) -> CellRange {
    CellRange::parse(text).unwrap_or_else(|error| panic!("{text} is a range: {error}"))
}

/// One element serialized to bytes, **rebuilt from the model** — no verbatim source range anywhere.
fn rebuilt(table: &WorksheetTable, interner: &mut Interner) -> String {
    let element = table.to_xml(interner);
    let mut out = Vec::new();
    mjx_xml::fidelity::serialize_element(&element, interner, None, &mut out);
    String::from_utf8(out).expect("UTF-8")
}

// -------------------------------------------------------------------------------------------
// The table itself
// -------------------------------------------------------------------------------------------

/// Both tables read, and they disagree in every way the fixture was built to make them disagree in.
///
/// One table would test neither the id allocation nor the built-in/local distinction, so this is the
/// case that fails when either is answered from a single example.
#[test]
fn the_two_tables_read_and_they_are_not_the_same_table_twice() {
    let (sales_interner, sales) = sales();
    let (regions_interner, regions) = regions();

    assert_eq!(sales.id(&sales_interner), Ok(1));
    assert_eq!(regions.id(&regions_interner), Ok(4));

    assert_eq!(
        sales.display_name(&sales_interner).as_deref(),
        Ok("Sales"),
        "`@displayName` is what a formula references the table by"
    );
    assert_eq!(
        regions.display_name(&regions_interner).as_deref(),
        Ok("Regions")
    );

    assert_eq!(
        sales.name(&sales_interner).unwrap().as_deref(),
        Some("Sales"),
        "`@name` and `@displayName` are separate attributes and this table writes both"
    );
    assert_eq!(
        regions.name(&regions_interner).unwrap(),
        None,
        "an absent `@name` says `it defaults to displayName`, which writing it out would not"
    );

    assert_eq!(sales.range(&sales_interner), Ok(range("A1:D6")));
    assert_eq!(regions.range(&regions_interner), Ok(range("F1:G4")));

    assert_eq!(
        sales.table_type(&sales_interner),
        Ok(TableType::Worksheet),
        "neither table writes `@tableType`, and the schema default is `worksheet`"
    );
}

/// A table's `@ref` includes its header and totals rows, and the data rows are what is left.
///
/// §18.5.1.2: *"The reference shall include the totals row if it is shown."* `A1:D6` with one header
/// row and one totals row is four data rows, not six and not five.
#[test]
fn the_range_includes_the_header_and_totals_rows() {
    let (interner, sales) = sales();
    assert_eq!(sales.header_row_count(&interner), Ok(1));
    assert_eq!(sales.totals_row_count(&interner), Ok(1));
    assert_eq!(sales.data_row_count(&interner).unwrap(), Some(4));

    let (interner, regions) = regions();
    assert_eq!(regions.header_row_count(&interner), Ok(1));
    assert_eq!(
        regions.totals_row_count(&interner),
        Ok(0),
        "the table writes no `@totalsRowCount`, and the schema default is 0"
    );
    assert_eq!(regions.data_row_count(&interner).unwrap(), Some(3));
}

/// The eight `@dxfId`s a table and its columns may name are read as the positions they are.
#[test]
fn the_differential_format_indices_are_read_as_positions() {
    let (interner, sales) = sales();
    assert_eq!(sales.header_row_format_index(&interner), Ok(Some(0)));
    assert_eq!(sales.data_format_index(&interner), Ok(Some(1)));
    assert_eq!(
        sales.totals_row_format_index(&interner),
        Ok(None),
        "an absent `@dxfId` is absent, not zero — zero is a real position in `dxfs`"
    );
    assert_eq!(sales.table_border_format_index(&interner), Ok(None));
}

// -------------------------------------------------------------------------------------------
// Columns, the totals row, and the formulas
// -------------------------------------------------------------------------------------------

/// The four columns read, with **two different** totals-row functions plus a custom one.
///
/// Two different functions rather than one repeated: a model that answered with the first column's
/// function for every column would pass a fixture where they all agreed.
#[test]
fn the_totals_row_carries_two_different_functions_and_a_custom_one() {
    let (interner, sales) = sales();
    let columns: Vec<_> = sales.table_columns().collect();
    assert_eq!(columns.len(), 4);

    let names: Vec<String> = columns
        .iter()
        .map(|column| column.name(&interner).unwrap().into_owned())
        .collect();
    assert_eq!(names, vec!["Region", "Q1", "Q2", "Total"]);

    let functions: Vec<TotalsRowFunction> = columns
        .iter()
        .map(|column| column.totals_row_function(&interner).unwrap())
        .collect();
    assert_eq!(
        functions,
        vec![
            TotalsRowFunction::None,
            TotalsRowFunction::Sum,
            TotalsRowFunction::Average,
            TotalsRowFunction::CustomFormula,
        ],
        "the wire tokens are `sum`, `average` and `custom`; the generated names are not derived \
         from them and are read rather than spelled"
    );

    assert_eq!(
        columns[0].totals_row_label(&interner).unwrap().as_deref(),
        Some("Total"),
        "the leftmost totals cell carries a label rather than an aggregate"
    );
}

/// A calculated-column formula is text, and a structured reference inside it is never parsed.
#[test]
fn a_calculated_column_formula_is_carried_as_text() {
    let (interner, sales) = sales();
    let total = sales.table_columns().nth(3).expect("a fourth column");

    let formula = total
        .calculated_column_formula()
        .expect("the column writes one");
    assert_eq!(
        formula.text(),
        "IF(Sales[[#This Row],[Q1]]>500,Sales[[#This Row],[Q1]]+Sales[[#This Row],[Q2]],0)",
        "the text comes back with its entity references decoded and nothing else done to it"
    );
    assert_eq!(
        formula.is_array(&interner),
        Ok(false),
        "`CT_TableFormula@array` defaults to false and this file writes none"
    );

    let totals = total.totals_row_formula().expect("the column writes one");
    assert_eq!(totals.text(), "SUBTOTAL(109,Sales[Total])");

    assert!(
        sales
            .table_columns()
            .take(3)
            .all(|column| column.calculated_column_formula().is_none()),
        "only the fourth column is calculated; a model that answered with the first formula it \
         found for every column would pass a fixture with one formula in it"
    );
}

/// `@count` says nine and there are four, and reading changes neither.
///
/// A producer's cache is the producer's. Correcting one nobody asked about is the repair this phase
/// keeps refusing.
#[test]
fn the_stale_column_count_is_left_exactly_as_the_file_wrote_it() {
    let (mut interner, sales) = sales();
    let columns = sales.columns().expect("the table writes tableColumns");
    assert_eq!(columns.declared_count(&interner), Ok(Some(9)));
    assert_eq!(columns.len(), 4);

    assert!(
        rebuilt(&sales, &mut interner).contains(r#"<tableColumns count="9">"#),
        "even re-emitted from the model, the cache is what the file said"
    );
}

/// `x:xmlColumnPr` is preserved with all three of its required attributes.
#[test]
fn the_xml_map_binding_on_a_column_is_preserved() {
    let (interner, regions) = regions();
    let code = regions.table_columns().next().expect("a first column");
    let binding = code
        .xml_column_properties()
        .expect("the column writes xmlColumnPr");

    assert_eq!(binding.map_id(&interner), Ok(1));
    assert_eq!(
        binding.xpath(&interner).as_deref(),
        Ok("/regions/region/@code")
    );
    assert_eq!(
        binding.xml_data_type(&interner).as_deref(),
        Ok("string"),
        "`ST_XmlDataType` is an unrestricted `xsd:string`, so this stays text"
    );
    assert_eq!(binding.is_denormalized(&interner), Ok(false));

    assert!(regions
        .table_columns()
        .nth(1)
        .expect("a second column")
        .xml_column_properties()
        .is_none());
}

/// The `extLst` on the table and the one on a column both come back, in position.
#[test]
fn the_extension_lists_on_a_table_and_on_a_column_survive_a_rebuild() {
    let (mut interner, sales) = sales();
    let markup = rebuilt(&sales, &mut interner);
    assert!(
        markup.contains(r#"<x14:table altText="Quarterly sales"/>"#),
        "the table's own extension is in the unknown bucket: {markup}"
    );
    assert!(
        markup.contains("<x14:tableColumn/>"),
        "so is the column's: {markup}"
    );
    assert!(
        markup.contains("</tableColumns><tableStyleInfo"),
        "and the table's `extLst` is still after `tableStyleInfo`, at rank 4: {markup}"
    );
}

// -------------------------------------------------------------------------------------------
// The two doors a formula's bytes can come out of
// -------------------------------------------------------------------------------------------

/// A formula read from a file **replays its own bytes**, entity spellings included.
///
/// This is the gate a defect in `TableFormula::as_raw_element` can be seen through, and no
/// byte-identity suite can: an unedited part is one `memcpy`, so the model's writer is never reached
/// there. Here the whole table is rebuilt from the model, which is the only way to reach it.
///
/// `&gt;` needs no escaping in XML character data. A writer that re-escaped the decoded text would
/// emit `>` — the same string, different bytes, and a part that no longer matches the one it was
/// read from.
#[test]
fn a_calculated_column_formula_read_from_a_file_replays_its_own_bytes() {
    let (mut interner, sales) = sales();
    let markup = rebuilt(&sales, &mut interner);
    assert!(
        markup.contains("Sales[[#This Row],[Q1]]&gt;500"),
        "the stored children are replayed, so the file's own entity spelling survives: {markup}"
    );
    assert!(
        !markup.contains("Sales[[#This Row],[Q1]]>500"),
        "re-escaping the decoded text would have produced a bare `>`: {markup}"
    );
}

/// The **authored** path is a different path, and it escapes what it is given.
///
/// MJXOFF-123 found that mutating one of these two paths leaves the other's test green, so both are
/// pinned. Here nothing was read, so there are no stored children to replay and the text is escaped
/// exactly once.
#[test]
fn an_authored_table_formula_escapes_the_text_it_was_given() {
    let mut interner = Interner::default();
    let formula = TableFormula::new(
        &mut interner,
        None,
        "calculatedColumnFormula",
        "IF(A1<2,\"a & b\",0)",
    );
    let mut out = Vec::new();
    mjx_xml::fidelity::serialize_element(&formula.as_raw_element(), &interner, None, &mut out);
    let markup = String::from_utf8(out).expect("UTF-8");
    assert!(
        markup.contains("IF(A1&lt;2,\"a &amp; b\",0)"),
        "an authored formula's `<` and `&` are escaped, because they must be: {markup}"
    );
    assert_eq!(formula.text(), "IF(A1<2,\"a & b\",0)");
}

/// Replacing a formula's text gives up the stored children, and only then.
#[test]
fn replacing_a_formulas_text_gives_up_the_bytes_it_was_keeping() {
    let (mut interner, sales) = sales();
    let mut sales = sales;
    sales
        .columns_mut()
        .expect("tableColumns")
        .column_mut(3)
        .expect("a fourth column")
        .calculated_column_formula_mut()
        .expect("a formula")
        .set_text("SUM(Sales[Q1])");

    let markup = rebuilt(&sales, &mut interner);
    assert!(markup.contains("<calculatedColumnFormula>SUM(Sales[Q1])</calculatedColumnFormula>"));
    assert!(!markup.contains("&gt;500"));
}

// -------------------------------------------------------------------------------------------
// The autofilter and the sort state are MJXOFF-123's, and there are three distinct slots
// -------------------------------------------------------------------------------------------

/// A table's `autoFilter` and `sortState` are the same complex types the worksheet's are, and they
/// are **not** the worksheet's.
///
/// The fixture makes the three `sortState` slots tell each other apart: the sheet writes none, the
/// table's autofilter writes none, and the table writes one. A model that conflated any two of them
/// answers differently here.
#[test]
fn the_tables_own_filter_and_sort_are_distinct_from_the_sheets() {
    let sheet = sheet();
    assert!(
        sheet.auto_filter().is_none(),
        "the sheet writes no autoFilter"
    );
    assert!(
        sheet.sort_state().is_none(),
        "and no sheet-level sortState at rank 11 either"
    );

    let (interner, sales) = sales();
    let filter = sales.auto_filter().expect("the table writes one");
    assert_eq!(filter.range(&interner), Ok(Some(range("A1:D5"))));
    assert_eq!(filter.column_count(), 1);
    assert!(
        filter.sort_state().is_none(),
        "`CT_AutoFilter`'s own rank-1 sortState is a third slot again, and this file writes none"
    );

    let sort = sales
        .sort_state()
        .expect("the table writes one at its rank 1");
    assert_eq!(sort.range(&interner), Ok(range("A2:D5")));
    assert_eq!(sort.len(), 1);

    let (interner, regions) = regions();
    assert!(regions.auto_filter().is_none());
    assert!(regions.sort_state().is_none());
    let _ = interner;
}

/// A filter records; it never hides. A sort records; it never reorders.
///
/// Restated here because a table is a second door onto the same two types, and the rule has to hold
/// at every door. Row 5 of the fixture is inside the filtered range and carries values the filter
/// excludes; nothing about it changes.
#[test]
fn reading_a_tables_filter_hides_no_row_and_reorders_nothing() {
    let before = part_bytes("/xl/worksheets/sheet1.xml");
    let sheet = sheet();
    let (interner, sales) = sales();
    let _ = sales.auto_filter().expect("a filter");
    let _ = sales.sort_state().expect("a sort");
    let _ = interner;

    assert_eq!(
        sheet.to_markup(),
        before,
        "reading a table's filter and sort touched no row of the sheet — and the sheet part is one \
         `memcpy` of its own bytes, because nothing edited it"
    );
}

// -------------------------------------------------------------------------------------------
// Never silently resize
// -------------------------------------------------------------------------------------------

/// `@ref`, `@headerRowCount` and `@totalsRowCount` move together.
///
/// There is deliberately no `set_range`: §18.5.1.2 makes the three one statement, so changing the
/// range alone would leave a table whose totals row is outside itself.
#[test]
fn resizing_writes_the_range_and_both_row_counts_together() {
    let (interner, sales) = sales();
    let mut interner = interner;
    let mut sales = sales;

    sales
        .resize(&mut interner, range("A1:D12"), 1, 2)
        .expect("the geometry fits");

    assert_eq!(sales.range(&interner), Ok(range("A1:D12")));
    assert_eq!(sales.header_row_count(&interner), Ok(1));
    assert_eq!(sales.totals_row_count(&interner), Ok(2));
    assert_eq!(sales.data_row_count(&interner).unwrap(), Some(9));

    let markup = rebuilt(&sales, &mut interner);
    assert!(markup.contains(r#"ref="A1:D12""#), "{markup}");
    assert!(markup.contains(r#"totalsRowCount="2""#), "{markup}");
}

/// A geometry that does not fit is refused, and refusing changes nothing.
#[test]
fn a_geometry_that_does_not_fit_is_refused_rather_than_written() {
    let (interner, sales) = sales();
    let mut interner = interner;
    let mut sales = sales;

    let error = sales
        .resize(&mut interner, range("A1:D3"), 2, 2)
        .expect_err("three rows cannot hold two header rows and two totals rows");
    assert!(
        matches!(
            &error,
            SmlError::TableGeometryDoesNotFit { range, header_rows, totals_rows }
                if range == "A1:D3" && *header_rows == 2 && *totals_rows == 2
        ),
        "got {error:?}"
    );

    assert_eq!(
        sales.range(&interner),
        Ok(range("A1:D6")),
        "the refusal left every one of the three attributes as it was"
    );
    assert_eq!(sales.header_row_count(&interner), Ok(1));
    assert_eq!(sales.totals_row_count(&interner), Ok(1));
}

/// Reading and re-emitting a table moves no boundary.
///
/// The other half of "never silently resize": nothing about opening a table, walking its columns and
/// writing it back adjusts `@ref` to the columns it found — even though this table's
/// `tableColumns@count` disagrees with them.
#[test]
fn walking_a_tables_columns_does_not_adjust_its_range() {
    let (mut interner, sales) = sales();
    let _: Vec<_> = sales.table_columns().collect();
    let markup = rebuilt(&sales, &mut interner);
    assert!(markup.contains(r#"ref="A1:D6""#), "{markup}");
    assert!(markup.contains(r#"headerRowCount="1""#), "{markup}");
    assert!(markup.contains(r#"totalsRowCount="1""#), "{markup}");
}

// -------------------------------------------------------------------------------------------
// `tableStyles`: a built-in name is not a missing style
// -------------------------------------------------------------------------------------------

/// The styles part's ninth slot is modelled, and the workbook defines exactly one style of its own.
#[test]
fn the_table_styles_block_is_modelled_and_holds_the_workbooks_own_style() {
    let (interner, part) = styles();
    let table_styles = part.table_styles().expect("the fixture writes tableStyles");

    assert_eq!(table_styles.declared_count(&interner), Ok(Some(1)));
    assert_eq!(
        table_styles
            .default_table_style(&interner)
            .unwrap()
            .as_deref(),
        Some("TableStyleMedium2"),
        "the workbook's preferred default is a preset it does not define"
    );
    assert_eq!(
        table_styles
            .default_pivot_style(&interner)
            .unwrap()
            .as_deref(),
        Some("PivotStyleLight16")
    );

    let acme = table_styles.styles().next().expect("one tableStyle");
    assert_eq!(acme.name(&interner).as_deref(), Ok("AcmeBlue"));
    assert_eq!(
        acme.offered_for_pivot_tables(&interner),
        Ok(false),
        "`@pivot` defaults to true and this style says otherwise"
    );
    assert_eq!(
        acme.offered_for_tables(&interner),
        Ok(true),
        "`@table` defaults to true and the style writes no attribute"
    );

    let regions: Vec<(TableStyleType, Option<u32>, u32)> = acme
        .regions()
        .map(|region| {
            (
                region.region(&interner).unwrap(),
                region.differential_format_index(&interner).unwrap(),
                region.band_size(&interner).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        regions,
        vec![
            (TableStyleType::WholeTable, Some(0), 1),
            (TableStyleType::FirstRowStripe, Some(1), 1),
        ],
        "`@size` defaults to 1, and both regions name a position in `dxfs`"
    );
}

/// **The gate the ticket asks for.** A preset name is reported as a preset, a defined style as
/// defined, and only a third kind of name as neither.
///
/// Reporting "style not found" for `TableStyleMedium2` would be the same class of error this phase
/// keeps refusing: a confident wrong answer where the honest one is a distinction. The 144 presets
/// are in no `.xlsx` at all.
///
/// One table would not test this, which is why the fixture has two: one wears a preset and the other
/// wears the workbook's own style.
#[test]
fn a_built_in_style_name_is_reported_as_built_in_and_not_as_missing() {
    let (interner, part) = styles();
    let table_styles = part.table_styles().expect("tableStyles");

    let (sales_interner, sales) = sales();
    let sales_style = sales
        .style()
        .expect("the table names a style")
        .name(&sales_interner)
        .unwrap()
        .expect("a name")
        .into_owned();
    assert_eq!(sales_style, "TableStyleMedium2");

    match table_styles.lookup(&interner, &sales_style).unwrap() {
        TableStyleLookup::BuiltIn(builtin) => {
            assert_eq!(builtin.to_string(), "TableStyleMedium2");
            assert_eq!(builtin.family(), BuiltInTableStyleFamily::TableMedium);
            assert_eq!(builtin.number(), 2);
            assert!(!builtin.family().is_pivot());
        }
        other => panic!("a preset must not be reported as missing: {other:?}"),
    }

    let (regions_interner, regions) = regions();
    let regions_style = regions
        .style()
        .expect("the table names a style")
        .name(&regions_interner)
        .unwrap()
        .expect("a name")
        .into_owned();
    assert_eq!(regions_style, "AcmeBlue");

    match table_styles.lookup(&interner, &regions_style).unwrap() {
        TableStyleLookup::LocallyDefined(style) => {
            assert_eq!(style.name(&interner).as_deref(), Ok("AcmeBlue"));
            assert_eq!(style.len(), 2);
        }
        other => panic!("a locally-defined style must be reported as defined: {other:?}"),
    }

    assert_eq!(
        table_styles.lookup(&interner, "AcmeGreen").unwrap(),
        TableStyleLookup::Undefined,
        "a name that is neither defined here nor a preset is the one case §18.5.1.5's default \
         applies to"
    );

    assert_eq!(
        [
            table_styles
                .lookup(&interner, "TableStyleMedium2")
                .unwrap()
                .origin(),
            table_styles.lookup(&interner, "AcmeBlue").unwrap().origin(),
            table_styles
                .lookup(&interner, "AcmeGreen")
                .unwrap()
                .origin(),
        ],
        [
            TableStyleOrigin::BuiltIn,
            TableStyleOrigin::LocallyDefined,
            TableStyleOrigin::Undefined
        ]
    );
}

/// A workbook may define a style whose name collides with a preset, and its own definition wins.
///
/// The check that the three-way answer is not a lookup order somebody could get backwards.
#[test]
fn a_locally_defined_style_named_after_a_preset_is_reported_as_locally_defined() {
    let markup = concat!(
        r#"<tableStyles xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1">"#,
        r#"<tableStyle name="TableStyleMedium2"><tableStyleElement type="headerRow" dxfId="3"/></tableStyle>"#,
        "</tableStyles>"
    );
    let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("parses");
    let styles = <mjx_sml::TableStyles as mjx_ooxml_core::FromXml>::from_xml(
        &document.root,
        &document.interner,
    )
    .expect("the element matches CT_TableStyles");

    assert!(
        matches!(
            styles
                .lookup(&document.interner, "TableStyleMedium2")
                .unwrap(),
            TableStyleLookup::LocallyDefined(_)
        ),
        "the file's own definition is what a consumer uses, preset name or not"
    );
    assert!(builtin_table_style_name("TableStyleMedium2").is_some());
}

// -------------------------------------------------------------------------------------------
// The worksheet's own `tableParts` slot
// -------------------------------------------------------------------------------------------

/// The sheet lists its tables in **document order**, which the fixture makes disagree with the
/// relationship order, and holds each identifier as the string the file wrote.
#[test]
fn the_sheet_lists_its_tables_in_document_order_and_resolves_nothing() {
    let sheet = sheet();
    let prefix = sheet.relationship_prefix();
    let list = sheet.table_parts().expect("the sheet writes tableParts");

    assert_eq!(list.declared_count(sheet.interner()), Ok(Some(2)));
    let ids: Vec<String> = list
        .parts()
        .map(|part| {
            part.relationship_id(sheet.interner(), prefix)
                .unwrap()
                .expect("an r:id")
        })
        .collect();
    assert_eq!(
        ids,
        vec!["rId2".to_owned(), "rId1".to_owned()],
        "the sheet's own order is `rId2` then `rId1`; the sheet part's `.rels` declares them the \
         other way round, and this crate has never heard of a `.rels`"
    );
}

// -------------------------------------------------------------------------------------------
// Authoring
// -------------------------------------------------------------------------------------------

/// A spec builds a whole table, in `CT_Table`'s own order, whatever order the pieces were stated in.
#[test]
fn an_authored_table_comes_out_in_schema_order() {
    let mut interner = Interner::default();
    let mut spec = WorksheetTableSpec::new("Inventory", range("A1:C4"), &["Item", "Count", "Note"]);
    spec.id = 7;
    spec.totals_row_count = 1;
    spec.header_row_count = 1;
    spec.style = Some(TableStyleReferenceSpec::named("TableStyleLight9"));
    spec.columns[1].totals_row_function = Some(TotalsRowFunction::CountNumbers);
    spec.columns[2].calculated_column_formula = Some("Inventory[[#This Row],[Item]]".to_owned());

    let table = spec
        .build(&mut interner, None)
        .expect("the spec is buildable");
    let markup = rebuilt(&table, &mut interner);

    assert!(markup.contains(r#"id="7""#), "{markup}");
    assert!(markup.contains(r#"ref="A1:C4""#), "{markup}");
    assert!(markup.contains(r#"totalsRowCount="1""#), "{markup}");
    assert!(
        markup.find("<tableColumns").unwrap() < markup.find("<tableStyleInfo").unwrap(),
        "`tableColumns` is rank 2 and `tableStyleInfo` rank 3: {markup}"
    );
    assert!(
        markup.contains(r#"<tableColumns count="3">"#),
        "an authored list declares its own count, so later pushes maintain it: {markup}"
    );
    assert!(
        markup.contains(r#"totalsRowFunction="countNums""#),
        "the wire token for `CountNumbers` is `countNums`: {markup}"
    );
}

/// A spec with no column is refused: `CT_TableColumns` declares `tableColumn` `minOccurs="1"`.
#[test]
fn a_table_with_no_column_is_refused_at_the_door() {
    let mut interner = Interner::default();
    let spec = WorksheetTableSpec::new("Empty", range("A1:A1"), &[]);
    let error = spec
        .build(&mut interner, None)
        .expect_err("a table with no column is markup no consumer will load");
    assert!(
        matches!(&error, SmlError::TableHasNoColumns { display_name } if display_name == "Empty"),
        "got {error:?}"
    );
}

/// A spec whose header and totals rows do not fit its range is refused before anything is written.
#[test]
fn an_authored_table_whose_geometry_does_not_fit_is_refused() {
    let mut interner = Interner::default();
    let mut spec = WorksheetTableSpec::new("Tight", range("A1:B2"), &["A", "B"]);
    spec.header_row_count = 1;
    spec.totals_row_count = 2;
    let error = spec
        .build(&mut interner, None)
        .expect_err("two rows, three reserved");
    assert!(
        matches!(&error, SmlError::TableGeometryDoesNotFit { .. }),
        "got {error:?}"
    );
}

/// The authored part carries the namespace declaration its seed had.
///
/// The rule `crates/mjx-xlsx/src/blank.rs` states: a part authored on demand writes back a root that
/// was **read**, never one freshly constructed. `mjx-docx`'s `create_footnotes_part` lost a
/// declaration exactly this way and every footnote with it, with a green gate throughout.
#[test]
fn an_authored_table_part_declares_the_spreadsheetml_namespace() {
    let spec = WorksheetTableSpec::new("Inventory", range("A1:B3"), &["Item", "Count"]);
    let mut authored = mjx_sml::AuthoredTable::from_spec(&spec).expect("authored");
    let bytes = authored.to_part_bytes();
    let text = String::from_utf8(bytes.clone()).expect("UTF-8");

    assert!(
        text.contains(r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#),
        "{text}"
    );
    assert!(text.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));

    let document = mjx_xml::fidelity::parse(&bytes).expect("the authored part parses back");
    let table = WorksheetTable::read_part(&document)
        .expect("it reads")
        .expect("its root is an x:table");
    assert_eq!(
        table.display_name(&document.interner).as_deref(),
        Ok("Inventory")
    );
    assert_eq!(table.table_columns().count(), 2);
}

/// A column spec numbered by the caller keeps the number it was given.
#[test]
fn a_column_spec_states_its_own_id() {
    let mut interner = Interner::default();
    let spec = TableColumnSpec::new(17, "Late");
    let column = spec.build(&mut interner, None);
    assert_eq!(column.id(&interner), Ok(17));
    assert_eq!(column.name(&interner).as_deref(), Ok("Late"));
    assert_eq!(
        column.totals_row_function(&interner),
        Ok(TotalsRowFunction::None)
    );
}
