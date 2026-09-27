//! A two-level group whose child sits in the wrong place under the single-application defect.
//!
//! # The defect, exactly
//!
//! A `p:grpSp` states two rectangles: `a:xfrm/a:ext`, the box it occupies on the slide, and
//! `a:xfrm/a:chExt` with `a:chOff`, the coordinate space its members' own `a:off` and `a:ext` are
//! measured in. When the two differ, everything inside the group is scaled by their ratio.
//!
//! The classic defect is applying that ratio **once** — at the shape, from its nearest enclosing
//! group — rather than composing it down the whole chain. On a one-level group the two are the same
//! answer, so a suite that only nests once cannot tell them apart. This one nests **twice**, with a
//! different ratio at each level, so the right answer (2 × 3 = 6) and the wrong one (3) are
//! different numbers.
//!
//! # Why this is asserted here and not left to `mjx-pptx`
//!
//! `mjx-pptx` composes the chain in `effective_shape_bounds`, and this crate consumes that rather
//! than re-deriving it — which is exactly why the assertion belongs here as well. The property the
//! renderer depends on is *"a fragment's rectangle is the shape's place on the slide"*, and it
//! would break just as completely if this crate started reading `a:xfrm` itself, or applied its own
//! transform on top of bounds that already carried one. Nothing about the tree would say so; a
//! nested shape would simply be somewhere else.

mod support;

use mjx_layout::{Fragment, FragmentTree, LayoutRect};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds};

/// EMU per inch, so the numbers below can be read as inches.
const INCH: i64 = 914_400;

/// A deck with a shape inside a group inside a group, where each group scales its children.
///
/// The innermost shape starts one inch square. The inner group is then stretched by two and the
/// outer group by three, each by widening its own `a:ext` while leaving the `a:chExt` that
/// `group_shapes` wrote — which is what a person dragging a group's handle does.
fn nested_groups() -> (Presentation, usize, Vec<usize>) {
    let (mut deck, slide) = support::blank_deck();

    // Three shapes: the two that become the inner group, and one more so the outer group has a
    // second member. `group_shapes` refuses a group of one (ECMA-376 §L.4.7.4 calls it degenerate).
    support::text_box(
        &mut deck,
        slide,
        "inner",
        ShapeBounds::new(INCH, INCH, INCH, INCH),
    );
    support::text_box(
        &mut deck,
        slide,
        "sibling",
        ShapeBounds::new(3 * INCH, INCH, INCH, INCH),
    );
    support::text_box(
        &mut deck,
        slide,
        "outer sibling",
        ShapeBounds::new(INCH, 3 * INCH, INCH, INCH),
    );

    let inner_group = deck
        .group_shapes(slide, &[0.into(), 1.into()])
        .expect("the inner group");
    let outer_group = deck
        .group_shapes(slide, &[inner_group.clone(), 1.into()])
        .expect("the outer group");

    // The inner group is now the outer group's first member. Widen each group's own box without
    // touching the child space it was given, so each contributes a scale of its own.
    let inner_path = outer_group.child(0);
    let inner_bounds = deck
        .shape_bounds(slide, inner_path.clone())
        .expect("the inner group's bounds")
        .expect("a group states its own box");
    deck.set_shape_bounds(
        slide,
        inner_path.clone(),
        ShapeBounds::new(
            inner_bounds.offset_x_emu,
            inner_bounds.offset_y_emu,
            inner_bounds.width_emu * 2,
            inner_bounds.height_emu,
        ),
    )
    .expect("the inner group stretches");

    let outer_bounds = deck
        .shape_bounds(slide, outer_group.clone())
        .expect("the outer group's bounds")
        .expect("a group states its own box");
    deck.set_shape_bounds(
        slide,
        outer_group.clone(),
        ShapeBounds::new(
            outer_bounds.offset_x_emu,
            outer_bounds.offset_y_emu,
            outer_bounds.width_emu * 3,
            outer_bounds.height_emu,
        ),
    )
    .expect("the outer group stretches");

    let deepest = inner_path.child(0).indices().to_vec();
    (deck, slide, deepest)
}

/// The rectangle the fragment whose source path is `path` was laid out at.
fn rect_at(tree: &FragmentTree, path: &[u32]) -> LayoutRect {
    tree.nodes()
        .find(|(_, node)| {
            node.source().path().segments() == path
                && matches!(
                    node.fragment(),
                    Fragment::Shape(_) | Fragment::Box(_) | Fragment::Image(_)
                )
        })
        .map(|(_, node)| node.rect())
        .unwrap_or_else(|| panic!("no fragment at path {path:?}"))
}

/// The same shape, ungrouped, so that "before" is measured rather than assumed.
fn unscaled_width() -> Emu {
    let (mut deck, slide) = support::blank_deck();
    support::text_box(
        &mut deck,
        slide,
        "inner",
        ShapeBounds::new(INCH, INCH, INCH, INCH),
    );
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    rect_at(&tree, &[0, 0]).width()
}

#[test]
fn a_shape_two_groups_deep_carries_both_scales() {
    let (mut deck, slide, deepest) = nested_groups();
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);

    let mut path = vec![0_u32];
    path.extend(deepest.iter().map(|&index| index as u32));
    let scaled = rect_at(&tree, &path).width();
    let single = unscaled_width();

    assert_eq!(
        single,
        Emu::from_emu(INCH),
        "the ungrouped shape is one inch wide, which is what the rest of this assertion is relative \
         to"
    );
    assert_eq!(
        scaled,
        Emu::from_emu(INCH * 6),
        "a shape inside a group scaled by two, inside a group scaled by three, is six times as wide \
         as it was. {} EMU is {}x. Three times means the outer group's scale was applied and the \
         inner group's was not — the single-application defect — and one time means neither was.",
        scaled.emu(),
        scaled.emu() as f64 / INCH as f64
    );
}

#[test]
fn the_group_boxes_themselves_are_still_where_the_document_puts_them() {
    // A composition applied to the *groups* as well as to their members would double-count: the
    // outer group's own box is stated on the slide and must not be scaled by anything.
    let (mut deck, slide, _) = nested_groups();
    let expected = deck
        .shape_bounds(slide, 0)
        .expect("the outer group's bounds")
        .expect("a group states its own box");

    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    let outer = rect_at(&tree, &[0, 0]);

    assert_eq!(
        (outer.left.emu(), outer.top.emu()),
        (expected.offset_x_emu, expected.offset_y_emu),
        "the outermost group's own fragment sits where its `a:off` says, in slide coordinates"
    );
    assert_eq!(
        (outer.width().emu(), outer.height().emu()),
        (expected.width_emu, expected.height_emu),
        "and it is the size its `a:ext` says. A group whose own box had been scaled by its own \
         child space would be the ratio out, in a way every member inherits."
    );
}

#[test]
fn every_member_of_a_group_becomes_a_fragment_under_it() {
    // The structural half: composing the transform is no use if a member is lost on the way. A
    // group's members are its *children* in the tree, not siblings of it.
    let (mut deck, slide, deepest) = nested_groups();
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);

    let mut path = vec![0_u32];
    path.extend(deepest.iter().map(|&index| index as u32));
    assert!(
        path.len() >= 4,
        "the deepest shape is two groups down, so its address is at least \
         slide/outer/inner/shape: {path:?}"
    );

    let (deep_id, _) = tree
        .nodes()
        .find(|(_, node)| {
            node.source().path().segments() == path
                && matches!(node.fragment(), Fragment::Shape(_) | Fragment::Box(_))
        })
        .expect("the deepest shape has a fragment");

    // Walk up to the root, collecting the source paths on the way. Each step must be a prefix of
    // the one below it, which is what "the group's box is behind everything inside it" means once
    // it is a tree rather than a flat list.
    let mut ancestors = Vec::new();
    let mut cursor = tree.node(deep_id).and_then(|node| node.parent());
    while let Some(id) = cursor {
        let Some(node) = tree.node(id) else { break };
        ancestors.push(node.source().path().segments().to_vec());
        cursor = node.parent();
    }
    assert!(
        ancestors.len() >= 3,
        "the deepest shape sits under the inner group, the outer group and the page: {ancestors:?}"
    );
    for ancestor in &ancestors {
        assert!(
            path.starts_with(ancestor),
            "{ancestor:?} is an ancestor of {path:?} in the tree but not a prefix of its address, \
             so the tree's shape and the document's disagree"
        );
    }
}
