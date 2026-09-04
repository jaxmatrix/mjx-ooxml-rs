//! Comments, footnotes, endnotes and bookmarks (MJXOFF-124).
//!
//! **No committed Word fixture carries a comment, footnote, endnote or bookmark** — this ticket's own
//! pre-dispatch note confirms it, and the overlapping-range case in particular could not come from one
//! anyway: a writer that only ever emits well-nested ranges cannot produce it. So every fixture below
//! is hand-spliced raw markup, exactly `tests/fields.rs`'s own `fields_and_hyperlinks.docx` splices
//! `w:fldChar`/`w:instrText` markup its own writer cannot produce into a blank document's
//! `word/document.xml` — see that file's own module doc for the technique. Nothing here is persisted
//! as a new binary fixture: each test builds its own in-memory document, since the content each one
//! needs differs enough that a shared corpus entry would mostly be dead weight for every other test.

use mjx_docx::{BookmarkResolution, Document, HyperlinkTarget, PageSize, RangeResolution};
use mjx_opc::{Package, PartName};

/// Splices `raw_body_xml` immediately after `<w:body>` in a fresh blank A4 document's own
/// `word/document.xml`, and reopens it — [`tests/fields.rs`]'s own `build_fixture`'s splicing step,
/// extracted here since every test in this file needs its own splice of different markup.
fn spliced_document(raw_body_xml: &str) -> Document {
    let blank = Document::blank(PageSize::a4()).expect("blank a4 document");
    let bytes = blank.save().expect("save the blank document");

    let document_part =
        PartName::new("/word/document.xml").expect("word/document.xml is a valid part name");
    let mut package = Package::open(&bytes).expect("reopen the intermediate package");
    let original = package
        .part_bytes(&document_part)
        .expect("word/document.xml exists")
        .to_vec();
    let original = String::from_utf8(original).expect("this crate's own writer emits UTF-8");
    let spliced = original.replacen("<w:body>", &format!("<w:body>{raw_body_xml}"), 1);
    package
        .replace_part_bytes(&document_part, spliced.into_bytes())
        .expect("splice in the raw paragraphs");
    let spliced_bytes = package.save().expect("serialize the spliced package");

    Document::open(&spliced_bytes).expect("reopen the spliced document")
}

// -------------------------------------------------------------------------------------------
// The overlapping-range trap: A starts, B starts, A ends, B ends.
// -------------------------------------------------------------------------------------------

#[test]
fn overlapping_comment_ranges_resolve_independently_of_nesting() {
    // Document order: start(100) start(200) end(100) end(200) — A and B overlap, neither nests
    // inside the other. Pairing by id gives A = "alpha beta ", B = "gamma "; a stack-based reader
    // (closing whichever range opened most recently) would instead close B at end(100) and get both
    // ranges wrong. See `ranges.rs`'s own module doc for the full account, and this test's own
    // sibling verification note in the PR description for the mutation that turns this red.
    let raw = concat!(
        "<w:p>",
        "<w:r><w:t xml:space=\"preserve\">before </w:t></w:r>",
        "<w:commentRangeStart w:id=\"100\"/>",
        "<w:r><w:t xml:space=\"preserve\">alpha </w:t></w:r>",
        "<w:commentRangeStart w:id=\"200\"/>",
        "<w:r><w:t xml:space=\"preserve\">beta </w:t></w:r>",
        "<w:commentRangeEnd w:id=\"100\"/>",
        "<w:r><w:commentReference w:id=\"100\"/></w:r>",
        "<w:r><w:t xml:space=\"preserve\">gamma </w:t></w:r>",
        "<w:commentRangeEnd w:id=\"200\"/>",
        "<w:r><w:commentReference w:id=\"200\"/></w:r>",
        "<w:r><w:t>after</w:t></w:r>",
        "</w:p>",
    );
    let mut document = spliced_document(raw);

    let a = document.comment_range(100).expect("comment_range(100)");
    let b = document.comment_range(200).expect("comment_range(200)");
    assert!(matches!(a, Some(RangeResolution::Resolved { .. })));
    assert!(matches!(b, Some(RangeResolution::Resolved { .. })));

    let a_text = document
        .comment_range_text(100)
        .expect("comment_range_text(100)")
        .expect("comment 100 resolved");
    let b_text = document
        .comment_range_text(200)
        .expect("comment_range_text(200)")
        .expect("comment 200 resolved");
    // A covers "alpha beta " (its own start to its own end); B covers "beta gamma " (its own start
    // to its own end) — the two spans genuinely share "beta ", which is what "overlap" means. A
    // stack-based reader gets *both* wrong: closing at end(100) it would pop B (the most recently
    // opened), pairing B with "alpha beta " and leaving A's own end(100) marker unconsumed.
    assert_eq!(a_text, "alpha beta ", "comment 100 (A) covered text");
    assert_eq!(b_text, "beta gamma ", "comment 200 (B) covered text");
}

// -------------------------------------------------------------------------------------------
// A comment spanning three paragraphs.
// -------------------------------------------------------------------------------------------

#[test]
fn a_comment_spanning_three_paragraphs_resolves_its_full_text() {
    let raw = concat!(
        "<w:p><w:r><w:t xml:space=\"preserve\">P1 lead </w:t></w:r>",
        "<w:commentRangeStart w:id=\"300\"/>",
        "<w:r><w:t>P1 tail</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>P2 full</w:t></w:r></w:p>",
        "<w:p><w:r><w:t>P3 head</w:t></w:r>",
        "<w:commentRangeEnd w:id=\"300\"/>",
        "<w:r><w:commentReference w:id=\"300\"/></w:r>",
        "<w:r><w:t xml:space=\"preserve\"> P3 tail</w:t></w:r></w:p>",
    );
    let mut document = spliced_document(raw);

    let text = document
        .comment_range_text(300)
        .expect("comment_range_text(300)")
        .expect("comment 300 resolved");
    assert_eq!(text, "P1 tail\nP2 full\nP3 head");
}

// -------------------------------------------------------------------------------------------
// A bookmark starting inside a table cell and ending outside the table.
// -------------------------------------------------------------------------------------------

#[test]
fn a_bookmark_crossing_a_table_cell_boundary_resolves() {
    let raw = concat!(
        "<w:p><w:r><w:t xml:space=\"preserve\">before table </w:t></w:r></w:p>",
        "<w:tbl>",
        "<w:tblPr/>",
        "<w:tblGrid><w:gridCol w:w=\"2000\"/></w:tblGrid>",
        "<w:tr><w:tc><w:tcPr/>",
        "<w:p><w:bookmarkStart w:id=\"500\" w:name=\"tableBookmark\"/>",
        "<w:r><w:t>cell text</w:t></w:r></w:p>",
        "</w:tc></w:tr>",
        "</w:tbl>",
        "<w:p><w:r><w:t xml:space=\"preserve\">after table </w:t></w:r>",
        "<w:bookmarkEnd w:id=\"500\"/>",
        "<w:r><w:t>tail</w:t></w:r></w:p>",
    );
    let mut document = spliced_document(raw);

    let resolution = document
        .resolve_bookmark("tableBookmark")
        .expect("resolve_bookmark")
        .expect("tableBookmark exists");
    match resolution {
        BookmarkResolution::Resolved { id, text } => {
            assert_eq!(id, 500);
            assert_eq!(text, "cell text\nafter table ");
        }
        BookmarkResolution::UnmatchedStart { .. } => {
            panic!("tableBookmark has a matching bookmarkEnd")
        }
    }
}

// -------------------------------------------------------------------------------------------
// An unmatched `w:bookmarkStart` — reported, not panicked on.
// -------------------------------------------------------------------------------------------

#[test]
fn an_unmatched_bookmark_start_is_reported_not_panicked() {
    let raw = concat!(
        "<w:p><w:r><w:t xml:space=\"preserve\">orphan </w:t></w:r>",
        "<w:bookmarkStart w:id=\"999\" w:name=\"orphanBookmark\"/>",
        "<w:r><w:t>tail</w:t></w:r></w:p>",
    );
    let mut document = spliced_document(raw);

    let resolution = document
        .resolve_bookmark("orphanBookmark")
        .expect("resolve_bookmark does not error")
        .expect("orphanBookmark exists");
    assert_eq!(resolution, BookmarkResolution::UnmatchedStart { id: 999 });
}

// -------------------------------------------------------------------------------------------
// Reserved separator/continuationSeparator entries.
// -------------------------------------------------------------------------------------------

#[test]
fn reserved_separator_entries_are_excluded_from_user_footnotes_and_present_when_authored() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document
        .add_footnote(0, "a user footnote")
        .expect("add_footnote");

    document
        .footnotes(|footnotes, interner| {
            let raw_count = footnotes.footnotes().count();
            let user_count = footnotes.user_footnotes(interner).count();
            // Two reserved entries plus one user footnote = three raw entries; the reserved two are
            // excluded from the user-visible count. A footnote-count test that includes the reserved
            // entries would report 3 here, not 1 — exactly the trap this ticket's own "Done when"
            // names.
            assert_eq!(raw_count, 3, "reserved separator + continuationSeparator + 1 user footnote");
            assert_eq!(user_count, 1, "only the user footnote is user-visible");

            for footnote in footnotes.footnotes() {
                if !footnote.is_user_visible(interner) {
                    assert!(
                        matches!(
                            footnote.kind(interner),
                            Ok(Some(
                                mjx_ooxml_types::wordprocessingml::FootnoteEndnoteType::Separator
                                    | mjx_ooxml_types::wordprocessingml::FootnoteEndnoteType::ContinuationSeparator
                            ))
                        ),
                        "a non-user-visible entry must be separator or continuationSeparator"
                    );
                }
            }
        })
        .expect("footnotes")
        .expect("footnotes part exists");
}

// -------------------------------------------------------------------------------------------
// Adding / removing a comment.
// -------------------------------------------------------------------------------------------

#[test]
fn adding_a_comment_to_a_document_with_none_produces_a_valid_package() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    assert!(document.comments(|_, _| ()).expect("comments").is_none());

    let id = document
        .add_comment(0, "Jane Doe", Some("JD"), "a comment")
        .expect("add_comment");

    let (author, initials, text) = document
        .comments(|comments, interner| {
            let comment = comments.comment(interner, id).expect("comment exists");
            (
                comment.author(interner).expect("author"),
                comment.initials(interner),
                comment.text(),
            )
        })
        .expect("comments")
        .expect("comments part now exists");
    assert_eq!(author, "Jane Doe");
    assert_eq!(initials.as_deref(), Some("JD"));
    assert_eq!(text, "a comment");

    let resolution = document
        .comment_range(id)
        .expect("comment_range")
        .expect("range resolved");
    assert!(matches!(resolution, RangeResolution::Resolved { .. }));

    document.validate().expect("Package::validate is clean");
    let saved = document.save().expect("save");
    let mut reopened = Document::open(&saved).expect("reopen");
    let reopened_text = reopened
        .comments(|comments, interner| {
            comments
                .comment(interner, id)
                .expect("comment exists")
                .text()
        })
        .expect("comments")
        .expect("comments part exists");
    assert_eq!(reopened_text, "a comment");
}

#[test]
fn removing_the_last_comment_removes_the_part_and_relationship_with_no_orphan() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    let id = document
        .add_comment(0, "Jane Doe", None, "a comment")
        .expect("add_comment");
    assert!(document.parts().comments.is_some());

    document.remove_comment(id).expect("remove_comment");

    assert!(
        document.parts().comments.is_none(),
        "the comments part is gone once its last comment is removed"
    );
    assert_eq!(
        document.paragraph_text(0).expect("paragraph_text"),
        "",
        "the range markers and reference are gone too"
    );
    document
        .validate()
        .expect("Package::validate reports no orphan");
}

// -------------------------------------------------------------------------------------------
// Adding / removing a footnote — the reserved entries survive.
// -------------------------------------------------------------------------------------------

#[test]
fn adding_and_removing_a_footnote_leaves_the_reserved_entries_in_place() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    let id = document
        .add_footnote(0, "a footnote")
        .expect("add_footnote");

    document.remove_footnote(id).expect("remove_footnote");

    document
        .footnotes(|footnotes, interner| {
            assert_eq!(
                footnotes.user_footnotes(interner).count(),
                0,
                "the user footnote is gone"
            );
            assert_eq!(
                footnotes.footnotes().count(),
                2,
                "the two reserved entries are still there — Word repairs a part that lacks them"
            );
        })
        .expect("footnotes")
        .expect("the part itself is never deleted for footnotes");

    document.validate().expect("Package::validate is clean");
}

// -------------------------------------------------------------------------------------------
// MJXOFF-121's `Hyperlink::anchor` seam, closed.
// -------------------------------------------------------------------------------------------

#[test]
fn a_hyperlink_anchor_resolves_through_the_bookmark_index() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    document.add_bookmark(0, "Target").expect("add_bookmark");
    document.append_paragraph().expect("append_paragraph");
    document
        .insert_hyperlink(1, 0, "jump", &HyperlinkTarget::Anchor("Target".to_owned()))
        .expect("insert_hyperlink");

    let target = document
        .hyperlink_target(1, 0)
        .expect("hyperlink_target")
        .expect("the hyperlink at (1, 0) exists");
    let HyperlinkTarget::Anchor(name) = target else {
        panic!("this hyperlink names an anchor, not a relationship");
    };

    let resolution = document
        .resolve_bookmark(&name)
        .expect("resolve_bookmark")
        .expect("Target exists");
    assert!(matches!(resolution, BookmarkResolution::Resolved { .. }));
}

// -------------------------------------------------------------------------------------------
// Regression (found by MJXOFF-139's own walkthrough): a footnote/endnote added through
// `add_footnote`/`add_endnote` on a document with no existing footnotes/endnotes part must
// survive `save` → `Document::open`, reserved entries included. `create_footnotes_part` used to
// populate those reserved entries by writing back a fresh `Footnotes::blank()` — built with
// `attributes: Vec::new()` — over a root that had just been parsed from the literal template
// (`<w:footnotes xmlns:w="...">`), discarding the `xmlns:w` that parse preserved. The saved bytes
// then had every `w:footnote` child under an `xmlns:w`-less `w:footnotes` root: schema-valid
// prefix use with no declaration in scope, so re-parsing it correctly finds no `w:` elements at
// all and every one of `Document::footnotes`'s three entries (two reserved, one user) vanishes.
// `Footnotes::seed_reserved_entries`/`Endnotes::seed_reserved_entries` fix this by mutating the
// already-`FromXml`-parsed value in place, keeping whatever attributes the real root carried.
// -------------------------------------------------------------------------------------------

#[test]
fn a_footnote_added_to_a_document_with_no_footnotes_part_survives_save_and_reopen() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    let id = document
        .add_footnote(0, "a footnote")
        .expect("add_footnote creates word/footnotes.xml");

    let bytes = document.save().expect("save");

    // The saved bytes must actually declare the namespace every `w:footnote` child uses — the
    // exact defect: a raw-byte check, not only a re-parsed one, so a fix that merely made the
    // *reader* more lenient (rather than fixing the *writer*) would not satisfy this.
    let raw_package = mjx_opc::Package::open(&bytes).expect("reopen as a raw package");
    let footnotes_part = mjx_opc::PartName::new("/word/footnotes.xml").expect("part name");
    let raw_bytes = raw_package
        .part_bytes(&footnotes_part)
        .expect("word/footnotes.xml is present");
    let raw_xml = String::from_utf8_lossy(raw_bytes);
    assert!(
        raw_xml.contains("xmlns:w="),
        "word/footnotes.xml's root does not declare xmlns:w: {raw_xml}"
    );

    let mut reopened = Document::open(&bytes).expect("reopen");
    reopened
        .footnotes(|footnotes, interner| {
            assert_eq!(
                footnotes.footnotes().count(),
                3,
                "the two reserved entries plus the one user footnote"
            );
            assert_eq!(
                footnotes.user_footnotes(interner).count(),
                1,
                "only the user footnote is user-visible"
            );
            let note = footnotes
                .footnote(interner, id)
                .expect("the added footnote resolves by its own id");
            assert_eq!(note.text(), "a footnote");
        })
        .expect("footnotes")
        .expect("word/footnotes.xml exists after reopen");
}

#[test]
fn an_endnote_added_to_a_document_with_no_endnotes_part_survives_save_and_reopen() {
    let mut document = Document::blank(PageSize::a4()).expect("blank");
    let id = document
        .add_endnote(0, "an endnote")
        .expect("add_endnote creates word/endnotes.xml");

    let bytes = document.save().expect("save");
    let raw_package = mjx_opc::Package::open(&bytes).expect("reopen as a raw package");
    let endnotes_part = mjx_opc::PartName::new("/word/endnotes.xml").expect("part name");
    let raw_bytes = raw_package
        .part_bytes(&endnotes_part)
        .expect("word/endnotes.xml is present");
    let raw_xml = String::from_utf8_lossy(raw_bytes);
    assert!(
        raw_xml.contains("xmlns:w="),
        "word/endnotes.xml's root does not declare xmlns:w: {raw_xml}"
    );

    let mut reopened = Document::open(&bytes).expect("reopen");
    reopened
        .endnotes(|endnotes, interner| {
            assert_eq!(endnotes.endnotes().count(), 3);
            assert_eq!(endnotes.user_endnotes(interner).count(), 1);
            let note = endnotes
                .endnote(interner, id)
                .expect("the added endnote resolves by its own id");
            assert_eq!(note.text(), "an endnote");
        })
        .expect("endnotes")
        .expect("word/endnotes.xml exists after reopen");
}
