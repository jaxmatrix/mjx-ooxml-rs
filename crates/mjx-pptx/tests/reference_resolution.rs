//! MJXOFF-200 — every reference a deck **this library authored** makes resolves.
//!
//! The class, not the instance. See `crates/mjx-schema-gate/src/references.rs` for the rules, their
//! deliberate limits, and the paired positive/negative controls that prove each one fires; this file
//! applies the gate to the packages `mjx-pptx` really writes.
//!
//! PowerPoint is the format the defect never showed up in, and the reason is worth pinning: a
//! `.pptx` always carries a theme because every slide master requires one, so the same chart markup
//! that painted nothing in a `.docx` painted blue and orange bars here. That makes the deck the
//! **control** for the whole of MJXOFF-200 rather than an afterthought — and it is why the negative
//! case below has to take the theme away by hand to make the gate speak.

use mjx_chart::{ChartData, ChartKind};
use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_pptx::{Presentation, ShapeBounds, SlideSize};
use mjx_schema_gate::{assert_authored_package_resolves_every_reference, audit_package_references};

fn widescreen() -> SlideSize {
    SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    }
}

fn blank() -> Presentation {
    Presentation::blank(widescreen()).expect("a blank deck")
}

/// The two-series chart the cases below author.
fn chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Plan", [1.0, 2.0])
        .series("Actual", [3.0, 4.0])
}

/// Every deck this crate's authoring surface can produce resolves every reference it makes.
#[test]
fn every_authored_deck_resolves_every_reference_it_makes() {
    let mut with_a_slide = blank();
    with_a_slide.add_slide().expect("a slide");

    let mut with_text = blank();
    with_text
        .add_slide_with_text(
            "Revenue by quarter",
            ShapeBounds::new(0, 0, 5_000_000, 1_000_000),
        )
        .expect("a slide with text");

    let mut with_a_table = blank();
    let slide = with_a_table.add_slide().expect("a slide");
    with_a_table
        .add_table(slide, 2, 2, ShapeBounds::new(0, 0, 1_000_000, 1_000_000))
        .expect("a table");

    let mut with_a_chart = blank();
    let slide = with_a_chart.add_slide().expect("a slide");
    with_a_chart
        .add_chart(
            slide,
            &chart(),
            ShapeBounds::new(0, 0, 4_572_000, 2_743_200),
        )
        .expect("a chart");

    for (label, deck) in [
        ("a blank deck", blank()),
        ("a deck with a slide", with_a_slide),
        ("a deck with text", with_text),
        ("a deck with a table", with_a_table),
        ("a deck with a chart", with_a_chart),
    ] {
        let bytes = deck.save().expect("it saves");
        assert_authored_package_resolves_every_reference(label, &bytes);
    }
}

/// **The deck is the control, and it only stays green because the theme is there.**
///
/// Taking `ppt/theme/theme1.xml` out of a deck we authored puts it in the state every `.docx` and
/// `.xlsx` this library wrote was in before MJXOFF-200 — and the gate reports the same thing here:
/// a chart series that can no longer resolve an accent, and every scheme colour the master and
/// layout state.
///
/// This is what stops the suite above from being a green that proves nothing.
#[test]
fn taking_the_theme_back_out_reddens_the_gate() {
    let mut deck = blank();
    let slide = deck.add_slide().expect("a slide");
    deck.add_chart(
        slide,
        &chart(),
        ShapeBounds::new(0, 0, 4_572_000, 2_743_200),
    )
    .expect("a chart");

    let mut package = mjx_opc::Package::open(&deck.save().expect("it saves")).expect("reopens");
    let theme = mjx_opc::PartName::new("/ppt/theme/theme1.xml").expect("a literal part name");
    package
        .remove_part_cascading(&theme)
        .expect("the theme goes");

    let audit = audit_package_references(&package.save_unchecked().expect("it saves unchecked"));
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
}
