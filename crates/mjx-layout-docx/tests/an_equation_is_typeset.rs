//! **OMML layout, asserted as geometry** — a nested fraction's two bar positions, a growing
//! delimiter's height, the script scale, and an equation array's alignment axis.
//!
//! # Why "the equation rendered" is not a gate
//!
//! An equation is a tree of boxes, and almost everything true about it is a *relationship* between
//! two of them. A renderer that produced one box of roughly the right size for the whole expression
//! would satisfy every assertion of the form *there is a fragment*, would look plausible in a
//! thumbnail, and would be wrong about every one of those relationships.
//!
//! So each test here reads two numbers and asserts the relation between them, and each says what a
//! wrong implementation would produce instead. The numbers themselves are `EngineDerived` — there is
//! no `MATH` table in `mjx-text`, so the constants are TeX's and MathML Core's rather than Word's,
//! and `crate::math` says so at every one. **The relations are not:** a bar on an axis, a script at a
//! stated fraction of its base's size and a delimiter as tall as what it encloses are what MathML
//! and the OpenType `MATH` specification define, and they are what a reader sees.
//!
//! MJX-LEDGER-LIMITATION: `mjx-text` parses no OpenType `MATH` table, so a stretchy delimiter is
//! *scaled* rather than assembled from glyph variants and its stroke weight grows with its height.

mod support;

use mjx_layout_docx::{MathBox, MathContent, MathContext, PlacedMathBox};
use mjx_ooxml_core::measure::Emu;
use mjx_text::FontSize;
use support::generated::{
    math_argument, math_delimiter, math_equation_array, math_fraction, math_paragraph, math_run,
    math_superscript,
};
use support::{document, flow, model};

/// Lays one equation out, at twelve points in the bundled face.
///
/// The paragraph carries a **run** before the equation, and that is not decoration: an equation
/// inherits the run style of the paragraph it sits in, so a paragraph holding nothing but an
/// equation is set at `ASSUMED_FONT_SIZE_POINTS` and every number below would be a sixth smaller
/// than the test says. The equation's own box tree does not include that run.
fn typeset(markup: &str) -> MathBox {
    let paragraphs = vec![math_paragraph("x", markup)];
    let mut document = document(&paragraphs);
    let read = flow(&mut document);
    let mut model = model();
    let request = mjx_layout_docx::LayoutRequest {
        width: Emu::from_inches(6.5),
        top: Emu::ZERO,
        exclusions: &[],
    };
    let block = model
        .lay_out_block(&read, 0, request)
        .expect("the block lays out");
    let composition = &block.as_paragraph().expect("a paragraph").composition;
    composition
        .equations()
        .first()
        .cloned()
        .expect("the paragraph carries one equation")
}

/// Every rule in a box tree, as `(distance above the tree's own baseline, width)`.
///
/// A fraction bar is a rule, so this is how a bar's position is read — and it is read *relative to
/// the outermost baseline*, which is what makes a nested fraction's two bars two different numbers.
fn rules(node: &MathBox, baseline: Emu, into: &mut Vec<(Emu, Emu)>) {
    if node.content == MathContent::Rule {
        into.push((Emu::ZERO - baseline, node.width));
    }
    for child in &node.children {
        rules(&child.content, baseline + child.baseline, into);
    }
}

/// Every rule in `node`, deepest last.
fn rule_positions(node: &MathBox) -> Vec<(Emu, Emu)> {
    let mut found = Vec::new();
    rules(node, Emu::ZERO, &mut found);
    found
}

/// Every glyph-bearing box in a tree, with its own x and baseline offset.
fn glyphs(node: &MathBox, x: Emu, baseline: Emu, into: &mut Vec<(Emu, Emu, Emu)>) {
    if matches!(node.content, MathContent::Glyphs { .. }) {
        into.push((x, baseline, node.ascent));
    }
    for child in &node.children {
        glyphs(&child.content, x + child.x, baseline + child.baseline, into);
    }
}

/// Every glyph box, in tree order.
fn glyph_boxes(node: &MathBox) -> Vec<(Emu, Emu, Emu)> {
    let mut found = Vec::new();
    glyphs(node, Emu::ZERO, Emu::ZERO, &mut found);
    found
}

/// The axis height of a twelve-point context — where a fraction bar belongs.
fn axis_at(points: f64) -> Emu {
    Emu::from_points(points * mjx_layout_docx::AXIS_HEIGHT_IN_EMS)
}

#[test]
fn a_fractions_bar_sits_on_the_axis() {
    // The single relationship a fraction is: the bar is on the **axis**, not on the baseline and not
    // at the middle of the total height. An implementation that centred the bar on the stack's own
    // extent would put it somewhere plausible and nowhere principled.
    let equation = typeset(&math_fraction(&math_run("1"), &math_run("2")));
    let bars = rule_positions(&equation);
    assert_eq!(bars.len(), 1, "one fraction, one bar");
    let (above_baseline, width) = bars[0];
    assert_eq!(
        above_baseline,
        axis_at(12.0),
        "the bar's own baseline is the axis"
    );
    assert!(width > Emu::ZERO, "and it spans the wider of the two parts");
}

#[test]
fn a_nested_fractions_two_bars_are_at_two_different_heights() {
    // The assertion the ticket names. The inner fraction sits in the outer one's numerator, which is
    // raised — so its bar is *higher* than the outer bar by the numerator's own offset. An
    // implementation that placed every bar at the same height (on the baseline, say) produces two
    // equal numbers and fails here.
    let inner = math_fraction(&math_run("1"), &math_run("2"));
    let equation = typeset(&math_fraction(&inner, &math_run("3")));
    let bars = rule_positions(&equation);
    assert_eq!(bars.len(), 2, "an outer bar and an inner one");
    let outer = bars
        .iter()
        .map(|(height, _)| *height)
        .min()
        .expect("two bars");
    let nested = bars
        .iter()
        .map(|(height, _)| *height)
        .max()
        .expect("two bars");
    assert_eq!(
        outer,
        axis_at(12.0),
        "the outer bar is still on the outer axis"
    );
    assert!(
        nested > outer,
        "and the nested one is higher: {nested:?} against {outer:?}"
    );
}

#[test]
fn a_growing_delimiter_is_taller_than_one_that_does_not_grow() {
    // A parenthesis around a fraction has to reach it; one around a letter does not. This asserts
    // the *same* delimiter around the *same* content, growing and not — so the only difference is
    // `m:grow` and the assertion cannot be satisfied by the content's own height.
    let content = math_fraction(&math_run("1"), &math_run("2"));
    let growing = typeset(&math_delimiter(&content, true));
    let fixed = typeset(&math_delimiter(&content, false));
    let tallest = |equation: &MathBox| -> Emu {
        glyph_boxes(equation)
            .into_iter()
            .map(|(_, _, ascent)| ascent)
            .max()
            .unwrap_or(Emu::ZERO)
    };
    assert!(
        tallest(&growing) > tallest(&fixed),
        "the growing delimiter reaches its content: {:?} against {:?}",
        tallest(&growing),
        tallest(&fixed)
    );
    assert!(
        growing.height() >= fixed.height(),
        "and the whole expression is at least as tall"
    );
}

#[test]
fn a_delimiter_that_does_not_grow_is_body_sized() {
    // The control for the test above: with `m:grow` off, the delimiter is the size every other run
    // is, so a difference in the growing case cannot have come from the delimiter being special.
    let equation = typeset(&math_delimiter(&math_run("x"), false));
    let heights: Vec<Emu> = glyph_boxes(&equation)
        .into_iter()
        .map(|(_, _, ascent)| ascent)
        .collect();
    assert!(
        heights.windows(2).all(|pair| pair[0] == pair[1]),
        "every glyph is the same size: {heights:?}"
    );
}

#[test]
fn a_superscript_is_set_at_the_stated_fraction_of_its_base() {
    // The relative-sizing assertion. `SCRIPT_SCALE` is the OpenType `MATH` table's own documented
    // default, and an implementation that set a script at full size — or at a size it invented —
    // produces a different ratio.
    let equation = typeset(&math_superscript(&math_run("x"), &math_run("2")));
    let boxes = glyph_boxes(&equation);
    assert_eq!(boxes.len(), 2, "a base and a script");
    let base_ascent = boxes[0].2;
    let script_ascent = boxes[1].2;
    let expected = base_ascent.scaled_by(mjx_layout_docx::SCRIPT_SCALE);
    assert_eq!(
        script_ascent,
        expected,
        "the script is {:?} of the base's size",
        mjx_layout_docx::SCRIPT_SCALE
    );
    assert!(
        boxes[1].1 < boxes[0].1,
        "and it sits above the base's baseline: {:?} against {:?}",
        boxes[1].1,
        boxes[0].1
    );
}

#[test]
fn the_two_script_scale_downs_are_stated_independently() {
    // A script of a script is `SCRIPT_SCRIPT_SCALE` of the body, **not** `SCRIPT_SCALE` squared —
    // the two are stated separately by the `MATH` table and an implementation that compounded them
    // would set every second-level script four per cent too small.
    let style = mjx_layout_docx::RunStyle {
        range: 0..0,
        family: support::FAMILY.to_owned(),
        size: FontSize::from_points(12.0),
        weight: mjx_text::FontWeight::REGULAR,
        slant: mjx_text::FontSlant::Upright,
        language: None,
        hidden: false,
    };
    let context = MathContext::new(&style);
    let once = context.scripted();
    let twice = once.scripted();
    let close = |left: f64, right: f64| (left - right).abs() < 1e-6;
    assert!(close(
        once.size.in_points(),
        12.0 * mjx_layout_docx::SCRIPT_SCALE
    ));
    assert!(close(
        twice.size.in_points(),
        12.0 * mjx_layout_docx::SCRIPT_SCRIPT_SCALE
    ));
    assert!(
        !close(
            twice.size.in_points(),
            12.0 * mjx_layout_docx::SCRIPT_SCALE * mjx_layout_docx::SCRIPT_SCALE
        ),
        "the compounded answer is the one this refuses"
    );
    let thrice = twice.scripted();
    assert_eq!(
        thrice.size.in_points(),
        twice.size.in_points(),
        "and past the second level the size stops shrinking, so a deep script does not vanish"
    );
}

#[test]
fn an_equation_array_aligns_its_rows_on_one_axis() {
    // The alignment assertion. Two rows of different widths, centred: their left edges differ by
    // exactly half the difference of their widths. A left-aligned implementation puts both at zero
    // and fails; a right-aligned one puts them at the full difference.
    let short = math_run("x=1");
    let long = math_run("xyz=1234");
    let wrapper = typeset(&math_equation_array(&[short, long]));
    // The box `typeset` answers is the equation's own row; the array is its one element.
    let equation = &wrapper
        .children
        .first()
        .expect("the array is the equation's one element")
        .content;
    let offsets: Vec<(Emu, Emu)> = equation
        .children
        .iter()
        .map(|child: &PlacedMathBox| (child.x, child.content.width))
        .collect();
    assert_eq!(offsets.len(), 2, "two rows");
    let (narrow_x, narrow_width) = offsets
        .iter()
        .copied()
        .min_by_key(|(_, width)| *width)
        .expect("two rows");
    let (wide_x, wide_width) = offsets
        .iter()
        .copied()
        .max_by_key(|(_, width)| *width)
        .expect("two rows");
    assert_eq!(
        wide_x,
        Emu::ZERO,
        "the widest row defines the array's width"
    );
    assert_eq!(
        narrow_x,
        mjx_layout_docx::half_of(wide_width - narrow_width),
        "and the narrow one is centred on the same axis"
    );
    assert_ne!(
        narrow_x,
        Emu::ZERO,
        "which a left-aligned implementation could not produce"
    );
}

#[test]
fn a_radicals_rule_covers_its_radicand_and_not_its_sign() {
    // A radical's rule spans what is under it and stops at the sign — the difference between a
    // square root and a strike-through.
    let equation = typeset(&format!(
        "<m:rad>{}{}</m:rad>",
        math_argument("deg", ""),
        math_argument("e", &math_run("xyz"))
    ));
    let bars = rule_positions(&equation);
    assert_eq!(bars.len(), 1, "one radical, one rule");
    let (_, width) = bars[0];
    assert!(
        width > Emu::ZERO && width < equation.width,
        "the rule covers the radicand and not the sign: {width:?} of {:?}",
        equation.width
    );
}

#[test]
fn a_deeply_nested_expression_terminates_rather_than_recursing_for_ever() {
    // A `.docx` is untrusted input and `m:e` nests without limit. This builds an expression far past
    // `MAXIMUM_DEPTH` and asserts it lays out at all — which it can only do by stopping.
    let mut markup = math_run("x");
    for _ in 0..(mjx_layout_docx::MAXIMUM_DEPTH * 2) {
        markup = math_fraction(&markup, &math_run("2"));
    }
    let equation = typeset(&markup);
    assert!(
        equation.width >= Emu::ZERO,
        "it produced a box rather than a stack overflow"
    );
}

#[test]
fn an_equation_is_measured_into_the_line_it_sits_on() {
    // The layout half, and the reason an equation is a `crate::generated::InlineObject` at all: the
    // line that carries it is **wider** than the same line without it. Before MJXOFF-177 an inline
    // object contributed no advance, so this difference was zero.
    let with_equation = math_paragraph(
        "before",
        &math_fraction(&math_run("123456"), &math_run("789012")),
    );
    let without = math_paragraph("before", "");
    let width_of = |markup: &str| -> Emu {
        let mut document = document(&[markup.to_owned()]);
        let read = flow(&mut document);
        let mut model = model();
        let request = mjx_layout_docx::LayoutRequest {
            width: Emu::from_inches(6.5),
            top: Emu::ZERO,
            exclusions: &[],
        };
        let block = model
            .lay_out_block(&read, 0, request)
            .expect("the block lays out");
        let paragraph = block.as_paragraph().expect("a paragraph");
        paragraph
            .lines
            .first()
            .map(|line| {
                line.placement
                    .segments
                    .last()
                    .map_or(Emu::ZERO, |placed| placed.x + placed.width)
            })
            .unwrap_or(Emu::ZERO)
    };
    let wide = width_of(&with_equation);
    let narrow = width_of(&without);
    assert!(
        wide > narrow,
        "the equation reserves width on the line: {wide:?} against {narrow:?}"
    );
}

#[test]
fn an_equation_raises_the_line_it_sits_on() {
    // And the vertical half: a two-storey fraction is taller than a run of text, so the line it is
    // on is taller. `crate::float::inline_height` computed exactly this for a picture and was never
    // called; an equation reaches it through the composition.
    let height_of = |markup: &str| -> Emu {
        let mut document = document(&[markup.to_owned()]);
        let read = flow(&mut document);
        let mut model = model();
        let request = mjx_layout_docx::LayoutRequest {
            width: Emu::from_inches(6.5),
            top: Emu::ZERO,
            exclusions: &[],
        };
        let block = model
            .lay_out_block(&read, 0, request)
            .expect("the block lays out");
        block
            .as_paragraph()
            .expect("a paragraph")
            .lines
            .first()
            .map_or(Emu::ZERO, |line| line.height)
    };
    let tall = height_of(&math_paragraph(
        "before",
        &math_fraction(&math_run("1"), &math_run("2")),
    ));
    let ordinary = height_of(&math_paragraph("before", ""));
    assert!(
        tall > ordinary,
        "a fraction is taller than a line of text: {tall:?} against {ordinary:?}"
    );
}

#[test]
fn an_equations_boxes_reach_the_fragment_tree() {
    // The whole point of laying an equation out: something draws it. This asserts the tree holds
    // more than the one box the object itself is — the glyphs and the bar are in it.
    let paragraphs = vec![math_paragraph(
        "before",
        &math_fraction(&math_run("1"), &math_run("2")),
    )];
    let tree = support::one_page(&paragraphs, &support::constraints(6.5, 9.0));
    let boxes = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), mjx_layout::Fragment::Box(_)))
        .count();
    let runs = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), mjx_layout::Fragment::GlyphRun(_)))
        .count();
    assert!(
        boxes >= 3,
        "the paragraph, the object and the fraction's own bar are all boxes: {boxes}"
    );
    assert!(
        runs >= 3,
        "and the surrounding text, the numerator and the denominator are glyph runs: {runs}"
    );
}
