//! **The opacity a document states reaches the display list** (MJXOFF-243, RC04).
//!
//! # Where this sits
//!
//! `crates/mjx-dml/tests/the_alpha_survives_resolution.rs` pins the form a resolved translucent
//! colour takes — `ColorSpec::Transformed { base: Srgb(hex), transforms: [Alpha(x)] }`. This suite
//! is the next seam: `color_of` must read that form, and a slide's fill, its outline and each of its
//! effect colours must arrive in `mjx_scene::Color` with the alpha the file stated.
//!
//! Until RC04 a `Transformed` colour drew **nothing** here — `color_of` answered
//! `SceneLossKind::ColourNotResolved` for it, on the honest ground that the crate had no way to
//! evaluate a transform chain. Once resolution carries the alpha and bakes everything else, that
//! reason is gone: the base is a triplet and the one transform is a channel.
//!
//! # Four opacities, on purpose
//!
//! A colour whose alpha is always `0xff` is reached constantly at one value, so "the alpha is
//! carried" is not a question a single-value suite can answer. The deck below states **25 %, 50 %,
//! 75 % and fully opaque** across a fill, a stroke and two effect colours, and every assertion names
//! the byte it expects: `0x40`, `0x80`, `0xBF`, `0xFF`. A build stuck at one value fails at least
//! three of them.
//!
//! The byte is `round(ratio * 255)`, which is the same conversion `mjx_tokens::Color` documents on
//! the other side — 25 % is `63.75`, so `0x40`; 75 % is `191.25`, so `0xBF`.

use mjx_dml::{ColorSpec, ColorTransform, Fraction};
use mjx_layout::{BoxModel, DecorationRef, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_pptx::{Package, PartName, Presentation, SlideSize};
use mjx_scene::{Color, DeviceScale, FillStyle, Resolved, ResourceResolver, SceneLossKind};
use mjx_scene_pptx::paint::color_of;
use mjx_scene_pptx::SlideResources;

const NAMESPACES: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

/// The shape's fill: `1F3864` at 25 %.
const FILL: &str =
    r#"<a:solidFill><a:srgbClr val="1F3864"><a:alpha val="25000"/></a:srgbClr></a:solidFill>"#;
/// Its outline: `C00000` at 50 %.
const OUTLINE: &str = r#"<a:ln w="12700"><a:solidFill><a:srgbClr val="C00000"><a:alpha val="50000"/></a:srgbClr></a:solidFill></a:ln>"#;
/// Its effects: a glow at 75 % and an outer shadow that states no alpha at all.
const EFFECTS: &str = concat!(
    r#"<a:effectLst>"#,
    r#"<a:glow rad="63500"><a:srgbClr val="FFC000"><a:alpha val="75000"/></a:srgbClr></a:glow>"#,
    r#"<a:outerShdw blurRad="40000" dist="20000" dir="5400000"><a:srgbClr val="000000"/></a:outerShdw>"#,
    r#"</a:effectLst>"#
);

/// The four opacities, as the bytes a `mjx_scene::Color` carries.
const QUARTER: u8 = 0x40;
const HALF: u8 = 0x80;
const THREE_QUARTERS: u8 = 0xBF;
const OPAQUE: u8 = 0xFF;

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

// The one decoration the translucent slide issues, resolved.
fn decoration() -> mjx_scene::Decoration {
    let bytes = deck(&format!("{FILL}{OUTLINE}{EFFECTS}"));
    let mut presentation = Presentation::open(&bytes).expect("the deck opens");
    let read = SlideDeck::read(&mut presentation).expect("the deck reads");
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    let mut model = SlideBoxModel::new(
        mjx_text::FontResolver::builder()
            .with_bundled_font_directory(&fonts)
            .expect("the committed faces index")
            .build(),
    );
    let _ = model
        .layout_page(&read, PageIndex::FIRST, &constraints_for(&read), None)
        .expect("the slide lays out");
    let catalogue = model.catalogue().clone();
    let resources = SlideResources::new(catalogue.clone(), DeviceScale::UNZOOMED);
    let handle = (0..catalogue.decoration_count())
        .map(|index| DecorationRef::new(index as u64))
        .find(|handle| {
            catalogue
                .decoration(*handle)
                .is_some_and(|entry| entry.fill.is_some())
        })
        .expect("the slide's rectangle issued a decoration");
    match resources.decoration(handle) {
        Resolved::Answered(decoration) => decoration,
        other => panic!(
            "the translucent shape's decoration answered {other:?}. Every part of it resolves — a \
             triplet, a width and two effect colours — so it is whole, and in particular it is no \
             longer `Partial` with a `PaintApproximated` for the opacity nobody could carry."
        ),
    }
}

/// The colour of a fill style, or a panic naming what it was instead.
#[track_caller]
fn solid(fill: &FillStyle) -> Color {
    match fill {
        FillStyle::Solid(color) => *color,
        other => panic!("expected a solid fill, got {other:?}"),
    }
}

/// `color_of` reads the resolved translucent form rather than refusing it.
#[test]
fn a_transformed_colour_is_read_as_a_colour_with_an_alpha() {
    let spec = ColorSpec::Srgb("1F3864".to_owned()).with_alpha(Fraction::from_ratio(0.25));
    assert_eq!(
        color_of(&spec),
        Ok(Color {
            red: 0x1F,
            green: 0x38,
            blue: 0x64,
            alpha: QUARTER,
        }),
        "a resolved colour under one `a:alpha` is a colour and an opacity; refusing it draws \
         nothing where the document asked for a translucent shape"
    );
}

/// An opaque colour is still opaque, so the arm above did not simply invent an alpha.
#[test]
fn a_plain_triplet_is_still_opaque() {
    assert_eq!(
        color_of(&ColorSpec::Srgb("1F3864".to_owned())),
        Ok(Color {
            red: 0x1F,
            green: 0x38,
            blue: 0x64,
            alpha: OPAQUE,
        })
    );
}

/// A transform chain this crate cannot evaluate is still refused, so nothing is guessed.
///
/// Only `a:alpha` survives resolution; a `Transformed` carrying anything else arrived from
/// somewhere that did not resolve it, and painting its base would put a shape on screen in a colour
/// the document does not state.
#[test]
fn a_transform_that_is_not_an_alpha_is_still_not_resolved() {
    let spec = ColorSpec::Transformed {
        base: Box::new(ColorSpec::Srgb("FF0000".to_owned())),
        transforms: vec![ColorTransform::LuminanceModulation(Fraction::from_ratio(
            0.5,
        ))],
    };
    assert_eq!(color_of(&spec), Err(SceneLossKind::ColourNotResolved));
}

/// The slide's fill reaches the scene at the opacity the file states.
#[test]
fn the_fills_alpha_reaches_the_scene() {
    let decoration = decoration();
    assert_eq!(
        solid(&decoration.fill),
        Color {
            red: 0x1F,
            green: 0x38,
            blue: 0x64,
            alpha: QUARTER,
        },
        "the rectangle's fill is `1F3864` at 25 %"
    );
}

/// The slide's outline reaches the scene at a *different* opacity from its fill.
#[test]
fn the_strokes_alpha_reaches_the_scene_and_differs_from_the_fills() {
    let decoration = decoration();
    let stroke = decoration
        .stroke
        .as_ref()
        .expect("the shape states an outline");
    assert_eq!(
        solid(&stroke.fill),
        Color {
            red: 0xC0,
            green: 0x00,
            blue: 0x00,
            alpha: HALF,
        },
        "the outline is `C00000` at 50 %"
    );
    assert_ne!(
        solid(&stroke.fill).alpha,
        solid(&decoration.fill).alpha,
        "the fill and the outline state different opacities, so a build that carried one value \
         everywhere would make them equal"
    );
}

/// Each effect colour reaches the scene with its own opacity, and an effect that states none is opaque.
#[test]
fn every_effect_colour_reaches_the_scene_with_its_own_alpha() {
    let decoration = decoration();
    let alphas: Vec<u8> = decoration
        .effects
        .iter()
        .map(|effect| solid(&effect.fill).alpha)
        .collect();
    assert_eq!(
        alphas,
        vec![THREE_QUARTERS, OPAQUE],
        "the glow states 75 % and the shadow states nothing, so the chain carries 0xBF then 0xFF; \
         the effects are in `CT_EffectList` order, glow before outer shadow"
    );
    let glow = decoration.effects.first().expect("the glow");
    assert_eq!(
        solid(&glow.fill),
        Color {
            red: 0xFF,
            green: 0xC0,
            blue: 0x00,
            alpha: THREE_QUARTERS,
        },
        "the glow's colour arrived as {:?}",
        solid(&glow.fill)
    );
}

/// Four different opacities in one decoration, which is what a stuck implementation cannot produce.
#[test]
fn the_slide_carries_four_distinct_opacities() {
    let decoration = decoration();
    let mut alphas: Vec<u8> = vec![solid(&decoration.fill).alpha];
    if let Some(stroke) = decoration.stroke.as_ref() {
        alphas.push(solid(&stroke.fill).alpha);
    }
    alphas.extend(
        decoration
            .effects
            .iter()
            .map(|effect| solid(&effect.fill).alpha),
    );
    alphas.sort_unstable();
    alphas.dedup();
    assert_eq!(
        alphas,
        vec![QUARTER, HALF, THREE_QUARTERS, OPAQUE],
        "the slide states four opacities and the scene carries {alphas:?}"
    );
}
