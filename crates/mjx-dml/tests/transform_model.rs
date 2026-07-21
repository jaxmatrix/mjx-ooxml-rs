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
