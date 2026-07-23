//! Unit tests for the DrawingML 2-D transform model (`a:xfrm`), through the public API only.
//!
//! The two properties this model exists for get the most attention: **absent is not zero** (what
//! makes inheritance decidable), and **`apply` writes only what it names** (what makes an edit
//! non-destructive). Every write assertion is paired with a read-back or a byte assertion, so
//! nothing can pass by preserving the input and reporting fiction.

use mjx_dml::{Angle, Emu, Position, Size, Transform2D};
use mjx_ooxml_core::{Interner, RawDocument, RawElement};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// Parses an `a:xfrm` fragment into its element plus the interner resolving its names.
fn parse(fragment: &str) -> (RawElement, Interner) {
    let doc = fidelity::parse(fragment.as_bytes()).expect("fragment parses");
    let RawDocument { root, interner, .. } = doc;
    (root, interner)
}

/// An `a:xfrm` fragment with the namespace declaration the fidelity reader needs to resolve it.
fn xfrm(body: &str) -> String {
    format!(r#"<a:xfrm xmlns:a="{A}"{body}</a:xfrm>"#)
}

/// Serializes an element back to a string, for asserting on what was actually written.
fn serialize(root: RawElement, interner: Interner) -> String {
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_every_field() {
    let (element, interner) = parse(&xfrm(
        r#" rot="2700000" flipH="1" flipV="0">
              <a:off x="914400" y="-100"/>
              <a:ext cx="3657600" cy="1828800"/>"#,
    ));
    let transform = Transform2D::read(&element, &interner);

    assert_eq!(transform.position, Some(Position::from_emu(914_400, -100)));
    assert_eq!(transform.size, Some(Size::from_emu(3_657_600, 1_828_800)));
    assert_eq!(transform.rotation.map(Angle::degrees), Some(45.0));
    assert_eq!(transform.flip_horizontal, Some(true));
    assert_eq!(transform.flip_vertical, Some(false));
    // Not a group: the child coordinate space is absent, not defaulted.
    assert_eq!(transform.child_position, None);
    assert_eq!(transform.child_size, None);
}

#[test]
fn an_absent_value_is_not_a_zero_one() {
    // The distinction the whole inheritance walk rests on.
    let (element, interner) = parse(&xfrm(">"));
    assert!(
        Transform2D::read(&element, &interner).is_empty(),
        "an empty a:xfrm names nothing, so a placeholder asks its layout"
    );

    let (element, interner) = parse(&xfrm(r#" rot="0"><a:off x="0" y="0"/>"#));
    let transform = Transform2D::read(&element, &interner);
    assert!(
        !transform.is_empty(),
        "an explicit zero is an answer, and stops the walk"
    );
    assert_eq!(transform.position, Some(Position::from_emu(0, 0)));
    assert_eq!(transform.rotation.map(Angle::degrees), Some(0.0));
    assert_eq!(transform.size, None, "only what was written is read");
}

#[test]
fn reads_a_groups_child_coordinate_space() {
    let (element, interner) = parse(&xfrm(
        r#"><a:off x="10" y="20"/><a:ext cx="30" cy="40"/>
            <a:chOff x="50" y="60"/><a:chExt cx="70" cy="80"/>"#,
    ));
    let transform = Transform2D::read(&element, &interner);

    assert_eq!(transform.position, Some(Position::from_emu(10, 20)));
    assert_eq!(transform.child_position, Some(Position::from_emu(50, 60)));
    assert_eq!(transform.child_size, Some(Size::from_emu(70, 80)));
}

#[test]
fn a_half_written_point_is_not_a_point() {
    // `x` and `y` are both `use="required"`; a child carrying one is malformed, not half-read.
    let (element, interner) = parse(&xfrm(r#"><a:off x="10"/><a:ext cy="4"/>"#));
    let transform = Transform2D::read(&element, &interner);

    assert_eq!(transform.position, None);
    assert_eq!(transform.size, None);
}

#[test]
fn an_unparsable_measure_does_not_fail_the_read() {
    // A file we cannot understand must still be readable — the value is absent, not an error.
    let (element, interner) = parse(&xfrm(r#" rot="clockwise"><a:off x="ten" y="20"/>"#));
    let transform = Transform2D::read(&element, &interner);

    assert_eq!(transform.rotation, None);
    assert_eq!(transform.position, None);
}

#[test]
fn every_boolean_spelling_reads() {
    for (wire, expected) in [("1", true), ("true", true), ("0", false), ("false", false)] {
        let (element, interner) = parse(&xfrm(&format!(r#" flipH="{wire}">"#)));
        assert_eq!(
            Transform2D::read(&element, &interner).flip_horizontal,
            Some(expected),
            "flipH=\"{wire}\""
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Writing — `apply` names what it changes, and touches nothing else
// ---------------------------------------------------------------------------------------------

#[test]
fn apply_writes_only_what_it_names() {
    let (mut element, mut interner) = parse(&xfrm(
        r#" rot="60000"><a:off x="1" y="2"/><a:ext cx="3" cy="4"/>"#,
    ));
    Transform2D {
        position: Some(Position::from_emu(99, 98)),
        ..Transform2D::default()
    }
    .apply(&mut element, &mut interner);

    let after = Transform2D::read(&element, &interner);
    assert_eq!(after.position, Some(Position::from_emu(99, 98)));
    // Left alone, because the caller did not name them. Unset means "leave it", never "clear it".
    assert_eq!(after.size, Some(Size::from_emu(3, 4)));
    assert_eq!(after.rotation.map(Angle::degrees), Some(1.0));
}

#[test]
fn moving_a_group_keeps_the_child_space_its_members_live_in() {
    // The reason `apply` edits in place instead of rebuilding: `chOff`/`chExt` are not this
    // caller's business, and losing them would move every member of the group.
    let (mut element, mut interner) = parse(&xfrm(
        r#"><a:off x="1" y="2"/><a:ext cx="3" cy="4"/>
            <a:chOff x="5" y="6"/><a:chExt cx="7" cy="8"/>"#,
    ));
    Transform2D {
        position: Some(Position::from_emu(100, 200)),
        ..Transform2D::default()
    }
    .apply(&mut element, &mut interner);

    let after = Transform2D::read(&element, &interner);
    assert_eq!(after.position, Some(Position::from_emu(100, 200)));
    assert_eq!(after.child_position, Some(Position::from_emu(5, 6)));
    assert_eq!(after.child_size, Some(Size::from_emu(7, 8)));
}

#[test]
fn an_edited_child_keeps_what_this_model_does_not_describe() {
    let (mut element, mut interner) =
        parse(&xfrm(r#"><a:off x="1" y="2" unknown="kept"/><a:extLst/>"#));
    Transform2D {
        position: Some(Position::from_emu(9, 9)),
        ..Transform2D::default()
    }
    .apply(&mut element, &mut interner);

    let xml = serialize(element, interner);
    assert!(xml.contains(r#"unknown="kept""#), "{xml}");
    assert!(xml.contains("<a:extLst/>"), "{xml}");
    assert!(xml.contains(r#"x="9""#), "{xml}");
}

#[test]
fn new_children_land_in_schema_order() {
    // Named out of sequence order; each must still land at its rank, because order is validity.
    let (mut element, mut interner) = parse(&xfrm(">"));
    Transform2D {
        child_size: Some(Size::from_emu(7, 8)),
        size: Some(Size::from_emu(3, 4)),
        child_position: Some(Position::from_emu(5, 6)),
        position: Some(Position::from_emu(1, 2)),
        ..Transform2D::default()
    }
    .apply(&mut element, &mut interner);

    let xml = serialize(element, interner);
    let at = |needle: &str| {
        xml.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {xml}"))
    };
    assert!(at("<a:off") < at("<a:ext"), "{xml}");
    assert!(at("<a:ext") < at("<a:chOff"), "{xml}");
    assert!(at("<a:chOff") < at("<a:chExt"), "{xml}");
}

#[test]
fn a_new_child_lands_at_its_rank_among_existing_ones() {
    // `a:off` is added to a transform that already has an `a:ext`: it belongs *before* it.
    let (mut element, mut interner) = parse(&xfrm(r#"><a:ext cx="3" cy="4"/>"#));
    Transform2D {
        position: Some(Position::from_emu(1, 2)),
        ..Transform2D::default()
    }
    .apply(&mut element, &mut interner);

    let xml = serialize(element, interner);
    assert!(
        xml.find("<a:off").unwrap() < xml.find("<a:ext").unwrap(),
        "{xml}"
    );
}

#[test]
fn a_shape_with_no_transform_gets_a_complete_one() {
    let mut interner = Interner::new();
    let mut element = Transform2D::empty_element(&mut interner);
    let transform = Transform2D {
        position: Some(Position::from_emu(914_400, 914_400)),
        size: Some(Size::from_emu(1_828_800, 914_400)),
        rotation: Some(Angle::from_degrees(90.0)),
        flip_horizontal: Some(true),
        ..Transform2D::default()
    };
    transform.apply(&mut element, &mut interner);

    assert_eq!(Transform2D::read(&element, &interner), transform);
    let xml = serialize(element, interner);
    assert!(xml.contains(r#"rot="5400000""#), "{xml}");
    assert!(xml.contains(r#"flipH="true""#), "{xml}");
    assert!(
        !xml.contains("<a:xfrm/>"),
        "a transform that gained children cannot stay self-closing: {xml}"
    );
}

#[test]
fn writing_the_same_value_twice_changes_nothing() {
    // Repeated edits must not accumulate attributes or children.
    let transform = Transform2D {
        position: Some(Position::from_emu(1, 2)),
        size: Some(Size::from_emu(3, 4)),
        flip_vertical: Some(true),
        ..Transform2D::default()
    };

    let mut once = Interner::new();
    let mut a = Transform2D::empty_element(&mut once);
    transform.apply(&mut a, &mut once);

    let mut twice = Interner::new();
    let mut b = Transform2D::empty_element(&mut twice);
    transform.apply(&mut b, &mut twice);
    transform.apply(&mut b, &mut twice);

    assert_eq!(serialize(a, once), serialize(b, twice));
}

// ---------------------------------------------------------------------------------------------
// Measures
// ---------------------------------------------------------------------------------------------

#[test]
fn rotation_round_trips_through_its_60000ths_of_a_degree_wire_form() {
    for degrees in [0.0, 45.0, 90.0, 180.0, 359.5, -30.0] {
        let mut interner = Interner::new();
        let mut element = Transform2D::empty_element(&mut interner);
        Transform2D {
            rotation: Some(Angle::from_degrees(degrees)),
            ..Transform2D::default()
        }
        .apply(&mut element, &mut interner);

        let read = Transform2D::read(&element, &interner)
            .rotation
            .expect("rotation written");
        assert!(
            (read.degrees() - degrees).abs() < 1e-6,
            "{degrees}° read back as {}°",
            read.degrees()
        );
    }
}

#[test]
fn positions_and_sizes_carry_emu_verbatim() {
    let position = Position::from_emu(-914_400, 0);
    assert_eq!(position.x, Emu::from_emu(-914_400));
    assert_eq!(position.y.emu(), 0);

    let size = Size::from_emu(914_400, 12_700);
    assert_eq!(size.width.points(), 72.0);
    assert_eq!(size.height.points(), 1.0);
}

#[test]
fn reading_a_transform_does_not_alter_it() {
    let source =
        xfrm(r#" rot="60000" flipV="1"><a:off x="1" y="2"/><a:ext cx="3" cy="4"/><a:extLst/>"#);
    let (element, interner) = parse(&source);
    let _ = Transform2D::read(&element, &interner);
    assert_eq!(serialize(element, interner), source);
}

#[test]
fn a_transform_element_is_not_required_to_be_drawingml() {
    // A `p:graphicFrame` holds its transform as `p:xfrm` — its own namespace, the same content.
    let doc = fidelity::parse(
        format!(
            r#"<p:xfrm xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                      xmlns:a="{A}"><a:off x="7" y="8"/><a:ext cx="9" cy="10"/></p:xfrm>"#
        )
        .as_bytes(),
    )
    .expect("fragment parses");

    let transform = Transform2D::read(&doc.root, &doc.interner);
    assert_eq!(transform.position, Some(Position::from_emu(7, 8)));
    assert_eq!(transform.size, Some(Size::from_emu(9, 10)));
}

// ---------------------------------------------------------------------------------------------
// A group's child coordinate space (ECMA-376 Part 1 §L.4.7.4)
//
// A group maps the box its members are laid out in onto the box it occupies in its own parent. The
// tests below take the four operations one at a time — scale, flip, rotate, and the degenerate flat
// child box — then check that the inverse really is one, and that groups compose.
// ---------------------------------------------------------------------------------------------

/// A group whose 4000×2000 child box maps onto a 2000×1000 box at (1000, 500) — a clean 0.5× scale.
fn half_scale_group() -> Transform2D {
    Transform2D {
        position: Some(Position::from_emu(1000, 500)),
        size: Some(Size::from_emu(2000, 1000)),
        child_position: Some(Position::from_emu(4000, 8000)),
        child_size: Some(Size::from_emu(4000, 2000)),
        ..Transform2D::default()
    }
}

#[test]
fn a_group_scales_and_translates_its_child_space() {
    let group = half_scale_group();
    assert_eq!(group.child_scale(), Some((0.5, 0.5)));

    // The child origin lands on the group's own origin.
    assert_eq!(
        group.child_to_parent(Position::from_emu(4000, 8000)),
        Some(Position::from_emu(1000, 500))
    );
    // The far corner of the child box lands on the far corner of the group box.
    assert_eq!(
        group.child_to_parent(Position::from_emu(8000, 10000)),
        Some(Position::from_emu(3000, 1500))
    );
    // And a point halfway across is halfway across.
    assert_eq!(
        group.child_to_parent(Position::from_emu(6000, 9000)),
        Some(Position::from_emu(2000, 1000))
    );
}

#[test]
fn a_flip_mirrors_about_the_group_box_centre() {
    // The group box spans x 1000..3000, so its centre is x = 2000: a point mapping to 1000 mirrors
    // to 3000. A flip is exact and axis-aligned, which is why it can be folded into bounds at all.
    let group = Transform2D {
        flip_horizontal: Some(true),
        ..half_scale_group()
    };
    assert_eq!(
        group.child_to_parent(Position::from_emu(4000, 8000)),
        Some(Position::from_emu(3000, 500))
    );

    let group = Transform2D {
        flip_vertical: Some(true),
        ..half_scale_group()
    };
    // The vertical centre is y = 1000, so 500 mirrors to 1500.
    assert_eq!(
        group.child_to_parent(Position::from_emu(4000, 8000)),
        Some(Position::from_emu(1000, 1500))
    );
}

#[test]
fn a_rotation_turns_clockwise_about_the_group_box_centre() {
    // §L.4.7.3.2: the y axis points down, so a positive `rot` is clockwise. The child origin maps to
    // the group box's top-left (1000, 500), which stands left-and-up of the box centre (2000, 1000).
    // A clockwise quarter-turn sends *left* to *up* and *up* to *right*, so it lands at (2500, 0) —
    // outside the box, as a quarter-turned wide rectangle must be.
    let group = Transform2D {
        rotation: Some(Angle::from_degrees(90.0)),
        ..half_scale_group()
    };
    assert_eq!(
        group.child_to_parent(Position::from_emu(4000, 8000)),
        Some(Position::from_emu(2500, 0))
    );
    // A half-turn is the same as flipping both ways: the top-left corner becomes the bottom-right.
    let group = Transform2D {
        rotation: Some(Angle::from_degrees(180.0)),
        ..half_scale_group()
    };
    assert_eq!(
        group.child_to_parent(Position::from_emu(4000, 8000)),
        Some(Position::from_emu(3000, 1500))
    );
}

#[test]
fn a_flat_child_box_is_not_scaled_on_that_axis() {
    // §L.4.7.3.1: a zero extent means that axis's scaling is skipped, not that the mapping fails —
    // a group around a horizontal line has a flat child box and still places its members.
    let group = Transform2D {
        child_size: Some(Size::from_emu(4000, 0)),
        ..half_scale_group()
    };
    assert_eq!(group.child_scale(), Some((0.5, 1.0)));
    assert_eq!(
        group.child_to_parent(Position::from_emu(6000, 8000)),
        Some(Position::from_emu(2000, 500))
    );
}

#[test]
fn a_group_that_states_no_child_box_maps_nothing() {
    // Without `a:chOff` / `a:chExt` the mapping is defined only implicitly, so it is refused rather
    // than guessed — a wrong rectangle is worse than no rectangle.
    let group = Transform2D {
        child_size: None,
        ..half_scale_group()
    };
    assert_eq!(group.child_scale(), None);
    assert_eq!(group.child_to_parent(Position::from_emu(4000, 8000)), None);

    let group = Transform2D {
        child_position: None,
        ..half_scale_group()
    };
    assert_eq!(group.child_to_parent(Position::from_emu(4000, 8000)), None);

    // A shape that is not a group at all states neither, and maps nothing.
    assert_eq!(
        Transform2D::default().child_to_parent(Position::from_emu(0, 0)),
        None
    );
}

#[test]
fn a_group_of_zero_extent_places_nothing() {
    // Every member would collapse onto one point, and the mapping could never be inverted.
    let group = Transform2D {
        size: Some(Size::from_emu(0, 1000)),
        ..half_scale_group()
    };
    assert_eq!(group.child_to_parent(Position::from_emu(4000, 8000)), None);
    assert_eq!(group.parent_to_child(Position::from_emu(1000, 500)), None);
}

#[test]
fn mapping_out_and_back_returns_the_same_point() {
    // Exact for a scale that divides evenly, which is what makes `set_shape_bounds(shape_bounds())`
    // a no-op on a typical group.
    for group in [
        half_scale_group(),
        Transform2D {
            flip_horizontal: Some(true),
            flip_vertical: Some(true),
            ..half_scale_group()
        },
        Transform2D {
            rotation: Some(Angle::from_degrees(90.0)),
            ..half_scale_group()
        },
    ] {
        for point in [
            Position::from_emu(4000, 8000),
            Position::from_emu(6000, 9000),
            Position::from_emu(8000, 10000),
        ] {
            let mapped = group.child_to_parent(point).expect("maps out");
            assert_eq!(
                group.parent_to_child(mapped),
                Some(point),
                "{point:?} did not survive the round trip"
            );
        }
    }
}

#[test]
fn nested_groups_compose_one_rung_at_a_time() {
    // The inner group's own box lives in the outer group's child space, so mapping a member's point
    // out through the inner group and then the outer one is what places it on the slide. Each rung
    // halves, so the pair quarters.
    let outer = half_scale_group();
    let inner = Transform2D {
        position: Some(Position::from_emu(4000, 8000)),
        size: Some(Size::from_emu(4000, 2000)),
        child_position: Some(Position::from_emu(0, 0)),
        child_size: Some(Size::from_emu(8000, 4000)),
        ..Transform2D::default()
    };
    assert_eq!(inner.child_scale(), Some((0.5, 0.5)));

    let in_outer_child_space = inner
        .child_to_parent(Position::from_emu(8000, 4000))
        .expect("inner maps");
    assert_eq!(in_outer_child_space, Position::from_emu(8000, 10000));
    assert_eq!(
        outer.child_to_parent(in_outer_child_space),
        Some(Position::from_emu(3000, 1500))
    );
}
