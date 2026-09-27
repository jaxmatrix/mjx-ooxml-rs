//! Speaker notes lay out through the same walk as a slide, and are addressed apart from one.
//!
//! # Why "the same walk" is the whole feature
//!
//! A notes slide is a `p:notesSlide` with a `p:spTree` in it: the same shapes, the same text
//! bodies, the same placeholder inheritance from the notes master. So the cost of laying one out is
//! **not** a second engine, and the way to get it wrong is to write one. Everything here goes
//! through `SlideBoxModel::lay_out_surface`, and the only two things that differ are the
//! [`PartId`](mjx_layout::PartId) the fragments are addressed under and the absence of a
//! continuation.
//!
//! # And why the address is the part that has to be tested
//!
//! `mjx-session` numbers notes slides part **3** and slides part **0** (see [`crate::address`]'s own
//! table). A notes fragment addressed under part 0 would collide with a slide fragment at the same
//! shape index — same path, same range, different document — so an edit journalled against one
//! would invalidate the other, and a hit test on the notes pane would answer with a shape from the
//! slide. Nothing about either tree would look wrong.
//!
//! [`crate::address`]: mjx_layout_pptx::address

mod support;

use mjx_layout::{BoxModel, Fragment, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideDeck, NOTES, SLIDES};

/// The committed fixture that carries speaker notes.
fn notes_deck() -> mjx_pptx::Presentation {
    mjx_pptx::Presentation::open(&support::fixture("notes.pptx")).expect("a well-formed package")
}

#[test]
fn the_fixture_really_has_notes_or_nothing_below_means_anything() {
    let mut deck = notes_deck();
    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    assert!(
        read.notes_count() > 0,
        "`notes.pptx` no longer carries a notes slide, so every assertion in this file would pass \
         on an empty answer"
    );
}

#[test]
fn a_notes_page_lays_out_and_carries_its_own_text() {
    let mut deck = notes_deck();

    // The committed fixture's notes placeholder states no bounds, and neither does the notes
    // master's — so **no tier places it**, and R14's rule applies: a shape nothing places has no
    // position, and this crate draws nothing rather than inventing the origin. That is correct and
    // it is also not what this test is about, so the placeholder is given a box first. Reading the
    // fixture as it stands is `the_unplaced_notes_placeholder_draws_nothing` below.
    deck.set_shape_bounds(
        mjx_pptx::Surface::Notes(0),
        1,
        mjx_pptx::ShapeBounds::from_inches(0.5, 3.0, 6.0, 3.0),
    )
    .expect("the notes body placeholder takes a box");

    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    let mut model = support::model();
    let constraints = constraints_for(&read);

    let page = model
        .layout_notes(&read, 0, &constraints)
        .expect("the notes page lays out")
        .expect("slide 0 has a notes page");
    let (tree, continuation) = page.into_parts();

    assert!(
        continuation.is_none(),
        "a notes page is reached by naming its slide, never by iterating, so it continues onto \
         nothing"
    );

    let glyphs = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .count();
    assert!(
        glyphs > 0,
        "the notes slide holds `Remember to smile.` and none of it was laid out. A notes page with \
         a root box and nothing under it is what a walk that stopped at the page produces."
    );
}

#[test]
fn the_unplaced_notes_placeholder_draws_nothing() {
    // The fixture as committed. A notes placeholder that no tier places is drawn nowhere, exactly
    // as a slide's would be — the rule is the box model's, not the surface's, and this is what
    // proves the notes path did not grow an exception to it.
    let mut deck = notes_deck();
    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    let notes = read.notes(0).expect("slide 0 has a notes page");
    assert!(
        notes.shapes().iter().any(|shape| shape.bounds.is_none()),
        "the fixture's notes placeholder states no bounds at any tier, so at least one of its \
         shapes reads as unplaced; if that has changed, this test is about a different file"
    );

    let mut model = support::model();
    let constraints = constraints_for(&read);
    let tree = model
        .layout_notes(&read, 0, &constraints)
        .expect("the notes page lays out")
        .expect("slide 0 has a notes page")
        .into_parts()
        .0;
    assert_eq!(
        tree.len(),
        1,
        "an unplaced notes placeholder must produce the page's own root and nothing else, which is \
         the same answer `sample.pptx`'s unplaced title gets on a slide"
    );
}

#[test]
fn a_notes_fragment_is_addressed_apart_from_a_slide_fragment() {
    let mut deck = notes_deck();
    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    let mut model = support::model();
    let constraints = constraints_for(&read);

    let notes = model
        .layout_notes(&read, 0, &constraints)
        .expect("the notes page lays out")
        .expect("slide 0 has a notes page")
        .into_parts()
        .0;
    for (_, node) in notes.nodes() {
        assert_eq!(
            node.source().part(),
            NOTES,
            "a notes fragment is addressed under part {} and this one says part {}. Part {} is \
             slides: a notes fragment there shares an address with the slide's shape at the same \
             index, so an edit to one would invalidate the other and a hit test would answer with \
             the wrong document.",
            NOTES.number(),
            node.source().part().number(),
            SLIDES.number()
        );
    }

    let slide = model
        .layout_page(&read, PageIndex::FIRST, &constraints, None)
        .expect("the slide lays out")
        .into_parts()
        .0;
    for (_, node) in slide.nodes() {
        assert_eq!(
            node.source().part(),
            SLIDES,
            "threading the part through the builders must not have moved a *slide's* fragments off \
             part {}",
            SLIDES.number()
        );
    }
}

#[test]
fn a_slide_with_no_notes_answers_none_rather_than_an_empty_page() {
    // *There is no notes page* and *there is a blank one* are different answers to "print the
    // notes", and a caller that could not tell them apart would print a blank sheet per slide.
    let mut deck = mjx_pptx::Presentation::open(&support::fixture("text_levels.pptx"))
        .expect("a well-formed package");
    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    let mut model = support::model();
    let constraints = constraints_for(&read);

    assert!(
        model
            .layout_notes(&read, 0, &constraints)
            .expect("asking is not an error")
            .is_none(),
        "`text_levels.pptx` carries no notes part, so its notes page is `None`"
    );
    assert!(
        model
            .layout_notes(&read, 99, &constraints)
            .expect("a slide past the end is not an error")
            .is_none(),
        "asking for the notes of a slide that does not exist answers `None` rather than failing: a \
         caller iterating a deck it is editing must not be punished for a stale index"
    );
}
