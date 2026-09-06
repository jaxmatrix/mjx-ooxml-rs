//! MJXOFF-107 (E3) — `xdr:wsDr`, the three anchor modes, and what each one promises when the grid
//! under it moves.
//!
//! # The non-discriminating test this file exists to avoid
//!
//! The ticket names it: *"a fixture whose anchors we authored will resolve to the coordinates we
//! authored them from, whichever direction the conversion is wrong in."* So nothing here reads back
//! only what it wrote:
//!
//! * every anchor read is parsed from a **fragment written out in full**, with offsets that are not
//!   zero and column/row indices that are not equal, so a reader that swapped `col` for `row` or
//!   `colOff` for `rowOff` fails;
//! * the `editAs` case uses a `twoCellAnchor` whose attribute says **`oneCell`** — a value that
//!   disagrees with the element name, which is what Apache POI writes and what
//!   `tests/fixtures/worksheet_drawings.xlsx` carries;
//! * the three shift cases assert three *different* outcomes from the same edit, so a shift that
//!   moved everything (or nothing) fails at least two of them.

use mjx_dml::spreadsheet_drawing::{
    new_absolute_anchor, new_anchored_picture, new_one_cell_anchor, new_two_cell_anchor, Anchor,
    AnchoredObject, CellMarker, WorksheetDrawing,
};
use mjx_dml::{Position, Size};
use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_xml::fidelity;

const XDR: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A drawing part with one anchor of each kind, whose numbers all differ from each other.
fn three_anchor_part() -> String {
    format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}" xmlns:a="{A}" xmlns:r="{R}">
  <xdr:twoCellAnchor editAs="oneCell">
    <xdr:from><xdr:col>1</xdr:col><xdr:colOff>190500</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>47625</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>3</xdr:col><xdr:colOff>95250</xdr:colOff><xdr:row>5</xdr:row><xdr:rowOff>19050</xdr:rowOff></xdr:to>
    <xdr:pic>
      <xdr:nvPicPr><xdr:cNvPr id="7" name="two-cell"/><xdr:cNvPicPr/></xdr:nvPicPr>
      <xdr:blipFill><a:blip r:embed="rId4"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>
      <xdr:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr>
    </xdr:pic>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
  <xdr:oneCellAnchor>
    <xdr:from><xdr:col>4</xdr:col><xdr:colOff>76200</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>38100</xdr:rowOff></xdr:from>
    <xdr:ext cx="914400" cy="457200"/>
    <xdr:sp macro="" textlink="">
      <xdr:nvSpPr><xdr:cNvPr id="8" name="one-cell"/><xdr:cNvSpPr/></xdr:nvSpPr>
      <xdr:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr>
    </xdr:sp>
    <xdr:clientData fPrintsWithSheet="0"/>
  </xdr:oneCellAnchor>
  <xdr:absoluteAnchor>
    <xdr:pos x="1905000" y="952500"/>
    <xdr:ext cx="685800" cy="342900"/>
    <xdr:pic>
      <xdr:nvPicPr><xdr:cNvPr id="9" name="absolute"/><xdr:cNvPicPr/></xdr:nvPicPr>
      <xdr:blipFill><a:blip r:embed="rId5"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>
      <xdr:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr>
    </xdr:pic>
    <xdr:clientData/>
  </xdr:absoluteAnchor>
</xdr:wsDr>"#
    )
}

fn read(part: &str) -> (WorksheetDrawing, RawDocument) {
    let document = fidelity::parse(part.as_bytes()).expect("the fragment parses");
    let drawing = WorksheetDrawing::read_part(&document)
        .expect("it reads")
        .expect("its root is an xdr:wsDr");
    (drawing, document)
}

fn serialize(drawing: &WorksheetDrawing, mut interner: Interner) -> String {
    let root = drawing.to_xml(&mut interner);
    let document = RawDocument::new(interner, false, Vec::new(), root, Vec::new());
    String::from_utf8(fidelity::serialize_to_vec(&document)).expect("utf-8")
}

// -------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------

#[test]
fn a_part_rooted_somewhere_else_is_a_question_rather_than_an_error() {
    let document =
        fidelity::parse(br#"<worksheet xmlns="http://x"/>"#).expect("the fragment parses");
    assert!(WorksheetDrawing::read_part(&document)
        .expect("no error")
        .is_none());
}

#[test]
fn the_three_anchor_kinds_are_told_apart_and_each_reports_its_own_geometry() {
    let (drawing, document) = read(&three_anchor_part());
    let interner = &document.interner;
    assert_eq!(drawing.anchor_count(interner), 3);

    let anchors: Vec<Anchor> = drawing.anchors(interner).collect();
    let Anchor::TwoCell(two) = &anchors[0] else {
        panic!("the first anchor is an xdr:twoCellAnchor");
    };
    let Anchor::OneCell(one) = &anchors[1] else {
        panic!("the second anchor is an xdr:oneCellAnchor");
    };
    let Anchor::Absolute(absolute) = &anchors[2] else {
        panic!("the third anchor is an xdr:absoluteAnchor");
    };

    // Every number differs from every other, so a reader that crossed two fields fails here.
    assert_eq!(
        two.from_marker(interner),
        Some(CellMarker::new(1, 190_500, 2, 47_625))
    );
    assert_eq!(
        two.to_marker(interner),
        Some(CellMarker::new(3, 95_250, 5, 19_050))
    );
    assert_eq!(
        one.from_marker(interner),
        Some(CellMarker::new(4, 76_200, 1, 38_100))
    );
    assert_eq!(one.extent(interner), Some(Size::from_emu(914_400, 457_200)));
    assert!(
        one.from_marker(interner).is_some() && two.to_marker(interner) != one.from_marker(interner)
    );
    assert_eq!(
        absolute.position(interner),
        Some(Position::from_emu(1_905_000, 952_500))
    );
    assert_eq!(
        absolute.extent(interner),
        Some(Size::from_emu(685_800, 342_900))
    );
}

#[test]
fn edit_as_is_read_from_the_attribute_and_not_inferred_from_the_element_name() {
    let (drawing, document) = read(&three_anchor_part());
    let interner = &document.interner;
    let anchors: Vec<Anchor> = drawing.anchors(interner).collect();

    // The element is a `twoCellAnchor`; the attribute says `oneCell`. The attribute wins.
    assert_eq!(
        anchors[0].resizing_behavior(interner),
        ResizingBehavior::MoveWithCellsButDoNotResize
    );
    // …and the two anchors that carry no `@editAs` answer from what they *are*.
    assert_eq!(
        anchors[1].resizing_behavior(interner),
        ResizingBehavior::MoveWithCellsButDoNotResize
    );
    assert_eq!(
        anchors[2].resizing_behavior(interner),
        ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns
    );
}

#[test]
fn a_two_cell_anchor_with_no_edit_as_takes_the_schema_default() {
    let part = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}" xmlns:a="{A}">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="s"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr/></xdr:sp>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#
    );
    let (drawing, document) = read(&part);
    assert_eq!(
        drawing
            .anchor(&document.interner, 0)
            .expect("one anchor")
            .resizing_behavior(&document.interner),
        ResizingBehavior::MoveAndResizeWithAnchorCells
    );
}

#[test]
fn client_data_defaults_both_flags_to_true_and_reads_a_written_false() {
    let (drawing, document) = read(&three_anchor_part());
    let interner = &document.interner;
    let anchors: Vec<Anchor> = drawing.anchors(interner).collect();

    // `<xdr:clientData/>` — no attribute at all, and `CT_AnchorClientData` defaults *both* to true.
    let empty = anchors[0].client_data(interner).expect("clientData");
    assert!(empty.locks_with_sheet(interner));
    assert!(empty.prints_with_sheet(interner));

    // The one that says otherwise, so the `true` above cannot be a constant.
    let written = anchors[1].client_data(interner).expect("clientData");
    assert!(written.locks_with_sheet(interner));
    assert!(!written.prints_with_sheet(interner));
}

#[test]
fn an_anchored_object_reports_its_kind_its_identity_and_its_image_relationship() {
    let (drawing, document) = read(&three_anchor_part());
    let interner = &document.interner;
    let anchors: Vec<Anchor> = drawing.anchors(interner).collect();

    let AnchoredObject::Picture(picture) = anchors[0].object(interner).expect("an object") else {
        panic!("the first anchor holds an xdr:pic");
    };
    assert_eq!(
        picture.image_relationship_id(interner).as_deref(),
        Some("rId4")
    );
    let identity = picture.identity(interner).expect("cNvPr");
    assert_eq!(identity.id(interner).ok(), Some(7));
    assert_eq!(identity.drawing_name(interner).as_deref(), Some("two-cell"));

    let AnchoredObject::Shape(shape) = anchors[1].object(interner).expect("an object") else {
        panic!("the second anchor holds an xdr:sp");
    };
    // `@fLocksText` is absent, and its schema default is `true` — the trap this type carries.
    assert!(shape.locks_text(interner));
    assert_eq!(
        shape.macro_reference(interner).ok().flatten().as_deref(),
        Some("")
    );
    assert!(shape.shape_properties(interner).is_some());

    // …and the third anchor's picture names a *different* relationship, so the first answer above
    // cannot have come from a lookup that ignores which picture it was asked about.
    let AnchoredObject::Picture(third) = anchors[2].object(interner).expect("an object") else {
        panic!("the third anchor holds an xdr:pic");
    };
    assert_eq!(
        third.image_relationship_id(interner).as_deref(),
        Some("rId5")
    );
}

// -------------------------------------------------------------------------------------------
// Round-tripping
// -------------------------------------------------------------------------------------------

#[test]
fn an_untouched_part_re_emits_byte_for_byte_through_the_model() {
    // Not through the document it was parsed from — that would only prove `mjx-xml` copies a buffer.
    // The model is rebuilt into a fresh element with a fresh interner and serialized from *that*, so
    // the assertion is about what this module stores: attribute order, prefixes, the whitespace
    // between siblings, and the self-closing spelling of `<xdr:clientData/>`.
    let part = three_anchor_part();
    let (drawing, document) = read(&part);
    assert_eq!(serialize(&drawing, document.interner), part);
}

#[test]
fn an_edited_anchor_re_emits_and_leaves_its_siblings_alone() {
    let part = three_anchor_part();
    let (mut drawing, document) = read(&part);
    let mut interner = document.interner;

    let Some(Anchor::TwoCell(mut two)) = drawing.anchor(&interner, 0) else {
        panic!("the first anchor is a two-cell anchor");
    };
    two.set_from_marker(&mut interner, CellMarker::new(2, 11_111, 6, 22_222));
    assert!(drawing.replace_anchor(&mut interner, 0, &Anchor::TwoCell(two)));

    let rendered = serialize(&drawing, interner);
    assert!(
        rendered.contains("<xdr:col>2</xdr:col><xdr:colOff>11111</xdr:colOff><xdr:row>6</xdr:row><xdr:rowOff>22222</xdr:rowOff>"),
        "the edited marker must be written in schema order:\n{rendered}"
    );
    // The untouched anchors are still exactly what the file said.
    assert!(rendered.contains(r#"<xdr:ext cx="914400" cy="457200"/>"#));
    assert!(rendered.contains(r#"<xdr:pos x="1905000" y="952500"/>"#));
    assert!(rendered.contains(r#"r:embed="rId5""#));
}

#[test]
fn an_unmodelled_child_of_an_anchor_survives_an_edit_beside_it() {
    let part = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}" xmlns:a="{A}" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="s"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr/></xdr:sp>
    <mc:AlternateContent><mc:Fallback>kept</mc:Fallback></mc:AlternateContent>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#
    );
    let (mut drawing, document) = read(&part);
    let mut interner = document.interner;
    let Some(Anchor::TwoCell(mut two)) = drawing.anchor(&interner, 0) else {
        panic!("a two-cell anchor");
    };
    two.set_to_marker(&mut interner, CellMarker::new(4, 5, 6, 7));
    assert!(drawing.replace_anchor(&mut interner, 0, &Anchor::TwoCell(two)));
    let rendered = serialize(&drawing, interner);
    assert!(
        rendered
            .contains("<mc:AlternateContent><mc:Fallback>kept</mc:Fallback></mc:AlternateContent>"),
        "an unmodelled child must survive an edit beside it:\n{rendered}"
    );
    assert!(rendered.contains("<xdr:col>4</xdr:col>"));
}

// -------------------------------------------------------------------------------------------
// Authoring
// -------------------------------------------------------------------------------------------

#[test]
fn the_three_authored_anchors_are_written_in_schema_order() {
    let mut interner = Interner::default();
    let mut drawing = WorksheetDrawing::new(&mut interner);

    let picture = new_anchored_picture(&mut interner, 1, "authored", "rId9");
    let object = AnchoredObject::Picture(picture);
    let two = new_two_cell_anchor(
        &mut interner,
        CellMarker::new(1, 100, 2, 200),
        CellMarker::new(3, 300, 4, 400),
        &object,
        ResizingBehavior::MoveWithCellsButDoNotResize,
    );
    let one = new_one_cell_anchor(
        &mut interner,
        CellMarker::new(5, 500, 6, 600),
        Size::from_emu(700, 800),
        &object,
    );
    let absolute = new_absolute_anchor(
        &mut interner,
        Position::from_emu(900, 1000),
        Size::from_emu(1100, 1200),
        &object,
    );
    drawing.push_anchor(&mut interner, &Anchor::TwoCell(two));
    drawing.push_anchor(&mut interner, &Anchor::OneCell(one));
    drawing.push_anchor(&mut interner, &Anchor::Absolute(absolute));

    let rendered = serialize(&drawing, interner);
    // `from` before `to` before the object before `clientData`, and `pos` before `ext`.
    let two_cell = rendered
        .split("<xdr:twoCellAnchor")
        .nth(1)
        .expect("a two-cell anchor");
    let order = |needle: &str| two_cell.find(needle).unwrap_or(usize::MAX);
    assert!(order("<xdr:from>") < order("<xdr:to>"));
    assert!(order("<xdr:to>") < order("<xdr:pic>"));
    assert!(order("<xdr:pic>") < order("<xdr:clientData/>"));
    assert!(rendered.contains(r#"editAs="oneCell""#));

    let absolute_part = rendered
        .split("<xdr:absoluteAnchor")
        .nth(1)
        .expect("an absolute anchor");
    assert!(
        absolute_part.find("<xdr:pos").unwrap_or(usize::MAX)
            < absolute_part.find("<xdr:ext").unwrap_or(usize::MAX)
    );

    // …and it reads back as what was written, through a fresh parse rather than the model in hand.
    let (reparsed, document) = read(&rendered);
    assert_eq!(reparsed.anchor_count(&document.interner), 3);
    let Some(Anchor::TwoCell(two)) = reparsed.anchor(&document.interner, 0) else {
        panic!("a two-cell anchor");
    };
    assert_eq!(
        two.from_marker(&document.interner),
        Some(CellMarker::new(1, 100, 2, 200))
    );
    assert_eq!(
        two.resizing_behavior(&document.interner),
        ResizingBehavior::MoveWithCellsButDoNotResize
    );
}

#[test]
fn the_schema_default_edit_as_is_not_written_out() {
    let mut interner = Interner::default();
    let mut drawing = WorksheetDrawing::new(&mut interner);
    let object = AnchoredObject::Picture(new_anchored_picture(&mut interner, 1, "p", "rId1"));
    let two = new_two_cell_anchor(
        &mut interner,
        CellMarker::new(0, 0, 0, 0),
        CellMarker::new(1, 0, 1, 0),
        &object,
        ResizingBehavior::MoveAndResizeWithAnchorCells,
    );
    drawing.push_anchor(&mut interner, &Anchor::TwoCell(two));
    let rendered = serialize(&drawing, interner);
    assert!(
        !rendered.contains("editAs"),
        "the schema default must not be authored:\n{rendered}"
    );
}

// -------------------------------------------------------------------------------------------
// The three anchor modes, under a sheet edit — three assertions that differ from each other
// -------------------------------------------------------------------------------------------

#[test]
fn inserting_a_row_above_every_anchor_moves_each_one_the_way_its_own_mode_promises() {
    let (mut drawing, document) = read(&three_anchor_part());
    let mut interner = document.interner;

    // Row 0 — above all three objects (their `from` rows are 2, 1 and «none»).
    let report = drawing.insert_rows(&mut interner, 0, 3);
    assert_eq!(report.len(), 3);

    let anchors: Vec<Anchor> = drawing.anchors(&interner).collect();

    // 1. The two-cell anchor moved **and** kept its span: both markers moved by three.
    let Anchor::TwoCell(two) = &anchors[0] else {
        panic!("a two-cell anchor")
    };
    assert_eq!(
        two.from_marker(&interner),
        Some(CellMarker::new(1, 190_500, 5, 47_625))
    );
    assert_eq!(
        two.to_marker(&interner),
        Some(CellMarker::new(3, 95_250, 8, 19_050))
    );
    assert!(report[0].moved && !report[0].resized && report[0].promise_kept);

    // 2. The one-cell anchor moved and its **extent is untouched** — a different outcome, and one
    //    a two-cell anchor cannot produce because it has no extent at all.
    let Anchor::OneCell(one) = &anchors[1] else {
        panic!("a one-cell anchor")
    };
    assert_eq!(
        one.from_marker(&interner),
        Some(CellMarker::new(4, 76_200, 4, 38_100))
    );
    assert_eq!(
        one.extent(&interner),
        Some(Size::from_emu(914_400, 457_200))
    );
    assert!(report[1].moved && !report[1].resized);

    // 3. The absolute anchor did **neither**: same position, same extent, and the report says so.
    let Anchor::Absolute(absolute) = &anchors[2] else {
        panic!("an absolute anchor")
    };
    assert_eq!(
        absolute.position(&interner),
        Some(Position::from_emu(1_905_000, 952_500))
    );
    assert_eq!(
        absolute.extent(&interner),
        Some(Size::from_emu(685_800, 342_900))
    );
    assert!(!report[2].moved && !report[2].resized);
    assert_eq!(
        report[2].promise,
        ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns
    );
}

#[test]
fn a_row_inserted_inside_a_two_cell_anchor_resizes_it_and_reports_a_promise_it_could_not_keep() {
    let (mut drawing, document) = read(&three_anchor_part());
    let mut interner = document.interner;

    // Row 4 is between the first anchor's `from` (row 2) and its `to` (row 5), and below the
    // second's `from` (row 1) — so the two anchors take different paths through the same call.
    let report = drawing.insert_rows(&mut interner, 4, 1);

    let anchors: Vec<Anchor> = drawing.anchors(&interner).collect();
    let Anchor::TwoCell(two) = &anchors[0] else {
        panic!("a two-cell anchor")
    };
    // `from` did not move; `to` did. The object is one row taller than it was.
    assert_eq!(
        two.from_marker(&interner),
        Some(CellMarker::new(1, 190_500, 2, 47_625))
    );
    assert_eq!(
        two.to_marker(&interner),
        Some(CellMarker::new(3, 95_250, 6, 19_050))
    );
    assert!(!report[0].moved && report[0].resized);
    // …and because this anchor's own `@editAs` says `oneCell` — do not resize — the markers alone
    // could not keep its promise, and it says so rather than being silently left wrong.
    assert_eq!(
        report[0].promise,
        ResizingBehavior::MoveWithCellsButDoNotResize
    );
    assert!(!report[0].promise_kept);

    // The one-cell anchor's `from` is at row 1, above the insertion: nothing moved.
    let Anchor::OneCell(one) = &anchors[1] else {
        panic!("a one-cell anchor")
    };
    assert_eq!(
        one.from_marker(&interner),
        Some(CellMarker::new(4, 76_200, 1, 38_100))
    );
    assert!(!report[1].moved);
}

#[test]
fn removing_the_rows_an_anchor_sits_in_clamps_it_rather_than_panicking() {
    let (mut drawing, document) = read(&three_anchor_part());
    let mut interner = document.interner;

    // Remove rows 2..=5 — exactly the rows the first anchor's two markers name.
    let report = drawing.remove_rows(&mut interner, 2, 4);
    let Some(Anchor::TwoCell(two)) = drawing.anchor(&interner, 0) else {
        panic!("a two-cell anchor")
    };
    assert_eq!(
        two.from_marker(&interner),
        Some(CellMarker::new(1, 190_500, 2, 0)),
        "a marker inside the removed run is clamped to the first surviving row, with no offset"
    );
    assert_eq!(
        two.to_marker(&interner),
        Some(CellMarker::new(3, 95_250, 2, 0))
    );
    assert!(report[0].moved);
}

#[test]
fn a_column_insert_moves_the_column_half_and_leaves_the_row_half_exactly_alone() {
    let (mut drawing, document) = read(&three_anchor_part());
    let mut interner = document.interner;
    drawing.insert_columns(&mut interner, 0, 2);

    let Some(Anchor::TwoCell(two)) = drawing.anchor(&interner, 0) else {
        panic!("a two-cell anchor")
    };
    // Columns 1 and 3 became 3 and 5; the rows 2 and 5 and every offset are untouched. A shift that
    // wrote the column index into the row field passes no part of this.
    assert_eq!(
        two.from_marker(&interner),
        Some(CellMarker::new(3, 190_500, 2, 47_625))
    );
    assert_eq!(
        two.to_marker(&interner),
        Some(CellMarker::new(5, 95_250, 5, 19_050))
    );
}

#[test]
fn an_anchor_naming_a_row_at_the_end_of_the_grid_saturates_rather_than_overflowing() {
    // Untrusted input: a file is free to write a row index this large, and a shift that overflowed
    // would be this library corrupting a part because somebody else wrote an implausible number.
    let part = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}" xmlns:a="{A}">
  <xdr:oneCellAnchor>
    <xdr:from><xdr:col>2147483647</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>2147483647</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:ext cx="1" cy="1"/>
    <xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="s"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr/></xdr:sp>
    <xdr:clientData/>
  </xdr:oneCellAnchor>
</xdr:wsDr>"#
    );
    let (mut drawing, document) = read(&part);
    let mut interner = document.interner;
    drawing.insert_rows(&mut interner, 0, 10);
    let Some(Anchor::OneCell(one)) = drawing.anchor(&interner, 0) else {
        panic!("a one-cell anchor")
    };
    assert_eq!(
        one.from_marker(&interner),
        Some(CellMarker::new(i32::MAX, 0, i32::MAX, 0))
    );
}

#[test]
fn a_marker_missing_a_child_reports_nothing_rather_than_inventing_a_zero() {
    // `CT_Marker`'s four children are all `minOccurs="1"`; a marker missing one names no cell, and
    // answering `0` for the missing half would be placing an object somewhere it was never put.
    let part = format!(
        r#"<xdr:wsDr xmlns:xdr="{XDR}">
  <xdr:oneCellAnchor>
    <xdr:from><xdr:col>3</xdr:col><xdr:row>4</xdr:row><xdr:rowOff>5</xdr:rowOff></xdr:from>
    <xdr:ext cx="1" cy="1"/>
    <xdr:clientData/>
  </xdr:oneCellAnchor>
</xdr:wsDr>"#
    );
    let (drawing, document) = read(&part);
    let Some(Anchor::OneCell(one)) = drawing.anchor(&document.interner, 0) else {
        panic!("a one-cell anchor")
    };
    assert_eq!(one.from_marker(&document.interner), None);
    // …and the anchor itself still round-trips, because nothing was repaired.
    assert_eq!(
        String::from_utf8(fidelity::serialize_to_vec(&document)).expect("utf-8"),
        part
    );
}

#[test]
fn removing_and_pushing_anchors_keeps_the_paint_order_of_the_rest() {
    let (mut drawing, document) = read(&three_anchor_part());
    let mut interner = document.interner;
    assert!(drawing.remove_anchor(&interner, 1));
    assert_eq!(drawing.anchor_count(&interner), 2);
    let locals: Vec<&'static str> = drawing.anchors(&interner).map(|a| a.local()).collect();
    assert_eq!(locals, vec!["twoCellAnchor", "absoluteAnchor"]);

    let object = AnchoredObject::Picture(new_anchored_picture(&mut interner, 42, "new", "rId1"));
    let added = new_one_cell_anchor(
        &mut interner,
        CellMarker::new(0, 0, 0, 0),
        Size::from_emu(1, 1),
        &object,
    );
    drawing.push_anchor(&mut interner, &Anchor::OneCell(added));
    let locals: Vec<&'static str> = drawing.anchors(&interner).map(|a| a.local()).collect();
    assert_eq!(
        locals,
        vec!["twoCellAnchor", "absoluteAnchor", "oneCellAnchor"],
        "a new anchor is appended, because document order is paint order"
    );
    assert!(!drawing.remove_anchor(&interner, 9));
}
