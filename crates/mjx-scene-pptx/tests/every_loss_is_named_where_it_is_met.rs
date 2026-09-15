//! The slide companion names every answer it cannot give, where it meets it (MJXOFF-299).

use mjx_dml::ColorSpec;
use mjx_layout::{BoxModel, Fragment, PageIndex};
use mjx_layout_pptx::{constraints_for, PageCatalogue, ShapeContent, SlideBoxModel, SlideDeck};
use mjx_scene::{DeviceScale, Resolved, ResourceResolver, SceneLossKind};
use mjx_scene_pptx::paint::color_of;
use mjx_scene_pptx::SlideResources;

// The bundled faces only.
fn model() -> SlideBoxModel {
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    SlideBoxModel::new(
        mjx_text::FontResolver::builder()
            .with_bundled_font_directory(&fonts)
            .expect("the committed faces index")
            .build(),
    )
}

#[test]
fn a_transformed_colour_is_a_colour_not_resolved() {
    assert_eq!(
        color_of(&ColorSpec::Transformed {
            base: Box::new(ColorSpec::Srgb("FF0000".to_owned())),
            transforms: Vec::new(),
        }),
        Err(SceneLossKind::ColourNotResolved)
    );
}

#[test]
fn a_chart_is_unanswerable_at_its_frame_and_at_the_handles_it_issued() {
    let bytes = mjx_fixtures::fixture("charts.pptx");
    let mut presentation = mjx_pptx::Presentation::open(&bytes).expect("the deck opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let (slide, charts) = (0..deck.slide_count())
        .map(|index| {
            let charts = deck.slide(index).map_or(0, |slide| {
                slide
                    .shapes()
                    .iter()
                    .filter(|shape| matches!(shape.content, ShapeContent::Chart(_)))
                    .count()
            });
            (index, charts)
        })
        .find(|(_, charts)| *charts > 0)
        .expect("`charts.pptx` frames a chart on some slide");

    let mut model = model();
    let page = model
        .layout_page(
            &deck,
            PageIndex::new(u32::try_from(slide).expect("a small deck")),
            &constraints_for(&deck),
            None,
        )
        .expect("the slide lays out");
    let resources = SlideResources::new(model.catalogue().clone(), DeviceScale::UNZOOMED);
    let frames = page
        .fragments()
        .nodes()
        .filter(|(_, node)| {
            resources.unanswerable_content(node.source()) == Some(SceneLossKind::ChartNotResolved)
        })
        .count();
    assert_eq!(frames, charts, "one unanswerable frame per chart");

    let handle = page
        .fragments()
        .nodes()
        .find_map(|(_, node)| {
            match node.fragment() {
                Fragment::Shape(shape) => shape.decoration,
                Fragment::Box(frame) => frame.decoration,
                _ => None,
            }
            .filter(|handle| PageCatalogue::is_chart_handle(handle.number()))
        })
        .expect("a chart issues decoration handles");
    assert_eq!(
        resources.decoration(handle),
        Resolved::Unanswerable(SceneLossKind::ChartNotResolved)
    );
}
