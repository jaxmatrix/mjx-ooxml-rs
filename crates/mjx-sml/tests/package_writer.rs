//! The package writer (MJXOFF-112): what it emits, and that it needs nothing above this crate.
//!
//! # The two things this suite exists to prove
//!
//! **1. The writer works with no `mjx-xlsx` in the dependency graph.** That is the whole
//! architectural point of `mjx-sml`: `mjx-chart` embeds a real workbook package inside a `.pptx`,
//! `mjx-chart → mjx-xlsx` points *upward* and is forbidden, and `mjx-chart → mjx-sml` points down.
//! This suite is compiled into `mjx-sml` itself, whose manifest names neither `mjx-xlsx` nor
//! `mjx-chart`; [`the_writer_needs_nothing_above_this_crate`] reads that manifest and asserts it, so
//! a later child that added such an edge would fail here as well as in
//! `xtask/tests/layering.rs`.
//!
//! **2. What the writer emits, stated independently of the writer.** The assertions below are
//! written against the file's *bytes* and against `sml.xsd`'s own rules — a `t="s"` cell holds an
//! index, a `dimension` covers the populated cells, `sharedStrings.xml` lists entries in first-use
//! order — not against a second call into the code under test. The byte-for-byte comparison with
//! `mjx_chart::EmbeddedWorkbook`, the writer this one replaces, lives in
//! `crates/mjx-chart/tests/workbook_parity.rs`, because only `mjx-chart` may see both.
//!
//! MJXOFF-99 deletes that file along with the writer it compares against; everything here survives
//! it, which is the point of the split.

use std::collections::BTreeSet;

use mjx_ooxml_types::spreadsheetml::PatternType;
use mjx_opc::Package;
use mjx_sml::write::{
    AuthoredCellValue, CellFormatSpec, CellFormatTarget, PatternFillSpec, WorkbookPackage,
    CONTENT_TYPE_SHARED_STRINGS, CONTENT_TYPE_STYLES, CONTENT_TYPE_WORKBOOK,
    CONTENT_TYPE_WORKSHEET, DEFAULT_SHEET_NAME, REL_OFFICE_DOCUMENT, REL_SHARED_STRINGS,
    REL_STYLES, REL_WORKSHEET,
};
use mjx_sml::{
    CellReference, CellValue, FontProperties, SharedStringTable, SheetList, StylesheetPart,
    WorkbookPart, WorksheetPart,
};

/// The SpreadsheetML namespace, as `sml.xsd` declares it.
const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// The grid `mjx_chart::EmbeddedWorkbook::for_chart_data` lays out for a two-series bar chart: a
/// header row whose first cell is blank, then one row per category.
///
/// Written out here rather than computed, so that the layout the assertions check is stated
/// independently of anything that produces it.
fn chart_grid() -> Vec<Vec<AuthoredCellValue>> {
    vec![
        vec![
            AuthoredCellValue::Blank,
            AuthoredCellValue::SharedText("Revenue".to_owned()),
            AuthoredCellValue::SharedText("Cost".to_owned()),
        ],
        vec![
            AuthoredCellValue::SharedText("Q1".to_owned()),
            AuthoredCellValue::Number(10.0),
            AuthoredCellValue::Number(4.5),
        ],
        vec![
            AuthoredCellValue::SharedText("Q2".to_owned()),
            AuthoredCellValue::Number(20.0),
            AuthoredCellValue::Number(9.0),
        ],
    ]
}

/// The package that grid produces.
fn chart_workbook() -> WorkbookPackage {
    let mut workbook = WorkbookPackage::new().expect("the writer's seeds parse");
    for row in chart_grid() {
        workbook.push_row(0, &row).expect("the grid fits the sheet");
    }
    workbook.recompute_dimensions();
    workbook
}

/// The bytes of one part of a saved package.
fn part_text(package: &Package, name: &str) -> String {
    let part = mjx_opc::PartName::new(name).expect("a literal part name");
    let bytes = package
        .part_bytes(&part)
        .unwrap_or_else(|| panic!("the package holds {name}"));
    String::from_utf8(bytes.to_vec()).expect("this writer emits UTF-8")
}

// -------------------------------------------------------------------------------------------
// The architectural gate
// -------------------------------------------------------------------------------------------

/// This crate's manifest names no format crate, so the writer above cannot be reaching one.
///
/// `xtask/tests/layering.rs` checks the *direction* of every edge in the workspace. This checks the
/// one edge that matters to this file, from inside the crate that would have to declare it, so that
/// the claim "the writer works with no `mjx-xlsx` in the graph" is a fact this suite establishes
/// rather than one it inherits.
///
/// The rest of this file is the other half of the proof: it authors a whole `.xlsx` package, reads
/// every part of it back, and validates it — and it compiles, so nothing it uses lives above
/// `mjx-sml`.
#[test]
fn the_writer_needs_nothing_above_this_crate() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("this crate has a manifest");
    // The *declared name* of each dependency, which is the token before the first `.`, `=` or space
    // on a non-comment line. Substring matching would be wrong in both directions here:
    // `mjx-ooxml-core` contains `mjx-ooxml`, and a comment explaining why an edge is absent would
    // read as the edge being present.
    let declared: Vec<&str> = manifest
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            line.split_once(|character: char| {
                character == '.' || character == '=' || character == ' '
            })
            .map_or(line, |(name, _)| name)
        })
        .collect();
    for forbidden in ["mjx-xlsx", "mjx-pptx", "mjx-docx", "mjx-ooxml", "mjx-chart"] {
        assert!(
            !declared.contains(&forbidden),
            "crates/mjx-sml/Cargo.toml declares `{forbidden}`. The package writer exists here \
             precisely so that `mjx-chart` can reach it without an upward edge; an edge from this \
             crate to a format crate, to the facade, or to `mjx-chart` inverts that and makes \
             MJXOFF-99's deletion illegal again."
        );
    }
    assert!(
        declared.contains(&"mjx-opc"),
        "the check above is only worth anything if it can see this crate's dependency names at all",
    );
}

// -------------------------------------------------------------------------------------------
// The package
// -------------------------------------------------------------------------------------------

/// The writer emits exactly the four content parts a minimal workbook is made of — plus the two
/// relationship parts that join them — each under the content type ECMA-376 Part 1 §12.3 gives it.
#[test]
fn the_authored_package_holds_the_parts_a_workbook_needs_and_no_others() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");

    let names: BTreeSet<String> = package
        .part_names()
        .map(|part| part.as_str().to_owned())
        .collect();
    let expected: BTreeSet<String> = [
        "/_rels/.rels",
        "/xl/_rels/workbook.xml.rels",
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/sharedStrings.xml",
        "/xl/styles.xml",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(names, expected, "the authored part list");

    for (name, content_type) in [
        ("/xl/workbook.xml", CONTENT_TYPE_WORKBOOK),
        ("/xl/worksheets/sheet1.xml", CONTENT_TYPE_WORKSHEET),
        ("/xl/sharedStrings.xml", CONTENT_TYPE_SHARED_STRINGS),
        ("/xl/styles.xml", CONTENT_TYPE_STYLES),
    ] {
        let part = mjx_opc::PartName::new(name).expect("a literal part name");
        assert_eq!(
            package.content_type_of(&part),
            Some(content_type),
            "{name}'s content type"
        );
    }
}

/// The package root names the workbook, and the workbook names its worksheet, its styles and its
/// shared strings — each under the relationship type Part 1 §12.3 gives it.
#[test]
fn the_relationship_graph_is_the_one_a_consumer_walks() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    let workbook_part = mjx_opc::PartName::new("/xl/workbook.xml").expect("a literal part name");

    let root = package
        .relationships_for(None)
        .expect("the package root has relationships");
    let office_document: Vec<&str> = root
        .by_type(REL_OFFICE_DOCUMENT)
        .map(|rel| rel.target.as_str())
        .collect();
    assert_eq!(office_document, ["xl/workbook.xml"]);

    let from_workbook = package
        .relationships_for(Some(&workbook_part))
        .expect("the workbook part has relationships");
    for (rel_type, target) in [
        (REL_WORKSHEET, "worksheets/sheet1.xml"),
        (REL_STYLES, "styles.xml"),
        (REL_SHARED_STRINGS, "sharedStrings.xml"),
    ] {
        let targets: Vec<&str> = from_workbook
            .by_type(rel_type)
            .map(|rel| rel.target.as_str())
            .collect();
        assert_eq!(targets, [target], "the {rel_type} relationship");
    }

    // The `r:id` on the one `sheet` entry resolves to the worksheet relationship, which is the only
    // thing that names the part — not `@sheetId`, not the position in the list.
    let workbook_text = part_text(&package, "/xl/workbook.xml");
    let document = mjx_xml::fidelity::parse(workbook_text.as_bytes()).expect("well-formed");
    let model = WorkbookPart::read_part(&document)
        .expect("a modelled workbook")
        .expect("the root is x:workbook");
    let prefix = model
        .relationship_prefix(&document.interner)
        .expect("the authored root binds the relationship namespace");
    let entry = model
        .sheets()
        .expect("x:sheets is mandatory")
        .entries()
        .next()
        .expect("x:sheet is mandatory");
    let id = entry
        .relationship_id(&document.interner, Some(prefix))
        .expect("a decodable r:id")
        .expect("the entry names a relationship");
    assert_eq!(
        from_workbook
            .by_id(&id)
            .map(|rel| rel.rel_type.as_str())
            .expect("the r:id resolves"),
        REL_WORKSHEET,
    );
}

/// A package with no sheet is not schema-valid — `CT_Workbook` declares `sheets` `minOccurs="1"`
/// and `CT_Sheets` declares `sheet` `minOccurs="1"` — so the writer starts with one.
#[test]
fn a_new_package_already_has_the_one_sheet_the_schema_requires() {
    let mut package = WorkbookPackage::new().expect("the writer's seeds parse");
    assert_eq!(package.sheet_count(), 1);
    assert_eq!(
        package.sheet(0).expect("the first tab").name(),
        DEFAULT_SHEET_NAME,
    );

    let text = part_text(
        &package.to_package().expect("the package assembles"),
        "/xl/workbook.xml",
    );
    let document = mjx_xml::fidelity::parse(text.as_bytes()).expect("well-formed");
    let model = WorkbookPart::read_part(&document)
        .expect("a modelled workbook")
        .expect("the root is x:workbook");
    assert_eq!(model.sheets().map(SheetList::len), Some(1));
}

/// A second tab gets its own part, its own relationship and its own `sheet` entry.
#[test]
fn a_second_sheet_gets_its_own_part_and_its_own_relationship() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    let second = writer.add_sheet("Data").expect("a second tab");
    assert_eq!(second, 1);
    writer
        .push_row(second, &[AuthoredCellValue::Number(7.0)])
        .expect("one cell fits");
    let package = writer.to_package().expect("the package assembles");

    let names: Vec<String> = package
        .part_names()
        .map(|part| part.as_str().to_owned())
        .filter(|name| name.starts_with("/xl/worksheets/"))
        .collect();
    assert_eq!(
        names.iter().collect::<BTreeSet<_>>(),
        ["/xl/worksheets/sheet1.xml", "/xl/worksheets/sheet2.xml"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>()
            .iter()
            .collect::<BTreeSet<_>>(),
    );
    assert!(part_text(&package, "/xl/worksheets/sheet2.xml").contains("<v>7</v>"));
    assert!(part_text(&package, "/xl/workbook.xml").contains(r#"name="Data""#));
}

/// Two calls produce byte-identical containers. Nothing here reads a clock or a random number, and
/// every round-trip assertion downstream would be flaky if one did.
#[test]
fn the_writer_is_deterministic() {
    let first = chart_workbook()
        .to_package_bytes()
        .expect("the package saves");
    let second = chart_workbook()
        .to_package_bytes()
        .expect("the package saves");
    assert_eq!(first, second, "two runs of the writer");

    // And twice out of *one* writer, which is the stronger statement: serializing must not consume
    // or mutate anything.
    let mut writer = chart_workbook();
    assert_eq!(
        writer.to_package_bytes().expect("the package saves"),
        writer.to_package_bytes().expect("the package saves"),
        "two saves of one writer",
    );
}

// -------------------------------------------------------------------------------------------
// The worksheet
// -------------------------------------------------------------------------------------------

/// The grid comes out cell for cell: a `t="s"` cell holding an index for text, a plain `<v>` with no
/// `t` for a number, and **no cell at all** where the grid was blank.
///
/// Asserted against the emitted bytes rather than against the model that wrote them.
#[test]
fn the_worksheet_writes_the_grid_it_was_given() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");

    // Row 1's blank corner is not written: a cell for it would be a statement the grid did not make.
    assert!(
        !sheet.contains(r#"<c r="A1""#),
        "the blank corner must not be written; the sheet was:\n{sheet}"
    );
    assert!(
        sheet.contains(r#"<row r="1"><c r="B1" t="s"><v>0</v></c>"#),
        "{sheet}"
    );
    assert!(sheet.contains(r#"<c r="C1" t="s"><v>1</v></c>"#), "{sheet}");
    // A number carries no `t`: `n` is the schema default and a cell must not gain the attribute.
    assert!(sheet.contains(r#"<c r="B2"><v>10</v></c>"#), "{sheet}");
    assert!(sheet.contains(r#"<c r="C3"><v>9</v></c>"#), "{sheet}");
    assert!(sheet.contains(r#"<c r="A2" t="s"><v>2</v></c>"#), "{sheet}");
}

/// `<dimension>` covers the populated cells, and it precedes `sheetData` — rank 1 against rank 5 of
/// `CT_Worksheet`, from the generated table rather than from a hand-written order.
#[test]
fn the_worksheet_caches_the_bounding_box_of_the_cells_it_holds() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");

    // B1..C3 is what the grid populates: A1 is blank, so the box starts at A2's column and B1's row.
    assert!(sheet.contains(r#"<dimension ref="A1:C3"/>"#), "{sheet}");
    let dimension_at = sheet.find("<dimension").expect("a dimension");
    let data_at = sheet.find("<sheetData").expect("a sheetData");
    assert!(
        dimension_at < data_at,
        "dimension is rank 1 and sheetData is rank 5:\n{sheet}"
    );
}

/// A sheet with no cell writes **no** `dimension` at all.
///
/// `@ref` is `use="required"` on `CT_SheetDimension` and `ref=""` is not an `ST_Ref`, so there is no
/// schema-valid dimension for an empty sheet to write.
#[test]
fn an_empty_sheet_writes_no_dimension_because_there_is_no_valid_one() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    writer.recompute_dimensions();
    let package = writer.to_package().expect("the package assembles");
    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");
    assert!(!sheet.contains("<dimension"), "{sheet}");
    // `sheetData` is `minOccurs="1"`, so it is there even with nothing in it.
    assert!(sheet.contains("<sheetData/>"), "{sheet}");
}

/// Every cell type the authoring surface offers comes back as the type it was written as.
#[test]
fn every_authored_cell_type_round_trips_through_the_reader() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    let shared = writer
        .intern_shared_string("shared")
        .expect("the table interns");
    for (address, value) in [
        ("A1", CellValue::Number(42.5)),
        ("B1", CellValue::SharedString(shared)),
        ("C1", CellValue::Boolean(true)),
        ("D1", CellValue::Error("#DIV/0!")),
        ("E1", CellValue::InlineString("inline & escaped")),
    ] {
        let reference = CellReference::parse(address).expect("a literal address");
        writer
            .set_cell_value(0, reference, value)
            .expect("the store accepts the value");
    }
    writer.recompute_dimensions();

    let package = writer.to_package().expect("the package assembles");
    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");
    let markup = WorksheetPart::read_part(sheet.as_bytes())
        .expect("well-formed")
        .expect("the root is x:worksheet");

    let cell = |address: &str| {
        markup
            .cell(CellReference::parse(address).expect("a literal address"))
            .unwrap_or_else(|| panic!("{address} is populated"))
    };
    assert_eq!(cell("A1").number(), Some(42.5));
    assert_eq!(cell("B1").shared_string_index(), Some(shared));
    assert_eq!(cell("C1").boolean(), Some(true));
    assert_eq!(
        cell("D1").value().expect("decodable").as_deref(),
        Some("#DIV/0!"),
    );
    assert_eq!(
        cell("E1").cell_type(),
        mjx_ooxml_types::spreadsheetml::CellType::InlineString,
    );
    assert_eq!(
        core::str::from_utf8(cell("E1").inline_string_markup().expect("an <is>")),
        Ok("<is><t>inline &amp; escaped</t></is>"),
        "the ampersand is escaped on the way out",
    );

    let strings = SharedStringTable::read_part(
        &mjx_xml::fidelity::parse(part_text(&package, "/xl/sharedStrings.xml").as_bytes())
            .expect("well-formed"),
    )
    .expect("a modelled table")
    .expect("the root is x:sst");
    assert_eq!(
        strings
            .item(shared)
            .map(|item| item.text().expect("decodable"))
            .as_deref(),
        Some("shared"),
    );
}

/// A number SpreadsheetML cannot spell is refused on the cell-at-a-time door, and skipped on the
/// grid door — two different questions with two different right answers.
#[test]
fn a_non_finite_number_is_refused_by_one_door_and_skipped_by_the_other() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    let reference = CellReference::parse("A1").expect("a literal address");
    let refused = writer.set_cell_value(0, reference, CellValue::Number(f64::NAN));
    assert!(
        matches!(
            refused,
            Err(mjx_sml::SmlError::UnrepresentableNumber { .. })
        ),
        "naming one cell states a value, and NaN is not one: {refused:?}",
    );

    let mut grid = WorkbookPackage::new().expect("the writer's seeds parse");
    grid.push_row(
        0,
        &[
            AuthoredCellValue::Number(f64::INFINITY),
            AuthoredCellValue::Number(1.0),
        ],
    )
    .expect("a grid with a hole in it is a grid");
    let package = grid.to_package().expect("the package assembles");
    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");
    assert!(!sheet.contains(r#"<c r="A1""#), "{sheet}");
    assert!(sheet.contains(r#"<c r="B1"><v>1</v></c>"#), "{sheet}");
}

// -------------------------------------------------------------------------------------------
// The shared-string table
// -------------------------------------------------------------------------------------------

/// Entries are listed in **first-use order**, and a repeat reuses the entry it first made.
///
/// First-use order is not a cosmetic choice: a cell holds an *index*, so re-sorting the list would
/// repoint every `t="s"` cell in the workbook. The assertion is on the order of the `<t>` elements
/// in the emitted part, and on the indices the cells hold.
#[test]
fn the_shared_string_table_is_in_first_use_order() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    // Deliberately not alphabetical: sorted order would be `alpha, mid, zeta`.
    writer
        .push_row(
            0,
            &[
                AuthoredCellValue::SharedText("zeta".to_owned()),
                AuthoredCellValue::SharedText("mid".to_owned()),
                AuthoredCellValue::SharedText("alpha".to_owned()),
                AuthoredCellValue::SharedText("zeta".to_owned()),
            ],
        )
        .expect("the row fits");
    let package = writer.to_package().expect("the package assembles");

    let strings = part_text(&package, "/xl/sharedStrings.xml");
    let order: Vec<&str> = strings
        .match_indices("<t>")
        .map(|(at, _)| {
            let rest = &strings[at + 3..];
            &rest[..rest.find("</t>").expect("a closed <t>")]
        })
        .collect();
    assert_eq!(order, ["zeta", "mid", "alpha"], "{strings}");
    assert!(
        strings.contains(r#"count="3" uniqueCount="3""#),
        "{strings}"
    );

    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"<c r="A1" t="s"><v>0</v></c>"#), "{sheet}");
    assert!(sheet.contains(r#"<c r="C1" t="s"><v>2</v></c>"#), "{sheet}");
    assert!(
        sheet.contains(r#"<c r="D1" t="s"><v>0</v></c>"#),
        "the repeat reuses entry 0:\n{sheet}"
    );
}

// -------------------------------------------------------------------------------------------
// The styles skeleton
// -------------------------------------------------------------------------------------------

/// The skeleton is the six tables a workbook needs to open, each with the entry index 0 resolves to.
///
/// Asserted through the *model*, so that the shape rather than the spelling is what is pinned —
/// `CT_Font`'s content model is an `xsd:choice`, so the order of a font's children is not part of
/// the claim.
#[test]
fn the_styles_skeleton_is_the_six_tables_index_zero_resolves_through() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    let text = part_text(&package, "/xl/styles.xml");
    let document = mjx_xml::fidelity::parse(text.as_bytes()).expect("well-formed");
    let styles = StylesheetPart::read_part(&document)
        .expect("a modelled stylesheet")
        .expect("the root is x:styleSheet");
    let interner = &document.interner;

    let fonts = styles.fonts().expect("a font table");
    assert_eq!(fonts.len(), 1);
    let font = fonts.get(0).expect("font 0").properties(interner);
    assert_eq!(font.font_name.as_deref(), Some("Calibri"));
    assert_eq!(font.size_in_points, Some(11.0));
    assert_eq!(font.family, Some(2));

    let fills = styles.fills().expect("a fill table");
    assert_eq!(fills.len(), 2, "Excel writes `none` then `gray125`, always");
    let pattern_of = |index: usize| {
        fills
            .get(index)
            .and_then(mjx_sml::Fill::pattern)
            .and_then(|fill| fill.pattern_type(interner).ok().flatten())
    };
    assert_eq!(
        pattern_of(0),
        Some(PatternType::None),
        "fill 0 must be `none`, or every unfilled cell is repainted",
    );
    assert_eq!(pattern_of(1), Some(PatternType::Gray12Point5Percent));

    let borders = styles.borders().expect("a border table");
    assert_eq!(borders.len(), 1);

    for (table, kind) in [
        (styles.cell_style_formats(), "cellStyleXfs"),
        (styles.cell_formats(), "cellXfs"),
    ] {
        let table = table.unwrap_or_else(|| panic!("a {kind} table"));
        assert_eq!(table.len(), 1, "{kind}");
        let record = table.get(0).unwrap_or_else(|| panic!("{kind}[0]"));
        assert_eq!(record.number_format_id(interner).ok().flatten(), Some(0));
        assert_eq!(record.font_index(interner).ok().flatten(), Some(0));
        assert_eq!(record.fill_index(interner).ok().flatten(), Some(0));
        assert_eq!(record.border_index(interner).ok().flatten(), Some(0));
    }
    assert_eq!(
        styles
            .cell_formats()
            .and_then(|table| table.get(0))
            .and_then(|record| record.cell_style_format_index(interner).ok().flatten()),
        Some(0),
        "a cellXfs record points at the cellStyleXfs record beneath it",
    );
    assert_eq!(
        styles
            .cell_style_formats()
            .and_then(|table| table.get(0))
            .and_then(|record| record.cell_style_format_index(interner).ok().flatten()),
        None,
        "a cellStyleXfs record has nothing beneath it to point at",
    );

    let named = styles.named_styles().expect("a cellStyles table");
    let normal = named.by_builtin_id(interner, 0).expect("the Normal style");
    assert_eq!(
        normal.style_name(interner).ok().flatten().as_deref(),
        Some("Normal"),
    );

    // Every table declares `@count`, and it agrees with what the table holds.
    for declared in [
        r#"<fonts count="1">"#,
        r#"<fills count="2">"#,
        r#"<borders count="1">"#,
        r#"<cellStyleXfs count="1">"#,
        r#"<cellXfs count="1">"#,
        r#"<cellStyles count="1">"#,
    ] {
        assert!(text.contains(declared), "{declared} in:\n{text}");
    }
}

/// The six tables come out in `CT_Stylesheet`'s `xsd:sequence` order, which is the generated table's
/// answer rather than the order they were built in.
#[test]
fn the_styles_tables_are_emitted_in_schema_order() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    let text = part_text(&package, "/xl/styles.xml");
    let positions: Vec<usize> = [
        "<fonts",
        "<fills",
        "<borders",
        "<cellStyleXfs",
        "<cellXfs",
        "<cellStyles",
    ]
    .iter()
    .map(|local| {
        text.find(local)
            .unwrap_or_else(|| panic!("{local} in:\n{text}"))
    })
    .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "the six tables are out of schema order:\n{text}"
    );
}

/// An appended resource gets the next index, `@count` follows it, and the earlier entries do not
/// move — indices are identity, so a table that reordered on append would repaint every cell that
/// referred to anything after the entry that moved.
#[test]
fn appending_a_resource_gives_it_the_next_index_and_moves_nothing() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");

    let bold = writer
        .append_font(&FontProperties {
            font_name: Some("Calibri".to_owned()),
            size_in_points: Some(11.0),
            bold: Some(true),
            ..FontProperties::default()
        })
        .expect("the font is well-formed");
    assert_eq!(bold, 1, "the skeleton already holds font 0");

    let yellow = writer.append_pattern_fill(&PatternFillSpec::solid("FFFF00"));
    assert_eq!(yellow, 2, "the skeleton already holds fills 0 and 1");

    let border = writer.append_border(&mjx_sml::write::BorderSpec::all_edges_plain());
    assert_eq!(border, 1);

    let style = writer.append_cell_format(
        CellFormatTarget::CellFormats,
        &CellFormatSpec {
            font_index: Some(bold),
            fill_index: Some(yellow),
            border_index: Some(border),
            applies_font: Some(true),
            applies_fill: Some(true),
            ..CellFormatSpec::skeleton_cell_format()
        },
    );
    assert_eq!(style, 1);

    let reference = CellReference::parse("A1").expect("a literal address");
    writer
        .set_cell_value(0, reference, CellValue::Number(1.0))
        .expect("the store accepts the value");
    writer
        .set_cell_style(0, reference, Some(style))
        .expect("the store accepts the style");

    let package = writer.to_package().expect("the package assembles");
    let text = part_text(&package, "/xl/styles.xml");
    assert!(text.contains(r#"<fonts count="2">"#), "{text}");
    assert!(text.contains(r#"<fills count="3">"#), "{text}");
    assert!(text.contains(r#"<borders count="2">"#), "{text}");
    assert!(text.contains(r#"<cellXfs count="2">"#), "{text}");
    assert!(
        text.contains(r#"<fill><patternFill patternType="none"/></fill>"#),
        "fill 0 is still `none` after the append:\n{text}"
    );
    assert!(
        text.contains(r#"patternType="solid"><fgColor rgb="FFFFFF00"/>"#),
        "the appended fill carries its opaque colour:\n{text}"
    );

    let sheet = part_text(&package, "/xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"<c r="A1" s="1"><v>1</v></c>"#), "{sheet}");
}

/// A border describes **nine** edges, not four, and the ones a spec leaves out are not written.
#[test]
fn a_border_carries_all_nine_of_the_edges_the_schema_declares() {
    let mut writer = WorkbookPackage::new().expect("the writer's seeds parse");
    writer.append_border(&mjx_sml::write::BorderSpec::all_edges_plain());
    let package = writer.to_package().expect("the package assembles");
    let text = part_text(&package, "/xl/styles.xml");

    for edge in [
        "start",
        "end",
        "left",
        "right",
        "top",
        "bottom",
        "diagonal",
        "vertical",
        "horizontal",
    ] {
        assert!(
            text.contains(&format!("<{edge}/>")),
            "<{edge}/> in:\n{text}"
        );
    }
    assert!(
        text.contains(
            "<border><start/><end/><left/><right/><top/><bottom/><diagonal/><vertical/>\
             <horizontal/></border>"
        ),
        "the nine edges come out in CT_Border's sequence order:\n{text}"
    );
}

// -------------------------------------------------------------------------------------------
// Namespaces, which are the thing a freshly built root loses
// -------------------------------------------------------------------------------------------

/// Every authored part declares the SpreadsheetML namespace on its own root, and the workbook part
/// also binds the relationship-reference namespace.
///
/// Asserted on the emitted **bytes**. `mjx-docx`'s `create_footnotes_part` lost exactly this
/// declaration by writing a freshly constructed root over a parsed one, and the gate stayed green
/// because it asserted on the model that had just been built. This case cannot pass that way.
#[test]
fn every_authored_part_declares_its_namespace_in_the_bytes() {
    let package = chart_workbook()
        .to_package()
        .expect("the package assembles");
    for name in [
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/sharedStrings.xml",
        "/xl/styles.xml",
    ] {
        let text = part_text(&package, name);
        assert!(
            text.contains(&format!(r#"xmlns="{SML_NS}""#)),
            "{name} does not declare the SpreadsheetML namespace:\n{text}"
        );
        assert!(
            text.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#),
            "{name} does not open with the declaration Office writes:\n{text}"
        );
    }
    assert!(
        part_text(&package, "/xl/workbook.xml").contains(
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#
        ),
        "an element in no namespace is not `r:id` however it is spelled",
    );
}

/// Renaming a tab changes `sheet@name` and touches nothing else — not the relationship, not the
/// worksheet part.
#[test]
fn renaming_a_tab_moves_the_name_and_nothing_else() {
    let mut writer = chart_workbook();
    let before = writer.to_package().expect("the package assembles");
    let sheet_before = part_text(&before, "/xl/worksheets/sheet1.xml");

    writer.rename_sheet(0, "Renamed").expect("the tab exists");
    let after = writer.to_package().expect("the package assembles");

    assert!(part_text(&after, "/xl/workbook.xml").contains(r#"name="Renamed""#));
    assert!(!part_text(&after, "/xl/workbook.xml").contains(r#"name="Sheet1""#));
    assert_eq!(
        sheet_before,
        part_text(&after, "/xl/worksheets/sheet1.xml"),
        "the worksheet part is byte-identical: a tab's name is not in its own markup",
    );
    assert!(matches!(
        writer.rename_sheet(9, "nowhere"),
        Err(mjx_sml::SmlError::SheetIndexOutOfRange {
            index: 9,
            sheets: 1
        })
    ));
}
