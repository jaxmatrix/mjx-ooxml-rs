//! **The parity gate (MJXOFF-112).** Everything [`EmbeddedWorkbook`] produces, produced through
//! `mjx-sml`, compared part by part.
//!
//! # Why this file exists, and why it is temporary
//!
//! `crates/mjx-chart/src/workbook.rs` is the workspace's one sanctioned duplicate: a minimal
//! SpreadsheetML writer inside a chart crate, written because a chart embeds a real `.xlsx` at
//! `/ppt/embeddings/*.xlsx` and no SpreadsheetML crate existed yet. Its own module documentation
//! names its executioner — **MJXOFF-99 deletes it** — and the deletion has to stay a *deletion*
//! rather than a migration, or the debt has simply moved.
//!
//! That is only true if the replacement covers everything the original did. This file is the proof:
//! it lays out the same grid through both writers and compares the packages **part by part, byte for
//! byte**. It lives in `mjx-chart` because `mjx-chart` is the only crate that may see both —
//! `mjx-chart → mjx-sml` is a legal downward edge (2.2 → 2.1), and `mjx-sml → mjx-chart` would be an
//! inversion `xtask/tests/layering.rs` rejects.
//!
//! **MJXOFF-99 deletes this file together with the writer it compares against.** Nothing here is
//! lost by that: every assertion about what the `mjx-sml` writer emits — the sheet name, the
//! first-use ordering, the styles skeleton, the `dimension`, the `t="s"` cells, the content types,
//! the relationship types — is *also* made in `crates/mjx-sml/tests/package_writer.rs`, against the
//! bytes rather than against a second writer. This file adds one thing those cannot: that the two
//! writers agree.
//!
//! # The one intended difference, and why it is intended
//!
//! Six of the seven parts are byte-identical. `xl/styles.xml` differs in the **order of a font's
//! children**: `mjx-chart` writes `sz`, `name`, `family` (which is what Excel writes) and `mjx-sml`
//! writes `name`, `family`, `sz` (`CT_Font`'s declaration order). `CT_Font`'s content model is
//! `<xsd:choice maxOccurs="unbounded">` — **the schema imposes no order at all** — so both are
//! valid, `mjx_ooxml_types::child_order` reports `ContentModel::Choice` with every slot at rank 0,
//! and there is no ordering table to consult. The styles comparison below is therefore made on the
//! *model*: the tables, their counts, and the record every index-0 reference resolves to.

use std::collections::BTreeSet;

use mjx_chart::{
    EmbeddedWorkbook, WorkbookCell, CONTENT_TYPE_WORKBOOK_PACKAGE, DEFAULT_SHEET_NAME,
};
use mjx_opc::{Package, PartName};
use mjx_sml::write::{AuthoredCellValue, WorkbookPackage};
use mjx_sml::StylesheetPart;

/// The grid both writers are given: a chart's header row with a blank corner, then one row per
/// category — exactly the layout `EmbeddedWorkbook::for_chart_data` builds, and exactly the layout
/// the chart's own `c:f` formulas name (`Sheet1!$A$2:$A$4` for the categories, `$B$1` for the first
/// series name).
///
/// Stated once, as data, and translated into each writer's own vocabulary below. A helper that built
/// it through one of the two writers would make this a comparison of that writer with itself.
const GRID: &[&[Value]] = &[
    &[Value::Blank, Value::Text("Revenue"), Value::Text("Cost")],
    &[Value::Text("Q1"), Value::Number(10.0), Value::Number(4.5)],
    &[Value::Text("Q2"), Value::Number(20.0), Value::Number(9.0)],
    // A repeat, so the shared-string table has to reuse an entry rather than append one.
    &[Value::Text("Q1"), Value::Number(30.0), Value::Blank],
];

/// One cell of [`GRID`], in neither writer's vocabulary.
#[derive(Clone, Copy)]
enum Value {
    Blank,
    Number(f64),
    Text(&'static str),
}

/// The grid, through the writer MJXOFF-99 deletes.
fn through_embedded_workbook() -> Vec<u8> {
    let mut workbook = EmbeddedWorkbook::new(DEFAULT_SHEET_NAME);
    for row in GRID {
        workbook.push_row(
            row.iter()
                .map(|cell| match cell {
                    Value::Blank => WorkbookCell::Blank,
                    Value::Number(number) => WorkbookCell::Number(*number),
                    Value::Text(text) => WorkbookCell::text(*text),
                })
                .collect(),
        );
    }
    workbook
        .to_package_bytes()
        .expect("the chart writer saves its package")
}

/// The grid, through the writer that replaces it.
fn through_mjx_sml() -> Vec<u8> {
    let mut workbook = WorkbookPackage::with_sheet_named(DEFAULT_SHEET_NAME)
        .expect("the mjx-sml writer's seeds parse");
    for row in GRID {
        let cells: Vec<AuthoredCellValue> = row
            .iter()
            .map(|cell| match cell {
                Value::Blank => AuthoredCellValue::Blank,
                Value::Number(number) => AuthoredCellValue::Number(*number),
                Value::Text(text) => AuthoredCellValue::SharedText((*text).to_owned()),
            })
            .collect();
        workbook
            .push_row(0, &cells)
            .expect("the grid fits the sheet");
    }
    workbook.recompute_dimensions();
    workbook
        .to_package_bytes()
        .expect("the mjx-sml writer saves its package")
}

/// The bytes of one part.
fn part(package: &Package, name: &str) -> Vec<u8> {
    let part = PartName::new(name).expect("a literal part name");
    package
        .part_bytes(&part)
        .unwrap_or_else(|| panic!("the package holds {name}"))
        .to_vec()
}

/// The bytes of one part, as text.
fn text(package: &Package, name: &str) -> String {
    String::from_utf8(part(package, name)).expect("both writers emit UTF-8")
}

/// One entry of a saved container, by name — read back out of the ZIP rather than through
/// [`Package`], because `[Content_Types].xml` is not a part and a consumer reads it as bytes.
fn zip_entry(bytes: &[u8], name: &str) -> Vec<u8> {
    let package = Package::open(bytes).expect("the container opens");
    package
        .entries()
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.bytes().map(<[u8]>::to_vec))
        .unwrap_or_else(|| panic!("the container holds {name}"))
}

/// Both packages, opened.
fn both() -> (Package, Package) {
    (
        Package::open(&through_embedded_workbook()).expect("the chart package opens"),
        Package::open(&through_mjx_sml()).expect("the mjx-sml package opens"),
    )
}

// -------------------------------------------------------------------------------------------
// The container
// -------------------------------------------------------------------------------------------

/// Both writers produce a ZIP container holding the same set of parts, under the same content types.
#[test]
fn the_part_list_and_the_content_types_are_the_same() {
    let chart = through_embedded_workbook();
    let sml = through_mjx_sml();
    assert_eq!(&chart[..2], b"PK");
    assert_eq!(&sml[..2], b"PK");

    let (chart, sml) = both();
    let names = |package: &Package| -> BTreeSet<String> {
        package
            .part_names()
            .map(|part| part.as_str().to_owned())
            .collect()
    };
    assert_eq!(names(&chart), names(&sml), "the part list");

    for name in names(&chart) {
        let part = PartName::new(&name).expect("a name the package reported");
        assert_eq!(
            chart.content_type_of(&part),
            sml.content_type_of(&part),
            "{name}'s content type",
        );
    }

    // The content-type part itself is byte-identical, overrides in the same order. `Package` does
    // not expose it as a part, so the comparison goes through the container entries — which is the
    // stronger statement anyway, since that is what a consumer reads.
    assert_eq!(
        zip_entry(&through_embedded_workbook(), "[Content_Types].xml"),
        zip_entry(&through_mjx_sml(), "[Content_Types].xml"),
        "[Content_Types].xml",
    );
}

/// The relationship graph is the same: the same ids, types and targets, from the same sources.
#[test]
fn the_relationship_graph_is_the_same() {
    let (chart, sml) = both();
    assert_eq!(
        part(&chart, "/_rels/.rels"),
        part(&sml, "/_rels/.rels"),
        "the package-root relationships",
    );
    assert_eq!(
        part(&chart, "/xl/_rels/workbook.xml.rels"),
        part(&sml, "/xl/_rels/workbook.xml.rels"),
        "the workbook's relationships",
    );
}

// -------------------------------------------------------------------------------------------
// The parts, one at a time
// -------------------------------------------------------------------------------------------

/// `xl/workbook.xml` is byte-identical: the same sheet name, the same `@sheetId`, the same `r:id`,
/// and the same two namespace declarations.
#[test]
fn the_workbook_part_is_byte_identical() {
    let (chart, sml) = both();
    let expected = text(&chart, "/xl/workbook.xml");
    assert_eq!(text(&sml, "/xl/workbook.xml"), expected);
    assert!(
        expected.contains(&format!(r#"name="{DEFAULT_SHEET_NAME}""#)),
        "the sheet a chart's `c:f` formulas name:\n{expected}"
    );
}

/// `xl/worksheets/sheet1.xml` is byte-identical: the `dimension`, the `t="s"` cells, the untyped
/// numeric cells, and the blank corner that is written as nothing at all.
#[test]
fn the_worksheet_part_is_byte_identical() {
    let (chart, sml) = both();
    let expected = text(&chart, "/xl/worksheets/sheet1.xml");
    assert_eq!(text(&sml, "/xl/worksheets/sheet1.xml"), expected);

    // …and the thing being compared is the thing the ticket names, rather than whatever the two
    // writers happened to agree on.
    assert!(
        expected.contains(r#"<dimension ref="A1:C4"/>"#),
        "{expected}"
    );
    assert!(
        expected.contains(r#"<c r="B1" t="s"><v>0</v></c>"#),
        "{expected}"
    );
    assert!(
        expected.contains(r#"<c r="B2"><v>10</v></c>"#),
        "{expected}"
    );
    assert!(
        !expected.contains(r#"<c r="A1""#),
        "the blank corner is written as no cell at all:\n{expected}"
    );
    assert!(
        !expected.contains(r#"<c r="C4""#),
        "a blank inside a row is written as no cell either:\n{expected}"
    );
}

/// `xl/sharedStrings.xml` is byte-identical: the same entries, in **first-use** order, with the
/// repeat reusing the entry it first made, and the same `count`/`uniqueCount`.
#[test]
fn the_shared_string_part_is_byte_identical_and_in_first_use_order() {
    let (chart, sml) = both();
    let expected = text(&chart, "/xl/sharedStrings.xml");
    assert_eq!(text(&sml, "/xl/sharedStrings.xml"), expected);

    let order: Vec<&str> = expected
        .match_indices("<t>")
        .map(|(at, _)| {
            let rest = &expected[at + 3..];
            &rest[..rest.find("</t>").expect("a closed <t>")]
        })
        .collect();
    assert_eq!(
        order,
        ["Revenue", "Cost", "Q1", "Q2"],
        "first use, not sorted (which would be Cost, Q1, Q2, Revenue):\n{expected}"
    );
    assert!(
        expected.contains(r#"count="4" uniqueCount="4""#),
        "the repeat reused entry 2 rather than appending a fifth:\n{expected}"
    );
}

/// `xl/styles.xml` states the same skeleton: the same six tables, the same counts, and the same
/// record behind every index-0 reference.
///
/// Compared through the **model** rather than byte for byte, for the one reason this file's own
/// documentation gives: `CT_Font` is an `xsd:choice`, so the order of a font's children is not
/// something either writer can get wrong.
#[test]
fn the_styles_part_states_the_same_skeleton() {
    let (chart, sml) = both();

    let read = |package: &Package, label: &str| {
        let bytes = text(package, "/xl/styles.xml");
        let document = mjx_xml::fidelity::parse(bytes.as_bytes())
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        let model = StylesheetPart::read_part(&document)
            .unwrap_or_else(|error| panic!("{label}: {error}"))
            .unwrap_or_else(|| panic!("{label}: the root is not x:styleSheet"));
        (document, model)
    };
    let (chart_document, chart_styles) = read(&chart, "the chart writer's styles");
    let (sml_document, sml_styles) = read(&sml, "the mjx-sml writer's styles");

    // The same tables, in the same order.
    let locals = |styles: &StylesheetPart, interner: &mjx_ooxml_core::Interner| -> Vec<String> {
        styles
            .child_element_locals(interner)
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(
        locals(&chart_styles, &chart_document.interner),
        locals(&sml_styles, &sml_document.interner),
        "the tables `styles.xml` writes, in order",
    );

    // One font, with the same three properties.
    let chart_font = chart_styles
        .fonts()
        .and_then(|table| table.get(0))
        .expect("the chart writer writes font 0")
        .properties(&chart_document.interner);
    let sml_font = sml_styles
        .fonts()
        .and_then(|table| table.get(0))
        .expect("the mjx-sml writer writes font 0")
        .properties(&sml_document.interner);
    assert_eq!(chart_font, sml_font, "font 0");
    assert_eq!(chart_font.font_name.as_deref(), Some("Calibri"));
    assert_eq!(chart_font.size_in_points, Some(11.0));
    assert_eq!(chart_font.family, Some(2));

    // The two fills, the one border, the two `xf` tables and the `Normal` style.
    assert_eq!(
        chart_styles.fills().map(mjx_sml::FillTable::len),
        sml_styles.fills().map(mjx_sml::FillTable::len),
    );
    assert_eq!(
        chart_styles.borders().map(mjx_sml::BorderTable::len),
        sml_styles.borders().map(mjx_sml::BorderTable::len),
    );
    for (chart_table, sml_table, label) in [
        (
            chart_styles.cell_style_formats(),
            sml_styles.cell_style_formats(),
            "cellStyleXfs",
        ),
        (
            chart_styles.cell_formats(),
            sml_styles.cell_formats(),
            "cellXfs",
        ),
    ] {
        let chart_table = chart_table.unwrap_or_else(|| panic!("the chart writer writes {label}"));
        let sml_table = sml_table.unwrap_or_else(|| panic!("the mjx-sml writer writes {label}"));
        assert_eq!(chart_table.len(), sml_table.len(), "{label}'s length");
        let chart_record = chart_table.get(0).expect("record 0");
        let sml_record = sml_table.get(0).expect("record 0");
        for (name, read) in [
            (
                "numFmtId",
                mjx_sml::CellFormat::number_format_id
                    as fn(&mjx_sml::CellFormat, &mjx_ooxml_core::Interner) -> _,
            ),
            ("fontId", mjx_sml::CellFormat::font_index),
            ("fillId", mjx_sml::CellFormat::fill_index),
            ("borderId", mjx_sml::CellFormat::border_index),
            ("xfId", mjx_sml::CellFormat::cell_style_format_index),
        ] {
            assert_eq!(
                read(chart_record, &chart_document.interner).ok().flatten(),
                read(sml_record, &sml_document.interner).ok().flatten(),
                "{label}[0]/@{name}",
            );
        }
        assert_eq!(
            chart_record
                .number_format_id(&chart_document.interner)
                .ok()
                .flatten(),
            Some(0),
            "{label}[0] names the General number format by id 0",
        );
    }

    let normal = sml_styles
        .named_styles()
        .and_then(|table| table.by_builtin_id(&sml_document.interner, 0))
        .expect("the Normal cell style");
    assert_eq!(
        normal
            .style_name(&sml_document.interner)
            .ok()
            .flatten()
            .as_deref(),
        Some("Normal"),
    );
}

/// The host package registers an embedded workbook by the `.xlsx` **file**'s content type, and both
/// writers agree on which constant that is.
///
/// `mjx-chart` exports it as `CONTENT_TYPE_WORKBOOK_PACKAGE`; MJXOFF-99 removes that copy in favour
/// of `mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE`, and `crates/mjx-pptx/src/presentation/charts.rs`
/// switches over. They have to be the same string for that to be a rename rather than a change.
#[test]
fn the_embedded_package_content_type_is_the_same_constant() {
    assert_eq!(
        CONTENT_TYPE_WORKBOOK_PACKAGE,
        mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE,
    );
    assert_eq!(DEFAULT_SHEET_NAME, mjx_sml::write::DEFAULT_SHEET_NAME);
}

/// An empty workbook — no rows at all — comes out the same from both, `dimension` absent from both.
///
/// The edge case `EmbeddedWorkbook::dimension` answers `None` for, and the one where "recompute the
/// bounding box" has no valid answer to write: `@ref` is `use="required"` and `ref=""` is not an
/// `ST_Ref`.
#[test]
fn an_empty_workbook_comes_out_the_same_from_both_writers() {
    let chart = EmbeddedWorkbook::new(DEFAULT_SHEET_NAME)
        .to_package_bytes()
        .expect("the chart writer saves");
    let mut writer = WorkbookPackage::with_sheet_named(DEFAULT_SHEET_NAME)
        .expect("the mjx-sml writer's seeds parse");
    writer.recompute_dimensions();
    let sml = writer.to_package_bytes().expect("the mjx-sml writer saves");

    let chart = Package::open(&chart).expect("the chart package opens");
    let sml = Package::open(&sml).expect("the mjx-sml package opens");
    for name in [
        "/xl/workbook.xml",
        "/xl/worksheets/sheet1.xml",
        "/xl/sharedStrings.xml",
    ] {
        assert_eq!(text(&chart, name), text(&sml, name), "{name}");
    }
    assert!(!text(&sml, "/xl/worksheets/sheet1.xml").contains("<dimension"));
}
