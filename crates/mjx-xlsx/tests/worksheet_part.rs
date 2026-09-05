//! **MJXOFF-102's package gate.** Opening a worksheet part through a [`Workbook`], editing one cell,
//! and writing it back without disturbing anything else.
//!
//! `crates/mjx-sml/tests/worksheet_spine.rs` holds the markup half — the thirty-nine slots, the
//! generated-rank placement, the two `cols` blocks. This file holds the half that needs a package:
//! the part is reached through a relationship, a `t="s"` cell resolves through
//! `xl/sharedStrings.xml`, a chartsheet reports its kind without pretending to be a worksheet, and a
//! save leaves every *other* part byte-identical.
//!
//! # Why the edit-isolation assertions compare against the file
//!
//! The obvious shape — save, edit, save again, compare — passes with copy-on-write switched off,
//! because both sides would then be the same rebuild. So every isolation assertion here compares the
//! saved bytes against the **committed fixture's own** part bytes, and `worksheet_spine.xlsx` is
//! authored to carry two things no rebuild reproduces: a doubled space inside two start tags, and a
//! `headerFooter` whose character data is written with `&amp;` and `&quot;`. A writer that re-flowed
//! what it did not touch fails on both.

use mjx_opc::{Package, PartName, PartProvenance, Relationship, TargetMode};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::{SheetKind, Workbook};

/// The decompressed bytes of one part of a container.
fn part_bytes(container: &[u8], part: &str) -> Vec<u8> {
    let package = Package::open(container).expect("the container opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// Every part of `container`, as a sorted list of (name, bytes).
fn all_parts(container: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(container).expect("the container opens");
    let names: Vec<PartName> = package.part_names().collect();
    let mut parts: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|name| {
            (
                name.as_str().to_owned(),
                package
                    .part_bytes(name)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default(),
            )
        })
        .collect();
    parts.sort_by(|left, right| left.0.cmp(&right.0));
    parts
}

// -------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------

/// A worksheet part is reached through the sheet list, and reading it dirties nothing.
///
/// The provenance check is the one that matters: the real hazard in a copy-on-write design is a read
/// that quietly promotes a part to `Edited`, after which its bytes come from this project's writer
/// rather than from the container. Reading a worksheet must not even promote it to `Parsed`.
#[test]
fn reading_a_worksheet_reaches_its_cells_and_dirties_no_part() {
    let bytes = mjx_fixtures::fixture("worksheet_spine.xlsx");
    let workbook = Workbook::open(&bytes).expect("the workbook opens");
    assert_eq!(workbook.sheets().len(), 1);
    assert_eq!(workbook.sheets()[0].name, "Spine");
    assert_eq!(workbook.sheets()[0].kind, Some(SheetKind::Worksheet));

    let sheet = workbook
        .worksheet_markup(0)
        .expect("the part reads")
        .expect("the tab reaches an x:worksheet");
    assert_eq!(sheet.row_count(), 4);
    assert_eq!(sheet.cell_count(), 9);
    assert_eq!(
        sheet
            .cell(CellReference::parse("B3").expect("B3"))
            .expect("B3 is populated")
            .number(),
        Some(9.5)
    );
    assert_eq!(
        sheet
            .dimension()
            .expect("a dimension")
            .range(sheet.interner())
            .expect("ref")
            .text()
            .as_str(),
        "A1:D6"
    );

    for entry in workbook.package().entries() {
        assert_ne!(
            entry.provenance(),
            PartProvenance::Authored,
            "{} was authored by a read",
            entry.name
        );
        assert!(
            entry.bytes().is_some(),
            "{} lost its container bytes to a read",
            entry.name
        );
    }

    // And the whole container still round-trips.
    assert_eq!(
        all_parts(&workbook.save().expect("save")),
        all_parts(&bytes),
        "reading a worksheet must not change any part"
    );
}

/// A `t="s"` cell resolves through `xl/sharedStrings.xml`; an `inlineStr` resolves from its own
/// `<is>`; a number answers from its `<v>`.
///
/// The shared-string half is the only thing this tier adds to the markup model — an index into
/// another part is not something a crate with no packages can follow.
#[test]
fn a_shared_string_cell_resolves_through_the_string_table() {
    let bytes = mjx_fixtures::fixture("sample.xlsx");
    let workbook = Workbook::open(&bytes).expect("open");

    // `sample.xlsx` writes A1 as `t="s"` pointing at index 0.
    let sheet = workbook
        .worksheet_markup(0)
        .expect("read")
        .expect("a sheet");
    let a1 = sheet
        .cell(CellReference::parse("A1").expect("A1"))
        .expect("A1 is populated");
    assert_eq!(a1.shared_string_index(), Some(0), "A1 is a shared string");

    assert_eq!(
        workbook
            .cell_text(0, CellReference::parse("A1").expect("A1"))
            .expect("resolved"),
        Some("name".to_owned()),
        "index 0 of sample.xlsx's string table is `name`, read from the part rather than guessed"
    );
    assert_eq!(
        workbook
            .cell_text(0, CellReference::parse("C2").expect("C2"))
            .expect("resolved"),
        Some("9.99".to_owned()),
        "a number answers from its own <v>"
    );
    assert_eq!(
        workbook
            .cell_text(0, CellReference::parse("Z99").expect("Z99"))
            .expect("no error"),
        None,
        "an unpopulated cell is absent, not empty"
    );

    // An `inlineStr` needs no other part at all.
    let inline = mjx_fixtures::fixture("worksheet_spine.xlsx");
    let spine = Workbook::open(&inline).expect("open");
    assert_eq!(
        spine
            .cell_text(0, CellReference::parse("A2").expect("A2"))
            .expect("resolved"),
        Some("North".to_owned())
    );
}

// -------------------------------------------------------------------------------------------
// Editing
// -------------------------------------------------------------------------------------------

/// **The edit-isolation clause.** Setting one cell leaves every other part, every other row and
/// every other worksheet child byte-identical.
#[test]
fn setting_one_cell_leaves_every_other_part_and_every_other_child_byte_identical() {
    let bytes = mjx_fixtures::fixture("worksheet_spine.xlsx");
    let original_sheet = part_bytes(&bytes, "/xl/worksheets/sheet1.xml");
    let original = all_parts(&bytes);

    let mut workbook = Workbook::open(&bytes).expect("open");
    workbook
        .set_cell_value(
            0,
            CellReference::parse("B2").expect("B2"),
            CellValue::Number(101.0),
        )
        .expect("B2 is inside the grid");
    let saved = workbook.save().expect("save");

    // Every part but the worksheet is untouched.
    let after = all_parts(&saved);
    assert_eq!(
        original.len(),
        after.len(),
        "the entry set must not change: {:?} -> {:?}",
        original.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        after.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    let mut differing = Vec::new();
    for ((name, before), (also, now)) in original.iter().zip(after.iter()) {
        assert_eq!(name, also, "the entry order must not change");
        if before != now {
            differing.push(name.clone());
        }
    }
    assert_eq!(
        differing,
        vec!["/xl/worksheets/sheet1.xml".to_owned()],
        "only the edited worksheet may differ"
    );

    // Inside the worksheet, everything but row 2 is the file's own bytes.
    let edited = part_bytes(&saved, "/xl/worksheets/sheet1.xml");
    let before = core::str::from_utf8(&original_sheet).expect("UTF-8");
    let now = core::str::from_utf8(&edited).expect("UTF-8");
    assert_ne!(before, now, "the edit has to have changed something");

    for fragment in [
        r#"<sheetFormatPr defaultColWidth="9.140625"  defaultRowHeight="15" outlineLevelRow="1"></sheetFormatPr>"#,
        r#"<cols><col min="1" max="2" width="14.5" customWidth="true"/></cols>"#,
        r#"<pageSetup paperSize="9"  orientation="landscape" fitToWidth="1" fitToHeight="0"/>"#,
        "<oddHeader>&amp;C&amp;&quot;Times New Roman,Regular&quot;&amp;12Spine</oddHeader>",
        r#"<tableParts count="1"><tablePart r:id="rId1"/></tableParts>"#,
        r#"<row r="1" spans="1:4">"#,
        r#"<c r="D3"><v>1</v><extLst>"#,
    ] {
        assert!(
            before.contains(fragment),
            "the fixture no longer carries {fragment} — the assertion below would be vacuous"
        );
        assert!(now.contains(fragment), "an edit to B2 disturbed {fragment}");
    }
    assert!(
        now.contains(r#"<c r="B2"><v>101</v></c>"#),
        "the edit landed"
    );
    assert!(
        !now.contains(r#"<c r="B2"><v>17</v></c>"#),
        "the old value is gone"
    );

    // The saved workbook still opens, and reads back what was written.
    let reopened = Workbook::open(&saved).expect("the saved workbook opens");
    assert_eq!(
        reopened
            .worksheet_markup(0)
            .expect("read")
            .expect("a sheet")
            .cell(CellReference::parse("B2").expect("B2"))
            .expect("B2")
            .number(),
        Some(101.0)
    );
}

/// Writing back a worksheet nobody edited changes nothing at all.
///
/// A model that re-flowed on the way out would fail here, and the fixture's doubled spaces and
/// entity spellings are what make "changes nothing" a claim about bytes rather than about intent.
#[test]
fn writing_back_an_unedited_worksheet_is_a_no_op() {
    let bytes = mjx_fixtures::fixture("worksheet_spine.xlsx");
    let mut workbook = Workbook::open(&bytes).expect("open");
    let sheet = workbook
        .worksheet_markup(0)
        .expect("read")
        .expect("a sheet");
    workbook
        .write_worksheet_markup(0, &sheet)
        .expect("the part is replaced");
    assert_eq!(
        all_parts(&workbook.save().expect("save")),
        all_parts(&bytes),
        "writing back an untouched model must reproduce the container"
    );
}

/// A cell written outside the recorded `dimension` widens it; the widened box is what a reopened
/// workbook reports.
#[test]
fn a_cell_outside_the_dimension_widens_it_in_the_saved_part() {
    let bytes = mjx_fixtures::fixture("worksheet_spine.xlsx");
    let mut workbook = Workbook::open(&bytes).expect("open");
    workbook
        .set_cell_value(
            0,
            CellReference::parse("F12").expect("F12"),
            CellValue::Number(7.0),
        )
        .expect("F12 is inside the grid");
    let saved = workbook.save().expect("save");
    let sheet = part_bytes(&saved, "/xl/worksheets/sheet1.xml");
    let text = core::str::from_utf8(&sheet).expect("UTF-8");
    assert!(
        text.contains(r#"<dimension ref="A1:F12"/>"#),
        "the cached box must be widened to contain a cell this library wrote: {text}"
    );
}

// -------------------------------------------------------------------------------------------
// Sheet kinds
// -------------------------------------------------------------------------------------------

/// A workbook containing a **chartsheet** opens without error, reports its kind, and reads as
/// `None` when asked for worksheet markup.
///
/// The package is built here rather than committed, and deliberately: a legal `CT_Chartsheet`
/// declares `drawing` `minOccurs="1"` (`sml.xsd:2965`), so a schema-valid chartsheet drags a
/// `dml-spreadsheetDrawing` part into the corpus — and that namespace has no arm in
/// `mjx_schema_gate::categories`, whose owner is MJXOFF-107 (E3). What is under test here is
/// **sheet-kind resolution**, which is a property of the part graph and the content type, so the
/// package is assembled in this file and never joins the fixture corpus.
#[test]
fn a_workbook_containing_a_chartsheet_opens_and_reports_its_kind() {
    let bytes = workbook_with_a_chartsheet();
    let workbook = Workbook::open(&bytes).expect("a workbook with a chartsheet opens");

    assert_eq!(workbook.sheets().len(), 2);
    assert_eq!(workbook.sheets()[0].name, "Grid");
    assert_eq!(workbook.sheets()[0].kind, Some(SheetKind::Worksheet));
    assert_eq!(workbook.sheets()[1].name, "Picture");
    assert_eq!(
        workbook.sheets()[1].kind,
        Some(SheetKind::Chartsheet),
        "the chartsheet must be distinguished, not guessed at"
    );

    // The worksheet reads; the chartsheet reads as `None` rather than as an error or an empty sheet.
    assert!(workbook.worksheet_markup(0).expect("read").is_some());
    assert!(
        workbook.worksheet_markup(1).expect("no error").is_none(),
        "a chartsheet is not an x:worksheet, and saying so is not a failure"
    );

    // The sheet handle resolves too, and the whole container round-trips.
    let sheet = workbook
        .worksheet(1)
        .expect("the chartsheet resolves")
        .expect("it reaches a part");
    assert_eq!(sheet.kind(), Some(SheetKind::Chartsheet));
    assert_eq!(sheet.part().as_str(), "/xl/chartsheets/sheet1.xml");
    assert_eq!(
        all_parts(&workbook.save().expect("save")),
        all_parts(&bytes)
    );
}

/// A **macrosheet** is not an ECMA-376 sheet kind, and this is what actually happens to one.
///
/// MJXOFF-102's ticket asks that "`CT_Chartsheet`, `CT_Dialogsheet` and `CT_Macrosheet` are
/// recognised as sheet kinds". The first two are; the third cannot be, and the reason is in the
/// schema rather than in this crate. `sml.xsd` declares global elements for `worksheet`
/// (`sml.xsd:2115`), `chartsheet` (`:2116`) and `dialogsheet` (`:2117`) — and **none for
/// `macrosheet`**. `CT_Macrosheet` (`sml.xsd:2118`) is a complex type nothing in Part 1 makes into a
/// part: there is no relationship type for one and no content type for one. The string real files
/// use is `application/vnd.ms-excel.macrosheet+xml`, which is the same `vnd.ms-excel` family
/// `crates/mjx-xlsx/src/parts.rs` has a written rule against inventing.
///
/// So [`SheetKind`] stays at three, and this case pins the behaviour a caller actually gets, so that
/// a later child changing it has to change a test rather than discover it:
///
/// * the workbook **opens**, which is the property the ticket was really after;
/// * the entry's `kind` is `None` and its part is `PartClassification::Unclassified`;
/// * a checked [`Workbook::save`] still writes it, byte for byte, because
///   `crates/mjx-xlsx/src/validate.rs` faults a sheet list only when *this library* wrote the
///   workbook part — a container somebody else wrote is preserved, not corrected.
#[test]
fn a_macrosheet_opens_is_reported_as_an_unknown_kind_and_still_round_trips() {
    let bytes = workbook_with_a_macrosheet();
    let workbook = Workbook::open(&bytes).expect("a workbook with a macrosheet opens");

    assert_eq!(workbook.sheets().len(), 2);
    assert_eq!(workbook.sheets()[1].name, "Macros");
    assert_eq!(
        workbook.sheets()[1].kind,
        None,
        "a macrosheet is not one of ECMA-376's three sheet kinds"
    );
    let inventory = workbook.part_inventory();
    let entry = inventory
        .iter()
        .find(|entry| entry.part.as_str() == "/xl/macrosheets/sheet1.xml")
        .expect("the macrosheet part is in the inventory");
    assert_eq!(
        entry.classification,
        mjx_xlsx::PartClassification::Unclassified
    );

    // A checked save still writes it, unchanged. `validate` faults a sheet list only for a workbook
    // part this library authored; the macrosheet's own part is preserved either way.
    assert_eq!(
        all_parts(&workbook.save().expect("a checked save writes")),
        all_parts(&bytes),
        "a workbook somebody else wrote is preserved, not corrected"
    );

    // The worksheet beside it still reads, so the unknown kind does not poison the sheet list.
    assert_eq!(
        workbook
            .worksheet_markup(0)
            .expect("read")
            .expect("tab 0 is a worksheet")
            .cell_count(),
        9
    );
    assert!(workbook.worksheet_markup(1).expect("no error").is_none());
}

/// A two-sheet workbook whose second tab is a macrosheet, assembled from `worksheet_spine.xlsx`.
fn workbook_with_a_macrosheet() -> Vec<u8> {
    const MACROSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<macrosheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></macrosheet>"#;

    let mut package = Package::open(&mjx_fixtures::fixture("worksheet_spine.xlsx")).expect("open");
    let macrosheet = PartName::new("/xl/macrosheets/sheet1.xml").expect("a part name");
    let workbook_part = PartName::new("/xl/workbook.xml").expect("a part name");

    package
        .insert_part(
            &macrosheet,
            "application/vnd.ms-excel.macrosheet+xml",
            MACROSHEET.to_vec(),
        )
        .expect("the macrosheet part is inserted");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rId9".to_owned(),
                rel_type: "http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet"
                    .to_owned(),
                target: "macrosheets/sheet1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the workbook reaches its macrosheet");
    rewrite_sheet_list(&mut package, "Macros", "rId9");
    package.save().expect("the assembled package saves")
}

/// A two-sheet workbook whose second tab is a chartsheet, assembled from `worksheet_spine.xlsx`.
///
/// The chartsheet's own body is MJXOFF-129's (D17) and its `drawing` target is MJXOFF-107's (E3);
/// both are here as the bytes a producer would have written, preserved and never modelled.
fn workbook_with_a_chartsheet() -> Vec<u8> {
    const CHARTSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetViews><sheetView zoomToFit="true" workbookViewId="0"/></sheetViews><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><drawing r:id="rId1"/></chartsheet>"#;
    const DRAWING: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;

    let mut package = Package::open(&mjx_fixtures::fixture("worksheet_spine.xlsx")).expect("open");
    let chartsheet = PartName::new("/xl/chartsheets/sheet1.xml").expect("a part name");
    let drawing = PartName::new("/xl/drawings/drawing1.xml").expect("a part name");
    let workbook_part = PartName::new("/xl/workbook.xml").expect("a part name");

    package
        .insert_part(
            &chartsheet,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml",
            CHARTSHEET.to_vec(),
        )
        .expect("the chartsheet part is inserted");
    package
        .insert_part(
            &drawing,
            "application/vnd.openxmlformats-officedocument.drawing+xml",
            DRAWING.to_vec(),
        )
        .expect("the drawing part is inserted");
    package
        .add_relationship(
            Some(&chartsheet),
            Relationship {
                id: "rId1".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
                        .to_owned(),
                target: "../drawings/drawing1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the chartsheet reaches its drawing");
    package
        .add_relationship(
            Some(&workbook_part),
            Relationship {
                id: "rId9".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet"
                        .to_owned(),
                target: "chartsheets/sheet1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the workbook reaches its chartsheet");

    rewrite_sheet_list(&mut package, "Picture", "rId9");
    package.save().expect("the assembled package saves")
}

/// Renames `worksheet_spine.xlsx`'s one tab to `Grid` and appends a second one.
///
/// The sheet list is the tab order and the only place it exists, so a part added to the package is
/// not a *tab* until the list names it.
fn rewrite_sheet_list(package: &mut Package, name: &str, relationship_id: &str) {
    let workbook_part = PartName::new("/xl/workbook.xml").expect("a part name");
    let document = package
        .part_tree_mut(&workbook_part)
        .expect("the workbook part parses");
    let text = String::from_utf8(mjx_xml::fidelity::serialize_to_vec(document)).expect("UTF-8");
    let with_both = text.replace(
        r#"<sheet name="Spine" sheetId="1" r:id="rId1"/>"#,
        &format!(
            r#"<sheet name="Grid" sheetId="1" r:id="rId1"/><sheet name="{name}" sheetId="2" r:id="{relationship_id}"/>"#
        ),
    );
    assert_ne!(with_both, text, "the sheet entry must have been rewritten");
    package
        .replace_part_bytes(&workbook_part, with_both.into_bytes())
        .expect("the workbook part is replaced");
}
