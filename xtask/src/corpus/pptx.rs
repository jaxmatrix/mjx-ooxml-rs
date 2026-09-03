//! The PresentationML corpus file: a deck of hundreds of slides, built through the real edit
//! surface (MJXOFF-147) — `mjx-pptx` is the one format with a model, so there is no reason to invent
//! markup for it the way [`super::docx`] and [`super::xlsx`] must.

use anyhow::{Context, Result};
use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_pptx::{PartName, Presentation, ShapeBounds, SlideSize};

/// The number of slides the generated deck carries — "hundreds of slides" per MJXOFF-68/MJXOFF-147.
pub const SLIDE_COUNT: usize = 300;

/// PowerPoint's widescreen default: 13⅓ × 7½ inches, in EMU.
const WIDESCREEN: SlideSize = SlideSize {
    width_emu: 12_192_000,
    height_emu: 6_858_000,
    kind: SlideSizeKind::Screen16X9,
};

/// Builds a [`SLIDE_COUNT`]-slide deck: each slide is added from the blank deck's own layout (a
/// title and a body placeholder), both placeholders get real text, and a third free-standing text
/// box is appended — three shapes a slide, the way a normal deck mixes placeholders with
/// free-standing content, rather than one synthetic shape repeated.
///
/// # Errors
/// Returns an error if building the blank deck or any edit fails.
pub fn build_large_deck() -> Result<Presentation> {
    let mut deck = Presentation::blank(WIDESCREEN).context("Presentation::blank")?;
    for i in 0..SLIDE_COUNT {
        let slide = deck
            .add_slide_from_layout(0)
            .with_context(|| format!("add_slide_from_layout at slide {i}"))?;
        deck.set_shape_text_content(slide, 0, &format!("Slide {} of {SLIDE_COUNT}", i + 1))
            .with_context(|| format!("title text on slide {i}"))?;
        deck.set_shape_text_content(
            slide,
            1,
            &format!(
                "Body text for slide {i}. This sentence pads the placeholder to a realistic \
                 length, the way a real deck's body text usually runs rather than a single word."
            ),
        )
        .with_context(|| format!("body text on slide {i}"))?;
        deck.add_text_box(
            slide,
            &format!("Footer note {i}"),
            ShapeBounds::new(457_200, 6_400_800, 3_000_000, 300_000),
        )
        .with_context(|| format!("footer text box on slide {i}"))?;
    }
    Ok(deck)
}

/// The part name of the slide roughly in the middle of a deck [`build_large_deck`] produced — the
/// membench target (MJXOFF-147). Computed rather than looked up on a live [`Presentation`], so a
/// `--mem pptx` run against an already-generated `deck_large.pptx` does not have to rebuild the
/// deck (and inflate its own peak-RSS reading) just to learn a part name: `next_slide_part` numbers
/// slides `1..=SLIDE_COUNT` in insertion order with no gaps, starting from a deck with none, so the
/// numbering is exactly `slide{index + 1}.xml` for as long as [`build_large_deck`] only appends.
///
/// # Errors
/// Returns an error only if the computed part name is somehow invalid (it is not, for any
/// `SLIDE_COUNT` this module ships).
pub fn representative_slide_part() -> Result<PartName> {
    let number = SLIDE_COUNT / 2 + 1;
    PartName::new(&format!("/ppt/slides/slide{number}.xml")).context("representative slide part")
}
