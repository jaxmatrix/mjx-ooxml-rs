//! Integration tests for the shape-effects surface: read a shape's [`EffectListSpec`], set effects,
//! and save — with fidelity (only the edited slide changes) and correct `p:spPr` placement (after the
//! fill and outline, before the 3-D children).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    Angle, BlendMode, BlurEffect, ColorSpec, EffectListSpec, Emu, FillOverlayEffect, FillSpec,
    Fraction, GlowEffect, InnerShadowEffect, LineSpec, LineWidth, OuterShadowEffect, PresetShadow,
    PresetShadowEffect, RectangleAlignment, ReflectionEffect, SchemeColor, SoftEdgeEffect,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::Package;
use mjx_pptx::{Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0)
}

/// Adds a fresh autoshape and returns its shape index.
fn added_shape(pres: &mut Presentation) -> usize {
    pres.add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape")
}

/// An outer-shadow-only effect list — the simple spec used by the read-back/replace tests.
fn shadow() -> EffectListSpec {
    EffectListSpec {
        outer_shadow: Some(OuterShadowEffect {
            color: ColorSpec::Srgb("808080".to_owned()),
            blur_radius: Some(Emu::from_emu(40_000)),
            distance: Some(Emu::from_emu(38_100)),
            direction: Some(Angle::from_degrees(45.0)),
            scale_x: None,
            scale_y: None,
            skew_x: None,
            skew_y: None,
            alignment: None,
            rotate_with_shape: None,
        }),
        ..EffectListSpec::new()
    }
}

/// A maximal effect list exercising all eight effects — its `to_effect_list` output is a fixed point
/// of `spec()`, so it round-trips exactly through a save/reopen.
fn rich_effects() -> EffectListSpec {
    EffectListSpec {
        blur: Some(BlurEffect {
            radius: Some(Emu::from_emu(50_800)),
            grow: Some(true),
        }),
        fill_overlay: Some(FillOverlayEffect {
            fill: FillSpec::Solid(ColorSpec::Srgb("FFFF00".to_owned())),
            blend: BlendMode::Over,
        }),
        glow: Some(GlowEffect {
            color: ColorSpec::Srgb("FF0000".to_owned()),
            radius: Some(Emu::from_emu(63_500)),
        }),
        inner_shadow: Some(InnerShadowEffect {
            color: ColorSpec::Srgb("000000".to_owned()),
            blur_radius: Some(Emu::from_emu(40_000)),
            distance: Some(Emu::from_emu(20_000)),
            direction: Some(Angle::from_degrees(45.0)),
        }),
        outer_shadow: Some(OuterShadowEffect {
            color: ColorSpec::Srgb("808080".to_owned()),
            blur_radius: Some(Emu::from_emu(50_000)),
            distance: Some(Emu::from_emu(38_100)),
            direction: Some(Angle::from_degrees(45.0)),
            scale_x: Some(Fraction::from_ratio(0.9)),
            scale_y: Some(Fraction::from_ratio(0.9)),
            skew_x: Some(Angle::from_degrees(10.0)),
            skew_y: None,
            alignment: Some(RectangleAlignment::TopLeft),
            rotate_with_shape: Some(false),
        }),
        preset_shadow: Some(PresetShadowEffect {
            preset: PresetShadow::Shadow13,
            color: ColorSpec::Srgb("C0C0C0".to_owned()),
            distance: Some(Emu::from_emu(45_000)),
            direction: Some(Angle::from_degrees(45.0)),
        }),
        reflection: Some(ReflectionEffect {
            blur_radius: Some(Emu::from_emu(6_350)),
            start_alpha: Some(Fraction::from_ratio(0.5)),
            start_position: Some(Fraction::from_ratio(0.0)),
            end_alpha: Some(Fraction::from_ratio(0.3)),
            end_position: Some(Fraction::from_ratio(0.9)),
            distance: Some(Emu::from_emu(60_000)),
            direction: Some(Angle::from_degrees(90.0)),
            fade_direction: Some(Angle::from_degrees(45.0)),
            scale_x: Some(Fraction::from_ratio(1.0)),
            scale_y: Some(Fraction::from_ratio(-1.0)),
            skew_x: None,
            skew_y: None,
            alignment: Some(RectangleAlignment::BottomLeft),
            rotate_with_shape: Some(false),
        }),
        soft_edge: Some(SoftEdgeEffect {
            radius: Emu::from_emu(112_500),
        }),
    }
}

#[test]
fn fresh_shape_has_no_effects() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    assert_eq!(pres.shape_effects(0, idx).expect("shape_effects"), None);
}

#[test]
fn set_effects_reads_back_and_persists() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    let spec = shadow();
    pres.set_shape_effects(0, idx, &spec).expect("set effects");

    assert_eq!(
        pres.shape_effects(0, idx).expect("shape_effects"),
        Some(spec.clone())
    );

    // Survives a save/reopen, and the shape's geometry is intact (the effects didn't clobber spPr).
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_effects(0, idx).expect("shape_effects"),
        Some(spec)
    );
    assert!(matches!(
        reread.shape_geometry(0, idx).expect("geometry"),
        mjx_dml::ShapeGeometry::RoundedRectangle { .. }
    ));
}

#[test]
fn set_effects_replaces_an_existing_effect_list_in_place() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_effects(0, idx, &shadow())
        .expect("set first effects");
    // Overriding replaces the a:effectLst element rather than adding a second one.
    let second = EffectListSpec {
        glow: Some(GlowEffect {
            color: ColorSpec::Srgb("00FF00".to_owned()),
            radius: Some(Emu::from_emu(30_000)),
        }),
        ..EffectListSpec::new()
    };
    pres.set_shape_effects(0, idx, &second)
        .expect("set second effects");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_effects(0, idx).expect("shape_effects"),
        Some(second)
    );
}

#[test]
fn set_rich_effects_round_trips_through_save() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    let spec = rich_effects();
    pres.set_shape_effects(0, idx, &spec).expect("set effects");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_effects(0, idx).expect("shape_effects"),
        Some(spec)
    );
}

#[test]
fn set_no_effects_writes_an_empty_effect_lst() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_no_effects(0, idx).expect("set no effects");
    // An explicit <a:effectLst/> reads back as an empty (all-None) effect list, distinct from absent.
    assert_eq!(
        pres.shape_effects(0, idx).expect("shape_effects"),
        Some(EffectListSpec::default())
    );
}

#[test]
fn effects_fill_and_outline_coexist_on_one_shape() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("FFFF00".to_owned())))
        .expect("set fill");
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FF0000".to_owned()),
        ),
    )
    .expect("set outline");
    pres.set_shape_effects(0, idx, &shadow())
        .expect("set effects");

    // All three slots read back after a save/reopen — the spPr children don't collide.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("FFFF00".to_owned())))
    );
    assert_eq!(
        reread.shape_outline(0, idx).expect("shape_outline"),
        Some(LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FF0000".to_owned())
        ))
    );
    assert_eq!(
        reread.shape_effects(0, idx).expect("shape_effects"),
        Some(shadow())
    );
}

#[test]
fn set_effects_keeps_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_effects(0, idx, &shadow())
        .expect("set effects");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "effects edit dirtied unrelated part {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// effective_shape_effects — the shape's rendered effects, resolved to concrete RGB
// ---------------------------------------------------------------------------------------------

#[test]
fn effective_effects_resolves_a_scheme_color_against_the_theme() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // An explicit glow whose color is the theme's accent1 scheme color.
    pres.set_shape_effects(
        0,
        idx,
        &EffectListSpec {
            glow: Some(GlowEffect {
                color: ColorSpec::Scheme(SchemeColor::Accent1),
                radius: Some(Emu::from_emu(63_500)),
            }),
            ..EffectListSpec::new()
        },
    )
    .expect("set effects");

    let effective = pres
        .effective_shape_effects(0, idx)
        .expect("effective_shape_effects")
        .expect("some effects");
    let glow = effective.glow.expect("glow");
    // accent1 bakes to the fixture theme's 4472C4; the radius is preserved.
    assert_eq!(glow.color, ColorSpec::Srgb("4472C4".into()));
    assert_eq!(glow.radius, Some(Emu::from_emu(63_500)));
}

#[test]
fn effective_effects_keeps_explicit_srgb() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_effects(
        0,
        idx,
        &EffectListSpec {
            glow: Some(GlowEffect {
                color: ColorSpec::Srgb("112233".into()),
                radius: None,
            }),
            ..EffectListSpec::new()
        },
    )
    .expect("set effects");

    let effective = pres
        .effective_shape_effects(0, idx)
        .expect("effective_shape_effects")
        .expect("some effects");
    assert_eq!(
        effective.glow.expect("glow").color,
        ColorSpec::Srgb("112233".into())
    );
}

#[test]
fn effective_effects_is_none_when_shape_declares_no_effects() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // A fresh autoshape has no explicit effectLst and no p:style effectRef.
    assert_eq!(
        pres.effective_shape_effects(0, idx)
            .expect("effective_shape_effects"),
        None
    );
}

#[test]
fn reading_effective_effects_keeps_all_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Reading the title placeholder's effective effects walks slide -> layout -> master -> theme.
    let _ = pres
        .effective_shape_effects(0, 0)
        .expect("effective_shape_effects");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    assert_eq!(reopened.len(), snapshot.len());
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "reading effective effects dirtied part {name}"
        );
    }
}

#[test]
fn effective_effects_resolves_a_theme_effect_ref_shadow() {
    // The `effects_theme.pptx` fixture has a shape (index 1) with no explicit effectLst but a
    // `p:style > a:effectRef idx="3"` into a theme effect style whose outer shadow is `phClr`. The
    // effectRef's accent1 substitutes the phClr, baking to the theme's 4472C4.
    let mut pres = Presentation::open(&fixture("effects_theme.pptx")).expect("open");
    let effective = pres
        .effective_shape_effects(0, 1)
        .expect("effective_shape_effects")
        .expect("some effects");
    let shadow = effective.outer_shadow.expect("outer shadow");
    assert_eq!(shadow.color, ColorSpec::Srgb("4472C4".into()));
    assert_eq!(shadow.blur_radius, Some(Emu::from_emu(40_000)));
    assert_eq!(shadow.distance, Some(Emu::from_emu(20_000)));
}
