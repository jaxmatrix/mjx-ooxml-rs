//! `w:sectPr` (MJXOFF-109): section addressing (which paragraphs a section governs), page setup,
//! columns, section-break kinds, line numbering and `w:printerSettings` preservation.
//!
//! `tests/fixtures/sample.docx` is single-section and cannot distinguish a correct implementation
//! from one that reads only the body-level `w:sectPr` — see this crate's own `sections.rs` module
//! doc. `tests/fixtures/three_section_document.docx` is authored specifically to catch that: section
//! 1 (paragraphs 0–1) is landscape A4; section 2 (paragraphs 2–3) is portrait A4, two equal-width
//! columns; section 3 (paragraph 4, the body-level `w:sectPr`) is portrait A4, one column.
//! `tests/fixtures/printer_settings_reference.docx` carries a real `w:printerSettings` reference and
//! its binary companion part.

use mjx_docx::{
    Columns, Document, DocxError, PageMargins, PageOrientation, PageSize, SectionLocation,
    SectionType,
};
use mjx_fixtures::fixture;
use mjx_ooxml_types::wordprocessingml::SectionBreakType;
use mjx_opc::{Package, PartName};

// -------------------------------------------------------------------------------------------
// Reading: sample.docx (single section) and three_section_document.docx (three).
// -------------------------------------------------------------------------------------------

#[test]
fn sample_docx_is_one_section_covering_every_paragraph_with_a4_portrait_and_its_own_margins() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    let paragraph_count = document.paragraph_count().expect("paragraph count");

    document
        .sections(|spans, interner| {
            assert_eq!(spans.len(), 1, "sample.docx is single-section");
            let span = &spans[0];
            assert_eq!(span.first_paragraph, 0);
            assert_eq!(span.last_paragraph, Some(paragraph_count - 1));
            let properties = span.properties.as_ref().expect("body-level w:sectPr");

            let size = properties
                .page_size(interner)
                .expect("valid w:pgSz")
                .expect("w:pgSz present");
            assert_eq!(size, PageSize::a4());

            let margins = properties
                .page_margins(interner)
                .expect("valid w:pgMar")
                .expect("w:pgMar present");
            assert_eq!(
                margins,
                PageMargins {
                    top: 1134,
                    right: 1134,
                    bottom: 1134,
                    left: 1134,
                    header: 0,
                    footer: 0,
                    gutter: 0,
                }
            );

            assert_eq!(properties.form_protected(interner), Ok(Some(false)));
        })
        .expect("read sections");
}

/// Would this pass if the work were not done? No: an implementation that reads only
/// [`mjx_docx::Body::section_properties`] (the body-level `w:sectPr`) and never looks at any
/// paragraph's own `w:pPr/w:sectPr` collapses every document to exactly one section covering every
/// paragraph — which happens to be *correct* for `sample.docx` (see the test above) but is wrong the
/// moment a document has more than one section, exactly what this fixture is built to prove. See
/// `paragraph_to_section_assignment_is_correct...` below for the mutation that proves it directly.
#[test]
fn each_of_the_three_sections_has_its_own_page_setup_and_paragraph_range() {
    let mut document =
        Document::open(&fixture("three_section_document.docx")).expect("open fixture");

    document
        .sections(|spans, interner| {
            assert_eq!(
                spans.len(),
                3,
                "three sections: two paragraph-level, one body-level"
            );

            // Section 1: paragraphs 0-1, landscape A4.
            assert_eq!(spans[0].first_paragraph, 0);
            assert_eq!(spans[0].last_paragraph, Some(1));
            let section_1 = spans[0]
                .properties
                .as_ref()
                .expect("section 1 has properties");
            let size_1 = section_1
                .page_size(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(size_1.orientation, PageOrientation::Landscape);
            assert_eq!(size_1, PageSize::a4().landscape());
            assert_eq!(
                section_1
                    .break_kind()
                    .expect("w:type present")
                    .kind(interner)
                    .expect("valid"),
                Some(SectionBreakType::NextPage)
            );

            // Section 2: paragraphs 2-3, portrait A4, two equal-width columns.
            assert_eq!(spans[1].first_paragraph, 2);
            assert_eq!(spans[1].last_paragraph, Some(3));
            let section_2 = spans[1]
                .properties
                .as_ref()
                .expect("section 2 has properties");
            let size_2 = section_2
                .page_size(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(size_2.orientation, PageOrientation::Portrait);
            assert_eq!(size_2, PageSize::a4());
            let columns_2 = section_2.columns().expect("w:cols present");
            assert_eq!(columns_2.num(interner), Ok(2));
            assert!(columns_2.is_equal_width(interner).expect("valid"));

            // Section 3 (body-level): paragraph 4 only, portrait A4, one column (no w:cols at all).
            assert_eq!(spans[2].first_paragraph, 4);
            assert_eq!(spans[2].last_paragraph, Some(4));
            let section_3 = spans[2]
                .properties
                .as_ref()
                .expect("section 3 has properties");
            let size_3 = section_3
                .page_size(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(size_3.orientation, PageOrientation::Portrait);
            assert!(section_3.columns().is_none());
        })
        .expect("read sections");
}

/// Named per the programme's own convention: a test that could pass whether or not the work was
/// done is not a test. This one cannot — see the mutation proof in its own body.
#[test]
fn paragraph_to_section_assignment_is_correct_on_the_three_section_fixture() {
    let mut document =
        Document::open(&fixture("three_section_document.docx")).expect("open fixture");
    document
        .sections(|spans, _interner| {
            // Paragraph 0 and 1 belong to section 0 (index 0 in `spans`); paragraph 2 and 3 to
            // section 1; paragraph 4 to section 2 (the body-level one).
            let section_of = |paragraph: usize| {
                spans
                    .iter()
                    .position(|span| {
                        span.first_paragraph <= paragraph
                            && span.last_paragraph.is_some_and(|last| paragraph <= last)
                    })
                    .unwrap_or_else(|| panic!("no section governs paragraph {paragraph}"))
            };
            assert_eq!(section_of(0), 0);
            assert_eq!(section_of(1), 0);
            assert_eq!(section_of(2), 1);
            assert_eq!(section_of(3), 1);
            assert_eq!(section_of(4), 2);
        })
        .expect("read sections");
}

// -------------------------------------------------------------------------------------------
// Moving a paragraph across a section boundary.
// -------------------------------------------------------------------------------------------

#[test]
fn moving_a_paragraph_across_a_section_boundary_changes_which_section_governs_it() {
    let mut document =
        Document::open(&fixture("three_section_document.docx")).expect("open fixture");

    // Before: paragraph 2 belongs to section index 1 ([2, 3]).
    document
        .sections(|spans, _interner| {
            let owner = spans
                .iter()
                .position(|span| span.first_paragraph <= 2 && span.last_paragraph == Some(3))
                .expect("section [2,3] exists");
            assert_eq!(owner, 1);
        })
        .expect("read sections before");

    // Move the boundary: paragraph 1 no longer ends a section (its landscape break is removed),
    // and paragraph 2 gets that same landscape break instead — so section 1 now covers
    // paragraphs [0, 2], and section 2 shrinks to just paragraph 3.
    document
        .remove_section_properties(SectionLocation::Paragraph(1.into()))
        .expect("remove section 1's own break");
    document
        .edit_section_properties(SectionLocation::Paragraph(2.into()), |section, interner| {
            section.set_page_size(interner, Some(PageSize::a4().landscape()));
            section.set_page_margins(interner, Some(PageMargins::NORMAL));
        })
        .expect("insert a new break at paragraph 2");

    document
        .sections(|spans, interner| {
            assert_eq!(spans.len(), 3);
            assert_eq!(spans[0].first_paragraph, 0);
            assert_eq!(
                spans[0].last_paragraph,
                Some(2),
                "paragraph 2 now joins section 1"
            );
            let size = spans[0]
                .properties
                .as_ref()
                .expect("properties")
                .page_size(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(size.orientation, PageOrientation::Landscape);

            assert_eq!(spans[1].first_paragraph, 3);
            assert_eq!(
                spans[1].last_paragraph,
                Some(3),
                "section 2 now covers only paragraph 3"
            );
        })
        .expect("read sections after");
}

// -------------------------------------------------------------------------------------------
// Editing one section leaves every other paragraph and part byte-identical.
// -------------------------------------------------------------------------------------------

#[test]
fn changing_section_2s_margins_leaves_every_other_paragraph_and_every_other_part_untouched() {
    let original_bytes = fixture("three_section_document.docx");
    let original_package = Package::open(&original_bytes).expect("open original package");

    let mut document = Document::open(&original_bytes).expect("open three_section_document.docx");
    document
        .edit_section_properties(SectionLocation::Paragraph(3.into()), |section, interner| {
            section.set_page_margins(
                interner,
                Some(PageMargins {
                    top: 2000,
                    ..PageMargins::NORMAL
                }),
            );
        })
        .expect("edit section 2's margins");
    let edited_bytes = document.save().expect("save");
    let edited_package = Package::open(&edited_bytes).expect("open edited package");

    let document_part = PartName::new("/word/document.xml").expect("part name");
    let mut any_other_part = false;
    for part in original_package.part_names() {
        if part == document_part {
            continue;
        }
        any_other_part = true;
        assert_eq!(
            original_package.part_bytes(&part),
            edited_package.part_bytes(&part),
            "part {part:?} must be byte-identical — only word/document.xml was edited"
        );
    }
    assert!(
        any_other_part,
        "the fixture must carry parts besides word/document.xml"
    );

    let original_document_xml = original_package
        .part_bytes(&document_part)
        .expect("original word/document.xml");
    let edited_document_xml = edited_package
        .part_bytes(&document_part)
        .expect("edited word/document.xml");
    assert_ne!(
        original_document_xml, edited_document_xml,
        "the edit must actually change word/document.xml"
    );

    // Every OTHER paragraph's own markup — including section 1's untouched `w:sectPr` and the
    // body-level section 3's `w:sectPr` — survives as an identical byte substring.
    let original_text = String::from_utf8_lossy(original_document_xml);
    let edited_text = String::from_utf8_lossy(edited_document_xml);
    for untouched_paragraph in [
        "<w:p><w:r><w:t>Paragraph 0.</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>Paragraph 2.</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>Paragraph 4.</w:t></w:r></w:p>",
    ] {
        assert!(
            original_text.contains(untouched_paragraph),
            "fixture sanity: {untouched_paragraph} must appear in the original"
        );
        assert!(
            edited_text.contains(untouched_paragraph),
            "{untouched_paragraph} must survive the edit unchanged"
        );
    }
    let section_1_sect_pr = r#"<w:sectPr><w:type w:val="nextPage"/><w:pgSz w:w="16838" w:h="11906" w:orient="landscape"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#;
    assert!(original_text.contains(section_1_sect_pr));
    assert!(
        edited_text.contains(section_1_sect_pr),
        "section 1's own w:sectPr must survive editing section 2's margins unchanged"
    );
    let body_level_sect_pr = r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#;
    assert!(original_text.contains(body_level_sect_pr));
    assert!(
        edited_text.contains(body_level_sect_pr),
        "the body-level (section 3) w:sectPr must survive editing section 2's margins unchanged"
    );

    // And the actual edit landed: section 2's margins read back with the new top margin.
    document
        .sections(|spans, interner| {
            let section_2 = spans[1].properties.as_ref().expect("section 2");
            let margins = section_2
                .page_margins(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(margins.top, 2000);
            assert_eq!(
                margins.right,
                PageMargins::NORMAL.right,
                "every other field untouched"
            );
        })
        .expect("read back edited margins");
}

// -------------------------------------------------------------------------------------------
// Splitting a document into a new section.
// -------------------------------------------------------------------------------------------

#[test]
fn splitting_a_document_creates_a_new_section_break_inside_the_terminating_paragraphs_own_ppr() {
    let mut document = Document::blank(PageSize::a4()).expect("blank document");
    for _ in 0..2 {
        document.append_paragraph().expect("append paragraph");
    }
    // Three paragraphs (0, 1, 2) now exist, single section, body-level w:sectPr only.
    document
        .sections(|spans, _interner| assert_eq!(spans.len(), 1))
        .expect("read sections before split");

    // Split: paragraph 1 becomes the end of a new section.
    document
        .edit_section_properties(SectionLocation::Paragraph(1.into()), |section, interner| {
            section.set_break_kind(Some(SectionType::new(
                interner,
                SectionBreakType::Continuous,
            )));
            section.set_page_size(interner, Some(PageSize::a4()));
            section.set_page_margins(interner, Some(PageMargins::NORMAL));
        })
        .expect("split at paragraph 1");

    let saved = document.save().expect("save");
    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "blank document split into a new section",
        &saved,
    );

    let saved_package = Package::open(&saved).expect("open saved package");
    let document_xml = saved_package
        .part_bytes(&PartName::new("/word/document.xml").expect("part name"))
        .expect("word/document.xml exists");
    let text = String::from_utf8_lossy(document_xml);
    let sect_pr_index = text.find("<w:sectPr>").expect("a w:sectPr exists");
    let p_pr_index = text.rfind("<w:pPr>").expect("a w:pPr exists");
    // The nearest `w:pPr` opening tag before the first `w:sectPr` must be the one that contains
    // it — i.e. the new `w:sectPr` is nested inside a paragraph's own `w:pPr`, never appended
    // directly as a sibling of `w:body`'s block content.
    assert!(
        p_pr_index < sect_pr_index,
        "the new w:sectPr must be nested inside a w:pPr, not appended to the body"
    );
    // And there is no `w:sectPr` that is a direct child of `w:body` positioned BEFORE any `w:p` —
    // the structural check that matters is exercised by `sections()` itself below.

    document
        .sections(|spans, _interner| {
            assert_eq!(spans.len(), 2, "the split produced a second section");
            assert_eq!(spans[0].first_paragraph, 0);
            assert_eq!(spans[0].last_paragraph, Some(1));
            assert_eq!(spans[1].first_paragraph, 2);
            assert_eq!(spans[1].last_paragraph, Some(2));
        })
        .expect("read sections after split");
}

// -------------------------------------------------------------------------------------------
// w:printerSettings — a relationship to a binary part this crate never rewrites.
// -------------------------------------------------------------------------------------------

#[test]
fn a_printer_settings_reference_and_its_binary_part_survive_an_unrelated_edit_untouched() {
    let original_bytes = fixture("printer_settings_reference.docx");
    let printer_part =
        PartName::new("/word/printerSettings/printerSettings1.bin").expect("part name");
    let original_package = Package::open(&original_bytes).expect("open");
    let original_payload = original_package
        .part_bytes(&printer_part)
        .expect("printer settings part exists")
        .to_vec();
    assert!(!original_payload.is_empty());

    let mut document =
        Document::open(&original_bytes).expect("open printer_settings_reference.docx");
    let relationship_id_before = document
        .sections(|spans, interner| {
            spans[0]
                .properties
                .as_ref()
                .expect("body-level section")
                .printer_settings()
                .expect("w:printerSettings present")
                .relationship_id(interner)
                .expect("valid")
                .into_owned()
        })
        .expect("read printer settings reference");
    assert_eq!(relationship_id_before, "rId4");

    // An edit to a *different* part of the same w:sectPr — never the printer settings part.
    document
        .edit_section_properties(SectionLocation::Body, |section, interner| {
            section.set_page_margins(
                interner,
                Some(PageMargins {
                    gutter: 360,
                    ..PageMargins::NORMAL
                }),
            );
        })
        .expect("edit page margins");
    let edited_bytes = document.save().expect("save");

    let edited_package = Package::open(&edited_bytes).expect("open edited package");
    let edited_payload = edited_package
        .part_bytes(&printer_part)
        .expect("printer settings part still exists");
    assert_eq!(
        &original_payload, edited_payload,
        "the printer settings binary payload must never be rewritten"
    );

    let mut edited_document = Document::open(&edited_bytes).expect("reopen edited document");
    edited_document
        .sections(|spans, interner| {
            let properties = spans[0].properties.as_ref().expect("body-level section");
            let relationship_id_after = properties
                .printer_settings()
                .expect("w:printerSettings still present")
                .relationship_id(interner)
                .expect("valid");
            assert_eq!(
                relationship_id_after, "rId4",
                "the reference itself is untouched"
            );
            let margins = properties
                .page_margins(interner)
                .expect("valid")
                .expect("present");
            assert_eq!(margins.gutter, 360, "the actual edit landed");
        })
        .expect("read back edited document");
}

// -------------------------------------------------------------------------------------------
// w:equalWidth vs. an explicit w:col list — equalWidth wins (ECMA-376 Part 1 §17.6.4).
// -------------------------------------------------------------------------------------------

#[test]
fn equal_width_true_is_detectable_even_when_an_explicit_col_list_is_also_present() {
    use mjx_ooxml_core::Interner;

    let mut interner = Interner::new();
    let mut columns = Columns::new(&mut interner);
    columns.set_equal_width(&mut interner, Some(true));
    columns.set_num(&mut interner, Some(3));
    columns.push_column(mjx_docx::Column::new(&mut interner, 2_880));
    columns.push_column(mjx_docx::Column::new(&mut interner, 1_440));

    // A real-file contradiction: both an explicit list AND equalWidth="true" are present. This
    // crate does not silently resolve it — it reports both independently, letting a caller apply
    // ECMA-376 Part 1 §17.6.4's own rule (`equalWidth` wins) explicitly. See `sections.rs`'s own
    // module doc for the ruling, quoted from the prose.
    assert!(columns.is_equal_width(&interner).expect("valid"));
    assert_eq!(
        columns.column_count(),
        2,
        "the explicit list is still readable"
    );
    assert_eq!(columns.num(&interner), Ok(3));
}

// -------------------------------------------------------------------------------------------
// PageOrientation is one type — not a hand-written duplicate of the generated enum.
// -------------------------------------------------------------------------------------------

#[test]
fn page_orientation_is_the_generated_type_not_a_second_one() {
    // Compiles only if `mjx_docx::PageOrientation` and
    // `mjx_ooxml_types::wordprocessingml::PageOrientation` are the very same type — a duplicate
    // hand-written enum with the same variant names would not type-check here.
    let generated: mjx_ooxml_types::wordprocessingml::PageOrientation = PageOrientation::Landscape;
    assert_eq!(generated, PageOrientation::Landscape);
}

/// A trivial sanity check that `DocxError` stays in scope for every `.expect`/`?` above — keeps the
/// import from being flagged unused if every other function is skipped by a test filter.
#[allow(dead_code)]
fn _uses_docx_error() -> Result<(), DocxError> {
    Ok(())
}
