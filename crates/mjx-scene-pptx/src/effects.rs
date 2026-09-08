//! Effects: `mjx-dml`'s [`EffectListSpec`] as `mjx-scene`'s effect DAG.
//!
//! # `a:effectLst` is a chain, and ECMA-376 says in what order
//!
//! ECMA-376 Part 1 §20.1.8.26 gives `CT_EffectList`'s children a fixed sequence — `blur`,
//! `fillOverlay`, `glow`, `innerShdw`, `outerShdw`, `prstShdw`, `reflection`, `softEdge` — and says
//! the effects are applied **in that order**, each to the result of the one before. That is why an
//! `a:effectDag` exists at all: the list form is the linear case.
//!
//! [`mjx_scene::EffectStyle`] models exactly that: `input` names an earlier entry of the same
//! decoration, the last entry is the root, and `None` means *the subtree itself*. So the
//! translation is a chain — entry zero consumes the subtree, entry *n* consumes entry *n − 1* — and
//! the order is the schema's, not one chosen here.
//!
//! **Getting this wrong is invisible.** A list of effects each stating `input: None` would leave
//! every entry but the last unreachable from the root, and
//! [`SceneBuilder::add_effect_styles`](mjx_scene::SceneBuilder::add_effect_styles) answers with the
//! *last* index — so a shape with a glow and a shadow would render its shadow and silently lose its
//! glow, with no error anywhere. `tests/every_effect_reaches_the_root.rs` is what refuses that.
//!
//! # ⚠ Every effect colour here is opaque, and none of them should be
//!
//! An `a:outerShdw`'s colour is almost always alpha'd — the standard Office theme's effect style is
//! `<a:srgbClr val="000000"><a:alpha val="63000"/></a:srgbClr>` — and `mjx-dml`'s `resolve_effects`
//! bakes that to a `ColorSpec::Srgb` hex triplet, which has no alpha. So a shadow drawn from this
//! translation is a **solid black** shadow rather than a 63% one. That is the sharpest consequence
//! of the loss described in [`crate::paint`], and it is why
//! `tests/the_opacity_is_lost_at_the_spec_boundary.rs` reads the loss off a real fixture rather
//! than describing it.

use mjx_dml::{ColorSpec, EffectListSpec};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_scene::{
    pixels_from_emu, BlendMode, DeviceScale, EffectKind, EffectStyle, FillStyle, RectangleAnchor,
};

use crate::paint::{color_of, fill_style};

/// The effect chain a resolved [`EffectListSpec`] names, at `scale`.
///
/// Empty when the list has no effect this build can draw, which is what
/// [`mjx_scene::Decoration::is_invisible`] wants to see rather than a chain of no-ops.
///
/// `image` resolves a fill overlay's picture, as [`fill_style`] takes it.
#[must_use]
pub fn effect_styles(
    spec: &EffectListSpec,
    scale: DeviceScale,
    image: &dyn Fn(&str) -> Option<u64>,
) -> Vec<EffectStyle> {
    let mut chain: Vec<EffectStyle> = Vec::new();

    // The schema's own child order, which is also the order the effects apply in. Each entry
    // consumes the one before it; the first consumes the subtree.
    if let Some(blur) = &spec.blur {
        let mut effect = base(EffectKind::Blur, &chain);
        effect.radius = pixels(blur.radius.unwrap_or(Emu::from_emu(0)), scale);
        effect.grow = blur.grow.unwrap_or(true);
        chain.push(effect);
    }
    if let Some(overlay) = &spec.fill_overlay {
        let mut effect = base(EffectKind::FillOverlay, &chain);
        effect.fill = fill_style(&overlay.fill, image);
        effect.blend = blend_mode(overlay.blend);
        chain.push(effect);
    }
    if let Some(glow) = &spec.glow {
        let mut effect = base(EffectKind::Glow, &chain);
        effect.fill = solid(&glow.color);
        effect.radius = pixels(glow.radius.unwrap_or(Emu::from_emu(0)), scale);
        chain.push(effect);
    }
    if let Some(shadow) = &spec.inner_shadow {
        let mut effect = base(EffectKind::InnerShadow, &chain);
        effect.fill = solid(&shadow.color);
        effect.radius = pixels(shadow.blur_radius.unwrap_or(Emu::from_emu(0)), scale);
        effect.distance = pixels(shadow.distance.unwrap_or(Emu::from_emu(0)), scale);
        effect.direction = radians(shadow.direction);
        chain.push(effect);
    }
    if let Some(shadow) = &spec.outer_shadow {
        let mut effect = base(EffectKind::OuterShadow, &chain);
        effect.fill = solid(&shadow.color);
        effect.radius = pixels(shadow.blur_radius.unwrap_or(Emu::from_emu(0)), scale);
        effect.distance = pixels(shadow.distance.unwrap_or(Emu::from_emu(0)), scale);
        effect.direction = radians(shadow.direction);
        effect.scale_x = fraction(shadow.scale_x, 1.0);
        effect.scale_y = fraction(shadow.scale_y, 1.0);
        effect.skew_x = radians(shadow.skew_x);
        effect.skew_y = radians(shadow.skew_y);
        effect.rotate_with_shape = shadow.rotate_with_shape.unwrap_or(true);
        effect.anchor = anchor_of(shadow.alignment);
        chain.push(effect);
    }
    if let Some(shadow) = &spec.preset_shadow {
        // GUESS: one of the twenty `a:prstShdw` presets is drawn as an ordinary outer shadow at the
        // distance and direction it states. The presets differ in blur and in perspective skew, and
        // ECMA-376 names them without defining any of their geometry — so drawing the family's
        // shared shape is the honest reading, and which preset differs how is a question for the
        // Windows sitting.
        let mut effect = base(EffectKind::OuterShadow, &chain);
        effect.fill = solid(&shadow.color);
        effect.distance = pixels(shadow.distance.unwrap_or(Emu::from_emu(0)), scale);
        effect.direction = radians(shadow.direction);
        chain.push(effect);
    }
    if let Some(reflection) = &spec.reflection {
        let mut effect = base(EffectKind::Reflection, &chain);
        effect.radius = pixels(reflection.blur_radius.unwrap_or(Emu::from_emu(0)), scale);
        effect.distance = pixels(reflection.distance.unwrap_or(Emu::from_emu(0)), scale);
        effect.direction = radians(reflection.direction);
        effect.start_alpha = fraction(reflection.start_alpha, 1.0);
        effect.start_position = fraction(reflection.start_position, 0.0);
        effect.end_alpha = fraction(reflection.end_alpha, 0.0);
        effect.end_position = fraction(reflection.end_position, 1.0);
        // `@fadeDir`'s schema default is 5 400 000 sixty-thousandths of a degree — 90°, straight
        // down, which is the direction a reflection fades in every document that does not say.
        effect.fade_direction = reflection
            .fade_direction
            .map_or(std::f32::consts::FRAC_PI_2, |angle| angle.radians() as f32);
        effect.scale_x = fraction(reflection.scale_x, 1.0);
        effect.scale_y = fraction(reflection.scale_y, 1.0);
        effect.skew_x = radians(reflection.skew_x);
        effect.skew_y = radians(reflection.skew_y);
        effect.rotate_with_shape = reflection.rotate_with_shape.unwrap_or(true);
        effect.anchor = anchor_of(reflection.alignment);
        chain.push(effect);
    }
    if let Some(soft_edge) = &spec.soft_edge {
        let mut effect = base(EffectKind::SoftEdge, &chain);
        effect.radius = pixels(soft_edge.radius, scale);
        chain.push(effect);
    }

    chain
}

/// An effect of `kind` consuming whatever the chain has produced so far.
///
/// The one place the chain is built, so that *"each effect consumes the one before it"* is written
/// once. An entry added without going through this would consume the subtree instead, which is the
/// defect this module's own documentation describes.
fn base(kind: EffectKind, chain: &[EffectStyle]) -> EffectStyle {
    EffectStyle {
        kind,
        input: chain.len().checked_sub(1),
        fill: FillStyle::None,
        radius: 0.0,
        distance: 0.0,
        direction: 0.0,
        scale_x: 1.0,
        scale_y: 1.0,
        skew_x: 0.0,
        skew_y: 0.0,
        start_alpha: 0.0,
        start_position: 0.0,
        end_alpha: 0.0,
        end_position: 0.0,
        fade_direction: 0.0,
        grow: true,
        rotate_with_shape: false,
        anchor: RectangleAnchor::BottomLeft,
        blend: BlendMode::Over,
    }
}

/// A solid fill of the colour an effect states, or nothing when it states none this build can read.
fn solid(color: &ColorSpec) -> FillStyle {
    color_of(color).map_or(FillStyle::None, FillStyle::Solid)
}

/// A length in device pixels.
fn pixels(length: Emu, scale: DeviceScale) -> f32 {
    pixels_from_emu(length, scale)
}

/// An angle in radians, or zero when the effect states none — which is `@dir`'s schema default.
fn radians(angle: Option<Angle>) -> f32 {
    angle.map_or(0.0, |angle| angle.radians() as f32)
}

/// A `a:...@sx`-style fraction, or `fallback` when the effect states none.
fn fraction(value: Option<mjx_dml::Fraction>, fallback: f32) -> f32 {
    value.map_or(fallback, |value| value.ratio() as f32)
}

/// Which corner a shadow or reflection is scaled and skewed about.
fn anchor_of(alignment: Option<mjx_dml::RectangleAlignment>) -> RectangleAnchor {
    use mjx_dml::RectangleAlignment;

    match alignment {
        Some(RectangleAlignment::TopLeft) => RectangleAnchor::TopLeft,
        Some(RectangleAlignment::Top) => RectangleAnchor::Top,
        Some(RectangleAlignment::TopRight) => RectangleAnchor::TopRight,
        Some(RectangleAlignment::Left) => RectangleAnchor::Left,
        Some(RectangleAlignment::Center) => RectangleAnchor::Center,
        Some(RectangleAlignment::Right) => RectangleAnchor::Right,
        Some(RectangleAlignment::BottomRight) => RectangleAnchor::BottomRight,
        // `@algn`'s schema default is `b`, the bottom edge's middle.
        Some(RectangleAlignment::BottomLeft) => RectangleAnchor::BottomLeft,
        Some(RectangleAlignment::Bottom) | None => RectangleAnchor::Bottom,
    }
}

/// How a fill overlay is composited over what it covers.
fn blend_mode(blend: mjx_dml::BlendMode) -> BlendMode {
    match blend {
        mjx_dml::BlendMode::Multiply => BlendMode::Multiply,
        mjx_dml::BlendMode::Screen => BlendMode::Screen,
        mjx_dml::BlendMode::Darken => BlendMode::Darken,
        mjx_dml::BlendMode::Lighten => BlendMode::Lighten,
        mjx_dml::BlendMode::Over => BlendMode::Over,
    }
}
