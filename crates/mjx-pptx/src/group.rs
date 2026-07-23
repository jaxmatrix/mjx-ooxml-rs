//! Structural edits to the shape tree: making a `p:grpSp`, dissolving one, and moving a shape
//! between one and another.
//!
//! Everything else in this crate edits a shape it has found; these four change **which shapes there
//! are and where they sit in the tree**, which is why they live apart. Each works on the part root
//! and its interner alone — no package, no relationships — so the surgery is testable on a shape tree
//! with no `.pptx` around it.
//!
//! # Nothing may move on screen
//!
//! A group maps the space its members are laid out in onto the box it occupies, so a shape that
//! changes parent changes coordinate system. Every operation here therefore reads where the shape
//! *is* ([`placement::compose`]), performs the surgery, and restates the shape for its new home
//! ([`placement::restate`]) — position, size, rotation and mirrors together. A shape moved into a
//! group that is scaled, turned and flipped comes out pixel-identical.
//!
//! Making a group is the one case where nothing has to be converted at all: the new group's child
//! space is set **identical** to its own box, so the mapping is the identity and its members keep the
//! coordinates they already had, exactly.

use mjx_dml::Transform2D;
use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::PML;

use crate::address::ShapePath;
use crate::build;
use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::slide::{self, ShapeKind};
use crate::surface::Surface;
use crate::{nav, placement};

/// Wraps `members` — which must be siblings — in a new `p:grpSp`, returning the new group's address.
///
/// The group's box is the union of the members' own boxes, and its child coordinate space is set
/// identical to it, so the mapping is the identity: **the members keep their stated coordinates
/// exactly** and nothing moves on screen. The group takes the z-order position of the earliest
/// member, and the members keep their relative order inside it.
pub(crate) fn group_shapes(
    root: &mut RawElement,
    interner: &mut Interner,
    surface: Surface,
    members: &[ShapePath],
) -> Result<ShapePath, PptxError> {
    // ECMA-376 Part 1 §L.4.7.4: "A group with zero shapes is degenerate… A group with one shape is
    // also degenerate; it has no representational power beyond that of the one shape."
    if members.len() < 2 {
        return Err(PptxError::GroupNeedsTwoShapes {
            surface,
            count: members.len(),
        });
    }

    // Every member must live in the same container, or there is no single place to put the group.
    let parent = members[0].parent();
    let mut indices: Vec<usize> = Vec::with_capacity(members.len());
    for member in members {
        let (last, container) = member
            .indices()
            .split_last()
            .ok_or_else(|| out_of_range(surface, member, 0))?;
        let container = (!container.is_empty()).then(|| ShapePath::from(container));
        if container != parent {
            return Err(PptxError::ShapesAreNotSiblings {
                surface,
                path: member.clone(),
            });
        }
        if indices.contains(last) {
            return Err(PptxError::ShapesAreNotSiblings {
                surface,
                path: member.clone(),
            });
        }
        indices.push(*last);
    }
    // Document order, so the members keep their z-order inside the group whatever order they were
    // named in, and the group lands where the earliest of them was.
    indices.sort_unstable();

    let sp_tree = slide::sp_tree(root, interner)?;
    let container = match &parent {
        Some(path) => resolve(sp_tree, interner, surface, path)?,
        None => sp_tree,
    };

    // The union of the members' own boxes is what ECMA calls the child bounding box.
    let mut boxes: Vec<ShapeBounds> = Vec::with_capacity(indices.len());
    for index in &indices {
        let shape = slide::shapes(container, interner)
            .nth(*index)
            .ok_or_else(|| out_of_range(surface, &member_path(&parent, *index), *index))?;
        let bounds = slide::shape_transform(shape, interner)
            .map(|element| Transform2D::read(element, interner))
            .as_ref()
            .and_then(ShapeBounds::from_transform)
            .ok_or_else(|| PptxError::ShapeHasNoBounds {
                surface,
                path: member_path(&parent, *index),
            })?;
        boxes.push(bounds);
    }
    // At least two members were checked above, so there is always something to reduce.
    let union =
        boxes
            .into_iter()
            .reduce(ShapeBounds::union)
            .ok_or(PptxError::GroupNeedsTwoShapes {
                surface,
                count: members.len(),
            })?;

    let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
    // The child positions to lift out, and where the group goes — read before anything is removed.
    let mut positions: Vec<usize> = Vec::with_capacity(indices.len());
    for index in &indices {
        positions.push(
            slide::nth_shape_position(container, interner, *index)
                .ok_or_else(|| out_of_range(surface, &member_path(&parent, *index), *index))?,
        );
    }
    let insert_at = positions[0];
    let group_index = indices[0];

    let group = build_group(interner, next_id, union);

    // Lift the members out last-first, so the earlier positions stay valid while doing it.
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let container = match &parent {
        Some(path) => resolve_mut(sp_tree, interner, surface, path)?,
        None => sp_tree,
    };
    let mut lifted: Vec<RawNode> = Vec::with_capacity(positions.len());
    for position in positions.iter().rev() {
        lifted.push(container.children.remove(*position));
    }
    lifted.reverse();

    let mut group = group;
    group.children.extend(lifted);
    container
        .children
        .insert(insert_at, RawNode::Element(group));
    container.empty = false;

    Ok(member_path(&parent, group_index))
}

/// Dissolves the group at `path`, returning where its members now are.
///
/// Each member keeps its absolute placement: the group's mapping is unwound into each member's own
/// transform. The members take the group's place in z-order, in the order they were in it.
pub(crate) fn ungroup(
    root: &mut RawElement,
    interner: &mut Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<Vec<ShapePath>, PptxError> {
    let parent = path.parent();
    let group_index = *path
        .indices()
        .last()
        .ok_or_else(|| out_of_range(surface, path, 0))?;

    // Where every member is now, before the tree changes under them.
    let sp_tree = slide::sp_tree(root, interner)?;
    let group = resolve(sp_tree, interner, surface, path)?;
    if slide::shape_kind(group, interner) != Some(ShapeKind::GroupShape) {
        return Err(PptxError::ShapeIsNotAGroup {
            surface,
            path: path.clone(),
        });
    }
    let member_count = slide::shapes(group, interner).count();
    let mut placements: Vec<Option<Transform2D>> = Vec::with_capacity(member_count);
    for member in 0..member_count {
        let member_path = path.child(member);
        let shape = resolve(sp_tree, interner, surface, &member_path)?;
        // A member that states no transform of its own has nothing to preserve; it keeps saying
        // nothing, and inherits in its new home exactly as it did in the group.
        let own = slide::shape_transform(shape, interner)
            .map(|element| Transform2D::read(element, interner));
        placements
            .push(own.and_then(|own| placement::compose(sp_tree, interner, &member_path, &own)));
    }

    // Lift the group out and splice its members into its place.
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let (container, position) = slide::resolve_shape_position(sp_tree, interner, path)
        .map_err(|count| out_of_range_with(surface, path, count))?;
    let RawNode::Element(mut group) = container.children.remove(position) else {
        return Err(PptxError::MalformedSlide("group is not an element"));
    };
    let members: Vec<RawNode> = group
        .children
        .drain(..)
        .filter(|node| match node {
            RawNode::Element(child) => slide::shape_kind(child, interner).is_some(),
            _ => false,
        })
        .collect();
    for (offset, member) in members.into_iter().enumerate() {
        container.children.insert(position + offset, member);
    }

    // Restate each member for the container it now sits in.
    let mut freed = Vec::with_capacity(member_count);
    for (member, absolute) in placements.into_iter().enumerate() {
        let new_path = member_path(&parent, group_index + member);
        restate_at(root, interner, surface, &new_path, absolute)?;
        freed.push(new_path);
    }
    Ok(freed)
}

/// Moves the shape at `shape` into the group at `group`, as its last member, and returns its new
/// address. The shape keeps its absolute placement.
pub(crate) fn move_into_group(
    root: &mut RawElement,
    interner: &mut Interner,
    surface: Surface,
    shape: &ShapePath,
    group: &ShapePath,
) -> Result<ShapePath, PptxError> {
    // A shape cannot be put inside itself, nor inside anything it contains.
    if group.indices().starts_with(shape.indices()) {
        return Err(PptxError::ShapeCannotContainItself {
            surface,
            path: shape.clone(),
        });
    }

    let sp_tree = slide::sp_tree(root, interner)?;
    let target = resolve(sp_tree, interner, surface, group)?;
    if slide::shape_kind(target, interner) != Some(ShapeKind::GroupShape) {
        return Err(PptxError::ShapeIsNotAGroup {
            surface,
            path: group.clone(),
        });
    }
    let moving = resolve(sp_tree, interner, surface, shape)?;
    let absolute = slide::shape_transform(moving, interner)
        .map(|element| Transform2D::read(element, interner))
        .and_then(|own| placement::compose(sp_tree, interner, shape, &own));

    // Lifting the shape out shifts the group's address if the shape sat before it in the same
    // container — so the destination is recomputed rather than assumed.
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let (container, position) = slide::resolve_shape_position(sp_tree, interner, shape)
        .map_err(|count| out_of_range_with(surface, shape, count))?;
    let node = container.children.remove(position);
    let group = shifted_by_removal(group, shape);

    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let target = resolve_mut(sp_tree, interner, surface, &group)?;
    let index = slide::shapes(target, interner).count();
    // `p:extLst` is last in `CT_GroupShape`, so a new member goes before it.
    let at = ext_lst_position(target, interner).unwrap_or(target.children.len());
    target.children.insert(at, node);
    target.empty = false;

    let new_path = group.child(index);
    restate_at(root, interner, surface, &new_path, absolute)?;
    Ok(new_path)
}

/// Moves the shape at `path` out of the group holding it, into that group's own container, directly
/// after the group in z-order. Returns its new address; the shape keeps its absolute placement.
pub(crate) fn move_out_of_group(
    root: &mut RawElement,
    interner: &mut Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<ShapePath, PptxError> {
    let group = path.parent().ok_or_else(|| PptxError::ShapeHasNoParent {
        surface,
        path: path.clone(),
    })?;
    let grandparent = group.parent();
    let group_index = *group
        .indices()
        .last()
        .ok_or_else(|| out_of_range(surface, &group, 0))?;

    let sp_tree = slide::sp_tree(root, interner)?;
    let moving = resolve(sp_tree, interner, surface, path)?;
    let absolute = slide::shape_transform(moving, interner)
        .map(|element| Transform2D::read(element, interner))
        .and_then(|own| placement::compose(sp_tree, interner, path, &own));

    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let (container, position) = slide::resolve_shape_position(sp_tree, interner, path)
        .map_err(|count| out_of_range_with(surface, path, count))?;
    let node = container.children.remove(position);

    // Straight after the group it came out of, so z-order stays intelligible.
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let (container, group_position) = slide::resolve_shape_position(sp_tree, interner, &group)
        .map_err(|count| out_of_range_with(surface, &group, count))?;
    container.children.insert(group_position + 1, node);

    let new_path = member_path(&grandparent, group_index + 1);
    restate_at(root, interner, surface, &new_path, absolute)?;
    Ok(new_path)
}

// ---------------------------------------------------------------------------------------------
// Shared machinery
// ---------------------------------------------------------------------------------------------

/// Writes the transform the shape at `path` must state to keep the absolute placement it had, if it
/// had one at all.
fn restate_at(
    root: &mut RawElement,
    interner: &mut Interner,
    surface: Surface,
    path: &ShapePath,
    absolute: Option<Transform2D>,
) -> Result<(), PptxError> {
    let Some(absolute) = absolute else {
        return Ok(()); // Stated nothing before; states nothing now.
    };
    let sp_tree = slide::sp_tree(root, interner)?;
    let Some(restated) = placement::restate(sp_tree, interner, path, &absolute) else {
        return Err(PptxError::ShapeCannotBePlaced {
            surface,
            path: path.clone(),
        });
    };
    let sp_tree = slide::sp_tree_mut(root, interner)?;
    let shape = resolve_mut(sp_tree, interner, surface, path)?;
    let slot = slide::shape_transform_slot_mut(shape, interner)?;
    restated.apply(slot, interner);
    Ok(())
}

/// `group`'s address once the shape at `removed` has been lifted out: an earlier sibling of one of
/// the group's ancestors shifts that ancestor's index down by one.
fn shifted_by_removal(group: &ShapePath, removed: &ShapePath) -> ShapePath {
    let removed = removed.indices();
    let mut indices = group.indices().to_vec();
    // The removal only shifts the index at the depth it happened, and only when the group's path
    // agrees with it above that depth and sits after it.
    let depth = removed.len() - 1;
    if indices.len() > depth
        && indices[..depth] == removed[..depth]
        && indices[depth] > removed[depth]
    {
        indices[depth] -= 1;
    }
    ShapePath::from(indices)
}

/// The address of shape `index` inside `parent` (the shape tree itself when `None`).
fn member_path(parent: &Option<ShapePath>, index: usize) -> ShapePath {
    match parent {
        Some(path) => path.child(index),
        None => ShapePath::from(index),
    }
}

/// The child position of a container's `p:extLst`, which the schema puts after every member.
fn ext_lst_position(container: &RawElement, interner: &Interner) -> Option<usize> {
    container.children.iter().position(|node| {
        matches!(node, RawNode::Element(child)
            if nav::name_is(&child.name, interner, PML, "extLst"))
    })
}

fn resolve<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<&'a RawElement, PptxError> {
    slide::resolve_shape(sp_tree, interner, path)
        .map_err(|count| out_of_range_with(surface, path, count))
}

fn resolve_mut<'a>(
    sp_tree: &'a mut RawElement,
    interner: &Interner,
    surface: Surface,
    path: &ShapePath,
) -> Result<&'a mut RawElement, PptxError> {
    slide::resolve_shape_mut(sp_tree, interner, path)
        .map_err(|count| out_of_range_with(surface, path, count))
}

fn out_of_range(surface: Surface, path: &ShapePath, count: usize) -> PptxError {
    out_of_range_with(surface, path, count)
}

fn out_of_range_with(surface: Surface, path: &ShapePath, count: usize) -> PptxError {
    PptxError::ShapeIndexOutOfRange {
        surface,
        path: path.clone(),
        count,
    }
}

/// A whole `p:grpSp`: `nvGrpSpPr` (id, name) + `grpSpPr` whose `a:xfrm` maps a child space
/// **identical** to `bounds` onto `bounds` — the identity, so members keep their own coordinates.
fn build_group(interner: &mut Interner, id: u32, bounds: ShapeBounds) -> RawElement {
    let attributes = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Group {id}")),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", attributes);
    let c_nv_grp_sp_pr = build::leaf(interner, "p", PML, "cNvGrpSpPr", Vec::new());
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    let nv_grp_sp_pr = build::node(
        interner,
        "p",
        PML,
        "nvGrpSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_grp_sp_pr),
            RawNode::Element(nv_pr),
        ],
    );

    // One spelling of an `a:xfrm` in the crate: built and edited transforms go through one writer.
    let mut xfrm = Transform2D::empty_element(interner);
    let transform = Transform2D {
        child_position: bounds.to_transform().position,
        child_size: bounds.to_transform().size,
        ..bounds.to_transform()
    };
    transform.apply(&mut xfrm, interner);
    let grp_sp_pr = build::node(
        interner,
        "p",
        PML,
        "grpSpPr",
        Vec::new(),
        vec![RawNode::Element(xfrm)],
    );

    build::node(
        interner,
        "p",
        PML,
        "grpSp",
        Vec::new(),
        vec![RawNode::Element(nv_grp_sp_pr), RawNode::Element(grp_sp_pr)],
    )
}
