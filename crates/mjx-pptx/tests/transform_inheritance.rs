//! Integration tests for **effective** transform resolution — where a shape actually renders, as
//! opposed to what it declares.
//!
//! The interesting cases are the ones where the two answers differ, and `tests/fixtures/layouts.pptx`
//! is arranged so a single deck shows all of them. Its master places a `title` and a `body`;
//! slideLayout2 (`Surface::Layout(1)`) places its body but **not** its title, deferring that to the
//! master; and `add_slide_from_layout` builds placeholders with an empty `p:spPr`, so a slide made
//! from that layout declares no transform at all. One slide therefore resolves its title at the
//! **master** and its body at the **layout**.
//!
//! `sample.pptx` is the other end: a placeholder whose layout has no shapes, so nothing anywhere
//! answers.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Angle, Transform2D};
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_opc::Package;
use mjx_pptx::{Presentation, ShapeBounds, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

/// The index of the placeholder of `kind` on `surface`, as the deck itself reports it.
fn placeholder_at(pres: &mut Presentation, surface: Surface, kind: PlaceholderType) -> usize {
    let count = pres.shape_count(surface).expect("shape count");
    (0..count)
        .find(|&idx| {
            pres.shape_placeholder(surface, idx)
                .expect("placeholder")
                .is_some_and(|info| info.kind == kind)
        })
        .unwrap_or_else(|| panic!("no {kind:?} placeholder on {surface:?}"))
}

/// A deck with a slide built from layout 1, whose placeholders declare no transform of their own.
fn deck_with_an_inheriting_slide() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres
        .add_slide_from_layout(1)
        .expect("add slide from layout");
    (pres, slide)
}

// ---------------------------------------------------------------------------------------------
// The tiers
// ---------------------------------------------------------------------------------------------

#[test]
fn a_placeholder_that_places_itself_nowhere_takes_the_layouts_position() {
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let body = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Object);

    // It declares nothing…
    assert_eq!(pres.shape_bounds(slide, body).expect("explicit"), None);

    // …and renders where the layout's same-slot placeholder says.
    let layout_body = placeholder_at(&mut pres, Surface::Layout(1), PlaceholderType::Object);
    let expected = pres
        .shape_bounds(Surface::Layout(1), layout_body)
        .expect("layout bounds")
        .expect("the layout places its body");

    assert_eq!(
        pres.effective_shape_bounds(slide, body).expect("effective"),
        Some(expected)
    );
}

#[test]
fn resolution_walks_past_a_layout_that_places_nothing_and_reaches_the_master() {
    // slideLayout2's title defers to the master, as real layouts commonly do.
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let title = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Title);

    assert_eq!(pres.shape_bounds(slide, title).expect("explicit"), None);

    let layout_title = placeholder_at(&mut pres, Surface::Layout(1), PlaceholderType::Title);
    assert_eq!(
        pres.shape_bounds(Surface::Layout(1), layout_title)
            .expect("layout"),
        None,
        "the layout must not place its title, or this test proves nothing"
    );

    let master_title = placeholder_at(&mut pres, Surface::Master(0), PlaceholderType::Title);
    let expected = pres
        .shape_bounds(Surface::Master(0), master_title)
        .expect("master bounds")
        .expect("the master places its title");

    assert_eq!(
        pres.effective_shape_bounds(slide, title)
            .expect("effective"),
        Some(expected)
    );
}

#[test]
fn the_two_tiers_answer_differently_in_the_same_deck() {
    // The title resolves at the master and the body at the layout: proof the walk stops per shape
    // rather than choosing one tier for the whole slide.
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let title = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Title);
    let body = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Object);

    let title_bounds = pres
        .effective_shape_bounds(slide, title)
        .expect("title")
        .expect("resolved");
    let body_bounds = pres
        .effective_shape_bounds(slide, body)
        .expect("body")
        .expect("resolved");

    assert_ne!(title_bounds, body_bounds);
}

#[test]
fn a_shape_that_places_itself_is_not_asked_to_inherit() {
    // layouts.pptx slide 1's placeholders declare their own transforms.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let explicit = pres
        .shape_bounds(0, 0)
        .expect("explicit")
        .expect("declared");
    assert_eq!(
        pres.effective_shape_bounds(0, 0).expect("effective"),
        Some(explicit)
    );
}

#[test]
fn nothing_anywhere_answers_when_no_tier_places_the_shape() {
    // sample.pptx's only shape is a ctrTitle whose layout defines no shapes at all.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    assert_eq!(pres.shape_bounds(0, 0).expect("explicit"), None);
    assert_eq!(pres.effective_shape_bounds(0, 0).expect("effective"), None);
}

#[test]
fn a_shape_that_is_not_a_placeholder_inherits_nothing() {
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let bounds = ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0);
    let idx = pres
        .add_text_box(slide, "not a placeholder", bounds)
        .expect("add text box");

    assert_eq!(
        pres.effective_shape_bounds(slide, idx).expect("effective"),
        Some(bounds),
        "a text box has no slot to inherit through — effective is explicit"
    );
}

#[test]
fn a_layouts_own_placeholder_resolves_against_the_master() {
    // Every surface is walkable, not just a slide.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let layout_title = placeholder_at(&mut pres, Surface::Layout(1), PlaceholderType::Title);
    let master_title = placeholder_at(&mut pres, Surface::Master(0), PlaceholderType::Title);
    let expected = pres
        .shape_bounds(Surface::Master(0), master_title)
        .expect("master")
        .expect("declared");

    assert_eq!(
        pres.effective_shape_bounds(Surface::Layout(1), layout_title)
            .expect("effective"),
        Some(expected)
    );
}

// ---------------------------------------------------------------------------------------------
// The rules the walk follows
// ---------------------------------------------------------------------------------------------

#[test]
fn inheritance_is_all_or_nothing_rather_than_field_by_field() {
    // A slide placeholder that states only a rotation must NOT then borrow the layout's position:
    // a transform is inherited whole, and stating anything stops the walk.
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let body = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Object);

    pres.set_shape_transform(
        slide,
        body,
        &Transform2D {
            rotation: Some(Angle::from_degrees(15.0)),
            ..Transform2D::default()
        },
    )
    .expect("state a rotation and nothing else");

    let resolved = pres
        .effective_shape_transform(slide, body)
        .expect("effective")
        .expect("the shape states something");

    assert!((resolved.rotation.expect("rotation").degrees() - 15.0).abs() < 1e-6);
    assert_eq!(
        resolved.position, None,
        "the layout's position must not be merged in under the slide's rotation"
    );
    assert_eq!(
        pres.effective_shape_bounds(slide, body).expect("bounds"),
        None,
        "the tier that answered names no bounds, so there are none"
    );
}

#[test]
fn an_empty_transform_element_states_nothing_and_the_walk_continues() {
    // `set_shape_transform` with an empty spec creates the `a:xfrm` element without filling it in.
    // The element exists, but it says nothing, so the layout still answers.
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let body = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Object);

    pres.set_shape_transform(slide, body, &Transform2D::default())
        .expect("create an empty transform");
    assert_eq!(
        pres.shape_transform(slide, body).expect("explicit"),
        Some(Transform2D::default()),
        "the element is there…"
    );

    let layout_body = placeholder_at(&mut pres, Surface::Layout(1), PlaceholderType::Object);
    let expected = pres
        .shape_bounds(Surface::Layout(1), layout_body)
        .expect("layout")
        .expect("declared");
    assert_eq!(
        pres.effective_shape_bounds(slide, body).expect("effective"),
        Some(expected),
        "…but it states nothing, so the layout still answers"
    );
}

#[test]
fn resolution_survives_a_save_and_reopen() {
    let (mut pres, slide) = deck_with_an_inheriting_slide();
    let body = placeholder_at(&mut pres, Surface::Slide(slide), PlaceholderType::Object);
    let before = pres
        .effective_shape_bounds(slide, body)
        .expect("effective")
        .expect("resolved");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.effective_shape_bounds(slide, body).expect("after"),
        Some(before)
    );
}

#[test]
fn resolving_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    for surface in [
        Surface::Slide(0),
        Surface::Slide(1),
        Surface::Layout(1),
        Surface::Master(0),
    ] {
        for idx in 0..pres.shape_count(surface).expect("count") {
            let _ = pres
                .effective_shape_bounds(surface, idx)
                .expect("effective");
            let _ = pres
                .effective_shape_transform(surface, idx)
                .expect("effective");
        }
    }

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original,
        "resolution is a read and must not dirty a part"
    );
}
