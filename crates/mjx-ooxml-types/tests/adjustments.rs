//! Validates the generated `adjustments_of` table against hand-checked `presetShapeDefinitions.xml`
//! facts: axes, defaults, and literal-vs-computed domain bounds.

use mjx_ooxml_types::drawingml::{
    adjustments_of, AdjustmentAxis, AdjustmentBound, AdjustmentSpec, PresetShapeType,
};

#[test]
fn rounded_rectangle_single_horizontal_literal_domain() {
    let adj = adjustments_of(PresetShapeType::RoundedRectangle);
    assert_eq!(
        adj,
        &[AdjustmentSpec {
            wire_name: "adj",
            axis: AdjustmentAxis::Horizontal,
            default: 16667,
            min: AdjustmentBound::Literal(0),
            max: AdjustmentBound::Literal(50000),
        }]
    );
}

#[test]
fn chevron_max_is_a_computed_guide() {
    let adj = adjustments_of(PresetShapeType::Chevron);
    assert_eq!(adj.len(), 1);
    assert_eq!(adj[0].wire_name, "adj");
    assert_eq!(adj[0].axis, AdjustmentAxis::Horizontal);
    // Data-dependent bound preserved as a guide name, not a literal.
    assert_eq!(adj[0].max, AdjustmentBound::Guide("maxAdj"));
}

#[test]
fn left_arrow_has_one_vertical_and_one_horizontal_adjustment() {
    let adj = adjustments_of(PresetShapeType::LeftArrow);
    assert_eq!(adj.len(), 2);
    assert_eq!(adj[0].wire_name, "adj1");
    assert_eq!(adj[0].axis, AdjustmentAxis::Vertical);
    assert_eq!(adj[0].max, AdjustmentBound::Literal(100000));
    assert_eq!(adj[1].wire_name, "adj2");
    assert_eq!(adj[1].axis, AdjustmentAxis::Horizontal);
    assert_eq!(adj[1].max, AdjustmentBound::Guide("maxAdj2"));
}

#[test]
fn block_arc_uses_polar_angle_and_radius_axes() {
    let adj = adjustments_of(PresetShapeType::BlockArc);
    assert_eq!(adj.len(), 3);
    assert_eq!(adj[0].axis, AdjustmentAxis::Angle);
    assert_eq!(adj[0].default, 10800000);
    assert_eq!(adj[2].axis, AdjustmentAxis::Radius);
}

#[test]
fn fixed_geometry_shapes_have_no_adjustments() {
    // Truly parameterless, avLst-but-no-handle (pentagon: fudge constants only), and a shape absent
    // from the geometry file (upArrow) all resolve to an empty slice via the `_ => &[]` wildcard.
    for shape in [
        PresetShapeType::Rectangle,
        PresetShapeType::Ellipse,
        PresetShapeType::Pentagon,
        PresetShapeType::UpArrow,
    ] {
        assert!(
            adjustments_of(shape).is_empty(),
            "{shape:?} should have no adjustments"
        );
    }
}
