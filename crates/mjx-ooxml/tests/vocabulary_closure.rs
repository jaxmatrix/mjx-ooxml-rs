//! The re-export list is **closed under reachability**, proved by using the parts that close it.
//!
//! `public_paths.rs` proves every `Deck` method is callable naming only `mjx_ooxml`. That is not
//! quite the whole claim. A method can be callable while a *field of what it returns*, or a *variant
//! payload of what it takes*, is a type no caller can name — and the compiler will not complain,
//! because the value still moves around opaquely in Rust. A binding cannot be that vague: PyO3 and
//! wasm-bindgen must wrap each of those types by name to hand it to Python or TypeScript.
//!
//! Writing the bindings therefore found ten types reachable from the vocabulary but absent from it.
//! This file names all ten and — more usefully — *destructures and reconstructs* the values that
//! carry them, through the facade, on a real deck. Deleting any one of the ten from
//! `mjx_ooxml`'s re-exports stops this file compiling.

use mjx_ooxml::{
    AdjustCoordinate, AdjustHandle, AdjustmentAxis, AdjustmentBound, AdjustmentSpec, AxisKind,
    ChartData, ChartKind, ColorKind, ColorSpec, ConnectionSite, CustomGeometrySpec, Deck,
    FontSchemeSlot, FontSlot, Geometry, GuideContext, GuideSpec, LineSpec, LineWidth, Point,
    PresetShapeType, ShapeBounds, SlideSize, TableStyleBorder, TableStyleFormat, TextFont,
    ThemeFontReference,
};

fn deck_with_one_slide() -> (Deck, u32) {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    (deck, slide)
}

/// `Geometry::Custom` carries a `CustomGeometrySpec`, whose `adjust_handles` and
/// `connection_sites` are `AdjustHandle` and `ConnectionSite`. Neither was nameable before.
#[test]
fn a_custom_geometry_round_trips_through_its_handles_and_sites() {
    let (mut deck, slide) = deck_with_one_slide();
    let shape = deck
        .add_shape(
            slide.into(),
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0),
        )
        .expect("a shape");

    let spec = CustomGeometrySpec {
        guides: vec![GuideSpec {
            name: "half".into(),
            formula: "*/ w 1 2".into(),
        }],
        connection_sites: vec![ConnectionSite {
            angle: mjx_ooxml::AdjustAngle::Angle(mjx_ooxml::Angle::from_degrees(90.0)),
            position: Point::from_emu(100, 200),
        }],
        adjust_handles: vec![AdjustHandle::Xy {
            position: Point::from_emu(300, 400),
            guide_ref_x: Some("half".into()),
            min_x: Some(AdjustCoordinate::Emu(mjx_ooxml::Emu::from_emu(0))),
            max_x: None,
            guide_ref_y: None,
            min_y: None,
            max_y: None,
        }],
        ..CustomGeometrySpec::default()
    };

    deck.set_shape_geometry(slide.into(), shape.into(), Geometry::Custom(spec))
        .expect("the custom geometry is written");

    let read = deck
        .shape_geometry(slide.into(), shape.into())
        .expect("the geometry reads back");
    let Geometry::Custom(read) = read else {
        panic!("a custom geometry must read back as one");
    };

    // The site and the handle survived the trip, and both are destructurable from here.
    assert_eq!(read.connection_sites.len(), 1);
    assert_eq!(read.connection_sites[0].position, Point::from_emu(100, 200));
    match &read.adjust_handles[0] {
        AdjustHandle::Xy {
            position,
            guide_ref_x,
            ..
        } => {
            assert_eq!(*position, Point::from_emu(300, 400));
            assert_eq!(guide_ref_x.as_deref(), Some("half"));
        }
        AdjustHandle::Polar { .. } => panic!("an `ahXY` must not read back as an `ahPolar`"),
    }
}

/// `Deck::shape_adjustments` hands back `BoundedAdjustment`s, each holding a
/// `&'static AdjustmentSpec` — whose `axis` and `min`/`max` are `AdjustmentAxis` and
/// `AdjustmentBound`. All three were unnameable, so a binding could report the value but never say
/// which adjustment it was.
#[test]
fn a_preset_adjustment_names_its_own_specification() {
    let (mut deck, slide) = deck_with_one_slide();
    let shape = deck
        .add_shape(
            slide.into(),
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("a rounded rectangle");

    let adjustments = deck
        .shape_adjustments(
            slide.into(),
            shape.into(),
            GuideContext::from_extents(
                mjx_ooxml::Emu::from_emu(1_828_800),
                mjx_ooxml::Emu::from_emu(914_400),
            ),
        )
        .expect("its adjustments");

    let first = adjustments.first().expect("a rounded rectangle has `adj`");
    let spec: &AdjustmentSpec = first.spec;
    assert_eq!(spec.wire_name, "adj");
    assert_eq!(spec.axis, AdjustmentAxis::Horizontal);
    // The bounds are literal, not guide-relative, for this shape.
    assert!(matches!(spec.min, AdjustmentBound::Literal(_)));
    assert!(matches!(spec.max, AdjustmentBound::Literal(_)));
    assert!(first.minimum <= first.pinned_value() && first.pinned_value() <= first.maximum);
}

/// `ChartAxisData::kind` is an `AxisKind`, which says whether an axis is the category axis or the
/// value axis — the one field a caller must read before deciding what a scale change means.
#[test]
fn a_chart_axis_names_its_kind() {
    let (mut deck, slide) = deck_with_one_slide();
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("2026", [1.0, 2.0]);
    let frame = deck
        .add_chart(
            slide.into(),
            &chart,
            ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0),
        )
        .expect("a chart");

    let axes = deck
        .chart_axes(slide.into(), frame.into())
        .expect("its axes");
    let kinds: Vec<AxisKind> = axes.iter().map(|axis| axis.kind).collect();
    assert!(
        kinds.contains(&AxisKind::Category) && kinds.contains(&AxisKind::Value),
        "a bar chart states a category axis and a value axis, got {kinds:?}"
    );
}

/// `ColorSpec::Other` carries a `ColorKind`: the one arm that says *what kind of colour element*
/// the document held when it was not an `srgbClr` or a `schemeClr`.
#[test]
fn a_colour_that_is_neither_srgb_nor_scheme_names_its_kind() {
    let other = ColorSpec::Other {
        kind: ColorKind::Hsl,
        value: Some("0 100000 50000".into()),
    };
    let ColorSpec::Other { kind, value } = &other else {
        panic!("the variant we just built");
    };
    assert_eq!(*kind, ColorKind::Hsl);
    assert_eq!(value.as_deref(), Some("0 100000 50000"));
}

/// `CharacterPropertiesSpec::with_font_for` / `font` are keyed by `FontSlot`, and a theme font
/// reference is a `FontSchemeSlot` plus a `FontSlot`. Without both, a binding can set the Latin
/// typeface and nothing else.
#[test]
fn a_run_can_state_a_font_per_script_slot() {
    let spec = mjx_ooxml::CharacterPropertiesSpec::new()
        .with_font_for(FontSlot::Latin, TextFont::named("Calibri"))
        .with_font_for(FontSlot::EastAsian, TextFont::named("Yu Gothic"))
        .with_font_for(FontSlot::ComplexScript, TextFont::named("Arial"));

    assert_eq!(
        spec.font(FontSlot::EastAsian)
            .map(|font| font.typeface.as_str()),
        Some("Yu Gothic")
    );
    assert_eq!(spec.font(FontSlot::Symbol), None);

    let major_latin = TextFont::named("+mj-lt");
    assert_eq!(
        major_latin.theme_reference(),
        Some(ThemeFontReference {
            collection: FontSchemeSlot::Major,
            slot: FontSlot::Latin,
        })
    );
}

/// `TableStyleFormat::with_border` is keyed by `TableStyleBorder` — eight edges including the two
/// *inside* ones, which have no counterpart in the per-cell `CellBorder`.
#[test]
fn a_table_style_part_can_state_its_inside_borders() {
    let (mut deck, slide) = deck_with_one_slide();
    let table = deck
        .add_table(
            slide.into(),
            2,
            2,
            ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0),
        )
        .expect("a table");

    deck.create_table_style("{4CFB1A47-DDDD-4A9B-BB5D-A1B0EF2E2C21}", "Inside grid")
        .expect("a new style");
    deck.format_table_style_part(
        "{4CFB1A47-DDDD-4A9B-BB5D-A1B0EF2E2C21}",
        mjx_ooxml::TableStylePart::WholeTable,
        &TableStyleFormat::new()
            .with_border(
                TableStyleBorder::InsideHorizontal,
                LineSpec::solid(
                    LineWidth::from_points(1.0),
                    ColorSpec::Srgb("1F3864".into()),
                ),
            )
            .with_border(
                TableStyleBorder::InsideVertical,
                LineSpec::solid(
                    LineWidth::from_points(1.0),
                    ColorSpec::Srgb("1F3864".into()),
                ),
            ),
    )
    .expect("the part formats");
    deck.set_table_style(
        slide.into(),
        table.into(),
        "{4CFB1A47-DDDD-4A9B-BB5D-A1B0EF2E2C21}",
    )
    .expect("the table adopts it");

    assert_eq!(
        deck.table_style_id(slide.into(), table.into())
            .expect("reading the id"),
        Some("{4CFB1A47-DDDD-4A9B-BB5D-A1B0EF2E2C21}".to_owned())
    );
    deck.validate().expect("the styled deck still validates");
}

/// `GuideError::Malformed` carries a `GuideFormulaError`, the only value that says *why* a formula
/// did not parse. A binding that cannot name it can only report "malformed".
#[test]
fn a_malformed_guide_formula_names_its_own_failure() {
    let failure = mjx_ooxml::ResolvedGuides::evaluate(
        [("bad", "notanoperator 1 2")],
        GuideContext::from_extents(
            mjx_ooxml::Emu::from_emu(914_400),
            mjx_ooxml::Emu::from_emu(914_400),
        ),
    )
    .expect_err("`notanoperator` is not one of the seventeen operators");

    // The failure is reported against the guide that carried it, so the parse error is one layer in.
    let mjx_ooxml::GuideError::Guide { guide, source } = &failure else {
        panic!("a failure inside a named guide is `Guide`, got {failure:?}");
    };
    assert_eq!(guide, "bad");
    let mjx_ooxml::GuideError::Malformed { source, .. } = source.as_ref() else {
        panic!("an unparseable formula is `Malformed`, got {source:?}");
    };
    assert!(
        matches!(source, mjx_ooxml::GuideFormulaError::UnknownOperator { token } if token == "notanoperator"),
        "the cause must name the token it choked on, got {source:?}"
    );
}
