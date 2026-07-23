//! Where a shape addressed by a [`ShapePath`] actually sits on the slide.
//!
//! A top-level shape states its own answer: its `a:off` / `a:ext` are slide coordinates. A **group
//! member** does not — its transform is written in the coordinate space its group lays members out
//! in (`a:chOff` / `a:chExt`), and every enclosing group maps that space onto the box it occupies in
//! *its* parent. Reaching a slide rectangle means walking that ladder.
//!
//! [`Transform2D`] carries one rung ([`child_to_parent`](Transform2D::child_to_parent) and its
//! inverse); this module walks the whole ladder for an address, in both directions:
//!
//! - [`compose`] takes what a shape states and returns where it is, in slide EMU;
//! - [`to_child_space`] takes a slide rectangle and returns what the shape must state to sit there.
//!
//! Both follow ECMA-376 Part 1 §L.4.7.4, which is not the naive "apply each transform in turn": a
//! nested shape is scaled and flipped by the **product** of its ancestors' factors, rotated by their
//! **sum**, and translated so its **centre** lands where the whole chain — rotations included — puts
//! it. So the size is a product of scale factors and the position is derived from the mapped centre,
//! never from the mapped corner.
//!
//! A path of length 1 has no ancestors, and every function here is then the identity: **nothing
//! about a top-level shape changes.**

use mjx_dml::{Position, Size, Transform2D};
use mjx_ooxml_core::{Interner, RawElement};

use crate::address::ShapePath;
use crate::geometry::ShapeBounds;
use crate::slide;

/// The transforms of the groups enclosing `path`, **outermost first**.
///
/// A step that is not a group, or a group stating no transform, contributes `None` — which makes the
/// whole address unplaceable, since a rung with no mapping breaks the ladder.
fn ancestors(
    sp_tree: &RawElement,
    interner: &Interner,
    path: &ShapePath,
) -> Option<Vec<Transform2D>> {
    let indices = path.indices();
    // The last index addresses the shape itself; everything before it is an enclosing group.
    let (_, ancestor_indices) = indices.split_last()?;
    let mut transforms = Vec::with_capacity(ancestor_indices.len());
    for depth in 1..=ancestor_indices.len() {
        let group = slide::resolve_shape(
            sp_tree,
            interner,
            &ShapePath::from(&ancestor_indices[..depth]),
        )
        .ok()?;
        let element = slide::shape_transform(group, interner)?;
        transforms.push(Transform2D::read(element, interner));
    }
    Some(transforms)
}

/// Where the shape at `path` sits on the slide, given the transform it states.
///
/// `own` is what the shape itself declares — its `a:off` / `a:ext` in whatever space it lives in.
/// The returned transform is in **slide coordinates**: `position` and `size` are the shape's
/// axis-aligned box before its own rotation (exactly what they already mean for a top-level shape),
/// `rotation` is the sum of its own and every ancestor's, and the flips are their exclusive or.
///
/// A group's `child_position` / `child_size` pass through untouched: they describe the space its own
/// members live in, which does not move when the group does.
///
/// `None` when the shape states no position or size of its own, or when an enclosing group states no
/// usable mapping — a rectangle that cannot be computed is not reported as one that can.
pub(crate) fn compose(
    sp_tree: &RawElement,
    interner: &Interner,
    path: &ShapePath,
    own: &Transform2D,
) -> Option<Transform2D> {
    let ancestors = ancestors(sp_tree, interner, path)?;
    if ancestors.is_empty() {
        return Some(*own); // A top-level shape already answers in slide coordinates.
    }
    let offset = own.position?;
    let extent = own.size?;

    // Size: the product of every ancestor's scale, applied along the *shape's* own axes (§L.4.7.4
    // steps 1-2) — so a rotated shape inside a scaled group is unaffected by the rotation here.
    let (mut scale_x, mut scale_y) = (1.0_f64, 1.0_f64);
    for group in &ancestors {
        let (x, y) = group.child_scale()?;
        scale_x *= x;
        scale_y *= y;
    }
    let size = Size::from_emu(
        scale_length(extent.width.emu(), scale_x),
        scale_length(extent.height.emu(), scale_y),
    );

    // Position: map the shape's centre out through each group, innermost first (§L.4.7.4 step 4),
    // then step back by half the composed size. The centre is what the spec places, because a
    // rotated ancestor moves a corner and a centre differently.
    let mut centre = Position::from_emu(
        offset.x.emu() + extent.width.emu() / 2,
        offset.y.emu() + extent.height.emu() / 2,
    );
    for group in ancestors.iter().rev() {
        centre = group.child_to_parent(centre)?;
    }

    Some(Transform2D {
        position: Some(Position::from_emu(
            centre.x.emu() - size.width.emu() / 2,
            centre.y.emu() - size.height.emu() / 2,
        )),
        size: Some(size),
        rotation: compose_rotation(own, &ancestors),
        flip_horizontal: compose_flip(own.flip_horizontal, &ancestors, |g| g.flip_horizontal),
        flip_vertical: compose_flip(own.flip_vertical, &ancestors, |g| g.flip_vertical),
        child_position: own.child_position,
        child_size: own.child_size,
    })
}

/// What the shape at `path` must state for its slide rectangle to be `absolute` — the inverse of
/// [`compose`], and what placing a member at a slide coordinate needs.
///
/// Rounds to whole EMU at each rung, so the round trip is exact whenever the composed scale is (a
/// group at half size, say) and within a few EMU — millionths of an inch — when it is not.
///
/// `None` under the same conditions as [`compose`].
pub(crate) fn to_child_space(
    sp_tree: &RawElement,
    interner: &Interner,
    path: &ShapePath,
    absolute: ShapeBounds,
) -> Option<ShapeBounds> {
    let ancestors = ancestors(sp_tree, interner, path)?;
    if ancestors.is_empty() {
        return Some(absolute); // A top-level shape states slide coordinates directly.
    }

    let (mut scale_x, mut scale_y) = (1.0_f64, 1.0_f64);
    for group in &ancestors {
        let (x, y) = group.child_scale()?;
        scale_x *= x;
        scale_y *= y;
    }
    let width = scale_length(absolute.width_emu, 1.0 / scale_x);
    let height = scale_length(absolute.height_emu, 1.0 / scale_y);

    // The absolute centre, mapped back in through each group — outermost first this time.
    let mut centre = Position::from_emu(
        absolute.offset_x_emu + absolute.width_emu / 2,
        absolute.offset_y_emu + absolute.height_emu / 2,
    );
    for group in &ancestors {
        centre = group.parent_to_child(centre)?;
    }

    Some(ShapeBounds {
        offset_x_emu: centre.x.emu() - width / 2,
        offset_y_emu: centre.y.emu() - height / 2,
        width_emu: width,
        height_emu: height,
    })
}

/// Multiplies a length by a scale factor, rounding to whole EMU and saturating rather than wrapping.
fn scale_length(length: i64, scale: f64) -> i64 {
    let scaled = (length as f64 * scale).round();
    if scaled.is_nan() {
        0
    } else if scaled >= i64::MAX as f64 {
        i64::MAX
    } else if scaled <= i64::MIN as f64 {
        i64::MIN
    } else {
        scaled as i64
    }
}

/// The shape's own rotation plus every ancestor's (§L.4.7.4 step 3), or `None` when neither the
/// shape nor any ancestor states one — an unstated rotation stays unstated.
fn compose_rotation(own: &Transform2D, ancestors: &[Transform2D]) -> Option<mjx_dml::Angle> {
    let stated = own.rotation.is_some() || ancestors.iter().any(|group| group.rotation.is_some());
    if !stated {
        return None;
    }
    let radians = own.rotation.map_or(0.0, |angle| angle.radians())
        + ancestors
            .iter()
            .filter_map(|group| group.rotation)
            .map(|angle| angle.radians())
            .sum::<f64>();
    Some(mjx_dml::Angle::from_radians(radians))
}

/// The shape's own flip combined with every ancestor's — mirroring twice is not mirroring, so the
/// flags compose by exclusive or. `None` when nothing in the chain states this flip.
fn compose_flip(
    own: Option<bool>,
    ancestors: &[Transform2D],
    field: fn(&Transform2D) -> Option<bool>,
) -> Option<bool> {
    let stated = own.is_some() || ancestors.iter().any(|group| field(group).is_some());
    if !stated {
        return None;
    }
    let flipped = own.unwrap_or(false)
        ^ ancestors
            .iter()
            .filter_map(field)
            .fold(false, |acc, flip| acc ^ flip);
    Some(flipped)
}
