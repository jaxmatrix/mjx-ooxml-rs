//! Fields, hyperlinks and form fields (MJXOFF-121).
//!
//! `fields_and_hyperlinks.docx` is authored, not templated — hand-written `w:fldChar`/`w:instrText`
//! markup spliced into a blank document's own `word/document.xml` for the three field shapes this
//! crate's own writer cannot produce (a nested `TOC`/`PAGEREF` result, an instruction split across
//! three runs, a field with no `separate`), then the hyperlink and the three form fields are added
//! through this crate's own public API (`Document::insert_hyperlink`/`insert_form_field`/
//! `edit_form_field`) — proving those write paths, not just the reader. See `regenerate_fixtures`
//! (`#[ignore]`) below for exactly how, and `crates/mjx-docx/tests/tables.rs`'s own module doc for
//! why this is the established technique in this crate (`ragged_table.docx`, `header_watermark.docx`
//! before it).
//!
//! Paragraph layout:
//! 0. The nested `TOC` field: cached result `"12"` from two nested `PAGEREF` fields.
//! 1. A `HYPERLINK` field whose instruction is split across three `w:instrText` runs.
//! 2. A `DATE` field with no `w:fldChar separate` — legal, no cached result.
//! 3. A hyperlink to an external URL (`Document::insert_hyperlink`).
//! 4. A hyperlink to an in-document bookmark anchor (no relationship).
//! 5. A checkbox form field, checked, with a name and help text.
//! 6. A drop-down-list form field with three entries and a selection.
//! 7. A text-input form field with a default and a maximum length.
//!
//! The deliberately unbalanced marker sequence this ticket's own trap also names is **not** part of
//! this fixture — ECMA-376 imposes no ordering/balance constraint on `w:fldChar`, so this crate's
//! own writer can never produce one; `crates/mjx-docx/src/document/fields.rs`'s own unit tests
//! construct it directly against the typed model instead (see that module's `tests` for
//! `an_unbalanced_sequence_returns_a_typed_error` and `an_unmatched_separate_returns_a_typed_error`).

use mjx_docx::{
    Document, DocxError, FieldForm, FormFieldCheckBox, FormFieldDropDownList, FormFieldTextInput,
    HyperlinkTarget, PageSize,
};
use mjx_fixtures::fixture;
use mjx_ooxml_types::wordprocessingml::{FormFieldTextType, HelpOrStatusTextType};
use mjx_opc::{Package, PartName};

fn open_fixture() -> Document {
    Document::open(&fixture("fields_and_hyperlinks.docx")).expect("open fields_and_hyperlinks.docx")
}

// -------------------------------------------------------------------------------------------
// Nested fields, split instructions, no-separate fields — the read model.
// -------------------------------------------------------------------------------------------

#[test]
fn a_nested_toc_fields_own_instruction_and_cached_result_are_read_separately() {
    let mut document = open_fixture();
    let fields = document.fields(0).expect("fields in paragraph 0");
    assert_eq!(
        fields.len(),
        1,
        "paragraph 0 holds exactly one top-level field"
    );
    let toc = &fields[0];
    assert_eq!(toc.form(), FieldForm::Complex);
    assert_eq!(toc.field_name(), Some("TOC"));
    assert_eq!(toc.instruction(), " TOC \\o \"1-3\" ");
    assert_eq!(
        toc.cached_result(),
        Some("12"),
        "the outer field's own cached result is its two nested PAGEREFs' own results, concatenated"
    );
    assert_eq!(toc.nested_fields().len(), 2);
    assert_eq!(toc.nested_fields()[0].instruction(), " PAGEREF _Toc1 ");
    assert_eq!(toc.nested_fields()[0].cached_result(), Some("1"));
    assert_eq!(toc.nested_fields()[1].instruction(), " PAGEREF _Toc2 ");
    assert_eq!(toc.nested_fields()[1].cached_result(), Some("2"));
}

// A marker-pairer that counts `begin`/`end` instead of nesting them, and one that concatenates
// every `w:instrText` in the paragraph — this ticket's own named trap — are proved wrong directly
// against the typed model (constructing the mutation itself, not just re-deriving a value that
// happens to differ) in `crates/mjx-docx/src/document/fields.rs`'s own
// `a_counting_pairer_would_report_the_wrong_outer_instruction` unit test. The fixture-level
// assertion above (`toc.instruction()` excludes both nested instructions, `toc.cached_result()`
// includes both nested results) is the same discriminating property, proved through the full
// `Document::open` → OPC → typed-model path rather than an in-memory `Paragraph`.

#[test]
fn a_field_whose_instruction_splits_across_three_runs_reads_as_one_instruction() {
    let mut document = open_fixture();
    let fields = document.fields(1).expect("fields in paragraph 1");
    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].instruction(),
        " HYPERLINK \"http://example.com\" "
    );
    assert_eq!(fields[0].field_name(), Some("HYPERLINK"));
    assert_eq!(fields[0].cached_result(), Some("example.com"));
}

#[test]
fn a_field_with_no_separate_reads_correctly_and_is_not_an_error() {
    let mut document = open_fixture();
    let fields = document.fields(2).expect("fields in paragraph 2");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].instruction(), " DATE ");
    assert_eq!(fields[0].cached_result(), None);
}

// -------------------------------------------------------------------------------------------
// Editing an instruction or a cached result leaves the other, and every other part, untouched.
// -------------------------------------------------------------------------------------------

#[test]
fn editing_a_fields_instruction_leaves_its_cached_result_and_every_other_part_byte_identical() {
    let original_bytes = fixture("fields_and_hyperlinks.docx");
    let original_package = Package::open(&original_bytes).expect("open original package");

    let mut document = open_fixture();
    document
        .set_field_instruction(1, 0, " HYPERLINK \"http://example.org\" ")
        .expect("edit the HYPERLINK field's instruction");
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

    let mut edited_document = Document::from_package(edited_package).expect("reopen edited");
    let fields = edited_document.fields(1).expect("fields in paragraph 1");
    assert_eq!(
        fields[0].instruction(),
        " HYPERLINK \"http://example.org\" "
    );
    assert_eq!(
        fields[0].cached_result(),
        Some("example.com"),
        "the cached result must survive an instruction-only edit untouched"
    );
}

#[test]
fn editing_a_fields_cached_result_leaves_its_instruction_byte_identical() {
    let mut document = open_fixture();
    document
        .set_field_cached_result_text(1, 0, "example.org")
        .expect("edit the HYPERLINK field's cached result");
    let fields = document.fields(1).expect("fields in paragraph 1");
    assert_eq!(
        fields[0].instruction(),
        " HYPERLINK \"http://example.com\" ",
        "the instruction must survive a cached-result-only edit untouched"
    );
    assert_eq!(fields[0].cached_result(), Some("example.org"));
}

// -------------------------------------------------------------------------------------------
// Hyperlinks: read, add, remove — the relationship follows the element.
// -------------------------------------------------------------------------------------------

#[test]
fn the_fixtures_own_external_hyperlink_resolves_through_its_relationship() {
    let mut document = open_fixture();
    let target = document
        .hyperlink_target(3, 0)
        .expect("hyperlink_target")
        .expect("paragraph 3 holds a hyperlink");
    assert_eq!(
        target,
        HyperlinkTarget::Url("http://example.com/target".to_owned())
    );
}

#[test]
fn the_fixtures_own_anchor_hyperlink_resolves_to_its_raw_bookmark_name() {
    let mut document = open_fixture();
    let target = document
        .hyperlink_target(4, 0)
        .expect("hyperlink_target")
        .expect("paragraph 4 holds a hyperlink");
    assert_eq!(target, HyperlinkTarget::Anchor("chapter3".to_owned()));
}

#[test]
fn adding_a_hyperlink_creates_a_valid_external_relationship_and_package_validate_is_clean() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    document
        .insert_hyperlink(
            0,
            0,
            "Example",
            &HyperlinkTarget::Url("http://example.com/".to_owned()),
        )
        .expect("insert hyperlink");
    document.validate().expect("Package::validate is clean");
    let target = document
        .hyperlink_target(0, 0)
        .expect("hyperlink_target")
        .expect("the hyperlink just inserted");
    assert_eq!(
        target,
        HyperlinkTarget::Url("http://example.com/".to_owned())
    );
}

#[test]
fn removing_a_hyperlink_removes_its_relationship_and_package_validate_is_clean() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    document
        .insert_hyperlink(
            0,
            0,
            "Example",
            &HyperlinkTarget::Url("http://example.com/".to_owned()),
        )
        .expect("insert hyperlink");
    document.remove_hyperlink(0, 0).expect("remove hyperlink");
    document.validate().expect("Package::validate is clean");
    let target = document.hyperlink_target(0, 0).expect("hyperlink_target");
    assert_eq!(target, None, "the hyperlink slot is gone");
    let bytes = document.save().expect("save");
    let package = Package::open(&bytes).expect("reopen saved package");
    let document_part = PartName::new("/word/document.xml").expect("part name");
    let rels = package.relationships_for(Some(&document_part));
    let still_has_hyperlink_rel = rels
        .map(|rels| rels.iter().any(|rel| rel.rel_type.ends_with("/hyperlink")))
        .unwrap_or(false);
    assert!(
        !still_has_hyperlink_rel,
        "removing the only hyperlink referencing it must remove the relationship too"
    );
}

// -------------------------------------------------------------------------------------------
// Form fields: all three kinds round-trip with names, help text and their own options.
// -------------------------------------------------------------------------------------------

#[test]
fn the_checkbox_form_field_round_trips_its_name_help_text_and_checked_state() {
    let mut document = open_fixture();
    document
        .form_field(5, 0, |data, interner| {
            let data = data.expect("paragraph 5's begin marker carries ffData");
            assert_eq!(data.name(interner), Some("Approved".to_owned()));
            let help_text = data.help_text().expect("help text");
            assert_eq!(help_text.kind(interner), Some(HelpOrStatusTextType::Text));
            assert_eq!(
                help_text.text(interner),
                Some("Check if approved".to_owned())
            );
            let checkbox = data.check_box().expect("checkbox definition");
            assert_eq!(checkbox.checked(interner), Some(true));
        })
        .expect("read the checkbox form field");
}

#[test]
fn the_drop_down_form_field_round_trips_its_entries_and_selection() {
    let mut document = open_fixture();
    document
        .form_field(6, 0, |data, interner| {
            let data = data.expect("paragraph 6's begin marker carries ffData");
            assert_eq!(data.name(interner), Some("Region".to_owned()));
            let list = data.drop_down_list().expect("drop-down list definition");
            assert_eq!(
                list.entries(interner).collect::<Vec<_>>(),
                vec!["North".to_owned(), "South".to_owned(), "East".to_owned()]
            );
            assert_eq!(list.selected_index(interner), Some(2));
        })
        .expect("read the drop-down form field");
}

#[test]
fn the_text_input_form_field_round_trips_its_kind_default_and_max_length() {
    let mut document = open_fixture();
    document
        .form_field(7, 0, |data, interner| {
            let data = data.expect("paragraph 7's begin marker carries ffData");
            assert_eq!(data.name(interner), Some("Comment".to_owned()));
            let input = data.text_input().expect("text-input definition");
            assert_eq!(input.kind(interner), Some(FormFieldTextType::Regular));
            assert_eq!(input.default_text(interner), Some("type here".to_owned()));
            assert_eq!(input.max_length(interner), Some(60));
        })
        .expect("read the text-input form field");
}

#[test]
fn editing_a_form_fields_data_leaves_every_other_part_byte_identical() {
    let original_bytes = fixture("fields_and_hyperlinks.docx");
    let original_package = Package::open(&original_bytes).expect("open original package");

    let mut document = open_fixture();
    document
        .edit_form_field(5, 0, |data, interner| {
            data.set_name(interner, "ApprovedByManager").unwrap();
        })
        .expect("edit the checkbox form field's name");
    let edited_bytes = document.save().expect("save");
    let edited_package = Package::open(&edited_bytes).expect("open edited package");

    let document_part = PartName::new("/word/document.xml").expect("part name");
    for part in original_package.part_names() {
        if part == document_part {
            continue;
        }
        assert_eq!(
            original_package.part_bytes(&part),
            edited_package.part_bytes(&part),
            "part {part:?} must be byte-identical — only word/document.xml was edited"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Fixture generation — not run by `cargo test`; the record of how `fields_and_hyperlinks.docx`
// was built. Mirrors `crates/mjx-docx/tests/tables.rs`'s own `regenerate_fixtures`.
// -------------------------------------------------------------------------------------------

#[test]
#[ignore = "one-shot generator for the committed fixture; run manually with --ignored"]
fn regenerate_fixtures() {
    std::fs::write(
        mjx_fixtures::fixtures_dir().join("fields_and_hyperlinks.docx"),
        build_fixture(),
    )
    .expect("write fields_and_hyperlinks.docx");
}

fn build_fixture() -> Vec<u8> {
    let document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let bytes = document.save().expect("intermediate save");

    // The three field shapes this crate's own writer cannot produce (a nested cached result, a
    // split instruction, a no-separate field) are spliced in as raw markup, exactly as
    // `tables.rs`'s `ragged_table.docx` splices in its own `w:tbl` — see this file's own module
    // doc comment for why.
    let document_part = mjx_opc::PartName::new("/word/document.xml")
        .expect("word/document.xml is a valid part name");
    let mut package = Package::open(&bytes).expect("reopen the intermediate package");
    let original = package
        .part_bytes(&document_part)
        .expect("word/document.xml exists")
        .to_vec();
    let original =
        String::from_utf8(original).expect("this crate's own writer only ever emits UTF-8");
    let spliced = original.replacen("<w:body>", &format!("<w:body>{}", raw_fields_xml()), 1);
    package
        .replace_part_bytes(&document_part, spliced.into_bytes())
        .expect("splice in the raw field paragraphs");
    let spliced_bytes = package.save().expect("serialize the spliced package");

    // Everything else — the hyperlink and the three form fields — goes through this crate's own
    // public API, proving the write paths (not just the reader) as it builds the fixture.
    let mut document = Document::open(&spliced_bytes).expect("reopen the spliced document");

    document.append_paragraph().expect("paragraph 3");
    document
        .insert_hyperlink(
            3,
            0,
            "Example",
            &HyperlinkTarget::Url("http://example.com/target".to_owned()),
        )
        .expect("insert the external hyperlink");

    document.append_paragraph().expect("paragraph 4");
    document
        .insert_hyperlink(
            4,
            0,
            "Go to Chapter Three",
            &HyperlinkTarget::Anchor("chapter3".to_owned()),
        )
        .expect("insert the anchor hyperlink");

    document.append_paragraph().expect("paragraph 5");
    document
        .insert_form_field(5, 0, " FORMCHECKBOX ", "")
        .expect("insert the checkbox form field skeleton");
    document
        .edit_form_field(5, 0, |data, interner| -> Result<(), DocxError> {
            data.set_name(interner, "Approved")?;
            data.set_enabled(interner, Some(true));
            data.set_help_text(
                interner,
                Some(HelpOrStatusTextType::Text),
                "Check if approved",
            )?;
            let mut checkbox = FormFieldCheckBox::new(interner);
            checkbox.set_checked(interner, Some(true));
            checkbox.set_default_checked(interner, Some(false));
            data.set_check_box(Some(checkbox));
            Ok(())
        })
        .expect("edit checkbox ffData")
        .expect("no length violation");

    document.append_paragraph().expect("paragraph 6");
    document
        .insert_form_field(6, 0, " FORMDROPDOWN ", "")
        .expect("insert the drop-down form field skeleton");
    document
        .edit_form_field(6, 0, |data, interner| -> Result<(), DocxError> {
            data.set_name(interner, "Region")?;
            data.set_enabled(interner, Some(true));
            let mut list = FormFieldDropDownList::new(interner, &["North", "South", "East"]);
            list.set_selected_index(interner, 2);
            data.set_drop_down_list(Some(list));
            Ok(())
        })
        .expect("edit ddList ffData")
        .expect("no length violation");

    document.append_paragraph().expect("paragraph 7");
    document
        .insert_form_field(7, 0, " FORMTEXT ", "type here")
        .expect("insert the text-input form field skeleton");
    document
        .edit_form_field(7, 0, |data, interner| -> Result<(), DocxError> {
            data.set_name(interner, "Comment")?;
            data.set_enabled(interner, Some(true));
            let mut input = FormFieldTextInput::new(interner, FormFieldTextType::Regular);
            input.set_default_text(interner, "type here");
            input.set_max_length(interner, 60);
            data.set_text_input(Some(input));
            Ok(())
        })
        .expect("edit textInput ffData")
        .expect("no length violation");

    document.save().expect("serialize the fixture package")
}

/// The literal field-marker markup for the three shapes this crate's own writer cannot produce —
/// see this file's own module doc comment for the layout.
fn raw_fields_xml() -> String {
    concat!(
        // Paragraph 0 — nested TOC/PAGEREF.
        "<w:p><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\"> TOC \\o \"1-3\" </w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\"> PAGEREF _Toc1 </w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>",
        "<w:r><w:t>1</w:t></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\"> PAGEREF _Toc2 </w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>",
        "<w:r><w:t>2</w:t></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
        "</w:p>",
        // Paragraph 1 — HYPERLINK, instruction split across three w:instrText runs.
        "<w:p><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\"> HYPER</w:instrText></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\">LINK \"http://example.com\" </w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>",
        "<w:r><w:t>example.com</w:t></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
        "</w:p>",
        // Paragraph 2 — DATE, no separate.
        "<w:p><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>",
        "<w:r><w:instrText xml:space=\"preserve\"> DATE </w:instrText></w:r>",
        "<w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
        "</w:p>",
    )
    .to_owned()
}
