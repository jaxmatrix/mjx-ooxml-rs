//! **The two defects MJXOFF-175 and MJXOFF-176 declared rather than fixed**, closed and gated here.
//!
//! # Both were layout errors hiding inside rendering deferrals
//!
//! R20 wrote *"the body's run stream contributes no character for a footnote's reference mark, so
//! there is nothing on that line to hang a glyph on"*, and R21 wrote *"an inline drawing's advance
//! is not measured … a line carrying one is measured as if the drawing were not on it"*. Both read
//! as *we do not draw this yet*, and neither is: **a line measured too narrow breaks in the wrong
//! place, so the page breaks in the wrong place, so every later paragraph is on the wrong page.**
//!
//! Neither child built around the wrong value — no assertion anywhere depended on it — which is why
//! they could be fixed without unpicking anything. This suite is what stops them coming back.
//!
//! # And a third, found on the way
//!
//! `crate::float::inline_height` was written by R21 to raise a line for an inline object's **height**
//! and is called by nothing at all — a grep of the crate finds it only in its own definition and in
//! the `pub use`. So an inline picture changed neither the width of its line nor its height.
//! [`an_inline_drawing_raises_the_line_it_sits_on`] is the assertion for the second half.

mod support;

use mjx_layout_docx::{InlineObjectKind, PieceKind};
use mjx_ooxml_core::measure::Emu;
use support::generated::{inline_drawing_paragraph, plain_run, raw_paragraph};
use support::{
    document, document_with_footnote, flow, model, note_entry, notes_part, page_geometry,
    referencing_note,
};

/// How wide the first line of block `index` comes out, and how tall.
fn first_line(paragraphs: &[String], index: usize) -> (Emu, Emu) {
    let mut document = document(paragraphs);
    let read = flow(&mut document);
    measure(&read, index)
}

/// The same for a document that already exists.
fn measure(read: &mjx_layout_docx::DocumentFlow, index: usize) -> (Emu, Emu) {
    let mut model = model();
    let request = mjx_layout_docx::LayoutRequest {
        width: Emu::from_inches(6.5),
        top: Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(read, index, request)
        .expect("the block lays out");
    let paragraph = block.as_paragraph().expect("a paragraph");
    let line = paragraph.lines.first().expect("at least one line");
    let width = line
        .placement
        .segments
        .last()
        .map_or(Emu::ZERO, |placed| placed.x + placed.width);
    (width, line.height)
}

#[test]
fn a_footnote_reference_mark_is_on_the_line_and_has_a_width() {
    // MJXOFF-175's declared gap. The same sentence twice **in one document**, once with a footnote
    // reference and once without: the referenced one is wider, by the width of the superscript
    // numeral a reader sees.
    //
    // One document, and both paragraphs written by the same builder, because the control has to
    // differ in *one* thing. Two documents built by two helpers differ in their run properties as
    // well, and the first version of this test compared a 12-point line against a 10-point one and
    // read the size difference as evidence.
    let mut document = document_with_footnote(
        &[
            referencing_note(true, 2, "a sentence"),
            support::paragraph("", "a sentence"),
        ],
        &page_geometry(8.5, 11.0, 1.0),
        2,
        &["the note itself"],
    );
    let read = flow(&mut document);
    let (marked, _) = measure(&read, 0);
    let (plain, _) = measure(&read, 1);
    assert!(
        marked > plain,
        "the reference mark takes width on the line: {marked:?} against {plain:?}"
    );
    let _ = (first_line, raw_paragraph, plain_run);
}

#[test]
fn the_mark_is_a_generated_piece_and_not_document_text() {
    // A generated mark is drawn and is **not** in the file, so its piece maps to the empty document
    // range at its anchor — which is what stops a caret landing inside a character no byte of the
    // `.docx` holds.
    let mut document = document_with_footnote(
        &[referencing_note(true, 2, "a sentence")],
        &page_geometry(8.5, 11.0, 1.0),
        2,
        &["the note itself"],
    );
    let read = flow(&mut document);
    let mut model = model();
    let request = mjx_layout_docx::LayoutRequest {
        width: Emu::from_inches(6.5),
        top: Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(&read, 0, request)
        .expect("the block lays out");
    let composition = &block.as_paragraph().expect("a paragraph").composition;
    let marks: Vec<&mjx_layout_docx::Piece> = composition
        .pieces()
        .iter()
        .filter(|piece| piece.kind == PieceKind::NoteMark)
        .collect();
    assert_eq!(marks.len(), 1, "one reference, one mark");
    assert!(
        marks[0].document.is_empty(),
        "and it names no bytes of the document: {:?}",
        marks[0].document
    );
    assert_eq!(
        composition.text().get(marks[0].layout.clone()),
        Some("1"),
        "the first footnote's mark is `1`"
    );
}

#[test]
fn the_mark_is_smaller_than_the_run_it_sits_in() {
    // A reference mark is superscript, and superscript is *smaller* — which is a fact about a width
    // and therefore this crate's to decide rather than a painter's. See
    // `crate::generated::SUPERSCRIPT_SCALE`.
    let mut document = document_with_footnote(
        &[referencing_note(true, 2, "a sentence")],
        &page_geometry(8.5, 11.0, 1.0),
        2,
        &["the note itself"],
    );
    let read = flow(&mut document);
    let mut model = model();
    let request = mjx_layout_docx::LayoutRequest {
        width: Emu::from_inches(6.5),
        top: Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(&read, 0, request)
        .expect("the block lays out");
    let composition = &block.as_paragraph().expect("a paragraph").composition;
    let mark = composition
        .pieces()
        .iter()
        .find(|piece| piece.kind == PieceKind::NoteMark)
        .expect("a mark");
    let mark_size = composition
        .runs()
        .iter()
        .find(|run| run.range.start == mark.layout.start)
        .map(|run| run.size.in_points())
        .expect("the mark has a run");
    let body_size = composition
        .runs()
        .iter()
        .find(|run| run.range.start == 0)
        .map(|run| run.size.in_points())
        .expect("the body has a run");
    assert!(
        mark_size < body_size,
        "a superscript mark is smaller: {mark_size} against {body_size}"
    );
}

#[test]
fn a_second_footnote_carries_the_next_number() {
    // The mark is the note's own ordinal, not a constant. An implementation that drew `1` for every
    // reference would satisfy every width assertion above.
    let paragraphs = vec![
        referencing_note(true, 2, "first"),
        referencing_note(true, 3, "second"),
    ];
    let mut document = support::document_with_notes(
        &paragraphs,
        &page_geometry(8.5, 11.0, 1.0),
        true,
        &[
            note_entry("footnote", 2, None, &["the first note"]),
            note_entry("footnote", 3, None, &["the second note"]),
        ],
    );
    let read = flow(&mut document);
    let mut model = model();
    let marks: Vec<String> = (0..2)
        .map(|index| {
            let request = mjx_layout_docx::LayoutRequest {
                width: Emu::from_inches(6.5),
                top: Emu::ZERO,
                exclusions: &[],
            };
            let block = model
                .lay_out_block(&read, index, request)
                .expect("the block lays out");
            let composition = block
                .as_paragraph()
                .expect("a paragraph")
                .composition
                .clone();
            composition
                .pieces()
                .iter()
                .find(|piece| piece.kind == PieceKind::NoteMark)
                .and_then(|piece| composition.text().get(piece.layout.clone()))
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(marks, vec!["1".to_owned(), "2".to_owned()]);
    let _ = notes_part;
}

#[test]
fn an_inline_drawing_reserves_its_own_width_on_the_line() {
    // MJXOFF-176's declared gap. Two paragraphs of the same text, one with a two-inch inline picture
    // after it: the one with the picture is two inches wider. Before MJXOFF-177 they were equal.
    let with_picture = inline_drawing_paragraph("before", 1_828_800, 457_200);
    let without = raw_paragraph(&plain_run("before"));
    let (wide, _) = first_line(&[with_picture], 0);
    let (narrow, _) = first_line(&[without], 0);
    let difference = wide - narrow;
    assert!(
        difference >= Emu::from_inches(1.9) && difference <= Emu::from_inches(2.1),
        "the line is wider by the object's own advance: {difference:?}"
    );
}

#[test]
fn an_inline_drawing_raises_the_line_it_sits_on() {
    // The other half, and the one `crate::float::inline_height` was written for and never called: an
    // inline object is a character of the line, so it raises the line's ascent to its own height.
    let tall = inline_drawing_paragraph("before", 457_200, 1_828_800);
    let without = raw_paragraph(&plain_run("before"));
    let (_, raised) = first_line(&[tall], 0);
    let (_, ordinary) = first_line(&[without], 0);
    assert!(
        raised > ordinary,
        "a two-inch picture makes its line taller than a line of text: {raised:?} against \
         {ordinary:?}"
    );
}

#[test]
fn an_inline_object_is_one_atomic_character_and_carries_no_glyphs() {
    // The mechanism: one `U+FFFC`, in a run of its own, with a fixed advance — so the line breaker
    // treats it as an opaque unit (UAX #14 class `CB`) and the shaper is never asked for a glyph it
    // has no way to draw.
    let mut document = document(&[inline_drawing_paragraph("before", 914_400, 457_200)]);
    let read = flow(&mut document);
    let mut model = model();
    let request = mjx_layout_docx::LayoutRequest {
        width: Emu::from_inches(6.5),
        top: Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(&read, 0, request)
        .expect("the block lays out");
    let paragraph = block.as_paragraph().expect("a paragraph");
    let composition = &paragraph.composition;
    assert_eq!(composition.objects().len(), 1);
    let object = composition.objects()[0];
    assert_eq!(object.kind, InlineObjectKind::Drawing(0));
    assert_eq!(object.width, Emu::from_emu(914_400));
    assert_eq!(
        composition.text().get(object.at..object.at + 3),
        Some("\u{FFFC}"),
        "one object replacement character stands for it"
    );
    let segment = paragraph
        .lines
        .first()
        .and_then(|line| {
            line.composed
                .segments
                .iter()
                .find(|segment| segment.range.start == object.at)
        })
        .expect("the object is a segment of the line");
    assert!(
        segment.run.glyphs().is_empty(),
        "and it carries no glyphs, so nothing draws a `.notdef` box for it"
    );
    assert!(
        (segment.width_in_points - object.width.points()).abs() < 1e-6,
        "while its width is the object's own: {} against {}",
        segment.width_in_points,
        object.width.points()
    );
}

#[test]
fn an_inline_object_moves_a_line_break() {
    // The consequence that matters. A line whose text just fits the measure no longer fits once an
    // object is on it, so it breaks a word earlier — which is the difference between a page that
    // matches Word's and one that does not.
    let text = "one two three four five six seven eight nine ten eleven twelve";
    let plain = raw_paragraph(&plain_run(text));
    let with_object = inline_drawing_paragraph(text, 2_743_200, 152_400);
    let lines_of = |markup: &String| -> usize {
        let mut document = document(std::slice::from_ref(markup));
        let read = flow(&mut document);
        let mut model = model();
        let request = mjx_layout_docx::LayoutRequest {
            width: Emu::from_inches(3.0),
            top: Emu::ZERO,
            exclusions: &[],
        };
        model
            .lay_out_block(&read, 0, request)
            .expect("the block lays out")
            .as_paragraph()
            .expect("a paragraph")
            .lines
            .len()
    };
    assert!(
        lines_of(&with_object) > lines_of(&plain),
        "a three-inch object on a three-inch measure takes a line of its own"
    );
}
