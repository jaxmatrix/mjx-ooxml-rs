//! A custom dash reaches the stroke as the preset `Dash` (MJXOFF-297).
//!
//! MJX-LEDGER-LIMITATION: a custom dash (`a:custDash`) draws as the preset `Dash`, because
//! `LineSpec` does not model its stops.

use mjx_dml::{ColorSpec, FillSpec, LineDash, LineSpec, LineWidth};
use mjx_ooxml_types::drawingml::PresetLineDash;
use mjx_scene::{DashPattern, DeviceScale};
use mjx_scene_pptx::stroke_style;

// A stroke that names no picture, so no relationship resolves to one.
fn no_images(_rel_id: &str) -> Option<u64> {
    None
}

// A two-point blue line with the given dash.
fn a_line(dash: LineDash) -> LineSpec {
    LineSpec {
        width: Some(LineWidth::from_points(2.0)),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("1F4E79".to_owned()))),
        dash: Some(dash),
        ..LineSpec::default()
    }
}

#[test]
fn a_custom_dash_is_the_whole_stroke_of_a_preset_dash() {
    let custom = stroke_style(&a_line(LineDash::Custom), DeviceScale::UNZOOMED, &no_images)
        .ok()
        .flatten()
        .expect("a filled line is a stroke");
    let preset = stroke_style(
        &a_line(LineDash::Preset(PresetLineDash::Dash)),
        DeviceScale::UNZOOMED,
        &no_images,
    )
    .ok()
    .flatten()
    .expect("a filled line is a stroke");
    let solid = stroke_style(
        &a_line(LineDash::Preset(PresetLineDash::Solid)),
        DeviceScale::UNZOOMED,
        &no_images,
    )
    .ok()
    .flatten()
    .expect("a filled line is a stroke");

    assert_eq!(custom.dash, DashPattern::Dash);
    assert_ne!(custom, solid, "a custom dash is not drawn solid");
    assert_eq!(
        custom, preset,
        "a custom dash now differs from the preset `Dash`, so its stops reach the scene: delete this suite"
    );
}
