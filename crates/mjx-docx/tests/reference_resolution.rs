//! MJXOFF-200 — every reference a document **this library authored** makes resolves.
//!
//! The class, not the instance. G4's defect was a chart with no bars, caused by a scheme colour
//! with no theme to resolve against; a test asserting *"the package has a theme part"* would close
//! that one case and pass the day someone wrote an empty theme. So the assertion here is the
//! general one — [`mjx_schema_gate::assert_authored_package_resolves_every_reference`], whose rules
//! and whose deliberate limits are in that module's own documentation, and whose paired
//! positive/negative controls are in `crates/mjx-schema-gate/tests/reference_resolution.rs`.
//!
//! **The population is packages we author**, on purpose. A producer's file may contain whatever it
//! contains — repairing one is the opposite of what this library is for — but a package *we* wrote
//! and cannot resolve is our defect. So each case below builds a document through the public
//! surface, saves it, and audits the bytes.

use mjx_chart::{ChartData, ChartKind};
use mjx_docx::{ChartPlacement, ChartWrap, Document, PageSize};
use mjx_ooxml_types::wordprocessingdrawing::WrapText;
use mjx_schema_gate::{assert_authored_package_resolves_every_reference, audit_package_references};

/// The two-series chart the cases below author. Two series so that more than one accent slot is
/// actually indexed into.
fn chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Plan", [1.0, 2.0])
        .series("Actual", [3.0, 4.0])
}

/// A blank document with one paragraph to hang things on.
fn blank() -> Document {
    Document::blank(PageSize::a4()).expect("a blank document")
}

/// Every document this crate's authoring surface can produce resolves every reference it makes.
#[test]
fn every_authored_document_resolves_every_reference_it_makes() {
    let mut inline = blank();
    inline
        .add_chart(0usize, &chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("an inline chart");

    let mut floating = blank();
    floating
        .add_chart_placed(
            0usize,
            &chart(),
            4_572_000,
            2_743_200,
            "Revenue",
            ChartPlacement::Floating {
                offset_x_emu: 0,
                offset_y_emu: 0,
                wrap: ChartWrap::Square(WrapText::BothSides),
            },
        )
        .expect("a floating chart");

    let mut two_charts = blank();
    for _ in 0..2 {
        two_charts
            .add_chart(0usize, &chart(), 4_572_000, 2_743_200, "Revenue")
            .expect("a chart");
    }

    for (label, document) in [
        ("a blank document", blank()),
        ("a document with an inline chart", inline),
        ("a document with a floating chart", floating),
        ("a document with two charts", two_charts),
    ] {
        let bytes = document.save().expect("it saves");
        assert_authored_package_resolves_every_reference(label, &bytes);
    }
}

/// **The gate catches G4's own defect.** Removing the theme from a document we authored — the exact
/// state every `.docx` this library wrote was in before MJXOFF-200 — is reported as a scheme colour
/// with nothing to resolve against.
///
/// Without this case the suite above would be green whether or not the gate can see anything: a
/// package that resolves everything and a gate that checks nothing look identical from the outside.
#[test]
fn taking_the_theme_back_out_reddens_the_gate() {
    let mut document = blank();
    document
        .add_chart(0usize, &chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("a chart");
    let mut package = mjx_opc::Package::open(&document.save().expect("it saves")).expect("reopens");
    let theme = mjx_opc::PartName::new("/word/theme/theme1.xml").expect("a literal part name");
    package
        .remove_part_cascading(&theme)
        .expect("the theme goes");

    let audit = audit_package_references(&package.save_unchecked().expect("it saves unchecked"));
    assert!(!audit.is_clean(), "the gate must see the theme is gone");

    // Not merely "a relationship target went missing" — that would be true of removing any part, and
    // `Package::validate` already says it. The report has to name the **chart series** and the
    // accent slot it can no longer resolve, once per series, because that is the sentence a reader
    // of a red build needs: *this is why there are no bars*.
    let series: Vec<String> = audit
        .dangling
        .iter()
        .filter(|dangling| dangling.site.starts_with("c:ser"))
        .map(|dangling| dangling.reference.clone())
        .collect();
    assert_eq!(
        series,
        ["accent1", "accent2"],
        "one implicit reference per series, cycling the accents:\n{}",
        audit.report()
    );
}
