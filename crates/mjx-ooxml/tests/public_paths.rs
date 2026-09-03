//! One deck driven through **every subject of the `Deck` surface**, importing only `mjx_ooxml::…`.
//!
//! `Deck` is split into sixteen modules — document addressing, slides, shapes, text, hyperlinks,
//! appearance, bounds, effective readers, tables, cells, charts, chart decoration, pictures, notes,
//! legacy content, package hygiene. The split is only correct if none of it is visible from outside:
//! every method must be an inherent method on the one re-exported `Deck`, reachable by the path a
//! caller writes.
//!
//! It is also the proof that the **re-export list is sufficient**. This file names no crate below
//! `mjx-ooxml` — not `mjx-dml` for a fill, not `mjx-chart` for a chart description, not
//! `mjx-ooxml-types` for a preset shape, not `mjx-pptx` for anything at all. If a caller had to reach
//! past the facade to state an argument, this file would not compile, and the facade's central claim
//! would be false.
//!
//! Its sibling `delegate_wiring.rs` proves each delegate calls the *right* method; this one proves
//! each is *reachable*.

use mjx_ooxml::{
    ActiveXControlSpec, ActiveXPersistence, Camera, CellBorder, CellFormat, CellMargins, Cells,
    CharacterPropertiesSpec, ChartData, ChartKind, ChartLabelScope, ColorSchemeSlot, ColorSpec,
    DataLabelPosition, DataLabelSpec, Deck, DiagramContent, DiagramPartKind, EffectListSpec, Emu,
    ErrorBarSpec, ErrorBarType, ErrorCode, ErrorValueType, FillSpec, Fraction, Geometry,
    GlowEffect, GuideContext, Hyperlink, IndentLevel, LightRig, LightRigDirection, LightRigType,
    LineSpec, LineWidth, OleObjectData, OleObjectSpec, ParagraphPropertiesSpec, PresetCamera,
    PresetShapeType, Scene3DSpec, Shape3DSpec, ShapeBounds, ShapeGeometry, ShapeKind, ShapePath,
    SlideSize, Surface, TablePart, TextAnchoring, TextDirection, TrendlineKind, TrendlineSpec,
    DEFAULT_PLACEHOLDER_IMAGE,
};

const INKML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"#;

const COMMAND_BUTTON: &str = "{D7053240-CE69-11CD-A777-00DD01143C57}";

fn bounds(x: i64, y: i64, w: i64, h: i64) -> ShapeBounds {
    ShapeBounds::new(x, y, w, h)
}

fn navy() -> ColorSpec {
    ColorSpec::Srgb("1F3864".into())
}

/// Every subject the surface is split into, exercised on one authored deck through `mjx_ooxml::`.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one deck driven through sixteen subjects; splitting it would stop proving they share one Deck"
)]
fn every_subject_of_the_facade_is_reachable_on_the_re_exported_deck() {
    // --- lifecycle ------------------------------------------------------------------------------
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");

    // --- document addressing --------------------------------------------------------------------
    assert_eq!(deck.slide_count(), 0, "a blank deck has no slides");
    assert_eq!(deck.master_count(), 1);
    assert_eq!(deck.layout_count(), 1);
    assert_eq!(deck.layout_master(0), Some(0));
    assert_eq!(deck.slide_size().expect("a size").width_emu, 12_192_000);
    assert!(deck.master_name(0).expect("a master name").is_some());
    assert!(deck.layout_name(0).expect("a layout name").is_some());
    let _ = deck.layout_kind(0).expect("a layout kind");
    assert_eq!(deck.layouts().expect("the layouts").len(), 1);
    let theme = deck
        .theme(Surface::Master(0))
        .expect("reading the theme")
        .expect("a theme");
    assert!(theme.color(ColorSchemeSlot::Accent1).is_some());
    assert!(deck
        .color_map(Surface::Master(0))
        .expect("reading the colour map")
        .is_some());

    // --- slides -----------------------------------------------------------------------------------
    let slide = Surface::Slide(deck.add_slide_from_layout(0).expect("a slide"));
    assert_eq!(deck.slide_count(), 1);
    assert_eq!(deck.slide_layout(0).expect("a layout"), Some(0));
    let doomed = deck.add_slide().expect("a second slide");
    deck.remove_slide(doomed).expect("removing it again");
    assert_eq!(deck.slide_count(), 1);

    // --- shapes -----------------------------------------------------------------------------------
    let inherited_shapes = deck.shape_count(slide).expect("a count");
    let box_a: ShapePath = deck
        .add_text_box(slide, "Alpha", bounds(0, 0, 900_000, 400_000))
        .expect("a text box")
        .into();
    let box_b: ShapePath = deck
        .add_shape(
            slide,
            PresetShapeType::Ellipse,
            bounds(1_000_000, 0, 400_000, 400_000),
        )
        .expect("an autoshape")
        .into();
    assert_eq!(
        deck.shape_kind(slide, box_a.clone()).expect("a kind"),
        ShapeKind::Shape
    );
    assert_eq!(
        deck.shapes(slide).expect("the shapes").len(),
        inherited_shapes as usize + 2
    );
    assert!(deck
        .shape_placeholder(slide, box_a.clone())
        .expect("reading")
        .is_none());
    assert!(deck
        .graphic_frame_kind(slide, box_a.clone())
        .expect("reading")
        .is_none());
    let _ = deck
        .shape_for_placeholder(slide, mjx_ooxml::PlaceholderType::Title)
        .expect("reading");

    // --- groups -----------------------------------------------------------------------------------
    let group = deck.group_shapes(slide, &[box_a, box_b]).expect("a group");
    assert_eq!(
        deck.shape_member_count(slide, group.clone())
            .expect("a count"),
        2
    );
    let member = group.child(0);
    let promoted = deck
        .move_shape_out_of_group(slide, member)
        .expect("promoting a member");
    let regrouped = deck
        .move_shape_into_group(slide, promoted, group.clone())
        .expect("demoting it again");
    assert_eq!(regrouped.depth(), 2);
    let loose = deck.ungroup(slide, group).expect("ungrouping");
    assert_eq!(loose.len(), 2);
    let shape = loose[0].clone();

    // --- text -------------------------------------------------------------------------------------
    deck.set_shape_text_content(slide, shape.clone(), "Alpha\nBeta")
        .expect("text");
    assert_eq!(
        deck.paragraph_count(slide, shape.clone()).expect("a count"),
        2
    );
    assert_eq!(deck.run_count(slide, shape.clone(), 0).expect("a count"), 1);
    assert_eq!(
        deck.paragraph_text(slide, shape.clone(), 1).expect("text"),
        "Beta"
    );
    assert_eq!(
        deck.run_text(slide, shape.clone(), 0, 0).expect("text"),
        "Alpha"
    );
    assert_eq!(
        deck.shape_text(slide, shape.clone()).expect("text"),
        "Alpha\nBeta"
    );
    deck.set_shape_text(slide, shape.clone(), 0, "Gamma")
        .expect("a run edit");
    assert_eq!(
        deck.run_text(slide, shape.clone(), 0, 0).expect("text"),
        "Gamma"
    );
    assert_eq!(
        deck.paragraph_field_count(slide, shape.clone(), 0)
            .expect("a count"),
        0
    );
    deck.set_run_properties(
        slide,
        shape.clone(),
        0,
        0,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("run properties");
    deck.set_paragraph_run_properties(
        slide,
        shape.clone(),
        1,
        &CharacterPropertiesSpec::new().with_italic(true),
    )
    .expect("paragraph run properties");
    deck.set_shape_run_properties(
        slide,
        shape.clone(),
        &CharacterPropertiesSpec::new().with_size_points(18.0),
    )
    .expect("shape run properties");
    deck.set_end_run_properties(
        slide,
        shape.clone(),
        0,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("end run properties");
    deck.set_paragraph_properties(
        slide,
        shape.clone(),
        0,
        &ParagraphPropertiesSpec::new().with_level(IndentLevel::new(1).expect("level 1")),
    )
    .expect("paragraph properties");
    deck.set_text_range_properties(
        slide,
        shape.clone(),
        0,
        0..3,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("a text range");
    deck.set_text_range_properties_by_grapheme(
        slide,
        shape.clone(),
        0,
        0..2,
        &CharacterPropertiesSpec::new().with_italic(true),
    )
    .expect("a grapheme range");
    assert!(deck
        .run_properties(slide, shape.clone(), 0, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .paragraph_properties(slide, shape.clone(), 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .end_run_properties(slide, shape.clone(), 0)
        .expect("reading")
        .is_some());
    let _ = deck
        .coalesce_paragraph_runs(slide, shape.clone(), 0)
        .expect("coalescing");
    let _ = deck
        .coalesce_shape_runs(slide, shape.clone())
        .expect("coalescing");

    // --- list styles ------------------------------------------------------------------------------
    let level = IndentLevel::new(0).expect("level 0");
    deck.set_shape_list_style_level(
        slide,
        shape.clone(),
        level,
        &ParagraphPropertiesSpec::new().with_alignment(mjx_ooxml::TextAlignment::Center),
    )
    .expect("a list style level");
    deck.set_shape_list_style_default(
        slide,
        shape.clone(),
        &ParagraphPropertiesSpec::new().with_alignment(mjx_ooxml::TextAlignment::Left),
    )
    .expect("a list style default");
    assert!(deck
        .shape_list_style_level(slide, shape.clone(), level)
        .expect("reading")
        .is_some());
    assert!(deck
        .shape_list_style_default(slide, shape.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .clear_shape_list_style_level(slide, shape.clone(), level)
        .expect("clearing"));
    assert!(deck
        .clear_shape_list_style_default(slide, shape.clone())
        .expect("clearing"));
    let _ = deck
        .clear_shape_list_style(slide, shape.clone())
        .expect("clearing");

    // --- hyperlinks -------------------------------------------------------------------------------
    let link = Hyperlink::Url("https://example.invalid/".to_owned());
    deck.set_run_hyperlink(slide, shape.clone(), 0, 0, &link)
        .expect("a run hyperlink");
    assert!(deck
        .run_hyperlink(slide, shape.clone(), 0, 0)
        .expect("reading")
        .is_some());
    deck.clear_run_hyperlink(slide, shape.clone(), 0, 0)
        .expect("clearing");
    deck.set_text_range_hyperlink(slide, shape.clone(), 0, 0..2, &link)
        .expect("a range hyperlink");
    deck.set_shape_hyperlink(slide, shape.clone(), &link)
        .expect("a shape hyperlink");
    assert!(deck
        .shape_hyperlink(slide, shape.clone())
        .expect("reading")
        .is_some());
    deck.clear_shape_hyperlink(slide, shape.clone())
        .expect("clearing");

    // --- appearance -------------------------------------------------------------------------------
    deck.set_shape_fill(slide, shape.clone(), &FillSpec::solid(navy()))
        .expect("a fill");
    deck.set_shape_outline(
        slide,
        shape.clone(),
        &LineSpec::solid(LineWidth::from_points(2.0), navy()),
    )
    .expect("an outline");
    deck.set_shape_effects(
        slide,
        shape.clone(),
        &EffectListSpec {
            glow: Some(GlowEffect::new(navy())),
            ..EffectListSpec::new()
        },
    )
    .expect("effects");
    deck.set_shape_scene_3d(
        slide,
        shape.clone(),
        &Scene3DSpec {
            camera: Camera {
                preset: PresetCamera::OrthographicFront,
                field_of_view: None,
                zoom: None,
                rotation: None,
            },
            light_rig: LightRig {
                rig: LightRigType::ThreePoint,
                direction: LightRigDirection::Top,
                rotation: None,
            },
        },
    )
    .expect("a 3-D scene");
    deck.set_shape_3d_properties(slide, shape.clone(), &Shape3DSpec::new())
        .expect("3-D properties");
    assert!(deck
        .shape_fill(slide, shape.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .shape_outline(slide, shape.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .shape_effects(slide, shape.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .shape_scene_3d(slide, shape.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .shape_3d_properties(slide, shape.clone())
        .expect("reading")
        .is_some());
    deck.clear_shape_scene_3d(slide, shape.clone())
        .expect("clearing");
    deck.clear_shape_3d_properties(slide, shape.clone())
        .expect("clearing");
    deck.set_shape_no_effects(slide, shape.clone())
        .expect("no effects");
    deck.set_shape_no_outline(slide, shape.clone())
        .expect("no outline");

    // --- bounds, transform and geometry -----------------------------------------------------------
    deck.set_shape_bounds(slide, shape.clone(), bounds(10, 20, 30, 40))
        .expect("bounds");
    let read = deck
        .shape_bounds(slide, shape.clone())
        .expect("reading")
        .expect("stated bounds");
    assert_eq!((read.offset_x_emu, read.height_emu), (10, 40));
    let transform = deck
        .shape_transform(slide, shape.clone())
        .expect("reading")
        .expect("a transform");
    deck.set_shape_transform(slide, shape.clone(), &transform)
        .expect("a transform");
    deck.set_shape_geometry(
        slide,
        shape.clone(),
        Geometry::Preset(ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.1),
        }),
    )
    .expect("a geometry");
    assert_eq!(
        deck.shape_geometry(slide, shape.clone()).expect("reading"),
        Geometry::Preset(ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.1),
        })
    );
    let _ = deck
        .shape_adjustments(
            slide,
            shape.clone(),
            GuideContext::from_extents(Emu::from_emu(30), Emu::from_emu(40)),
        )
        .expect("the adjustments");

    // --- effective readers -------------------------------------------------------------------------
    let _ = deck
        .effective_shape_fill(slide, shape.clone())
        .expect("an effective fill");
    let _ = deck
        .effective_shape_outline(slide, shape.clone())
        .expect("an effective outline");
    let _ = deck
        .effective_shape_effects(slide, shape.clone())
        .expect("effective effects");
    let _ = deck
        .effective_shape_transform(slide, shape.clone())
        .expect("an effective transform");
    let _ = deck
        .effective_shape_bounds(slide, shape.clone())
        .expect("effective bounds");
    let _ = deck
        .effective_run_properties(slide, shape.clone(), 0, 0)
        .expect("effective run properties");
    let _ = deck
        .effective_paragraph_properties(slide, shape.clone(), 0)
        .expect("effective paragraph properties");

    // --- notes -------------------------------------------------------------------------------------
    deck.set_notes_text(0, "Say the number first.")
        .expect("notes");
    assert_eq!(
        deck.notes_text(0).expect("reading").expect("some notes"),
        "Say the number first."
    );
    deck.clear_notes(0).expect("clearing the notes");
    assert!(deck
        .notes_text(0)
        .expect("reading")
        .is_none_or(|text| text.is_empty()));

    // --- tables ------------------------------------------------------------------------------------
    let table: ShapePath = deck
        .add_table(slide, 2, 2, bounds(0, 2_000_000, 6_000_000, 1_000_000))
        .expect("a table")
        .into();
    assert_eq!(
        deck.table_dimensions(slide, table.clone())
            .expect("dimensions"),
        (2, 2)
    );
    deck.set_column_width(slide, table.clone(), 0, Emu::from_emu(3_000_000))
        .expect("a width");
    deck.set_row_height(slide, table.clone(), 0, Emu::from_emu(500_000))
        .expect("a height");
    assert!(deck
        .column_width(slide, table.clone(), 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .row_height(slide, table.clone(), 0)
        .expect("reading")
        .is_some());
    deck.set_table_part(slide, table.clone(), TablePart::FirstRow, true)
        .expect("a banding flag");
    assert_eq!(
        deck.table_part(slide, table.clone(), TablePart::FirstRow)
            .expect("reading"),
        Some(true)
    );
    assert!(deck
        .table_style_id(slide, table.clone())
        .expect("reading")
        .is_none());
    assert_eq!(
        deck.merged_cell_anchor(slide, table.clone(), 0, 0)
            .expect("an anchor"),
        (0, 0)
    );

    // --- cells -------------------------------------------------------------------------------------
    deck.set_cell_text(slide, table.clone(), 0, 0, 0, "Region")
        .expect("cell text");
    assert_eq!(
        deck.cell_text(slide, table.clone(), 0, 0).expect("text"),
        "Region"
    );
    assert_eq!(
        deck.visible_cell_text(slide, table.clone(), 0, 0)
            .expect("text"),
        "Region"
    );
    assert_eq!(
        deck.cell_paragraph_count(slide, table.clone(), 0, 0)
            .expect("a count"),
        1
    );
    assert_eq!(
        deck.cell_run_count(slide, table.clone(), 0, 0, 0)
            .expect("a count"),
        1
    );
    assert_eq!(
        deck.cell_paragraph_text(slide, table.clone(), 0, 0, 0)
            .expect("text"),
        "Region"
    );
    assert_eq!(
        deck.cell_run_text(slide, table.clone(), 0, 0, 0, 0)
            .expect("text"),
        "Region"
    );
    deck.set_cell_fill(slide, table.clone(), 0, 0, &FillSpec::solid(navy()))
        .expect("a cell fill");
    deck.set_cell_border(
        slide,
        table.clone(),
        0,
        0,
        CellBorder::Bottom,
        &LineSpec::solid(LineWidth::from_points(1.0), navy()),
    )
    .expect("a cell border");
    deck.set_cell_margins(
        slide,
        table.clone(),
        0,
        0,
        CellMargins::uniform(Emu::from_emu(45_720)),
    )
    .expect("cell margins");
    deck.set_cell_anchor(slide, table.clone(), 0, 0, TextAnchoring::Center)
        .expect("a cell anchor");
    deck.set_cell_text_direction(slide, table.clone(), 0, 0, TextDirection::Horizontal)
        .expect("a cell text direction");
    deck.set_cell_headers(slide, table.clone(), 1, 0, &["h1"])
        .expect("cell headers");
    deck.set_cell_run_properties(
        slide,
        table.clone(),
        0,
        0,
        0,
        0,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("cell run properties");
    deck.set_cell_paragraph_run_properties(
        slide,
        table.clone(),
        0,
        0,
        0,
        &CharacterPropertiesSpec::new().with_italic(true),
    )
    .expect("cell paragraph run properties");
    deck.set_cell_run_properties_all(
        slide,
        table.clone(),
        0,
        0,
        &CharacterPropertiesSpec::new().with_size_points(11.0),
    )
    .expect("all cell run properties");
    deck.set_cell_end_run_properties(
        slide,
        table.clone(),
        0,
        0,
        0,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("cell end run properties");
    deck.set_cell_paragraph_properties(
        slide,
        table.clone(),
        0,
        0,
        0,
        &ParagraphPropertiesSpec::new().with_alignment(mjx_ooxml::TextAlignment::Center),
    )
    .expect("cell paragraph properties");
    deck.set_cell_text_range_properties(
        slide,
        table.clone(),
        0,
        0,
        0,
        0..3,
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("a cell text range");
    assert!(deck
        .cell_fill(slide, table.clone(), 0, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .cell_border(slide, table.clone(), 0, 0, CellBorder::Bottom)
        .expect("reading")
        .is_some());
    assert!(deck
        .cell_anchor(slide, table.clone(), 0, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .cell_text_direction(slide, table.clone(), 0, 0)
        .expect("reading")
        .is_some());
    assert_eq!(
        deck.cell_headers(slide, table.clone(), 1, 0)
            .expect("reading"),
        vec!["h1".to_owned()]
    );
    let _ = deck
        .cell_margins(slide, table.clone(), 0, 0)
        .expect("reading");
    assert!(deck
        .cell_paragraph_properties(slide, table.clone(), 0, 0, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .cell_run_properties(slide, table.clone(), 0, 0, 0, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .cell_end_run_properties(slide, table.clone(), 0, 0, 0)
        .expect("reading")
        .is_some());
    let _ = deck
        .effective_cell_fill(slide, table.clone(), 0, 0)
        .expect("an effective cell fill");
    let _ = deck
        .effective_cell_border(slide, table.clone(), 0, 0, CellBorder::Bottom)
        .expect("an effective cell border");
    let _ = deck
        .effective_cell_run_properties(slide, table.clone(), 0, 0, 0, 0)
        .expect("effective cell run properties");
    deck.format_cells(
        slide,
        table.clone(),
        Cells::row(0),
        &CellFormat::new().with_fill(FillSpec::solid(navy())),
    )
    .expect("bulk cell formatting");
    deck.format_cell_text(
        slide,
        table.clone(),
        Cells::all(),
        &CharacterPropertiesSpec::new().with_bold(true),
    )
    .expect("bulk cell text");
    deck.format_cell_paragraphs(
        slide,
        table.clone(),
        Cells::column(0),
        &ParagraphPropertiesSpec::new().with_alignment(mjx_ooxml::TextAlignment::Right),
    )
    .expect("bulk cell paragraphs");
    deck.merge_cells(slide, table.clone(), Cells::row(1))
        .expect("a merge");
    assert_eq!(
        deck.cell_span(slide, table.clone(), 1, 0).expect("a span"),
        (1, 2)
    );
    deck.unmerge_cells(slide, table.clone(), 1, 0)
        .expect("unmerging");
    deck.clear_cell_fill(slide, table.clone(), 0, 0)
        .expect("clearing the fill");
    deck.clear_cell_border(slide, table.clone(), 0, 0, CellBorder::Bottom)
        .expect("clearing the border");
    deck.insert_row(slide, table.clone(), 0).expect("a row");
    deck.insert_column(slide, table.clone(), 0)
        .expect("a column");
    deck.remove_row(slide, table.clone(), 0).expect("a row");
    deck.remove_column(slide, table, 0).expect("a column");

    // --- pictures and media --------------------------------------------------------------------------
    let picture: ShapePath = deck
        .add_picture(
            slide,
            DEFAULT_PLACEHOLDER_IMAGE,
            bounds(0, 3_500_000, 900_000, 900_000),
        )
        .expect("a picture")
        .into();
    assert!(deck
        .picture_image_bytes(slide, picture.clone())
        .expect("reading")
        .is_some());
    assert!(deck
        .picture_image_link_target(slide, picture.clone())
        .expect("reading")
        .is_none());
    deck.set_picture_image(slide, picture, DEFAULT_PLACEHOLDER_IMAGE)
        .expect("replacing the image");
    let _ = deck
        .add_image(slide, DEFAULT_PLACEHOLDER_IMAGE)
        .expect("an image part");
    assert!(deck.media_references(slide).expect("reading").is_empty());
    assert!(deck.linked_images(slide).expect("reading").is_empty());

    // --- charts ---------------------------------------------------------------------------------------
    let chart: ShapePath = deck
        .add_chart(
            slide,
            &ChartData::new(ChartKind::Bar)
                .categories(["Q1", "Q2"])
                .series("2026", [1.0, 2.0]),
            bounds(6_500_000, 0, 4_000_000, 3_000_000),
        )
        .expect("a chart")
        .into();
    assert_eq!(
        deck.chart_kinds(slide, chart.clone()).expect("the kinds"),
        vec![ChartKind::Bar]
    );
    assert_eq!(
        deck.chart_series(slide, chart.clone()).expect("the series")[0].values,
        vec![1.0, 2.0]
    );
    deck.set_chart_series_values(slide, chart.clone(), 0, &[3.0, 4.0])
        .expect("new values");
    deck.set_chart_series_categories(slide, chart.clone(), 0, &["A", "B"])
        .expect("new categories");
    deck.set_chart_title(slide, chart.clone(), Some("Trend"))
        .expect("a title");
    assert_eq!(
        deck.chart_title(slide, chart.clone()).expect("reading"),
        Some("Trend".to_owned())
    );
    deck.set_chart_legend(
        slide,
        chart.clone(),
        Some(mjx_ooxml::LegendPosition::Bottom),
    )
    .expect("a legend");
    assert!(deck
        .chart_legend(slide, chart.clone())
        .expect("reading")
        .is_some());
    let axes = deck.chart_axes(slide, chart.clone()).expect("the axes");
    assert!(!axes.is_empty());
    deck.set_chart_axis_scale(slide, chart.clone(), 0, Some(0.0), Some(10.0))
        .expect("an axis scale");
    deck.set_chart_axis_orientation(
        slide,
        chart.clone(),
        0,
        mjx_ooxml::AxisOrientation::MinimumToMaximum,
    )
    .expect("an axis orientation");
    deck.set_chart_axis_title(slide, chart.clone(), 0, Some("Quarter"))
        .expect("an axis title");
    deck.set_chart_axis_gridlines(slide, chart.clone(), 0, true, false)
        .expect("gridlines");
    let _ = deck.chart_style_id(slide, chart.clone()).expect("reading");
    // A chart part that has been edited is held as a tree until it is written, so its raw bytes may
    // legitimately be absent; what matters here is that the reader is reachable and answers.
    let _ = deck
        .chart_part_bytes(slide, chart.clone())
        .expect("reading");
    let _ = deck.chart_workbooks(slide).expect("the workbooks");
    let _ = deck
        .refresh_chart_workbook(slide, chart.clone())
        .expect("refreshing");

    // --- chart decoration -------------------------------------------------------------------------------
    deck.set_chart_series_fill(slide, chart.clone(), 0, &FillSpec::solid(navy()))
        .expect("a series fill");
    deck.set_chart_series_line(
        slide,
        chart.clone(),
        0,
        &LineSpec::solid(LineWidth::from_points(1.0), navy()),
    )
    .expect("a series line");
    assert!(deck
        .chart_series_fill(slide, chart.clone(), 0)
        .expect("reading")
        .is_some());
    deck.set_chart_data_labels(
        slide,
        chart.clone(),
        ChartLabelScope::Series { series_idx: 0 },
        &DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd),
    )
    .expect("data labels");
    let _ = deck
        .chart_data_labels(slide, chart.clone(), 0, None)
        .expect("reading");
    assert!(deck
        .chart_data_label_tier(
            slide,
            chart.clone(),
            ChartLabelScope::Series { series_idx: 0 }
        )
        .expect("reading")
        .is_some());
    let _ = deck
        .chart_point_label_text(slide, chart.clone(), 0, 0)
        .expect("reading");
    deck.suppress_chart_data_labels(
        slide,
        chart.clone(),
        ChartLabelScope::Series { series_idx: 0 },
    )
    .expect("suppressing the labels");
    assert!(deck
        .remove_chart_data_labels(
            slide,
            chart.clone(),
            ChartLabelScope::Series { series_idx: 0 }
        )
        .expect("removing them"));
    deck.set_chart_point_fill(slide, chart.clone(), 0, 1, &FillSpec::solid(navy()))
        .expect("a point fill");
    deck.set_chart_point_line(
        slide,
        chart.clone(),
        0,
        1,
        &LineSpec::solid(LineWidth::from_points(1.0), navy()),
    )
    .expect("a point line");
    deck.set_chart_point_explosion(slide, chart.clone(), 0, 1, Some(10))
        .expect("a point explosion");
    assert!(!deck
        .chart_point_formats(slide, chart.clone(), 0)
        .expect("reading")
        .is_empty());
    assert!(deck
        .remove_chart_point_format(slide, chart.clone(), 0, 1)
        .expect("removing"));
    deck.add_chart_trendline(
        slide,
        chart.clone(),
        0,
        &TrendlineSpec::new(TrendlineKind::Linear),
    )
    .expect("a trendline");
    deck.set_chart_trendline(
        slide,
        chart.clone(),
        0,
        0,
        &TrendlineSpec::new(TrendlineKind::Linear),
    )
    .expect("editing the trendline");
    assert_eq!(
        deck.chart_trendlines(slide, chart.clone(), 0)
            .expect("reading")
            .len(),
        1
    );
    assert_eq!(
        deck.remove_chart_trendlines(slide, chart.clone(), 0)
            .expect("removing"),
        1
    );
    deck.set_chart_error_bars(
        slide,
        chart.clone(),
        0,
        &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
    )
    .expect("error bars");
    assert_eq!(
        deck.chart_error_bars(slide, chart.clone(), 0)
            .expect("reading")
            .len(),
        1
    );
    assert_eq!(
        deck.remove_chart_error_bars(slide, chart.clone(), 0)
            .expect("removing"),
        1
    );
    assert!(deck
        .chart_dangling_decoration(slide, chart.clone(), 0)
        .expect("reading")
        .is_empty());
    assert_eq!(
        deck.drop_chart_dangling_decoration(slide, chart.clone(), 0)
            .expect("dropping"),
        0
    );
    deck.detach_chart_workbook(slide, chart)
        .expect("detaching the workbook");

    // --- legacy content -----------------------------------------------------------------------------
    let ink = deck.add_ink(slide, INKML).expect("an ink shape");
    assert_eq!(deck.ink_references(slide).expect("reading").len(), 1);
    assert!(deck
        .ink_part_for_shape(slide, ink)
        .expect("reading")
        .is_some());
    let ink_part = deck.ink_part_names()[0].clone();
    assert!(deck.ink_part_bytes(&ink_part).is_some());
    assert_eq!(
        deck.shape_for_ink_part(slide, &ink_part).expect("reading"),
        Some(ink)
    );
    deck.set_ink_content(slide, ink, INKML).expect("new ink");

    let diagram: ShapePath = deck
        .add_diagram(
            slide,
            &DiagramContent::vertical_list(&["Plan", "Build"]),
            bounds(0, 5_000_000, 3_000_000, 2_000_000),
        )
        .expect("a diagram")
        .into();
    let parts = deck
        .diagram_parts(slide, diagram.clone())
        .expect("reading")
        .expect("the diagram parts");
    let data_part = parts.data.clone().expect("a data part");
    assert!(deck.diagram_part_bytes(&data_part).is_some());
    assert!(deck
        .diagram_relationship_ids(slide, diagram.clone())
        .expect("reading")
        .is_some());
    let data_bytes = deck.diagram_part_bytes(&data_part).expect("the bytes");
    deck.set_diagram_part(slide, diagram, DiagramPartKind::Data, data_bytes)
        .expect("replacing the data part");

    let ole: ShapePath = deck
        .add_ole_object(
            slide,
            &OleObjectSpec {
                prog_id: "Excel.Sheet.12",
                data: OleObjectData::Linked("file:///elsewhere/book.xlsx"),
                snapshot_image: DEFAULT_PLACEHOLDER_IMAGE,
                name: None,
                show_as_icon: true,
            },
            bounds(4_000_000, 5_000_000, 1_000_000, 1_000_000),
        )
        .expect("an OLE object")
        .into();
    assert_eq!(
        deck.ole_prog_id(slide, ole.clone()).expect("reading"),
        Some("Excel.Sheet.12".to_owned())
    );
    assert_eq!(deck.ole_objects(slide).expect("reading").len(), 1);
    deck.set_ole_prog_id(slide, ole.clone(), "Word.Document.12")
        .expect("a new prog id");
    deck.set_ole_legacy_shape_id(slide, ole.clone(), "_x0000_s1027")
        .expect("a legacy shape id");
    assert!(deck
        .ole_legacy_shape_id(slide, ole.clone())
        .expect("reading")
        .is_some());
    deck.set_ole_snapshot_image(slide, ole.clone(), DEFAULT_PLACEHOLDER_IMAGE)
        .expect("a snapshot");
    assert!(deck
        .ole_snapshot_image_bytes(slide, ole.clone())
        .expect("reading")
        .is_some());
    // This OLE object *links* its data, so there is no part inside the package to read — the
    // reference leaves it, which is exactly what makes it the external link the hygiene delegates
    // find below.
    assert_eq!(
        deck.ole_object_part_bytes(slide, ole)
            .expect_err("a linked object has no part here")
            .code(),
        ErrorCode::UnsupportedContent
    );

    deck.add_activex_control(
        slide,
        &ActiveXControlSpec {
            name: "Button1",
            class_id: COMMAND_BUTTON,
            persistence: ActiveXPersistence::Stream,
            state: Some(b"initial state"),
            snapshot_image: DEFAULT_PLACEHOLDER_IMAGE,
        },
        bounds(5_500_000, 5_000_000, 1_000_000, 500_000),
    )
    .expect("an ActiveX control");
    assert_eq!(deck.activex_control_count(slide).expect("a count"), 1);
    assert_eq!(
        deck.activex_control_name(slide, 0).expect("reading"),
        Some("Button1".to_owned())
    );
    assert_eq!(
        deck.activex_class_id(slide, 0).expect("reading"),
        Some(COMMAND_BUTTON.to_owned())
    );
    assert!(deck
        .activex_persistence(slide, 0)
        .expect("reading")
        .is_some());
    deck.set_activex_control_name(slide, 0, "Button2")
        .expect("a new name");
    deck.set_activex_control_shape_id(slide, 0, "_x0000_s1028")
        .expect("a shape id");
    assert!(deck
        .activex_control_shape_id(slide, 0)
        .expect("reading")
        .is_some());
    deck.set_activex_state(slide, 0, b"state").expect("state");
    assert!(deck
        .activex_state_bytes(slide, 0)
        .expect("reading")
        .is_some());
    deck.set_activex_snapshot_image(slide, 0, DEFAULT_PLACEHOLDER_IMAGE)
        .expect("a snapshot");
    assert!(deck
        .activex_snapshot_image_bytes(slide, 0)
        .expect("reading")
        .is_some());
    assert!(deck
        .activex_part_bytes(slide, 0)
        .expect("reading")
        .is_some());
    deck.remove_activex_control(slide, 0).expect("removing it");

    // --- package hygiene ------------------------------------------------------------------------------
    assert!(!deck.external_links().is_empty(), "the linked OLE object");
    let link = deck.external_links()[0].clone();
    assert!(deck
        .retarget_external_link(
            link.source.as_deref(),
            &link.id,
            "file:///elsewhere/other.xlsx",
            mjx_ooxml::TargetMode::External
        )
        .expect("retargeting"));
    let _ = deck.remove_unused_parts().expect("sweeping");

    // --- lifecycle, again ------------------------------------------------------------------------------
    deck.validate().expect("the deck validates");
    let expected_text = deck
        .shape_text(slide, shape.clone())
        .expect("the text this deck holds");
    let slides = deck.slide_count();
    let saved = deck.save().expect("the deck saves");

    let mut reopened = Deck::open(&saved).expect("it reopens");
    assert_eq!(reopened.slide_count(), slides);
    assert_eq!(
        reopened.shape_text(slide, shape).expect("text"),
        expected_text,
        "the saved deck lost the text it was holding"
    );
}
