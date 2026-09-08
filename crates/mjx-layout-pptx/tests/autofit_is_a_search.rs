//! Autofit, proved the only way it can be: **on text that overflows**.
//!
//! # The trap this suite exists to avoid
//!
//! A shape whose text happens to fit is laid out identically by a correct autofit implementation
//! and by no implementation at all. Every fixture here therefore overflows its box by a wide
//! margin, so the search is forced to run, and every assertion is about **the number it arrived
//! at** rather than about "the text fits" — which would also be true of a renderer that clipped.
//!
//! And the search is proved to be what makes the difference by switching it off:
//! [`AutofitPolicy::Disabled`] lays the same shape out and the two disagree.
//!
//! # ⚠ The computed number is not parity with PowerPoint
//!
//! `a:normAutofit@fontScale` is a value PowerPoint computed. Honouring a stored one is exact;
//! computing one is running PowerPoint's own search, which has never been specified. These tests
//! assert **this crate's** search is deterministic, monotonic and reaches a value on the ladder
//! PowerPoint writes — not that Office would have chosen the same rung. That is a question for the
//! Windows sitting, and [`AutofitOutcome::recomputed`] is the flag that keeps the two apart.

mod support;

use mjx_dml::{Emu, Fraction, TextAutofit, TextBodyPropertiesSpec};
use mjx_layout::{BoxModel, PageIndex, SourcePath};
use mjx_layout_pptx::{constraints_for, AutofitPolicy, SlideBoxModel, SlideDeck};
use mjx_pptx::{Presentation, ShapeBounds};

use support::{blank_deck, resolver, text_box};

/// Enough text that a 2 × 0.4 inch box cannot hold it at any size the ladder reaches quickly.
const OVERFLOWING: &str = "The quick brown fox jumps over the lazy dog, and then jumps back over \
                           it again, and again, and again, until the box is quite full.";

/// A deck with one overflowing text box whose body states `autofit`.
fn overflowing_deck(autofit: TextAutofit) -> (Presentation, usize, usize) {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        OVERFLOWING,
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 0.4),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_autofit(autofit),
    )
    .expect("the body geometry lands");
    (deck, slide, shape)
}

/// The autofit outcome of the shape at `shape` on slide `slide`.
fn outcome(
    policy: AutofitPolicy,
    deck: &mut Presentation,
    slide: usize,
    shape: usize,
) -> mjx_layout_pptx::AutofitOutcome {
    let read = SlideDeck::read(deck).expect("read");
    let mut model = SlideBoxModel::new(resolver()).with_autofit(policy);
    model
        .layout_page(
            &read,
            PageIndex::new(u32::try_from(slide).expect("small")),
            &constraints_for(&read),
            None,
        )
        .expect("lays out");
    let path = SourcePath::new(&[
        u32::try_from(slide).expect("small"),
        u32::try_from(shape).expect("small"),
    ]);
    model
        .catalogue()
        .autofit(&path)
        .expect("the shape has a text body, so it has an outcome")
}

#[test]
fn overflowing_text_makes_the_search_run_and_it_reports_a_number() {
    let (mut deck, slide, shape) = overflowing_deck(TextAutofit::Normal {
        font_scale: None,
        line_space_reduction: None,
    });
    let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);

    assert!(
        found.recomputed,
        "the stored scale did not fit, so one was computed"
    );
    assert!(
        found.font_scale < 1.0,
        "the text overflowed at full size, so the scale is below one: {}",
        found.font_scale
    );
    assert!(
        mjx_layout_pptx::autofit::FONT_SCALES.contains(&found.font_scale),
        "{} is not a rung of the ladder PowerPoint writes",
        found.font_scale
    );
    assert!(found.scales_text());
}

#[test]
fn the_search_is_what_makes_the_difference() {
    // The proof the trap asks for: the same shape, laid out with the search off, comes out
    // differently. A gate that only asserted "the text fits" would pass with the search deleted.
    let (mut deck, slide, shape) = overflowing_deck(TextAutofit::Normal {
        font_scale: None,
        line_space_reduction: None,
    });
    let searched = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
    let unsearched = outcome(AutofitPolicy::HonourOnly, &mut deck, slide, shape);
    let disabled = outcome(AutofitPolicy::Disabled, &mut deck, slide, shape);

    assert_ne!(
        searched.font_scale, unsearched.font_scale,
        "switching the search off must change the answer"
    );
    assert_eq!(unsearched.font_scale, 1.0, "nothing was stored to honour");
    assert_eq!(disabled.font_scale, 1.0);
    assert!(!unsearched.recomputed && !disabled.recomputed);
}

#[test]
fn a_stored_scale_is_honoured_exactly_and_is_not_reported_as_computed() {
    // The half that *is* exact: PowerPoint wrote the number, so applying it reproduces what the
    // author saw. A shape whose stored scale already makes the text fit must not be searched again.
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        "A short line.",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 1.0),
    );
    deck.set_body_properties(
        slide,
        shape,
        &TextBodyPropertiesSpec::new().with_autofit(TextAutofit::Normal {
            font_scale: Some(Fraction::from_ratio(0.625)),
            line_space_reduction: Some(Fraction::from_ratio(0.2)),
        }),
    )
    .expect("the body geometry lands");

    let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
    assert!((found.font_scale - 0.625).abs() < 1e-12);
    assert!((found.line_space_reduction - 0.2).abs() < 1e-12);
    assert!(
        !found.recomputed,
        "a scale read from the file is PowerPoint's answer, not ours"
    );
}

#[test]
fn a_stored_scale_that_still_overflows_is_searched_from_rather_than_replaced() {
    let (mut deck, slide, shape) = overflowing_deck(TextAutofit::Normal {
        font_scale: Some(Fraction::from_ratio(0.85)),
        line_space_reduction: None,
    });
    let honoured = outcome(AutofitPolicy::HonourOnly, &mut deck, slide, shape);
    let searched = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);

    assert!((honoured.font_scale - 0.85).abs() < 1e-12);
    assert!(honoured.recomputed.eq(&false));
    assert!(
        searched.font_scale < honoured.font_scale,
        "the search continued down from the stored scale: {} against {}",
        searched.font_scale,
        honoured.font_scale
    );
    assert!(searched.recomputed);
}

#[test]
fn an_autofit_that_recomputes_lands_on_a_rung_of_the_ladder_even_from_a_stored_scale() {
    // The correctness half of the search, and the one an obvious implementation gets wrong: a
    // recomputed scale is **absolute**, so a stored `0.85` that still overflows becomes another rung
    // — never `0.85 × 0.70`, which is a number PowerPoint does not write and so is guaranteed to
    // disagree with Office. The stored scale says where the search *starts*, and nothing else.
    for stored in [0.925_f64, 0.85, 0.70, 0.50] {
        let (mut deck, slide, shape) = overflowing_deck(TextAutofit::Normal {
            font_scale: Some(Fraction::from_ratio(stored)),
            line_space_reduction: None,
        });
        let searched = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
        assert!(searched.recomputed, "stored {stored} still overflows");
        assert!(
            mjx_layout_pptx::autofit::FONT_SCALES.contains(&searched.font_scale),
            "a stored {stored} produced {}, which is not a rung PowerPoint writes",
            searched.font_scale
        );
        assert!(
            searched.font_scale <= stored,
            "the search never goes back up past a scale that already overflowed: {} from {stored}",
            searched.font_scale
        );
    }
}

#[test]
fn the_scale_a_search_finds_shrinks_as_the_box_does() {
    // Monotonicity over four *distinct* box heights, and the identity-value instrument applied to
    // autofit: a search that always answered the bottom rung would pass a single-fixture test and
    // fail this one, and so would one that ran but never read the box.
    //
    // The heights are chosen so that each one overflows at full size and settles at a *different*
    // rung — the text is one line of prose in a four-inch box, so halving the height roughly halves
    // the area and moves the answer two or three rungs down the ladder.
    const ONE_LINE: &str = "The quick brown fox jumps over the lazy dog and runs away again.";

    let mut previous = f64::INFINITY;
    let mut distinct = std::collections::BTreeSet::new();
    for hundredths in [160_i32, 110, 70, 45] {
        let (mut deck, slide) = blank_deck();
        let height = f64::from(hundredths) / 100.0;
        let shape = text_box(
            &mut deck,
            slide,
            ONE_LINE,
            ShapeBounds::from_inches(1.0, 1.0, 4.0, height),
        );
        deck.set_body_properties(
            slide,
            shape,
            &TextBodyPropertiesSpec::new().with_autofit(TextAutofit::Normal {
                font_scale: None,
                line_space_reduction: None,
            }),
        )
        .expect("the body geometry lands");
        let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
        assert!(
            found.recomputed,
            "the text overflows a {height}-inch box at full size, so the search must run"
        );
        assert!(
            found.font_scale <= previous,
            "a shorter box must not need a larger font: {} at {height} inches after {previous}",
            found.font_scale
        );
        previous = found.font_scale;
        distinct.insert(found.font_scale.to_bits());
    }
    assert!(
        distinct.len() >= 3,
        "four box heights that produce fewer than three scales prove little about the search"
    );
}

#[test]
fn a_no_autofit_body_is_left_to_overflow() {
    let (mut deck, slide, shape) = overflowing_deck(TextAutofit::None);
    let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
    assert_eq!(found.font_scale, 1.0, "`a:noAutofit` means exactly that");
    assert!(!found.recomputed);
}

#[test]
fn a_shape_autofit_body_reports_the_height_its_text_needs() {
    let (mut deck, slide, shape) = overflowing_deck(TextAutofit::Shape);
    let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
    assert_eq!(
        found.font_scale, 1.0,
        "`a:spAutoFit` grows the shape rather than shrinking the text"
    );
    let grown = found.grown_height.expect("a height to grow to");
    assert!(
        grown > Emu::from_inches(0.4),
        "the text needs more than the 0.4 inch the shape states: {} EMU",
        grown.emu()
    );
}

#[test]
fn a_body_with_no_autofit_at_all_reports_no_growth_and_no_scale() {
    let (mut deck, slide) = blank_deck();
    let shape = text_box(
        &mut deck,
        slide,
        OVERFLOWING,
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 0.4),
    );
    let found = outcome(AutofitPolicy::HonourAndRecompute, &mut deck, slide, shape);
    assert_eq!(found.font_scale, 1.0);
    assert_eq!(found.grown_height, None);
    assert!(!found.scales_text());
}
