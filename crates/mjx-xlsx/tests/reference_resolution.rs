//! MJXOFF-200 — every reference a workbook **this library authored** makes resolves.
//!
//! The class, not the instance. See `crates/mjx-schema-gate/src/references.rs` for the rules, their
//! deliberate limits, and the paired positive/negative controls that prove each one fires; this file
//! applies the gate to the packages `mjx-xlsx` really writes.
//!
//! Excel is the format where the gate has the most to say, because a workbook's own `styles.xml` is
//! a set of tables every cell indexes into: `c@s` names a `cellXfs` record, an `xf` names a font, a
//! fill, a border and a number format, and font 0 names the theme twice over
//! (`<color theme="1"/>`, `<scheme val="minor"/>`). None of that is checked by the schema, by
//! `Package::validate` or by byte identity.

use mjx_chart::{ChartData, ChartKind};
use mjx_dml::spreadsheet_drawing::CellMarker;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_schema_gate::{assert_authored_package_resolves_every_reference, audit_package_references};
use mjx_sml::write::{CellFormatSpec, CellFormatTarget};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

/// A cell reference from a literal address.
fn at(address: &str) -> CellReference {
    CellReference::parse(address).expect("a literal address")
}

/// The two-series chart the cases below author.
fn chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Plan", [1.0, 2.0])
        .series("Actual", [3.0, 4.0])
}

/// Anchors `chart` on the first tab of `workbook`.
fn add_a_chart(workbook: &mut Workbook) {
    workbook
        .add_chart(
            0,
            &chart(),
            CellMarker::new(0, 0, 10, 0),
            CellMarker::new(5, 0, 25, 0),
            "Plan",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("the chart is added");
}

/// Every workbook this crate's authoring surface can produce resolves every reference it makes.
#[test]
fn every_authored_workbook_resolves_every_reference_it_makes() {
    let mut with_chart = Workbook::blank().expect("a blank workbook");
    add_a_chart(&mut with_chart);

    let mut with_cells = Workbook::blank().expect("a blank workbook");
    with_cells
        .set_cell_value(0, at("A1"), CellValue::Number(1.0))
        .expect("a number");
    let interned = with_cells
        .intern_shared_string("Plan")
        .expect("a shared string");
    with_cells
        .set_cell_value(0, at("A2"), CellValue::SharedString(interned))
        .expect("the interned string in a cell");

    let mut with_a_second_sheet = Workbook::blank().expect("a blank workbook");
    with_a_second_sheet.add_sheet("Data").expect("a second tab");

    // A cell that names a format is the SpreadsheetML half of the class: `c@s` is an index into
    // `cellXfs`, and an index is a reference like any other.
    let mut with_a_format = Workbook::blank().expect("a blank workbook");
    let format = with_a_format
        .append_cell_format(CellFormatTarget::CellFormats, &CellFormatSpec::default())
        .expect("an xf");
    with_a_format
        .set_cell_value(0, at("A1"), CellValue::Number(1.0))
        .expect("a number");
    with_a_format
        .set_cell_style(0, at("A1"), Some(format))
        .expect("the cell names the format");

    let mut everything = Workbook::blank().expect("a blank workbook");
    let interned = everything
        .intern_shared_string("Plan")
        .expect("a shared string");
    everything
        .set_cell_value(0, at("A1"), CellValue::SharedString(interned))
        .expect("the interned string in a cell");
    add_a_chart(&mut everything);

    for (label, workbook) in [
        ("a blank workbook", Workbook::blank().expect("blank")),
        ("a workbook with a chart", with_chart),
        ("a workbook with cells", with_cells),
        ("a workbook with a second tab", with_a_second_sheet),
        ("a workbook with a cell format", with_a_format),
        ("a workbook with cells and a chart", everything),
    ] {
        let bytes = workbook.save().expect("it saves");
        assert_authored_package_resolves_every_reference(label, &bytes);
    }
}

/// **The gate catches G4's own defect.** Removing the theme from a workbook we authored — the state
/// every `.xlsx` this library wrote was in before MJXOFF-200 — is reported twice over: once for each
/// chart series that can no longer resolve an accent, and once for the `styles.xml` font 0 whose
/// `<color theme="1"/>` and `<scheme val="minor"/>` now name nothing.
///
/// Without this case the suite above would be green whether or not the gate can see anything.
#[test]
fn taking_the_theme_back_out_reddens_the_gate() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    add_a_chart(&mut workbook);
    let mut package = mjx_opc::Package::open(&workbook.save().expect("it saves")).expect("reopens");
    let theme = mjx_opc::PartName::new("/xl/theme/theme1.xml").expect("a literal part name");
    package
        .remove_part_cascading(&theme)
        .expect("the theme goes");

    let audit = audit_package_references(&package.save_unchecked().expect("it saves unchecked"));
    assert!(!audit.is_clean(), "the gate must see the theme is gone");

    let series: Vec<String> = audit
        .dangling
        .iter()
        .filter(|dangling| dangling.site.starts_with("c:ser"))
        .map(|dangling| dangling.reference.clone())
        .collect();
    assert_eq!(
        series,
        ["accent1", "accent2"],
        "one implicit reference per series:\n{}",
        audit.report()
    );

    let font_zero: Vec<String> = audit
        .dangling
        .iter()
        .filter(|dangling| dangling.part == "/xl/styles.xml")
        .map(|dangling| dangling.site.clone())
        .collect();
    assert_eq!(
        font_zero,
        ["color@theme", "scheme@val"],
        "font 0's two theme references, which are F5's whole point:\n{}",
        audit.report()
    );
}

/// **The gate catches an index into a style table that is not there**, which is the same class one
/// layer down: a `c@s` naming a `cellXfs` record the file does not hold.
///
/// Proved by taking `styles.xml` out of a workbook we authored rather than by hand-writing markup,
/// so the assertion is about this crate's own output.
#[test]
fn taking_the_styles_part_out_reddens_the_gate_too() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let format = workbook
        .append_cell_format(CellFormatTarget::CellFormats, &CellFormatSpec::default())
        .expect("an xf");
    workbook
        .set_cell_value(0, at("A1"), CellValue::Number(1.0))
        .expect("a number");
    workbook
        .set_cell_style(0, at("A1"), Some(format))
        .expect("the cell names the format");
    let mut package = mjx_opc::Package::open(&workbook.save().expect("it saves")).expect("reopens");
    let styles = mjx_opc::PartName::new("/xl/styles.xml").expect("a literal part name");
    package
        .remove_part_cascading(&styles)
        .expect("the styles part goes");

    let audit = audit_package_references(&package.save_unchecked().expect("it saves unchecked"));
    assert!(
        audit
            .dangling
            .iter()
            .any(|dangling| dangling.site == "c@s" && dangling.reason.contains("no styles.xml")),
        "the report must name the cell whose @s now indexes into nothing:\n{}",
        audit.report()
    );
}
