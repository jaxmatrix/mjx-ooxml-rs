//! Integration tests for a text body's geometry — what `a:bodyPr` states, and what a placeholder's
//! text body actually lays out under once the layout and master have been consulted.
//!
//! `a:bodyPr` was preserved verbatim and modeled nowhere until MJXOFF-169, because fidelity never
//! needed it: an untouched body re-emits byte for byte whatever this crate understands. Layout does
//! need it — insets, anchor, wrap, columns and autofit are the whole of a text body's geometry — so
//! these tests are about the two questions a box model asks and nothing else.
//!
//! The inheritance fixture is `layouts.pptx` for the same reason `transform_inheritance.rs` uses it:
//! its master places a `title` and a `body`, slideLayout2 places the body and not the title, and
//! `add_slide_from_layout` builds placeholders that declare nothing — so one deck shows a slot
//! resolved at the layout and a slot resolved at the master.

use std::path::PathBuf;

use mjx_dml::{Emu, Fraction, TextAnchoring, TextAutofit, TextBodyPropertiesSpec, TextWrapping};
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_pptx::{Presentation, ShapeBounds, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("reading fixture {name}: {error}"))
}

/// The index of the placeholder of `kind` on `surface`, as the deck itself reports it.
fn placeholder_at(deck: &mut Presentation, surface: Surface, kind: PlaceholderType) -> usize {
    let count = deck.shape_count(surface).expect("shape count");
    (0..count)
        .find(|&index| {
            deck.shape_placeholder(surface, index)
                .expect("placeholder")
                .is_some_and(|info| info.kind == kind)
        })
        .unwrap_or_else(|| panic!("no {kind:?} placeholder on {surface:?}"))
}

/// A deck with one text box, which is not a placeholder and therefore inherits nothing.
fn deck_with_a_text_box() -> (Presentation, usize) {
    let mut deck = Presentation::blank(mjx_pptx::SlideSize::widescreen()).expect("blank deck");
    let slide = deck.add_slide().expect("a slide");
    let shape = deck
        .add_text_box(slide, "text", ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("a text box");
    (deck, shape)
}

// ---------------------------------------------------------------------------------------------
// What a shape states
// ---------------------------------------------------------------------------------------------

#[test]
fn a_body_that_states_nothing_answers_none_for_every_field() {
    let (mut deck, shape) = deck_with_a_text_box();
    let stated = deck
        .body_properties(0, shape)
        .expect("body properties")
        .expect("the text box has an `a:bodyPr`");
    assert_eq!(stated, TextBodyPropertiesSpec::new());
    assert_eq!(stated.left_inset(), None, "unstated is not the default");
}

#[test]
fn every_field_survives_a_write_and_reads_back() {
    let (mut deck, shape) = deck_with_a_text_box();
    let written = TextBodyPropertiesSpec::new()
        .with_insets(
            Emu::from_emu(11),
            Emu::from_emu(22),
            Emu::from_emu(33),
            Emu::from_emu(44),
        )
        .with_anchor(TextAnchoring::Bottom)
        .with_anchor_centered(true)
        .with_wrap(TextWrapping::None)
        .with_columns(3)
        .with_column_space(Emu::from_emu(228_600))
        .with_autofit(TextAutofit::Normal {
            font_scale: Some(Fraction::from_ratio(0.625)),
            line_space_reduction: Some(Fraction::from_ratio(0.1)),
        });
    deck.set_body_properties(0, shape, &written)
        .expect("the write lands");

    let read = deck
        .body_properties(0, shape)
        .expect("body properties")
        .expect("it has one");
    assert_eq!(read, written);
}

#[test]
fn a_write_that_names_one_inset_leaves_the_other_three_alone() {
    let (mut deck, shape) = deck_with_a_text_box();
    deck.set_body_properties(
        0,
        shape,
        &TextBodyPropertiesSpec::new().with_insets(
            Emu::from_emu(1),
            Emu::from_emu(2),
            Emu::from_emu(3),
            Emu::from_emu(4),
        ),
    )
    .expect("the first write lands");
    deck.set_body_properties(
        0,
        shape,
        &TextBodyPropertiesSpec::new().with_left_inset(Emu::from_emu(99)),
    )
    .expect("the second write lands");

    let read = deck
        .body_properties(0, shape)
        .expect("body properties")
        .expect("it has one");
    assert_eq!(read.left_inset(), Some(Emu::from_emu(99)));
    assert_eq!(read.top_inset(), Some(Emu::from_emu(2)));
    assert_eq!(read.right_inset(), Some(Emu::from_emu(3)));
    assert_eq!(read.bottom_inset(), Some(Emu::from_emu(4)));
}

#[test]
fn setting_body_properties_dirties_only_that_slide() {
    let (mut deck, shape) = deck_with_a_text_box();
    deck.settle_dirty_parts();
    deck.set_body_properties(
        0,
        shape,
        &TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Center),
    )
    .expect("the write lands");
    let dirty = deck.dirty_parts();
    assert_eq!(dirty.len(), 1, "{dirty:?}");
    assert!(dirty[0].contains("slide1.xml"), "{dirty:?}");
}

// ---------------------------------------------------------------------------------------------
// What a placeholder renders under
// ---------------------------------------------------------------------------------------------

#[test]
fn a_shape_that_is_not_a_placeholder_inherits_nothing() {
    let (mut deck, shape) = deck_with_a_text_box();
    // The master of a blank deck states body-level insets; a text box is not a placeholder, so it
    // has no slot to be matched on and takes none of them.
    let effective = deck
        .effective_body_properties(0, shape)
        .expect("effective body properties");
    let stated = deck
        .body_properties(0, shape)
        .expect("body properties")
        .unwrap_or_default();
    assert_eq!(effective, stated);
}

#[test]
fn a_placeholder_takes_what_its_layout_states() {
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let layout_title = placeholder_at(&mut deck, Surface::Layout(1), PlaceholderType::Title);
    deck.set_body_properties(
        Surface::Layout(1),
        layout_title,
        &TextBodyPropertiesSpec::new()
            .with_anchor(TextAnchoring::Bottom)
            .with_columns(2),
    )
    .expect("the layout write lands");

    let slide = deck.add_slide_from_layout(1).expect("a slide");
    let slide_title = placeholder_at(&mut deck, Surface::Slide(slide), PlaceholderType::Title);
    assert_eq!(
        deck.body_properties(Surface::Slide(slide), slide_title)
            .expect("stated")
            .unwrap_or_default(),
        TextBodyPropertiesSpec::new(),
        "the slide's own placeholder states nothing"
    );

    let effective = deck
        .effective_body_properties(Surface::Slide(slide), slide_title)
        .expect("effective");
    assert_eq!(effective.anchor(), Some(TextAnchoring::Bottom));
    assert_eq!(effective.columns(), Some(2));
}

#[test]
fn a_placeholder_falls_through_the_layout_to_the_master() {
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let master_title = placeholder_at(&mut deck, Surface::Master(0), PlaceholderType::Title);
    deck.set_body_properties(
        Surface::Master(0),
        master_title,
        &TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Distributed),
    )
    .expect("the master write lands");

    let slide = deck.add_slide_from_layout(1).expect("a slide");
    let slide_title = placeholder_at(&mut deck, Surface::Slide(slide), PlaceholderType::Title);
    // Neither the slide's placeholder nor slideLayout2's states an anchor, so the master is the
    // tier that answers.
    let effective = deck
        .effective_body_properties(Surface::Slide(slide), slide_title)
        .expect("effective");
    assert_eq!(effective.anchor(), Some(TextAnchoring::Distributed));
}

#[test]
fn the_tiers_merge_attribute_by_attribute_rather_than_whole() {
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let master_title = placeholder_at(&mut deck, Surface::Master(0), PlaceholderType::Title);
    deck.set_body_properties(
        Surface::Master(0),
        master_title,
        &TextBodyPropertiesSpec::new()
            .with_anchor(TextAnchoring::Bottom)
            .with_columns(4)
            .with_left_inset(Emu::from_emu(7)),
    )
    .expect("the master write lands");

    let layout_title = placeholder_at(&mut deck, Surface::Layout(1), PlaceholderType::Title);
    deck.set_body_properties(
        Surface::Layout(1),
        layout_title,
        &TextBodyPropertiesSpec::new().with_columns(2),
    )
    .expect("the layout write lands");

    let slide = deck.add_slide_from_layout(1).expect("a slide");
    let slide_title = placeholder_at(&mut deck, Surface::Slide(slide), PlaceholderType::Title);
    deck.set_body_properties(
        Surface::Slide(slide),
        slide_title,
        &TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Center),
    )
    .expect("the slide write lands");

    let effective = deck
        .effective_body_properties(Surface::Slide(slide), slide_title)
        .expect("effective");
    assert_eq!(
        effective.anchor(),
        Some(TextAnchoring::Center),
        "the slide's own anchor wins"
    );
    assert_eq!(
        effective.columns(),
        Some(2),
        "the layout's column count wins"
    );
    assert_eq!(
        effective.left_inset(),
        Some(Emu::from_emu(7)),
        "and the master supplies what neither stated"
    );
}

#[test]
fn a_real_files_layout_resolves_its_masters_body_geometry() {
    // `charts.pptx` is the one committed deck whose master states a full `a:bodyPr` — the
    // `vert`/`lIns`/`tIns`/`rIns`/`bIns`/`rtlCol`/`anchor` PowerPoint writes onto every placeholder
    // it authors. Its layouts state a bare `<a:bodyPr/>`, so a layout placeholder resolves the
    // master's, and a resolver that only ever read the addressed surface would answer nothing.
    let mut deck = Presentation::open(&fixture("charts.pptx")).expect("open");
    let layout_title = placeholder_at(
        &mut deck,
        Surface::Layout(0),
        PlaceholderType::CenteredTitle,
    );
    assert_eq!(
        deck.body_properties(Surface::Layout(0), layout_title)
            .expect("stated")
            .unwrap_or_default(),
        TextBodyPropertiesSpec::new(),
        "the layout's own `a:bodyPr` states nothing"
    );

    let effective = deck
        .effective_body_properties(Surface::Layout(0), layout_title)
        .expect("effective");
    assert_eq!(effective.left_inset(), Some(Emu::from_emu(91_440)));
    assert_eq!(effective.top_inset(), Some(Emu::from_emu(45_720)));
    assert_eq!(effective.right_inset(), Some(Emu::from_emu(91_440)));
    assert_eq!(effective.bottom_inset(), Some(Emu::from_emu(45_720)));
    assert_eq!(effective.anchor(), Some(TextAnchoring::Center));
    assert_eq!(effective.has_right_to_left_columns(), Some(false));
    assert_eq!(
        effective.vertical(),
        Some(mjx_dml::TextDirection::Horizontal)
    );
}

#[test]
fn a_real_files_slide_states_its_own_wrap() {
    // The other direction: `charts.pptx`'s first slide carries `<a:bodyPr wrap="none"/>` on a shape
    // that is not a placeholder, so the answer is the shape's own and nothing is inherited.
    let mut deck = Presentation::open(&fixture("charts.pptx")).expect("open");
    let count = deck.shape_count(0).expect("shape count");
    let found = (0..count).any(|shape| {
        deck.effective_body_properties(0, shape)
            .expect("effective")
            .wrap()
            == Some(TextWrapping::None)
    });
    assert!(
        found,
        "no shape on charts.pptx's first slide states wrap=none"
    );
}

#[test]
fn reading_dirties_nothing() {
    let mut deck = Presentation::open(&fixture("layouts.pptx")).expect("open");
    deck.settle_dirty_parts();
    let count = deck.shape_count(0).expect("shape count");
    for shape in 0..count {
        let _ = deck.body_properties(0, shape).expect("stated");
        let _ = deck.effective_body_properties(0, shape).expect("effective");
    }
    assert!(deck.dirty_parts().is_empty());
}
