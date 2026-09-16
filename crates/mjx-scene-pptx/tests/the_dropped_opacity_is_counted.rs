//! **No opacity is dropped, so nothing counts one** (MJXOFF-243, RC04).
//!
// The name is kept from MJXOFF-300, which wrote this file to assert the count; RC04 inverted it.
//!
//! # What this file used to say
//!
//! MJXOFF-300 found that a 35 % overlay painted as a solid slab over the table row beneath it while
//! the render called itself lossless: `color_of` answered `Ok` for a valid triplet and no loss kind
//! named a discarded opacity. Its answer was to *count* the drop —
//! `mjx_dml::LostOpacities` through `ShapeDecoration::lost_opacities` to one
//! `SceneLossKind::PaintApproximated` per colour — so the approximation was visible while RC04 was
//! still ahead.
//!
//! RC04 carries the channel, so there is nothing left to approximate. The count must go to **zero**,
//! and the decoration that was `Partial` must answer whole. A fix that carried the alpha and left
//! the count standing would leave every consumer — the acceptance render, the parity ledger, a
//! host's own report — declaring a loss the library no longer takes.
//!
//! # Proved by mutation
//!
//! Leaving the `lost.push(SceneLossKind::PaintApproximated)` in `src/resources.rs` in place while
//! `mjx-dml` carries the alpha fails [`no_decoration_on_the_corporate_slide_loses_an_opacity`] and
//! [`the_overlays_decoration_answers_whole`], naming the decoration that still reports one.

use mjx_layout::{BoxModel, DecorationRef, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_scene::{Color, DeviceScale, FillStyle, Resolved, ResourceResolver, SceneLossKind};
use mjx_scene_pptx::SlideResources;

// The corporate deck's overlay rectangle is `1F3864` at 35 % — the one alpha on the slide.
const FIXTURE: &str = "corporate.pptx";

/// 35 % as the byte a `mjx_scene::Color` carries: `round(0.35 * 255)` is `89.25`, so `0x59`.
const THIRTY_FIVE_PERCENT: u8 = 0x59;

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

/// **No decoration on the corporate slide loses an opacity**, because the resolution carries it.
#[test]
fn no_decoration_on_the_corporate_slide_loses_an_opacity() {
    let (_, entries) = decorations();
    let lost: Vec<(DecorationRef, usize)> = entries
        .iter()
        .copied()
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        lost,
        vec![],
        "the slide states one `a:alpha` — the overlay rectangle's 35 % fill — and the box model \
         still reports it as an opacity the resolution could not carry"
    );
}

/// The overlay's decoration answers whole, at the opacity the slide states.
#[test]
fn the_overlays_decoration_answers_whole() {
    let (resources, entries) = decorations();
    let overlay = entries
        .iter()
        .map(|(handle, _)| *handle)
        .find(|handle| {
            matches!(
                resources.decoration(*handle),
                Resolved::Answered(decoration) | Resolved::Partial(decoration, _)
                    if decoration.fill
                        == FillStyle::Solid(Color {
                            red: 0x1F,
                            green: 0x38,
                            blue: 0x64,
                            alpha: THIRTY_FIVE_PERCENT,
                        })
            )
        })
        .expect(
            "no decoration on the corporate slide is `1F3864` at 35 %. The overlay band states \
             exactly that, so either its alpha is not carried or its colour is wrong.",
        );

    let answer = resources.decoration(overlay);
    assert!(
        matches!(answer, Resolved::Answered(_)),
        "the overlay's decoration answered {answer:?}. Every part of it resolves and its opacity is \
         carried, so there is nothing left to report and it is whole."
    );
}

/// Every other decoration still answers whole, so the assertion above is not the only one that could.
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
        "no decoration on the corporate slide answers whole"
    );
}

/// The kind still draws rather than standing in, for whatever else reaches it.
///
/// `PaintApproximated` is not deleted by RC04 — a hatch with no preset and a `a:grpFill` still take
/// it — so the property it was chosen for has to stay true.
#[test]
fn an_approximated_paint_draws_rather_than_standing_in() {
    assert!(
        !SceneLossKind::PaintApproximated.draws_placeholder(),
        "an approximated paint would put a grey box over content that is drawn"
    );
}
