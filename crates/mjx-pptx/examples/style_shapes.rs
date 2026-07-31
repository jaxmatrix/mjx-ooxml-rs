//! Fill, outline, effects and 3-D — and the cursor that applies them all in one pass.
//!
//! ```sh
//! cargo run -p mjx-pptx --example style_shapes -- out.pptx
//! ```

use anyhow::Result;
use mjx_dml::{
    Angle, ColorSpec, EffectListSpec, Emu, FillSpec, Fraction, GradientStopSpec, LineSpec,
    LineWidth, OuterShadowEffect,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_pptx::{Presentation, ShapeBounds};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("style_shapes.pptx");
    let mut deck = Presentation::open(&support::template()?)?;
    let slide = deck.add_slide_from_layout(2)?; // the blank layout

    // ---- A solid fill and a stroke ---------------------------------------------------------
    let solid = deck.add_shape(
        slide,
        PresetShapeType::RoundedRectangle,
        ShapeBounds::from_inches(0.5, 1.0, 2.5, 1.5),
    )?;
    deck.set_shape_fill(
        slide,
        solid,
        &FillSpec::solid(ColorSpec::Srgb("1F3864".into())),
    )?;
    deck.set_shape_outline(
        slide,
        solid,
        &LineSpec::solid(
            LineWidth::from_points(2.0),
            ColorSpec::Srgb("FFC000".into()),
        ),
    )?;

    // ---- A gradient ------------------------------------------------------------------------
    let gradient = deck.add_shape(
        slide,
        PresetShapeType::Rectangle,
        ShapeBounds::from_inches(3.5, 1.0, 2.5, 1.5),
    )?;
    deck.set_shape_fill(
        slide,
        gradient,
        &FillSpec::Gradient {
            stops: vec![
                GradientStopSpec {
                    color: ColorSpec::Srgb("4472C4".into()),
                    position: Fraction::from_ratio(0.0),
                },
                GradientStopSpec {
                    color: ColorSpec::Srgb("ED7D31".into()),
                    position: Fraction::from_ratio(1.0),
                },
            ],
            angle: None,
        },
    )?;

    // ---- An effect -------------------------------------------------------------------------
    let shadowed = deck.add_shape(
        slide,
        PresetShapeType::Ellipse,
        ShapeBounds::from_inches(6.5, 1.0, 2.0, 1.5),
    )?;
    deck.set_shape_fill(
        slide,
        shadowed,
        &FillSpec::solid(ColorSpec::Srgb("70AD47".into())),
    )?;
    deck.set_shape_effects(
        slide,
        shadowed,
        // Only `color` is required; every schema-defaulted attribute may stay `None`.
        &EffectListSpec {
            outer_shadow: Some(OuterShadowEffect {
                color: ColorSpec::Srgb("404040".into()),
                blur_radius: Some(Emu::from_points(4.0)),
                distance: Some(Emu::from_points(3.0)),
                direction: Some(Angle::from_degrees(45.0)),
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: None,
                rotate_with_shape: None,
            }),
            ..EffectListSpec::new()
        },
    )?;

    // ---- A theme colour, and what it resolves to -------------------------------------------
    // Naming a theme slot rather than a hex value is usually the better choice: it follows the
    // theme. The effective reader tells you what it currently resolves to.
    use mjx_dml::SchemeColor;
    let themed = deck.add_shape(
        slide,
        PresetShapeType::Rectangle,
        ShapeBounds::from_inches(0.5, 3.0, 2.5, 1.5),
    )?;
    deck.set_shape_fill(
        slide,
        themed,
        &FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent2)),
    )?;
    println!(
        "accent2 resolves to {:?}",
        deck.effective_shape_fill(slide, themed)?
    );

    // ---- Several edits, one pass -----------------------------------------------------------
    // Naming the address once per edit reads badly. The cursor records edits and writes them
    // together; nothing happens until `.apply()`.
    let cursor_target = deck.add_shape(
        slide,
        PresetShapeType::Diamond,
        ShapeBounds::from_inches(3.5, 3.0, 2.5, 1.5),
    )?;
    deck.shape(slide, cursor_target)?
        .fill(FillSpec::solid(ColorSpec::Srgb("C00000".into())))
        .outline(LineSpec::solid(
            LineWidth::from_points(1.0),
            ColorSpec::Srgb("FFFFFF".into()),
        ))
        .text("One pass")
        .apply()?;

    let bytes = deck.save()?;
    std::fs::write(&out, &bytes)?;

    let mut reopened = Presentation::open(&bytes)?;
    anyhow::ensure!(reopened.shape_fill(slide, solid)?.is_some());
    anyhow::ensure!(reopened.shape_effects(slide, shadowed)?.is_some());
    anyhow::ensure!(reopened.shape_text(slide, cursor_target)? == "One pass");
    println!("wrote {} and verified", out.display());

    Ok(())
}
