//! A chart laid out with nominal metrics is counted once as text measured and not shaped (MJXOFF-299).

mod support;

use mjx_layout::{BoxModel, LayoutLossKind, PageIndex};
use mjx_layout_pptx::{constraints_for, ShapeContent, SlideDeck};
use mjx_pptx::Presentation;

#[test]
fn every_chart_on_a_slide_is_one_text_measured_not_shaped_at_its_frame() {
    let bytes = support::fixture("charts.pptx");
    let mut presentation = Presentation::open(&bytes).expect("the deck opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let (slide, frames) = (0..deck.slide_count())
        .map(|index| {
            let frames: Vec<Vec<u32>> = deck.slide(index).map_or(Vec::new(), |slide| {
                slide
                    .shapes()
                    .iter()
                    .filter(|shape| matches!(shape.content, ShapeContent::Chart(_)))
                    .map(|shape| {
                        std::iter::once(index as u32)
                            .chain(shape.path.iter().copied())
                            .collect()
                    })
                    .collect()
            });
            (index, frames)
        })
        .find(|(_, frames)| !frames.is_empty())
        .expect("`charts.pptx` frames a chart on some slide");
    let mut model = support::model();
    let losses = model
        .layout_page(
            &deck,
            PageIndex::new(slide as u32),
            &constraints_for(&deck),
            None,
        )
        .expect("the slide lays out")
        .losses()
        .clone();
    let measured: Vec<Vec<u32>> = losses
        .iter()
        .filter(|loss| loss.kind == LayoutLossKind::TextMeasuredNotShaped)
        .map(|loss| {
            assert!(loss.area.is_none(), "an approximation draws no placeholder");
            loss.source.path().segments().to_vec()
        })
        .collect();
    assert_eq!(
        measured, frames,
        "one approximation per chart, at its frame"
    );
}
