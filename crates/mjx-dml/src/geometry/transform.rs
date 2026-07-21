//! The 2-D transform that places a shape: `a:xfrm` (`CT_Transform2D` / `CT_GroupTransform2D`).
//!
//! Where a shape's *geometry* says what outline it draws, its transform says **where that outline
//! lands and which way up**: an offset (`a:off`), an extent (`a:ext`), a rotation (`@rot`) and two
//! mirror flags (`@flipH` / `@flipV`).
//!
//! ```xml
//! <a:xfrm rot="2700000" flipH="1">
//!   <a:off x="914400" y="914400"/>
//!   <a:ext cx="3657600" cy="1828800"/>
//! </a:xfrm>
//! ```
//!
//! [`Transform2D`] models both schema types: a **group**'s transform (`CT_GroupTransform2D`, in
//! `p:grpSpPr`) adds `a:chOff` / `a:chExt`, the child coordinate space its members are laid out in,
//! and those two fields are simply always `None` on any other shape.
//!
//! # Why every field is optional
//!
//! Absent and zero are different answers, and collapsing them would break the two things this type
//! exists for:
//!
//! - **Inheritance.** A placeholder that declares no `a:xfrm` takes its position from the layout, and
//!   then the master. A transform read as "at the origin, zero-sized" could never be told from one
//!   that says nothing and means *ask my layout* — see `Presentation::effective_shape_bounds`.
//! - **Editing without destroying.** [`Transform2D::apply`] writes only the fields a caller names,
//!   in place, so moving a group does not discard the `a:chOff` that keeps its members where they
//!   are, and an unset field means *leave it alone* rather than *clear it*.
//!
//! The wire defaults, for a caller resolving a value it did not find: `rot="0"`, `flipH="false"`,
//! `flipV="false"`. `a:off` and `a:ext` have no defaults — a shape with neither has no position of
//! its own at all.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement};
use mjx_ooxml_types::support::on_off;

use crate::build::{
    angle_to_wire, attr_angle, attr_bool, attr_emu, dml_attr, dml_child, dml_child_mut,
    dml_element, replace_or_insert_child, set_attr,
};
use crate::geometry::{Angle, Emu};

/// A point in the coordinate space its holder is laid out in (`CT_Point2D` — `a:off`, `a:chOff`).
///
/// Both coordinates may be **negative**: a shape is allowed to sit partly (or wholly) off the slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// The horizontal coordinate (`@x`, `ST_Coordinate`).
    pub x: Emu,
    /// The vertical coordinate (`@y`, `ST_Coordinate`).
    pub y: Emu,
}

impl Position {
    /// A position from raw EMU coordinates.
    #[must_use]
    pub const fn from_emu(x: i64, y: i64) -> Self {
        Self {
            x: Emu::from_emu(x),
            y: Emu::from_emu(y),
        }
    }
}

/// A width and a height (`CT_PositiveSize2D` — `a:ext`, `a:chExt`).
///
/// The schema types both as `ST_PositiveCoordinate`, so a negative extent is invalid; this type does
/// not enforce it, because a file in the wild is read as it is written, not as it should be — a shape
/// is mirrored with `flip_horizontal`, never with a negative width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    /// The width (`@cx`, `ST_PositiveCoordinate`).
    pub width: Emu,
    /// The height (`@cy`, `ST_PositiveCoordinate`).
    pub height: Emu,
}

impl Size {
    /// A size from raw EMU extents.
    #[must_use]
    pub const fn from_emu(width: i64, height: i64) -> Self {
        Self {
            width: Emu::from_emu(width),
            height: Emu::from_emu(height),
        }
    }
}

/// A shape's 2-D transform — `a:xfrm` as both `CT_Transform2D` and `CT_GroupTransform2D`.
///
/// Every field is `Option`, and `None` means the attribute or child element is **absent** rather
/// than zero — the distinction that lets a shape saying *I am at the origin* be told from one saying
/// *ask my layout where I go*, which is what makes the inheritance walk decidable, and what lets an
/// edit name one field without clearing the rest.
///
/// Not `Eq`/`Hash`: [`Angle`] is a floating-point value, as it is everywhere else in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Transform2D {
    /// The top-left corner of the shape (`a:off`).
    pub position: Option<Position>,
    /// The width and height of the shape (`a:ext`).
    pub size: Option<Size>,
    /// Clockwise rotation about the shape's centre (`@rot`; wire default `0`).
    pub rotation: Option<Angle>,
    /// Whether the shape is mirrored left-to-right (`@flipH`; wire default `false`).
    pub flip_horizontal: Option<bool>,
    /// Whether the shape is mirrored top-to-bottom (`@flipV`; wire default `false`).
    pub flip_vertical: Option<bool>,
    /// **Groups only** — the origin of the coordinate space the group's members are placed in
    /// (`a:chOff`, `CT_GroupTransform2D`). A group maps that child space onto its own
    /// `position`/`size`, which is how one group can be moved without touching any member.
    pub child_position: Option<Position>,
    /// **Groups only** — the extent of the child coordinate space (`a:chExt`).
    pub child_size: Option<Size>,
}

impl Transform2D {
    /// Reads an `a:xfrm` (or the `p:xfrm` of a `p:graphicFrame` — the wrapper's own namespace
    /// differs, its `a:off` / `a:ext` children do not).
    ///
    /// Never fails: an attribute that is absent, malformed, or not a number reads as `None`, because
    /// a transform this model cannot parse must still leave the file readable.
    #[must_use]
    pub fn read(element: &RawElement, interner: &Interner) -> Self {
        Self {
            position: read_point(element, interner, "off"),
            size: read_extent(element, interner, "ext"),
            rotation: attr_angle(&element.attributes, interner, "rot"),
            flip_horizontal: attr_bool(&element.attributes, interner, "flipH"),
            flip_vertical: attr_bool(&element.attributes, interner, "flipV"),
            child_position: read_point(element, interner, "chOff"),
            child_size: read_extent(element, interner, "chExt"),
        }
    }

    /// Whether this transform names nothing at all — the state a shape that declares no `a:xfrm`
    /// resolves to, and the signal to keep walking the inheritance chain.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Writes the fields this transform **names** onto an existing `a:xfrm`, in place; a field left
    /// `None` is not touched.
    ///
    /// In-place rather than rebuilt, because an `a:xfrm` carries content this model does not
    /// describe — a group's `a:chOff` / `a:chExt` when only the position is being changed, an
    /// `extLst`, unknown attributes on the `a:off` itself — and a wholesale rebuild would drop it.
    /// A child element that does not exist yet is inserted at its rank in the schema's sequence
    /// (`off` → `ext` → `chOff` → `chExt`), since order is validity here, not style.
    ///
    /// Applying to a freshly built empty `a:xfrm` is how a shape that had no transform gets one.
    pub fn apply(&self, element: &mut RawElement, interner: &mut Interner) {
        if let Some(position) = self.position {
            write_point(element, interner, "off", position);
        }
        if let Some(size) = self.size {
            write_extent(element, interner, "ext", size);
        }
        if let Some(position) = self.child_position {
            write_point(element, interner, "chOff", position);
        }
        if let Some(size) = self.child_size {
            write_extent(element, interner, "chExt", size);
        }
        if let Some(rotation) = self.rotation {
            set_attr(
                &mut element.attributes,
                interner,
                "rot",
                &angle_to_wire(rotation),
            );
        }
        if let Some(flip) = self.flip_horizontal {
            set_attr(
                &mut element.attributes,
                interner,
                "flipH",
                on_off::to_wire(flip),
            );
        }
        if let Some(flip) = self.flip_vertical {
            set_attr(
                &mut element.attributes,
                interner,
                "flipV",
                on_off::to_wire(flip),
            );
        }
        // A transform that gained a child can no longer serialize as `<a:xfrm/>`.
        element.empty = element.empty && element.children.is_empty();
    }

    /// Builds a fresh, empty `a:xfrm` element — the slot [`apply`](Self::apply) fills for a shape
    /// that had no transform at all.
    #[must_use]
    pub fn empty_element(interner: &mut Interner) -> RawElement {
        dml_element(interner, "xfrm", Vec::new(), Vec::new())
    }
}

/// Reads a `CT_Point2D` child (`a:off` / `a:chOff`). Both coordinates are required by the schema, so
/// a child missing either is not a point and reads as `None`.
fn read_point(element: &RawElement, interner: &Interner, local: &str) -> Option<Position> {
    let child = dml_child(&element.children, interner, local)?;
    Some(Position {
        x: attr_emu(&child.attributes, interner, "x")?,
        y: attr_emu(&child.attributes, interner, "y")?,
    })
}

/// Reads a `CT_PositiveSize2D` child (`a:ext` / `a:chExt`). Both extents are required by the schema.
fn read_extent(element: &RawElement, interner: &Interner, local: &str) -> Option<Size> {
    let child = dml_child(&element.children, interner, local)?;
    Some(Size {
        width: attr_emu(&child.attributes, interner, "cx")?,
        height: attr_emu(&child.attributes, interner, "cy")?,
    })
}

/// Writes a `CT_Point2D` child, editing the existing element's attributes in place when there is one.
fn write_point(element: &mut RawElement, interner: &mut Interner, local: &str, point: Position) {
    let x = point.x.emu().to_string();
    let y = point.y.emu().to_string();
    if let Some(child) = dml_child_mut(&mut element.children, interner, local) {
        set_attr(&mut child.attributes, interner, "x", &x);
        set_attr(&mut child.attributes, interner, "y", &y);
        return;
    }
    let attributes = vec![dml_attr(interner, "x", &x), dml_attr(interner, "y", &y)];
    insert_in_order(element, interner, local, attributes);
}

/// Writes a `CT_PositiveSize2D` child, editing the existing element's attributes in place.
fn write_extent(element: &mut RawElement, interner: &mut Interner, local: &str, size: Size) {
    let cx = size.width.emu().to_string();
    let cy = size.height.emu().to_string();
    if let Some(child) = dml_child_mut(&mut element.children, interner, local) {
        set_attr(&mut child.attributes, interner, "cx", &cx);
        set_attr(&mut child.attributes, interner, "cy", &cy);
        return;
    }
    let attributes = vec![dml_attr(interner, "cx", &cx), dml_attr(interner, "cy", &cy)];
    insert_in_order(element, interner, local, attributes);
}

/// Inserts a newly built transform child at its rank in `CT_GroupTransform2D`'s sequence.
fn insert_in_order(
    element: &mut RawElement,
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
) {
    let child = dml_element(interner, local, attributes, Vec::new());
    replace_or_insert_child(
        &mut element.children,
        interner,
        child,
        |candidate| candidate == local,
        child_rank,
    );
}

/// A transform child's position in `CT_GroupTransform2D`'s `xsd:sequence`. `CT_Transform2D` is the
/// same sequence without its last two members, so one ranking serves both.
fn child_rank(local: &str) -> Option<usize> {
    match local {
        "off" => Some(0),
        "ext" => Some(1),
        "chOff" => Some(2),
        "chExt" => Some(3),
        _ => None,
    }
}
