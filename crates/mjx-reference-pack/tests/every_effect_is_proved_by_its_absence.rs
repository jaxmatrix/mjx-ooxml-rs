//! Each of DrawingML's seven effects, drawn and not drawn, with the pixels compared.
//!
//! # Why "it rendered" is not evidence for an effect
//!
//! An effect that silently no-ops produces an image **very close** to one with the effect applied:
//! the shape is still there, in the right place, in the right colour, and only the halo or the blur
//! around it is missing. A perceptual comparison with a loose tolerance passes that. So does a
//! structural check that the effect reached the display list, because reaching the list and being
//! *drawn* are different things — `mjx_scene::SceneBuilder::add_effect_styles` returns the last
//! entry's index, and an entry nothing reaches is encoded and never executed.
//!
//! The only assertion that cannot be satisfied by an effect doing nothing is a **difference**: the
//! same scene, once with the effect and once without, has to produce different pixels. This suite
//! turns each of the seven off in turn and requires exactly that. An effect whose translation
//! quietly becomes a no-op fails here and nowhere else in the workspace.
//!
//! # The identity-value trap, applied to this suite itself
//!
//! Every effect below states a **real** radius, distance and colour. An effect whose alphas are all
//! zero draws nothing, an effect whose radius is zero blurs nothing, and a suite built out of those
//! would compare two identical images and report agreement. So the parameters are stated explicitly
//! and [`the_parameters_are_not_identity_values`] refuses a chain whose numbers have collapsed.
//!
//! MJX-STAND-IN: every fragment here is a `Fragment::Box`, which `build_scene` encodes as a
//! `Geometry::Rectangle` — so the geometry provider is never asked a single question, and the
//! `PlaceholderGeometry` below is a required argument rather than a source of outlines. Using the
//! real preset tables would need a document, and this suite deliberately has none: it is about the
//! effect that wraps a shape, and a stand-in for the shape itself would be indistinguishable from
//! the rectangle every one of these scenes actually draws.

use mjx_dml::{
    BlurEffect, ColorSpec, EffectListSpec, FillOverlayEffect, FillSpec, GlowEffect,
    InnerShadowEffect, OuterShadowEffect, ReflectionEffect, SoftEdgeEffect,
};
use mjx_layout::{
    BoxFragment, DecorationRef, Fragment, FragmentTree, FragmentTreeBuilder, ImageRef, LayoutPoint,
    LayoutRect, LayoutSize, PartId, SourcePath, SourceRef,
};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_ooxml_types::drawingml::BlendMode;
use mjx_paint::{render_offscreen, NoGlyphs, NoImages, Pixels, Resources, SoftwarePainter};
use mjx_scene::{
    build_scene, Decoration, DeviceScale, EffectKind, FillStyle, PlaceholderGeometry,
    ResourceResolver, SceneOptions,
};
use mjx_scene_pptx::effect_styles;
use mjx_text::{GlyphAtlas, GlyphRasteriser};

/// The page every scene below is drawn on: two inches square, so a shape's shadow has room to fall
/// outside it and still be inside the image.
const PAGE: i64 = 1_828_800;
/// The shape: one inch square, in the middle, so every effect's halo lands on the page rather than
/// off it.
const SHAPE: LayoutRect = LayoutRect {
    left: Emu::from_emu(457_200),
    top: Emu::from_emu(457_200),
    right: Emu::from_emu(1_371_600),
    bottom: Emu::from_emu(1_371_600),
};

/// A colour that is neither the background nor black, so that a difference is a difference in the
/// effect rather than in the shape.
fn shape_colour() -> ColorSpec {
    ColorSpec::Srgb("2E5C8A".to_owned())
}

/// A colour for the effect itself, distinct from the shape's, so a shadow that drew in the shape's
/// colour on top of the shape would still register as a difference.
fn effect_colour() -> ColorSpec {
    ColorSpec::Srgb("C8501E".to_owned())
}

/// The resolver for the one-shape scenes below.
struct OneShape {
    decoration: Decoration,
}

impl ResourceResolver for OneShape {
    fn decoration(&self, _reference: DecorationRef) -> Option<Decoration> {
        Some(self.decoration.clone())
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        None
    }

    fn image(&self, _reference: ImageRef) -> Option<mjx_scene::Image> {
        None
    }
}

/// One filled box on a page, with `decoration` applied to it.
fn tree() -> FragmentTree {
    let mut builder = FragmentTreeBuilder::new();
    let page = builder.push_simple(
        None,
        SourceRef::node(PartId::new(0), SourcePath::new(&[0])),
        LayoutRect::from_origin_and_size(
            LayoutPoint::ORIGIN,
            LayoutSize::new(Emu::from_emu(PAGE), Emu::from_emu(PAGE)),
        ),
        Fragment::Box(BoxFragment {
            decoration: None,
            cell: None,
        }),
    );
    builder.push_simple(
        page,
        SourceRef::node(PartId::new(0), SourcePath::new(&[0, 0])),
        SHAPE,
        Fragment::Box(BoxFragment {
            decoration: Some(DecorationRef::new(0)),
            cell: None,
        }),
    );
    builder.finish()
}

/// Renders the one-shape page with `effects` applied.
fn render(effects: &EffectListSpec) -> Pixels {
    let scale = DeviceScale::UNZOOMED;
    let no_images = |_: &str| None;
    let decoration = Decoration {
        fill: FillStyle::Solid(
            mjx_scene_pptx::paint::color_of(&shape_colour()).expect("a readable colour"),
        ),
        stroke: None,
        opacity: 1.0,
        effects: effect_styles(effects, scale, &no_images),
    };
    let resolver = OneShape { decoration };

    let tree = tree();
    let mut options = SceneOptions::new(LayoutSize::new(Emu::from_emu(PAGE), Emu::from_emu(PAGE)));
    options.device_scale = scale;

    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let list = build_scene(&tree, &resolver, &mut rasteriser, &mut atlas, &options)
        .expect("the scene builds");

    let (width, height) = list.page_size();
    let mut painter = SoftwarePainter::new();
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let geometry = PlaceholderGeometry::new();
    let mut resources = Resources::new(&mut glyphs, &geometry, &images);
    render_offscreen(
        &mut painter,
        &list,
        width.ceil().max(1.0) as u32,
        height.ceil().max(1.0) as u32,
        1.0,
        &mut resources,
    )
    .expect("the list rasterises")
    .pixels
}

/// How many pixels of `left` and `right` differ at all.
fn differing(left: &Pixels, right: &Pixels) -> usize {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "two renders of the same page have to be the same size before a comparison means anything"
    );
    let mut count = 0;
    for y in 0..left.height {
        for x in 0..left.width {
            if left.pixel(x, y) != right.pixel(x, y) {
                count += 1;
            }
        }
    }
    count
}

/// An effect list holding exactly one effect, of `kind`, with parameters that are not no-ops.
fn only(kind: EffectKind) -> EffectListSpec {
    let mut spec = EffectListSpec::new();
    match kind {
        EffectKind::Blur => {
            spec.blur = Some(BlurEffect {
                radius: Some(Emu::from_emu(120_000)),
                grow: Some(true),
            });
        }
        EffectKind::Glow => {
            spec.glow = Some(GlowEffect {
                color: effect_colour(),
                radius: Some(Emu::from_emu(140_000)),
            });
        }
        EffectKind::OuterShadow => {
            spec.outer_shadow = Some(OuterShadowEffect {
                color: effect_colour(),
                blur_radius: Some(Emu::from_emu(100_000)),
                distance: Some(Emu::from_emu(150_000)),
                direction: Some(Angle::from_degrees(45.0)),
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: None,
                rotate_with_shape: None,
            });
        }
        EffectKind::InnerShadow => {
            spec.inner_shadow = Some(InnerShadowEffect {
                color: effect_colour(),
                blur_radius: Some(Emu::from_emu(90_000)),
                distance: Some(Emu::from_emu(120_000)),
                direction: Some(Angle::from_degrees(135.0)),
            });
        }
        EffectKind::SoftEdge => {
            spec.soft_edge = Some(SoftEdgeEffect {
                radius: Emu::from_emu(150_000),
            });
        }
        EffectKind::Reflection => {
            spec.reflection = Some(ReflectionEffect {
                blur_radius: Some(Emu::from_emu(40_000)),
                start_alpha: Some(mjx_dml::Fraction::from_ratio(0.8)),
                start_position: Some(mjx_dml::Fraction::from_ratio(0.0)),
                end_alpha: Some(mjx_dml::Fraction::from_ratio(0.1)),
                end_position: Some(mjx_dml::Fraction::from_ratio(0.9)),
                distance: Some(Emu::from_emu(20_000)),
                direction: None,
                fade_direction: None,
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: None,
                rotate_with_shape: None,
            });
        }
        EffectKind::FillOverlay => {
            spec.fill_overlay = Some(FillOverlayEffect {
                fill: FillSpec::Solid(effect_colour()),
                blend: BlendMode::Multiply,
            });
        }
    }
    spec
}

#[test]
fn turning_each_effect_off_changes_the_pixels() {
    let without = render(&EffectListSpec::new());
    let total = without.width as usize * without.height as usize;
    assert!(total > 0, "the page has a size");

    // A floor in *pixels*, not a ratio, and a generous one: the point is to refuse a no-op, not to
    // pin how large each effect's footprint is. A translation that dropped an effect entirely
    // produces zero differing pixels, which is what this catches.
    let floor = total / 500;

    let mut report = Vec::new();
    for kind in EffectKind::ALL {
        let with = render(&only(kind));
        let changed = differing(&without, &with);
        report.push(format!(
            "{kind:?}: {changed} of {total} pixels ({:.2}%)",
            changed as f64 / total as f64 * 100.0
        ));
        assert!(
            changed > floor,
            "{kind:?} changed {changed} pixels out of {total}, which is at or below the {floor} \
             this suite treats as \"nothing happened\". An effect that draws nothing renders an \
             image very close to one with the effect applied — that is the whole reason this gate \
             compares against the *absence* rather than against a golden image.\n\nEvery effect so \
             far:\n{}",
            report.join("\n")
        );
    }
}

#[test]
fn two_different_effects_do_not_produce_the_same_image() {
    // The other half. Every effect differing from *no* effect is satisfied by a translation that
    // mapped all seven onto one — a shadow drawn wherever the document asked for a glow, say —
    // because each would still differ from the unadorned shape. Two effects that produce identical
    // pixels are one effect wearing two names.
    let mut rendered: Vec<(EffectKind, Pixels)> = Vec::new();
    for kind in EffectKind::ALL {
        rendered.push((kind, render(&only(kind))));
    }

    for (index, (left_kind, left)) in rendered.iter().enumerate() {
        for (right_kind, right) in rendered.iter().skip(index + 1) {
            // `a:prstShdw` is deliberately drawn as an outer shadow (see `mjx_scene_pptx::effects`),
            // but it is not in `EffectKind::ALL` — every pair here is a pair of distinct kinds.
            assert!(
                differing(left, right) > 0,
                "{left_kind:?} and {right_kind:?} rasterise to identical images. Two effect kinds \
                 that draw the same thing are one translation arm serving two elements, and every \
                 comparison against the unadorned shape passes anyway."
            );
        }
    }
}

#[test]
fn the_parameters_are_not_identity_values() {
    // This suite's own instrument, turned on itself: every effect above must state a radius, a
    // distance or a colour that actually does something, or the comparisons are between two copies
    // of the same image and the whole file passes vacuously.
    let no_images = |_: &str| None;
    for kind in EffectKind::ALL {
        let chain = effect_styles(&only(kind), DeviceScale::UNZOOMED, &no_images);
        let entry = chain
            .first()
            .unwrap_or_else(|| panic!("{kind:?} translated to no effect at all"));
        assert_eq!(chain.len(), 1, "{kind:?} was set up as exactly one effect");

        let does_something = entry.radius > 0.0
            || entry.distance > 0.0
            || matches!(entry.fill, FillStyle::Solid(colour) if colour.alpha > 0)
            || entry.start_alpha > 0.0;
        assert!(
            does_something,
            "{kind:?} translated to an effect with no radius, no offset, no paint and no starting \
             alpha. Every number is at its no-op value, so the render with it and the render \
             without it would be the same image — and this suite would report agreement."
        );
    }
}
