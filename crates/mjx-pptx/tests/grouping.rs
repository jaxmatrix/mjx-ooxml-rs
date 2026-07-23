//! Integration tests for **group structure**: making a group, dissolving one, and moving shapes
//! between them.
//!
//! One property runs through all of it — *nothing moves on screen*. Grouping, ungrouping and
//! reparenting all change the coordinate system a shape's numbers are written in, so every test here
//! checks absolute `shape_bounds` before and after and demands they match. That is the whole point:
//! the file changes, the slide does not.
//!
//! `tests/fixtures/layouts.pptx` slide 2 holds four top-level shapes — `0` Title, `1` Subtitle,
//! `2` `Group 3` (two members, a clean 0.5× map), `3` a table.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Angle, Transform2D};
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds, ShapeKind, ShapePath, Surface};

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

#[track_caller]
fn bounds_of(pres: &mut Presentation, path: impl Into<ShapePath>) -> ShapeBounds {
    pres.shape_bounds(SLIDE2, path)
        .expect("bounds")
        .expect("the shape places itself")
}

#[track_caller]
fn stated(pres: &mut Presentation, path: impl Into<ShapePath>) -> Transform2D {
    pres.shape_transform(SLIDE2, path)
        .expect("transform")
        .expect("the shape states one")
}

/// Every `p:cNvPr@id` in the saved slide, for the uniqueness check.
fn non_visual_ids(saved: &[u8]) -> Vec<String> {
    let xml = String::from_utf8(
        Package::open(saved)
            .expect("reopen")
            .entries()
            .iter()
            .find(|e| e.name == SLIDE2_PART)
            .and_then(|e| e.bytes())
            .expect("slide bytes")
            .to_vec(),
    )
    .expect("utf-8");
    xml.split("<p:cNvPr ")
        .skip(1)
        .filter_map(|rest| {
            let id = rest.split("id=\"").nth(1)?;
            Some(id.split('"').next()?.to_owned())
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Grouping is lossless
// ---------------------------------------------------------------------------------------------

#[test]
fn grouping_boxes_the_members_and_moves_nothing() {
    let mut pres = layouts();
    let title = bounds_of(&mut pres, 0);
    let subtitle = bounds_of(&mut pres, 1);

    let group = pres
        .group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group the title and subtitle");

    // The group takes the earliest member's z-order position, so it is shape 0 and the two shapes
    // that were 2 and 3 have moved down one.
    assert_eq!(group.indices(), [0]);
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 3);
    assert_eq!(
        pres.shape_kind(SLIDE2, 0).expect("kind"),
        ShapeKind::GroupShape
    );
    assert_eq!(pres.shape_member_count(SLIDE2, 0).expect("members"), 2);

    // Its box is the union of what it wraps.
    assert_eq!(bounds_of(&mut pres, &group), title.union(subtitle));

    // And neither member has moved a single EMU.
    assert_eq!(bounds_of(&mut pres, group.child(0)), title);
    assert_eq!(bounds_of(&mut pres, group.child(1)), subtitle);
}

#[test]
fn a_grouped_member_keeps_its_coordinates_verbatim() {
    // The new group's child space is identical to its box, so the mapping is the identity and the
    // members' own numbers are untouched — grouping introduces no rounding anywhere.
    let mut pres = layouts();
    let before = stated(&mut pres, 0);
    let group = pres
        .group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group");
    assert_eq!(stated(&mut pres, group.child(0)).position, before.position);
    assert_eq!(stated(&mut pres, group.child(0)).size, before.size);
}

#[test]
fn the_order_members_are_named_in_does_not_matter() {
    let mut pres = layouts();
    let forwards = pres
        .group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group");
    let forwards_first = bounds_of(&mut pres, forwards.child(0));

    let mut pres = layouts();
    let backwards = pres
        .group_shapes(SLIDE2, &[1.into(), 0.into()])
        .expect("group");
    // Members keep document order inside the group whichever way round they were named.
    assert_eq!(bounds_of(&mut pres, backwards.child(0)), forwards_first);
}

#[test]
fn grouping_then_ungrouping_puts_everything_back() {
    let mut pres = layouts();
    let before: Vec<ShapeBounds> = (0..4).map(|i| bounds_of(&mut pres, i)).collect();
    let stated_before: Vec<Transform2D> = (0..4).map(|i| stated(&mut pres, i)).collect();

    let group = pres
        .group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group");
    let freed = pres.ungroup(SLIDE2, &group).expect("ungroup");
    assert_eq!(freed.len(), 2);

    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 4);
    for index in 0..4 {
        assert_eq!(bounds_of(&mut pres, index), before[index], "shape {index}");
        assert_eq!(
            stated(&mut pres, index).position,
            stated_before[index].position,
            "shape {index} states a different position"
        );
    }
}

#[test]
fn grouping_keeps_every_non_visual_id_unique() {
    let mut pres = layouts();
    pres.group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group");
    let saved = pres.save().expect("save");
    let mut ids = non_visual_ids(&saved);
    let count = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "a p:cNvPr@id was reused");
}

#[test]
fn a_group_can_be_made_inside_a_group() {
    // `Group 3`'s two members are siblings of each other, so they can be grouped where they are.
    let mut pres = layouts();
    let first = bounds_of(&mut pres, [2, 0]);
    let inner = pres
        .group_shapes(SLIDE2, &[[2, 0].into(), [2, 1].into()])
        .expect("group inside the group");

    assert_eq!(inner.indices(), [2, 0]);
    assert_eq!(pres.shape_member_count(SLIDE2, 2).expect("members"), 1);
    assert_eq!(pres.shape_member_count(SLIDE2, &inner).expect("members"), 2);
    // Two levels of nesting, and the shape has still not moved.
    assert_eq!(bounds_of(&mut pres, inner.child(0)), first);
}

// ---------------------------------------------------------------------------------------------
// Reparenting keeps a shape exactly where it looks
// ---------------------------------------------------------------------------------------------

#[test]
fn moving_a_shape_into_a_scaled_group_does_not_move_it() {
    // `Group 3` halves its members, so the table's stated size must double to look the same.
    let mut pres = layouts();
    let before = bounds_of(&mut pres, 3);
    let before_stated = stated(&mut pres, 3);

    let moved = pres
        .move_shape_into_group(SLIDE2, 3, 2)
        .expect("move the table into the group");

    assert_eq!(moved.indices(), [2, 2]);
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 3);
    assert_eq!(bounds_of(&mut pres, &moved), before, "the table moved");
    assert_eq!(
        stated(&mut pres, &moved).size.expect("ext").width.emu(),
        before_stated.size.expect("ext").width.emu() * 2,
        "the stated extent must double to survive the group's halving"
    );
}

#[test]
fn moving_into_a_mirrored_and_rotated_group_does_not_move_it() {
    // The case a translate-only conversion gets wrong: the group turns and flips its members, so the
    // shape's own rotation and mirror have to cancel that out.
    let mut pres = layouts();
    pres.set_shape_transform(
        SLIDE2,
        2,
        &Transform2D {
            rotation: Some(Angle::from_degrees(90.0)),
            flip_horizontal: Some(true),
            ..Transform2D::default()
        },
    )
    .expect("turn and flip the group");

    let before = bounds_of(&mut pres, 3);
    let moved = pres.move_shape_into_group(SLIDE2, 3, 2).expect("move in");
    assert_eq!(bounds_of(&mut pres, &moved), before, "the table moved");

    // It compensates by stating the opposite turn and mirror.
    let own = stated(&mut pres, &moved);
    assert!((own.rotation.expect("rotated").degrees() + 90.0).abs() < 1e-9);
    assert_eq!(own.flip_horizontal, Some(true));
}

#[test]
fn a_shape_moved_in_and_out_again_states_what_it_started_with() {
    let mut pres = layouts();
    let before = stated(&mut pres, 3);
    let absolute = bounds_of(&mut pres, 3);

    let inside = pres.move_shape_into_group(SLIDE2, 3, 2).expect("move in");
    let outside = pres
        .move_shape_out_of_group(SLIDE2, &inside)
        .expect("move out");

    assert_eq!(bounds_of(&mut pres, &outside), absolute);
    assert_eq!(stated(&mut pres, &outside).position, before.position);
    assert_eq!(stated(&mut pres, &outside).size, before.size);
}

#[test]
fn a_shape_lands_directly_after_the_group_it_left() {
    let mut pres = layouts();
    // Member 0 of the group at index 2 comes out as top-level shape 3, right behind its old group.
    let freed = pres
        .move_shape_out_of_group(SLIDE2, [2, 0])
        .expect("move out");
    assert_eq!(freed.indices(), [3]);
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 5);
    assert_eq!(pres.shape_member_count(SLIDE2, 2).expect("members"), 1);
}

#[test]
fn moving_a_shape_that_sits_before_its_destination_still_finds_it() {
    // Lifting shape 0 out shifts every later index down one, the destination group included — the
    // move has to account for that or it puts the shape somewhere else entirely.
    let mut pres = layouts();
    let before = bounds_of(&mut pres, 0);
    let moved = pres
        .move_shape_into_group(SLIDE2, 0, 2)
        .expect("move the title into the group");

    // The group was index 2 and is index 1 once the title is gone.
    assert_eq!(moved.indices(), [1, 2]);
    assert_eq!(
        pres.shape_kind(SLIDE2, 1).expect("kind"),
        ShapeKind::GroupShape
    );
    assert_eq!(bounds_of(&mut pres, &moved), before);
}

// ---------------------------------------------------------------------------------------------
// Refusals — each changes nothing
// ---------------------------------------------------------------------------------------------

#[track_caller]
fn unchanged_by(what: impl FnOnce(&mut Presentation) -> PptxError) -> PptxError {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    let error = what(&mut pres);
    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot,
        "a refused operation changed the package"
    );
    error
}

#[test]
fn a_group_needs_at_least_two_shapes() {
    let error = unchanged_by(|pres| {
        pres.group_shapes(SLIDE2, &[0.into()])
            .expect_err("one shape is a degenerate group")
    });
    assert!(matches!(
        error,
        PptxError::GroupNeedsTwoShapes { count: 1, .. }
    ));
    assert!(matches!(
        unchanged_by(|pres| pres.group_shapes(SLIDE2, &[]).expect_err("no shapes")),
        PptxError::GroupNeedsTwoShapes { count: 0, .. }
    ));
}

#[test]
fn members_of_a_group_must_be_siblings_and_distinct() {
    // One top-level, one inside `Group 3`: there is no single place to put the group.
    assert!(matches!(
        unchanged_by(|pres| pres
            .group_shapes(SLIDE2, &[0.into(), [2, 0].into()])
            .expect_err("not siblings")),
        PptxError::ShapesAreNotSiblings { .. }
    ));
    // The same shape twice would have to be in two places at once.
    assert!(matches!(
        unchanged_by(|pres| pres
            .group_shapes(SLIDE2, &[0.into(), 0.into()])
            .expect_err("named twice")),
        PptxError::ShapesAreNotSiblings { .. }
    ));
}

#[test]
fn a_shape_cannot_be_moved_inside_itself() {
    assert!(matches!(
        unchanged_by(|pres| pres
            .move_shape_into_group(SLIDE2, 2, 2)
            .expect_err("into itself")),
        PptxError::ShapeCannotContainItself { .. }
    ));
    // Nor into one of its own members.
    assert!(matches!(
        unchanged_by(|pres| pres
            .move_shape_into_group(SLIDE2, 2, [2, 0])
            .expect_err("into its own member")),
        PptxError::ShapeCannotContainItself { .. }
    ));
}

#[test]
fn only_a_group_can_take_a_member_or_be_dissolved() {
    assert!(matches!(
        unchanged_by(|pres| pres
            .move_shape_into_group(SLIDE2, 0, 1)
            .expect_err("the subtitle is not a group")),
        PptxError::ShapeIsNotAGroup { .. }
    ));
    assert!(matches!(
        unchanged_by(|pres| pres.ungroup(SLIDE2, 0).expect_err("not a group")),
        PptxError::ShapeIsNotAGroup { .. }
    ));
}

#[test]
fn a_top_level_shape_has_no_group_to_leave() {
    assert!(matches!(
        unchanged_by(|pres| pres
            .move_shape_out_of_group(SLIDE2, 0)
            .expect_err("already top-level")),
        PptxError::ShapeHasNoParent { .. }
    ));
}

// ---------------------------------------------------------------------------------------------
// Fidelity, and the cursor
// ---------------------------------------------------------------------------------------------

#[test]
fn a_structural_edit_dirties_only_its_slide() {
    let bytes = fixture("layouts.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    pres.group_shapes(SLIDE2, &[0.into(), 1.into()])
        .expect("group");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "no part added or removed"
    );
    for (name, original) in &before {
        if name == SLIDE2_PART {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "part {name} was dirtied");
    }
    assert_ne!(after.get(SLIDE2_PART), before.get(SLIDE2_PART));
}

#[test]
fn the_cursor_moves_a_shape_exactly_as_the_flat_call_does() {
    let mut fluent = layouts();
    fluent
        .shape(SLIDE2, 3)
        .expect("cursor")
        .into_group(2)
        .expect("move in")
        .apply()
        .expect("apply");

    let mut flat = layouts();
    flat.move_shape_into_group(SLIDE2, 3, 2).expect("move in");

    assert_eq!(
        byte_map(&Package::open(&fluent.save().expect("save")).expect("reopen")),
        byte_map(&Package::open(&flat.save().expect("save")).expect("reopen"))
    );
}

#[test]
fn a_structural_step_commits_what_was_recorded_before_it() {
    // The rule that makes structure and deferred edits safe together: the fill lands on the shape it
    // was recorded against, and the outline on the same shape at its new address — not on whatever
    // else happens to answer to index 3 afterwards.
    use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};
    let navy = FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned()));
    let rule = LineSpec {
        width: Some(LineWidth::from_points(2.0)),
        ..LineSpec::new()
    };

    let mut pres = layouts();
    // Shape 0 is the title; grouping 1 and 2 renumbers everything after them.
    pres.shape(SLIDE2, 0)
        .expect("cursor")
        .fill(navy.clone())
        .into_group(2)
        .expect("move into the group")
        .outline(rule.clone())
        .apply()
        .expect("apply");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    // The title is now the group's third member, carrying both edits.
    let title = ShapePath::from([1, 2]);
    assert_eq!(reread.shape_fill(SLIDE2, &title).expect("fill"), Some(navy));
    assert_eq!(
        reread
            .shape_outline(SLIDE2, &title)
            .expect("outline")
            .and_then(|line| line.width),
        rule.width
    );
    // And the shape that inherited index 0 was left alone.
    assert_eq!(reread.shape_fill(SLIDE2, 0).expect("fill"), None);
}

#[test]
fn the_cursor_can_group_and_then_address_the_group() {
    let mut pres = layouts();
    let mut group = pres
        .shape(SLIDE2, 0)
        .expect("cursor")
        .group_with(&[1.into()])
        .expect("group with the subtitle");
    assert_eq!(group.path().indices(), [0]);
    assert_eq!(group.kind().expect("kind"), ShapeKind::GroupShape);

    let freed = group.ungroup().expect("dissolve it again");
    assert_eq!(freed.len(), 2);
    assert_eq!(pres.shape_count(SLIDE2).expect("count"), 4);
}

#[test]
fn reading_the_tree_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres.shape_member_count(SLIDE2, 2).expect("members");
    let _ = bounds_of(&mut pres, [2, 0]);
    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        snapshot
    );
}
