//! MJXOFF-96's "Done when": all 33 `CT_PPrBase` members read, author and round-trip
//! canonicalisation-equal on `tests/fixtures/paragraph_properties.docx` (the fixture this child
//! seeds — no existing fixture carries `w:line`/`w:tabs`/`w:ind`/`w:pBdr`/`w:framePr`); setting a
//! paragraph's justification leaves the paragraph-mark `w:rPr` untouched and vice versa (proved at
//! the type level in `paragraph_properties.rs`'s own `#[cfg(test)]`); and editing one paragraph's
//! spacing leaves every other paragraph and every other part byte-identical.
//!
//! Opened through `Package`/`MainDocument` directly, exactly the way `run_properties.rs`'s own
//! integration test is — `Document` itself grows no new accessors here, matching MJXOFF-94's own
//! precedent that a child adding typed properties does not also add a `Document`-level surface for
//! them.

use mjx_docx::{MainDocument, Package, ParagraphBorders, ParagraphProperties, PartName, Spacing};
use mjx_fixtures::fixture;
use mjx_ooxml_core::{FromXml, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::shared::{
    RelativeHorizontalAlignment, RelativeVerticalAlignment, TwipsMeasure,
};
use mjx_ooxml_types::wordprocessingml::{
    BorderStyle, DropCap, HeightRule, HorizontalAnchor, Justification, LineSpacingRule,
    ShadingPattern, SignedTwipsMeasure, TabStopLeader, TabStopType, TextBoxTightWrap,
    TextFlowDirection, TextFrameWrapping, VerticalAnchor, VerticalTextAlignment,
};

const DOCUMENT_XML: &str = "/word/document.xml";

fn open_fixture() -> (Package, PartName) {
    let package = Package::open(&fixture("paragraph_properties.docx")).expect("open the fixture");
    let part = PartName::new(DOCUMENT_XML).expect("a valid part name");
    (package, part)
}

/// Would this pass if the work were not done? No: a missing or misnamed accessor fails to compile;
/// a wrong wire local or a dropped attribute reads back `None`/the schema default instead of the
/// seeded value — exactly the class of defect `run_properties.rs`'s own equivalent test caught
/// (missing `prefix = "w"`).
#[test]
fn every_ct_pprbase_member_reads_back_from_the_seeded_fixture() {
    let (mut package, part) = open_fixture();
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;
    let main = MainDocument::from_xml(&doc.root, interner).expect("parse w:document");

    let body = main.body().expect("the fixture has a body");
    let paragraph = body.paragraph(0).expect("the first paragraph");
    let ppr = paragraph.properties().expect("the paragraph carries w:pPr");

    assert_eq!(
        ppr.style()
            .map(|s| s.style_id(interner).map(|v| v.into_owned())),
        Some(Ok("Heading1".to_owned()))
    );
    assert_eq!(ppr.keep_with_next(interner), Ok(Some(true)));
    assert_eq!(ppr.keep_lines_together(interner), Ok(Some(false)));
    assert_eq!(ppr.page_break_before(interner), Ok(Some(true)));

    let frame = ppr.frame().expect("w:framePr");
    assert_eq!(frame.drop_cap(interner), Ok(Some(DropCap::Drop)));
    assert_eq!(frame.drop_cap_lines(interner), Ok(Some(3)));
    assert_eq!(
        frame.width(interner),
        Ok(Some(TwipsMeasure::from_wire("1440")))
    );
    assert_eq!(
        frame.height(interner),
        Ok(Some(TwipsMeasure::from_wire("720")))
    );
    assert_eq!(
        frame.vertical_spacing(interner),
        Ok(Some(TwipsMeasure::from_wire("120")))
    );
    assert_eq!(
        frame.horizontal_spacing(interner),
        Ok(Some(TwipsMeasure::from_wire("120")))
    );
    assert_eq!(frame.wrap(interner), Ok(Some(TextFrameWrapping::Around)));
    assert_eq!(
        frame.horizontal_anchor(interner),
        Ok(Some(HorizontalAnchor::Margin))
    );
    assert_eq!(
        frame.vertical_anchor(interner),
        Ok(Some(VerticalAnchor::Page))
    );
    assert_eq!(
        frame.x(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("100")))
    );
    assert_eq!(
        frame.x_alignment(interner),
        Ok(Some(RelativeHorizontalAlignment::Left))
    );
    assert_eq!(
        frame.y(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("200")))
    );
    assert_eq!(
        frame.y_alignment(interner),
        Ok(Some(RelativeVerticalAlignment::Top))
    );
    assert_eq!(frame.height_rule(interner), Ok(Some(HeightRule::Exact)));
    assert_eq!(frame.anchor_lock(interner), Ok(Some(true)));

    assert_eq!(ppr.widow_control(interner), Ok(Some(false)));

    let numbering = ppr.numbering().expect("w:numPr");
    assert_eq!(numbering.level(interner), Ok(Some(1)));
    assert_eq!(numbering.numbering_id(interner), Ok(Some(5)));

    assert_eq!(ppr.suppress_line_numbers(interner), Ok(Some(true)));

    let borders = ppr.borders().expect("w:pBdr");
    for accessor in [
        ParagraphBorders::top,
        ParagraphBorders::left,
        ParagraphBorders::bottom,
        ParagraphBorders::right,
    ] {
        let border = accessor(borders).expect("each of top/left/bottom/right is present");
        assert_eq!(border.style(interner), Ok(BorderStyle::Single));
        assert_eq!(border.width_eighths_of_a_point(interner), Ok(Some(8)));
    }
    let between = borders.between().expect("w:between");
    assert_eq!(between.style(interner), Ok(BorderStyle::Dashed));
    let bar = borders.bar().expect("w:bar");
    assert_eq!(bar.style(interner), Ok(BorderStyle::Dashed));

    let shading = ppr.shading().expect("w:shd");
    assert_eq!(shading.pattern(interner), Ok(ShadingPattern::Clear));
    assert_eq!(
        shading
            .fill_color(interner)
            .map(|c| c.map(|v| v.to_wire().to_owned())),
        Ok(Some("D9D9D9".to_owned()))
    );

    let tabs = ppr.tab_stops().expect("w:tabs");
    let seeded: Vec<_> = tabs.tabs().collect();
    assert_eq!(seeded.len(), 2);
    assert_eq!(seeded[0].alignment(interner), Ok(TabStopType::Left));
    assert_eq!(
        seeded[0].position(interner),
        Ok(SignedTwipsMeasure::from_wire("720"))
    );
    assert_eq!(seeded[1].alignment(interner), Ok(TabStopType::Right));
    assert_eq!(seeded[1].leader(interner), Ok(Some(TabStopLeader::Dot)));
    assert_eq!(
        seeded[1].position(interner),
        Ok(SignedTwipsMeasure::from_wire("4320"))
    );

    assert_eq!(ppr.suppress_auto_hyphens(interner), Ok(Some(true)));
    assert_eq!(
        ppr.east_asian_line_breaking_rules(interner),
        Ok(Some(false))
    );
    assert_eq!(ppr.word_wrap(interner), Ok(Some(true)));
    assert_eq!(ppr.overflow_punctuation(interner), Ok(Some(false)));
    assert_eq!(
        ppr.compress_punctuation_at_line_start(interner),
        Ok(Some(true))
    );
    assert_eq!(
        ppr.auto_space_latin_and_east_asian(interner),
        Ok(Some(false))
    );
    assert_eq!(
        ppr.auto_space_east_asian_and_numbers(interner),
        Ok(Some(true))
    );
    assert_eq!(ppr.right_to_left_layout(interner), Ok(Some(false)));
    assert_eq!(
        ppr.adjust_right_indent_for_document_grid(interner),
        Ok(Some(true))
    );
    assert_eq!(ppr.snap_to_grid(interner), Ok(Some(false)));

    let spacing = ppr.spacing().expect("w:spacing");
    assert_eq!(
        spacing.before(interner),
        Ok(Some(TwipsMeasure::from_wire("120")))
    );
    assert_eq!(
        spacing.after(interner),
        Ok(Some(TwipsMeasure::from_wire("240")))
    );
    let line = spacing
        .line_spacing(interner)
        .expect("valid")
        .expect("w:line present");
    assert_eq!(line.rule, LineSpacingRule::Auto);
    assert_eq!(line.value, SignedTwipsMeasure::from_wire("360"));

    let ind = ppr.indentation().expect("w:ind");
    assert_eq!(
        ind.leading_edge(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("720")))
    );
    assert_eq!(
        ind.trailing_edge(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("360")))
    );
    assert_eq!(
        ind.first_line(interner),
        Ok(Some(TwipsMeasure::from_wire("240")))
    );

    assert_eq!(ppr.contextual_spacing(interner), Ok(Some(true)));
    assert_eq!(ppr.mirror_indents(interner), Ok(Some(false)));
    assert_eq!(ppr.suppress_overlap(interner), Ok(Some(true)));

    assert_eq!(
        ppr.alignment().map(|a| a.value(interner)),
        Some(Ok(Justification::Justified))
    );
    assert_eq!(
        ppr.text_direction().map(|t| t.value(interner)),
        Some(Ok(TextFlowDirection::LeftToRightTopToBottom))
    );
    assert_eq!(
        ppr.vertical_character_alignment()
            .map(|v| v.value(interner)),
        Some(Ok(VerticalTextAlignment::Baseline))
    );
    assert_eq!(
        ppr.text_box_tight_wrap().map(|t| t.value(interner)),
        Some(Ok(TextBoxTightWrap::AllLines))
    );

    assert_eq!(ppr.outline_level(interner), Ok(Some(2)));
    assert_eq!(ppr.associated_html_div_id(interner), Ok(Some(123_456_789)));

    let cnf = ppr.conditional_formatting().expect("w:cnfStyle");
    assert_eq!(
        cnf.bitmask(interner)
            .map(|b| b.map(|v| v.to_wire().to_owned())),
        Ok(Some("100000000000".to_owned()))
    );
    assert_eq!(cnf.first_row(interner), Ok(Some(true)));
    assert_eq!(cnf.last_row(interner), Ok(Some(false)));

    let mark = ppr
        .paragraph_mark_properties()
        .expect("the paragraph mark carries w:rPr");
    assert_eq!(mark.bold(interner), Ok(Some(true)));
    assert_eq!(
        mark.color().map(|c| c.hex_value(interner)),
        Some(Ok(
            mjx_ooxml_types::wordprocessingml::HexadecimalColor::from_wire("FF0000")
        ))
    );
}

/// The physical `w:left`/`w:right`/`w:hanging` spelling and a `w:tab` with `val="clear"` — neither
/// carried by any fixture before this child.
#[test]
fn physical_indentation_and_a_clear_tab_stop_read_back_from_the_second_paragraph() {
    let (mut package, part) = open_fixture();
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;
    let main = MainDocument::from_xml(&doc.root, interner).expect("parse w:document");
    let body = main.body().expect("the fixture has a body");
    let paragraph = body.paragraph(1).expect("the second paragraph");
    let ppr = paragraph.properties().expect("carries w:pPr");

    let ind = ppr.indentation().expect("w:ind");
    assert_eq!(
        ind.left(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("480")))
    );
    assert_eq!(
        ind.right(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("240")))
    );
    assert_eq!(
        ind.leading_edge(interner),
        Ok(Some(SignedTwipsMeasure::from_wire("480"))),
        "no w:start present, so the physical spelling is what leading_edge falls back to"
    );
    assert_eq!(
        ind.hanging(interner),
        Ok(Some(TwipsMeasure::from_wire("240")))
    );

    let tabs = ppr.tab_stops().expect("w:tabs");
    let seeded: Vec<_> = tabs.tabs().collect();
    assert_eq!(seeded.len(), 2);
    assert_eq!(seeded[0].alignment(interner), Ok(TabStopType::Clear));
    assert_eq!(
        ppr.alignment().map(|a| a.value(interner)),
        Some(Ok(Justification::Distribute))
    );
}

const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// The full round-trip proof: extract each seeded `w:pPr`'s own verbatim source bytes (via its
/// retained [`RawElement::source_span`]), parse that fragment standalone, rebuild through `to_xml`
/// (a guaranteed full rebuild, per that trait's own doc comment), and compare bytes.
/// `ParagraphProperties` never reorders children already present — see the module's own doc comment
/// — so an already schema-ordered `w:pPr` (what a real, valid file always carries) must reproduce
/// exactly.
///
/// The extracted fragment carries no `xmlns:w` of its own (it inherits the declaration from
/// `w:document`), so it is re-declared on the same opening tag before parsing — both seeded
/// fragments open with a bare `<w:pPr>`, confirmed against the fixture generator.
#[test]
fn the_seeded_paragraphs_ppr_round_trips_canonicalisation_equal() {
    let (mut package, part) = open_fixture();
    let raw_document_xml = package
        .part_bytes(&part)
        .expect("the part starts unedited")
        .to_vec();
    let doc = package.part_tree(&part).expect("read word/document.xml");
    let interner = &doc.interner;

    for index in 0..2 {
        let ppr = paragraph_properties_element(&doc.root, interner, index)
            .expect("w:pPr present in the raw tree");
        let span = ppr
            .source_span()
            .unwrap_or_else(|| panic!("paragraph {index}'s w:pPr has no retained span"));
        let fragment = &raw_document_xml[span.start as usize..span.end as usize];
        let fragment_str = String::from_utf8_lossy(fragment);
        assert!(
            fragment_str.starts_with("<w:pPr>"),
            "paragraph {index}'s w:pPr must open with a bare <w:pPr> for this test's namespace \
             injection to be valid, got: {fragment_str}"
        );
        let standalone = fragment_str.replacen(
            "<w:pPr>",
            &format!(r#"<w:pPr xmlns:w="{WML_NAMESPACE}">"#),
            1,
        );

        let mut fragment_doc = mjx_xml::fidelity::parse(standalone.as_bytes())
            .expect("the fragment parses standalone");
        let typed = ParagraphProperties::from_xml(&fragment_doc.root, &fragment_doc.interner)
            .expect("parses as ParagraphProperties");
        fragment_doc.root = typed.to_xml(&mut fragment_doc.interner);
        let out = mjx_xml::fidelity::serialize_to_vec(&fragment_doc);
        assert_eq!(
            String::from_utf8_lossy(&out),
            standalone,
            "paragraph {index}'s w:pPr must round-trip through the typed model unchanged"
        );
    }
}

/// Finds the `w:pPr` element that is the `index`th `w:p`'s own child, in the raw tree.
fn paragraph_properties_element<'a>(
    root: &'a RawElement,
    interner: &mjx_ooxml_core::Interner,
    index: usize,
) -> Option<&'a RawElement> {
    let body = root.children.iter().find_map(|node| match node {
        RawNode::Element(element) if interner.resolve(element.name.local) == "body" => {
            Some(element)
        }
        _ => None,
    })?;
    let paragraph = body
        .children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if interner.resolve(element.name.local) == "p" => {
                Some(element)
            }
            _ => None,
        })
        .nth(index)?;
    paragraph.children.iter().find_map(|node| match node {
        RawNode::Element(element) if interner.resolve(element.name.local) == "pPr" => Some(element),
        _ => None,
    })
}

/// The retained [`RawElement::source_span`] of the second `<w:p>` under `<w:body>`.
fn second_paragraph_span(doc: &RawDocument) -> Option<std::ops::Range<u32>> {
    let body = doc.root.children.iter().find_map(|node| match node {
        RawNode::Element(element) if doc.interner.resolve(element.name.local) == "body" => {
            Some(element)
        }
        _ => None,
    })?;
    body.children
        .iter()
        .filter_map(|node| match node {
            RawNode::Element(element) if doc.interner.resolve(element.name.local) == "p" => {
                Some(element)
            }
            _ => None,
        })
        .nth(1)
        .and_then(RawElement::source_span)
}

/// The core byte-identity gate: editing one paragraph's spacing must not disturb any other
/// paragraph's retained source bytes, nor any other part in the package.
///
/// Would this pass if the work were not done? No: `sample.docx`'s own edit-isolation test found (and
/// MJXOFF-92 documented) that a fixture with no incidental formatting can reproduce identical bytes
/// even from a full model reflow — so this asserts the *mechanism*
/// ([`RawElement::source_span`] surviving untouched, not merely equal bytes) exactly as that test
/// does, plus every other **part**'s bytes, which a per-element span check alone would not cover.
#[test]
fn editing_one_paragraphs_spacing_leaves_every_other_paragraph_and_part_byte_identical() {
    let bytes = fixture("paragraph_properties.docx");
    let mut package = Package::open(&bytes).expect("open the fixture");
    let document_part = PartName::new(DOCUMENT_XML).expect("a valid part name");

    let other_parts: Vec<PartName> = package
        .part_names()
        .filter(|name| name != &document_part)
        .collect();
    let other_parts_before: Vec<(PartName, Vec<u8>)> = other_parts
        .iter()
        .map(|name| {
            (
                name.clone(),
                package
                    .part_bytes(name)
                    .expect("every other part starts unedited")
                    .to_vec(),
            )
        })
        .collect();

    let span_before = {
        let doc = package.part_tree(&document_part).expect("read");
        second_paragraph_span(doc)
    };
    assert!(
        span_before.is_some(),
        "a freshly parsed, never-touched element always has a span"
    );

    {
        let doc = package.part_tree_mut(&document_part).expect("mutate");
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner).expect("parse w:document");
        let body = main.body_mut().expect("the fixture has a body");
        let paragraph = body.paragraph_mut(0).expect("the first paragraph");
        let properties = paragraph.properties_or_insert(interner);
        let mut spacing = properties
            .spacing()
            .cloned()
            .unwrap_or_else(|| Spacing::new(interner));
        spacing.set_before(interner, Some(TwipsMeasure::from_wire("500")));
        properties.set_spacing(Some(spacing));
        main.write_back(root, interner);
    }

    let span_after = {
        let doc = package.part_tree(&document_part).expect("read");
        second_paragraph_span(doc)
    };
    assert_eq!(
        span_before, span_after,
        "editing paragraph 0's spacing must not disturb paragraph 1's retained source span"
    );

    for (name, before) in &other_parts_before {
        let after = package
            .part_bytes(name)
            .unwrap_or_else(|| panic!("part {name:?} must still be present, untouched"));
        assert_eq!(
            after,
            before.as_slice(),
            "part {name:?} must stay byte-identical after editing only word/document.xml"
        );
    }
}
