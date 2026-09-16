//! **A colour whose opacity was dropped is counted, not silently painted opaque** (MJXOFF-300).
//!
//! # The defect this closes
//!
//! `tests/the_opacity_is_lost_at_the_spec_boundary.rs` proves the alpha is destroyed at the
//! `mjx-dml` boundary, and that is still true — carrying it through is MJXOFF-243 (RC04). What was
//! *also* true until RC03's audit is that nothing counted the loss: `color_of` answers `Ok` for a
//! valid triplet, no loss kind named a discarded opacity, and a 35 % overlay therefore painted as a
//! solid slab over the row beneath it while the render reported itself lossless.
//!
//! So the resolution now reports what it could not represent, the box model carries the count, and
//! this crate raises one [`SceneLossKind::PaintApproximated`] per dropped opacity at the shape's own
//! source. An approximation rather than a placeholder is the right kind: the shape *is* drawn, in
//! the right colour at the wrong opacity, and a placeholder over it would hide the content.
//!
//! # Proved by mutation
//!
//! Deleting the `lost.push(SceneLossKind::PaintApproximated)` in `src/resources.rs` fails
//! [`the_overlays_dropped_opacity_is_counted_where_it_is_met`], naming the decoration that answered
//! whole.

use mjx_layout::{BoxModel, DecorationRef, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_scene::{DeviceScale, Resolved, ResourceResolver, SceneLossKind};
use mjx_scene_pptx::SlideResources;

// The corporate deck's overlay rectangle is `1F3864` at 35 % — the one alpha on the slide.
const FIXTURE: &str = "corporate.pptx";

// The bundled faces only, so the layout does not depend on what the machine has installed.
fn model() -> SlideBoxModel {
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    SlideBoxModel::new(
        mjx_text::FontResolver::builder()
            .with_bundled_font_directory(&fonts)
            .expect("the committed faces index")
            .build(),
    )
}

/// The slide's decorations, as (handle, how many opacities its resolution could not represent).
fn decorations() -> (SlideResources, Vec<(DecorationRef, usize)>) {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let mut presentation = mjx_pptx::Presentation::open(&bytes).expect("the corporate deck opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let mut model = model();
    let _ = model
        .layout_page(&deck, PageIndex::new(0), &constraints_for(&deck), None)
        .expect("the slide lays out");
    let catalogue = model.catalogue().clone();
    let entries = (0..catalogue.decoration_count())
        .map(|index| {
            let handle = DecorationRef::new(index as u64);
            let entry = catalogue
                .decoration(handle)
                .expect("a handle the catalogue issued resolves");
            (handle, entry.lost_opacities)
        })
        .collect();
    (
        SlideResources::new(catalogue, DeviceScale::UNZOOMED),
        entries,
    )
}

/// **Exactly one decoration on the corporate slide lost an opacity**, and it is counted where it is met.
#[test]
fn the_overlays_dropped_opacity_is_counted_where_it_is_met() {
    let (resources, entries) = decorations();
    let lost: Vec<(DecorationRef, usize)> = entries
        .iter()
        .copied()
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        lost.len(),
        1,
        "the corporate slide states one `a:alpha` — the overlay rectangle's 35 % fill — and {} \
         decoration(s) report a dropped opacity",
        lost.len()
    );
    let (handle, count) = lost[0];
    assert_eq!(count, 1, "the overlay states one colour, so one opacity");

    let answer = resources.decoration(handle);
    let Resolved::Partial(_, losses) = &answer else {
        panic!(
            "the overlay's decoration answered {answer:?}. A fill whose opacity was dropped is \
             drawn — in the right colour at the wrong opacity — so it is `Partial`, never whole \
             and never unanswerable."
        );
    };
    assert_eq!(
        losses,
        &vec![SceneLossKind::PaintApproximated],
        "a dropped opacity is an approximation: the shape is drawn, so it takes no placeholder"
    );
}

/// Every other decoration on the slide still answers whole, so the count above is not everything.
#[test]
fn a_decoration_that_lost_nothing_still_answers_whole() {
    let (resources, entries) = decorations();
    let whole = entries
        .iter()
        .filter(|(handle, count)| {
            *count == 0 && matches!(resources.decoration(*handle), Resolved::Answered(_))
        })
        .count();
    assert!(
        whole >= 1,
        "no decoration on the corporate slide answers whole, so the assertion that one of them \
         answers `Partial` measures nothing"
    );
}

/// An approximation draws, so it never takes a placeholder — the property the kind is chosen for.
#[test]
fn an_approximated_paint_draws_rather_than_standing_in() {
    assert!(
        !SceneLossKind::PaintApproximated.draws_placeholder(),
        "a dropped opacity would put a grey box over content that is drawn"
    );
}
