//! MJXOFF-108 at the **package** tier: `Workbook` finds the styles part and the worksheet, and
//! answers what a cell's format is.
//!
//! # What this suite is for, and what it deliberately does not do
//!
//! The resolution order itself — cell → row → column → the default record, then per aspect the
//! direct `cellXfs` record or the `cellStyleXfs` one beneath it — is `mjx-sml`'s, and
//! `crates/mjx-sml/tests/effective_cell_format.rs` is what pins it, against a fixture whose two
//! layers disagree on every aspect. Repeating those assertions here would be a second copy of a
//! specification, free to drift from the first.
//!
//! So this file asserts the two things that are genuinely this crate's:
//!
//! 1. **The right two parts are found**, through the workbook's own relationship graph, and the
//!    answer they produce is the same one the markup tier gives for the same cell. A
//!    [`Workbook::effective_cell_format`] wired to the wrong sheet or the wrong styles part passes
//!    every `mjx-sml` test and fails here.
//! 2. **Reading does not dirty the package** — asserted on the fixture whose whole point is that it
//!    is read heavily.

use mjx_fixtures::fixture;
use mjx_opc::{Package, PartProvenance};
use mjx_sml::styles::effective::{CellFormatResolver, FormatLayer, StyleIndexSource};
use mjx_sml::{CellReference, ColumnStyles, StylesheetPart};
use mjx_xlsx::Workbook;

/// The fixture MJXOFF-108 authored: two `xf` layers that disagree on all six aspects.
const FIXTURE: &str = "effective_cell_format.xlsx";

#[test]
fn the_package_tier_finds_both_parts_and_answers_what_the_markup_tier_answers() {
    let bytes = fixture(FIXTURE);
    let workbook = Workbook::open(&bytes).expect("the fixture opens");
    assert_eq!(
        workbook
            .parts()
            .styles
            .as_ref()
            .expect("the workbook relates to a styles part")
            .as_str(),
        "/xl/styles.xml"
    );

    let formatting = workbook
        .sheet_formatting(0)
        .expect("the sheet's formatting reads")
        .expect("sheet 0 is a worksheet and the workbook has styles");
    let resolver = formatting.resolver().expect("the xf tables decode");

    // The same four cells the markup-tier suite walks, answered through the package.
    let expected = [
        ("C2", StyleIndexSource::Cell, 3),
        ("B2", StyleIndexSource::Row, 6),
        ("B3", StyleIndexSource::Column, 7),
        ("F4", StyleIndexSource::Default, 0),
    ];
    for (reference, source, style_index) in expected {
        let address = CellReference::parse(reference).expect("a cell reference");
        let format = resolver
            .effective_cell_format(address)
            .expect("the cell resolves");
        assert_eq!(format.style_index_source(), source, "{reference}");
        assert_eq!(format.style_index(), style_index, "{reference}");

        // The one-shot form on `Workbook` must agree with the held resolver, or one of the two is
        // reaching for a different part.
        let once = workbook
            .effective_cell_format(0, address)
            .expect("the one-shot form resolves")
            .expect("the sheet has formatting");
        assert_eq!(once, format, "{reference}: the two forms disagree");
    }

    // And the answer is the one the markup tier produces from the same two parts, resolved by hand
    // here so that a `sheet_formatting` pointing at the wrong sheet cannot agree with itself.
    let styles_bytes = Package::open(&bytes)
        .expect("reopen")
        .part_bytes(&mjx_opc::PartName::new("/xl/styles.xml").expect("a part name"))
        .expect("the styles part")
        .to_vec();
    let document = mjx_xml::fidelity::parse(&styles_bytes).expect("the styles part parses");
    let stylesheet = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    let markup_resolver =
        CellFormatResolver::new(&stylesheet, &document.interner).expect("the resolver builds");
    let worksheet = workbook
        .worksheet_markup(0)
        .expect("the worksheet reads")
        .expect("sheet 0 is a worksheet");
    let cells = worksheet.sheet_data().expect("the sheet writes sheetData");
    let columns = ColumnStyles::read(worksheet.column_blocks(), worksheet.interner())
        .expect("the col runs decode");

    let address = CellReference::parse("A1").expect("a cell reference");
    let by_hand = markup_resolver
        .effective_cell_format(
            cells.cell(address).as_ref(),
            cells.row(1).as_ref(),
            columns.style_index(1),
        )
        .expect("A1 resolves");
    let through_package = resolver
        .effective_cell_format(address)
        .expect("A1 resolves through the package");
    assert_eq!(by_hand, through_package);

    // A1 is the disagreement cell: `s="1"`, every applyX false, so every aspect comes from the
    // layer beneath. Asserted here too, because a package tier that resolved the *wrong sheet's*
    // A1 would still agree with itself above.
    assert_eq!(through_package.style_index(), 1);
    assert_eq!(through_package.font().layer, FormatLayer::CellStyle);
    assert_eq!(through_package.font().resource_index, Some(2));
}

#[test]
fn resolving_every_cell_of_the_fixture_never_dirties_a_part() {
    let bytes = fixture(FIXTURE);
    let workbook = Workbook::open(&bytes).expect("the fixture opens");

    let formatting = workbook
        .sheet_formatting(0)
        .expect("the sheet's formatting reads")
        .expect("sheet 0 has formatting");
    let resolver = formatting.resolver().expect("the xf tables decode");
    for reference in ["A1", "B1", "C1", "D1", "E1", "B2", "C2", "B3", "F4", "Z99"] {
        let address = CellReference::parse(reference).expect("a cell reference");
        resolver
            .effective_cell_format(address)
            .unwrap_or_else(|error| panic!("{reference}: {error}"));
    }
    let _ = workbook.styles_markup().expect("the styles part reads");

    let saved = workbook.save().expect("the package saves");
    let original = Package::open(&bytes).expect("reopen the fixture");
    let written = Package::open(&saved).expect("reopen what was written");
    assert_eq!(original.entries().len(), written.entries().len());
    for (before, after) in original.entries().iter().zip(written.entries()) {
        assert_eq!(before.name, after.name);
        assert_eq!(
            before.bytes(),
            after.bytes(),
            "{} was rewritten, so resolving a format dirtied it",
            before.name
        );
        assert_eq!(
            after.provenance(),
            PartProvenance::FromContainer,
            "{} must still come from the container",
            after.name
        );
    }
}

/// A workbook that relates to no styles part has nothing to resolve against, and says so rather
/// than inventing a default stylesheet.
#[test]
fn a_workbook_with_no_styles_part_answers_none() {
    let mut package = Package::open(&fixture(FIXTURE)).expect("the fixture opens");
    let rels = mjx_opc::PartName::new("/xl/_rels/workbook.xml.rels").expect("a part name");
    let stripped = String::from_utf8(
        package
            .part_bytes(&rels)
            .expect("the workbook's rels")
            .to_vec(),
    )
    .expect("the rels are UTF-8")
    .replace(
        r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
        "",
    );
    package
        .replace_part_bytes(&rels, stripped.into_bytes())
        .expect("drop the styles relationship");
    let bytes = package.save_unchecked().expect("write it out");

    let workbook = Workbook::open(&bytes).expect("it still opens");
    assert!(workbook.parts().styles.is_none());
    assert!(workbook.styles_markup().expect("no styles part").is_none());
    assert!(workbook
        .sheet_formatting(0)
        .expect("no styles part")
        .is_none());
    let address = CellReference::parse("A1").expect("a cell reference");
    assert!(workbook
        .effective_cell_format(0, address)
        .expect("no styles part")
        .is_none());
}
