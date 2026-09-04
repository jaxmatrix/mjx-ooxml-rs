//! Headers, footers and the legacy VML they carry (MJXOFF-113).
//!
//! Two fixtures, both authored through this crate's own public API (never a template):
//!
//! - `header_footer_variants.docx` — two sections. Section 1 (paragraph 0's own `w:sectPr`) states
//!   all three header variants and all three footer variants, distinguishable by their own text, and
//!   carries no `w:titlePg`. Section 2 (the body-level `w:sectPr`) states none at all. This is the
//!   fixture the ticket's own trap names: a resolver that always answers "the first (or only)
//!   reference" passes against a document shaped like this one only by accident, because every
//!   assertion below is chosen specifically to disagree with that answer.
//! - `header_watermark.docx` — one header whose content is hand-authored `mc:AlternateContent`
//!   wrapping a legacy `w:pict` VML watermark (`v:shapetype`/`v:shape`/`v:textpath`), the shape real
//!   Word output uses. `regenerate_fixtures` (below, `#[ignore]`) is how both were produced; it is
//!   kept only as the record of how, not run by `cargo test`.

use mjx_docx::{Document, HeaderFooterType, PageMargins, PageSize, Run, SectionLocation};
use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName};

fn variants_fixture() -> Document {
    Document::open(&fixture("header_footer_variants.docx"))
        .expect("open header_footer_variants.docx")
}

fn watermark_fixture() -> Document {
    Document::open(&fixture("header_watermark.docx")).expect("open header_watermark.docx")
}

// -------------------------------------------------------------------------------------------
// header_footer_variants.docx — structure and byte identity.
// -------------------------------------------------------------------------------------------

#[test]
fn the_fixture_has_two_sections_and_six_header_footer_parts() {
    let mut document = variants_fixture();
    let section_count = document
        .sections(|spans, _| spans.len())
        .expect("read sections");
    assert_eq!(
        section_count, 2,
        "paragraph 0 ends section 1; the body-level sectPr is section 2"
    );
    assert_eq!(document.parts().headers.len(), 3);
    assert_eq!(document.parts().footers.len(), 3);
}

#[test]
fn every_header_and_footer_variant_round_trips_byte_identically_on_an_unrelated_edit() {
    let original = fixture("header_footer_variants.docx");
    let mut document = Document::open(&original).expect("open fixture");

    let mut before = Vec::new();
    for part in document
        .parts()
        .headers
        .clone()
        .into_iter()
        .chain(document.parts().footers.clone())
    {
        let bytes = {
            let mut probe = Document::open(&original).expect("reopen for byte snapshot");
            probe
                .header_footer(&part, |_content, _interner| ())
                .expect("read header/footer");
            // Reading alone must not dirty the part — snapshot straight from the untouched package.
            Package::open(&original)
                .expect("open package")
                .part_bytes(&part)
                .expect("part exists")
                .to_vec()
        };
        before.push((part, bytes));
    }
    assert_eq!(
        before.len(),
        6,
        "three header variants and three footer variants"
    );

    // The unrelated edit: a new run in paragraph 1 (section 2's own paragraph), nothing about a
    // header or footer.
    document
        .append_run(1, "Edited elsewhere.")
        .expect("edit an unrelated paragraph");
    let saved = document.save().expect("save the edited document");
    let saved_package = Package::open(&saved).expect("reopen saved package");

    for (part, original_bytes) in before {
        let after_bytes = saved_package
            .part_bytes(&part)
            .unwrap_or_else(|| panic!("{part:?} missing after save"));
        assert_eq!(
            after_bytes,
            &original_bytes[..],
            "{part:?} must survive an edit to an unrelated paragraph byte-identically"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Variant resolution — would these pass if resolution were "always the first/only reference"?
// No: every assertion below specifically disagrees with that answer.
// -------------------------------------------------------------------------------------------

#[test]
fn with_title_pg_off_the_first_header_exists_but_is_not_chosen() {
    let mut document = variants_fixture();
    // Would fail if resolution ignored `w:titlePg` and simply returned the `first` reference because
    // it exists: the resolved part must be the DEFAULT header, not the FIRST header.
    let resolved = document
        .resolve_header(0, HeaderFooterType::First)
        .expect("resolve")
        .expect("some header applies");
    let default_part = document
        .resolve_header(0, HeaderFooterType::Default)
        .expect("resolve")
        .expect("some header applies");
    assert_eq!(
        resolved, default_part,
        "titlePg is off, so a First query must resolve exactly as a Default query would"
    );
    assert_ne!(
        resolved,
        document.parts().headers[2],
        "the First header part exists in the package but must not be the one titlePg-off resolves to"
    );
}

#[test]
fn flipping_title_pg_on_changes_which_header_and_footer_apply() {
    let mut document = variants_fixture();
    let before_header = document
        .resolve_header(0, HeaderFooterType::First)
        .expect("resolve")
        .expect("some header applies");
    let before_footer = document
        .resolve_footer(0, HeaderFooterType::First)
        .expect("resolve")
        .expect("some footer applies");

    document
        .edit_section_properties(
            SectionLocation::Paragraph(0.into()),
            |properties, interner| {
                properties.set_title_page(interner, Some(true));
            },
        )
        .expect("flip titlePg on");

    let after_header = document
        .resolve_header(0, HeaderFooterType::First)
        .expect("resolve")
        .expect("some header applies");
    let after_footer = document
        .resolve_footer(0, HeaderFooterType::First)
        .expect("resolve")
        .expect("some footer applies");

    assert_ne!(
        before_header, after_header,
        "a resolver that ignores w:titlePg answers the same header before and after — this must go red"
    );
    assert_ne!(before_footer, after_footer, "same, for footers");
    assert_eq!(
        after_header,
        document.parts().headers[2],
        "the winner is now the First header"
    );
    assert_eq!(
        after_footer,
        document.parts().footers[2],
        "the winner is now the First footer"
    );
}

#[test]
fn with_even_and_odd_headers_off_the_even_header_exists_but_is_not_chosen() {
    let mut document = variants_fixture();
    assert!(
        !document.even_and_odd_headers().expect("read the flag"),
        "the fixture relates to no settings.xml at all — the schema default (false) must hold"
    );
    let resolved = document
        .resolve_header(0, HeaderFooterType::Even)
        .expect("resolve")
        .expect("some header applies");
    let default_part = document
        .resolve_header(0, HeaderFooterType::Default)
        .expect("resolve")
        .expect("some header applies");
    assert_eq!(resolved, default_part);
    assert_ne!(
        resolved,
        document.parts().headers[1],
        "the Even header exists but must not win"
    );
}

#[test]
fn flipping_even_and_odd_headers_on_changes_which_header_and_footer_apply_to_an_even_page() {
    let original = fixture("header_footer_variants.docx");
    let mut document = Document::open(&original).expect("open fixture");
    let before_header = document
        .resolve_header(0, HeaderFooterType::Even)
        .expect("resolve")
        .expect("some header applies");
    let before_footer = document
        .resolve_footer(0, HeaderFooterType::Even)
        .expect("resolve")
        .expect("some footer applies");

    // `w:evenAndOddHeaders` lives in `settings.xml`, a part this document relates to none of and a
    // part this crate deliberately does not model (MJXOFF-136's own scope) — so the flag is flipped
    // the one way available: add the part directly through the packaging layer, then hand the result
    // back to `Document::from_package`, exactly the constructor a caller who already holds a `Package`
    // uses.
    let mut package = Package::open(&original).expect("reopen as a package");
    let document_part = PartName::new("/word/document.xml").expect("document part name");
    let settings_part = PartName::new("/word/settings.xml").expect("settings part name");
    package
        .insert_part(
            &settings_part,
            mjx_docx::constants::CONTENT_TYPE_SETTINGS,
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:evenAndOddHeaders/></w:settings>"#.to_vec(),
        )
        .expect("insert settings.xml");
    package
        .add_relationship(
            Some(&document_part),
            mjx_opc::Relationship {
                id: "rIdEvenOdd".to_owned(),
                rel_type: mjx_docx::constants::REL_SETTINGS.to_owned(),
                target: "settings.xml".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )
        .expect("relate settings.xml");
    let mut document = Document::from_package(package).expect("reopen with the flag on");

    assert!(document.even_and_odd_headers().expect("read the flag"));

    let after_header = document
        .resolve_header(0, HeaderFooterType::Even)
        .expect("resolve")
        .expect("some header applies");
    let after_footer = document
        .resolve_footer(0, HeaderFooterType::Even)
        .expect("resolve")
        .expect("some footer applies");

    assert_ne!(
        before_header, after_header,
        "a resolver that ignores w:evenAndOddHeaders answers the same header before and after — \
         this must go red"
    );
    assert_ne!(before_footer, after_footer, "same, for footers");
    assert_eq!(
        after_header,
        document.parts().headers[1],
        "the winner is now the Even header"
    );
    assert_eq!(
        after_footer,
        document.parts().footers[1],
        "the winner is now the Even footer"
    );
}

#[test]
fn a_section_with_no_reference_at_all_inherits_every_variant_from_the_previous_section() {
    let mut document = variants_fixture();
    for kind in [
        HeaderFooterType::Default,
        HeaderFooterType::Even,
        HeaderFooterType::First,
    ] {
        let section_1 = document
            .resolve_header(0, kind)
            .expect("resolve section 1")
            .expect("some header applies");
        let section_2 = document
            .resolve_header(1, kind)
            .expect("resolve section 2")
            .expect("section 2 must inherit, not come back empty");
        assert_eq!(
            section_1, section_2,
            "section 2 states no headerReference at all, so every variant must inherit \
             section 1's own — {kind:?} did not"
        );

        let footer_1 = document
            .resolve_footer(0, kind)
            .expect("resolve")
            .expect("some footer");
        let footer_2 = document
            .resolve_footer(1, kind)
            .expect("resolve")
            .expect("inherited");
        assert_eq!(footer_1, footer_2, "same, for footers — {kind:?}");
    }
}

#[test]
fn out_of_range_section_index_is_a_typed_error() {
    let mut document = variants_fixture();
    let error = document
        .resolve_header(99, HeaderFooterType::Default)
        .unwrap_err();
    assert!(matches!(
        error,
        mjx_docx::DocxError::SectionOutOfRange {
            index: 99,
            count: 2
        }
    ));
}

// -------------------------------------------------------------------------------------------
// Creating and removing a header/footer on demand — package stays valid, reference lands right.
// -------------------------------------------------------------------------------------------

#[test]
fn creating_a_header_for_a_section_that_lacks_one_yields_a_valid_package_with_the_reference_placed_correctly(
) {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    assert!(
        document.parts().headers.is_empty(),
        "a blank document relates to no header yet"
    );

    let part = document
        .create_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("create a default header for the body-level section");

    assert_eq!(document.parts().headers, vec![part.clone()]);
    let resolved = document
        .resolve_header(0, HeaderFooterType::Default)
        .expect("resolve")
        .expect("the just-created header applies");
    assert_eq!(resolved, part);

    document
        .validate()
        .expect("Package::validate must accept the new part and relationship");
    let saved = document
        .save()
        .expect("save must succeed too (validate + serialize)");

    // Re-open independently and confirm the reference is really inside w:sectPr, not merely
    // resolvable through this same in-memory Document.
    let mut reopened = Document::open(&saved).expect("reopen the saved package");
    let placed = reopened
        .sections(|spans, interner| {
            spans[0]
                .properties
                .as_ref()
                .expect("body-level sectPr")
                .header_references()
                .any(|reference| reference.kind(interner) == Ok(HeaderFooterType::Default))
        })
        .expect("read sections");
    assert!(
        placed,
        "w:headerReference must be inside the section's own w:sectPr"
    );
}

#[test]
fn removing_a_header_removes_its_reference_part_and_relationship() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    let part = document
        .create_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("create header");
    assert!(document
        .resolve_header(0, HeaderFooterType::Default)
        .unwrap()
        .is_some());

    document
        .remove_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("remove it");

    assert!(
        document
            .resolve_header(0, HeaderFooterType::Default)
            .expect("resolve")
            .is_none(),
        "no reference should resolve any more"
    );
    assert!(
        document.parts().headers.is_empty(),
        "the part-graph view must drop the removed part"
    );
    document
        .validate()
        .expect("removing must still leave a valid package");
    let saved = document.save().expect("save after removal");
    let package = Package::open(&saved).expect("reopen saved bytes");
    assert!(
        package.part_bytes(&part).is_none(),
        "the swept part must not still be in the container"
    );
}

#[test]
fn removing_a_header_that_does_not_exist_is_a_no_op_and_never_fabricates_a_sectpr() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    let before = document.save().expect("save before");
    document
        .remove_header(SectionLocation::Body, HeaderFooterType::First)
        .expect("removing nothing must not error");
    let after = document.save().expect("save after");
    assert_eq!(
        before, after,
        "a no-op removal must not touch word/document.xml at all"
    );
}

// -------------------------------------------------------------------------------------------
// header_watermark.docx — VML through mjx_vml, not a second model.
// -------------------------------------------------------------------------------------------

#[test]
fn the_watermark_fixture_actually_carries_vml_content() {
    // Proves the fixture itself, independent of this crate's reader: the header part's raw bytes
    // contain real VML markup, not merely an AlternateContent wrapper with nothing inside it.
    let document = watermark_fixture();
    let part = document.parts().headers[0].clone();
    let bytes = Package::open(&fixture("header_watermark.docx"))
        .expect("open package")
        .part_bytes(&part)
        .expect("header part exists")
        .to_vec();
    let xml = String::from_utf8(bytes).expect("utf8");
    assert!(
        xml.contains("<v:shape"),
        "the fixture must contain a real VML shape, not just wrapper markup"
    );
    assert!(
        xml.contains("<v:textpath"),
        "and the watermark's own text path"
    );
    assert!(
        xml.contains("mc:AlternateContent"),
        "wrapped in mc:AlternateContent, as real Word output is"
    );
}

#[test]
fn the_watermark_reads_through_mjx_vml_not_a_second_model() {
    let mut document = watermark_fixture();
    let part = document.parts().headers[0].clone();
    let drawings = document
        .header_footer_vml_drawings(&part)
        .expect("resolve mc:AlternateContent and read the VML");
    assert_eq!(drawings.len(), 1, "exactly one w:pict in the fixture");

    let drawing = &drawings[0];
    // These are `mjx_vml::Drawing`/`mjx_vml::Shape`/`mjx_vml::TextBox`... — MJXOFF-58's own types.
    // If this crate had grown a second VML model, these calls would not compile against them.
    let shapes: Vec<_> = drawing.shapes().collect();
    assert_eq!(shapes.len(), 1, "one v:shape");
    // The interner these attribute reads need is a private detail of `Drawing` in this API shape —
    // this test only needs to prove the model recognizes the shape as a shape, which `shapes()`
    // returning it already does; the shape's own attributes are `mjx_vml`'s own suite's job.
    let _ = shapes;
}

#[test]
fn a_document_carrying_the_watermark_survives_an_unrelated_edit_byte_identically() {
    let original = fixture("header_watermark.docx");
    let mut document = Document::open(&original).expect("open fixture");
    let header = document.parts().headers[0].clone();
    let before = Package::open(&original)
        .expect("open original package")
        .part_bytes(&header)
        .expect("header exists")
        .to_vec();

    document
        .append_paragraph()
        .expect("an edit with nothing to do with the header");
    document
        .append_run(1, "Body text, unrelated to the watermark.")
        .expect("append run");
    let saved = document.save().expect("save");

    let after = Package::open(&saved)
        .expect("open saved package")
        .part_bytes(&header)
        .expect("header still present")
        .to_vec();
    assert_eq!(
        before, after,
        "the watermark header must survive an unrelated body edit byte-for-byte"
    );

    // And it must still read through mjx_vml after the round trip.
    let mut reopened = Document::open(&saved).expect("reopen saved bytes");
    let drawings = reopened
        .header_footer_vml_drawings(&header)
        .expect("still resolves and parses as VML");
    assert_eq!(drawings.len(), 1);
}

// -------------------------------------------------------------------------------------------
// Fixture generation — not run by `cargo test`; kept as the record of how the two fixtures above
// were produced, entirely through this crate's own public API (headers/footers) plus one literal
// VML fragment (this crate models no VML-authoring surface of its own).
// -------------------------------------------------------------------------------------------

#[test]
#[ignore = "one-shot generator for the committed fixtures; run manually with --ignored"]
fn regenerate_fixtures() {
    std::fs::write(
        mjx_fixtures::fixtures_dir().join("header_footer_variants.docx"),
        build_variants_fixture(),
    )
    .expect("write header_footer_variants.docx");
    std::fs::write(
        mjx_fixtures::fixtures_dir().join("header_watermark.docx"),
        build_watermark_fixture(),
    )
    .expect("write header_watermark.docx");
}

fn build_variants_fixture() -> Vec<u8> {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    document.append_paragraph().expect("second paragraph");

    document
        .edit_section_properties(
            SectionLocation::Paragraph(0.into()),
            |properties, interner| {
                properties.set_page_size(interner, Some(PageSize::a4()));
                properties.set_page_margins(interner, Some(PageMargins::NORMAL));
            },
        )
        .expect("give paragraph 0 its own section (section 1); the body-level sectPr is section 2");

    let variants = [
        (
            HeaderFooterType::Default,
            "Default header",
            "Default footer",
        ),
        (HeaderFooterType::Even, "Even header", "Even footer"),
        (HeaderFooterType::First, "First header", "First footer"),
    ];
    for (kind, header_text, footer_text) in variants {
        let header = document
            .create_header(SectionLocation::Paragraph(0.into()), kind)
            .expect("create header");
        write_header_footer_text(&mut document, &header, header_text);
        let footer = document
            .create_footer(SectionLocation::Paragraph(0.into()), kind)
            .expect("create footer");
        write_header_footer_text(&mut document, &footer, footer_text);
    }

    document.save().expect("save header_footer_variants.docx")
}

fn write_header_footer_text(document: &mut Document, part: &PartName, text: &str) {
    document
        .edit_header_footer(part, |content, interner| {
            let paragraph = content
                .paragraph_mut(0)
                .expect("freshly created part has one paragraph");
            paragraph.append_run(Run::with_text(interner, text));
        })
        .expect("write header/footer text");
}

fn build_watermark_fixture() -> Vec<u8> {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let header = document
        .create_header(SectionLocation::Body, HeaderFooterType::Default)
        .expect("create a default header");
    let bytes = document.save().expect("intermediate save");

    // The typed model has no VML-authoring surface (this crate is preserve/read-first for legacy
    // VML — MJXOFF-131 is where body w:pict gets one, and even that is not authoring). The one
    // literal fragment in this file: real, hand-verified watermark markup, in the exact shape Word
    // itself emits — `mc:AlternateContent` wrapping a `w:pict` fallback around a `v:shapetype`,
    // `v:shape` and `v:textpath`.
    let mut package = Package::open(&bytes).expect("reopen the intermediate package");
    package
        .replace_part_bytes(&header, watermark_header_xml())
        .expect("replace the header's content with the watermark markup");
    package.save().expect("serialize the watermark package")
}

fn watermark_header_xml() -> Vec<u8> {
    br##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" mc:Ignorable="wps">
  <w:p>
    <w:r>
      <w:rPr><w:noProof/></w:rPr>
      <mc:AlternateContent>
        <mc:Choice Requires="wps">
          <w:drawing/>
        </mc:Choice>
        <mc:Fallback>
          <w:pict>
            <v:shapetype id="_x0000_t136" coordsize="1600,21600" o:spt="136" adj="10800" path="m@7,0l@8,0m@5,21600l@6,21600e">
              <v:formulas>
                <v:f eqn="sum #0 0 10800"/>
              </v:formulas>
            </v:shapetype>
            <v:shape id="PowerPlusWaterMarkObject1" o:spid="_x0000_s1026" type="#_x0000_t136" style="position:absolute;margin-left:0;margin-top:0;width:415pt;height:207.5pt;rotation:315" o:allowincell="f" fillcolor="silver" stroked="f">
              <v:textpath style="font-family:&quot;Calibri&quot;;font-size:1pt" string="DRAFT"/>
            </v:shape>
          </w:pict>
        </mc:Fallback>
      </mc:AlternateContent>
    </w:r>
  </w:p>
</w:hdr>
"##
    .to_vec()
}
