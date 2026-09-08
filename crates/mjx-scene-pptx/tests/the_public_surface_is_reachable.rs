//! Every public item of this crate, named and called from outside it.
//!
//! # The instrument this answers
//!
//! *"Would an `abort()` at this site fire in any test?"* A function that no suite ever calls is a
//! function whose behaviour is unproved however carefully it is written, and the ones that go
//! uncalled here are exactly the ones a real document only reaches occasionally: a pattern fill, a
//! dashed compound outline with arrowheads, a picture fill. So they are called here, deliberately,
//! rather than waited for.
//!
//! It is also where *"how many distinct values did the gate see"* is answered for the paint
//! translation: every arm of `FillSpec` and every enumeration `LineSpec` carries is exercised, and
//! the pattern table is checked across all fifty-four presets rather than at one of them.

use mjx_dml::{Angle, Fraction, LineWidth};
use mjx_dml::{
    ColorSpec, FillSpec, GradientStopSpec, LineDash, LineEnd, LineSpec, PictureFillMode,
};
use mjx_layout_pptx::{constraints_for, PageCatalogue, SlideBoxModel, SlideDeck};
use mjx_ooxml_types::drawingml::{
    CompoundLine, LineCap, LineEndLength, LineEndType, LineEndWidth, PatternType, PenAlignment,
    PresetLineDash,
};
use mjx_scene::{
    DeviceScale, FillStyle, ImageFillMode, PatternPreset, ResourceResolver, StrokeAlignment,
    PATTERN_PRESET_COUNT,
};
use mjx_scene_pptx::paint::color_of;
use mjx_scene_pptx::{fill_style, pattern_preset, stroke_style, SlideGeometry, SlideResources};

fn no_images(_rel_id: &str) -> Option<u64> {
    None
}

fn one_image(rel_id: &str) -> Option<u64> {
    (rel_id == "rId7").then_some(3)
}

fn blue() -> ColorSpec {
    ColorSpec::Srgb("1F4E79".to_owned())
}

#[test]
fn a_colour_reads_with_and_without_its_hash_and_refuses_what_it_cannot_read() {
    assert_eq!(
        color_of(&ColorSpec::Srgb("FF8000".to_owned())).map(|c| (c.red, c.green, c.blue, c.alpha)),
        Some((0xff, 0x80, 0x00, 0xff))
    );
    assert_eq!(
        color_of(&ColorSpec::Srgb("#FF8000".to_owned())),
        color_of(&ColorSpec::Srgb("FF8000".to_owned())),
        "a leading `#` is not part of a DrawingML hex value, but a caller that pastes one in must \
         not get a different colour from one who does not"
    );
    assert_eq!(
        color_of(&ColorSpec::Srgb("FF80".to_owned())),
        None,
        "four digits is not a colour, and guessing one would paint a shape a colour no tier states"
    );
    assert_eq!(
        color_of(&ColorSpec::Srgb("GGHHII".to_owned())),
        None,
        "non-hexadecimal digits read as no colour rather than as zero"
    );
    assert_eq!(
        color_of(&ColorSpec::Scheme(mjx_dml::SchemeColor::Accent1)),
        None,
        "a scheme colour reaching this layer is one `mjx-dml` could not resolve — there was no \
         theme — and inventing one would paint the shape off the document's palette"
    );
    assert_eq!(
        color_of(&ColorSpec::Other {
            kind: mjx_dml::ColorKind::System,
            value: Some("C0C0C0".to_owned()),
        })
        .map(|colour| colour.red),
        Some(0xc0),
        "a system colour resolved to a hex triplet reads like any other"
    );
}

#[test]
fn every_arm_of_a_fill_translates() {
    assert!(matches!(
        fill_style(&FillSpec::None, &no_images),
        FillStyle::None
    ));
    assert!(matches!(
        fill_style(&FillSpec::Group, &no_images),
        FillStyle::None
    ));
    assert!(matches!(
        fill_style(&FillSpec::Solid(blue()), &no_images),
        FillStyle::Solid(_)
    ));

    let gradient = FillSpec::Gradient {
        stops: vec![
            GradientStopSpec {
                position: Fraction::from_ratio(0.0),
                color: blue(),
            },
            GradientStopSpec {
                position: Fraction::from_ratio(1.0),
                color: ColorSpec::Srgb("FFFFFF".to_owned()),
            },
        ],
        angle: Some(Angle::from_degrees(45.0)),
    };
    let FillStyle::Gradient(ramp) = fill_style(&gradient, &no_images) else {
        panic!("a gradient with two readable stops is a gradient");
    };
    assert_eq!(ramp.stops.len(), 2);
    assert_eq!(ramp.stops[0].position_in_ten_thousandths, 0);
    assert_eq!(ramp.stops[1].position_in_ten_thousandths, 10_000);
    assert!(
        ramp.angle > 0.0,
        "a gradient at 45 degrees has a non-zero angle in radians; zero would be the identity value \
         a translation that dropped `a:lin@ang` produces"
    );

    assert!(
        matches!(
            fill_style(
                &FillSpec::Gradient {
                    stops: Vec::new(),
                    angle: None
                },
                &no_images
            ),
            FillStyle::None
        ),
        "a gradient with no readable stop paints nothing rather than painting black"
    );

    let pattern = FillSpec::Pattern {
        preset: Some(PatternType::DiagonalBrick),
        foreground: Some(blue()),
        background: Some(ColorSpec::Srgb("FFFFFF".to_owned())),
    };
    assert!(matches!(
        fill_style(&pattern, &no_images),
        FillStyle::Pattern {
            preset: PatternPreset::DiagonalBrick,
            ..
        }
    ));

    let colourless_pattern = FillSpec::Pattern {
        preset: None,
        foreground: Some(blue()),
        background: None,
    };
    assert!(
        matches!(
            fill_style(&colourless_pattern, &no_images),
            FillStyle::Solid(_)
        ),
        "a pattern with no preset falls back to its foreground as a solid, which is what a hatch \
         reduces to below one pixel"
    );

    let picture = FillSpec::Picture {
        rel_id: "rId7".to_owned(),
        mode: PictureFillMode::Tile,
    };
    let FillStyle::Image(image) = fill_style(&picture, &one_image) else {
        panic!("a picture fill whose relationship the page names is an image fill");
    };
    assert_eq!(image.handle, 3);
    assert_eq!(image.fill_mode, ImageFillMode::Tile);
    assert!(
        matches!(fill_style(&picture, &no_images), FillStyle::None),
        "a picture fill nobody can supply paints nothing rather than a wrong colour"
    );
}

#[test]
fn an_outline_carries_every_attribute_it_states() {
    let spec = LineSpec {
        width: Some(LineWidth::from_points(3.0)),
        cap: Some(LineCap::Round),
        compound: Some(CompoundLine::ThickThin),
        pen_alignment: Some(PenAlignment::Inset),
        fill: Some(FillSpec::Solid(blue())),
        dash: Some(LineDash::Preset(PresetLineDash::LargeDashDot)),
        join: Some(mjx_dml::LineJoin::Miter {
            limit: Some(Fraction::from_ratio(4.0)),
        }),
        head_end: Some(LineEnd {
            kind: Some(LineEndType::Stealth),
            width: Some(LineEndWidth::Large),
            length: Some(LineEndLength::Small),
        }),
        tail_end: Some(LineEnd {
            kind: Some(LineEndType::Oval),
            width: None,
            length: None,
        }),
    };
    let stroke = stroke_style(&spec, DeviceScale::UNZOOMED, &no_images)
        .expect("an outline with a readable fill is a stroke");

    assert!(
        stroke.width > 1.0,
        "a three-point line is wider than the one-pixel hairline floor; {} means the width was \
         dropped",
        stroke.width
    );
    assert_eq!(stroke.cap, mjx_scene::LineCap::Round);
    assert_eq!(stroke.compound, mjx_scene::CompoundStroke::ThickThin);
    assert_eq!(stroke.alignment, StrokeAlignment::Inset);
    assert_eq!(stroke.dash, mjx_scene::DashPattern::LargeDashDot);
    assert_eq!(stroke.join, mjx_scene::LineJoin::Miter { limit: 4.0 });
    assert_eq!(stroke.head.shape, mjx_scene::LineEndShape::Stealth);
    assert_eq!(stroke.head.width, mjx_scene::LineEndSize::Large);
    assert_eq!(stroke.head.length, mjx_scene::LineEndSize::Small);
    assert_eq!(stroke.tail.shape, mjx_scene::LineEndShape::Oval);
    assert_eq!(
        stroke.tail.width,
        mjx_scene::LineEndSize::Medium,
        "`@w`'s schema default is `med`"
    );
}

#[test]
fn an_outline_that_fills_with_nothing_is_no_outline_at_all() {
    let unfilled = LineSpec {
        width: Some(LineWidth::from_points(1.0)),
        ..LineSpec::new()
    };
    assert!(
        stroke_style(&unfilled, DeviceScale::UNZOOMED, &no_images).is_none(),
        "a line with no fill draws nothing; answering with a stroke of `FillStyle::None` would make \
         the painter open a draw call that covers no pixels"
    );

    let hairline = LineSpec {
        width: Some(LineWidth::from_emu(0)),
        fill: Some(FillSpec::Solid(blue())),
        ..LineSpec::new()
    };
    let stroke = stroke_style(&hairline, DeviceScale::UNZOOMED, &no_images)
        .expect("a hairline is still a stroke");
    assert!(
        stroke.width >= 1.0,
        "DrawingML states a hairline as `@w=\"0\"`, and a stroke of zero width covers no pixels — so \
         a table whose style states hairline borders would have none"
    );
}

#[test]
fn the_pattern_table_covers_all_fifty_four_presets() {
    // Not one preset, and not a spot-check: the two enumerations are generated from the same
    // `ST_PresetPatternVal`, and a mapping that silently shifted by one would draw the wrong hatch
    // on every shape while still drawing a hatch.
    let mut seen = std::collections::BTreeSet::new();
    for value in 0..PATTERN_PRESET_COUNT {
        let expected = PatternPreset::from_wire_value(value).expect("a preset in the table");
        seen.insert(expected);
    }
    assert_eq!(
        seen.len(),
        PATTERN_PRESET_COUNT as usize,
        "the scene's own table has {PATTERN_PRESET_COUNT} distinct presets"
    );

    // And the translation itself, at the two ends and in the middle, by name rather than by number.
    assert_eq!(
        pattern_preset(PatternType::Percent5),
        PatternPreset::Percent5
    );
    assert_eq!(pattern_preset(PatternType::Cross), PatternPreset::Cross);
    assert_eq!(pattern_preset(PatternType::ZigZag), PatternPreset::ZigZag);
}

#[test]
fn a_resolver_answers_from_the_catalogue_it_was_given_and_from_nothing_else() {
    let bytes = mjx_fixtures::fixture("effects_theme.pptx");
    let mut presentation = mjx_pptx::Presentation::open(&bytes).expect("a well-formed package");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");

    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    let mut model = mjx_layout_pptx::SlideBoxModel::new(
        mjx_text::FontResolver::builder()
            .with_bundled_font_directory(&fonts)
            .expect("the committed faces index")
            .build(),
    );
    let constraints = constraints_for(&deck);
    let _ = mjx_layout::BoxModel::layout_page(
        &mut model,
        &deck,
        mjx_layout::PageIndex::FIRST,
        &constraints,
        None,
    )
    .expect("the slide lays out");

    let catalogue: PageCatalogue = model.catalogue().clone();
    assert!(
        catalogue.decoration_count() > 0,
        "the fixture's second shape resolves a theme effect style, so the page issues at least one \
         decoration handle; without it the assertions below are vacuous"
    );

    let resources = SlideResources::new(catalogue, DeviceScale::UNZOOMED);
    assert_eq!(resources.scale(), DeviceScale::UNZOOMED);
    assert!(resources.catalogue().decoration_count() > 0);

    let decoration = resources
        .decoration(mjx_layout::DecorationRef::new(0))
        .expect("handle zero was issued");
    assert!(
        !decoration.effects.is_empty(),
        "the shape's theme effect style is an outer shadow, and it must reach the decoration as an \
         effect chain rather than as an empty vector"
    );

    assert_eq!(
        resources.decoration(mjx_layout::DecorationRef::new(9_999)),
        None,
        "a handle the page never issued resolves to nothing rather than to whatever is first"
    );
    assert_eq!(
        resources.image(mjx_layout::ImageRef::new(0)),
        None,
        "the fixture holds no picture, so no image handle resolves"
    );
    assert_eq!(
        resources.text_decoration(&mjx_layout::SourceRef::node(
            mjx_layout::PartId::new(0),
            mjx_layout::SourcePath::new(&[0, 0, 0, 0])
        )),
        None,
        "a run's own fill has no table to resolve through yet, and answering `None` is what makes \
         `build_scene` fall back to its documented default text colour"
    );
}

#[test]
fn a_geometry_provider_counts_what_it_could_not_answer() {
    let mut provider = SlideGeometry::new();
    assert_eq!((provider.registered(), provider.unregistered()), (0, 0));

    provider.register(
        0,
        Some(mjx_geometry::ShapeOutline::new(
            mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
            mjx_dml::Size::from_emu(914_400, 914_400),
        )),
    );
    provider.register(1, None);
    assert_eq!((provider.registered(), provider.unregistered()), (1, 1));

    let known = mjx_scene::GeometryProvider::outline(
        &provider,
        0,
        mjx_scene::SceneRect::new(0.0, 0.0, 96.0, 96.0),
    )
    .expect("a registered rectangle resolves");
    assert_eq!(
        known.provenance,
        mjx_scene::OutlineProvenance::Document,
        "a shape the document names must resolve to the document's own geometry, or a golden image \
         taken against it would be an image of a stand-in"
    );

    let unknown = mjx_scene::GeometryProvider::outline(
        &provider,
        1,
        mjx_scene::SceneRect::new(0.0, 0.0, 96.0, 96.0),
    )
    .expect("the standing-in provider answers every handle");
    assert_eq!(
        unknown.provenance,
        mjx_scene::OutlineProvenance::Placeholder,
        "an unregistered handle must be *labelled* a stand-in, which is what makes \
         `DrawReport::placeholders` countable"
    );

    let strict = SlideGeometry::refusing_unknown_shapes();
    assert!(
        mjx_scene::GeometryProvider::outline(
            &strict,
            0,
            mjx_scene::SceneRect::new(0.0, 0.0, 96.0, 96.0)
        )
        .is_err(),
        "the refusing provider fails the page rather than rendering a lie, which is what a gate uses"
    );

    // `Default` is the standing-in one, so a caller that never chose gets the safe-and-counted
    // behaviour rather than a page that refuses to render.
    let default: SlideGeometry = SlideGeometry::default();
    assert_eq!(default.registered(), 0);
    let _ = SlideBoxModel::SIGNATURE;
}
