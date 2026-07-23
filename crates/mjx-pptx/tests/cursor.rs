//! Integration tests for [`ShapeCursor`](mjx_pptx::ShapeCursor) — the shape addressed once and
//! edited fluently.
//!
//! The cursor is an ergonomic layer, so the tests that matter most are the ones that pin it *to* the
//! flat API: a chain and the equivalent `set_shape_*` calls must produce the same package, byte for
//! byte. The rest cover what a cursor adds — moving around a group, applying in recorded order,
//! committing once, and changing nothing at all when it is never applied.
//!
//! `tests/fixtures/layouts.pptx` slide 2 carries `Group 3` at top-level index `2`, holding a
//! rectangle at member `0` and an ellipse at member `1` — the same fixture `tests/groups.rs` uses.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth, ParagraphPropertiesSpec,
    TextAlignment,
};
use mjx_opc::Package;
use mjx_pptx::{Hyperlink, PptxError, Presentation, ShapeBounds, ShapeKind, Surface};

const SLIDE2: Surface = Surface::Slide(1);
const SLIDE2_PART: &str = "ppt/slides/slide2.xml";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn layouts() -> Presentation {
    Presentation::open(&fixture("layouts.pptx")).expect("open layouts.pptx")
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn text_of(map: &BTreeMap<String, Vec<u8>>, part: &str) -> String {
    String::from_utf8(map[part].clone()).expect("utf-8")
}

fn navy() -> FillSpec {
    FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned()))
}

fn gold() -> FillSpec {
    FillSpec::Solid(ColorSpec::Srgb("C9A227".to_owned()))
}

fn rule() -> LineSpec {
    LineSpec {
        width: Some(LineWidth::from_points(2.0)),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".to_owned()))),
        ..LineSpec::new()
    }
}

/// A valid 2×2 truecolour PNG, as `tests/pictures.rs` inlines it.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// A second, different valid 2×2 PNG.
const OTHER_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x50, 0x58, 0xE0, 0xF0,
    0xE1, 0x81, 0x00, 0x03, 0x10, 0x03, 0x59, 0x00, 0x29, 0xCE, 0x05, 0xC1, 0x82, 0x11, 0xDA, 0x8B,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

// ---------------------------------------------------------------------------------------------
// The cursor is the flat API said once — not a second implementation of it
// ---------------------------------------------------------------------------------------------

#[test]
fn a_chain_and_the_flat_calls_write_the_same_package() {
    // The load-bearing test of the whole atom: every edit a cursor records is executed by the code
    // the corresponding `set_shape_*` method calls, so the two must be indistinguishable on disk.
    let mut fluent = layouts();
    fluent
        .shape(SLIDE2, 2)
        .expect("cursor")
        .member(0)
        .expect("member 0")
        .fill(navy())
        .outline(rule())
        .text("Q3")
        .sibling(1)
        .expect("member 1")
        .fill(gold())
        .no_outline()
        .apply()
        .expect("apply");

    let mut flat = layouts();
    flat.set_shape_fill(SLIDE2, [2, 0], &navy()).expect("fill");
    flat.set_shape_outline(SLIDE2, [2, 0], &rule())
        .expect("outline");
    flat.set_shape_text_content(SLIDE2, [2, 0], "Q3")
        .expect("text");
    flat.set_shape_fill(SLIDE2, [2, 1], &gold()).expect("fill");
    flat.set_shape_no_outline(SLIDE2, [2, 1]).expect("no line");

    assert_eq!(
        byte_map(&Package::open(&fluent.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&flat.save().expect("save")).expect("reopen")),
        "a cursor chain must write exactly what the flat calls write"
    );
}

#[test]
fn the_fluent_group_example_reads_back_property_for_property() {
    let mut pres = layouts();
    let frame = ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0);
    pres.shape(SLIDE2, 2)
        .expect("cursor")
        .bounds(frame)
        .member(0)
        .expect("member 0")
        .fill(navy())
        .outline(rule())
        .sibling(1)
        .expect("member 1")
        .fill(gold())
        .apply()
        .expect("apply");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_bounds(SLIDE2, 2).expect("group bounds"),
        Some(frame)
    );
    assert_eq!(
        reread.shape_fill(SLIDE2, [2, 0]).expect("fill"),
        Some(navy())
    );
    assert_eq!(
        reread
            .shape_outline(SLIDE2, [2, 0])
            .expect("outline")
            .and_then(|line| line.width),
        Some(LineWidth::from_points(2.0))
    );
    assert_eq!(
        reread.shape_fill(SLIDE2, [2, 1]).expect("fill"),
        Some(gold())
    );
}

// ---------------------------------------------------------------------------------------------
// Moving around a group
// ---------------------------------------------------------------------------------------------

#[test]
fn a_cursor_descends_and_steps_back_out() {
    let mut pres = layouts();
    let mut cursor = pres.shape(SLIDE2, 2).expect("cursor");
    assert_eq!(cursor.kind().expect("kind"), ShapeKind::GroupShape);
    assert_eq!(cursor.member_count().expect("members"), 2);

    let mut cursor = cursor.member(1).expect("member 1");
    assert_eq!(cursor.path().indices(), [2, 1]);
    assert_eq!(cursor.kind().expect("kind"), ShapeKind::Shape);
    // A leaf shape has no members, which is what makes `member` refuse to descend into one.
    assert_eq!(cursor.member_count().expect("members"), 0);

    let cursor = cursor.parent().expect("parent");
    assert_eq!(cursor.path().indices(), [2]);
    // Back out at the top level, `sibling` addresses another top-level shape.
    let mut cursor = cursor.sibling(3).expect("sibling");
    assert_eq!(cursor.kind().expect("kind"), ShapeKind::GraphicFrame);
}

#[test]
fn only_a_group_can_be_descended_into() {
    let mut pres = layouts();
    let err = pres
        .shape(SLIDE2, 0)
        .expect("cursor")
        .member(0)
        .expect_err("the title is not a group");
    match err {
        PptxError::ShapeIsNotAGroup { surface, path } => {
            assert_eq!(surface, SLIDE2);
            assert_eq!(path.indices(), [0]);
        }
        other => panic!("expected ShapeIsNotAGroup, got {other:?}"),
    }
}

#[test]
fn a_member_past_the_end_is_out_of_range() {
    let mut pres = layouts();
    let err = pres
        .shape(SLIDE2, 2)
        .expect("cursor")
        .member(9)
        .expect_err("the group has two members");
    match err {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path,
            count,
        } => {
            assert_eq!(surface, SLIDE2);
            assert_eq!(path.indices(), [2, 9]);
            assert_eq!(count, 2);
        }
        other => panic!("expected ShapeIndexOutOfRange, got {other:?}"),
    }
}

#[test]
fn a_top_level_shape_has_no_parent_to_step_out_to() {
    // The shape tree is not itself a shape, so there is nothing above a top-level address.
    let mut pres = layouts();
    let err = pres
        .shape(SLIDE2, 2)
        .expect("cursor")
        .parent()
        .expect_err("top level");
    match err {
        PptxError::ShapeHasNoParent { surface, path } => {
            assert_eq!(surface, SLIDE2);
            assert_eq!(path.indices(), [2]);
        }
        other => panic!("expected ShapeHasNoParent, got {other:?}"),
    }
}

#[test]
fn a_cursor_is_never_opened_on_a_shape_that_is_not_there() {
    let mut pres = layouts();
    assert!(matches!(
        pres.shape(SLIDE2, 99).map(|_| ()),
        Err(PptxError::ShapeIndexOutOfRange { .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// Nothing happens until `apply`
// ---------------------------------------------------------------------------------------------

#[test]
fn a_cursor_that_is_never_applied_changes_nothing() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let cursor = pres
        .shape(SLIDE2, 2)
        .expect("cursor")
        .member(0)
        .expect("member")
        .fill(navy())
        .text("dropped on the floor");
    drop(cursor);

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot,
        "recording edits must not touch the package"
    );
}

#[test]
fn navigating_alone_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let mut cursor = pres.shape(SLIDE2, 2).expect("cursor");
    let _ = cursor.member_count().expect("members");
    let mut cursor = cursor.member(0).expect("member");
    let _ = cursor.kind().expect("kind");
    cursor.apply().expect("apply nothing");

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot,
        "reading and moving must not dirty the part"
    );
}

#[test]
fn an_edit_that_cannot_be_made_stops_the_pass_where_it_is() {
    // A cursor is the flat calls said once, failure semantics included: an edit that a shape cannot
    // take stops the pass, and what was recorded before it is already written — exactly where a
    // column of `set_shape_*` calls would have stopped.
    let mut pres = layouts();
    let err = pres
        .shape(SLIDE2, 0)
        .expect("cursor")
        .fill(navy())
        .sibling(3)
        .expect("the table")
        .fill(gold())
        .apply()
        .expect_err("a graphic frame has no p:spPr to fill");
    assert!(matches!(err, PptxError::ShapeHasNoProperties));

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_fill(SLIDE2, 0).expect("fill"),
        Some(navy()),
        "the edit recorded before the failure is written"
    );
}

// ---------------------------------------------------------------------------------------------
// Order, and the single commit
// ---------------------------------------------------------------------------------------------

#[test]
fn edits_apply_in_the_order_they_were_recorded() {
    // `text` replaces the paragraphs; the formatting recorded after it must land on the new run,
    // which only holds if the passes run in order and the second sees the rebuilt body.
    let mut pres = layouts();
    pres.shape(SLIDE2, 0)
        .expect("cursor")
        .text("Q3 results")
        .all_run_properties(CharacterPropertiesSpec::new().with_bold(true))
        .paragraph_properties(
            0,
            ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Center),
        )
        .apply()
        .expect("apply");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(reread.shape_text(SLIDE2, 0).expect("text"), "Q3 results");
    assert_eq!(
        reread
            .run_properties(SLIDE2, 0, 0, 0)
            .expect("run props")
            .and_then(|spec| spec.is_bold()),
        Some(true)
    );
    assert_eq!(
        reread
            .paragraph_properties(SLIDE2, 0, 0)
            .expect("para props")
            .and_then(|spec| spec.alignment()),
        Some(TextAlignment::Center)
    );
}

#[test]
fn a_chain_dirties_only_the_part_it_edits() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.shape(SLIDE2, 2)
        .expect("cursor")
        .member(0)
        .expect("member 0")
        .fill(navy())
        .sibling(1)
        .expect("member 1")
        .fill(gold())
        .apply()
        .expect("apply");
    let saved = pres.save().expect("save");
    let after = byte_map(&Package::open(&saved).expect("reopen"));

    assert_eq!(
        snapshot.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "no part added or removed"
    );
    for (name, original) in &snapshot {
        if name == SLIDE2_PART {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(original),
            "a cursor dirtied unrelated part {name}"
        );
    }
    assert_ne!(after.get(SLIDE2_PART), snapshot.get(SLIDE2_PART));
}

// ---------------------------------------------------------------------------------------------
// Edits that reach the package
// ---------------------------------------------------------------------------------------------

#[test]
fn a_link_replaced_within_one_chain_leaves_exactly_one_relationship() {
    // Both links are added before the writing pass, so the first is unreferenced by the end of it —
    // and the sweep, which checks every id the pass touched, must take it away again.
    let mut pres = Presentation::open(&fixture("hyperlinks.pptx")).expect("open");
    pres.shape(1, 0)
        .expect("cursor")
        .hyperlink(Hyperlink::Url("https://first.example/".to_owned()))
        .hyperlink(Hyperlink::Url("https://second.example/".to_owned()))
        .apply()
        .expect("apply");

    let saved = pres.save().expect("save");
    let rels = text_of(
        &byte_map(&Package::open(&saved).expect("reopen")),
        "ppt/slides/_rels/slide2.xml.rels",
    );
    assert!(rels.contains("https://second.example/"), "{rels}");
    assert!(
        !rels.contains("https://first.example/"),
        "the superseded relationship must be swept: {rels}"
    );

    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reread.shape_hyperlink(1, 0).expect("link"),
        Some(Hyperlink::Url("https://second.example/".to_owned()))
    );
}

#[test]
fn clearing_a_link_through_a_cursor_removes_its_relationship() {
    // The fixture's shape 1 on slide 1 carries a slide-jump link.
    let mut pres = Presentation::open(&fixture("hyperlinks.pptx")).expect("open");
    assert_eq!(
        pres.shape_hyperlink(0, 1).expect("link"),
        Some(Hyperlink::Slide(1))
    );
    pres.shape(0, 1)
        .expect("cursor")
        .clear_hyperlink()
        .apply()
        .expect("apply");

    let saved = pres.save().expect("save");
    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(reread.shape_hyperlink(0, 1).expect("link"), None);
}

#[test]
fn a_pictures_image_can_be_replaced_through_a_cursor() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0))
        .expect("add picture");

    pres.shape(0, picture)
        .expect("cursor")
        .image(OTHER_PNG)
        .outline(rule())
        .apply()
        .expect("apply");

    let saved = pres.save().expect("save");
    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reread.picture_image_bytes(0, picture).expect("bytes"),
        Some(OTHER_PNG)
    );
    // The `p:spPr` edit recorded alongside it landed on the same shape, in the same pass.
    assert!(reread.shape_outline(0, picture).expect("outline").is_some());
}

#[test]
fn an_image_edit_on_something_that_is_not_a_picture_adds_no_media_part() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let err = pres
        .shape(SLIDE2, 0)
        .expect("cursor")
        .image(TINY_PNG)
        .apply()
        .expect_err("the title is not a picture");
    assert!(matches!(err, PptxError::ShapeIsNotAPicture));

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot,
        "a refused image edit must leave no media part behind"
    );
}
