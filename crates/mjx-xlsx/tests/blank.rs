//! `Workbook::blank`, and the authoring surface beside it (MJXOFF-112).
//!
//! # What is asserted, and against what
//!
//! Every case here goes through **`save` then `open`**, and asserts on the reopened container. That
//! is deliberate and it is the rule `crates/mjx-xlsx/src/blank.rs` states: a defect in `mjx-docx`
//! dropped an `xmlns:w` declaration by writing a freshly constructed root over a parsed one, every
//! footnote vanished on the next open, and the gate stayed green throughout **because it asserted on
//! the model that had just been built**. Several cases below additionally assert on the raw bytes of
//! the reopened part, so that a change which merely made the reader more forgiving would not satisfy
//! them.
//!
//! The schema arm lives in `tests/schema_gate.rs`, with the rest of the ECMA-376 gate.

use mjx_ooxml_types::spreadsheetml::BorderStyle;
use mjx_opc::doc_props::{CoreProperties, DocumentTimestamp, ExtendedProperties};
use mjx_opc::{Package, PartName};
use mjx_sml::write::{
    BorderEdgeSpec, BorderSpec, CellFormatSpec, CellFormatTarget, PatternFillSpec,
};
use mjx_sml::{CellReference, CellValue, FontProperties, SharedStringTable, WorksheetPart};
use mjx_xlsx::{Workbook, XlsxError};

/// The SpreadsheetML namespace, as `sml.xsd` declares it.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A blank workbook, saved and reopened — which is the only state worth asserting on.
fn reopened_blank() -> Workbook {
    let bytes = Workbook::blank()
        .expect("a blank workbook is authored")
        .save()
        .expect("it saves");
    Workbook::open(&bytes).expect("it reopens")
}

/// The bytes of one part of a package.
fn part_text(package: &Package, name: &str) -> String {
    let part = PartName::new(name).expect("a literal part name");
    let bytes = package
        .part_bytes(&part)
        .unwrap_or_else(|| panic!("the package holds {name}"));
    String::from_utf8(bytes.to_vec()).expect("this library emits UTF-8")
}

// -------------------------------------------------------------------------------------------
// The blank workbook
// -------------------------------------------------------------------------------------------

/// A blank workbook opens, has the one tab the schema requires, and passes `Package::validate`
/// through `Workbook::save`.
///
/// `CT_Workbook` declares `sheets` `minOccurs="1"` and `CT_Sheets` declares `sheet`
/// `minOccurs="1"` — there is no schema-valid empty workbook — so `blank()` necessarily authors a
/// worksheet part as well. That is the assertion, not a nicety.
#[test]
fn a_blank_workbook_has_the_one_sheet_the_schema_requires() {
    let workbook = reopened_blank();
    assert_eq!(workbook.sheets().len(), 1);
    assert_eq!(workbook.sheets()[0].name, "Sheet1");
    assert!(workbook.sheets()[0].is_visible());
    assert_eq!(
        workbook.sheets()[0].part.as_ref().map(PartName::as_str),
        Some("/xl/worksheets/sheet1.xml"),
        "the tab's r:id resolves to a real part",
    );
    workbook.validate().expect("the invariants hold");
    workbook.save().expect("it saves again");
}

/// The part list is the one a workbook Office wrote carries, minus the theme.
///
/// The theme is **deliberately** absent: no schema or OPC rule requires one in a SpreadsheetML
/// package, `mjx_chart::EmbeddedWorkbook` has shipped without one, and authoring one here would put
/// a third hand-written `a:theme` in this workspace on the very child whose premise is that a
/// duplicated markup writer is a debt.
#[test]
fn a_blank_workbook_carries_the_parts_office_writes_bar_the_theme() {
    let workbook = reopened_blank();
    let mut names: Vec<String> = workbook
        .package()
        .part_names()
        .map(|part| part.as_str().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "/_rels/.rels",
            "/docProps/app.xml",
            "/docProps/core.xml",
            "/xl/_rels/workbook.xml.rels",
            "/xl/sharedStrings.xml",
            "/xl/styles.xml",
            "/xl/workbook.xml",
            "/xl/worksheets/sheet1.xml",
        ],
    );
    assert!(
        !names.iter().any(|name| name.contains("theme")),
        "no theme is authored, and that is a decision rather than an omission",
    );

    let parts = workbook.parts();
    assert!(parts.styles.is_some());
    assert!(parts.shared_strings.is_some());
    assert!(parts.theme.is_none());
}

/// Every authored part declares the SpreadsheetML namespace **in the reopened file's bytes**.
///
/// The exact property `mjx-docx`'s footnote defect broke. A reader that had been made more forgiving
/// would not satisfy this.
#[test]
fn every_authored_part_still_declares_its_namespace_after_a_round_trip() {
    let bytes = Workbook::blank().expect("authored").save().expect("saves");
    let package = Package::open(&bytes).expect("reopens");
    for name in [
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/sharedStrings.xml",
        "/xl/styles.xml",
    ] {
        let text = part_text(&package, name);
        assert!(
            text.contains(&format!(r#"xmlns="{SML_NS}""#)),
            "{name} lost its namespace declaration:\n{text}"
        );
    }
    assert!(part_text(&package, "/xl/workbook.xml").contains(
        r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#
    ));
}

/// Two blank workbooks are byte-identical containers. Nothing here reads a clock.
#[test]
fn blank_is_deterministic() {
    let first = Workbook::blank().expect("authored").save().expect("saves");
    let second = Workbook::blank().expect("authored").save().expect("saves");
    assert_eq!(first, second);
}

/// `blank_with_properties` writes what it is given into `docProps`, and nothing else moves.
#[test]
fn blank_with_properties_writes_the_two_document_property_parts() {
    let core = CoreProperties {
        title: Some("Quarterly".to_owned()),
        creator: Some("mjx-ooxml".to_owned()),
        created: Some(DocumentTimestamp::new(2026, 9, 5, 12, 0, 0).expect("a valid timestamp")),
        modified: None,
    };
    let extended = ExtendedProperties {
        application: Some("mjx-ooxml-rs".to_owned()),
    };
    let bytes = Workbook::blank_with_properties(&core, &extended)
        .expect("authored")
        .save()
        .expect("saves");
    let package = Package::open(&bytes).expect("reopens");

    let core_text = part_text(&package, "/docProps/core.xml");
    assert!(
        core_text.contains("<dc:title>Quarterly</dc:title>"),
        "{core_text}"
    );
    assert!(
        core_text.contains("<dc:creator>mjx-ooxml</dc:creator>"),
        "{core_text}"
    );
    assert!(part_text(&package, "/docProps/app.xml")
        .contains("<Application>mjx-ooxml-rs</Application>"),);
}

/// A blank workbook opened and saved again is byte-identical, part for part: reading models nothing
/// into a dirty state.
#[test]
fn opening_and_saving_a_blank_workbook_changes_no_part() {
    let first = Workbook::blank().expect("authored").save().expect("saves");
    let second = Workbook::open(&first)
        .expect("reopens")
        .save()
        .expect("saves");
    let (before, after) = (
        Package::open(&first).expect("opens"),
        Package::open(&second).expect("opens"),
    );
    for part in before.part_names() {
        assert_eq!(
            before.part_bytes(&part),
            after.part_bytes(&part),
            "{} changed on a no-op round trip",
            part.as_str(),
        );
    }
}

// -------------------------------------------------------------------------------------------
// The authoring surface
// -------------------------------------------------------------------------------------------

/// A workbook authored from nothing with numbers, strings, a boolean, an error value and a styled
/// cell round-trips through `save` → `open` with every value intact.
///
/// This is the ticket's Done-when, made literal.
#[test]
fn an_authored_workbook_round_trips_with_every_value_intact() {
    let mut workbook = Workbook::blank().expect("authored");

    let shared = workbook
        .intern_shared_string("North")
        .expect("the table interns");
    let bold = workbook
        .append_font(&FontProperties {
            font_name: Some("Calibri".to_owned()),
            size_in_points: Some(11.0),
            bold: Some(true),
            ..FontProperties::default()
        })
        .expect("the font is well-formed");
    let yellow = workbook
        .append_pattern_fill(&PatternFillSpec::solid("FFFF00"))
        .expect("the fill is appended");
    let boxed = workbook
        .append_border(&BorderSpec {
            top: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            bottom: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            ..BorderSpec::default()
        })
        .expect("the border is appended");
    let highlight = workbook
        .append_cell_format(
            CellFormatTarget::CellFormats,
            &CellFormatSpec {
                font_index: Some(bold),
                fill_index: Some(yellow),
                border_index: Some(boxed),
                applies_font: Some(true),
                applies_fill: Some(true),
                applies_border: Some(true),
                ..CellFormatSpec::skeleton_cell_format()
            },
        )
        .expect("the xf is appended");
    assert_eq!((bold, yellow, boxed, highlight), (1, 2, 1, 1));

    let at = |address: &str| CellReference::parse(address).expect("a literal address");
    for (address, value) in [
        ("A1", CellValue::SharedString(shared)),
        ("B1", CellValue::Number(19.25)),
        ("C1", CellValue::Boolean(true)),
        ("D1", CellValue::Error("#N/A")),
        ("E1", CellValue::InlineString("in the cell")),
    ] {
        workbook
            .set_cell_value(0, at(address), value)
            .expect("the store accepts the value");
    }
    workbook
        .set_cell_style(0, at("B1"), Some(highlight))
        .expect("the store accepts the style");

    let bytes = workbook.save().expect("saves");
    let reopened = Workbook::open(&bytes).expect("reopens");

    assert_eq!(
        reopened.cell_text(0, at("A1")).expect("readable"),
        Some("North".to_owned()),
        "a t=\"s\" cell resolves through the shared-string part",
    );
    let markup = reopened
        .worksheet_markup(0)
        .expect("readable")
        .expect("the tab holds a worksheet");
    assert_eq!(
        markup.cell(at("B1")).and_then(|cell| cell.number()),
        Some(19.25)
    );
    assert_eq!(
        markup.cell(at("C1")).and_then(|cell| cell.boolean()),
        Some(true)
    );
    assert_eq!(
        markup
            .cell(at("D1"))
            .and_then(|cell| cell.value().expect("decodable"))
            .as_deref(),
        Some("#N/A"),
    );
    assert_eq!(
        reopened.cell_text(0, at("E1")).expect("readable"),
        Some("in the cell".to_owned()),
    );
    assert_eq!(
        markup.cell(at("B1")).map(|cell| cell.style()),
        Some(highlight)
    );

    // The style index resolves to the record that was appended, aspect for aspect.
    let effective = reopened
        .effective_cell_format(0, at("B1"))
        .expect("resolvable")
        .expect("the workbook has styles and a worksheet");
    assert_eq!(effective.font().resource_index, Some(bold));
    assert_eq!(effective.fill().resource_index, Some(yellow));
    assert_eq!(effective.border().resource_index, Some(boxed));
}

/// A second tab gets its own part, its own relationship and its own `sheet` entry — and the first
/// tab's part is untouched.
#[test]
fn add_sheet_authors_a_part_and_wires_it_into_the_sheet_list() {
    let mut workbook = Workbook::blank().expect("authored");
    let before = part_text(workbook.package(), "/xl/worksheets/sheet1.xml");

    let index = workbook.add_sheet("Data").expect("a second tab");
    assert_eq!(index, 1);
    workbook
        .set_cell_value(
            index,
            CellReference::parse("A1").expect("a literal address"),
            CellValue::Number(7.0),
        )
        .expect("the store accepts the value");

    let bytes = workbook.save().expect("saves");
    let reopened = Workbook::open(&bytes).expect("reopens");
    assert_eq!(reopened.sheets().len(), 2);
    assert_eq!(reopened.sheets()[1].name, "Data");
    assert_eq!(
        reopened.sheets()[1].part.as_ref().map(PartName::as_str),
        Some("/xl/worksheets/sheet2.xml"),
    );
    assert_ne!(
        reopened.sheets()[0].sheet_id,
        reopened.sheets()[1].sheet_id,
        "@sheetId identifies a tab inside xl/workbook.xml; two tabs must not share one",
    );
    assert_eq!(
        part_text(reopened.package(), "/xl/worksheets/sheet1.xml"),
        before,
        "the first tab's part is byte-identical",
    );
    assert_eq!(
        reopened
            .worksheet_markup(1)
            .expect("readable")
            .and_then(|markup| markup
                .cell(CellReference::parse("A1").expect("a literal address"))
                .and_then(|cell| cell.number())),
        Some(7.0),
    );
}

/// Renaming a tab moves the name and leaves the part behind it alone.
#[test]
fn rename_sheet_moves_the_name_and_not_the_relationship() {
    let mut workbook = Workbook::blank().expect("authored");
    let before = part_text(workbook.package(), "/xl/worksheets/sheet1.xml");
    workbook.rename_sheet(0, "Q1 & Q2").expect("the tab exists");

    let bytes = workbook.save().expect("saves");
    let reopened = Workbook::open(&bytes).expect("reopens");
    assert_eq!(reopened.sheets()[0].name, "Q1 & Q2");
    assert_eq!(
        reopened.sheets()[0].part.as_ref().map(PartName::as_str),
        Some("/xl/worksheets/sheet1.xml"),
    );
    assert_eq!(
        part_text(reopened.package(), "/xl/worksheets/sheet1.xml"),
        before,
    );
    assert!(
        part_text(reopened.package(), "/xl/workbook.xml").contains(r#"name="Q1 &amp; Q2""#),
        "the ampersand is escaped in the attribute",
    );
}

/// Interning the same text twice answers the same index and appends nothing.
#[test]
fn interning_a_repeated_string_reuses_its_entry() {
    let mut workbook = Workbook::blank().expect("authored");
    let first = workbook.intern_shared_string("total").expect("interns");
    let second = workbook.intern_shared_string("total").expect("interns");
    let other = workbook.intern_shared_string(" total ").expect("interns");
    assert_eq!(first, second);
    assert_ne!(
        first, other,
        "the comparison is exact: padding makes a different value",
    );

    let bytes = workbook.save().expect("saves");
    let package = Package::open(&bytes).expect("reopens");
    let document =
        mjx_xml::fidelity::parse(part_text(&package, "/xl/sharedStrings.xml").as_bytes())
            .expect("well-formed");
    let table = SharedStringTable::read_part(&document)
        .expect("modelled")
        .expect("the root is x:sst");
    assert_eq!(table.len(), 2);
}

/// An append leaves every earlier entry of its table exactly where it was — indices are identity.
#[test]
fn appending_to_a_style_table_does_not_move_what_is_already_in_it() {
    let mut workbook = Workbook::blank().expect("authored");
    let before = part_text(workbook.package(), "/xl/styles.xml");
    let skeleton_font = before
        .find("<font>")
        .map(|at| {
            before[at..]
                .split("</font>")
                .next()
                .unwrap_or("")
                .to_owned()
        })
        .expect("the skeleton writes font 0");

    workbook
        .append_font(&FontProperties {
            font_name: Some("Arial".to_owned()),
            size_in_points: Some(9.0),
            ..FontProperties::default()
        })
        .expect("appends");

    let bytes = workbook.save().expect("saves");
    let package = Package::open(&bytes).expect("reopens");
    let after = part_text(&package, "/xl/styles.xml");
    assert!(
        after.contains(&skeleton_font),
        "font 0 moved or changed:\nbefore: {skeleton_font}\nafter: {after}"
    );
    assert!(after.contains(r#"<fonts count="2">"#), "{after}");
    assert!(after.contains(r#"<name val="Arial"/>"#), "{after}");
    // The other five tables are untouched.
    for unchanged in [
        r#"<fills count="2">"#,
        r#"<borders count="1">"#,
        r#"<cellStyleXfs count="1">"#,
        r#"<cellXfs count="1">"#,
        r#"<cellStyles count="1">"#,
    ] {
        assert!(after.contains(unchanged), "{unchanged} in:\n{after}");
    }
}

/// Naming a tab that does not exist is a typed error, not a panic and not a silent no-op.
#[test]
fn the_authoring_surface_refuses_an_index_that_names_no_tab() {
    let mut workbook = Workbook::blank().expect("authored");
    let reference = CellReference::parse("A1").expect("a literal address");
    assert!(matches!(
        workbook.set_cell_style(4, reference, Some(0)),
        Err(XlsxError::NoSuchSheet {
            index: 4,
            sheets: 1
        })
    ));
    assert!(matches!(
        workbook.rename_sheet(4, "nowhere"),
        Err(XlsxError::NoSuchSheet {
            index: 4,
            sheets: 1
        })
    ));
}

/// The worksheet a blank workbook authors is a real `CT_Worksheet`: it reads back through the same
/// model a file does, with the `sheetData` the schema requires and no `dimension` it cannot fill in.
#[test]
fn the_authored_worksheet_reads_back_through_the_ordinary_reader() {
    let workbook = reopened_blank();
    let text = part_text(workbook.package(), "/xl/worksheets/sheet1.xml");
    let markup = WorksheetPart::read_part(text.as_bytes())
        .expect("well-formed")
        .expect("the root is x:worksheet");
    assert_eq!(markup.cell_count(), 0);
    assert!(
        markup.sheet_data().is_some(),
        "CT_Worksheet declares sheetData minOccurs=\"1\"",
    );
    assert!(
        markup.dimension().is_none(),
        "@ref is required and there is no range to write for an empty sheet",
    );
}
