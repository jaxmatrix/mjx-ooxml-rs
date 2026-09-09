//! A hairline that halves to nothing draws nothing, everywhere, with no error — and no test reaches
//! the path unless one is written for it.
//!
//! # The defect, from `mjx-layout-xlsx`
//!
//! A border sits **astride** the edge it belongs to, so the geometry a painter strokes is the box's
//! rectangle moved in by half the stroke. A stroke floored at one EMU halves to **zero**: the paint
//! is issued, the rectangle is right, and the line is invisible. `mjx-layout-xlsx`'s
//! `border::half_of` was written for exactly that, and Word has six paragraph borders and a `bar`
//! tab that is nothing but a rule — so it is the same arithmetic on a second format, and this is the
//! suite that holds it.
//!
//! Every case below fails on the obvious implementation (`width / 2`).

use mjx_layout::LayoutRect;
use mjx_layout_docx::{border_width, half_of, stroke_rect, ParagraphDecoration, Rule, HAIRLINE};
use mjx_ooxml_core::measure::Emu;

#[test]
fn half_of_one_emu_is_not_nothing() {
    assert_eq!(
        half_of(Emu::from_emu(1)),
        Emu::from_emu(1),
        "a visible stroke's half must be visible"
    );
    assert_eq!(half_of(Emu::from_emu(2)), Emu::from_emu(1));
    assert_eq!(half_of(Emu::from_emu(3)), Emu::from_emu(1));
    assert_eq!(half_of(Emu::from_emu(4)), Emu::from_emu(2));
}

#[test]
fn half_of_nothing_is_nothing() {
    assert_eq!(half_of(Emu::ZERO), Emu::ZERO, "no stroke, no half");
    assert_eq!(
        half_of(Emu::from_emu(-5)),
        Emu::ZERO,
        "a negative width is not a stroke either"
    );
}

/// `w:sz="0"` means *the thinnest line the renderer can draw*, not *no line* — the absence of a line
/// is `w:val="none"`, which is a different attribute. A zero read as a zero-width stroke silently
/// drops a border a reader can see in Word.
#[test]
fn a_zero_width_border_is_a_hairline_and_not_an_absence() {
    assert_eq!(border_width(Some(&0)), HAIRLINE);
    assert_eq!(border_width(None), HAIRLINE);
    assert!(HAIRLINE > Emu::ZERO, "and a hairline is visible");
}

/// A border's stated width is honoured above the hairline.
#[test]
fn a_stated_border_width_is_eighths_of_a_point() {
    // `w:sz="8"` is one point.
    assert_eq!(border_width(Some(&8)), Emu::from_points(1.0));
    // `w:sz="2"` is a quarter point, which is the hairline itself.
    assert_eq!(border_width(Some(&2)), Emu::from_points(0.25));
}

/// The whole point of [`half_of`]: a one-EMU rule's stroke rectangle must still be *inside* the box
/// it decorates. Under `width / 2` it would be the box itself, and the stroke would be drawn half
/// outside it.
#[test]
fn a_one_emu_rule_still_insets_the_stroke_rectangle() {
    let rect = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(1.0),
        Emu::from_inches(1.0),
    );
    let decoration = ParagraphDecoration {
        top: Some(hairline_rule(Emu::from_emu(1))),
        left: Some(hairline_rule(Emu::from_emu(1))),
        bottom: Some(hairline_rule(Emu::from_emu(1))),
        right: Some(hairline_rule(Emu::from_emu(1))),
        ..ParagraphDecoration::default()
    };
    let stroked = stroke_rect(rect, &decoration);
    assert_eq!(stroked.left, Emu::from_emu(1));
    assert_eq!(stroked.top, Emu::from_emu(1));
    assert_ne!(
        stroked, rect,
        "a one-EMU rule that inset nothing is a stroke drawn half outside its own box"
    );
}

/// An undecorated paragraph is inset by nothing at all, which is the identity this rule must not
/// disturb.
#[test]
fn a_paragraph_with_no_rules_is_not_inset() {
    let rect = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_inches(1.0),
        Emu::from_inches(1.0),
    );
    assert_eq!(stroke_rect(rect, &ParagraphDecoration::default()), rect);
}

/// A decoration that paints nothing gets no handle at all, and two paragraphs that paint the same
/// thing share one.
#[test]
fn the_decoration_table_is_shared_and_skips_the_undecorated() {
    use mjx_layout_docx::DecorationCatalogue;
    let mut catalogue = DecorationCatalogue::new();
    assert_eq!(catalogue.intern(ParagraphDecoration::default()), None);
    assert!(catalogue.is_empty());

    let decorated = ParagraphDecoration {
        top: Some(hairline_rule(Emu::from_points(1.0))),
        ..ParagraphDecoration::default()
    };
    let first = catalogue.intern(decorated.clone());
    let second = catalogue.intern(decorated);
    assert_eq!(first, second, "the same decoration is one handle");
    assert_eq!(catalogue.len(), 1);
    assert!(catalogue.get(first.expect("a handle")).is_some());
}

fn hairline_rule(width: Emu) -> Rule {
    Rule {
        width,
        space: Emu::ZERO,
        border: mjx_docx::EffectiveBorder {
            style: mjx_ooxml_types::wordprocessingml::BorderStyle::Single,
            color: mjx_docx::EffectiveColor::Auto,
            width_eighths_of_a_point: None,
            spacing_points: 0,
            shadow: None,
            frame: None,
        },
    }
}
