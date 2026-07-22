//! Integration tests for **group descent** (`ShapePath`): a shape inside a `p:grpSp` is addressed by
//! a path `[group, member, …]` and is then read, edited and removed exactly like a top-level shape,
//! with the same fidelity guarantees.
//!
//! `tests/fixtures/layouts.pptx` slide 2 carries `Group 3` at top-level index `2`, holding two member
//! autoshapes: a rectangle at member `0` and an ellipse at member `1`. Their child-space transforms
//! (`a:off` inside the group's `a:chOff`/`a:chExt` space) are what the reads below check; mapping that
//! to an absolute slide rectangle is a later atom (G3), so these only assert the explicit values.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSpec, FillSpec};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

const SLIDE2: Surface = Surface::Slide(1);

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

/// Asserts every part except those named survived a save byte-for-byte.
#[track_caller]
fn only_these_parts_changed(before: &BTreeMap<String, Vec<u8>>, saved: &[u8], edited: &[&str]) {
    let after = byte_map(&Package::open(saved).expect("reopen package"));
    for (name, original) in before {
        if edited.iter().any(|part| name.ends_with(part)) {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(original),
            "a group edit dirtied unrelated part {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Addressing — the group is one shape on the top-level index space; its members are a path deeper
// ---------------------------------------------------------------------------------------------

#[test]
fn a_group_counts_as_one_shape_and_its_members_are_not_top_level() {
    let mut pres = layouts();
    // Title, Subtitle, Group 3, Table — the group's two members are not on the top-level space.
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 4);
    assert_eq!(
        pres.shape_kind(SLIDE2, 2).expect("kind"),
        ShapeKind::GroupShape
    );
}

#[test]
fn a_path_addresses_a_member_by_its_kind_and_geometry() {
    let mut pres = layouts();
    // Member 0 is a rectangle, member 1 an ellipse — reached only through the group at index 2.
    assert_eq!(
        pres.shape_kind(SLIDE2, [2, 0]).expect("member 0 kind"),
        ShapeKind::Shape
    );
    assert_eq!(
        pres.shape_kind(SLIDE2, [2, 1]).expect("member 1 kind"),
        ShapeKind::Shape
    );
    // The one index space still means a bare index is a top-level shape.
    assert_eq!(
        pres.shape_kind(SLIDE2, 3).expect("table kind"),
        ShapeKind::GraphicFrame
    );
}

#[test]
fn a_members_explicit_transform_is_read_in_the_groups_child_space() {
    let mut pres = layouts();
    // Rectangle 4 sits at the child-space origin the group declares (chOff = 1000000, 2000000).
    let rect = pres
        .shape_bounds(SLIDE2, [2, 0])
        .expect("member 0 bounds")
        .expect("the member places itself");
    assert_eq!(rect.offset_x_emu, 1_000_000);
    assert_eq!(rect.offset_y_emu, 2_000_000);
    assert_eq!(rect.width_emu, 1_828_800);
    assert_eq!(rect.height_emu, 1_828_800);

    // Rectangle 5 (the ellipse) is one rectangle-width to the right, in that same child space.
    let ellipse = pres
        .shape_bounds(SLIDE2, [2, 1])
        .expect("member 1 bounds")
        .expect("the member places itself");
    assert_eq!(ellipse.offset_x_emu, 2_828_800);
}

// ---------------------------------------------------------------------------------------------
// Out-of-range addresses name the path and the container that ran out of shapes
// ---------------------------------------------------------------------------------------------

#[test]
fn a_member_index_past_the_end_reports_the_groups_member_count() {
    let mut pres = layouts();
    let err = pres
        .shape_bounds(SLIDE2, [2, 9])
        .expect_err("the group has only two members");
    match err {
        PptxError::ShapeIndexOutOfRange {
            surface,
            path,
            count,
        } => {
            assert_eq!(surface, SLIDE2);
            assert_eq!(path.indices(), [2, 9]);
            assert_eq!(count, 2, "the group holds two members");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn descending_into_a_non_group_finds_zero_members() {
    let mut pres = layouts();
    // Index 0 is the Title, an ordinary shape — it has no members, so [0, 0] is out of range at a
    // zero-member container rather than silently reaching some child element.
    let err = pres
        .shape_kind(SLIDE2, [0, 0])
        .expect_err("a plain shape has no members");
    match err {
        PptxError::ShapeIndexOutOfRange { path, count, .. } => {
            assert_eq!(path.indices(), [0, 0]);
            assert_eq!(count, 0);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Editing a member — the same surface as a top-level shape, and the same fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn a_members_fill_can_be_set_and_read_back_and_dirties_only_its_slide() {
    let bytes = fixture("layouts.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    let red = FillSpec::Solid(ColorSpec::Srgb("FF0000".into()));
    pres.set_shape_fill(SLIDE2, [2, 1], &red)
        .expect("set member fill");
    assert_eq!(
        pres.shape_fill(SLIDE2, [2, 1]).expect("read member fill"),
        Some(red.clone())
    );

    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &["slides/slide2.xml"]);

    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reread
            .shape_fill(SLIDE2, [2, 1])
            .expect("reread member fill"),
        Some(red)
    );
    // The sibling member was untouched.
    assert_eq!(
        reread.shape_fill(SLIDE2, [2, 0]).expect("sibling fill"),
        None
    );
}

#[test]
fn removing_a_member_leaves_the_group_and_the_top_level_intact() {
    let bytes = fixture("layouts.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    // Remove the rectangle (member 0). The ellipse survives as the group's only member.
    pres.remove_shape(SLIDE2, [2, 0]).expect("remove member 0");

    // The top-level count is unchanged — the group is still one shape.
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 4);
    // What was member 1 (the ellipse) is now member 0.
    assert_eq!(
        pres.shape_kind(SLIDE2, [2, 0]).expect("kind"),
        ShapeKind::Shape
    );
    let err = pres
        .shape_kind(SLIDE2, [2, 1])
        .expect_err("only one member remains");
    assert!(matches!(
        err,
        PptxError::ShapeIndexOutOfRange { count: 1, .. }
    ));

    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &["slides/slide2.xml"]);
}

#[test]
fn reading_a_member_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres.shape_kind(SLIDE2, [2, 0]).expect("kind");
    let _ = pres.shape_bounds(SLIDE2, [2, 1]).expect("bounds");
    let _ = pres.shape_fill(SLIDE2, [2, 1]).expect("fill");
    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &[]);
}
