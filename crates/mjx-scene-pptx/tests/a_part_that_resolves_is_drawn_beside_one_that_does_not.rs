//! A shape draws every part of its decoration that resolves and counts each part that does not (MJXOFF-299).

use mjx_layout::{BoxModel, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_pptx::{Package, PartName, Presentation, SlideSize};
use mjx_scene::{build_page, Command, DisplayList, SceneLossKind, SceneOptions};
use mjx_scene_pptx::SlideResources;
use mjx_text::GlyphAtlas;

const NAMESPACES: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

const BLUE_FILL: &str = r#"<a:solidFill><a:srgbClr val="4472C4"/></a:solidFill>"#;

const PLACEHOLDER_COLOUR_LINE: &str = r#"<a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"><a:lumMod val="75000"/></a:schemeClr></a:solidFill></a:ln>"#;

// A one-slide deck whose one rectangle states `properties` after its geometry.
fn deck(properties: &str) -> Vec<u8> {
    let mut blank = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    blank.add_slide_from_layout(0).expect("one slide");
    let mut package = Package::open(&blank.save().expect("it saves")).expect("it reopens");
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {NAMESPACES}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape 2"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="1828800"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{properties}</p:spPr></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    );
    package
        .replace_part_bytes(
            &PartName::new("/ppt/slides/slide1.xml").expect("a part name"),
            slide.into_bytes(),
        )
        .expect("the slide is replaceable");
    package.save().expect("the deck saves")
}

// Slide 0 of a deck whose one rectangle states `properties`, built into a display list.
fn list_of(properties: &str) -> DisplayList {
    let bytes = deck(properties);
    let mut presentation = Presentation::open(&bytes).expect("the deck opens");
    let read = SlideDeck::read(&mut presentation).expect("the deck reads");
    let constraints = constraints_for(&read);
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    let mut model = SlideBoxModel::new(
        mjx_text::FontResolver::builder()
            .with_bundled_font_directory(&fonts)
            .expect("the committed faces index")
            .build(),
    );
    let page = model
        .layout_page(&read, PageIndex::FIRST, &constraints, None)
        .expect("the slide lays out");
    let options = SceneOptions::new(constraints.page);
    let resources = SlideResources::new(model.catalogue().clone(), options.device_scale);
    build_page(
        &page,
        &resources,
        model.rasteriser_mut(),
        &mut GlyphAtlas::new(),
        &options,
    )
    .expect("no loss fails a scene")
}

// How many fills, strokes and effect groups the list draws.
fn draws(list: &DisplayList) -> (usize, usize, usize) {
    let count = |wanted: fn(&Command) -> bool| list.commands().filter(wanted).count();
    (
        count(|command| matches!(command, Command::FillPath { .. })),
        count(|command| matches!(command, Command::StrokePath { .. })),
        count(|command| matches!(command, Command::PushEffect(_))),
    )
}

#[test]
fn a_fill_that_resolves_is_drawn_under_an_outline_that_does_not() {
    let list = list_of(&format!("{BLUE_FILL}{PLACEHOLDER_COLOUR_LINE}"));
    assert_eq!(
        (
            draws(&list),
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len()
        ),
        ((1, 0, 0), 1, 0),
        "the blue fill draws, the outline's placeholder colour is one loss, and nothing is missing"
    );
}

#[test]
fn a_fill_that_resolves_is_drawn_without_a_shadow_that_does_not() {
    let list = list_of(&format!(
        r#"{BLUE_FILL}<a:ln><a:noFill/></a:ln><a:effectLst><a:outerShdw blurRad="38100" dist="38100" dir="2700000"><a:schemeClr val="phClr"/></a:outerShdw></a:effectLst>"#
    ));
    assert_eq!(
        (
            draws(&list),
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len()
        ),
        ((1, 0, 0), 1, 0),
        "the fill draws with no shadow group, and the shadow's colour is one loss"
    );
}

#[test]
fn a_shape_with_nothing_left_to_draw_is_one_placeholder_and_one_loss_per_part() {
    let list = list_of(&format!(
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>{PLACEHOLDER_COLOUR_LINE}"#
    ));
    assert_eq!(
        (
            draws(&list),
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len()
        ),
        ((0, 0, 0), 2, 1),
        "a fill and an outline that both fail are two losses under one placeholder"
    );
}
