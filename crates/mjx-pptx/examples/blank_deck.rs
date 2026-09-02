//! Creating a deck from nothing — the runnable version of the opening section of
//! [the building-a-deck guide](mjx_pptx::guide::building_a_deck).
//!
//! No template, no fixture, no file read at all: `Presentation::blank` authors `[Content_Types].xml`,
//! the relationships, `presentation.xml`, a theme, a slide master and one slide layout, and this
//! example fills it in and writes it out.
//!
//! ```sh
//! cargo run -p mjx-pptx --example blank_deck -- out.pptx
//! ```
//!
//! Then it reopens what it wrote and asserts on it — the only file the library itself touches is the
//! one `main` hands it.

use anyhow::{Context, Result};
use mjx_ooxml_types::presentationml::{PlaceholderType, SlideSizeKind};
use mjx_pptx::{Presentation, ShapeBounds, SlideSize};

mod support;

/// PowerPoint's widescreen default: 13⅓ × 7½ inches, in EMU.
const WIDESCREEN: SlideSize = SlideSize {
    width_emu: 12_192_000,
    height_emu: 6_858_000,
    kind: SlideSizeKind::Screen16X9,
};

fn main() -> Result<()> {
    let out = support::output_path("blank_deck.pptx");

    // ---- A deck from nothing ----------------------------------------------------------------
    // Every part below is authored in memory. `p:sldSz` can only express 914400..=51206400 EMU per
    // side, so a size outside that range comes back as `PptxError::InvalidSlideSize` rather than
    // being written out.
    let mut deck = Presentation::blank(WIDESCREEN).context("building a blank deck")?;
    println!(
        "blank deck: {} slides, {} layouts, {} masters",
        deck.slide_count(),
        deck.layout_count(),
        deck.master_count()
    );
    println!(
        "  layout 0: {:?} ({:?})",
        deck.layout_name(0)?.unwrap_or_default(),
        deck.layout_kind(0)?
    );

    // ---- A slide on the deck's own layout ---------------------------------------------------
    // The blank deck's single layout is "Title and Text", so the new slide arrives with a title and
    // a body placeholder already positioned by the master.
    let slide = deck.add_slide_from_layout(0)?;
    for shape in deck.shapes(slide)? {
        if let Some(placeholder) = shape.placeholder {
            println!(
                "  slide shape {}: {:?} placeholder",
                shape.index, placeholder.kind
            );
        }
    }
    deck.set_shape_text_content(slide, 0, "Built from nothing")?;
    deck.set_shape_text_content(
        slide,
        1,
        "No template was opened\nEvery part was authored in memory",
    )?;

    // Nothing above stated a size, a font or a position: all three come from the master and theme
    // that `blank` wrote.
    let title = deck.effective_run_properties(slide, 0, 0, 0)?;
    println!(
        "  the title renders at {:?}pt without anyone saying so",
        title.size_points()
    );

    // ---- A second slide, and a plain text box -----------------------------------------------
    let second = deck.add_slide_with_text(
        "A second slide",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    )?;
    deck.add_text_box(
        second,
        "…and a text box beside it",
        ShapeBounds::from_inches(1.0, 3.5, 6.0, 1.0),
    )?;

    // ---- Save ---------------------------------------------------------------------------------
    let bytes = deck.save()?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    // ---- Reopen what was written, and check it ------------------------------------------------
    let mut reopened = Presentation::open(&bytes).context("reopening the deck just written")?;
    anyhow::ensure!(reopened.slide_count() == 2, "expected two slides");
    anyhow::ensure!(reopened.layout_count() == 1, "expected one layout");
    anyhow::ensure!(
        reopened.slide_size()? == WIDESCREEN,
        "the slide size did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.shape_text(0, 0)? == "Built from nothing",
        "the title did not survive the round trip"
    );
    anyhow::ensure!(
        reopened
            .shape_placeholder(0, 0)?
            .map(|placeholder| placeholder.kind)
            == Some(PlaceholderType::Title),
        "shape 0 should still be the title placeholder"
    );
    println!("reopened: {} slides, title intact", reopened.slide_count());

    Ok(())
}
