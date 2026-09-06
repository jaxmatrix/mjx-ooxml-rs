//! **MJXOFF-107's package gate.** The six things a worksheet drawing is, and what only a package can
//! be asked about them.
//!
//! `crates/mjx-dml/tests/spreadsheet_drawing_model.rs` pins the markup — the three anchors, the
//! `@editAs` attribute, the shift each mode promises — and
//! `crates/mjx-sml/tests/anchor_geometry.rs` pins the conversion to EMU. This file pins **which
//! part**, **which relationship**, **which content type**, and whether the six of them still agree
//! after this library has written one.
//!
//! # The fixture, and what this project did not write
//!
//! `tests/fixtures/worksheet_drawings.xlsx` was produced by **Apache POI 5.5.1**, not by this
//! project. Its two-cell anchor comes from `XSSFDrawing.createPicture`; its one-cell and absolute
//! anchors come from the `CTDrawing` XMLBeans API POI exposes, because POI's own high-level API only
//! ever writes `twoCellAnchor`. Four choices in it are load-bearing here:
//!
//! * its `twoCellAnchor` carries **`editAs="oneCell"`** — an attribute value that disagrees with the
//!   element's own name, so a reader that inferred the behaviour from the element fails;
//! * its columns are **3.5, 20.75 and 12** characters wide and its row 3 is **42** points tall, so
//!   no answer here is the default by accident, and the sheet states **no `defaultColWidth`** at
//!   all, so column D exercises the `baseColWidth` fallback;
//! * its two-cell anchor's picture starts **mid-cell** (`colOff="190500"`, `rowOff="47625"`);
//! * it anchors a **PNG** on two anchors and a **JPEG** on the third, so the media path is not
//!   proved by one image format and assumed for the rest.
//!
//! And the number this file leans on hardest is POI's, not ours: POI wrote
//! `a:ext cx="2085975" cy="885825"` into that anchor's own `xdr:spPr` from the same column widths,
//! through its own implementation of §18.3.1.13. [`the_resolved_bounds_agree_with_the_extent_apache_poi_computed`]
//! asserts our resolver reaches the same rectangle.
//!
//! # What each mutation would break
//!
//! * Relating the image from the **sheet** instead of from the drawing part turns
//!   [`adding_a_picture_writes_a_part_a_content_type_two_relationships_and_a_drawing_entry`] red,
//!   because the `a:blip@r:embed` would then resolve against a `.rels` that does not declare it.
//! * Reusing an existing `cNvPr@id` turns [`a_second_picture_gets_a_second_id`] red.
//! * Touching any other part while adding a picture turns [`adding_a_picture_leaves_every_other_part_byte_identical`] red.

use mjx_dml::spreadsheet_drawing::CellMarker;
use mjx_dml::{Position, Size};
use mjx_fixtures::fixture;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_opc::{Package, PartName};
use mjx_sml::{CellReference, CellValue, ColumnMetrics, GeometrySource};
use mjx_xlsx::Workbook;

/// A 1×1 red PNG — the smallest thing `ImageFormat::sniff` calls a PNG.
const PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R',
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N',
    b'D', 0xAE, 0x42, 0x60, 0x82,
];

/// A different image, in a different format, so a deduplication test cannot pass by holding one.
const JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01,
];

fn producer_workbook() -> Workbook {
    Workbook::open(&fixture("worksheet_drawings.xlsx")).expect("the fixture opens")
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("a valid part name")
}

// -------------------------------------------------------------------------------------------
// Reading a producer's drawing
// -------------------------------------------------------------------------------------------

#[test]
fn the_producer_fixture_reports_four_anchored_objects_in_paint_order() {
    let workbook = producer_workbook();
    let drawing = workbook
        .sheet_drawing(0)
        .expect("it reads")
        .expect("sheet 1 has a drawing");

    assert_eq!(drawing.part.as_str(), "/xl/drawings/drawing1.xml");
    assert_eq!(drawing.relationship_id, "rId1");
    assert_eq!(drawing.objects.len(), 4);

    let shapes: Vec<(&str, Option<&str>)> = drawing
        .objects
        .iter()
        .map(|object| (object.anchor, object.object))
        .collect();
    assert_eq!(
        shapes,
        vec![
            ("twoCellAnchor", Some("pic")),
            ("twoCellAnchor", Some("sp")),
            ("oneCellAnchor", Some("pic")),
            ("absoluteAnchor", Some("pic")),
        ],
        "the four objects, in the order the file lists them — which is paint order"
    );

    // The first anchor's `@editAs` says `oneCell` on a `twoCellAnchor`. The attribute wins.
    assert_eq!(
        drawing.objects[0].resizing,
        ResizingBehavior::MoveWithCellsButDoNotResize
    );
    // …and the second `twoCellAnchor` beside it says `twoCell`, so the answer above is not a
    // constant this reader returns for every two-cell anchor.
    assert_eq!(
        drawing.objects[1].resizing,
        ResizingBehavior::MoveAndResizeWithAnchorCells
    );
    assert_eq!(
        drawing.objects[3].resizing,
        ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns
    );

    // Two anchors show the PNG and one shows the JPEG, resolved through the **drawing part's** own
    // `.rels` rather than the sheet's.
    assert_eq!(
        drawing.objects[0].image.as_ref().map(PartName::as_str),
        Some("/xl/media/image1.png")
    );
    assert_eq!(drawing.objects[1].image, None, "a shape shows no image");
    assert_eq!(
        drawing.objects[2].image.as_ref().map(PartName::as_str),
        Some("/xl/media/image1.png")
    );
    assert_eq!(
        drawing.objects[3].image.as_ref().map(PartName::as_str),
        Some("/xl/media/image2.jpeg"),
        "the absolute anchor shows the other format"
    );

    // Every `xdr:clientData` in the fixture is written `<xdr:clientData/>`, and both of its
    // attributes default to **true**.
    assert!(drawing
        .objects
        .iter()
        .all(|object| object.prints_with_sheet));
}

#[test]
fn a_sheet_with_no_drawing_answers_none_rather_than_an_empty_drawing() {
    let workbook = Workbook::open(&fixture("sample.xlsx")).expect("open");
    assert_eq!(workbook.sheet_drawing(0).expect("it reads"), None);
    assert!(
        workbook.sheet_drawing(9).is_err(),
        "a tab that is not there"
    );
}

#[test]
fn the_resolved_bounds_agree_with_the_extent_apache_poi_computed() {
    let workbook = producer_workbook();
    let bounds = workbook
        .sheet_anchor_bounds(0, 0, ColumnMetrics::CALIBRI_11_AT_96_DPI)
        .expect("it reads")
        .expect("the sheet places it");

    // POI wrote these two numbers into the fixture's own `xdr:pic/xdr:spPr/a:xfrm/a:ext`, computed
    // from the same column widths through its own implementation of §18.3.1.13.
    assert_eq!(bounds.size.width.emu(), 2_085_975);
    assert_eq!(bounds.size.height.emu(), 885_825);

    // The honest half: the rows fell back to `defaultRowHeight` while every column crossed states
    // its own width, so the two axes report different sources.
    assert_eq!(bounds.row_source, GeometrySource::SheetDefault);
    assert_eq!(bounds.column_source, GeometrySource::Stated);
    assert!(!bounds.is_fully_stated());
    assert_eq!(bounds.column_metrics, ColumnMetrics::CALIBRI_11_AT_96_DPI);

    // The absolute anchor needed no sheet at all, and says so.
    let absolute = workbook
        .sheet_anchor_bounds(0, 3, ColumnMetrics::CALIBRI_11_AT_96_DPI)
        .expect("it reads")
        .expect("placed");
    assert_eq!(absolute.position, Position::from_emu(1_905_000, 952_500));
    assert!(absolute.is_fully_stated());
}

#[test]
fn a_producer_authored_workbook_with_drawings_round_trips_byte_for_byte() {
    // Tier 1: every decompressed part comes back exactly as POI wrote it, drawing part and both
    // media parts included.
    let bytes = fixture("worksheet_drawings.xlsx");
    let package = Package::open(&bytes).expect("open");
    let saved = Workbook::open(&bytes).expect("open").save().expect("save");
    let reopened = Package::open(&saved).expect("reopen");

    for name in package.part_names() {
        assert_eq!(
            package.part_bytes(&name),
            reopened.part_bytes(&name),
            "{} changed on the way through",
            name.as_str()
        );
    }
    assert_eq!(
        package.part_names().count(),
        reopened.part_names().count(),
        "no part was added or lost"
    );
}

// -------------------------------------------------------------------------------------------
// Authoring
// -------------------------------------------------------------------------------------------

#[test]
fn adding_a_picture_writes_a_part_a_content_type_two_relationships_and_a_drawing_entry() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let at = workbook
        .add_two_cell_anchored_picture(
            0,
            PNG,
            "logo",
            CellMarker::new(1, 190_500, 2, 47_625),
            CellMarker::new(3, 95_250, 5, 19_050),
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("the picture is added");
    assert_eq!(at, 0);

    // 1. the drawing part, and 2. its content type.
    let drawing_part = part("/xl/drawings/drawing1.xml");
    assert!(workbook.package().part_bytes(&drawing_part).is_some());
    assert_eq!(
        workbook.package().content_type_of(&drawing_part),
        Some("application/vnd.openxmlformats-officedocument.drawing+xml")
    );

    // 3. the image part, registered through a content-type `Default` on its extension.
    let media_part = part("/xl/media/image1.png");
    assert_eq!(workbook.package().part_bytes(&media_part), Some(PNG));
    assert_eq!(
        workbook.package().content_type_of(&media_part),
        Some("image/png")
    );

    // 4. the sheet → drawing relationship.
    let sheet_part = workbook.sheets()[0].part.clone().expect("a sheet part");
    let sheet_rels = workbook
        .package()
        .relationships_for(Some(&sheet_part))
        .expect("the sheet has relationships");
    assert_eq!(
        sheet_rels
            .by_type(mjx_xlsx::parts::REL_DRAWING)
            .map(|rel| rel.target.as_str())
            .collect::<Vec<_>>(),
        vec!["../drawings/drawing1.xml"]
    );

    // 5. the **drawing** → image relationship. Relating the image from the sheet would leave the
    //    `a:blip@r:embed` naming nothing, which is the near-miss this assertion exists for.
    let drawing_rels = workbook
        .package()
        .relationships_for(Some(&drawing_part))
        .expect("the drawing has relationships");
    assert_eq!(
        drawing_rels
            .by_type(mjx_xlsx::parts::REL_IMAGE)
            .map(|rel| rel.target.as_str())
            .collect::<Vec<_>>(),
        vec!["../media/image1.png"]
    );
    assert_eq!(
        sheet_rels.by_type(mjx_xlsx::parts::REL_IMAGE).count(),
        0,
        "the image is the drawing's, not the sheet's"
    );

    // 6. the `x:drawing` entry, at rank 29 of `CT_Worksheet`.
    let markup = workbook
        .worksheet_markup(0)
        .expect("it reads")
        .expect("a worksheet");
    assert!(markup.drawing().is_some());
    let locals: Vec<&str> = markup.child_element_locals().collect();
    let drawing_at = locals.iter().position(|local| *local == "drawing");
    assert!(
        drawing_at.is_some(),
        "the sheet claims the part: {locals:?}"
    );
    assert!(
        locals
            .iter()
            .position(|local| *local == "sheetData")
            .unwrap_or(usize::MAX)
            < drawing_at.expect("a drawing"),
        "rank 5 must come before rank 29: {locals:?}"
    );

    // …and the six of them agree well enough that the package validator accepts it.
    workbook.save().expect("the workbook validates and saves");
}

#[test]
fn the_three_anchor_modes_are_authored_as_three_different_elements() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .add_two_cell_anchored_picture(
            0,
            PNG,
            "two-cell",
            CellMarker::new(0, 0, 0, 0),
            CellMarker::new(2, 0, 2, 0),
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .expect("added");
    workbook
        .add_one_cell_anchored_picture(
            0,
            PNG,
            "one-cell",
            CellMarker::new(4, 76_200, 1, 38_100),
            Size::from_emu(914_400, 457_200),
        )
        .expect("added");
    workbook
        .add_absolute_anchored_picture(
            0,
            JPEG,
            "absolute",
            Position::from_emu(1_905_000, 952_500),
            Size::from_emu(685_800, 342_900),
        )
        .expect("added");

    let drawing = workbook
        .sheet_drawing(0)
        .expect("it reads")
        .expect("a drawing");
    assert_eq!(
        drawing
            .objects
            .iter()
            .map(|object| object.anchor)
            .collect::<Vec<_>>(),
        vec!["twoCellAnchor", "oneCellAnchor", "absoluteAnchor"]
    );
    assert_eq!(
        drawing
            .objects
            .iter()
            .map(|object| object.name.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("two-cell"), Some("one-cell"), Some("absolute")]
    );
    // Two formats, two media parts, and the JPEG on the anchor that asked for it.
    assert_eq!(
        drawing.objects[2].image.as_ref().map(PartName::as_str),
        Some("/xl/media/image2.jpeg")
    );

    // The one-cell anchor kept the extent it was given, which a two-cell anchor cannot state — read
    // back off the markup, because a blank workbook's sheet states no row height at all (see below).
    let extent = workbook
        .drawing_markup(0, |drawing, interner| match drawing.anchor(interner, 1) {
            Some(mjx_dml::Anchor::OneCell(one)) => one.extent(interner),
            _ => None,
        })
        .expect("it reads")
        .expect("a drawing");
    assert_eq!(extent, Some(Size::from_emu(914_400, 457_200)));

    // …and `Workbook::blank` writes no `x:sheetFormatPr`, so `@defaultRowHeight` — which is
    // `use="required"` — is stated nowhere. Every anchor on it is therefore **unplaceable**, and the
    // honest answer is `None` rather than Excel's 15 points, which this library was never told.
    assert_eq!(
        workbook
            .sheet_anchor_bounds(0, 1, ColumnMetrics::CALIBRI_11_AT_96_DPI)
            .expect("it reads"),
        None
    );
    // The absolute anchor names no cell, so it is placeable on the same sheet — which is what makes
    // the `None` above a statement about the rows rather than about the workbook.
    assert!(workbook
        .sheet_anchor_bounds(0, 2, ColumnMetrics::CALIBRI_11_AT_96_DPI)
        .expect("it reads")
        .is_some());

    workbook.save().expect("the workbook validates and saves");
}

#[test]
fn a_second_picture_gets_a_second_id() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .add_one_cell_anchored_picture(
            0,
            PNG,
            "first",
            CellMarker::new(0, 0, 0, 0),
            Size::from_emu(1, 1),
        )
        .expect("added");
    workbook
        .add_one_cell_anchored_picture(
            0,
            JPEG,
            "second",
            CellMarker::new(1, 0, 1, 0),
            Size::from_emu(1, 1),
        )
        .expect("added");

    let drawing = workbook
        .sheet_drawing(0)
        .expect("it reads")
        .expect("a drawing");
    let ids: Vec<Option<u32>> = drawing.objects.iter().map(|object| object.id).collect();
    assert_eq!(
        ids,
        vec![Some(1), Some(2)],
        "one past the highest, never reused"
    );
}

#[test]
fn the_same_image_added_twice_is_stored_once_and_related_once() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    for name in ["first", "second"] {
        workbook
            .add_one_cell_anchored_picture(
                0,
                PNG,
                name,
                CellMarker::new(0, 0, 0, 0),
                Size::from_emu(1, 1),
            )
            .expect("added");
    }
    let media: Vec<String> = workbook
        .package()
        .part_names()
        .filter(|part| part.as_str().starts_with("/xl/media/"))
        .map(|part| part.as_str().to_owned())
        .collect();
    assert_eq!(media, vec!["/xl/media/image1.png".to_owned()]);

    let drawing_part = part("/xl/drawings/drawing1.xml");
    assert_eq!(
        workbook
            .package()
            .relationships_for(Some(&drawing_part))
            .expect("relationships")
            .by_type(mjx_xlsx::parts::REL_IMAGE)
            .count(),
        1,
        "the second picture reuses the relationship the first made"
    );

    // …and a *different* image does get a part of its own, so the count above is not a ceiling.
    workbook
        .add_one_cell_anchored_picture(
            0,
            JPEG,
            "third",
            CellMarker::new(0, 0, 0, 0),
            Size::from_emu(1, 1),
        )
        .expect("added");
    assert_eq!(
        workbook
            .package()
            .part_names()
            .filter(|part| part.as_str().starts_with("/xl/media/"))
            .count(),
        2
    );
}

#[test]
fn adding_a_picture_leaves_every_other_part_byte_identical() {
    // Tier 3: edit isolation. Adding a drawing to the fixture's only sheet touches the worksheet
    // (which now claims a part), the drawing part, the media part, the content types and two `.rels`
    // — and nothing else.
    let bytes = fixture("sheet_grid.xlsx");
    let before = Package::open(&bytes).expect("open");
    let mut workbook = Workbook::open(&bytes).expect("open");
    workbook
        .add_absolute_anchored_picture(
            0,
            PNG,
            "logo",
            Position::from_emu(0, 0),
            Size::from_emu(914_400, 914_400),
        )
        .expect("added");
    let saved = workbook.save().expect("save");
    let after = Package::open(&saved).expect("reopen");

    let sheet_part = workbook.sheets()[0].part.clone().expect("a sheet part");
    let mut changed = Vec::new();
    for name in before.part_names() {
        if before.part_bytes(&name) != after.part_bytes(&name) {
            changed.push(name.as_str().to_owned());
        }
    }
    changed.sort();
    assert_eq!(
        changed,
        vec![sheet_part.as_str().to_owned()],
        "only the sheet that gained the drawing may change — every other part of the fixture, \
         including its styles and its cells, comes back byte for byte"
    );
    // The two parts the edit *adds* are new, so they are not in the loop above; assert they arrived.
    assert!(after
        .part_bytes(&part("/xl/drawings/drawing1.xml"))
        .is_some());
    assert!(after.part_bytes(&part("/xl/media/image1.png")).is_some());
}

// -------------------------------------------------------------------------------------------
// The three modes under a sheet edit, end to end
// -------------------------------------------------------------------------------------------

#[test]
fn inserting_rows_moves_each_anchor_the_way_its_own_mode_promises() {
    let mut workbook = producer_workbook();
    let before = markers(&workbook);

    // Row 0 is above every anchor in the fixture.
    let report = workbook
        .insert_rows_into_drawing(0, 0, 3)
        .expect("the drawing is edited");
    assert_eq!(report.len(), 4);
    let after = markers(&workbook);

    // 1. Both `twoCellAnchor`s moved **and kept their span**: `from` and `to` each moved by three.
    for index in 0..2 {
        let (from_before, to_before) = before[index];
        let (from_after, to_after) = after[index];
        assert_eq!(
            (from_after, to_after),
            (from_before.map(|row| row + 3), to_before.map(|row| row + 3)),
            "anchor {index} is a two-cell anchor: both of its markers move"
        );
        assert!(report[index].moved && !report[index].resized);
    }

    // 2. The `oneCellAnchor` moved its one marker, and its `xdr:ext` — which a two-cell anchor does
    //    not have at all — is untouched. A different outcome from the same call.
    assert_eq!(after[2].0, before[2].0.map(|row| row + 3));
    assert_eq!(after[2].1, None, "a one-cell anchor has no `to` marker");
    assert!(report[2].moved && !report[2].resized);
    let extent = workbook
        .drawing_markup(0, |drawing, interner| match drawing.anchor(interner, 2) {
            Some(mjx_dml::Anchor::OneCell(one)) => one.extent(interner),
            _ => None,
        })
        .expect("it reads")
        .expect("a drawing");
    assert_eq!(extent, Some(Size::from_emu(914_400, 457_200)));

    // 3. The `absoluteAnchor` did **neither**: it names no cell to move, and its own rectangle is
    //    exactly what it was — which is the third outcome, and the only one with no marker at all.
    assert_eq!(after[3], (None, None));
    assert!(!report[3].moved && !report[3].resized);
    assert_eq!(
        report[3].promise,
        ResizingBehavior::DoNotMoveOrResizeWithRowsOrColumns
    );
    let absolute = workbook
        .sheet_anchor_bounds(0, 3, ColumnMetrics::CALIBRI_11_AT_96_DPI)
        .expect("it reads")
        .expect("placed");
    assert_eq!(absolute.position, Position::from_emu(1_905_000, 952_500));
    assert_eq!(absolute.size, Size::from_emu(685_800, 342_900));

    workbook
        .save()
        .expect("the edited workbook still validates");
}

#[test]
fn a_row_inserted_inside_a_two_cell_anchor_reports_a_promise_the_markers_could_not_keep() {
    let mut workbook = producer_workbook();
    // Row 4 sits between the first anchor's `from` (row 2) and its `to` (row 5), and that anchor's
    // own `@editAs` says `oneCell` — do not resize.
    let report = workbook
        .insert_rows_into_drawing(0, 4, 1)
        .expect("the drawing is edited");
    assert!(report[0].resized);
    assert!(
        !report[0].promise_kept,
        "a two-cell anchor resized against its own `@editAs` must say so"
    );
    // …and the second `twoCellAnchor`, whose `@editAs` is the default, keeps its promise even though
    // the same call reached it — so `promise_kept` is not a constant.
    assert!(report[1].promise_kept);

    workbook
        .save()
        .expect("the edited workbook still validates");
}

#[test]
fn an_edit_that_moves_nothing_leaves_the_drawing_part_byte_identical() {
    // The write-back goes through `ToXml::write_back` and the document the part was parsed from,
    // not through a fresh document around a rebuilt root. So a shift that reaches every anchor and
    // moves none of them re-emits the part exactly — **including its prologue**, which is POI's
    // `<?xml version="1.0" encoding="UTF-8"?>` and not the `standalone="yes"` this project writes
    // for parts of its own.
    let bytes = fixture("worksheet_drawings.xlsx");
    let before = Package::open(&bytes).expect("open");
    let drawing_part = part("/xl/drawings/drawing1.xml");
    let original = before
        .part_bytes(&drawing_part)
        .expect("the fixture has a drawing")
        .to_vec();
    assert!(
        original.starts_with(br#"<?xml version="1.0" encoding="UTF-8"?>"#),
        "the fixture's own declaration is what this case is about"
    );

    let mut workbook = producer_workbook();
    // Row 999,999 is below every anchor, so every one of the four is visited and none moves.
    let report = workbook
        .insert_rows_into_drawing(0, 999_999, 1)
        .expect("the drawing is edited");
    assert_eq!(report.len(), 4);
    assert!(report.iter().all(|shift| !shift.moved && !shift.resized));

    let saved = workbook.save().expect("save");
    let after = Package::open(&saved).expect("reopen");
    assert_eq!(
        after.part_bytes(&drawing_part),
        Some(original.as_slice()),
        "an edit that changed nothing must not re-flow the part or rewrite its declaration"
    );
}

#[test]
fn an_edit_that_moves_one_anchor_leaves_the_others_bytes_alone() {
    let mut workbook = producer_workbook();
    let drawing_part = part("/xl/drawings/drawing1.xml");

    // Row 3 is at or below the first anchor's `from` (row 2) and its `to` (row 5), and below the
    // one-cell anchor's `from` (row 1) — so exactly one of the four anchors is left untouched by
    // its own markers, and the absolute anchor by having none.
    workbook
        .insert_rows_into_drawing(0, 3, 1)
        .expect("the drawing is edited");
    let saved = workbook.save().expect("save");
    let after = Package::open(&saved).expect("reopen");
    let payload = after
        .part_bytes(&drawing_part)
        .expect("the drawing survives")
        .to_vec();
    let text = String::from_utf8(payload).expect("utf-8");

    // The prologue is still the file's own.
    assert!(text.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    // The absolute anchor names no cell, so its whole subtree comes back verbatim — the exact
    // spelling POI wrote, whitespace and self-closing tags included.
    assert!(
        text.contains(r#"<xdr:pos x="1905000" y="952500"/>"#)
            && text.contains(r#"<xdr:ext cx="685800" cy="342900"/>"#),
        "the absolute anchor must be untouched:\n{text}"
    );
    // …and the anchor that did move says so.
    assert!(
        text.contains("<xdr:row>6</xdr:row>"),
        "the `to` marker moved"
    );
}

#[test]
fn editing_a_cell_never_opens_the_drawing_part() {
    let bytes = fixture("worksheet_drawings.xlsx");
    let before = Package::open(&bytes).expect("open");
    let mut workbook = Workbook::open(&bytes).expect("open");
    workbook
        .set_cell_value(
            0,
            CellReference::parse("A2").expect("A2"),
            CellValue::Number(7.5),
        )
        .expect("the cell is set");
    let saved = workbook.save().expect("save");
    let after = Package::open(&saved).expect("reopen");

    for name in [
        "/xl/drawings/drawing1.xml",
        "/xl/media/image1.png",
        "/xl/media/image2.jpeg",
    ] {
        let name = part(name);
        assert_eq!(
            before.part_bytes(&name),
            after.part_bytes(&name),
            "{} must be untouched by a cell edit",
            name.as_str()
        );
    }
}

#[test]
fn removing_an_anchor_leaves_its_image_part_where_it_is() {
    let mut workbook = producer_workbook();
    assert!(workbook
        .remove_sheet_drawing_object(0, 3)
        .expect("the drawing is edited"));
    let drawing = workbook
        .sheet_drawing(0)
        .expect("it reads")
        .expect("a drawing");
    assert_eq!(drawing.objects.len(), 3);

    // The JPEG is now shown by nothing, and is still in the package: another sheet's drawing may
    // name it, and sweeping it is `Package::remove_unreferenced_parts`'s decision, not this call's.
    assert!(workbook
        .package()
        .part_bytes(&part("/xl/media/image2.jpeg"))
        .is_some());
    assert!(!workbook
        .remove_sheet_drawing_object(0, 9)
        .expect("no such anchor"));
}

#[test]
fn the_markup_door_hands_over_the_interner_the_anchors_names_live_in() {
    // An anchor read out of the drawing part cannot be read through the worksheet's interner, which
    // is a different one — so the closure is handed both together.
    let workbook = producer_workbook();
    let locals = workbook
        .drawing_markup(0, |drawing, interner| {
            drawing
                .anchors(interner)
                .map(|anchor| anchor.local())
                .collect::<Vec<_>>()
        })
        .expect("it reads")
        .expect("a drawing");
    assert_eq!(
        locals,
        vec![
            "twoCellAnchor",
            "twoCellAnchor",
            "oneCellAnchor",
            "absoluteAnchor"
        ]
    );
}

#[test]
fn bytes_that_are_not_an_image_are_refused_before_anything_is_written() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let before = workbook.save().expect("save");
    let error = workbook
        .add_one_cell_anchored_picture(
            0,
            b"not an image at all",
            "nope",
            CellMarker::new(0, 0, 0, 0),
            Size::from_emu(1, 1),
        )
        .expect_err("the bytes match no image format");
    assert!(matches!(
        error,
        mjx_xlsx::XlsxError::UnrecognizedImageFormat
    ));
    assert_eq!(
        workbook.save().expect("save"),
        before,
        "a refusal must leave the workbook exactly as it was"
    );
}

/// The `from` and `to` marker rows of each anchor of the tab at 0.
///
/// Markers rather than resolved EMU, because `insert_rows_into_drawing` moves the **drawing** and
/// not the sheet's own rows: the fixture's 42-point row 3 stays where it is, so an object that
/// crosses it after the shift is genuinely taller in EMU than it was, and asserting on EMU would be
/// asserting the wrong fact.
fn markers(workbook: &Workbook) -> Vec<(Option<i32>, Option<i32>)> {
    workbook
        .drawing_markup(0, |drawing, interner| {
            drawing
                .anchors(interner)
                .map(|anchor| match anchor {
                    mjx_dml::Anchor::TwoCell(two) => (
                        two.from_marker(interner).map(|marker| marker.row),
                        two.to_marker(interner).map(|marker| marker.row),
                    ),
                    mjx_dml::Anchor::OneCell(one) => {
                        (one.from_marker(interner).map(|marker| marker.row), None)
                    }
                    mjx_dml::Anchor::Absolute(_) => (None, None),
                })
                .collect()
        })
        .expect("it reads")
        .expect("a drawing")
}
