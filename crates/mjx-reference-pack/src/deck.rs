//! Authoring the two preset decks.
//!
//! # Solid fills only, and the exclusion list that is therefore empty
//!
//! Every plate is filled with one `a:solidFill` and stroked with one `a:solidFill` line. Not a
//! gradient, not a shade, not a hatch — deliberately, because
//! [`ReferenceProvider::LibreOffice`](crate::ReferenceProvider::LibreOffice) cannot speak about
//! those and a sheet on which every plate is excluded proves nothing. The exclusion is real and it
//! belongs to the *third* artefact, which carries the hatches; on these two it is empty, and
//! `tests/an_excluded_result_is_not_evidence.rs` asserts the emptiness rather than leaving it to be
//! believed. An exclusion quietly carried onto a sheet that did not need it removes the sheet from
//! the comparison.
//!
//! # Filled **and** stroked, both
//!
//! Sixty-three of the 186 presets end a contour without an `a:close`, and a connector such as
//! `straightConnector1` is a line segment with no interior at all. A comparison that only ever
//! filled would show those plates as blank on both sides and report agreement — which is precisely
//! the defect MJXOFF-206 found in the workspace's own stand-in census, where
//! `DrawReport::placeholders`' stroke branch had never executed in any test. So every plate is drawn
//! twice, and our own side does the same.
//!
//! # The caption is not inside the shape
//!
//! `add_shape` gives each shape a text body with one empty run, and it stays empty. The caption is a
//! separate text box in the strip below the plate, outside the comparison window — because text
//! inside an autoshape is text PowerPoint may autofit, and a shape whose *geometry* is under
//! comparison must not also be exercising a text engine.

use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_pptx::{PptxError, Presentation};

use crate::layout::{slide_size, Rect, ToShapeBounds, HEADING_HEIGHT, PLATES_PER_SLIDE, SLIDE};
use crate::plates::{plates, Plate, PlateKind, PresetDeck};

/// The plate fill. A pale blue, so an outline in the darker stroke colour is legible against it and
/// so a plate that drew nothing is white rather than *nearly* white.
pub const PLATE_FILL: &str = "DBE6F5";

/// The plate outline.
pub const PLATE_STROKE: &str = "2B5AA8";

/// The outline width, in points. One point at 96 dpi is 1⅓ pixels, which is wide enough that a
/// stroked-only shape has ink and narrow enough that the stroke does not dominate a 100 × 70 plate.
pub const PLATE_STROKE_POINTS: f64 = 1.0;

/// The caption's type size, in points.
pub const CAPTION_POINTS: f64 = 8.0;

/// The heading's type size, in points.
pub const HEADING_POINTS: f64 = 14.0;

/// The typeface every caption and heading is set in.
///
/// **Named rather than inherited.** A caption that took the theme's font would be set in whatever
/// the reader's machine substitutes, and the caption strip is the one part of a plate the two sides
/// are known to differ over — so it is kept outside the comparison window *and* kept boring.
pub const CAPTION_FONT: &str = "Arial";

/// Author one preset deck and answer its bytes.
///
/// # Errors
///
/// Whatever the presentation layer fails with. Nothing here is input-driven — the shapes come from a
/// generated table and the geometry from constants — so a failure is a defect rather than a bad
/// file.
pub fn preset_deck(which: PresetDeck) -> Result<Vec<u8>, PptxError> {
    let plates = plates(which);
    let mut deck = Presentation::blank(slide_size())?;
    let pages = crate::layout::pages_for(plates.len());

    for page in 0..pages {
        let slide = deck.add_slide()?;
        write_heading(&mut deck, slide, which, page, pages)?;
        for plate in plates
            .iter()
            .skip(page * PLATES_PER_SLIDE)
            .take(PLATES_PER_SLIDE)
        {
            write_plate(&mut deck, slide, plate)?;
        }
    }
    deck.save()
}

/// The heading strip across the top of a page.
fn write_heading(
    deck: &mut Presentation,
    slide: usize,
    which: PresetDeck,
    page: usize,
    pages: usize,
) -> Result<(), PptxError> {
    let text = format!("{} {} of {pages}", which.heading(), page + 1);
    let bounds = Rect::new(12, 6, SLIDE.0 - 24, HEADING_HEIGHT - 10).to_shape_bounds();
    let index = deck.add_text_box(slide, &text, bounds)?;
    deck.set_shape_run_properties(
        slide,
        index,
        &CharacterPropertiesSpec::new()
            .with_font(CAPTION_FONT)
            .with_size_points(HEADING_POINTS),
    )
}

/// One plate: the shape, its adjustments, its fill and outline, and its caption.
fn write_plate(deck: &mut Presentation, slide: usize, plate: &Plate) -> Result<(), PptxError> {
    let geometry = plate.geometry();

    let shape = deck.add_shape(slide, plate.preset, geometry.shape_box().to_shape_bounds())?;
    if !plate.adjustments.is_empty() {
        deck.set_shape_adjustments(slide, shape, &plate.adjustments)?;
    }
    deck.set_shape_fill(
        slide,
        shape,
        &FillSpec::Solid(ColorSpec::Srgb(PLATE_FILL.to_owned())),
    )?;
    deck.set_shape_outline(
        slide,
        shape,
        &LineSpec::solid(
            LineWidth::from_points(PLATE_STROKE_POINTS),
            ColorSpec::Srgb(PLATE_STROKE.to_owned()),
        ),
    )?;

    let caption = deck.add_text_box(
        slide,
        &caption_of(plate),
        geometry.caption().to_shape_bounds(),
    )?;
    deck.set_shape_run_properties(
        slide,
        caption,
        &CharacterPropertiesSpec::new()
            .with_font(CAPTION_FONT)
            .with_size_points(CAPTION_POINTS),
    )
}

/// What a plate's caption says: its index, its wire token, and — where it is not a plain comparison
/// — which of the three it is.
///
/// The marker is on the *sheet*, not only in the report, because the person holding the sheet is
/// the one who has to know that `upArrow` is not a plate anybody can check against our side.
#[must_use]
pub fn caption_of(plate: &Plate) -> String {
    let index = plate.index + 1;
    match &plate.kind {
        PlateKind::Comparable => format!("{index}. {}", plate.token),
        PlateKind::NoTable => format!("{index}. {} — no published geometry", plate.token),
        PlateKind::Singular { .. } => format!("{index}. {} — no value here", plate.token),
        PlateKind::ClampedExtreme { handles } => {
            format!("{index}. {} — {handles} clamped", plate.token)
        }
    }
}
