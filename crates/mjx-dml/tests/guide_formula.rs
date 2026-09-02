//! The DrawingML guide-formula evaluator (`a:gd@fmla`), through the public API.
//!
//! Every expected value here is **derived from the ECMA-376 Part 1 prose**, not from what the code
//! happens to do:
//!
//! * §20.1.9.11 (`a:gd`, the `fmla` attribute) states each of the seventeen operators as an
//!   arithmetic identity — `"*/ x y z" = ((x * y) / z)`, `"mod x y z" = sqrt(x^2 + …)`, and so on.
//!   The inputs below are chosen so a different reading of the identity gives a *different* number:
//!   `mod 2 3 6` is `7` only for the sum of squares, `?: 0 …` takes the else branch only for a strict
//!   `> 0`, and every trigonometric case would move if the angle unit were wrong.
//! * §20.1.10.56 (`ST_ShapeType`) states the built-in variables and their values, and states the
//!   angular ones in 60000ths of a degree (`cd4` is `5400000.0`, "equivalent to 90 degrees").
//!
//! The two places the prose is not self-consistent are pinned by the spec's own normative preset
//! shape definitions rather than by preference; see `at2_is_the_two_argument_arc_tangent` and
//! `cat2_and_sat2_use_the_two_argument_arc_tangent`.

use mjx_dml::{
    AdjustAngle, AdjustCoordinate, CustomGeometrySpec, DrawCommand, Emu, GuideArgument,
    GuideContext, GuideError, GuideFormula, GuideFormulaError, GuideOperator, GuideSpec,
    Path2DSpec, Point, PresetGeometry, Rectangle, ResolvedDrawCommand, ResolvedGuides,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_ooxml_types::drawingml::{
    adjustable_shapes, adjustment_bound_guides_of, adjustments_of, AdjustmentBound, PresetShapeType,
};
use mjx_xml::fidelity;

/// A shape 1000 × 400 units: wide enough that `ss` is the height and every `w`/`h` built-in differs.
fn wide_context() -> GuideContext {
    GuideContext::from_extents(Emu::from_emu(1000), Emu::from_emu(400))
}

/// Evaluates one formula in an empty environment over [`wide_context`].
#[track_caller]
fn evaluate(formula: &str) -> f64 {
    ResolvedGuides::new(wide_context())
        .evaluate_formula(formula)
        .unwrap_or_else(|error| panic!("`{formula}` should evaluate: {error}"))
}

/// Evaluates one formula and returns the failure it produced.
#[track_caller]
fn evaluate_error(formula: &str) -> GuideError {
    ResolvedGuides::new(wide_context())
        .evaluate_formula(formula)
        .expect_err("should not evaluate")
}

#[track_caller]
fn assert_close(actual: f64, expected: f64, what: &str) {
    let tolerance = expected.abs().max(1.0) * 1e-9;
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: got {actual}, expected {expected}"
    );
}

// ---------------------------------------------------------------------------------------------
// Parsing the formula language
// ---------------------------------------------------------------------------------------------

#[test]
fn every_operator_round_trips_through_its_exact_wire_token() {
    let tokens = [
        "*/", "+-", "+/", "?:", "abs", "at2", "cat2", "cos", "max", "min", "mod", "pin", "sat2",
        "sin", "sqrt", "tan", "val",
    ];
    assert_eq!(GuideOperator::ALL.len(), 17, "the prose lists seventeen");
    for token in tokens {
        let operator = GuideOperator::from_wire(token)
            .unwrap_or_else(|| panic!("`{token}` should be an operator"));
        assert_eq!(operator.to_wire(), token);
        assert!(GuideOperator::ALL.contains(&operator));
    }
    assert_eq!(GuideOperator::from_wire("*"), None);
    assert_eq!(GuideOperator::from_wire("VAL"), None);
}

#[test]
fn a_formula_parses_into_an_operator_and_its_arguments() {
    let formula = GuideFormula::parse("*/ w adj1 100000").expect("well-formed");
    assert_eq!(formula.operator(), GuideOperator::MultiplyDivide);
    assert_eq!(formula.text(), "*/ w adj1 100000");
    assert_eq!(
        formula.arguments(),
        [
            GuideArgument::Name("w"),
            GuideArgument::Name("adj1"),
            GuideArgument::Literal(100_000.0),
        ]
    );
}

#[test]
fn a_formula_takes_exactly_the_arguments_its_operator_states() {
    assert_eq!(GuideOperator::LiteralValue.argument_count(), 1);
    assert_eq!(GuideOperator::Sine.argument_count(), 2);
    assert_eq!(GuideOperator::PinToRange.argument_count(), 3);

    assert_eq!(
        GuideFormula::parse("val 1 2"),
        Err(GuideFormulaError::ArgumentCount {
            operator: "val",
            expected: 1,
            found: 2,
        })
    );
    assert_eq!(
        GuideFormula::parse("pin 1 2"),
        Err(GuideFormulaError::ArgumentCount {
            operator: "pin",
            expected: 3,
            found: 2,
        })
    );
    // Overlong argument lists are counted, not written past the end of the fixed argument array.
    assert_eq!(
        GuideFormula::parse("pin 1 2 3 4 5 6 7"),
        Err(GuideFormulaError::ArgumentCount {
            operator: "pin",
            expected: 3,
            found: 7,
        })
    );
    assert_eq!(GuideFormula::parse("   "), Err(GuideFormulaError::Empty));
    assert_eq!(
        GuideFormula::parse("nope 1"),
        Err(GuideFormulaError::UnknownOperator {
            token: "nope".to_owned(),
        })
    );
}

#[test]
fn only_a_plain_decimal_token_is_a_literal() {
    for (token, value) in [
        ("0", 0.0),
        ("-25000", -25_000.0),
        ("+7", 7.0),
        ("1.5", 1.5),
        ("2.", 2.0),
        (".5", 0.5),
        ("1e3", 1000.0),
        ("1.5E-2", 0.015),
    ] {
        assert_eq!(
            GuideFormula::parse(&format!("val {token}"))
                .expect("parses")
                .arguments()[0],
            GuideArgument::Literal(value),
            "`{token}` is a numeric literal"
        );
    }
    // `inf`, `infinity` and `NaN` parse as f64 but are legal ST_GeomGuideName tokens: a file must not
    // be able to smuggle a non-finite value in as a "number".
    for token in [
        "inf", "-inf", "infinity", "NaN", "0x10", "1_000", "adj1", "3cd4",
    ] {
        assert_eq!(
            GuideFormula::parse(&format!("val {token}"))
                .expect("parses")
                .arguments()[0],
            GuideArgument::Name(token),
            "`{token}` is a name, not a literal"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The seventeen operators, each against the identity §20.1.9.11 states
// ---------------------------------------------------------------------------------------------

#[test]
fn multiply_divide_is_x_times_y_over_z() {
    // "*/ x y z" = ((x * y) / z)
    assert_close(evaluate("*/ 3 4 6"), 2.0, "*/ 3 4 6");
    assert_close(evaluate("*/ 100000 w ss"), 250_000.0, "*/ 100000 w ss");
}

#[test]
fn add_subtract_is_x_plus_y_minus_z() {
    // "+- x y z" = ((x + y) - z)
    assert_close(evaluate("+- 3 4 6"), 1.0, "+- 3 4 6");
    // Not ((x + y) - z) read as x + (y - z)? Both agree; pick one where the grouping shows: with a
    // negative z the sum still rises, which a "subtract both" reading would not do.
    assert_close(evaluate("+- 3 4 -6"), 13.0, "+- 3 4 -6");
}

#[test]
fn add_divide_is_x_plus_y_over_z() {
    // "+/ x y z" = ((x + y) / z) — distinct from `+-` and from `*/` on the same arguments.
    assert_close(evaluate("+/ 3 5 4"), 2.0, "+/ 3 5 4");
    assert_close(evaluate("+- 3 5 4"), 4.0, "+- 3 5 4");
    assert_close(evaluate("*/ 3 5 4"), 3.75, "*/ 3 5 4");
}

#[test]
fn if_else_takes_the_then_branch_only_when_x_is_strictly_positive() {
    // "?: x y z" = if (x > 0), then y … else z
    assert_close(evaluate("?: 1 10 20"), 10.0, "?: 1 10 20");
    assert_close(evaluate("?: 0 10 20"), 20.0, "?: 0 10 20 — zero is not > 0");
    assert_close(evaluate("?: -1 10 20"), 20.0, "?: -1 10 20");
    assert_close(evaluate("?: 0.5 10 20"), 10.0, "?: 0.5 10 20");
}

#[test]
fn absolute_value_negates_only_a_negative() {
    // "abs x" = if (x < 0), then (-1) * x … else x
    assert_close(evaluate("abs -5"), 5.0, "abs -5");
    assert_close(evaluate("abs 5"), 5.0, "abs 5");
    assert_close(evaluate("abs 0"), 0.0, "abs 0");
}

#[test]
fn arc_tangent_answers_in_sixty_thousandths_of_a_degree() {
    // "at2 x y" = arctan(y / x). §20.1.10.56 states the angular built-ins in 60000ths of a degree, so
    // an angle-valued result is in those units too: `at2 1 1` is 45°, i.e. 2_700_000 — which is also
    // exactly the `cd8` constant, so the unit is checked against the spec's own number.
    assert_close(evaluate("at2 1 1"), 2_700_000.0, "at2 1 1 is 45°");
    assert_close(evaluate("at2 1 1"), evaluate("val cd8"), "at2 1 1 == cd8");
    assert_close(evaluate("at2 1 0"), 0.0, "at2 1 0 is 0°");
    assert_close(evaluate("at2 0 1"), 5_400_000.0, "at2 0 1 is 90°");
}

#[test]
fn at2_is_the_two_argument_arc_tangent() {
    // The prose writes `arctan(y / x)`, which would fold the two arguments into one ratio and lose
    // the half-plane. The spec's own `moon` definition rules that out: it computes
    // `stAng1 = at2 dx2 dy2` with **both** arguments negative and then, in `enAng1`, subtracts a
    // whole turn (21600000) from a second `at2` — an adjustment that only makes sense if a negative
    // first argument has already carried the angle past ±90°.
    assert_close(
        evaluate("at2 -1 1"),
        8_100_000.0,
        "at2 -1 1 is 135°, not -45°",
    );
    assert_close(
        evaluate("at2 -1 -1"),
        -8_100_000.0,
        "at2 -1 -1 is -135°, not 45°",
    );
    // A one-argument reading would give -45° and +45° respectively.
    assert!(evaluate("at2 -1 1") > 0.0);
    assert!(evaluate("at2 -1 -1") < -5_400_000.0);
}

#[test]
fn cosine_sine_and_tangent_take_their_angle_in_sixty_thousandths_of_a_degree() {
    // "cos x y" = (x * cos( y )), "sin x y" = (x * sin( y )), "tan x y" = (x * tan( y )).
    // Read as radians these would be cos(180) = -0.598, sin(90) = 0.894, tan(45) = 1.6197.
    assert_close(evaluate("cos 100 10800000"), -100.0, "cos 100 cd2");
    assert_close(evaluate("sin 100 5400000"), 100.0, "sin 100 cd4");
    assert_close(evaluate("tan 100 2700000"), 100.0, "tan 100 cd8");
    assert_close(evaluate("cos 100 0"), 100.0, "cos 100 0");
    assert_close(evaluate("sin 100 0"), 0.0, "sin 100 0");
}

#[test]
fn maximum_and_minimum_pick_the_larger_and_the_smaller() {
    // "max x y" = if (x > y), then x … else y; "min x y" = if (x < y), then x … else y
    assert_close(evaluate("max 3 7"), 7.0, "max 3 7");
    assert_close(evaluate("max 7 3"), 7.0, "max 7 3");
    assert_close(evaluate("min 3 7"), 3.0, "min 3 7");
    assert_close(evaluate("min -7 3"), -7.0, "min -7 3");
}

#[test]
fn modulus_is_the_length_of_the_three_dimensional_vector() {
    // "mod x y z" = sqrt(x^2 + b^2 + c^2) — `b` and `c` are the prose's slips for y and z. 2, 3, 6 is
    // a Pythagorean quadruple: sqrt(4 + 9 + 36) = 7 exactly, a number no remainder reading produces.
    assert_close(evaluate("mod 2 3 6"), 7.0, "mod 2 3 6");
    assert_close(evaluate("mod 3 4 0"), 5.0, "mod 3 4 0");
    assert_close(evaluate("mod -2 -3 -6"), 7.0, "mod is sign-blind");
}

#[test]
fn pin_to_range_clamps_the_middle_argument_between_the_outer_two() {
    // "pin x y z" = if (y < x), then x … else if (y > z), then z … else y
    assert_close(evaluate("pin 10 5 20"), 10.0, "pin 10 5 20 — below");
    assert_close(evaluate("pin 10 15 20"), 15.0, "pin 10 15 20 — inside");
    assert_close(evaluate("pin 10 25 20"), 20.0, "pin 10 25 20 — above");
    assert_close(
        evaluate("pin 10 10 20"),
        10.0,
        "pin 10 10 20 — on the floor",
    );
    assert_close(
        evaluate("pin 10 20 20"),
        20.0,
        "pin 10 20 20 — on the ceiling",
    );
}

#[test]
fn square_root_and_literal_value() {
    // "sqrt x" = sqrt(x); "val x" = x
    assert_close(evaluate("sqrt 144"), 12.0, "sqrt 144");
    assert_close(evaluate("sqrt 0"), 0.0, "sqrt 0");
    assert_close(evaluate("val -25000"), -25_000.0, "val -25000");
    assert_close(evaluate("val 1.5"), 1.5, "val 1.5");
}

#[test]
fn cosine_and_sine_arc_tangent_scale_the_axis_by_the_arc_tangent_of_the_ratio() {
    // "cat2 x y z" = (x*(cos(arctan(z / y)))); "sat2 x y z" = (x*sin(arctan(z / y)))
    // arctan(1/1) is 45°, whose cosine and sine are both 1/sqrt(2).
    let root_half = 0.5f64.sqrt();
    assert_close(evaluate("cat2 10 1 1"), 10.0 * root_half, "cat2 10 1 1");
    assert_close(evaluate("sat2 10 1 1"), 10.0 * root_half, "sat2 10 1 1");
    assert_close(evaluate("cat2 10 1 0"), 10.0, "cat2 10 1 0 — arctan 0");
    assert_close(evaluate("sat2 10 1 0"), 0.0, "sat2 10 1 0 — arctan 0");
}

#[test]
fn cat2_and_sat2_use_the_two_argument_arc_tangent() {
    // A one-argument arctan lands in (-90°, 90°), where the cosine is never negative — so `cat2`
    // could never return a negative multiple of x. The spec's `arc` shape needs exactly that: it
    // places the arc's start at `x1 = hc + cat2 wd2 ht1 wt1`, with `ht1 = cos hd2 stAng` and
    // `wt1 = sin wd2 stAng`, and for a start angle of `cd2` (180°) that point must be the shape's
    // **left** edge. It only is because `ht1` is then negative and the arc tangent honours it.
    assert_close(
        evaluate("cat2 10 -1 0"),
        -10.0,
        "cat2 10 -1 0 is -10, not +10",
    );
    assert_close(
        evaluate("sat2 10 -1 1"),
        10.0 * 0.5f64.sqrt(),
        "sat2 10 -1 1 is +7.07, not -7.07",
    );
}

#[test]
fn the_arc_shapes_start_point_lands_on_the_left_edge_at_a_half_turn() {
    // The end-to-end form of the argument above, evaluated as `arc`'s own gdLst does it.
    let guides = ResolvedGuides::evaluate(
        [
            ("stAng", "val 10800000"),
            ("wt1", "sin wd2 stAng"),
            ("ht1", "cos hd2 stAng"),
            ("dx1", "cat2 wd2 ht1 wt1"),
            ("dy1", "sat2 hd2 ht1 wt1"),
            ("x1", "+- hc dx1 0"),
            ("y1", "+- vc dy1 0"),
        ],
        wide_context(),
    )
    .expect("arc's guides evaluate");

    assert_close(guides.value("x1").expect("x1"), 0.0, "x1 is the left edge");
    assert_close(guides.value("y1").expect("y1"), 200.0, "y1 is the centre");
}

// ---------------------------------------------------------------------------------------------
// The built-in variables (§20.1.10.56)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_size_derived_built_ins_come_from_the_shape_extents() {
    let context = wide_context();
    for (name, expected) in [
        ("w", 1000.0),
        ("h", 400.0),
        ("l", 0.0),
        ("t", 0.0),
        ("r", 1000.0),
        ("b", 400.0),
        ("hc", 500.0),
        ("vc", 200.0),
        ("ss", 400.0),
        ("ls", 1000.0),
        ("wd2", 500.0),
        ("wd3", 1000.0 / 3.0),
        ("wd10", 100.0),
        ("hd2", 200.0),
        ("hd6", 400.0 / 6.0),
        ("ssd2", 200.0),
        ("ssd8", 50.0),
        // Not in the prose's table, but used by the spec's own preset definitions.
        ("wd12", 1000.0 / 12.0),
        ("wd32", 1000.0 / 32.0),
        ("hd10", 40.0),
        ("ssd16", 25.0),
        ("ssd32", 12.5),
    ] {
        assert_close(
            context.variable(name).unwrap_or_else(|| panic!("{name}")),
            expected,
            name,
        );
    }
}

#[test]
fn the_circle_constants_are_the_values_the_prose_states() {
    let context = wide_context();
    // §20.1.10.56 states each of these literally.
    for (name, expected) in [
        ("cd2", 10_800_000.0),
        ("cd4", 5_400_000.0),
        ("cd8", 2_700_000.0),
        ("3cd4", 16_200_000.0),
        ("3cd8", 8_100_000.0),
        ("5cd8", 13_500_000.0),
        ("7cd8", 18_900_000.0),
        // `cd3` is not in the prose's list but is used by the spec's own preset definitions; the
        // general rule the listed values follow gives 21600000 / 3.
        ("cd3", 7_200_000.0),
    ] {
        assert_close(
            context.variable(name).unwrap_or_else(|| panic!("{name}")),
            expected,
            name,
        );
    }
}

#[test]
fn a_name_that_is_not_a_built_in_stays_undefined() {
    let context = wide_context();
    // `lsd`N is symmetrical with `ssd`N but appears in neither the prose nor the preset definitions,
    // so it is not invented here.
    for name in [
        "lsd2", "wd", "wd0", "cd0", "cd", "wdx", "hd2x", "myGuide", "adj", "W", "",
    ] {
        assert_eq!(context.variable(name), None, "`{name}` is not a built-in");
    }
}

#[test]
fn an_environment_without_a_size_keeps_only_the_size_independent_constants() {
    let guides = ResolvedGuides::without_size();
    assert_eq!(guides.context(), None);
    assert_eq!(guides.value("cd4"), Some(5_400_000.0));
    assert_eq!(guides.value("w"), None);
    assert_eq!(guides.value("hc"), None);
    assert_eq!(guides.value("ss"), None);
}

// ---------------------------------------------------------------------------------------------
// Evaluation order — the cycle defence
// ---------------------------------------------------------------------------------------------

#[test]
fn guides_are_evaluated_in_declaration_order() {
    let guides = ResolvedGuides::evaluate(
        [
            ("a", "val 25000"),
            ("b", "*/ w a 100000"),
            ("c", "+- b b 0"),
        ],
        wide_context(),
    )
    .expect("evaluates");
    assert_close(guides.value("a").expect("a"), 25_000.0, "a");
    assert_close(guides.value("b").expect("b"), 250.0, "b");
    assert_close(guides.value("c").expect("c"), 500.0, "c");
    assert_eq!(guides.len(), 3);
    assert!(!guides.is_empty());
}

#[test]
fn a_guide_that_references_itself_is_undefined_rather_than_a_loop() {
    // §20.1.9.11: "it is not possible to specify a guide that uses another guides result when that
    // guide has not yet been calculated". A self-reference is that case, so it can never spin.
    let error = ResolvedGuides::evaluate([("x", "+- x 1 0")], wide_context())
        .expect_err("a self-reference does not resolve");
    let GuideError::Guide { guide, source } = &error else {
        panic!("expected the failing guide to be named: {error}");
    };
    assert_eq!(guide, "x");
    assert_eq!(
        **source,
        GuideError::UndefinedGuide {
            name: "x".to_owned()
        }
    );
}

#[test]
fn a_forward_reference_and_a_mutual_cycle_are_undefined_rather_than_a_loop() {
    let error = ResolvedGuides::evaluate(
        [("first", "+- second 0 0"), ("second", "val 1")],
        wide_context(),
    )
    .expect_err("a forward reference does not resolve");
    assert!(
        matches!(&error, GuideError::Guide { guide, .. } if guide == "first"),
        "{error}"
    );

    // `alpha`/`beta` rather than `a`/`b`: `b` is the built-in bottom edge, and a cycle through a
    // name the format already defines would resolve rather than fail.
    let error = ResolvedGuides::evaluate(
        [("alpha", "+- beta 0 0"), ("beta", "+- alpha 0 0")],
        wide_context(),
    )
    .expect_err("a mutual cycle does not resolve");
    assert!(
        matches!(&error, GuideError::Guide { guide, .. } if guide == "alpha"),
        "{error}"
    );
}

#[test]
fn a_repeated_guide_name_takes_the_value_most_recently_calculated() {
    let guides = ResolvedGuides::evaluate(
        [("x", "val 1"), ("x", "+- x 1 0"), ("y", "*/ x 10 1")],
        wide_context(),
    )
    .expect("evaluates");
    assert_close(guides.value("x").expect("x"), 2.0, "x");
    assert_close(guides.value("y").expect("y"), 20.0, "y");
}

#[test]
fn a_long_chain_of_guides_evaluates_in_one_pass_without_recursion() {
    // A hostile file can make the chain as long as it likes; a single ordered pass costs one hash
    // insert each and never touches the stack, so 50 000 links neither overflow nor hang.
    const LINKS: usize = 50_000;
    let mut names = Vec::with_capacity(LINKS);
    let mut formulas = Vec::with_capacity(LINKS);
    names.push("g0".to_owned());
    formulas.push("val 1".to_owned());
    for index in 1..LINKS {
        names.push(format!("g{index}"));
        formulas.push(format!("+- g{} 1 0", index - 1));
    }
    let pairs: Vec<(&str, &str)> = names
        .iter()
        .map(String::as_str)
        .zip(formulas.iter().map(String::as_str))
        .collect();

    let guides = ResolvedGuides::evaluate(pairs, wide_context()).expect("evaluates");
    assert_close(
        guides.value(&format!("g{}", LINKS - 1)).expect("last link"),
        LINKS as f64,
        "the last link",
    );
}

#[test]
fn a_guide_naming_an_unknown_variable_names_which_one() {
    let error = evaluate_error("+- nosuch 1 0");
    assert_eq!(
        error,
        GuideError::UndefinedGuide {
            name: "nosuch".to_owned()
        }
    );
    assert!(error.to_string().contains("nosuch"), "{error}");
}

#[test]
fn arithmetic_that_leaves_the_reals_is_an_error_not_an_infinity() {
    for formula in ["*/ 1 1 0", "+/ 1 1 0", "sqrt -1", "val nan"] {
        let error = evaluate_error(formula);
        assert!(
            matches!(
                &error,
                GuideError::NotFinite { .. } | GuideError::UndefinedGuide { .. }
            ),
            "`{formula}` gave {error}"
        );
    }
    assert_eq!(
        evaluate_error("*/ 1 1 0"),
        GuideError::NotFinite {
            formula: "*/ 1 1 0".to_owned()
        }
    );
}

#[test]
fn a_malformed_formula_reports_the_text_and_the_reason() {
    let error = evaluate_error("nope 1 2");
    let GuideError::Malformed { formula, source } = &error else {
        panic!("expected a parse failure: {error}");
    };
    assert_eq!(formula, "nope 1 2");
    assert_eq!(
        *source,
        GuideFormulaError::UnknownOperator {
            token: "nope".to_owned()
        }
    );
}

// ---------------------------------------------------------------------------------------------
// Custom geometry — coordinates, rect edges, arc angles
// ---------------------------------------------------------------------------------------------

#[test]
fn a_coordinate_and_an_angle_resolve_through_the_guides_that_name_them() {
    let guides = ResolvedGuides::evaluate(
        [("x1", "*/ w 1 4"), ("stAng", "+- 3cd4 0 0")],
        wide_context(),
    )
    .expect("evaluates");

    assert_eq!(
        AdjustCoordinate::Guide("x1".to_owned())
            .resolve(&guides)
            .expect("x1 resolves"),
        Emu::from_emu(250)
    );
    assert_eq!(
        AdjustCoordinate::Emu(Emu::from_emu(42))
            .resolve(&guides)
            .expect("a literal resolves to itself"),
        Emu::from_emu(42)
    );
    assert_close(
        AdjustAngle::Guide("stAng".to_owned())
            .resolve(&guides)
            .expect("stAng resolves")
            .degrees(),
        270.0,
        "3cd4 is 270 degrees",
    );
    assert_eq!(
        AdjustCoordinate::Guide("nosuch".to_owned()).resolve(&guides),
        Err(GuideError::UndefinedGuide {
            name: "nosuch".to_owned()
        })
    );
}

/// The `custom_geometry_model` fixture geometry, as a spec: one adjust value, one computed guide, a
/// rect with two guide-named edges, and a path.
fn fixture_geometry() -> CustomGeometrySpec {
    CustomGeometrySpec {
        adjust_values: vec![GuideSpec {
            name: "adj1".to_owned(),
            formula: "val 25000".to_owned(),
        }],
        guides: vec![
            GuideSpec {
                name: "x1".to_owned(),
                formula: "*/ w adj1 100000".to_owned(),
            },
            GuideSpec {
                name: "swAng".to_owned(),
                formula: "+- cd4 0 0".to_owned(),
            },
        ],
        adjust_handles: Vec::new(),
        connection_sites: Vec::new(),
        text_rectangle: Some(Rectangle {
            left: AdjustCoordinate::Emu(Emu::from_emu(0)),
            top: AdjustCoordinate::Emu(Emu::from_emu(0)),
            right: AdjustCoordinate::Guide("w".to_owned()),
            bottom: AdjustCoordinate::Guide("h".to_owned()),
        }),
        paths: vec![Path2DSpec {
            commands: vec![
                DrawCommand::MoveTo(Point {
                    x: AdjustCoordinate::Guide("x1".to_owned()),
                    y: AdjustCoordinate::Guide("hc".to_owned()),
                }),
                DrawCommand::ArcTo {
                    width_radius: AdjustCoordinate::Guide("wd2".to_owned()),
                    height_radius: AdjustCoordinate::Emu(Emu::from_emu(25)),
                    start_angle: AdjustAngle::Guide("swAng".to_owned()),
                    swing_angle: AdjustAngle::Angle(mjx_dml::Angle::from_degrees(90.0)),
                },
                DrawCommand::Close,
            ],
            ..Default::default()
        }],
    }
}

#[test]
fn a_custom_geometry_resolves_every_coordinate_rect_edge_and_arc_angle() {
    let resolved = fixture_geometry()
        .resolve(wide_context())
        .expect("the fixture geometry resolves");

    let rect = resolved.text_rectangle.expect("a text rectangle");
    assert_eq!(rect.left, Emu::from_emu(0));
    assert_eq!(rect.right, Emu::from_emu(1000));
    assert_eq!(rect.bottom, Emu::from_emu(400));

    let commands = &resolved.paths[0].commands;
    let ResolvedDrawCommand::MoveTo(start) = commands[0] else {
        panic!("expected a moveTo");
    };
    // x1 = */ w adj1 100000 = 1000 * 25000 / 100000
    assert_eq!(start.x, Emu::from_emu(250));
    // hc, a built-in, resolves as readily as a declared guide.
    assert_eq!(start.y, Emu::from_emu(500));

    let ResolvedDrawCommand::ArcTo {
        width_radius,
        start_angle,
        swing_angle,
        ..
    } = commands[1]
    else {
        panic!("expected an arcTo");
    };
    assert_eq!(width_radius, Emu::from_emu(500));
    assert_close(start_angle.degrees(), 90.0, "cd4 is 90 degrees");
    assert_close(swing_angle.degrees(), 90.0, "a literal angle is itself");
    assert_eq!(commands[2], ResolvedDrawCommand::Close);
}

#[test]
fn resolving_a_geometry_uses_the_size_it_is_given() {
    let geometry = fixture_geometry();
    let narrow = geometry
        .resolve(GuideContext::from_extents(
            Emu::from_emu(400),
            Emu::from_emu(1000),
        ))
        .expect("resolves");
    let ResolvedDrawCommand::MoveTo(start) = narrow.paths[0].commands[0] else {
        panic!("expected a moveTo");
    };
    assert_eq!(start.x, Emu::from_emu(100), "x1 follows w");
    assert_eq!(start.y, Emu::from_emu(200), "hc follows w");
}

#[test]
fn a_geometry_naming_a_guide_it_never_defines_reports_the_name() {
    let mut geometry = fixture_geometry();
    geometry.paths[0].commands[0] = DrawCommand::MoveTo(Point {
        x: AdjustCoordinate::Guide("nosuch".to_owned()),
        y: AdjustCoordinate::Emu(Emu::from_emu(0)),
    });
    assert_eq!(
        geometry.resolve(wide_context()),
        Err(GuideError::UndefinedGuide {
            name: "nosuch".to_owned()
        })
    );
}

#[test]
fn a_geometry_with_a_cyclic_guide_list_reports_the_guide_rather_than_hanging() {
    let mut geometry = fixture_geometry();
    geometry.guides[0].formula = "+- x1 1 0".to_owned();
    let error = geometry
        .resolve(wide_context())
        .expect_err("a self-referencing guide does not resolve");
    assert!(
        matches!(&error, GuideError::Guide { guide, .. } if guide == "x1"),
        "{error}"
    );
}

#[test]
fn guide_values_expose_the_whole_evaluated_environment() {
    let geometry = fixture_geometry();
    let guides = geometry.guide_values(wide_context()).expect("evaluates");
    assert_close(guides.value("adj1").expect("adj1"), 25_000.0, "adj1");
    assert_close(guides.value("x1").expect("x1"), 250.0, "x1");
    assert_eq!(guides.len(), 3, "one adjust value and two computed guides");
    let mut names: Vec<&str> = guides.iter().map(|(name, _)| name).collect();
    names.sort_unstable();
    assert_eq!(names, ["adj1", "swAng", "x1"]);
}

// ---------------------------------------------------------------------------------------------
// Preset geometry — adjustment values and their numeric domains
// ---------------------------------------------------------------------------------------------

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

fn parse_preset(fragment: &str) -> (PresetGeometry, RawDocument) {
    let doc = fidelity::parse(fragment.as_bytes()).expect("fragment parses");
    let geometry = PresetGeometry::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (geometry, doc)
}

#[test]
fn a_computed_adjust_value_reads_as_a_number() {
    // Before the evaluator this returned `None` and the shape fell back to its 16667 default: only
    // `val N` was understood.
    let (geometry, doc) = parse_preset(&format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="roundRect"><a:avLst><a:gd name="adj" fmla="*/ 50000 1 2"/></a:avLst></a:prstGeom>"#
    ));
    assert_eq!(geometry.adjustment(&doc.interner, "adj"), Some(25_000));
    let adjustments = geometry.adjustments(&doc.interner);
    assert_eq!(adjustments.len(), 1);
    assert_eq!(adjustments[0].value, 25_000);
    assert!(adjustments[0].is_overridden);
}

#[test]
fn a_computed_adjust_value_may_build_on_an_earlier_one() {
    let (geometry, doc) = parse_preset(&format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="chevron"><a:avLst><a:gd name="seed" fmla="val 20000"/><a:gd name="adj" fmla="+- seed seed 0"/></a:avLst></a:prstGeom>"#
    ));
    assert_eq!(geometry.adjustment(&doc.interner, "adj"), Some(40_000));
}

#[test]
fn an_adjust_value_that_cannot_be_evaluated_leaves_the_default_standing() {
    // A malformed override costs only itself: the shape still reads, with the spec default.
    for formula in ["nonsense", "*/ 1 1 0", "+- w 0 0", "val"] {
        let (geometry, doc) = parse_preset(&format!(
            r#"<a:prstGeom xmlns:a="{A}" prst="roundRect"><a:avLst><a:gd name="adj" fmla="{formula}"/></a:avLst></a:prstGeom>"#
        ));
        assert_eq!(
            geometry.adjustment(&doc.interner, "adj"),
            Some(16_667),
            "`{formula}` should fall back to the roundRect default"
        );
        assert!(!geometry.adjustments(&doc.interner)[0].is_overridden);
    }
}

#[test]
fn evaluating_an_adjust_value_never_changes_what_is_written_back() {
    // Resolution is a read-side capability: the stored `fmla` text stays exactly the file's.
    let xml = format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="roundRect"><a:avLst><a:gd name="adj" fmla="*/ 50000 1 2" foo="bar"/></a:avLst></a:prstGeom>"#
    );
    let (geometry, mut doc) = parse_preset(&xml);
    assert_eq!(geometry.adjustment(&doc.interner, "adj"), Some(25_000));
    let _ = geometry.adjustments(&doc.interner);
    let _ = geometry.adjustments_for_size(&doc.interner, wide_context());

    doc.root = geometry.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        xml,
        "round-trip byte mismatch"
    );
}

#[test]
fn an_adjustment_domain_resolves_against_a_shape_size() {
    // chevron: `min` is the literal 0, `max` is the guide `maxAdj = */ 100000 w ss`. For a
    // 1000000 x 400000 shape ss is the height, so maxAdj = 100000 * 1000000 / 400000 = 250000.
    let (geometry, doc) = parse_preset(&format!(r#"<a:prstGeom xmlns:a="{A}" prst="chevron"/>"#));
    let wide = GuideContext::from_extents(Emu::from_emu(1_000_000), Emu::from_emu(400_000));
    let adjustments = geometry
        .adjustments_for_size(&doc.interner, wide)
        .expect("chevron's bound resolves");
    assert_eq!(adjustments.len(), 1);
    assert_close(adjustments[0].value, 50_000.0, "the chevron default");
    assert_close(adjustments[0].minimum, 0.0, "the literal minimum");
    assert_close(adjustments[0].maximum, 250_000.0, "maxAdj");
    assert!(!adjustments[0].is_overridden);

    // The bound follows the shape: turned on its side, ss is the width and maxAdj is 100000.
    let tall = GuideContext::from_extents(Emu::from_emu(400_000), Emu::from_emu(1_000_000));
    let adjustments = geometry
        .adjustments_for_size(&doc.interner, tall)
        .expect("chevron's bound resolves");
    assert_close(adjustments[0].maximum, 100_000.0, "maxAdj, turned around");
}

#[test]
fn an_out_of_range_adjustment_pins_into_its_resolved_domain() {
    let (geometry, doc) = parse_preset(&format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="chevron"><a:avLst><a:gd name="adj" fmla="val 999999"/></a:avLst></a:prstGeom>"#
    ));
    let wide = GuideContext::from_extents(Emu::from_emu(1_000_000), Emu::from_emu(400_000));
    let adjustments = geometry
        .adjustments_for_size(&doc.interner, wide)
        .expect("resolves");
    assert_close(adjustments[0].value, 999_999.0, "the value as written");
    assert_close(adjustments[0].pinned_value(), 250_000.0, "pinned to maxAdj");
    assert!(adjustments[0].is_overridden);
}

#[test]
fn a_bound_guide_chain_resolves_through_the_shapes_own_adjustments() {
    // `can`: maxAdj = */ 50000 h ss. For 1000000 x 400000, ss is the height, so maxAdj = 50000.
    let (geometry, doc) = parse_preset(&format!(r#"<a:prstGeom xmlns:a="{A}" prst="can"/>"#));
    let adjustments = geometry
        .adjustments_for_size(
            &doc.interner,
            GuideContext::from_extents(Emu::from_emu(1_000_000), Emu::from_emu(400_000)),
        )
        .expect("resolves");
    assert_close(adjustments[0].maximum, 50_000.0, "can's maxAdj");
}

#[test]
fn a_degenerate_shape_size_is_an_error_not_a_panic() {
    let (geometry, doc) = parse_preset(&format!(r#"<a:prstGeom xmlns:a="{A}" prst="chevron"/>"#));
    let error = geometry
        .adjustments_for_size(
            &doc.interner,
            GuideContext::from_extents(Emu::from_emu(0), Emu::from_emu(0)),
        )
        .expect_err("`*/ 100000 w ss` divides by zero when ss is zero");
    assert!(
        matches!(&error, GuideError::Guide { guide, .. } if guide == "maxAdj"),
        "{error}"
    );
}

#[test]
fn a_shape_with_no_adjustments_resolves_to_an_empty_list() {
    let (geometry, doc) = parse_preset(&format!(r#"<a:prstGeom xmlns:a="{A}" prst="rect"/>"#));
    assert!(geometry
        .adjustments_for_size(&doc.interner, wide_context())
        .expect("resolves")
        .is_empty());

    // An unknown `prst` names no shape, so there is nothing to resolve — and no error either.
    let (geometry, doc) = parse_preset(&format!(
        r#"<a:prstGeom xmlns:a="{A}" prst="fromTheFuture"/>"#
    ));
    assert!(geometry
        .adjustments_for_size(&doc.interner, wide_context())
        .expect("resolves")
        .is_empty());
}

#[test]
fn every_adjustment_of_every_adjustable_preset_shape_resolves_to_numbers() {
    // The whole point of the child: not one preset shape is left with a formula where a number
    // belongs. `adjustable_shapes` is generated from `presetShapeDefinitions.xml`, so this sweep
    // cannot drift from the spec's own shape set.
    let shapes = adjustable_shapes();
    assert_eq!(
        shapes.len(),
        119,
        "presetShapeDefinitions.xml defines 119 shapes with user-facing adjustments"
    );

    let mut interner = Interner::new();
    let sizes = [
        GuideContext::from_extents(Emu::from_emu(1_000_000), Emu::from_emu(400_000)),
        GuideContext::from_extents(Emu::from_emu(400_000), Emu::from_emu(1_000_000)),
        GuideContext::from_extents(Emu::from_emu(914_400), Emu::from_emu(914_400)),
    ];

    let mut resolved_shapes = 0usize;
    let mut resolved_adjustments = 0usize;
    for shape in shapes {
        let geometry = PresetGeometry::new(&mut interner, *shape, None);
        let specs = adjustments_of(*shape);
        assert!(!specs.is_empty(), "{shape:?} should have adjustments");
        for size in sizes {
            let adjustments = geometry
                .adjustments_for_size(&interner, size)
                .unwrap_or_else(|error| panic!("{shape:?} should resolve: {error}"));
            assert_eq!(adjustments.len(), specs.len(), "{shape:?}");
            for adjustment in &adjustments {
                assert!(
                    adjustment.value.is_finite(),
                    "{shape:?}/{} value",
                    adjustment.spec.wire_name
                );
                assert!(
                    adjustment.minimum.is_finite(),
                    "{shape:?}/{} minimum",
                    adjustment.spec.wire_name
                );
                assert!(
                    adjustment.maximum.is_finite(),
                    "{shape:?}/{} maximum",
                    adjustment.spec.wire_name
                );
                assert!(
                    adjustment.pinned_value().is_finite(),
                    "{shape:?}/{} pinned",
                    adjustment.spec.wire_name
                );
            }
            resolved_adjustments += adjustments.len();
        }
        resolved_shapes += 1;
    }
    assert_eq!(resolved_shapes, 119);
    assert_eq!(
        resolved_adjustments,
        285 * sizes.len(),
        "285 adjustments across the 119 shapes, at each of the three sizes"
    );
}

#[test]
fn every_guide_named_bound_names_a_guide_the_table_can_evaluate() {
    // A guide-named bound is only resolvable because `adjustment_bound_guides_of` carries the
    // closure it needs. Nothing else in the table is reachable, so this is the invariant that keeps
    // the sweep above honest as the generated data changes.
    for shape in adjustable_shapes() {
        let guides = adjustment_bound_guides_of(*shape);
        for spec in adjustments_of(*shape) {
            for bound in [spec.min, spec.max] {
                let AdjustmentBound::Guide(name) = bound else {
                    continue;
                };
                let known_guide = guides.iter().any(|guide| guide.wire_name == name);
                let known_adjustment = adjustments_of(*shape)
                    .iter()
                    .any(|other| other.wire_name == name);
                let built_in = wide_context().variable(name).is_some();
                assert!(
                    known_guide || known_adjustment || built_in,
                    "{shape:?} bounds an adjustment by `{name}`, which nothing defines"
                );
            }
        }
    }
}

#[test]
fn the_bound_guide_table_only_covers_shapes_that_need_it() {
    // 46 of the 119 adjustable shapes have a bound that is not a literal and not already a built-in
    // or an adjustment; the rest carry no guides at all. Keeping the table to that closure is what
    // makes it 334 guides rather than the file's 3923.
    let with_guides: Vec<PresetShapeType> = adjustable_shapes()
        .iter()
        .copied()
        .filter(|shape| !adjustment_bound_guides_of(*shape).is_empty())
        .collect();
    assert_eq!(with_guides.len(), 46);
    let total: usize = with_guides
        .iter()
        .map(|shape| adjustment_bound_guides_of(*shape).len())
        .sum();
    assert_eq!(total, 334);
    assert!(adjustment_bound_guides_of(PresetShapeType::Rectangle).is_empty());
}

// ---------------------------------------------------------------------------------------------
// The whole normative corpus — gated, because `References/` is git-ignored
// ---------------------------------------------------------------------------------------------

/// `presetShapeDefinitions.xml`, or `None` when the suite should skip.
///
/// The file ships with ECMA-376 Part 1 as an electronic addendum and is not committed (`References/`
/// is git-ignored by a standing rule of this repository), so the sweep below **skips** when it is
/// absent, printing a notice and passing — the pattern `schema_validity.rs` established.
/// `MJX_REQUIRE_PRESET_GEOMETRY=1` turns the absence into a hard failure;
/// `MJX_PRESET_GEOMETRY` overrides where the file is looked for.
///
/// Nothing else in this file depends on `References/`: the shape set, the adjustment table and the
/// bound-guide closure are all committed, generated data.
fn preset_shape_definitions() -> Option<Vec<u8>> {
    let path = match std::env::var_os("MJX_PRESET_GEOMETRY") {
        Some(path) => std::path::PathBuf::from(path),
        None => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../References/ECMA-376-1_5th_edition_december_2016")
            .join("OfficeOpenXML-DrawingMLGeometries/presetShapeDefinitions.xml"),
    };
    if let Ok(bytes) = std::fs::read(&path) {
        return Some(bytes);
    }
    assert!(
        std::env::var_os("MJX_REQUIRE_PRESET_GEOMETRY").is_none(),
        "MJX_REQUIRE_PRESET_GEOMETRY is set but {} could not be read",
        path.display()
    );
    eprintln!(
        "skipping the preset-shape-definitions sweep: {} not available on this machine",
        path.display()
    );
    None
}

/// Every attribute of the preset shape definitions whose value is an `ST_AdjCoordinate`, an
/// `ST_AdjAngle`, or a plain guide reference — everything a shape's geometry is placed by.
const COORDINATE_ATTRIBUTES: [&str; 17] = [
    "x", "y", "wR", "hR", "stAng", "swAng", "l", "t", "r", "b", "ang", "minX", "maxX", "minY",
    "maxY", "minR", "maxR",
];

/// One shape block of `presetShapeDefinitions.xml`, as it is read.
#[derive(Default)]
struct PresetShapeBlock {
    token: String,
    names: Vec<String>,
    formulas: Vec<String>,
    references: Vec<String>,
}

impl PresetShapeBlock {
    /// Records one element inside a shape: a guide definition, and every coordinate reference.
    fn record(&mut self, element: &mjx_xml::Element) {
        if element.name.local == "gd" {
            if let (Some(name), Some(formula)) = (element.attr("name"), element.attr("fmla")) {
                self.names.push(name.to_owned());
                self.formulas.push(formula.to_owned());
            }
        }
        for attribute in COORDINATE_ATTRIBUTES {
            if let Some(value) = element.attr(attribute) {
                if GuideFormula::parse(&format!("val {value}"))
                    .ok()
                    .map(|formula| matches!(formula.arguments()[0], GuideArgument::Name(_)))
                    .unwrap_or(false)
                {
                    self.references.push(value.to_owned());
                }
            }
        }
    }

    /// Evaluates the block's guides in declaration order, then resolves every coordinate reference
    /// it collected. Returns how many guides and how many references were checked, and every guide
    /// whose formula the addendum itself writes malformed.
    fn check(
        &self,
        context: GuideContext,
        defects: &mut Vec<(String, String, String)>,
    ) -> (usize, usize) {
        let mut resolved = ResolvedGuides::new(context);
        for (name, formula) in self.names.iter().zip(&self.formulas) {
            let value = match resolved.evaluate_formula(formula) {
                Ok(value) => value,
                Err(GuideError::Malformed { source, .. }) => {
                    // The addendum has a handful of guides with one argument too many (a stray
                    // trailing `0`). Strict arity is the right reading of "Arguments: 3", so the
                    // evaluator rejects them; to keep sweeping the other guides of these shapes,
                    // the obvious intent is substituted here and the defect is reported upward, so
                    // the set of them is pinned rather than quietly tolerated.
                    assert!(
                        matches!(source, GuideFormulaError::ArgumentCount { .. }),
                        "<{}>/{name}: {source}",
                        self.token
                    );
                    defects.push((self.token.clone(), name.clone(), formula.clone()));
                    let (corrected, _) = formula
                        .rsplit_once(char::is_whitespace)
                        .expect("a too-long formula has at least two tokens");
                    resolved
                        .evaluate_formula(corrected)
                        .unwrap_or_else(|error| panic!("<{}>/{name}: {error}", self.token))
                }
                Err(error) => panic!("<{}>/{name}: {error}", self.token),
            };
            assert!(value.is_finite(), "<{}>/{name} is not finite", self.token);
            resolved.define(name, value);
        }
        for reference in &self.references {
            let value = resolved.value(reference).unwrap_or_else(|| {
                panic!(
                    "<{}> places geometry at `{reference}`, which nothing defines",
                    self.token
                )
            });
            assert!(
                value.is_finite(),
                "<{}>/`{reference}` is not finite",
                self.token
            );
        }
        (self.names.len(), self.references.len())
    }
}

#[test]
fn every_guide_of_every_preset_shape_definition_evaluates() {
    let Some(xml) = preset_shape_definitions() else {
        return;
    };

    // A deliberately lopsided shape, so `ss`, `ls`, `w` and `h` are four different numbers and a
    // guide that confuses two of them cannot pass by coincidence.
    let context = GuideContext::from_extents(Emu::from_emu(1_000_000), Emu::from_emu(700_000));
    let mut reader = mjx_xml::Reader::new(&xml);
    let mut depth = 0usize;
    let mut block = PresetShapeBlock::default();
    let mut shapes = 0usize;
    let mut guides = 0usize;
    let mut references = 0usize;
    let mut defects: Vec<(String, String, String)> = Vec::new();

    loop {
        let event = reader.read().expect("presetShapeDefinitions.xml parses");
        match event {
            mjx_xml::Event::Start(element) => {
                depth += 1;
                if depth == 2 {
                    block = PresetShapeBlock {
                        token: element.name.local.clone(),
                        ..PresetShapeBlock::default()
                    };
                } else if depth > 2 {
                    block.record(&element);
                }
            }
            mjx_xml::Event::Empty(element) => {
                if depth >= 2 {
                    block.record(&element);
                }
            }
            mjx_xml::Event::End(_) => {
                if depth == 2 {
                    let (shape_guides, shape_references) = block.check(context, &mut defects);
                    shapes += 1;
                    guides += shape_guides;
                    references += shape_references;
                }
                depth = depth.saturating_sub(1);
            }
            mjx_xml::Event::Text(_) => {}
            mjx_xml::Event::Eof => break,
        }
    }

    // The December 2016 addendum: 187 shape blocks (`upDownArrow` is defined twice, byte-identical)
    // and 3923 guides between them.
    assert_eq!(shapes, 187, "shape blocks");
    assert_eq!(guides, 3_923, "guides");
    assert!(references > 2_000, "coordinate references: {references}");

    // Exactly eight guides in the addendum are written with one argument too many, all of the form
    // `+- A 0 B 0`, all in the three circular-arrow shapes. Pinned so a change in either the file or
    // the evaluator's strictness is visible rather than silent.
    let defects: Vec<(&str, &str, &str)> = defects
        .iter()
        .map(|(shape, guide, formula)| (shape.as_str(), guide.as_str(), formula.as_str()))
        .collect();
    assert_eq!(
        defects,
        [
            ("circularArrow", "xB", "+- xH 0 dxB 0"),
            ("circularArrow", "yB", "+- yH 0 dyB 0"),
            ("leftCircularArrow", "xB", "+- xH 0 dxB 0"),
            ("leftCircularArrow", "yB", "+- yH 0 dyB 0"),
            ("leftRightCircularArrow", "xB", "+- xH 0 dxB 0"),
            ("leftRightCircularArrow", "yB", "+- yH 0 dyB 0"),
            ("leftRightCircularArrow", "xJ", "+- xI 0 dxJ 0"),
            ("leftRightCircularArrow", "yJ", "+- yI 0 dyJ 0"),
        ]
    );
}
