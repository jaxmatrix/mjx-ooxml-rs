//! MJXOFF-94's "Done when": all 39 `EG_RPrBase` members read, author and round-trip
//! canonicalisation-equal on `tests/fixtures/run_properties.docx`; the three `w:rPr` emptiness states
//! round-trip byte-identically; setting one run's colour leaves every other run's `w:rPr` bytes
//! untouched; an explicit `w:val="0"` reads as off, not absent; and the schema gate stays green on
//! every authored variant.
//!
//! The mechanism-level proofs for the emptiness states and the `ST_OnOff` default (would a mutation
//! that always emits self-closing, or one that drops explicit `false`, actually turn something red)
//! live in `crates/mjx-docx/src/document/run_properties.rs`'s own `#[cfg(test)]` module, which calls
//! `to_xml()` directly — a *guaranteed* full rebuild, per that trait's own doc comment — to prove the
//! mechanism in isolation. This file is what a caller of the crate actually gets: the real fixture,
//! opened through `Package`/`MainDocument` exactly the way `Document`'s own methods do internally
//! (`Document` itself grows no new accessors here — MJXOFF-94's brief asks for typed getters and
//! setters on `RunProperties`, reached off `Run::run_properties`, not a new Document-level surface).

use mjx_docx::{Color, MainDocument, Package, PartName};
use mjx_fixtures::fixture;
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_ooxml_types::shared::VerticalTextPosition;
use mjx_ooxml_types::wordprocessingml::{
    BorderStyle, CombineBrackets, EmphasisMark, FontTypeHint, HalfPointMeasure, HexadecimalColor,
    HighlightColor, ShadingPattern, SignedHalfPointMeasure, SignedTwipsMeasure,
    TextEffect as TextEffectKind, TextScale, ThemeColor, TwoDigitHexadecimalNumber,
    Underline as UnderlineKind,
};

const DOCUMENT_XML: &str = "/word/document.xml";

/// Every `EG_RPrBase` member, read back from paragraph 1's one run — the fixture's "every property"
/// run — matching exactly what `tests/fixtures/run_properties.docx` was authored with.
///
/// **Would this pass if the work were not done?** No: a missing or misnamed accessor fails to
/// compile; a wrong wire local reads back `None`/the schema default instead of the seeded value. This
/// is exactly the class of defect a missing `prefix = "w"` produced during development — every single
/// attribute here read the schema default instead of the file's own value until that was fixed, and
/// this test (run against the fixture, not just the isolated unit tests) is what would have caught it
/// end to end.
#[test]
fn every_eg_rprbase_member_reads_back_from_the_seeded_fixture() {
    let mut package = Package::open(&fixture("run_properties.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;
    let main = MainDocument::from_xml(&doc.root, interner).expect("parse w:document");

    let body = main.body().expect("the fixture has a body");
    let paragraph = body.paragraph(1).expect("the second paragraph");
    let run = paragraph.run(0).expect("its one run");
    let rpr = run.run_properties().expect("the run carries w:rPr");

    // The twenty CT_OnOff members. w:b is the explicit-off case; every other bare element (present,
    // no val) reads on; w:iCs w:val="1" is present-and-explicitly-on.
    assert_eq!(rpr.bold(interner), Ok(Some(false)), "w:b w:val=\"0\"");
    assert_eq!(rpr.bold_complex_script(interner), Ok(Some(true)), "w:bCs");
    assert_eq!(rpr.italic(interner), Ok(Some(true)), "w:i");
    assert_eq!(
        rpr.italic_complex_script(interner),
        Ok(Some(true)),
        "w:iCs w:val=\"1\""
    );
    assert_eq!(rpr.all_capitals(interner), Ok(Some(true)), "w:caps");
    assert_eq!(rpr.small_caps(interner), Ok(Some(true)), "w:smallCaps");
    assert_eq!(rpr.strikethrough(interner), Ok(Some(true)), "w:strike");
    assert_eq!(
        rpr.double_strikethrough(interner),
        Ok(Some(true)),
        "w:dstrike"
    );
    assert_eq!(rpr.outline(interner), Ok(Some(true)), "w:outline");
    assert_eq!(rpr.shadow(interner), Ok(Some(true)), "w:shadow");
    assert_eq!(rpr.embossing(interner), Ok(Some(true)), "w:emboss");
    assert_eq!(rpr.imprinting(interner), Ok(Some(true)), "w:imprint");
    assert_eq!(rpr.proofing_exempt(interner), Ok(Some(true)), "w:noProof");
    assert_eq!(rpr.snap_to_grid(interner), Ok(Some(true)), "w:snapToGrid");
    assert_eq!(rpr.hidden(interner), Ok(Some(true)), "w:vanish");
    assert_eq!(rpr.web_hidden(interner), Ok(Some(true)), "w:webHidden");
    assert_eq!(rpr.right_to_left(interner), Ok(Some(true)), "w:rtl");
    assert_eq!(rpr.complex_script(interner), Ok(Some(true)), "w:cs");
    assert_eq!(rpr.always_hidden(interner), Ok(Some(true)), "w:specVanish");
    assert_eq!(rpr.math(interner), Ok(Some(true)), "w:oMath");

    // rStyle, rFonts.
    let style = rpr.character_style().expect("w:rStyle");
    assert_eq!(
        style.style_id(interner),
        Ok(std::borrow::Cow::Borrowed("Strong"))
    );
    let fonts = rpr.fonts().expect("w:rFonts");
    assert_eq!(fonts.hint(interner), Ok(Some(FontTypeHint::EastAsia)));
    assert_eq!(
        fonts.ascii_font(interner),
        Ok(None),
        "no font name was seeded"
    );

    // color.
    let color = rpr.color().expect("w:color");
    assert_eq!(
        color.hex_value(interner),
        Ok(HexadecimalColor::from_wire("FF0000"))
    );
    assert_eq!(color.theme_color(interner), Ok(Some(ThemeColor::Accent1)));
    assert_eq!(
        color.theme_tint(interner),
        Ok(Some(TwoDigitHexadecimalNumber::from_wire("80")))
    );
    assert_eq!(
        color.theme_shade(interner),
        Ok(Some(TwoDigitHexadecimalNumber::from_wire("40")))
    );

    // The three measures reused from one shape (sz/szCs/kern) and the two dedicated ones.
    assert_eq!(
        rpr.font_size(interner),
        Ok(Some(HalfPointMeasure::from_wire("28")))
    );
    assert_eq!(
        rpr.complex_script_font_size(interner),
        Ok(Some(HalfPointMeasure::from_wire("28")))
    );
    assert_eq!(
        rpr.kerning(interner),
        Ok(Some(HalfPointMeasure::from_wire("16")))
    );
    let position = rpr.vertical_offset().expect("w:position");
    assert_eq!(
        position.half_points(interner),
        Ok(SignedHalfPointMeasure::from_wire("-4"))
    );
    let spacing = rpr.character_spacing().expect("w:spacing");
    assert_eq!(
        spacing.twentieths_of_a_point(interner),
        Ok(SignedTwipsMeasure::from_wire("20"))
    );
    let scale = rpr.character_scale().expect("w:w");
    assert_eq!(
        scale.percentage(interner),
        Ok(Some(TextScale::from_wire("150%")))
    );

    // highlight, u (color and themeColor alongside val — the ticket's own seeded case), effect.
    let highlight = rpr.highlight().expect("w:highlight");
    assert_eq!(highlight.color(interner), Ok(HighlightColor::Yellow));
    let underline = rpr.underline().expect("w:u");
    assert_eq!(underline.style(interner), Ok(Some(UnderlineKind::Wave)));
    assert_eq!(
        underline.color(interner),
        Ok(HexadecimalColor::from_wire("00FF00"))
    );
    assert_eq!(
        underline.theme_color(interner),
        Ok(Some(ThemeColor::Accent2))
    );
    let effect = rpr.text_effect().expect("w:effect");
    assert_eq!(effect.kind(interner), Ok(TextEffectKind::SparklingLights));

    // bdr, shd.
    let border = rpr.border().expect("w:bdr");
    assert_eq!(border.style(interner), Ok(BorderStyle::Single));
    assert_eq!(
        border.color(interner),
        Ok(HexadecimalColor::from_wire("0000FF"))
    );
    assert_eq!(border.width_eighths_of_a_point(interner), Ok(Some(8)));
    assert_eq!(border.spacing_points(interner), Ok(1));
    let shading = rpr.shading().expect("w:shd");
    assert_eq!(shading.pattern(interner), Ok(ShadingPattern::Percent10));
    assert_eq!(
        shading.color(interner),
        Ok(Some(HexadecimalColor::from_wire("auto")))
    );
    assert_eq!(
        shading.fill_color(interner),
        Ok(Some(HexadecimalColor::from_wire("FFFF00")))
    );

    // fitText, vertAlign, em, lang, eastAsianLayout.
    let fit_text = rpr.manual_run_width().expect("w:fitText");
    assert_eq!(
        fit_text.width(interner).map(|w| w.to_wire().to_owned()),
        Ok("1440".to_owned())
    );
    assert_eq!(fit_text.id(interner), Ok(Some(1)));
    let vert_align = rpr.vertical_alignment().expect("w:vertAlign");
    assert_eq!(
        vert_align.position(interner),
        Ok(VerticalTextPosition::Superscript)
    );
    let em = rpr.emphasis_mark().expect("w:em");
    assert_eq!(em.mark(interner), Ok(EmphasisMark::Dot));
    let lang = rpr.languages().expect("w:lang");
    assert_eq!(
        lang.latin(interner)
            .map(|v| v.map(|t| t.to_wire().to_owned())),
        Ok(Some("en-US".to_owned()))
    );
    assert_eq!(
        lang.east_asian(interner)
            .map(|v| v.map(|t| t.to_wire().to_owned())),
        Ok(Some("ja-JP".to_owned()))
    );
    assert_eq!(
        lang.complex_script(interner)
            .map(|v| v.map(|t| t.to_wire().to_owned())),
        Ok(Some("ar-SA".to_owned()))
    );
    let layout = rpr.east_asian_layout().expect("w:eastAsianLayout");
    assert_eq!(layout.id(interner), Ok(Some(2)));
    assert_eq!(layout.combine_two_lines(interner), Ok(Some(true)));
    assert_eq!(
        layout.combine_brackets(interner),
        Ok(Some(CombineBrackets::Round))
    );
    assert_eq!(layout.vertical(interner), Ok(Some(false)));
    assert_eq!(layout.vertical_compressed(interner), Ok(Some(true)));
}

/// The three `w:rPr` emptiness states as the fixture's first paragraph actually carries them
/// (self-closed, absent, separate end tag), forced through a full parse/rebuild of
/// `word/document.xml` — the same "read everything, then write back with no logical change" idiom
/// `run_content_docx_round_trips_with_the_model_materialized` uses — and compared byte for byte.
///
/// As MJXOFF-92 found for its own similar test: this **black-box** form is not, by itself, proof
/// that the *mechanism* (rather than `write_back`'s span-preserving copy of an untouched subtree) is
/// what keeps the three states apart — see this file's module doc for where that proof actually
/// lives. What this test proves is the contract a caller of the crate depends on: open the fixture,
/// touch nothing, save, and get the same bytes back.
#[test]
fn the_three_rpr_emptiness_states_round_trip_byte_identically() {
    let original = fixture("run_properties.docx");
    let mut package = Package::open(&original).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");

    {
        let doc = package
            .part_tree_mut(&part)
            .expect("edit word/document.xml");
        let RawDocument { interner, root, .. } = doc;
        let main = MainDocument::from_xml(root, interner).expect("parse w:document");
        // Read every paragraph and run, forcing the whole tree through the typed model — mirrors
        // `run_content_docx_round_trips_with_the_model_materialized`.
        let body = main.body().expect("has a body");
        for index in 0..body.paragraph_count() {
            let paragraph = body.paragraph(index).expect("paragraph exists");
            let _ = paragraph.text();
        }
        main.write_back(root, interner);
    }

    let saved = package.save().expect("save");
    let original_pkg = Package::open(&original).expect("open original");
    let saved_pkg = Package::open(&saved).expect("open saved");
    for (before, after) in original_pkg.entries().iter().zip(saved_pkg.entries()) {
        assert_eq!(
            before.bytes(),
            after.bytes(),
            "decompressed bytes changed for {}",
            before.name
        );
    }

    mjx_schema_gate::assert_deck_is_in_schema_order("saved run_properties.docx", &saved);
    mjx_schema_gate::assert_authored_deck_is_schema_valid("saved run_properties.docx", &saved);
}

/// Setting one run's colour (paragraph 2's run, which starts with only `w:b`) leaves every other
/// part, and every other run's `w:rPr` bytes inside `word/document.xml`, untouched — the same
/// edit-isolation contract `content_model.rs`'s
/// `editing_one_run_leaves_every_other_part_and_the_sibling_paragraph_byte_identical` proves for
/// plain text, proved here for a `RunProperties` setter. The edited document also stays schema
/// valid, closing MJXOFF-94's "the schema gate is green on every authored variant".
#[test]
fn setting_one_runs_colour_leaves_every_other_runs_run_properties_bytes_untouched() {
    let original = fixture("run_properties.docx");
    let original_pkg = Package::open(&original).expect("open original");

    let mut package = Package::open(&original).expect("open for editing");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    {
        let doc = package
            .part_tree_mut(&part)
            .expect("edit word/document.xml");
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner).expect("parse w:document");
        let body = main.body_mut().expect("has a body");
        let paragraph = body.paragraph_mut(2).expect("the edit-isolation paragraph");
        let run = paragraph.run_mut(0).expect("its one run");
        let rpr = run.run_properties_or_insert(interner);
        rpr.set_color(Some(Color::new(interner, "00FF00")));
        main.write_back(root, interner);
    }
    let edited = package.save().expect("save");
    let edited_pkg = Package::open(&edited).expect("open edited");

    // The "every property" run's own w:rPr, verbatim in both the original and the edited fixture —
    // the literal substring this test's isolation claim rests on.
    let every_property_rpr = "<w:rPr><w:rStyle w:val=\"Strong\"/><w:rFonts w:hint=\"eastAsia\"/>\
<w:b w:val=\"0\"/><w:bCs/><w:i/><w:iCs w:val=\"1\"/><w:caps/><w:smallCaps/><w:strike/><w:dstrike/>\
<w:outline/><w:shadow/><w:emboss/><w:imprint/><w:noProof/><w:snapToGrid/><w:vanish/><w:webHidden/>\
<w:color w:val=\"FF0000\" w:themeColor=\"accent1\" w:themeTint=\"80\" w:themeShade=\"40\"/>\
<w:spacing w:val=\"20\"/><w:w w:val=\"150%\"/><w:kern w:val=\"16\"/><w:position w:val=\"-4\"/>\
<w:sz w:val=\"28\"/><w:szCs w:val=\"28\"/><w:highlight w:val=\"yellow\"/>\
<w:u w:val=\"wave\" w:color=\"00FF00\" w:themeColor=\"accent2\"/><w:effect w:val=\"sparkle\"/>\
<w:bdr w:val=\"single\" w:sz=\"8\" w:space=\"1\" w:color=\"0000FF\"/>\
<w:shd w:val=\"pct10\" w:color=\"auto\" w:fill=\"FFFF00\"/><w:fitText w:val=\"1440\" w:id=\"1\"/>\
<w:vertAlign w:val=\"superscript\"/><w:rtl/><w:cs/><w:em w:val=\"dot\"/>\
<w:lang w:val=\"en-US\" w:eastAsia=\"ja-JP\" w:bidi=\"ar-SA\"/>\
<w:eastAsianLayout w:id=\"2\" w:combine=\"1\" w:combineBrackets=\"round\" w:vert=\"0\" \
w:vertCompress=\"1\"/><w:specVanish/><w:oMath/></w:rPr>";

    let original_document_xml = extract_document_xml(&original);
    assert!(
        original_document_xml.contains(every_property_rpr),
        "the literal substring this test relies on is not in the original fixture:\n{original_document_xml}"
    );

    for (before, after) in original_pkg.entries().iter().zip(edited_pkg.entries()) {
        assert_eq!(before.name, after.name, "part order changed");
        if before.name == "word/document.xml" {
            assert_ne!(
                before.bytes(),
                after.bytes(),
                "editing a run's colour must actually dirty word/document.xml"
            );
            let edited_document_xml = extract_document_xml(&edited);
            assert!(
                edited_document_xml.contains(every_property_rpr),
                "the untouched \"every property\" run's w:rPr bytes must survive verbatim:\n\
                 {edited_document_xml}"
            );
            assert!(
                edited_document_xml.contains("<w:color w:val=\"00FF00\"/>"),
                "the edited run's new colour must be present:\n{edited_document_xml}"
            );
        } else {
            assert_eq!(
                before.bytes(),
                after.bytes(),
                "editing one run's colour must not change {}",
                before.name
            );
        }
    }

    mjx_schema_gate::assert_deck_is_in_schema_order("edited run_properties.docx", &edited);
    mjx_schema_gate::assert_authored_deck_is_schema_valid("edited run_properties.docx", &edited);
}

/// Extracts `word/document.xml`'s decompressed text from a `.docx`'s bytes.
fn extract_document_xml(bytes: &[u8]) -> String {
    let package = Package::open(bytes).expect("open package");
    let entry = package
        .entries()
        .iter()
        .find(|entry| entry.name == "word/document.xml")
        .expect("word/document.xml is in the package");
    String::from_utf8(
        entry
            .bytes()
            .expect("word/document.xml has decompressed bytes")
            .to_vec(),
    )
    .expect("word/document.xml is UTF-8")
}
