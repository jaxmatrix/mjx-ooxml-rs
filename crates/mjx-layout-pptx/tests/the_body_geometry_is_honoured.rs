//! Insets, anchoring, columns and wrap — each with a fixture that would look wrong if the attribute
//! were ignored.
//!
//! # The identity-value instrument, applied here
//!
//! A test whose every shape is a landscape box with the default insets, the default anchor, one
//! column and wrapping on exercises **one value** of four parameters, and would pass with all four
//! deleted. So every case below is run against a value that is *not* the default, and the assertion
//! is that the answer moved — never merely that a number is plausible.
//!
//! The anchor is the sharpest of them: five anchors × three alignments is the "nine positions" the
//! ticket asks for, and the two axes are asserted **independently**, because a bug that swapped them
//! would satisfy a test that only ever checked the corners.

mod support;

use mjx_dml::{
    Emu, ParagraphPropertiesSpec, TextAlignment, TextAnchoring, TextBodyPropertiesSpec,
    TextDirection, TextWrapping,
};
use mjx_layout::{Fragment, FragmentTree, LayoutRect, TransformId};
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, lay_out, model, text_box};

/// A deck with one text box whose body geometry is `geometry`.
fn deck_with(
    text: &str,
    bounds: ShapeBounds,
    geometry: &TextBodyPropertiesSpec,
) -> (Presentation, usize, usize) {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(&mut deck, slide, text, bounds);
    deck.set_body_properties(slide, shape, geometry)
        .expect("the body geometry lands");
    (deck, slide, shape)
}

/// The union of every line fragment's rectangle — the block of text, wherever it ended up.
fn text_block(tree: &FragmentTree) -> LayoutRect {
    tree.nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::Line(_)))
        .fold(LayoutRect::ZERO, |union, (_, node)| {
            union.union(node.rect())
        })
}

/// Every line fragment's rectangle, in tree order.
fn line_rects(tree: &FragmentTree) -> Vec<LayoutRect> {
    tree.nodes()
        .filter_map(|(_, node)| matches!(node.fragment(), Fragment::Line(_)).then_some(node.rect()))
        .collect()
}

const BOX: fn() -> ShapeBounds = || ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0);

// ---------------------------------------------------------------------------------------------
// Insets
// ---------------------------------------------------------------------------------------------

#[test]
fn a_stated_inset_moves_the_text_and_the_default_is_not_it() {
    let default = {
        let (mut deck, slide, _) = deck_with("Inset", BOX(), &TextBodyPropertiesSpec::new());
        text_block(&lay_out(&mut model(), &mut deck, slide))
    };
    let inset = {
        let (mut deck, slide, _) = deck_with(
            "Inset",
            BOX(),
            &TextBodyPropertiesSpec::new().with_insets(
                Emu::from_inches(0.5),
                Emu::from_inches(0.25),
                Emu::from_inches(0.5),
                Emu::from_inches(0.25),
            ),
        );
        text_block(&lay_out(&mut model(), &mut deck, slide))
    };

    assert!(
        inset.left > default.left,
        "a wider left inset moves the text right: {} against {}",
        inset.left.emu(),
        default.left.emu()
    );
    assert!(
        inset.top > default.top,
        "and a taller top inset moves it down"
    );
    // The default is 0.1 inch horizontally and 0.05 inch vertically, so the two must differ by
    // exactly the difference between the stated value and the default.
    assert_eq!(
        inset.left - default.left,
        Emu::from_inches(0.5) - Emu::from_emu(91_440)
    );
    assert_eq!(
        inset.top - default.top,
        Emu::from_inches(0.25) - Emu::from_emu(45_720)
    );
}

#[test]
fn a_zero_inset_is_honoured_rather_than_read_as_unstated() {
    // The whole point of `None` meaning *unstated*: an authored `0` must not be read as the schema's
    // 0.1-inch default. The paragraph's own `marL` is zeroed too, so the only thing between the
    // shape's edge and the text is the inset under test.
    let (mut deck, slide) = blank_deck();
    let bounds = BOX();
    let shape = text_box(&mut deck, slide, "Flush", bounds);
    deck.set_paragraph_properties(
        slide,
        shape,
        0,
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(0.0)
            .with_indent_points(0.0)
            .without_bullet(),
    )
    .expect("the paragraph properties land");
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_insets(Emu::ZERO, Emu::ZERO, Emu::ZERO, Emu::ZERO),
    )
    .expect("the body geometry lands");

    let block = text_block(&lay_out(&mut model(), &mut deck, slide));
    assert_eq!(block.left, Emu::from_emu(bounds.offset_x_emu));
    assert_eq!(block.top, Emu::from_emu(bounds.offset_y_emu));
}

// ---------------------------------------------------------------------------------------------
// Anchoring — the vertical axis
// ---------------------------------------------------------------------------------------------

#[test]
fn the_three_vertical_anchors_put_the_block_in_three_places() {
    let mut tops = Vec::new();
    for anchor in [
        TextAnchoring::Top,
        TextAnchoring::Center,
        TextAnchoring::Bottom,
    ] {
        let (mut deck, slide, _) = deck_with(
            "Anchored",
            BOX(),
            &TextBodyPropertiesSpec::new().with_anchor(anchor),
        );
        tops.push(text_block(&lay_out(&mut model(), &mut deck, slide)).top);
    }
    assert!(
        tops[0] < tops[1] && tops[1] < tops[2],
        "top, centre and bottom must be three distinct places: {tops:?}"
    );
}

#[test]
fn a_bottom_anchored_block_ends_at_the_content_boxs_bottom() {
    let (mut deck, slide, _) = deck_with(
        "Bottom",
        BOX(),
        &TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Bottom),
    );
    let block = text_block(&lay_out(&mut model(), &mut deck, slide));
    let bounds = BOX();
    let content_bottom =
        Emu::from_emu(bounds.offset_y_emu + bounds.height_emu) - Emu::from_emu(45_720);
    // Within one line's leading: the block is flush with the bottom inset.
    assert!(
        (content_bottom - block.bottom).emu().abs() < 100_000,
        "{} against {}",
        block.bottom.emu(),
        content_bottom.emu()
    );
}

#[test]
fn the_justified_and_distributed_anchors_spread_the_lines_rather_than_stacking_them() {
    // Three lines in a tall box: `t` stacks them at the top, `just` spreads the slack between them,
    // and `dist` also puts some above the first. A renderer that treated all five anchors as `t`
    // would pass every corner test and fail this one.
    let text = "One two three four five six seven eight nine ten eleven twelve thirteen";
    // Wide enough that the lines fit with room to spare: with no slack there is nothing to spread,
    // and the three anchors would agree for a reason that has nothing to do with the anchor.
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 4.0);
    let mut spans = Vec::new();
    for anchor in [
        TextAnchoring::Top,
        TextAnchoring::Justified,
        TextAnchoring::Distributed,
    ] {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new().with_anchor(anchor),
        );
        let tree = lay_out(&mut model(), &mut deck, slide);
        let rects = line_rects(&tree);
        assert!(rects.len() >= 3, "the fixture wraps to several lines");
        spans.push((rects[0].top, rects[rects.len() - 1].bottom));
    }
    let (top_first, top_last) = spans[0];
    let (just_first, just_last) = spans[1];
    let (dist_first, dist_last) = spans[2];

    assert!(
        just_last > top_last,
        "`just` spreads the lines further down the box than `t` stacks them"
    );
    assert_eq!(just_first, top_first, "`just` starts at the top like `t`");
    assert!(
        dist_first > top_first,
        "`dist` also leaves room above the first line"
    );
    assert!(dist_last > top_last);
}

// ---------------------------------------------------------------------------------------------
// Anchoring — the horizontal axis
// ---------------------------------------------------------------------------------------------

#[test]
fn the_three_alignments_put_a_line_in_three_places() {
    let mut lefts = Vec::new();
    for alignment in [
        TextAlignment::Left,
        TextAlignment::Center,
        TextAlignment::Right,
    ] {
        let (mut deck, slide) = blank_deck();
        let shape = text_box(&mut deck, slide, "Aligned", BOX());
        deck.set_paragraph_properties(
            slide,
            shape,
            0,
            &ParagraphPropertiesSpec::new().with_alignment(alignment),
        )
        .expect("the alignment lands");
        lefts.push(text_block(&lay_out(&mut model(), &mut deck, slide)).left);
    }
    assert!(
        lefts[0] < lefts[1] && lefts[1] < lefts[2],
        "left, centre and right must be three distinct places: {lefts:?}"
    );
}

#[test]
fn anchor_centre_moves_the_whole_block_and_not_each_line() {
    // The distinction `@anchorCtr` exists for: every line moves by the *same* amount, so two lines
    // of different lengths still start at the same place. Centring each line instead would give
    // them two different left edges.
    let text = "A much longer first line of text and a short one";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0);
    let (mut deck, slide, _) = deck_with(
        text,
        bounds,
        &TextBodyPropertiesSpec::new().with_anchor_centered(true),
    );
    let tree = lay_out(&mut model(), &mut deck, slide);
    let rects = line_rects(&tree);
    assert!(rects.len() >= 2, "the fixture wraps");
    let first = rects[0].left;
    for rect in &rects[1..] {
        assert_eq!(rect.left, first, "every line moved by the same amount");
    }

    let plain = {
        let (mut deck, slide, _) = deck_with(text, bounds, &TextBodyPropertiesSpec::new());
        line_rects(&lay_out(&mut model(), &mut deck, slide))[0].left
    };
    assert!(
        first > plain,
        "the block moved right: {} against {}",
        first.emu(),
        plain.emu()
    );
}

// ---------------------------------------------------------------------------------------------
// Columns
// ---------------------------------------------------------------------------------------------

#[test]
fn two_columns_pour_the_overflow_into_the_second_rather_than_off_the_bottom() {
    let text = "One two three four five six seven eight nine ten eleven twelve thirteen \
                fourteen fifteen sixteen seventeen eighteen";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.2);
    let one = {
        let (mut deck, slide, _) = deck_with(text, bounds, &TextBodyPropertiesSpec::new());
        line_rects(&lay_out(&mut model(), &mut deck, slide))
    };
    let two = {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new()
                .with_columns(2)
                .with_column_space(Emu::from_inches(0.2)),
        );
        line_rects(&lay_out(&mut model(), &mut deck, slide))
    };

    let one_lefts: std::collections::BTreeSet<i64> =
        one.iter().map(|rect| rect.left.emu()).collect();
    let two_lefts: std::collections::BTreeSet<i64> =
        two.iter().map(|rect| rect.left.emu()).collect();
    assert_eq!(
        one_lefts.len(),
        1,
        "one column puts every line at one left edge"
    );
    assert!(
        two_lefts.len() >= 2,
        "two columns put lines at two left edges: {two_lefts:?}"
    );

    // And the second column starts back at the top rather than continuing down.
    let column_boundary = two
        .windows(2)
        .find(|pair| pair[1].top < pair[0].top)
        .expect("some line starts a new column");
    assert!(column_boundary[1].left > column_boundary[0].left);
}

#[test]
fn right_to_left_columns_fill_the_rightmost_one_first() {
    let text = "One two three four five six seven eight nine ten eleven twelve thirteen \
                fourteen fifteen sixteen seventeen eighteen";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.2);
    let geometry = TextBodyPropertiesSpec::new()
        .with_columns(2)
        .with_column_space(Emu::from_inches(0.2));
    let left_to_right = {
        let (mut deck, slide, _) = deck_with(text, bounds, &geometry);
        line_rects(&lay_out(&mut model(), &mut deck, slide))[0].left
    };
    let right_to_left = {
        let (mut deck, slide, _) =
            deck_with(text, bounds, &geometry.with_right_to_left_columns(true));
        line_rects(&lay_out(&mut model(), &mut deck, slide))[0].left
    };
    assert!(
        right_to_left > left_to_right,
        "the first line of an `rtlCol` body is in the rightmost column: {} against {}",
        right_to_left.emu(),
        left_to_right.emu()
    );
}

// ---------------------------------------------------------------------------------------------
// Wrap
// ---------------------------------------------------------------------------------------------

#[test]
fn wrap_none_leaves_a_long_line_long() {
    let text = "One two three four five six seven eight nine ten eleven twelve";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 1.5, 2.0);
    let wrapped = {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new().with_wrap(TextWrapping::Square),
        );
        line_rects(&lay_out(&mut model(), &mut deck, slide))
    };
    let unwrapped = {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new().with_wrap(TextWrapping::None),
        );
        line_rects(&lay_out(&mut model(), &mut deck, slide))
    };

    assert!(wrapped.len() > 1, "a narrow box wraps");
    assert_eq!(unwrapped.len(), 1, "`wrap=none` does not");
    assert!(
        unwrapped[0].right > Emu::from_emu(bounds.offset_x_emu + bounds.width_emu),
        "and the one line runs past the shape, which is what `none` means"
    );
}

// ---------------------------------------------------------------------------------------------
// Vertical text
// ---------------------------------------------------------------------------------------------

/// Every line's page-space bounding box, and whether any line is turned.
fn turned_lines(tree: &FragmentTree) -> (Vec<LayoutRect>, bool) {
    let mut bounds = Vec::new();
    let mut turned = false;
    for (id, node) in tree.nodes() {
        if !matches!(node.fragment(), Fragment::Line(_)) {
            continue;
        }
        turned |= node.transform() != TransformId::IDENTITY;
        if let Some(page) = tree.page_bounds(id) {
            bounds.push(page);
        }
    }
    (bounds, turned)
}

#[test]
fn vertical_text_is_broken_to_the_shapes_height_and_then_turned() {
    // A wide, short shape: horizontally the text fits on few lines, and vertically it must break to
    // the *height* instead — which is the half a renderer that turned finished lines would get
    // wrong, and the half that is visible.
    let text = "One two three four five six seven eight";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 5.0, 1.2);

    let horizontal = {
        let (mut deck, slide, _) = deck_with(text, bounds, &TextBodyPropertiesSpec::new());
        let tree = lay_out(&mut model(), &mut deck, slide);
        turned_lines(&tree)
    };
    let vertical = {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new().with_vertical(TextDirection::Vertical),
        );
        let tree = lay_out(&mut model(), &mut deck, slide);
        turned_lines(&tree)
    };

    assert!(!horizontal.1, "horizontal text is not turned");
    assert!(vertical.1, "`vert` turns the text block");
    assert!(
        vertical.0.len() > horizontal.0.len(),
        "a five-inch-wide shape gives vertical text a 1.2-inch measure, so it breaks more often: \
         {} lines against {}",
        vertical.0.len(),
        horizontal.0.len()
    );

    // Each line now *runs down* the shape rather than across it, which is the whole of what `vert`
    // means: a horizontal line is wider than it is tall and a turned one is not.
    let first_horizontal = horizontal.0.first().expect("a line");
    assert!(first_horizontal.width() > first_horizontal.height());
    let first_vertical = vertical.0.first().expect("a line");
    assert!(
        first_vertical.height() > first_vertical.width(),
        "a turned line is taller than it is wide: {} by {}",
        first_vertical.width().emu(),
        first_vertical.height().emu()
    );

    // And the lines stack **right to left**, which is what tells `vert` from `vert270`.
    if let [first, second, ..] = vertical.0.as_slice() {
        assert!(
            second.left < first.left,
            "`vert` stacks its lines right to left: {} then {}",
            first.left.emu(),
            second.left.emu()
        );
    }
}

#[test]
fn the_two_vertical_directions_turn_opposite_ways() {
    let text = "One two three four";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0);
    let mut firsts = Vec::new();
    for vertical in [TextDirection::Vertical, TextDirection::Vertical270] {
        let (mut deck, slide, _) = deck_with(
            text,
            bounds,
            &TextBodyPropertiesSpec::new().with_vertical(vertical),
        );
        let tree = lay_out(&mut model(), &mut deck, slide);
        let (lines, turned) = turned_lines(&tree);
        assert!(turned, "{vertical:?} is a turn");
        firsts.push(*lines.first().expect("a line"));
    }
    assert_ne!(
        firsts[0], firsts[1],
        "`vert` and `vert270` are opposite turns, not the same one"
    );
}

#[test]
fn an_east_asian_vertical_body_is_laid_out_horizontally_rather_than_wrongly_turned() {
    // A stated limitation asserted as one: `eaVert` stacks upright glyphs down a column, which needs
    // vertical font metrics this box model does not ask for. Laying it out horizontally is legible
    // and wrong; turning it would be illegible and wrong.
    let text = "One two three four";
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0);
    let (mut deck, slide, _) = deck_with(
        text,
        bounds,
        &TextBodyPropertiesSpec::new().with_vertical(TextDirection::EastAsianVertical),
    );
    let tree = lay_out(&mut model(), &mut deck, slide);
    assert!(
        !turned_lines(&tree).1,
        "`eaVert` is not a rotation, so nothing is turned"
    );
}
