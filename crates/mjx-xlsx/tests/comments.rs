//! MJXOFF-114 (E5) — Excel's legacy surfaces: cell comments, their VML backing, and the
//! `shapeId` hop from a sheet's modern markup to the legacy shape that draws an object.
//!
//! # Where every fixture came from, and why that is the first thing this file says
//!
//! The ticket names the trap in this child's own terms: *"a VML fixture we hand-crafted will parse
//! and re-serialise through our own writer whatever the model gets wrong … every fixture is
//! hand-crafted, so the gate proves our reader agrees with our writer."* **No fixture below was
//! written by this project.** Two producers wrote three files:
//!
//! | fixture | producer | how |
//! |---|---|---|
//! | `cell_comments.xlsx` | **LibreOffice 25.8.7.3** | driven headless over UNO: two sheets, one visible note and one hidden, one comment two lines long, saved through the *Calc MS Excel 2007 XML* filter |
//! | `legacy_form_control.xlsx` | **LibreOffice 25.8.7.3** | the same way, with a `com.sun.star.form.component.CheckBox` on the sheet — which the filter exports as an `x:control` inside an `mc:AlternateContent`, an `xl/ctrlProps/ctrlProps2.xml`, and a second `v:shape` beside the note's |
//! | `comments_third_party.xlsx` | **XlsxWriter 3.2.9** | `write_comment` and `insert_button`, through the library's own writer |
//!
//! Their VML disagrees with anything this project would emit, and disagrees with *each other*:
//!
//! * LibreOffice writes two `v:shape`s with the **same `id`** (`shape_0`), duplicates the
//!   `v:shapetype` declaration once per shape, puts an `o:allowincell` on every shape, and wraps a
//!   `v:shapetype`'s start tag across four lines;
//! * XlsxWriter writes **no XML declaration at all** on its `.vml`, opens with an
//!   `o:shapelayout`/`o:idmap`, and numbers its shapes `_x0000_s1025` the way Excel does;
//! * LibreOffice puts the control's *name* in `@id` and the generated identifier in `@o:spid`,
//!   which is why [`mjx_vml::Drawing::shape_by_numeric_identifier`] looks at both.
//!
//! # The schema gate is not coverage here
//!
//! `.vml` is a pinned skip — `mjx_schema_gate::categories` states why — so a green schema run says
//! nothing about a VML defect. What guards this child is byte identity over producer files and the
//! two-halves invariant, and both are below.

use mjx_ooxml_core::Interner;
use mjx_opc::{Package, PartName};
use mjx_sml::{CellReference, CommentText, Comments};
use mjx_vml::{AttachedObjectKind, Drawing, Shape};
use mjx_xlsx::{SpreadsheetDefect, Workbook, XlsxError};

/// LibreOffice's two-sheet workbook: `Budget` with two comments, `Notes` with one.
fn libreoffice_comments() -> Vec<u8> {
    mjx_fixtures::fixture("cell_comments.xlsx")
}

/// LibreOffice's form-control workbook: a note and an `x:control` sharing one VML part.
fn libreoffice_control() -> Vec<u8> {
    mjx_fixtures::fixture("legacy_form_control.xlsx")
}

/// XlsxWriter's workbook: one comment and one button, in a `.vml` with no XML declaration.
fn xlsxwriter_comments() -> Vec<u8> {
    mjx_fixtures::fixture("comments_third_party.xlsx")
}

fn cell(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn part(name: &str) -> PartName {
    PartName::new(name).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// The decompressed payload of one part of a **saved** package.
///
/// Saved, not live: `Package::part_bytes` answers `None` for an edited body, so a comparison over a
/// live workbook after an edit compares `None` with `None` and passes whatever happened. MJXOFF-111
/// found its own strongest assertion vacuous exactly that way, and this file's two-halves and
/// isolation cases are the same shape of claim.
fn saved_part(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let package = Package::open(bytes).expect("open");
    package.part_bytes(&part(name)).map(<[u8]>::to_vec)
}

/// Every part of a saved package, by name.
fn saved_parts(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(bytes).expect("open");
    let names: Vec<PartName> = package.part_names().collect();
    names
        .iter()
        .filter_map(|name| {
            package
                .part_bytes(name)
                .map(|body| (name.as_str().to_owned(), body.to_vec()))
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// Tier 1: a producer's file comes back unchanged
// -------------------------------------------------------------------------------------------

#[test]
fn every_producer_workbook_re_emits_its_comment_parts_byte_for_byte() {
    for (name, bytes) in [
        ("cell_comments.xlsx", libreoffice_comments()),
        ("legacy_form_control.xlsx", libreoffice_control()),
        ("comments_third_party.xlsx", xlsxwriter_comments()),
    ] {
        let workbook = Workbook::open(&bytes).expect("open");
        let saved = workbook.save().expect("save");
        let before = saved_parts(&bytes);
        let after = saved_parts(&saved);
        assert_eq!(
            before.len(),
            after.len(),
            "{name}: the saved package holds a different number of parts"
        );
        for ((before_name, before_body), (after_name, after_body)) in
            before.iter().zip(after.iter())
        {
            assert_eq!(before_name, after_name, "{name}: part order changed");
            assert_eq!(
                String::from_utf8_lossy(before_body),
                String::from_utf8_lossy(after_body),
                "{name}: {before_name} did not come back verbatim"
            );
        }
    }
}

#[test]
fn a_vml_part_this_project_did_not_write_keeps_its_wrapping_and_its_prologue() {
    // The two producers disagree about the prologue, which is the discriminating half: XlsxWriter
    // writes **no** XML declaration on a `.vml` and LibreOffice writes one. A reader that rebuilt
    // the part around a fresh document would give both the same first line and pass a test that
    // only looked at the shapes.
    let libreoffice = saved_part(&libreoffice_comments(), "/xl/drawings/vmlDrawing1.vml")
        .expect("LibreOffice's VML part");
    assert!(
        libreoffice.starts_with(b"<?xml "),
        "LibreOffice writes an XML declaration on its .vml"
    );
    let third_party = saved_part(&xlsxwriter_comments(), "/xl/drawings/vmlDrawing1.vml")
        .expect("XlsxWriter's VML part");
    assert!(
        third_party.starts_with(b"<xml "),
        "XlsxWriter writes no XML declaration on its .vml:\n{}",
        String::from_utf8_lossy(&third_party[..40.min(third_party.len())])
    );

    // …and LibreOffice wraps a start tag across lines, which a decomposed tree does not record.
    let control = saved_part(&libreoffice_control(), "/xl/drawings/vmlDrawing1.vml")
        .expect("LibreOffice's control VML");
    let control = String::from_utf8_lossy(&control);
    assert!(
        control.contains("<v:shapetype id=\"_x0000_t201\""),
        "the control's shape template is in the fixture"
    );
    assert!(
        control.contains(">\n<v:stroke joinstyle=\"miter\"/>"),
        "the wrapped `v:shapetype` came back wrapped"
    );

    // Re-saving changes none of it.
    let workbook = Workbook::open(&libreoffice_control()).expect("open");
    let saved = workbook.save().expect("save");
    assert_eq!(
        saved_part(&saved, "/xl/drawings/vmlDrawing1.vml").as_deref(),
        saved_part(&libreoffice_control(), "/xl/drawings/vmlDrawing1.vml").as_deref()
    );
}

// -------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------

#[test]
fn a_producer_comment_reads_with_its_author_its_text_and_its_box() {
    let workbook = Workbook::open(&libreoffice_comments()).expect("open");
    let comments = workbook.sheet_comments(0).expect("comments");
    assert_eq!(
        comments.len(),
        2,
        "LibreOffice wrote two comments on Budget"
    );

    let checked = comments
        .iter()
        .find(|comment| comment.cell == cell("A2"))
        .expect("the comment on A2");
    assert_eq!(checked.author.as_deref(), Some("Unknown Author"));
    assert_eq!(checked.author_index, 0);
    // Two lines, in a file that spells the break `&#10;` and wraps the text in a formatted run —
    // so a reader that took the element's own character data would answer the empty string.
    assert_eq!(checked.text, "Checked against the ledger.\nSecond line.");
    assert_eq!(
        checked.shape_id, None,
        "LibreOffice writes no @shapeId, which is why the Row/Column route exists"
    );

    let drawn = checked.comment_box.as_ref().expect("A2's box");
    assert!(drawn.is_visible, "the note on A2 was made visible");
    assert_eq!(drawn.row, Some(1));
    assert_eq!(drawn.column, Some(0));
    // Read, never inferred: the anchor comes back exactly as the producer spelled it, spaces and
    // all, rather than as a rectangle this library computed.
    assert_eq!(
        drawn.anchor_text.as_deref(),
        Some("1, 23, 0, 0, 2, 47, 3, 1")
    );
    assert_eq!(drawn.identifier.as_deref(), Some("shape_0"));

    let hidden = comments
        .iter()
        .find(|comment| comment.cell == cell("B1"))
        .expect("the comment on B1");
    assert!(
        !hidden.comment_box.as_ref().expect("B1's box").is_visible,
        "the second note is hidden"
    );

    // The second sheet has its own comments part and its own VML part.
    let second = workbook.sheet_comments(1).expect("sheet 2 comments");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].text, "A note on the second sheet.");
}

#[test]
fn two_producers_that_disagree_read_to_the_same_shape_of_answer() {
    // XlsxWriter writes what Excel writes: `_x0000_s1025` ids and an `o:shapelayout` header. The
    // reader must not have been tuned to LibreOffice's conventions.
    let workbook = Workbook::open(&xlsxwriter_comments()).expect("open");
    let comment = workbook
        .comment_at(0, cell("B2"))
        .expect("comments")
        .expect("the comment on B2");
    assert_eq!(comment.author.as_deref(), Some("Fixture"));
    assert_eq!(comment.text, "A note from XlsxWriter.");
    let drawn = comment.comment_box.as_ref().expect("B2's box");
    assert_eq!(drawn.identifier.as_deref(), Some("_x0000_s1026"));
    assert_eq!((drawn.row, drawn.column), (Some(1), Some(1)));
    assert!(!drawn.is_visible);
    assert!(drawn
        .style
        .as_deref()
        .is_some_and(|style| style.contains("visibility:hidden")));

    // The button in the same part is not a comment and must not be reported as one.
    let notes = workbook
        .vml_drawing_markup(0, |drawing, interner| {
            drawing
                .all_shapes()
                .into_iter()
                .filter_map(|shape| {
                    shape
                        .attached_object_data()
                        .and_then(|data| data.kind(interner))
                })
                .collect::<Vec<_>>()
        })
        .expect("vml")
        .expect("a VML part");
    assert_eq!(
        notes,
        vec![AttachedObjectKind::PushButton, AttachedObjectKind::Comment],
        "the part holds a button and a note, in that order"
    );
    assert_eq!(workbook.sheet_comments(0).expect("comments").len(), 1);
}

#[test]
fn the_markup_accessors_hand_over_the_part_with_its_own_interner() {
    // MJXOFF-107's trap: a model read from one part carries a different interner, and a wrong one
    // answers whatever string sits at that index rather than failing. Three parts are in play here —
    // the worksheet, the comments part and the VML drawing — so each accessor hands its own over.
    let workbook = Workbook::open(&libreoffice_comments()).expect("open");
    let authors = workbook
        .comments_markup(0, |comments: &Comments, interner: &Interner| {
            let _ = interner;
            comments
                .authors()
                .map(|authors| {
                    authors
                        .authors()
                        .map(|author| author.text().to_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .expect("markup")
        .expect("a comments part");
    assert_eq!(authors, vec!["Unknown Author".to_owned()]);

    let ids = workbook
        .vml_drawing_markup(0, |drawing: &Drawing, interner: &Interner| {
            drawing
                .all_shapes()
                .into_iter()
                .filter_map(|shape| shape.identifier(interner))
                .collect::<Vec<_>>()
        })
        .expect("vml")
        .expect("a VML part");
    assert_eq!(ids, vec!["shape_0".to_owned(), "shape_0".to_owned()]);
}

#[test]
fn a_part_that_is_not_a_vml_drawing_is_refused() {
    // A `mjx_vml::Drawing` does not check the root element's own name — it is the same type that
    // reads a `w:pict` inside a Word body — so a `legacyDrawing` relationship pointing at a
    // worksheet would otherwise be parsed into plausible nonsense.
    let mut package = Package::open(&libreoffice_comments()).expect("open");
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    let vml: Vec<(String, String)> = package
        .relationships_for(Some(&sheet_part))
        .expect("rels")
        .iter()
        .filter(|rel| rel.rel_type.ends_with("/vmlDrawing"))
        .map(|rel| (rel.id.clone(), rel.rel_type.clone()))
        .collect();
    for (id, rel_type) in vml {
        package
            .remove_relationship(Some(&sheet_part), &id)
            .expect("remove");
        package
            .add_relationship(
                Some(&sheet_part),
                mjx_opc::Relationship {
                    id,
                    rel_type,
                    target: "sheet1.xml".to_owned(),
                    mode: mjx_opc::TargetMode::Internal,
                },
            )
            .expect("retarget");
    }
    let workbook = Workbook::open(&package.save().expect("save")).expect("open");
    assert!(matches!(
        workbook.vml_drawing_markup(0, |_, _| ()),
        Err(XlsxError::PartIsNotVmlDrawing(_))
    ));
}

// -------------------------------------------------------------------------------------------
// The `shapeId` hop
// -------------------------------------------------------------------------------------------

#[test]
fn a_form_control_resolves_to_its_legacy_shape_through_an_mc_alternate_content() {
    // The hop this ticket asks for, asserted against a fixture this project did not author. The
    // control's `@shapeId` is 1001 and the shape carries `o:spid="_x0000_s1001"` with a *different*
    // `@id` — so a resolver that looked only at `@id` finds nothing, and one that looked only at
    // `@o:spid` would miss every file Excel writes.
    //
    // LibreOffice wraps the whole `x:controls` list in an `mc:AlternateContent` requiring `x14`,
    // and wraps each entry in a second one, so the list never reaches `WorksheetPart`'s typed slot
    // at all.
    let workbook = Workbook::open(&libreoffice_control()).expect("open");
    let found = workbook
        .with_vml_shape_for_form_control(0, 0, |shape: &Shape, interner: &Interner| {
            (
                shape.identifier(interner),
                shape.application_shape_identifier(interner),
                shape
                    .attached_object_data()
                    .and_then(|data| data.kind(interner)),
            )
        })
        .expect("hop")
        .expect("the control's shape");
    assert_eq!(found.0.as_deref(), Some("AcceptTerms"));
    assert_eq!(found.1.as_deref(), Some("_x0000_s1001"));
    assert_eq!(found.2, Some(AttachedObjectKind::Checkbox));

    assert!(
        workbook
            .with_vml_shape_for_form_control(0, 7, |_, _| ())
            .expect("hop")
            .is_none(),
        "a control index past the end reaches no shape"
    );
    assert!(
        workbook
            .with_vml_shape_for_ole_object(0, 0, |_, _| ())
            .expect("hop")
            .is_none(),
        "the sheet lists no OLE object"
    );
}

#[test]
fn a_comment_resolves_to_its_box_by_both_routes() {
    // Route two — the `x:Row`/`x:Column` the shape states — against LibreOffice, which writes no
    // `@shapeId` at all.
    let libreoffice = Workbook::open(&libreoffice_comments()).expect("open");
    let by_cell = libreoffice
        .with_vml_shape_for_comment(0, cell("B1"), |shape: &Shape, interner: &Interner| {
            shape.style(interner)
        })
        .expect("hop")
        .expect("B1's box");
    assert!(
        by_cell.is_some_and(|style| style.contains("visibility:hidden")),
        "B1's box is the hidden one"
    );

    // Route one — `@shapeId` — against a workbook this library authored on top of a producer's.
    let mut authored = Workbook::open(&xlsxwriter_comments()).expect("open");
    let shape_id = authored
        .add_comment(0, cell("D4"), "Jai", "Route one.")
        .expect("add");
    assert_eq!(shape_id, 1027, "one past the highest the producer used");
    let identifier = authored
        .with_vml_shape_for_comment(0, cell("D4"), |shape: &Shape, interner: &Interner| {
            shape.identifier(interner)
        })
        .expect("hop")
        .expect("D4's box");
    assert_eq!(identifier.as_deref(), Some("_x0000_s1027"));

    assert!(
        authored
            .with_vml_shape_for_comment(0, cell("Z99"), |_, _| ())
            .expect("hop")
            .is_none(),
        "a cell with no comment reaches no shape"
    );
}

// -------------------------------------------------------------------------------------------
// Authoring, editing and deleting — both halves or neither
// -------------------------------------------------------------------------------------------

#[test]
fn adding_a_comment_writes_both_halves_and_leaves_every_other_part_alone() {
    let original = libreoffice_comments();
    let mut workbook = Workbook::open(&original).expect("open");
    workbook
        .add_comment(0, cell("C3"), "Jai Shukla", "A fresh note.")
        .expect("add");
    let saved = workbook.save().expect("save");

    let read_back = Workbook::open(&saved).expect("reopen");
    let comment = read_back
        .comment_at(0, cell("C3"))
        .expect("comments")
        .expect("the new comment");
    assert_eq!(comment.author.as_deref(), Some("Jai Shukla"));
    assert_eq!(comment.text, "A fresh note.");
    assert_eq!(comment.shape_id, Some(1025));
    let drawn = comment.comment_box.as_ref().expect("the new box");
    assert_eq!(drawn.identifier.as_deref(), Some("_x0000_s1025"));
    assert_eq!((drawn.row, drawn.column), (Some(2), Some(2)));
    assert_eq!(
        drawn.anchor_text.as_deref(),
        Some("3, 15, 2, 10, 5, 15, 6, 4")
    );

    // The producer's own two comments survive unchanged.
    assert_eq!(read_back.sheet_comments(0).expect("comments").len(), 3);

    // Tier 3: adding a comment to one sheet leaves every other part byte-identical — compared over
    // **saved** packages, because `part_bytes` answers `None` for an edited body.
    let before = saved_parts(&original);
    let after = saved_parts(&saved);
    let dirtied: Vec<&str> = before
        .iter()
        .filter_map(|(name, body)| {
            let changed = after
                .iter()
                .find(|(other, _)| other == name)
                .is_none_or(|(_, other)| other != body);
            changed.then_some(name.as_str())
        })
        .collect();
    assert_eq!(
        dirtied,
        ["/xl/comments1.xml", "/xl/drawings/vmlDrawing1.vml"],
        "only the two halves of the comment changed"
    );
}

#[test]
fn adding_a_comment_to_a_sheet_with_neither_half_creates_both_parts() {
    // `worksheet_spine.xlsx` has no comments part and no VML part, so this exercises the seven
    // things `add_comment` has to write rather than the two it has to edit.
    let mut workbook =
        Workbook::open(&mjx_fixtures::fixture("worksheet_spine.xlsx")).expect("open");
    workbook
        .add_comment(0, cell("B2"), "Jai", "From nothing.")
        .expect("add");
    let saved = workbook.save().expect("save");
    let package = Package::open(&saved).expect("reopen");

    let comments_part = part("/xl/comments1.xml");
    let vml_part = part("/xl/drawings/vmlDrawing1.vml");
    assert_eq!(
        package.content_type_of(&comments_part),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml")
    );
    assert_eq!(
        package.content_type_of(&vml_part),
        Some("application/vnd.openxmlformats-officedocument.vmlDrawing")
    );
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    let types: Vec<&str> = package
        .relationships_for(Some(&sheet_part))
        .expect("rels")
        .iter()
        .map(|rel| rel.rel_type.as_str())
        .collect();
    assert!(types.iter().any(|kind| kind.ends_with("/comments")));
    assert!(types.iter().any(|kind| kind.ends_with("/vmlDrawing")));

    let sheet = String::from_utf8(
        package
            .part_bytes(&sheet_part)
            .expect("sheet bytes")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        sheet.contains("<legacyDrawing r:id="),
        "the sheet claims its VML drawing at rank 30:\n{sheet}"
    );

    let read_back = Workbook::open(&saved).expect("reopen");
    assert_eq!(
        read_back
            .comment_at(0, cell("B2"))
            .expect("comments")
            .expect("the comment")
            .text,
        "From nothing."
    );
    // The invariant the whole feature rests on holds for a package this library built from nothing.
    read_back.validate().expect("both halves are present");
}

#[test]
fn setting_a_comments_text_leaves_its_box_exactly_as_it_was() {
    let original = libreoffice_comments();
    let mut workbook = Workbook::open(&original).expect("open");
    assert!(workbook
        .set_comment_text(0, cell("A2"), "Rewritten.")
        .expect("set"));
    assert!(
        !workbook
            .set_comment_text(0, cell("Z9"), "nobody")
            .expect("set"),
        "a cell with no comment reports false and writes nothing"
    );
    let saved = workbook.save().expect("save");
    assert_eq!(
        Workbook::open(&saved)
            .expect("reopen")
            .comment_at(0, cell("A2"))
            .expect("comments")
            .expect("A2")
            .text,
        "Rewritten."
    );
    assert_eq!(
        saved_part(&saved, "/xl/drawings/vmlDrawing1.vml"),
        saved_part(&original, "/xl/drawings/vmlDrawing1.vml"),
        "the box is untouched by a text edit"
    );
}

#[test]
fn removing_a_comment_removes_both_halves() {
    let mut workbook = Workbook::open(&libreoffice_comments()).expect("open");
    assert!(workbook.remove_comment(0, cell("A2")).expect("remove"));
    let saved = workbook.save().expect("save");
    let read_back = Workbook::open(&saved).expect("reopen");

    let left = read_back.sheet_comments(0).expect("comments");
    assert_eq!(left.len(), 1, "one comment left on Budget");
    assert_eq!(left[0].cell, cell("B1"));
    let shapes = read_back
        .vml_drawing_markup(0, |drawing, interner| {
            drawing
                .all_shapes()
                .into_iter()
                .filter_map(|shape| {
                    shape
                        .attached_object_data()
                        .and_then(|data| data.kind(interner))
                })
                .collect::<Vec<_>>()
        })
        .expect("vml")
        .expect("a VML part");
    assert_eq!(
        shapes,
        vec![AttachedObjectKind::Comment],
        "the removed comment's box went with it"
    );
    read_back.validate().expect("both halves still agree");

    assert!(
        !Workbook::open(&saved)
            .expect("reopen")
            .remove_comment(0, cell("A2"))
            .expect("remove"),
        "removing a comment that is not there reports false"
    );
}

#[test]
fn removing_the_last_comment_removes_the_comments_part_and_leaves_the_vml_alone() {
    // The VML part is *not* removed with the comments part: it also draws form controls and OLE
    // fallbacks. `legacy_form_control.xlsx` is the case that proves it — one note and one check box
    // in one `vmlDrawing1.vml`.
    let mut workbook = Workbook::open(&libreoffice_control()).expect("open");
    assert!(workbook.remove_comment(0, cell("A1")).expect("remove"));
    let saved = workbook.save().expect("save");
    let package = Package::open(&saved).expect("reopen");

    // Asked of the part list, not of `content_type_of`: a package with an `xml` **Default** answers
    // `application/xml` for a part it does not hold at all, so a content-type probe would pass
    // whether or not the part went.
    let names: Vec<String> = package
        .part_names()
        .map(|name| name.as_str().to_owned())
        .collect();
    assert!(
        !names.iter().any(|name| name == "/xl/comments1.xml"),
        "an empty comments part is removed, not left behind: {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|name| name == "/xl/drawings/vmlDrawing1.vml"),
        "the VML part stays: the check box still lives in it"
    );
    let sheet_part = part("/xl/worksheets/sheet1.xml");
    assert!(
        !package
            .relationships_for(Some(&sheet_part))
            .expect("rels")
            .iter()
            .any(|rel| rel.rel_type.ends_with("/comments")),
        "the comments relationship went with the part"
    );

    // The control still resolves, which is the point of leaving the VML part alone.
    let read_back = Workbook::open(&saved).expect("reopen");
    assert!(read_back
        .with_vml_shape_for_form_control(0, 0, |_, _| ())
        .expect("hop")
        .is_some());
    read_back.validate().expect("no half-comments remain");
}

#[test]
fn editing_the_vml_drawing_dirties_only_that_part_and_reflows_only_what_changed() {
    let original = libreoffice_comments();
    let mut workbook = Workbook::open(&original).expect("open");
    workbook
        .edit_vml_drawing_markup(0, |drawing: &mut Drawing, interner: &mut Interner| {
            let shape = drawing
                .content_mut()
                .iter_mut()
                .find_map(|child| match child {
                    mjx_vml::DrawingContent::Shape(shape) => Some(shape),
                    _ => None,
                })
                .expect("the first shape");
            shape.set_fill_color(interner, "#ff0000");
        })
        .expect("edit")
        .expect("a VML part");
    let saved = workbook.save().expect("save");

    let before = saved_parts(&original);
    let after = saved_parts(&saved);
    let dirtied: Vec<&str> = before
        .iter()
        .filter_map(|(name, body)| {
            let changed = after
                .iter()
                .find(|(other, _)| other == name)
                .is_none_or(|(_, other)| other != body);
            changed.then_some(name.as_str())
        })
        .collect();
    assert_eq!(dirtied, ["/xl/drawings/vmlDrawing1.vml"]);

    // MJXOFF-143's span-preserving write-back: the *second* shape was not touched, so its bytes
    // come back exactly — the wrapped `v:shapetype` before it included.
    let text = String::from_utf8(saved_part(&saved, "/xl/drawings/vmlDrawing1.vml").expect("vml"))
        .expect("utf-8");
    assert!(
        text.contains("fillcolor=\"#ff0000\""),
        "the edit landed:\n{text}"
    );
    assert!(
        text.contains(
            "<v:shape id=\"shape_0\" fillcolor=\"#ffffc0\" stroked=\"t\" type=\"#_x0000_t202\" \
             o:allowincell=\"f\""
        ),
        "the shape nothing touched kept its own attribute order and its `o:` attribute:\n{text}"
    );
    assert!(
        text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"),
        "the producer's own prologue survived an edit"
    );
}

// -------------------------------------------------------------------------------------------
// The two-halves invariant
// -------------------------------------------------------------------------------------------

#[test]
fn a_comment_whose_box_was_deleted_refuses_to_save() {
    let mut workbook = Workbook::open(&libreoffice_comments()).expect("open");
    // Remove *only* the box, which is exactly the half a careless delete would leave behind.
    workbook
        .edit_vml_drawing_markup(0, |drawing: &mut Drawing, _| {
            drawing
                .content_mut()
                .retain(|child| !matches!(child, mjx_vml::DrawingContent::Shape(_)));
        })
        .expect("edit")
        .expect("a VML part");
    let error = workbook
        .save()
        .expect_err("a comment with no box must be refused");
    assert!(
        matches!(
            &error,
            XlsxError::InvalidWorkbook(defect)
                if matches!(**defect, SpreadsheetDefect::CommentWithoutABox { .. })
        ),
        "wrong defect: {error}"
    );
}

#[test]
fn a_box_whose_comment_was_deleted_refuses_to_save() {
    let mut workbook = Workbook::open(&libreoffice_comments()).expect("open");
    // Remove *only* the text half.
    workbook
        .edit_comments_markup(0, |comments: &mut Comments, _| {
            if let Some(list) = comments.list_mut() {
                while list.remove(0).is_some() {}
            }
        })
        .expect("edit")
        .expect("a comments part");
    let error = workbook
        .save()
        .expect_err("a box with no comment must be refused");
    assert!(
        matches!(
            &error,
            XlsxError::InvalidWorkbook(defect)
                if matches!(**defect, SpreadsheetDefect::CommentBoxWithoutAComment { .. })
        ),
        "wrong defect: {error}"
    );
}

#[test]
fn a_producer_workbook_nobody_touched_is_never_faulted_for_its_own_comments() {
    // The scope rule every markup check here follows: a workbook opened and saved untouched is not
    // held to invariants about markup it arrived with. Without this the three fixtures above could
    // not be opened at all.
    for bytes in [
        libreoffice_comments(),
        libreoffice_control(),
        xlsxwriter_comments(),
    ] {
        Workbook::open(&bytes)
            .expect("open")
            .validate()
            .expect("an untouched producer workbook validates");
    }
}

#[test]
fn a_comment_text_that_is_authored_writes_a_t_element_rather_than_bare_characters() {
    // A `CT_Rst` carries no character data of its own, so an authored comment has to write a `t`
    // child. Asserted on the bytes, because the model would read either back the same way.
    let mut workbook =
        Workbook::open(&mjx_fixtures::fixture("worksheet_spine.xlsx")).expect("open");
    workbook
        .add_comment(0, cell("A1"), "Jai", "  padded  ")
        .expect("add");
    let saved = workbook.save().expect("save");
    let comments = String::from_utf8(saved_part(&saved, "/xl/comments1.xml").expect("comments"))
        .expect("utf-8");
    assert!(
        comments.contains("<text><t xml:space=\"preserve\">  padded  </t></text>"),
        "an authored comment writes a `t`, and `xml:space` where the string needs it:\n{comments}"
    );
    assert_eq!(
        Workbook::open(&saved)
            .expect("reopen")
            .comment_at(0, cell("A1"))
            .expect("comments")
            .expect("A1")
            .text,
        "  padded  "
    );
}

#[test]
fn the_comment_text_model_keeps_a_producers_runs_until_it_is_replaced() {
    let workbook = Workbook::open(&libreoffice_comments()).expect("open");
    let has_runs = workbook
        .comments_markup(0, |comments: &Comments, interner: &Interner| {
            let _ = interner;
            comments
                .list()
                .and_then(|list| list.comments().next())
                .and_then(|comment| comment.text().map(CommentText::runs_markup))
                .map(|markup| markup.is_some())
        })
        .expect("markup")
        .expect("a comments part");
    assert_eq!(
        has_runs,
        Some(true),
        "a comment read from a file replays the runs it was written with"
    );
}
