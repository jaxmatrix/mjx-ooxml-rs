//! An effect that is translated but never reached draws nothing, and nothing reports it.
//!
//! # The defect this suite exists for
//!
//! [`mjx_scene::SceneBuilder::add_effect_styles`] answers with the index of the **last** entry it
//! placed, and `build_scene` pushes exactly that one as the root of a `PushEffect`. So a chain whose
//! entries each state `input: None` has every entry but the last unreachable: a shape with a glow
//! and a shadow renders its shadow, silently loses its glow, and reports success.
//!
//! That is the exact shape of this programme's second instrument — *a parameter reached constantly
//! but only at its no-op value* — turned inside out: the parameter is reached, the value is real,
//! and the **edge** is missing. Nothing about the rendered page says so. So the chain is asserted
//! structurally here rather than by looking at pixels.
//!
//! # And the same suite is where "how many distinct values did the gate see" is answered
//!
//! One effect proves nothing about a chain: with a single entry, `input: None` and
//! `input: Some(n - 1)` are the same answer. Every count from one to eight is exercised below, and
//! the eight-effect case is the whole of `CT_EffectList`.

use mjx_dml::{
    BlurEffect, ColorSpec, EffectListSpec, FillOverlayEffect, FillSpec, GlowEffect,
    InnerShadowEffect, OuterShadowEffect, PresetShadowEffect, ReflectionEffect, SoftEdgeEffect,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::drawingml::{BlendMode, PresetShadow};
use mjx_scene::{DeviceScale, EffectKind, FillStyle};
use mjx_scene_pptx::effect_styles;

/// A colour that is visibly not black, so a translation that dropped it is visible in an assertion.
fn colour() -> ColorSpec {
    ColorSpec::Srgb("336699".to_owned())
}

fn no_images(_rel_id: &str) -> Option<u64> {
    None
}

/// Every effect `CT_EffectList` can hold, all at once.
fn everything() -> EffectListSpec {
    EffectListSpec {
        blur: Some(BlurEffect {
            radius: Some(Emu::from_emu(50_000)),
            grow: Some(false),
        }),
        fill_overlay: Some(FillOverlayEffect {
            fill: FillSpec::Solid(colour()),
            blend: BlendMode::Multiply,
        }),
        glow: Some(GlowEffect {
            color: colour(),
            radius: Some(Emu::from_emu(60_000)),
        }),
        inner_shadow: Some(InnerShadowEffect {
            color: colour(),
            blur_radius: Some(Emu::from_emu(30_000)),
            distance: Some(Emu::from_emu(20_000)),
            direction: Some(mjx_dml::Angle::from_degrees(45.0)),
        }),
        outer_shadow: Some(OuterShadowEffect {
            color: colour(),
            blur_radius: Some(Emu::from_emu(40_000)),
            distance: Some(Emu::from_emu(25_000)),
            direction: Some(mjx_dml::Angle::from_degrees(90.0)),
            scale_x: None,
            scale_y: None,
            skew_x: None,
            skew_y: None,
            alignment: None,
            rotate_with_shape: None,
        }),
        preset_shadow: Some(PresetShadowEffect {
            preset: PresetShadow::Shadow1,
            color: colour(),
            distance: Some(Emu::from_emu(15_000)),
            direction: Some(mjx_dml::Angle::from_degrees(135.0)),
        }),
        reflection: Some(ReflectionEffect {
            blur_radius: Some(Emu::from_emu(10_000)),
            start_alpha: None,
            start_position: None,
            end_alpha: None,
            end_position: None,
            distance: Some(Emu::from_emu(5_000)),
            direction: None,
            fade_direction: None,
            scale_x: None,
            scale_y: None,
            skew_x: None,
            skew_y: None,
            alignment: None,
            rotate_with_shape: None,
        }),
        soft_edge: Some(SoftEdgeEffect {
            radius: Emu::from_emu(70_000),
        }),
    }
}

/// The same list with every effect after the first `keep` removed, in the schema's own order.
fn first(keep: usize) -> EffectListSpec {
    let mut spec = everything();
    let clears: [&dyn Fn(&mut EffectListSpec); 8] = [
        &|spec: &mut EffectListSpec| spec.blur = None,
        &|spec: &mut EffectListSpec| spec.fill_overlay = None,
        &|spec: &mut EffectListSpec| spec.glow = None,
        &|spec: &mut EffectListSpec| spec.inner_shadow = None,
        &|spec: &mut EffectListSpec| spec.outer_shadow = None,
        &|spec: &mut EffectListSpec| spec.preset_shadow = None,
        &|spec: &mut EffectListSpec| spec.reflection = None,
        &|spec: &mut EffectListSpec| spec.soft_edge = None,
    ];
    for clear in clears.iter().skip(keep) {
        clear(&mut spec);
    }
    spec
}

#[test]
fn the_chain_is_reachable_from_its_root_at_every_length() {
    for length in 1..=8 {
        let chain = effect_styles(&first(length), DeviceScale::UNZOOMED, &no_images);
        assert_eq!(
            chain.len(),
            length,
            "a list of {length} effects translated to {} entries",
            chain.len()
        );

        // Walk back from the root — the last entry — and mark everything it can reach. An entry
        // nobody reaches draws nothing, and this is the only place that shows.
        let mut reached = vec![false; chain.len()];
        let mut cursor = chain.len().checked_sub(1);
        while let Some(index) = cursor {
            reached[index] = true;
            cursor = chain.get(index).and_then(|entry| entry.input);
        }
        let unreachable: Vec<usize> = reached
            .iter()
            .enumerate()
            .filter(|(_, seen)| !**seen)
            .map(|(index, _)| index)
            .collect();
        assert!(
            unreachable.is_empty(),
            "with {length} effects, entries {unreachable:?} are not reachable from the root. \
             `build_scene` pushes only the last entry, so those effects would be translated, \
             encoded and never drawn — with no error anywhere."
        );
    }
}

#[test]
fn the_first_entry_consumes_the_subtree_and_no_other_one_does() {
    let chain = effect_styles(&everything(), DeviceScale::UNZOOMED, &no_images);
    assert_eq!(
        chain.len(),
        8,
        "the whole of `CT_EffectList` is eight kinds"
    );
    assert_eq!(
        chain.first().and_then(|entry| entry.input),
        None,
        "the first effect must consume the shape's own subtree"
    );
    for (index, entry) in chain.iter().enumerate().skip(1) {
        assert_eq!(
            entry.input,
            Some(index - 1),
            "effect {index} ({:?}) must consume effect {}, which is what ECMA-376's fixed child \
             order means by *applied in order*",
            entry.kind,
            index - 1
        );
    }
}

#[test]
fn the_order_is_the_schemas_own() {
    let chain = effect_styles(&everything(), DeviceScale::UNZOOMED, &no_images);
    let kinds: Vec<EffectKind> = chain.iter().map(|entry| entry.kind).collect();
    assert_eq!(
        kinds,
        vec![
            EffectKind::Blur,
            EffectKind::FillOverlay,
            EffectKind::Glow,
            EffectKind::InnerShadow,
            EffectKind::OuterShadow,
            // `a:prstShdw` is drawn as an outer shadow — see `crate::effects`'s `GUESS:`.
            EffectKind::OuterShadow,
            EffectKind::Reflection,
            EffectKind::SoftEdge,
        ],
        "the chain must follow `CT_EffectList`'s own child sequence"
    );
}

#[test]
fn an_empty_list_produces_no_chain_at_all() {
    let chain = effect_styles(&EffectListSpec::new(), DeviceScale::UNZOOMED, &no_images);
    assert!(
        chain.is_empty(),
        "an effect list with nothing in it must produce no entries, so that \
         `Decoration::is_invisible` can skip the node rather than opening a layer for a chain of \
         no-ops"
    );
}

/// The instrument this programme calls *identity values*: a translation whose every number is at
/// its no-op value produces a chain that is structurally right and draws nothing, and two painters
/// drawing nothing agree perfectly.
#[test]
fn the_numbers_are_not_all_at_their_no_op_values() {
    let chain = effect_styles(&everything(), DeviceScale::UNZOOMED, &no_images);

    let radii: Vec<f32> = chain.iter().map(|entry| entry.radius).collect();
    assert!(
        radii.iter().filter(|radius| **radius > 0.0).count() >= 5,
        "at least five of the eight effects state a blur radius, and a translation that dropped \
         them would produce a chain of zero-radius effects that draws almost exactly what no \
         effect at all draws: {radii:?}"
    );

    let distances: Vec<f32> = chain.iter().map(|entry| entry.distance).collect();
    assert!(
        distances.iter().filter(|distance| **distance > 0.0).count() >= 4,
        "four effects state an offset distance, and a shadow at zero distance is a shadow behind \
         the shape it is cast by: {distances:?}"
    );

    let coloured = chain
        .iter()
        .filter(|entry| matches!(entry.fill, FillStyle::Solid(_)))
        .count();
    assert!(
        coloured >= 4,
        "the glow, both shadows and the preset shadow all state a colour, and an effect with \
         `FillStyle::None` draws nothing whatever its geometry says; only {coloured} carry one"
    );

    assert!(
        chain
            .iter()
            .any(|entry| entry.kind == EffectKind::Reflection && entry.start_alpha > 0.0),
        "a reflection whose starting alpha is zero is a reflection nobody can see, which is the \
         identity value this assertion exists to refuse"
    );
}
