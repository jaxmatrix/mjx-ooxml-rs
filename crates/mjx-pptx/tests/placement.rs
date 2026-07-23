//! Integration tests for **where a group member actually is**: `shape_bounds` and
//! `set_shape_bounds` in absolute slide coordinates, composing and inverting every enclosing group's
//! child coordinate space.
//!
//! `tests/fixtures/layouts.pptx` slide 2 carries `Group 3` at top-level index `2`. Its own box is
//! `off (457200, 365125)` / `ext (1828800, 914400)`, and the space it lays members out in is
//! `chOff (1000000, 2000000)` / `chExt (3657600, 1828800)` — a clean **0.5×** map, so every expected
//! number below is exact rather than rounded. Member `0` is a rectangle at the child origin and
//! member `1` an ellipse one rectangle-width to its right.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Angle, Position, Size, Transform2D};
use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_opc::{Package, PartName};
use mjx_pptx::{PptxError, Presentation, ShapeBounds, Surface};

const SLIDE2: Surface = Surface::Slide(1);
const SLIDE2_PART: &str = "ppt/slides/slide2.xml";

/// The group's own box and the child space it maps onto it.
const GROUP_OFFSET: (i64, i64) = (457_200, 365_125);
const GROUP_EXTENT: (i64, i64) = (1_828_800, 914_400);

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

#[track_caller]
fn bounds_of(pres: &mut Presentation, path: impl Into<mjx_pptx::ShapePath>) -> ShapeBounds {
    pres.shape_bounds(SLIDE2, path)
        .expect("bounds")
        .expect("the shape places itself")
}

// ---------------------------------------------------------------------------------------------
// Reading — a member answers in slide coordinates
// ---------------------------------------------------------------------------------------------

#[test]
fn a_member_reports_where_it_is_on_the_slide() {
    let mut pres = layouts();

    // Member 0 sits at the child origin, so it lands on the group's own origin, at half size.
    assert_eq!(
        bounds_of(&mut pres, [2, 0]),
        ShapeBounds::new(GROUP_OFFSET.0, GROUP_OFFSET.1, 914_400, 914_400)
    );

    // Member 1 is 1828800 further right in child space; halved, that is 914400 on the slide — the
    // absolute x of 1371600 that MJX-91 names.
    assert_eq!(
        bounds_of(&mut pres, [2, 1]),
        ShapeBounds::new(1_371_600, GROUP_OFFSET.1, 914_400, 914_400)
    );
}

#[test]
fn a_top_level_shape_reads_exactly_as_it_always_did() {
    // Composition over an empty ancestor list is the identity; this is the regression guard that
    // says so. The group itself, the title, and the table are all top-level.
    let mut pres = layouts();
    assert_eq!(
        bounds_of(&mut pres, 2),
        ShapeBounds::new(
            GROUP_OFFSET.0,
            GROUP_OFFSET.1,
            GROUP_EXTENT.0,
            GROUP_EXTENT.1
        )
    );
    assert_eq!(
        bounds_of(&mut pres, 0),
        ShapeBounds::new(685_800, 2_130_425, 7_772_400, 1_470_025)
    );
    assert_eq!(
        bounds_of(&mut pres, 3),
        ShapeBounds::new(5_486_400, 365_125, 3_048_000, 370_840)
    );
}

#[test]
fn the_members_lie_inside_the_group_that_holds_them() {
    // The strongest sanity check there is: whatever the arithmetic, a member cannot render outside
    // the box its group occupies.
    let mut pres = layouts();
    let group = bounds_of(&mut pres, 2);
    for member in 0..2 {
        let bounds = bounds_of(&mut pres, [2, member]);
        assert!(
            bounds.offset_x_emu >= group.offset_x_emu
                && bounds.offset_y_emu >= group.offset_y_emu
                && bounds.offset_x_emu + bounds.width_emu <= group.offset_x_emu + group.width_emu
                && bounds.offset_y_emu + bounds.height_emu <= group.offset_y_emu + group.height_emu,
            "member {member} at {bounds:?} escapes its group at {group:?}"
        );
    }
}

#[test]
fn reading_a_members_position_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = bounds_of(&mut pres, [2, 1]);
    let _ = pres
        .effective_shape_bounds(SLIDE2, [2, 1])
        .expect("effective");
    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot
    );
}

// ---------------------------------------------------------------------------------------------
// Mirrors and rotation — the part the naive affine gets wrong
// ---------------------------------------------------------------------------------------------

/// Applies `transform` to `Group 3` itself, leaving its members untouched.
fn with_group_transform(transform: Transform2D) -> Presentation {
    let mut pres = layouts();
    pres.set_shape_transform(SLIDE2, 2, &transform)
        .expect("transform the group");
    pres
}

#[test]
fn mirroring_a_group_moves_its_members_within_it() {
    // A flip reflects about the group box centre, so the two members swap sides — and each stays an
    // axis-aligned rectangle, which is why a flip can be folded into `ShapeBounds` at all.
    let mut pres = with_group_transform(Transform2D {
        flip_horizontal: Some(true),
        ..Transform2D::default()
    });

    // Member 0 occupied the left half (x 457200..1371600); mirrored it occupies the right half.
    assert_eq!(
        bounds_of(&mut pres, [2, 0]),
        ShapeBounds::new(1_371_600, GROUP_OFFSET.1, 914_400, 914_400)
    );
    assert_eq!(
        bounds_of(&mut pres, [2, 1]),
        ShapeBounds::new(GROUP_OFFSET.0, GROUP_OFFSET.1, 914_400, 914_400)
    );

    // The group itself has not moved; only where its members sit inside it has changed.
    assert_eq!(
        bounds_of(&mut pres, 2),
        ShapeBounds::new(
            GROUP_OFFSET.0,
            GROUP_OFFSET.1,
            GROUP_EXTENT.0,
            GROUP_EXTENT.1
        )
    );
}

#[test]
fn mirroring_a_group_twice_puts_its_members_back() {
    let mut pres = with_group_transform(Transform2D {
        flip_horizontal: Some(true),
        flip_vertical: Some(true),
        ..Transform2D::default()
    });
    let flipped = bounds_of(&mut pres, [2, 0]);

    pres.set_shape_transform(
        SLIDE2,
        2,
        &Transform2D {
            flip_horizontal: Some(false),
            flip_vertical: Some(false),
            ..Transform2D::default()
        },
    )
    .expect("unflip");
    assert_ne!(flipped, bounds_of(&mut pres, [2, 0]));
    assert_eq!(
        bounds_of(&mut pres, [2, 0]),
        ShapeBounds::new(GROUP_OFFSET.0, GROUP_OFFSET.1, 914_400, 914_400)
    );
}

#[test]
fn rotating_a_group_carries_its_members_centres_round() {
    // §L.4.7.4: a member is placed so its *centre* lands where the whole chain puts it. The group
    // box is 1828800 x 914400 centred at (1371600, 822325); member 0's centre starts at
    // (914400, 822325), one quarter-width left of it. A half-turn puts it the same distance right.
    let mut pres = with_group_transform(Transform2D {
        rotation: Some(Angle::from_degrees(180.0)),
        ..Transform2D::default()
    });
    assert_eq!(
        bounds_of(&mut pres, [2, 0]),
        ShapeBounds::new(1_371_600, GROUP_OFFSET.1, 914_400, 914_400)
    );

    // A quarter-turn takes member 0's centre from (914400, 822325) — a quarter-width left of the
    // group centre — up to (1371600, 365125). Its box is still 914400 square, because a member
    // scales along its own axes rather than the group's, so it now starts 457200 *above* the group's
    // top edge: a quarter-turned wide group sweeps its members outside the box it occupied, and a
    // negative offset is a shape hanging off the top of the slide, which is legal.
    let mut pres = with_group_transform(Transform2D {
        rotation: Some(Angle::from_degrees(90.0)),
        ..Transform2D::default()
    });
    let quarter = bounds_of(&mut pres, [2, 0]);
    assert_eq!((quarter.width_emu, quarter.height_emu), (914_400, 914_400));
    assert_eq!(quarter.offset_x_emu, 914_400);
    assert_eq!(quarter.offset_y_emu, 365_125 - 457_200);

    // The composed rotation is the part `ShapeBounds` cannot hold, so it is read from the transform.
    let composed = pres
        .effective_shape_transform(SLIDE2, [2, 0])
        .expect("effective")
        .expect("placed");
    assert!((composed.rotation.expect("rotated").degrees() - 90.0).abs() < 1e-9);
}

#[test]
fn a_group_with_no_child_space_places_nothing() {
    // Without `a:chOff` / `a:chExt` a group's mapping is defined only implicitly, so a member is
    // reported as unplaced rather than given a rectangle computed from a guess.
    let mut pres = layouts();
    let stripped = strip_child_space(&mut pres);
    let mut pres = Presentation::open(&stripped).expect("reopen");

    assert_eq!(pres.shape_bounds(SLIDE2, [2, 0]).expect("bounds"), None);
    // The group itself is top-level and still places itself perfectly well.
    assert!(pres.shape_bounds(SLIDE2, 2).expect("bounds").is_some());
    // And the member's own stated transform is still readable.
    assert!(pres
        .shape_transform(SLIDE2, [2, 0])
        .expect("transform")
        .is_some());

    // Placing it by slide coordinates is refused rather than written to the wrong place.
    let err = pres
        .set_shape_bounds(SLIDE2, [2, 0], ShapeBounds::from_inches(1.0, 1.0, 1.0, 1.0))
        .expect_err("no mapping to invert");
    match err {
        PptxError::ShapeCannotBePlaced { surface, path } => {
            assert_eq!(surface, SLIDE2);
            assert_eq!(path.indices(), [2, 0]);
        }
        other => panic!("expected ShapeCannotBePlaced, got {other:?}"),
    }
}

/// Saves the deck with every `a:chOff` / `a:chExt` dropped from slide 2 — only `Group 3` has them,
/// so this is exactly "a group that states no child coordinate space".
fn strip_child_space(pres: &mut Presentation) -> Vec<u8> {
    let saved = pres.save().expect("save");
    let mut package = Package::open(&saved).expect("reopen");
    let part = PartName::new(&format!("/{SLIDE2_PART}")).expect("part name");
    {
        let doc = package.part_tree_mut(&part).expect("slide tree");
        let RawDocument { root, interner, .. } = doc;
        drop_child_space(root, interner);
    }
    package.save().expect("save package")
}

/// Removes every `a:chOff` / `a:chExt` element in the subtree.
fn drop_child_space(element: &mut RawElement, interner: &Interner) {
    element.children.retain(|node| match node {
        RawNode::Element(child) => !matches!(interner.resolve(child.name.local), "chOff" | "chExt"),
        _ => true,
    });
    for node in &mut element.children {
        if let RawNode::Element(child) = node {
            drop_child_space(child, interner);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Writing — the same space, mapped back
// ---------------------------------------------------------------------------------------------

#[test]
fn writing_back_what_was_read_changes_nothing() {
    // The scale is exactly 0.5, so the round trip is exact and the part is byte-identical.
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    let read_back = bounds_of(&mut pres, [2, 1]);
    pres.set_shape_bounds(SLIDE2, [2, 1], read_back)
        .expect("write it back");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(
        after.get(SLIDE2_PART),
        snapshot.get(SLIDE2_PART),
        "writing back the same position must not change the file"
    );
}

#[test]
fn placing_a_member_writes_child_space_coordinates() {
    let mut pres = layouts();
    // Move the ellipse onto the rectangle's absolute position.
    let target = ShapeBounds::new(GROUP_OFFSET.0, GROUP_OFFSET.1, 914_400, 914_400);
    pres.set_shape_bounds(SLIDE2, [2, 1], target)
        .expect("place the member");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    // Read back absolutely: it is where it was put.
    assert_eq!(bounds_of(&mut reread, [2, 1]), target);
    // And the file states it in the group's child space, un-halved.
    let stated = reread
        .shape_transform(SLIDE2, [2, 1])
        .expect("transform")
        .expect("placed");
    assert_eq!(
        stated.position,
        Some(Position::from_emu(1_000_000, 2_000_000))
    );
    assert_eq!(stated.size, Some(Size::from_emu(1_828_800, 1_828_800)));
}

#[test]
fn placing_a_member_dirties_only_its_slide() {
    let bytes = fixture("layouts.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_shape_bounds(SLIDE2, [2, 0], ShapeBounds::from_inches(2.0, 2.0, 1.0, 1.0))
        .expect("place");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for (name, original) in &before {
        if name == SLIDE2_PART {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "part {name} was dirtied");
    }
}

#[test]
fn resizing_a_member_scales_what_it_states() {
    // Doubling a member's absolute size doubles what it states, because the group halves it again.
    let mut pres = layouts();
    let current = bounds_of(&mut pres, [2, 0]);
    let doubled = ShapeBounds::new(
        current.offset_x_emu,
        current.offset_y_emu,
        current.width_emu * 2,
        current.height_emu * 2,
    );
    pres.set_shape_bounds(SLIDE2, [2, 0], doubled)
        .expect("resize");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(bounds_of(&mut reread, [2, 0]), doubled);
    assert_eq!(
        reread
            .shape_transform(SLIDE2, [2, 0])
            .expect("transform")
            .expect("placed")
            .size,
        Some(Size::from_emu(3_657_600, 3_657_600))
    );
}

// ---------------------------------------------------------------------------------------------
// The cursor says the same thing
// ---------------------------------------------------------------------------------------------

#[test]
fn the_cursor_places_a_member_exactly_as_the_flat_call_does() {
    // `.bounds()` is slide-absolute like `set_shape_bounds`, so it must run the same conversion
    // rather than writing absolute numbers straight into the group's child space.
    let target = ShapeBounds::new(GROUP_OFFSET.0, GROUP_OFFSET.1, 914_400, 914_400);

    let mut fluent = layouts();
    fluent
        .shape(SLIDE2, 2)
        .expect("cursor")
        .member(1)
        .expect("member 1")
        .bounds(target)
        .apply()
        .expect("apply");

    let mut flat = layouts();
    flat.set_shape_bounds(SLIDE2, [2, 1], target)
        .expect("place");

    assert_eq!(
        byte_map(&Package::open(&fluent.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&flat.save().expect("save")).expect("reopen"))
    );
}

#[test]
fn a_cursor_transform_stays_in_the_shapes_own_space() {
    // `.transform()` is the verbatim writer, so it must *not* convert — the pair mirrors
    // `set_shape_transform` against `set_shape_bounds`.
    let mut pres = layouts();
    pres.shape(SLIDE2, 2)
        .expect("cursor")
        .member(0)
        .expect("member 0")
        .transform(Transform2D {
            position: Some(Position::from_emu(1_500_000, 2_500_000)),
            ..Transform2D::default()
        })
        .apply()
        .expect("apply");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread
            .shape_transform(SLIDE2, [2, 0])
            .expect("transform")
            .expect("placed")
            .position,
        Some(Position::from_emu(1_500_000, 2_500_000)),
        "the transform was written verbatim, in child space"
    );
    // Which lands at half the child-space displacement on the slide.
    assert_eq!(bounds_of(&mut reread, [2, 0]).offset_x_emu, 707_200);
}
