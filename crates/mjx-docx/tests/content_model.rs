//! MJXOFF-92's "Done when": `sample.docx`'s text reads back exactly (both paragraphs), edit
//! isolation is provable by mutation, and a fixture carrying every canonicalization-sensitive
//! run-inner element round-trips exactly with the model materialized.

use mjx_docx::Document;
use mjx_fixtures::fixture;
use mjx_opc::Package;

/// `sample.docx`'s exact text — two paragraphs (the ticket's own prose names only the first; the
/// fixture actually carries a second, *"This is a fixture paragraph."*, and both must read back).
#[test]
fn sample_docx_reads_back_both_paragraphs_and_every_run() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");

    assert_eq!(document.paragraph_count().expect("paragraph count"), 2);

    assert_eq!(
        document.paragraph_text(0).expect("paragraph 0 text"),
        "Hello OOXML from mjx-ooxml-rs."
    );
    assert_eq!(
        document.paragraph_text(1).expect("paragraph 1 text"),
        "This is a fixture paragraph."
    );

    assert_eq!(document.run_count(0).expect("run count 0"), 1);
    assert_eq!(document.run_count(1).expect("run count 1"), 1);

    assert_eq!(
        document.run_text(0, 0).expect("run 0 text"),
        "Hello OOXML from mjx-ooxml-rs."
    );
    assert_eq!(
        document.run_text(1, 0).expect("run 1 text"),
        "This is a fixture paragraph."
    );
}

/// The non-discriminating-test trap, closed: `run_content.docx`'s first paragraph interleaves runs
/// with non-run content (`w:br`, `w:tab`, `w:sym`, …) and wraps two runs in a `w:hyperlink`. A reader
/// that only walked `w:r` children of the paragraph directly (missing the hyperlink's own two runs),
/// or that walked children in the wrong order, produces a *different* string than this assertion —
/// so this test is discriminating on both counts at once. Would fail if hyperlink descent, or
/// document-order walking, were not implemented.
#[test]
fn runs_out_of_a_naive_order_and_inside_a_hyperlink_are_still_reachable() {
    let mut document = Document::open(&fixture("run_content.docx")).expect("open run_content.docx");

    assert_eq!(
        document.paragraph_text(0).expect("paragraph 0 text"),
        "  leading space, after a break, a link, with two runs, tabbed, "
    );

    // Twelve top-level run-or-hyperlink slots: three plain runs, the hyperlink (one slot, its own
    // two runs are not counted here — mirrors `Presentation::shape_count`'s "a group counts as one
    // shape"), then eight more plain runs (tab/tabbed/sym/cr/noBreakHyphen/ptab/ruby/fldChar).
    assert_eq!(document.run_count(0).expect("run count"), 12);

    // The hyperlink is slot 3 (0-indexed: leading-space run, br run, "after a break" run, then the
    // hyperlink); its own two runs are reached with a depth-2 RunPath.
    assert_eq!(
        document
            .run_text(0, [3, 0])
            .expect("first run inside the hyperlink"),
        "a link, "
    );
    assert_eq!(
        document
            .run_text(0, [3, 1])
            .expect("second run inside the hyperlink"),
        "with two runs, "
    );
    // A bare top-level index into the hyperlink's own slot does not resolve to a run: the address
    // must be followed all the way to an actual `w:r`.
    assert!(
        document.run_text(0, 3).is_err(),
        "slot 3 is a hyperlink, not a run"
    );

    assert_eq!(
        document.paragraph_text(1).expect("paragraph 1 text"),
        "Second paragraph, plain."
    );
}

/// `Text::set_text`'s `xml:space` rule, both directions, proved through the public API rather than
/// against the type directly (so the test also proves `Document::set_run_text` reaches it).
#[test]
fn set_run_text_writes_xml_space_preserve_only_when_the_new_text_needs_it() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");

    // Direction 1: text with significant leading/trailing whitespace gets `xml:space="preserve"` —
    // without it, re-opening the saved bytes would return whitespace-collapsed text and this
    // assertion would catch it (the read side never trims, but a consumer that does not preserve
    // literal source bytes could not roundtrip through Word without the attribute).
    document
        .set_run_text(0, 0, "  padded  ")
        .expect("set run 0 text to whitespace-bearing text");
    let saved = document.save_unchecked().expect("save");
    let bytes = extract_document_xml(&saved);
    assert!(
        bytes.contains("<w:t xml:space=\"preserve\">  padded  </w:t>"),
        "expected xml:space=\"preserve\" on whitespace-bearing text:\n{bytes}"
    );
    let mut reopened = Document::open(&saved).expect("reopen");
    assert_eq!(
        reopened.run_text(0, 0).expect("re-read run text"),
        "  padded  "
    );

    // Direction 2: setting text that no longer needs it removes the attribute — proving this is not
    // "always write preserve", which would churn markup a caller never asked to touch.
    reopened
        .set_run_text(0, 0, "no edges")
        .expect("set run 0 text to text with no significant whitespace");
    let saved_again = reopened.save_unchecked().expect("save again");
    let bytes_again = extract_document_xml(&saved_again);
    assert!(
        bytes_again.contains("<w:t>no edges</w:t>"),
        "expected a bare w:t with no xml:space:\n{bytes_again}"
    );
    assert!(
        !bytes_again.contains("xml:space"),
        "xml:space must be gone once the text no longer needs it:\n{bytes_again}"
    );
}

/// Edit isolation, proved by mutation: editing run 0 of paragraph 0 changes `word/document.xml` and
/// nothing else — every other part is byte-identical — and **within** `word/document.xml`, paragraph
/// 1's own bytes (*"This is a fixture paragraph."*, the paragraph the edit never touches) are
/// unchanged at the byte level.
///
/// **This assertion alone is not discriminating** — confirmed by hand while developing it:
/// `sample.docx` has no incidental whitespace or unusual attribute formatting, so bypassing
/// `ToXml::write_back`'s span-preserving restore entirely (a full reflow from the model) *still*
/// reproduces byte-identical output for this specific fixture. The property this test's name claims
/// is real, but the mechanism that guarantees it — a untouched element keeps its retained
/// [`mjx_ooxml_core::RawElement::source_span`], so it is *copied*, not *reconstructed* — is proved
/// directly (and found to go red when neutralized) by
/// `document::tests::editing_one_run_retains_the_untouched_sibling_paragraphs_source_span` in
/// `crates/mjx-docx/src/document/mod.rs`, which has same-crate access to the live part tree this
/// black-box test cannot reach. Keeping both: this one is what a caller of the crate actually gets;
/// that one is what proves *why*.
#[test]
fn editing_one_run_leaves_every_other_part_and_the_sibling_paragraph_byte_identical() {
    let original = fixture("sample.docx");
    let original_pkg = Package::open(&original).expect("open original");

    let mut document = Document::open(&original).expect("open sample.docx");
    document
        .set_run_text(0, 0, "Edited text.")
        .expect("edit paragraph 0's run");
    let edited = document.save_unchecked().expect("save");
    let edited_pkg = Package::open(&edited).expect("open edited");

    let sibling_paragraph =
        "<w:p><w:pPr><w:pStyle w:val=\"PreformattedText\"/><w:bidi w:val=\"0\"/>\
<w:spacing w:before=\"0\" w:after=\"0\"/><w:jc w:val=\"left\"/><w:rPr></w:rPr></w:pPr>\
<w:r><w:rPr></w:rPr><w:t>This is a fixture paragraph.</w:t></w:r></w:p>";
    let original_document_xml = extract_document_xml(&original);
    assert!(
        original_document_xml.contains(sibling_paragraph),
        "the literal substring this test relies on is not in the original fixture:\n{original_document_xml}"
    );

    for (before, after) in original_pkg.entries().iter().zip(edited_pkg.entries()) {
        assert_eq!(before.name, after.name, "part order changed");
        if before.name == "word/document.xml" {
            // The one part the edit is allowed to change.
            assert_ne!(
                before.bytes(),
                after.bytes(),
                "editing a run's text must actually dirty word/document.xml"
            );
            let edited_document_xml = extract_document_xml(&edited);
            assert!(
                edited_document_xml.contains(sibling_paragraph),
                "the untouched sibling paragraph's bytes must survive verbatim inside the edited \
                 word/document.xml:\n{edited_document_xml}"
            );
            assert!(
                edited_document_xml.contains("Edited text."),
                "the edited run's new text must be present:\n{edited_document_xml}"
            );
        } else {
            assert_eq!(
                before.bytes(),
                after.bytes(),
                "editing one run must not change {}",
                before.name
            );
        }
    }
}

/// Round-trip canonicalization equality for the fixture carrying `w:br`, `w:tab`, `w:sym`, `w:cr`,
/// `w:noBreakHyphen`, `w:ptab`, `w:ruby`, a `w:t` with `xml:space="preserve"`, and `w:fldChar` (a
/// run-inner element whose payload is still `Unmodeled`/raw) — with the model actually forced to
/// materialize, not a bare open/save that would pass on `mjx-opc`'s copy-on-write alone.
#[test]
fn run_content_docx_round_trips_with_the_model_materialized() {
    let original = fixture("run_content.docx");
    let mut document = Document::open(&original).expect("open run_content.docx");

    // Read every paragraph and run — parses the whole tree through the typed model — then force a
    // write with no logical change, exactly as `roundtrip.rs` does for `sample.docx`.
    assert_eq!(document.paragraph_count().expect("paragraph count"), 2);
    for paragraph in 0..document.paragraph_count().expect("paragraph count") {
        let _ = document.paragraph_text(paragraph).expect("paragraph text");
    }
    let conformance = document.conformance().expect("read @conformance");
    document
        .set_conformance(conformance)
        .expect("force word/document.xml through the typed model");

    let saved = document.save().expect("save");

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

    mjx_schema_gate::assert_deck_is_in_schema_order("saved run_content.docx", &saved);
    mjx_schema_gate::assert_authored_deck_is_schema_valid("saved run_content.docx", &saved);
}

/// Insert and remove, for both paragraphs and runs — the last of the ticket's six reading/editing
/// operations, and the only one the tests above do not already exercise.
#[test]
fn paragraphs_and_runs_can_be_inserted_and_removed() {
    let mut document = Document::open(&fixture("sample.docx")).expect("open sample.docx");
    assert_eq!(document.paragraph_count().expect("count"), 2);

    // Insert a run at the front of paragraph 0, shifting the existing run to slot 1.
    document
        .insert_run(0, 0, "Before. ")
        .expect("insert a run at slot 0");
    assert_eq!(document.run_count(0).expect("run count"), 2);
    assert_eq!(document.run_text(0, 0).expect("new run"), "Before. ");
    assert_eq!(
        document.run_text(0, 1).expect("the original run, shifted"),
        "Hello OOXML from mjx-ooxml-rs."
    );
    assert_eq!(
        document.paragraph_text(0).expect("paragraph 0 text"),
        "Before. Hello OOXML from mjx-ooxml-rs."
    );

    // Remove it again — paragraph 0 is back to exactly what it started as.
    document.remove_run(0, 0).expect("remove the inserted run");
    assert_eq!(document.run_count(0).expect("run count"), 1);
    assert_eq!(
        document.paragraph_text(0).expect("paragraph 0 text"),
        "Hello OOXML from mjx-ooxml-rs."
    );

    // Append a new paragraph, give it a run, then insert a second one *before* it — both by
    // position and by content.
    document.append_paragraph().expect("append a paragraph");
    assert_eq!(document.paragraph_count().expect("count"), 3);
    document
        .append_run(2, "Third paragraph.")
        .expect("append a run to the new paragraph");
    assert_eq!(
        document.paragraph_text(2).expect("paragraph 2 text"),
        "Third paragraph."
    );

    document
        .insert_paragraph(2)
        .expect("insert a paragraph before the new one");
    assert_eq!(document.paragraph_count().expect("count"), 4);
    assert_eq!(document.paragraph_text(2).expect("paragraph 2 text"), "");
    assert_eq!(
        document.paragraph_text(3).expect("paragraph 3 text"),
        "Third paragraph.",
        "inserting at 2 must shift the paragraph that used to be there to 3"
    );

    // Removing the two new paragraphs restores the original document exactly.
    document.remove_paragraph(3).expect("remove paragraph 3");
    document.remove_paragraph(2).expect("remove paragraph 2");
    assert_eq!(document.paragraph_count().expect("count"), 2);
    assert_eq!(
        document.paragraph_text(0).expect("paragraph 0 text"),
        "Hello OOXML from mjx-ooxml-rs."
    );
    assert_eq!(
        document.paragraph_text(1).expect("paragraph 1 text"),
        "This is a fixture paragraph."
    );

    // Out-of-range addresses are rejected, not silently clamped.
    assert!(document.remove_paragraph(5).is_err());
    assert!(document.insert_paragraph(10).is_err());
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
