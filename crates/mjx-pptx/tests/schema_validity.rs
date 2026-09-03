//! Schema validity for PowerPoint: markup this project ships or authors must never deviate from
//! ECMA-376.
//!
//! The harness itself lives in the workspace-level [`mjx_schema_gate`] crate — an integration test
//! compiles only into its own crate, so a harness in this directory could never be reached from
//! `mjx-docx` or `mjx-xlsx`. What stays here is what is genuinely PresentationML: the committed
//! `.pptx` corpus and **every deck this library authors**, which is the half that protects future
//! work, because a new authoring path cannot land invalid markup without a case here going red.
//!
//! `mjx_schema_gate` draws the line — see its `categories` module for the three-category rule, and
//! its `harness` module for the skip behaviour and for how `wml.xsd`'s undeclared `xml.xsd` import
//! is resolved. This file also carries the **whole-workspace meta-gate**, because it is the only
//! place that can see both the committed corpus and the authoring paths at once.

use mjx_dml::{
    AdjustAngle, AdjustCoordinate, Angle, Bevel, BevelPreset, Camera, CellBorder,
    CharacterPropertiesSpec, ColorSpec, ConnectionSite, CustomGeometrySpec, DrawCommand,
    EffectListSpec, Emu, FillSpec, Fraction, GlowEffect, GradientStopSpec, GuideSpec, IndentLevel,
    LightRig, LightRigDirection, LightRigType, LineCap, LineDash, LineJoin, LineSpec, LineWidth,
    OnOffStyle, OuterShadowEffect, ParagraphPropertiesSpec, Path2DSpec, PatternType,
    PictureFillMode, Point, PresetCamera, PresetLineDash, PresetMaterial, Rectangle,
    RectangleAlignment, Scene3DSpec, SchemeColor, Shape3DSpec, ShapeGeometry, TablePart,
    TableStyleBorder, TableStylePart, TextAlignment, TextAnchoring, TextSpacing,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::presentationml::SlideSizeKind;
use mjx_pptx::{
    default_placeholder_ole, ActiveXControlSpec, ActiveXPersistence, AxisOrientation, CellFormat,
    CellMargins, Cells, ChartData, ChartKind, ChartLabelScope, DataLabelPosition, DataLabelSpec,
    DiagramContent, DiagramPartKind, ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorValueType,
    Geometry, Hyperlink, LegendPosition, OleObjectData, OleObjectSpec, Presentation, ShapeBounds,
    SlideSize, Surface, TableStyleFormat, TrendlineKind, TrendlineSpec,
};
use mjx_schema_gate::{
    assert_authored_deck_is_schema_valid, assert_fixture_is_schema_valid, audit_deck_order,
    fixture, harness, inspect_deck, inspect_fixture, outcome_table, package_fixtures,
    package_fixtures_with_extension, PartOutcome, Sweep,
};

/// A valid 2×2 truecolour PNG (76 bytes), inlined so no binary fixture is committed.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

// ---------------------------------------------------------------------------------------------
// The committed fixtures — the corpus is the directory, never a list
// ---------------------------------------------------------------------------------------------

#[test]
fn every_pptx_fixture_is_schema_valid() {
    // This used to be thirteen named cases plus a fourteenth asserting the thirteen matched the
    // directory. The directory is the only source of truth now: a fixture added in any later phase
    // is validated the moment it lands, and there is no list to forget to update.
    let fixtures = package_fixtures_with_extension("pptx");
    assert!(
        fixtures.len() >= 13,
        "the .pptx corpus shrank to {} fixtures",
        fixtures.len()
    );
    for name in fixtures {
        assert_fixture_is_schema_valid(&name);
    }
}

#[test]
fn markup_compatibility_is_resolved_and_validated_rather_than_skipped() {
    // The MCE skip used to be a hole with a name on it: a part carrying `mc:AlternateContent` was
    // reported skipped and never validated. Resolving instead is what lets `word/document.xml` —
    // which LibreOffice writes `mc:Ignorable` on — be validated at all, so pin the PowerPoint half
    // of that mechanism: both fixtures that carry markup compatibility are *validated*, against
    // `pml.xsd`, with the winning `mc:Choice` or `mc:Fallback` in place.
    let Some(harness) = harness() else { return };

    for (name, part) in [
        ("ole.pptx", "/ppt/slides/slide1.xml"),
        ("ink.pptx", "/ppt/slides/slide1.xml"),
    ] {
        let rows = inspect_deck(&harness, name, &fixture(name), &[]);
        let row = rows
            .iter()
            .find(|row| row.name == part)
            .unwrap_or_else(|| panic!("{name}: {part} is not in the sweep"));
        assert!(
            matches!(row.outcome, PartOutcome::Validated("pml.xsd")),
            "{name}{part} carries markup compatibility and must be resolved and validated, not \
             skipped — it reported: {}",
            row.outcome.describe()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The whole-workspace meta-gate
// ---------------------------------------------------------------------------------------------

/// A deck that reaches every authoring path whose markup no committed fixture carries.
///
/// Only the diagram parts need this today — `dgm:` markup exists in no fixture — but building it
/// through the public surface rather than listing part names is what keeps the meta-gate honest.
fn deck_reaching_the_authored_only_schemas() -> Vec<u8> {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_diagram(
        0,
        &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
    )
    .expect("add diagram");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["a", "b"])
        .series("s", [1.0, 2.0]);
    pres.add_chart(0, &chart, ShapeBounds::from_inches(1.0, 4.0, 4.0, 2.0))
        .expect("add chart");
    pres.save().expect("save")
}

#[test]
fn every_schema_arm_is_exercised_and_every_preserved_skip_is_reached() {
    // The systemic guard. Two facts about the sweep as a whole, neither of which any per-package
    // assertion can state:
    //
    //  * every arm of the schema table is reached by something — an arm nothing exercises is an arm
    //    nobody would notice breaking, and is indistinguishable from an arm that does not exist;
    //  * every entry of the category-2 allowlist is reached by something — a dead entry is an
    //    unproven claim about markup the corpus does not contain.
    //
    // The third fact — a namespace on neither list fails, naming it — is enforced per part by
    // `PartOutcome::Uncategorised`, which is why it is not restated here.
    let Some(harness) = harness() else { return };

    let mut sweep = Sweep::new();
    for name in package_fixtures() {
        let tolerances = mjx_schema_gate::tolerances_for(&name);
        sweep.record(
            &name,
            &inspect_deck(&harness, &name, &fixture(&name), &tolerances),
        );
    }
    let authored = deck_reaching_the_authored_only_schemas();
    sweep.record(
        "a deck reaching the authored-only schemas",
        &inspect_deck(&harness, "authored", &authored, &[]),
    );

    sweep.assert_every_modeled_schema_was_exercised();
    sweep.assert_pinned_skips();
}

#[test]
fn the_fixture_directory_holds_nothing_the_corpora_would_ignore() {
    // A `.dotx` or a `.xlsm` dropped in here must join a corpus by being classified, never by being
    // ignored: every byte-identity suite and this one derive their corpus from the same directory.
    mjx_schema_gate::assert_every_fixture_has_a_known_kind();
}

#[test]
fn the_per_part_table_is_printed_for_every_pptx_fixture() {
    // The report this child is graded on. Printing it as a test keeps it reproducible rather than
    // something someone once pasted into a ticket.
    for name in package_fixtures_with_extension("pptx") {
        let rows = inspect_fixture(&name);
        if rows.is_empty() {
            return;
        }
        println!("{}", outcome_table(&name, &rows));
    }
}
// ---------------------------------------------------------------------------------------------
// The decks this library authors — the half that protects future work
// ---------------------------------------------------------------------------------------------

/// The four slide extents worth checking: PowerPoint's two defaults, and the two ends of what
/// `ST_SlideSizeCoordinate` permits — the placeholder geometry is rescaled per deck, so a bad
/// rescale shows up as an invalid `a:ext` at one end and not the other.
const BLANK_DECK_SIZES: &[(&str, i64, i64, SlideSizeKind)] = &[
    ("16:9", 12_192_000, 6_858_000, SlideSizeKind::Screen16X9),
    ("4:3", 9_144_000, 6_858_000, SlideSizeKind::Screen4X3),
    ("smallest", 914_400, 914_400, SlideSizeKind::Custom),
    ("largest", 51_206_400, 51_206_400, SlideSizeKind::Custom),
];

#[test]
fn a_blank_deck_is_schema_valid() {
    // `Presentation::blank` writes `presentation.xml`, a theme, a slide master and a slide layout
    // from nothing, plus the `[Content_Types].xml` and four `.rels` parts underneath them. Nothing
    // in this deck came from a file, so every byte of it is this project's to answer for — which is
    // exactly why a committed binary template was refused.
    for (label, width_emu, height_emu, kind) in BLANK_DECK_SIZES.iter().copied() {
        let deck = Presentation::blank(SlideSize {
            width_emu,
            height_emu,
            kind,
        })
        .expect("blank");
        let saved = deck.save().expect("save");
        assert_authored_deck_is_schema_valid(&format!("blank deck ({label})"), &saved);
    }
}

#[test]
fn a_blank_deck_filled_end_to_end_is_schema_valid() {
    // The whole "create a document from nothing" story: a blank deck, a slide built on its own
    // layout, both placeholders filled, a text box and a second slide added — every part authored,
    // none preserved.
    let mut deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    let slide = deck
        .add_slide_from_layout(0)
        .expect("add slide from layout");
    deck.set_shape_text_content(slide, 0, "Built from nothing")
        .expect("set the title");
    deck.set_shape_text_content(slide, 1, "First point\nSecond point")
        .expect("set the body");
    deck.add_text_box(
        slide,
        "A text box too",
        ShapeBounds::from_inches(1.0, 5.0, 4.0, 1.0),
    )
    .expect("add text box");
    deck.add_slide_with_text(
        "A second slide",
        ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0),
    )
    .expect("add slide with text");
    let saved = deck.save().expect("save");
    assert_authored_deck_is_schema_valid("blank deck, filled end to end", &saved);
}

#[test]
fn the_blank_deck_validates_every_part_it_ships() {
    // A classification bug that skipped the new parts would let invalid markup through as a pass,
    // so pin the verdicts: all nine entries are accounted for and the five markup streams are
    // genuinely validated, not skipped.
    let Some(harness) = harness() else { return };
    let deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    let saved = deck.save().expect("save");
    let rows = inspect_deck(&harness, "blank deck coverage", &saved, &[]);

    let validated: Vec<&str> = rows
        .iter()
        .filter(|row| matches!(row.outcome, PartOutcome::Validated(_)))
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(
        validated,
        [
            "/[Content_Types].xml",
            "/_rels/.rels",
            "/ppt/presentation.xml",
            "/ppt/slideMasters/slideMaster1.xml",
            "/ppt/slideLayouts/slideLayout1.xml",
            "/ppt/theme/theme1.xml",
            "/ppt/_rels/presentation.xml.rels",
            "/ppt/slideMasters/_rels/slideMaster1.xml.rels",
            "/ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        ],
        "every entry of a blank deck must be validated, none skipped"
    );
    assert_eq!(rows.len(), validated.len());
}

#[test]
fn the_child_order_audit_reaches_every_authored_markup_part_and_is_not_vacuous() {
    // An audit that visits nothing passes for the wrong reason. Pin which parts of a filled blank
    // deck the child-order walk reaches, and that it descends into each of them rather than stopping
    // at the root — this is what makes `assert_authored_deck_is_schema_valid` a real ordering gate
    // on every one of the authoring cases in this file, with or without `References/`.
    let mut deck = Presentation::blank(SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: SlideSizeKind::Screen16X9,
    })
    .expect("blank");
    let slide = deck
        .add_slide_from_layout(0)
        .expect("add slide from layout");
    deck.set_shape_text_content(slide, 0, "Ordered by construction")
        .expect("set the title");
    let saved = deck.save().expect("save");

    let audited = audit_deck_order("blank deck order coverage", &saved);
    let parts: Vec<&str> = audited.iter().map(|part| part.name.as_str()).collect();
    assert_eq!(
        parts,
        [
            "/ppt/presentation.xml",
            "/ppt/slideMasters/slideMaster1.xml",
            "/ppt/slideLayouts/slideLayout1.xml",
            "/ppt/theme/theme1.xml",
            "/ppt/slides/slide1.xml",
        ],
        "every PresentationML and DrawingML part this deck authors must be audited"
    );
    for part in &audited {
        assert!(
            part.elements_visited > 5,
            "{}: the walk checked only {} elements — it is not descending",
            part.name,
            part.elements_visited
        );
    }
}

#[test]
fn an_unedited_deck_saves_schema_valid() {
    // The save path itself: open and re-emit, touching nothing. A regression in the writer that
    // corrupted a part would show up here before any authoring case.
    let pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("saved unedited sample.pptx", &saved);
}

#[test]
fn an_added_text_box_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_text_box(
        0,
        "Schema canary\nLine two",
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0),
    )
    .expect("add text box");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added text box", &saved);
}

#[test]
fn an_added_slide_is_schema_valid() {
    // `build::empty_slide_bytes` — a whole new part with its own root and namespaces.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide().expect("add slide");
    pres.add_slide_with_text("Second slide", ShapeBounds::from_inches(1.0, 1.0, 5.0, 2.0))
        .expect("add slide with text");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added slides", &saved);
}

#[test]
fn a_slide_built_from_a_layout_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Built from a layout")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "The placeholders came with the slide")
        .expect("set the body");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("slide from a layout", &saved);
}

#[test]
fn speaker_notes_are_schema_valid() {
    // `build::empty_notes_slide_bytes` and `build::notes_master_bytes` — sample.pptx has neither a
    // notes slide nor a notes master, so both templates are synthesized here.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.set_notes_text(0, "Speaker notes, written from scratch")
        .expect("set notes");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("speaker notes", &saved);
}

#[test]
fn an_added_picture_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let picture = pres
        .add_picture(0, TINY_PNG, ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0))
        .expect("add picture");
    pres.set_shape_outline(
        0,
        picture,
        &LineSpec {
            fill: Some(FillSpec::solid(ColorSpec::Srgb("203864".into()))),
            width: Some(LineWidth::from_points(3.0)),
            ..LineSpec::new()
        },
    )
    .expect("outline the picture");

    let filled = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(5.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    let rel_id = pres.add_image(0, TINY_PNG).expect("add image");
    pres.set_shape_fill(
        0,
        filled,
        &FillSpec::Picture {
            rel_id,
            mode: PictureFillMode::Stretch,
        },
    )
    .expect("picture fill");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("added picture", &saved);
}

#[test]
fn shape_geometry_fill_outline_and_effects_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");

    let preset = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(0.5, 0.5, 3.0, 1.5),
        )
        .expect("add shape");
    pres.set_shape_geometry(
        0,
        preset,
        Geometry::Preset(ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.3),
        }),
    )
    .expect("set geometry");
    pres.set_shape_fill(
        0,
        preset,
        &FillSpec::linear_gradient(
            vec![
                GradientStopSpec {
                    position: Fraction::from_ratio(0.0),
                    color: ColorSpec::Srgb("FF0000".into()),
                },
                GradientStopSpec {
                    position: Fraction::from_ratio(1.0),
                    color: ColorSpec::Scheme(SchemeColor::Accent1),
                },
            ],
            Angle::from_degrees(45.0),
        ),
    )
    .expect("gradient fill");
    pres.set_shape_outline(
        0,
        preset,
        &LineSpec {
            width: Some(LineWidth::from_points(3.0)),
            cap: Some(LineCap::Round),
            fill: Some(FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent1))),
            dash: Some(LineDash::Preset(PresetLineDash::Dash)),
            join: Some(LineJoin::Round),
            ..LineSpec::new()
        },
    )
    .expect("outline");
    pres.set_shape_effects(
        0,
        preset,
        &EffectListSpec {
            glow: Some(GlowEffect {
                color: ColorSpec::Scheme(SchemeColor::Accent1),
                radius: Some(Emu::from_points(5.0)),
            }),
            outer_shadow: Some(OuterShadowEffect {
                color: ColorSpec::Srgb("808080".into()),
                blur_radius: Some(Emu::from_points(4.0)),
                distance: Some(Emu::from_points(3.0)),
                direction: Some(Angle::from_degrees(45.0)),
                scale_x: None,
                scale_y: None,
                skew_x: None,
                skew_y: None,
                alignment: Some(RectangleAlignment::BottomRight),
                rotate_with_shape: Some(false),
            }),
            ..EffectListSpec::new()
        },
    )
    .expect("effects");

    let pattern = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(0.5, 2.5, 3.0, 1.5),
        )
        .expect("add pattern shape");
    pres.set_shape_fill(
        0,
        pattern,
        &FillSpec::pattern(
            PatternType::Percent25,
            ColorSpec::Srgb("000000".into()),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )
    .expect("pattern fill");

    let custom = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(4.5, 0.5, 2.0, 2.0),
        )
        .expect("add custom-geometry shape");
    pres.set_shape_geometry(
        0,
        custom,
        Geometry::Custom(CustomGeometrySpec {
            paths: vec![Path2DSpec {
                width: Some(Emu::from_emu(1_828_800)),
                height: Some(Emu::from_emu(1_828_800)),
                commands: vec![
                    DrawCommand::MoveTo(Point::from_emu(914_400, 0)),
                    DrawCommand::LineTo(Point::from_emu(1_828_800, 1_828_800)),
                    DrawCommand::LineTo(Point::from_emu(0, 1_828_800)),
                    DrawCommand::Close,
                ],
                ..Path2DSpec::default()
            }],
            ..CustomGeometrySpec::default()
        }),
    )
    .expect("custom geometry");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("shape geometry, fill, outline and effects", &saved);
}

#[test]
fn a_guide_driven_custom_geometry_is_schema_valid() {
    // `CT_CustomGeometry2D` is a fixed sequence — `avLst`, `gdLst`, `ahLst`, `cxnLst`, `rect`, then
    // the required `pathLst`. The case above authors only the path list; this one authors every
    // auxiliary child, which is the geometry the guide-formula evaluator exists to read, so a
    // misordered or malformed guide list cannot slip out of the writer unnoticed.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0),
        )
        .expect("add shape");
    pres.set_shape_geometry(
        0,
        idx,
        Geometry::Custom(CustomGeometrySpec {
            adjust_values: vec![GuideSpec {
                name: "adj1".to_owned(),
                formula: "val 25000".to_owned(),
            }],
            guides: vec![GuideSpec {
                name: "apex".to_owned(),
                formula: "*/ w adj1 100000".to_owned(),
            }],
            connection_sites: vec![ConnectionSite {
                angle: AdjustAngle::Guide("3cd4".to_owned()),
                position: Point {
                    x: AdjustCoordinate::Guide("apex".to_owned()),
                    y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                },
            }],
            text_rectangle: Some(Rectangle {
                left: AdjustCoordinate::Guide("l".to_owned()),
                top: AdjustCoordinate::Guide("t".to_owned()),
                right: AdjustCoordinate::Guide("r".to_owned()),
                bottom: AdjustCoordinate::Guide("b".to_owned()),
            }),
            paths: vec![Path2DSpec {
                commands: vec![
                    DrawCommand::MoveTo(Point {
                        x: AdjustCoordinate::Guide("apex".to_owned()),
                        y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                    }),
                    DrawCommand::LineTo(Point::from_emu(1_828_800, 1_828_800)),
                    DrawCommand::LineTo(Point::from_emu(0, 1_828_800)),
                    DrawCommand::Close,
                ],
                ..Path2DSpec::default()
            }],
            ..CustomGeometrySpec::default()
        }),
    )
    .expect("custom geometry");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("a guide-driven custom geometry", &saved);
}

#[test]
fn a_3d_shape_is_schema_valid() {
    // `a:scene3d` is exactly where defect B lived: `CT_Scene3D` requires a camera *and* a light rig,
    // and a scene with only a camera is invalid. Our writer must never emit one.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
        )
        .expect("add shape");
    pres.set_shape_scene_3d(
        0,
        idx,
        &Scene3DSpec {
            camera: Camera {
                preset: PresetCamera::OrthographicFront,
                field_of_view: None,
                zoom: Some(Fraction::from_ratio(1.0)),
                rotation: None,
            },
            light_rig: LightRig {
                rig: LightRigType::ThreePoint,
                direction: LightRigDirection::Top,
                rotation: None,
            },
        },
    )
    .expect("set scene");
    pres.set_shape_3d_properties(
        0,
        idx,
        &Shape3DSpec {
            z: None,
            extrusion_height: Some(Emu::from_emu(190_500)),
            contour_width: Some(Emu::from_emu(12_700)),
            material: Some(PresetMaterial::Metal),
            bevel_top: Some(Bevel {
                width: Some(Emu::from_emu(76_200)),
                height: Some(Emu::from_emu(38_100)),
                preset: Some(BevelPreset::Circle),
            }),
            bevel_bottom: None,
            extrusion_color: Some(ColorSpec::Srgb("C0C0C0".to_owned())),
            contour_color: Some(ColorSpec::Srgb("404040".to_owned())),
        },
    )
    .expect("set 3-D properties");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("3-D shape", &saved);
}

#[test]
fn grouped_shapes_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let first = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("add first");
    let second = pres
        .add_shape(
            0,
            PresetShapeType::Ellipse,
            ShapeBounds::from_inches(4.0, 1.0, 2.0, 1.0),
        )
        .expect("add second");
    pres.group_shapes(0, &[first.into(), second.into()])
        .expect("group");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("grouped shapes", &saved);
}

#[test]
fn a_created_table_is_schema_valid() {
    // The whole table builder: the graphic frame, the grid, every cell, cell formatting, merges, and
    // growing and shrinking the grid afterwards.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.5, 8.0, 3.0))
        .expect("add table");
    for (row, column, text) in [
        (0, 0, "Region"),
        (0, 1, "Revenue"),
        (0, 2, "Change"),
        (1, 0, "North"),
        (1, 1, "1,204"),
        (1, 2, "+12%"),
        (2, 0, "South"),
        (2, 1, "987"),
        (2, 2, "-3%"),
    ] {
        pres.set_cell_text(0, table, row, column, 0, text)
            .expect("set cell text");
    }
    pres.format_cell_text(
        0,
        table,
        Cells::row(0),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold the header");
    pres.format_cell_paragraphs(
        0,
        table,
        Cells::rectangle(1..3, 1..3),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
    )
    .expect("align the numbers");
    pres.set_row_height(0, table, 0, Emu::from_points(30.0))
        .expect("taller header row");
    pres.format_cells(
        0,
        table,
        Cells::row(0),
        &CellFormat::new()
            .with_fill(FillSpec::Solid(ColorSpec::Srgb("1F3864".to_owned())))
            .with_border(
                CellBorder::Bottom,
                LineSpec {
                    width: Some(LineWidth::from_emu(19_050)),
                    fill: Some(FillSpec::Solid(ColorSpec::Srgb("FFFFFF".to_owned()))),
                    ..LineSpec::default()
                },
            )
            .with_anchor(TextAnchoring::Center),
    )
    .expect("style the header row");
    pres.format_cells(
        0,
        table,
        Cells::all(),
        &CellFormat::new().with_margins(CellMargins::uniform(Emu::from_points(6.0))),
    )
    .expect("roomier insets");
    pres.merge_cells(0, table, Cells::rectangle(2..3, 1..3))
        .expect("merge the totals");
    pres.set_cell_text(0, table, 2, 1, 0, "984 (-3%)")
        .expect("the merged cell's text");
    pres.insert_column(0, table, 1).expect("insert a column");
    pres.insert_row(0, table, 3).expect("append a row");
    pres.remove_column(0, table, 1).expect("remove the column");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("created table", &saved);
}

#[test]
fn a_created_table_style_is_schema_valid() {
    // `build::table_styles_bytes` plus everything `mjx-dml` appends to it — a brand-new part with a
    // content-type override and a relationship off the presentation.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.5, 8.0, 3.0))
        .expect("add table");
    pres.set_cell_text(0, table, 0, 0, 0, "Region")
        .expect("cell text");
    pres.set_table_part(0, table, TablePart::FirstRow, true)
        .expect("header flag");
    pres.set_table_part(0, table, TablePart::BandedRows, true)
        .expect("banding flag");

    let style_id = "{9A8B7C6D-5E4F-4A3B-8C2D-1E0F9A8B7C6D}";
    pres.create_table_style(style_id, "Report Style")
        .expect("create style");
    pres.format_table_style_part(
        style_id,
        TableStylePart::WholeTable,
        &TableStyleFormat::new()
            .with_border(TableStyleBorder::InsideHorizontal, LineSpec::default()),
    )
    .expect("whole-table borders");
    pres.format_table_style_part(
        style_id,
        TableStylePart::FirstRow,
        &TableStyleFormat::new()
            .with_bold(OnOffStyle::On)
            .with_text_color(ColorSpec::Srgb("FFFFFF".to_owned()))
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".to_owned())))
            .with_cell_material(PresetMaterial::Metal)
            .with_cell_bevel(Bevel {
                width: Some(Emu::from_emu(76_200)),
                height: Some(Emu::from_emu(38_100)),
                preset: Some(BevelPreset::Circle),
            })
            .with_cell_light_rig(LightRig {
                rig: LightRigType::ThreePoint,
                direction: LightRigDirection::Top,
                rotation: None,
            }),
    )
    .expect("header style");
    pres.format_table_style_part(
        style_id,
        TableStylePart::Band1Horizontal,
        &TableStyleFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("D9E1F2".to_owned()))),
    )
    .expect("banded style");
    pres.set_table_style(0, table, style_id)
        .expect("assign style");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("created table style", &saved);
}

/// Every chart kind this library can author. `Stock` is absent: `CT_StockChart` requires three or
/// four series, so it is exercised by its own case rather than with the shared two-series data.
const AUTHORED_CHART_KINDS: [ChartKind; 15] = [
    ChartKind::Bar,
    ChartKind::Bar3D,
    ChartKind::Line,
    ChartKind::Line3D,
    ChartKind::Pie,
    ChartKind::Pie3D,
    ChartKind::OfPie,
    ChartKind::Area,
    ChartKind::Area3D,
    ChartKind::Scatter,
    ChartKind::Doughnut,
    ChartKind::Radar,
    ChartKind::Bubble,
    ChartKind::Surface,
    ChartKind::Surface3D,
];

#[test]
fn authored_charts_are_schema_valid() {
    // `mjx-chart`'s authoring path for every chart kind, in one deck — and with it every embedded
    // workbook, whose SpreadsheetML the harness now validates against `sml.xsd` rather than skipping
    // as a binary blob. This is also the case that proves we never emit the negative `c:axId` that
    // charts.pptx (python-pptx's template) carries: no tolerance applies to an authored deck, so a
    // signed axis id here would fail.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    for (i, kind) in AUTHORED_CHART_KINDS.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let offset = i as f64;
        let chart = ChartData::new(kind)
            .categories(["Q1", "Q2", "Q3"])
            .series("Revenue", [1.0 + offset, 2.5, 3.25])
            .series("Cost", [0.5, 1.5, 2.0]);
        pres.add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0))
            .expect("add chart");
    }
    let stock = ChartData::new(ChartKind::Stock)
        .categories(["Mon", "Tue", "Wed"])
        .series("High", [12.0, 13.0, 11.5])
        .series("Low", [9.0, 9.5, 8.75])
        .series("Close", [11.0, 10.5, 10.0]);
    pres.add_chart(0, &stock, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0))
        .expect("add a stock chart");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored charts (every kind)", &saved);
}

#[test]
fn an_authored_chart_with_a_title_and_a_legend_is_schema_valid() {
    // The title carries DrawingML rich text inside the chart namespace, and the legend is a whole
    // element `CT_Chart` admits at exactly one position — both are markup this library now writes.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Line)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [1.0, 2.0, 3.0])
        .title("Revenue by quarter")
        .legend(LegendPosition::Bottom);
    pres.add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored chart with a title and a legend", &saved);
}

#[test]
fn an_authored_chart_that_labels_itself_is_schema_valid() {
    // `CT_DLbls` is sequence-dense — fifteen ranked children, with `c:delete` and the settings group
    // sharing one `xsd:choice` — and every kind that admits it puts it at a different rank. This
    // authors one on every kind that declares it.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    for kind in AUTHORED_CHART_KINDS
        .into_iter()
        .filter(|kind| kind.admits_plot_child("dLbls"))
    {
        let chart = ChartData::new(kind)
            .categories(["Q1", "Q2", "Q3"])
            .series("Revenue", [1.0, 2.5, 3.25])
            .series("Cost", [0.5, 1.5, 2.0])
            .data_labels(
                DataLabelSpec::new()
                    .value(true)
                    .category_name(true)
                    .series_name(false)
                    .percentage(true)
                    .bubble_size(false)
                    .legend_key(true)
                    .leader_lines(true)
                    .position(DataLabelPosition::Center)
                    .separator("; ")
                    .number_format("#,##0.0"),
            );
        pres.add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0))
            .expect("add a labelled chart");
    }
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored charts that label themselves", &saved);
}

#[test]
fn every_chart_decoration_edit_is_schema_valid() {
    // The four decoration families, written into one chart in the **reverse** of the order
    // `CT_BarSer` declares them, so a writer that appended rather than placing would fail here.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [1.0, 2.0, 3.0])
        .series("Cost", [0.5, 1.5, 2.5]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");

    // Error bars first, then the trendline, then the labels, then the point formatting.
    pres.set_chart_error_bars(
        0,
        frame,
        0,
        &ErrorBarSpec::custom(ErrorBarType::Both, vec![0.1, 0.2, 0.3], vec![0.1, 0.1, 0.1])
            .direction(ErrorBarDirection::Y)
            .no_end_cap(true),
    )
    .expect("add error bars");
    pres.add_chart_trendline(
        0,
        frame,
        0,
        &TrendlineSpec::new(TrendlineKind::Polynomial)
            .name("Fit")
            .polynomial_order(3)
            .projection(2.0, 1.0)
            .intercept(0.5)
            .display(true, true),
    )
    .expect("add a trendline");
    pres.add_chart_trendline(
        0,
        frame,
        1,
        &TrendlineSpec::new(TrendlineKind::MovingAverage).moving_average_period(2),
    )
    .expect("add a moving average");
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Plot { plot_idx: 0 },
        &DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd)
            .number_format("0.0"),
    )
    .expect("label the plot");
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Series { series_idx: 0 },
        &DataLabelSpec::new()
            .category_name(true)
            .separator(" — ")
            .leader_lines(true),
    )
    .expect("label the series");
    // Point 2 before point 0, so the `c:dLbl` run has to be ordered rather than appended.
    for point in [2_u32, 0] {
        pres.set_chart_data_labels(
            0,
            frame,
            ChartLabelScope::Point {
                series_idx: 0,
                point_idx: point,
            },
            &DataLabelSpec::new().percentage(true),
        )
        .expect("label a point");
    }
    pres.suppress_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 1,
        },
    )
    .expect("silence a point");
    pres.suppress_chart_data_labels(0, frame, ChartLabelScope::Series { series_idx: 1 })
        .expect("silence a series");
    for point in [2_u32, 0] {
        pres.set_chart_point_fill(
            0,
            frame,
            0,
            point,
            &FillSpec::Solid(ColorSpec::Srgb("C00000".into())),
        )
        .expect("colour a point");
        pres.set_chart_point_line(
            0,
            frame,
            0,
            point,
            &LineSpec {
                width: Some(LineWidth::from_points(1.25)),
                ..LineSpec::default()
            },
        )
        .expect("outline a point");
    }
    pres.set_chart_point_explosion(0, frame, 0, 0, Some(15))
        .expect("explode a point");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("every chart decoration edit", &saved);
}

#[test]
fn a_pie_chart_decorated_where_its_schema_allows_is_schema_valid() {
    // `CT_PieSer` places `c:dPt` and `c:dLbls` at different ranks from `CT_BarSer` — after
    // `c:explosion` rather than after `c:pictureOptions` — which is exactly why the write surface is
    // bound to the owning plot's kind.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Pie)
        .categories(["A", "B", "C"])
        .series("Share", [3.0, 2.0, 1.0])
        .data_labels(DataLabelSpec::new().percentage(true).leader_lines(true));
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add a pie chart");
    pres.set_chart_point_explosion(0, frame, 0, 1, Some(25))
        .expect("explode a slice");
    pres.set_chart_point_fill(
        0,
        frame,
        0,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("2E75B6".into())),
    )
    .expect("colour a slice");
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 2,
        },
        &DataLabelSpec::new().category_name(true).value(true),
    )
    .expect("label a slice");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("a decorated pie chart", &saved);
}

#[test]
fn a_scatter_chart_with_two_sets_of_error_bars_is_schema_valid() {
    // `CT_ScatterSer` admits `c:errBars` twice — one per axis — where `CT_BarSer` admits one. The
    // write surface reads that from the generated table rather than assuming.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Scatter)
        .categories(["1", "2", "3"])
        .series("Sample", [2.0, 4.0, 8.0]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add a scatter chart");
    for direction in [ErrorBarDirection::X, ErrorBarDirection::Y] {
        pres.set_chart_error_bars(
            0,
            frame,
            0,
            &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::Percentage, 5.0)
                .direction(direction),
        )
        .expect("add error bars");
    }
    assert_eq!(
        pres.chart_error_bars(0, frame, 0).expect("read").len(),
        2,
        "a scatter series admits one set of error bars per axis"
    );
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("a scatter chart with x and y error bars", &saved);
}

#[test]
fn an_edited_chart_is_schema_valid() {
    // Editing an authored chart part in place — the series values and categories are rewritten
    // through the model, so the part is re-serialized rather than re-emitted verbatim, and the
    // embedded workbook is regenerated alongside it.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Revenue", [1.0, 2.0]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");
    pres.set_chart_series_values(0, frame, 0, &[9.5, 8.25])
        .expect("rewrite the values");
    pres.set_chart_series_categories(0, frame, 0, &["Spring", "Summer"])
        .expect("rewrite the categories");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("edited chart", &saved);
}

#[test]
fn an_edited_chart_axis_legend_title_and_series_style_are_schema_valid() {
    // Every setter this tier adds, on one chart: each inserts an element into a `CT_*` sequence, so
    // a child placed in the wrong position fails here rather than in PowerPoint.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [1.0, 2.0, 3.0])
        .series("Cost", [0.5, 1.5, 2.5]);
    let frame = pres
        .add_chart(0, &chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");

    pres.set_chart_title(0, frame, Some("Quarterly results"))
        .expect("set the title");
    pres.set_chart_legend(0, frame, Some(LegendPosition::Right))
        .expect("place the legend");
    pres.set_chart_axis_title(0, frame, 0, Some("Quarter"))
        .expect("title the category axis");
    pres.set_chart_axis_title(0, frame, 1, Some("Millions"))
        .expect("title the value axis");
    pres.set_chart_axis_scale(0, frame, 1, Some(0.0), Some(10.0))
        .expect("bound the value axis");
    pres.set_chart_axis_orientation(0, frame, 1, AxisOrientation::MaximumToMinimum)
        .expect("reverse the value axis");
    pres.set_chart_axis_gridlines(0, frame, 1, true, true)
        .expect("rule gridlines");
    pres.set_chart_series_fill(
        0,
        frame,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned())),
    )
    .expect("fill the first series");
    pres.set_chart_series_line(
        0,
        frame,
        1,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("ED7D31".to_owned()),
        ),
    )
    .expect("outline the second series");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid(
        "edited chart axes, legend, title and series style",
        &saved,
    );
}

#[test]
fn formatted_text_is_schema_valid() {
    // The text model at three scopes — shape-wide, paragraph-wide and one character range — plus the
    // paragraph properties (bullets, indents, spacing) that carry the most attributes.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Formatted title")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "A bulleted line of body text")
        .expect("set the body");
    pres.set_shape_run_properties(
        slide,
        0,
        &CharacterPropertiesSpec::new()
            .with_size_points(32.0)
            .with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("size the title");
    pres.set_paragraph_properties(
        slide,
        1,
        0,
        &ParagraphPropertiesSpec::new()
            .with_level(IndentLevel::of(1))
            .with_alignment(TextAlignment::Left)
            .with_left_margin_points(36.0)
            .with_indent_points(-18.0)
            .with_space_before(TextSpacing::points(6.0))
            .with_bullet_character("•"),
    )
    .expect("lay out the body");
    pres.set_text_range_properties(
        slide,
        1,
        0,
        2..10,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bold one word");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("formatted text", &saved);
}

#[test]
fn an_authored_shape_list_style_is_schema_valid() {
    // `CT_TextBody` is `bodyPr`, `lstStyle?`, `p+` and `CT_TextListStyle` is `defPPr` then
    // `lvl1pPr` … `lvl9pPr` — both sequences, so an authored list style is only valid if every
    // element lands in order. The levels are written *in* schema order (0, then 8, then the default
    // that precedes both), which is the order an implementation ignoring the sequence gets wrong.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 1, 0, "A body whose levels are stated by the shape")
        .expect("set the body");
    pres.set_shape_list_style_level(
        slide,
        1,
        IndentLevel::TOP,
        &ParagraphPropertiesSpec::new()
            .with_alignment(TextAlignment::Center)
            .with_space_before(TextSpacing::points(6.0))
            .with_default_run_properties(
                CharacterPropertiesSpec::new()
                    .with_size_points(20.0)
                    .with_color(ColorSpec::Scheme(SchemeColor::Accent2)),
            ),
    )
    .expect("author level 0");
    pres.set_shape_list_style_level(
        slide,
        1,
        IndentLevel::of(8),
        &ParagraphPropertiesSpec::new()
            .with_left_margin_points(72.0)
            .with_bullet_character("-"),
    )
    .expect("author the deepest level");
    pres.set_shape_list_style_default(
        slide,
        1,
        &ParagraphPropertiesSpec::new().with_indent_points(9.0),
    )
    .expect("author the default");

    // A text box authors its list style the same way, on a body this library wrote itself.
    let box_idx = pres
        .add_text_box(
            slide,
            "A text box",
            ShapeBounds::from_inches(1.0, 5.0, 3.0, 1.0),
        )
        .expect("add text box");
    pres.set_shape_list_style_level(
        slide,
        box_idx,
        IndentLevel::of(1),
        &ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Right),
    )
    .expect("author a level on the text box");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored shape list style", &saved);
}

#[test]
fn hyperlinks_are_schema_valid() {
    // `a:hlinkClick` on a run and on a shape, external and internal — both add a relationship and an
    // element whose attribute set the schema constrains tightly.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_slide().expect("a slide to jump to");
    let box_idx = pres
        .add_text_box(0, "Visit us", ShapeBounds::from_inches(1.0, 3.0, 4.0, 1.0))
        .expect("add text box");
    pres.set_run_hyperlink(
        0,
        box_idx,
        0,
        0,
        &Hyperlink::Url("https://example.invalid/".to_owned()),
    )
    .expect("run hyperlink");

    let shape = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 4.5, 3.0, 1.0),
        )
        .expect("add shape");
    pres.set_shape_hyperlink(0, shape, &Hyperlink::Slide(1))
        .expect("shape hyperlink");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("hyperlinks", &saved);
}

#[test]
fn transformed_shapes_are_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.set_shape_bounds(0, 0, ShapeBounds::from_inches(0.5, 0.3, 8.0, 1.0))
        .expect("place the title");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 2.0, 3.0, 1.0),
        )
        .expect("add shape");
    let mut transform = pres
        .shape_transform(0, idx)
        .expect("read transform")
        .unwrap_or_default();
    transform.rotation = Some(Angle::from_degrees(30.0));
    transform.flip_horizontal = Some(true);
    transform.flip_vertical = Some(true);
    pres.set_shape_transform(0, idx, &transform)
        .expect("rotate and mirror");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("transformed shapes", &saved);
}

#[test]
fn an_edited_layout_and_a_pruned_deck_are_schema_valid() {
    // Editing a layout (not a slide) and then removing shapes and a slide: the surfaces and the
    // pruning path, which rewrite parts other cases never touch.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    pres.set_shape_fill(
        Surface::Layout(1),
        0,
        &FillSpec::solid(ColorSpec::Srgb("C00000".into())),
    )
    .expect("fill the layout's title");

    let slide = pres.add_slide_from_layout(1).expect("add slide");
    pres.set_shape_text(slide, 0, 0, "Edited and pruned")
        .expect("set the title");
    let doomed = pres
        .add_text_box(
            slide,
            "removed again",
            ShapeBounds::from_inches(5.0, 5.0, 3.0, 1.0),
        )
        .expect("add text box");
    pres.remove_shape(slide, doomed).expect("remove the box");
    pres.remove_slide(0).expect("remove the first slide");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("edited layout, pruned deck", &saved);
}

#[test]
fn a_deck_built_from_every_authoring_path_is_schema_valid() {
    // One deck touched by everything at once. Individually valid parts can still combine into an
    // invalid one — a slide carrying a text box, a shape, a picture, a table and a chart exercises
    // `p:spTree`'s content model, not just each element's.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let slide = pres
        .add_slide_with_text("Everything", ShapeBounds::from_inches(0.5, 0.3, 8.0, 1.0))
        .expect("add slide with text");
    pres.set_notes_text(slide, "Notes for the everything slide")
        .expect("notes");
    pres.add_shape(
        slide,
        PresetShapeType::Ellipse,
        ShapeBounds::from_inches(0.5, 1.5, 2.0, 1.0),
    )
    .expect("shape");
    pres.add_picture(
        slide,
        TINY_PNG,
        ShapeBounds::from_inches(3.0, 1.5, 2.0, 1.0),
    )
    .expect("picture");
    let table = pres
        .add_table(slide, 2, 2, ShapeBounds::from_inches(0.5, 3.0, 4.0, 1.5))
        .expect("table");
    pres.set_cell_text(slide, table, 0, 0, 0, "Cell")
        .expect("cell text");
    let chart = ChartData::new(ChartKind::Line)
        .categories(["A", "B"])
        .series("Series", [1.0, 2.0]);
    pres.add_chart(slide, &chart, ShapeBounds::from_inches(5.0, 3.0, 4.0, 1.5))
        .expect("chart");
    pres.add_diagram(
        slide,
        &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
        ShapeBounds::from_inches(5.0, 1.5, 3.0, 1.5),
    )
    .expect("diagram");
    pres.add_ole_object(
        slide,
        &OleObjectSpec::embedded_stream("Excel.Sheet.12", &default_placeholder_ole(), TINY_PNG),
        ShapeBounds::from_inches(0.5, 4.6, 2.0, 1.0),
    )
    .expect("OLE object");
    pres.add_activex_control(
        slide,
        &ActiveXControlSpec::new(
            "CommandButton1",
            "{D7053240-CE69-11CD-A777-00DD01143C57}",
            b"state",
            TINY_PNG,
        ),
        ShapeBounds::from_inches(3.0, 4.6, 2.0, 0.5),
    )
    .expect("ActiveX control");
    pres.add_ink(slide, INK_STROKES).expect("ink");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("every authoring path in one deck", &saved);
}

// ---------------------------------------------------------------------------------------------
// The legacy surfaces this project now authors (MJX-140)
// ---------------------------------------------------------------------------------------------

/// A minimal but real InkML document, inlined so no binary fixture is committed.
const INK_STROKES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"#;

#[test]
fn an_authored_diagram_is_schema_valid() {
    // The four documents a SmartArt diagram is made of are markup this project writes, so each is
    // validated against `dml-diagram.xsd` — the arm added for exactly this.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_diagram(
        0,
        &DiagramContent::vertical_list(&["Plan", "Build", "Ship"]),
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
    )
    .expect("add diagram");
    // An empty diagram is the other end of the generator, and must be valid too.
    pres.add_diagram(
        0,
        &DiagramContent::vertical_list(&[]),
        ShapeBounds::from_inches(6.0, 1.0, 2.0, 2.0),
    )
    .expect("add empty diagram");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored diagram", &saved);
}

#[test]
fn the_diagram_parts_are_really_validated_and_not_skipped() {
    // A byte-identity or "no failures" assertion passes just as happily when every part was skipped.
    // This pins that the four diagram parts were *validated*, against the schema named.
    let Some(harness) = harness() else { return };
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_diagram(
        0,
        &DiagramContent::vertical_list(&["Plan"]),
        ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
    )
    .expect("add diagram");
    let saved = pres.save().expect("save");

    let rows = inspect_deck(&harness, "authored diagram", &saved, &[]);
    let mut validated: Vec<&str> = rows
        .iter()
        .filter(|row| {
            row.name.contains("/ppt/diagrams/")
                && matches!(row.outcome, PartOutcome::Validated("dml-diagram.xsd"))
        })
        .map(|row| row.name.as_str())
        .collect();
    validated.sort_unstable();
    assert_eq!(
        validated,
        vec![
            "/ppt/diagrams/colors1.xml",
            "/ppt/diagrams/data1.xml",
            "/ppt/diagrams/layout1.xml",
            "/ppt/diagrams/quickStyle1.xml",
        ],
        "all four diagram parts must be validated against dml-diagram.xsd"
    );
}

#[test]
fn a_diagram_part_dml_diagram_xsd_rejects_is_caught_naming_the_diagram_schema() {
    // The two tests above prove the diagram arm accepts what this library writes. Neither can
    // distinguish a live `xmllint` check from an arm that always reports success — the arm named in
    // MJXOFF-148's own "done when" clause. This one writes markup no writer here would ever
    // produce (`dgm:pt` with no `modelId`, a required attribute per `dml-diagram.xsd`'s `CT_Pt`) and
    // asserts the sweep reports it `Failed` against `dml-diagram.xsd`, not silently `Validated` or
    // skipped.
    let Some(harness) = harness() else { return };
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres
        .add_diagram(
            0,
            &DiagramContent::vertical_list(&["Plan"]),
            ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0),
        )
        .expect("add diagram");

    const MISSING_REQUIRED_MODEL_ID: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:ptLst><dgm:pt/></dgm:ptLst></dgm:dataModel>"#;
    pres.set_diagram_part(
        0,
        shape,
        DiagramPartKind::Data,
        MISSING_REQUIRED_MODEL_ID.to_vec(),
    )
    .expect("replace data part with invalid markup");
    let saved = pres.save().expect("save");

    let rows = inspect_deck(&harness, "diagram with an invalid data part", &saved, &[]);
    let row = rows
        .iter()
        .find(|row| row.name == "/ppt/diagrams/data1.xml")
        .expect("the data part is in the sweep");
    match &row.outcome {
        PartOutcome::Failed { schema, report } => {
            assert_eq!(*schema, "dml-diagram.xsd", "wrong schema named: {report}");
            assert!(
                report.contains("modelId"),
                "the report should name the missing required attribute: {report}"
            );
        }
        other => panic!(
            "a `dgm:pt` with no `modelId` must fail dml-diagram.xsd, not report {}",
            other.describe()
        ),
    }
}

#[test]
fn an_authored_ole_object_is_schema_valid() {
    // PowerPoint wraps its `p:oleObj` in `mc:AlternateContent` for the VML fallback; this project
    // writes the bare element `CT_OleObject` describes, which is why the slide is *validated* here
    // rather than skipped for markup compatibility.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let payload = default_placeholder_ole();
    pres.add_ole_object(
        0,
        &OleObjectSpec::embedded_stream("Excel.Sheet.12", &payload, TINY_PNG).named("Worksheet"),
        ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0),
    )
    .expect("add OLE object");
    pres.add_ole_object(
        0,
        &OleObjectSpec {
            prog_id: "Excel.Sheet.12",
            data: OleObjectData::Linked("file:///elsewhere/book.xlsx"),
            snapshot_image: TINY_PNG,
            name: None,
            show_as_icon: true,
        },
        ShapeBounds::from_inches(5.0, 1.0, 3.0, 2.0),
    )
    .expect("add linked OLE object");
    // A frame bound to its legacy VML fallback: `spid` is `a:ST_ShapeID`, so a bad value would fail
    // the gate rather than pass unnoticed.
    pres.set_ole_legacy_shape_id(0, 1, "_x0000_s1026")
        .expect("bind the fallback");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored OLE object", &saved);
}

#[test]
fn an_edited_ole_object_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("ole.pptx")).expect("open");
    pres.set_ole_prog_id(0, 1, "Word.Document.12")
        .expect("set progId");
    pres.set_ole_snapshot_image(0, 1, TINY_PNG)
        .expect("set snapshot");
    let saved = pres.save().expect("save");
    // The fixture's own slide carries `mc:AlternateContent`, so it is skipped for that reason and
    // not for anything this edit did; every other part of the deck is validated.
    assert_authored_deck_is_schema_valid("edited OLE object", &saved);
}

#[test]
fn an_authored_activex_control_is_schema_valid() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_activex_control(
        0,
        &ActiveXControlSpec::new(
            "CommandButton1",
            "{D7053240-CE69-11CD-A777-00DD01143C57}",
            b"persisted state",
            TINY_PNG,
        ),
        ShapeBounds::from_inches(1.0, 1.0, 2.0, 0.5),
    )
    .expect("add control");
    pres.add_activex_control(
        0,
        &ActiveXControlSpec {
            name: "Label1",
            class_id: "{978C9E23-D4B0-11CE-BF2D-00AA003F40D0}",
            persistence: ActiveXPersistence::PropertyBag,
            state: None,
            snapshot_image: TINY_PNG,
        },
        ShapeBounds::from_inches(4.0, 1.0, 2.0, 0.5),
    )
    .expect("add stateless control");
    pres.set_activex_control_name(0, 0, "OkButton")
        .expect("rename");
    pres.set_activex_control_shape_id(0, 0, "_x0000_s1026")
        .expect("bind the fallback");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored ActiveX control", &saved);
}

#[test]
fn authored_ink_is_schema_valid() {
    // `p:contentPart` is `CT_Rel` in `pml.xsd`, so an authored ink reference is validated markup —
    // unlike the `p14:contentPart` inside `mc:AlternateContent` that producers write.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let shape = pres.add_ink(0, INK_STROKES).expect("add ink");
    pres.set_ink_content(0, shape, INK_STROKES)
        .expect("edit ink");
    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored ink", &saved);
}

#[test]
#[cfg(feature = "vml")]
fn an_authored_vml_drawing_is_schema_valid() {
    // The VML part itself is Transitional-only markup outside the base schema set and is reported
    // skipped-as-foreign; what must still hold is that the *deck* around it — the content types with
    // their new `vml` Default, and the slide's relationships — validates.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let mut drawing = mjx_vml::DrawingPart::new();
    {
        let (model, interner) = drawing.drawing_and_interner();
        model.push(mjx_vml::DrawingContent::ShapeLayout(
            mjx_vml::ShapeLayout::new(interner, "1"),
        ));
        model.push(mjx_vml::DrawingContent::Shape(mjx_vml::Shape::new(
            interner,
            "_x0000_s1026",
            "position:absolute;width:100pt;height:50pt",
        )));
    }
    pres.add_vml_drawing(0, &drawing.to_bytes())
        .expect("add VML drawing");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_schema_valid("authored VML drawing", &saved);
}
