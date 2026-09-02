//! Validates the generated preset-shape tables against hand-checked `presetShapeDefinitions.xml`
//! facts: axes, defaults, literal-vs-computed domain bounds, the shape list, and the `gdLst` closure
//! a computed bound is resolved from.

use mjx_ooxml_types::drawingml::{
    adjustable_shapes, adjustment_bound_guides_of, adjustments_of, AdjustmentAxis, AdjustmentBound,
    AdjustmentSpec, PresetGuide, PresetShapeType,
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

#[test]
fn the_adjustable_shape_list_is_exactly_the_shapes_with_adjustments() {
    let shapes = adjustable_shapes();
    assert_eq!(
        shapes.len(),
        119,
        "presetShapeDefinitions.xml defines 119 shapes with a handle-referenced adjustment"
    );
    for shape in shapes {
        assert!(
            !adjustments_of(*shape).is_empty(),
            "{shape:?} is listed but has no adjustments"
        );
    }
    for shape in [
        PresetShapeType::Rectangle,
        PresetShapeType::Ellipse,
        PresetShapeType::Pentagon,
        PresetShapeType::UpArrow,
    ] {
        assert!(!shapes.contains(&shape), "{shape:?} should not be listed");
    }
}

#[test]
fn chevrons_computed_bound_carries_the_guide_it_is_computed_from() {
    // `chevron`'s `maxAdj` is one guide over the built-ins, so the closure is that guide alone.
    assert_eq!(
        adjustment_bound_guides_of(PresetShapeType::Chevron),
        &[PresetGuide {
            wire_name: "maxAdj",
            formula: "*/ 100000 w ss",
        }]
    );
}

#[test]
fn a_bound_guide_closure_is_in_declaration_order_and_reaches_back_through_the_chain() {
    // `bentArrow`'s `maxAdj1` is computed from guides that are themselves computed; the closure has
    // to arrive in the order the file declares them, or a guide would be evaluated before its input.
    let guides = adjustment_bound_guides_of(PresetShapeType::BentArrow);
    let names: Vec<&str> = guides.iter().map(|guide| guide.wire_name).collect();
    assert!(names.contains(&"maxAdj1"), "{names:?}");
    assert!(names.contains(&"maxAdj4"), "{names:?}");

    let mut defined: Vec<&str> = Vec::new();
    for guide in guides {
        for argument in guide.formula.split_whitespace().skip(1) {
            if argument.parse::<f64>().is_ok() {
                continue;
            }
            let is_adjustment = adjustments_of(PresetShapeType::BentArrow)
                .iter()
                .any(|spec| spec.wire_name == argument);
            assert!(
                defined.contains(&argument) || is_adjustment || !names.contains(&argument),
                "`{}` uses `{argument}` before the closure defines it",
                guide.wire_name
            );
        }
        defined.push(guide.wire_name);
    }
}

#[test]
fn a_shape_whose_bounds_are_all_literals_carries_no_bound_guides() {
    for shape in [
        PresetShapeType::RoundedRectangle,
        PresetShapeType::Rectangle,
        PresetShapeType::UpArrow,
    ] {
        assert!(
            adjustment_bound_guides_of(shape).is_empty(),
            "{shape:?} needs no bound guides"
        );
    }
}
