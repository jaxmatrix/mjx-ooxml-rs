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

    // -----------------------------------------------------------------------------------------
    // A group's child coordinate space
    //
    // A group maps the box its members are laid out in (`a:chOff` / `a:chExt`) onto the box it
    // occupies in its own parent (`a:off` / `a:ext`), then flips and rotates the result. ECMA-376
    // Part 1 §L.4.7.4 defines the sequence; §L.4.7.3.2 fixes the sign of the rotation (the y axis
    // points down, so a positive `rot` is clockwise and the ordinary rotation matrix applies), and
    // §L.4.7.3.1 the degenerate case (an extent of zero means that axis is not scaled at all).
    //
    // These three answer the question a member cannot answer for itself: where on the slide it is.
    // -----------------------------------------------------------------------------------------

    /// How much this group scales its members, per axis — `ext / chExt`.
    ///
    /// An axis whose `chExt` is zero is **not scaled** (`1.0`) rather than undefined: a child box can
    /// legitimately be flat, "the `cx` attribute of `a:ext` is ignored and the horizontal scaling is
    /// skipped" (ECMA-376 Part 1 §L.4.7.3.1).
    ///
    /// `None` unless the group states both its own extent and its child extent — without them there
    /// is no mapping to describe. The factors are magnitudes; a mirror is
    /// [`flip_horizontal`](Self::flip_horizontal), never a negative scale.
    #[must_use]
    pub fn child_scale(&self) -> Option<(f64, f64)> {
        let extent = self.size?;
        let child = self.child_size?;
        let axis = |extent: Emu, child: Emu| {
            if child.emu() == 0 {
                1.0
            } else {
                extent.emu() as f64 / child.emu() as f64
            }
        };
        Some((
            axis(extent.width, child.width),
            axis(extent.height, child.height),
        ))
    }

    /// Maps a point from this group's **child** coordinate space into the space the group itself sits
    /// in — one rung of the ladder from a member's `a:off` to a slide coordinate.
    ///
    /// The point is placed by the fraction of the child box it stands at, then flipped and rotated
    /// about the centre of the group's own box, exactly as ECMA-376 Part 1 §L.4.7.4 specifies.
    ///
    /// `None` unless the group states all four of `a:off`, `a:ext`, `a:chOff` and `a:chExt`: without
    /// its child box a group's mapping is defined only implicitly (as the union of its members'
    /// boxes), and inventing one would put shapes in the wrong place rather than admit it cannot say.
    #[must_use]
    pub fn child_to_parent(&self, point: Position) -> Option<Position> {
        let (offset, extent, child_offset, scale) = self.mapping()?;
        // Scale about the child origin, into the group's own box.
        let x = offset.x.emu() as f64 + (point.x.emu() - child_offset.x.emu()) as f64 * scale.0;
        let y = offset.y.emu() as f64 + (point.y.emu() - child_offset.y.emu()) as f64 * scale.1;
        let (x, y) = self.flip_and_rotate(offset, extent, (x, y));
        Some(Position::from_emu(round_emu(x), round_emu(y)))
    }

    /// Maps a point from the space this group sits in back into its **child** coordinate space — the
    /// exact inverse of [`child_to_parent`](Self::child_to_parent), which is what placing a member at
    /// a slide coordinate needs.
    ///
    /// Rounds to whole EMU at each rung, so a round trip is exact whenever the scale is (a group at
    /// half size, say) and within a few EMU — a few millionths of an inch — when it is not.
    ///
    /// `None` under the same conditions as [`child_to_parent`](Self::child_to_parent).
    #[must_use]
    pub fn parent_to_child(&self, point: Position) -> Option<Position> {
        let (offset, extent, child_offset, scale) = self.mapping()?;
        // Undo the rotation and the flip about the group box centre, then the scale.
        let (x, y) =
            self.unrotate_and_unflip(offset, extent, (point.x.emu() as f64, point.y.emu() as f64));
        let x = child_offset.x.emu() as f64 + (x - offset.x.emu() as f64) / scale.0;
        let y = child_offset.y.emu() as f64 + (y - offset.y.emu() as f64) / scale.1;
        Some(Position::from_emu(round_emu(x), round_emu(y)))
    }

    /// The four things a child-space mapping needs, or `None` if the group does not state them all.
    fn mapping(&self) -> Option<(Position, Size, Position, (f64, f64))> {
        let scale = self.child_scale()?;
        // A zero scale cannot be inverted, and would collapse every member onto one point going the
        // other way; such a group places nothing.
        if scale.0 == 0.0 || scale.1 == 0.0 {
            return None;
        }
        Some((self.position?, self.size?, self.child_position?, scale))
    }

    /// The centre of the group's own box, which its flip and rotation are about.
    fn box_centre(offset: Position, extent: Size) -> (f64, f64) {
        (
            offset.x.emu() as f64 + extent.width.emu() as f64 / 2.0,
            offset.y.emu() as f64 + extent.height.emu() as f64 / 2.0,
        )
    }

    /// Mirrors then rotates `point` about the group box centre (the order §L.4.7.4 gives).
    fn flip_and_rotate(&self, offset: Position, extent: Size, point: (f64, f64)) -> (f64, f64) {
        let (cx, cy) = Self::box_centre(offset, extent);
        let x = if self.flip_horizontal == Some(true) {
            2.0 * cx - point.0
        } else {
            point.0
        };
        let y = if self.flip_vertical == Some(true) {
            2.0 * cy - point.1
        } else {
            point.1
        };
        match self.rotation {
            Some(rotation) if rotation.radians() != 0.0 => {
                let (sin, cos) = rotation.radians().sin_cos();
                let (dx, dy) = (x - cx, y - cy);
                // The y axis points down, so this is a clockwise rotation (§L.4.7.3.2).
                (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
            }
            _ => (x, y),
        }
    }

    /// Undoes [`flip_and_rotate`](Self::flip_and_rotate): unrotate first, then unmirror.
    fn unrotate_and_unflip(&self, offset: Position, extent: Size, point: (f64, f64)) -> (f64, f64) {
        let (cx, cy) = Self::box_centre(offset, extent);
        let (x, y) = match self.rotation {
            Some(rotation) if rotation.radians() != 0.0 => {
                let (sin, cos) = (-rotation.radians()).sin_cos();
                let (dx, dy) = (point.0 - cx, point.1 - cy);
                (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
            }
            _ => point,
        };
        let x = if self.flip_horizontal == Some(true) {
            2.0 * cx - x
        } else {
            x
        };
        let y = if self.flip_vertical == Some(true) {
            2.0 * cy - y
        } else {
            y
        };
        (x, y)
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

/// Rounds a computed coordinate to whole EMU, saturating rather than wrapping on a value no `i64`
/// can hold — a transform read from a hostile file must not turn arithmetic into nonsense.
fn round_emu(value: f64) -> i64 {
    if value.is_nan() {
        return 0;
    }
    let rounded = value.round();
    if rounded >= i64::MAX as f64 {
        i64::MAX
    } else if rounded <= i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
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
