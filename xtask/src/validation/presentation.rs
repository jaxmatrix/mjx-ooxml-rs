//! The PresentationML areas — `V-PPTX-01` … `V-PPTX-08`.
//!
//! Every function here is a `mjx_ooxml::Deck` caller and names no crate below the facade, which is
//! what lets `bindings/mjx-python/tests/test_validation_artefacts.py` and
//! `bindings/mjx-wasm/tests/node/validation_artefacts.mjs` be the same calls in the other two
//! languages and produce the same bytes.
//!
//! Read the two halves of each area together: `authored_*` builds from
//! [`Deck::blank`](mjx_ooxml::Deck::blank), so every byte is one this library wrote; `edit_*` opens
//! an Office-authored original and changes the same things, so the artefact is mostly markup we did
//! not write with our edits in it. **Neither marks a result.**

use anyhow::{Context, Result};
use mjx_ooxml::{
    AdjustAngle, AdjustCoordinate, Angle, AxisOrientation, CellBorder, CellFormat, CellMargins,
    Cells, CharacterPropertiesSpec, ChartData, ChartKind, ChartLabelScope, ColorSpec,
    ColorTransform, ConnectionSite, CustomGeometrySpec, DataLabelPosition, DataLabelSpec, Deck,
    DrawCommand, EffectListSpec, Emu, ErrorBarDirection, ErrorBarSpec, ErrorBarType,
    ErrorValueType, FillSpec, Fraction, Geometry, GlowEffect, GradientStopSpec, GuideContext,
    GuideSpec, Hyperlink, LegendPosition, LineSpec, LineWidth, OuterShadowEffect,
    ParagraphPropertiesSpec, Path2DSpec, PictureFillMode, Point, PresetShapeType, Rectangle,
    SchemeColor, ShapeBounds, SlideSize, Surface, TableStyleBorder, TableStyleFormat,
    TableStylePart, TextAlignment, TextAnchoring, Transform2D, TrendlineKind, TrendlineSpec,
    DEFAULT_PLACEHOLDER_IMAGE,
};

/// A deck with one slide, ready for an area to fill.
fn one_slide_deck() -> Result<(Deck, Surface)> {
    let mut deck = Deck::blank(SlideSize::widescreen()).context("blank deck")?;
    let slide = deck.add_slide().context("add slide")?;
    Ok((deck, slide.into()))
}

/// The first slide of an opened deck, so the edit variants all start the same way.
fn opened(original: &[u8]) -> Result<(Deck, Surface)> {
    let deck = Deck::open(original).context("opening the Office-authored original")?;
    Ok((deck, 0.into()))
}

// -------------------------------------------------------------------------------------------
// V-PPTX-01 — text, paragraph and run properties, and what they inherit
// -------------------------------------------------------------------------------------------

/// Writes the text of `surface`: three text boxes whose properties are stated at three different
/// levels — the run, the paragraph, and nothing at all — so the reader can see what each inherits.
fn write_text_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    let stated = deck
        .add_text_box(
            surface,
            "Stated on the run: 24pt bold, accent 1",
            ShapeBounds::from_inches(0.5, 0.5, 8.0, 0.8),
        )
        .context("stated text box")?;
    deck.set_shape_run_properties(
        surface,
        stated.into(),
        &CharacterPropertiesSpec::new()
            .with_size_points(24.0)
            .with_bold(true)
            .with_color(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .context("run properties")?;

    let paragraph = deck
        .add_text_box(
            surface,
            "Stated on the paragraph: centred, 1.5 line spacing",
            ShapeBounds::from_inches(0.5, 1.6, 8.0, 0.8),
        )
        .context("paragraph text box")?;
    deck.set_paragraph_properties(
        surface,
        paragraph.into(),
        0,
        &ParagraphPropertiesSpec::new()
            .with_alignment(TextAlignment::Center)
            .with_left_margin_points(18.0)
            .with_indent_points(-18.0),
    )
    .context("paragraph properties")?;

    let inherited = deck
        .add_text_box(
            surface,
            "Nothing stated: this run resolves from the layout and the master",
            ShapeBounds::from_inches(0.5, 2.7, 8.0, 0.8),
        )
        .context("inherited text box")?;
    // The read-back is not an assertion about Office; it is what makes the artefact's *own*
    // resolution visible in the harness output, so a reviewer knows what this library thinks the
    // answer is before opening the file.
    let _resolved = deck
        .effective_run_properties(surface, inherited.into(), 0, 0)
        .context("effective run properties")?;
    Ok(())
}

/// `V-PPTX-01` authored.
///
/// # Errors
/// If the facade refuses any call, which is a defect here rather than anything a caller did.
pub(crate) fn authored_text_inheritance() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_text_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-01` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_text_inheritance(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_text_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-02 — preset geometry, fills, outlines and effects
// -------------------------------------------------------------------------------------------

fn write_shape_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    let solid = deck
        .add_shape(
            surface,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(0.5, 0.5, 2.5, 1.5),
        )
        .context("solid shape")?;
    deck.set_shape_fill(
        surface,
        solid.into(),
        &FillSpec::solid(ColorSpec::Srgb("1F3864".into())),
    )?;
    deck.set_shape_outline(
        surface,
        solid.into(),
        &LineSpec::solid(
            LineWidth::from_points(3.0),
            ColorSpec::Scheme(SchemeColor::Accent2),
        ),
    )?;

    let gradient = deck
        .add_shape(
            surface,
            PresetShapeType::Ellipse,
            ShapeBounds::from_inches(3.5, 0.5, 2.5, 1.5),
        )
        .context("gradient shape")?;
    deck.set_shape_fill(
        surface,
        gradient.into(),
        &FillSpec::linear_gradient(
            vec![
                GradientStopSpec {
                    position: Fraction::from_ratio(0.0),
                    color: ColorSpec::Srgb("FFF2CC".into()),
                },
                GradientStopSpec {
                    position: Fraction::from_ratio(1.0),
                    color: ColorSpec::Srgb("C00000".into()),
                },
            ],
            Angle::from_degrees(45.0),
        ),
    )?;

    let effects = deck
        .add_shape(
            surface,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(6.5, 0.5, 2.5, 1.5),
        )
        .context("effect shape")?;
    deck.set_shape_fill(
        surface,
        effects.into(),
        &FillSpec::solid(ColorSpec::Srgb("FFFFFF".into())),
    )?;
    deck.set_shape_effects(
        surface,
        effects.into(),
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
                ..OuterShadowEffect::new(ColorSpec::Srgb("808080".into()))
            }),
            ..EffectListSpec::new()
        },
    )?;
    write_colour_transform_areas(deck, surface)?;
    Ok(())
}

/// The two rows of swatches `V-PPTX-02.4` needs, and the reason that entry had no artefact until
/// MJXOFF-219: before it, `ColorSpec` carried a colour's kind and value and no transform children,
/// so **no facade call could author a colour transform at all** — and no committed fixture has one
/// either, because every fixture here was written by this project or by LibreOffice rather than by
/// Office. Both halves of the corpus were closed at once.
///
/// Read the two rows against each other. The **top** row is the four transforms `V-PPTX-02.4`
/// names, plus `a:inv`, over a fixed `4472C4`; `crates/mjx-dml/src/resolve.rs` says in as many
/// words that these follow *a documented interpretation* and are **not** guaranteed pixel-identical
/// to Office, which is exactly why R3 is the third-highest risk item in this repository and why the
/// eyedropper is the only instrument that can settle it. The **bottom** row is what a real file
/// actually contains — `tint`, `shade`, `satMod`, and the `lumMod`/`lumOff` pair PowerPoint writes
/// for every "Accent 1, Lighter 40 %" — over the theme's accent 1, so a reviewer can check the
/// common cases in the same pass as the rare ones. The first swatch in each row carries **no**
/// transform, and is the baseline every other swatch in that row is compared against.
fn write_colour_transform_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    let base = ColorSpec::Srgb("4472C4".into());
    let accent = ColorSpec::Scheme(SchemeColor::Accent1);
    let half = Fraction::from_ratio(0.5);
    let rows: [(f64, [(&str, ColorSpec); 6]); 2] = [
        (
            2.4,
            [
                ("4472C4", base.clone()),
                (
                    "comp",
                    base.clone().with_transform(ColorTransform::Complement),
                ),
                (
                    "gray",
                    base.clone().with_transform(ColorTransform::Grayscale),
                ),
                ("gamma", base.clone().with_transform(ColorTransform::Gamma)),
                (
                    "invGamma",
                    base.clone().with_transform(ColorTransform::InverseGamma),
                ),
                ("inv", base.with_transform(ColorTransform::Inverse)),
            ],
        ),
        (
            4.2,
            [
                ("accent 1", accent.clone()),
                ("tint 50%", accent.clone().with_tint(half)),
                ("shade 50%", accent.clone().with_shade(half)),
                (
                    "satMod 150%",
                    accent
                        .clone()
                        .with_saturation_modulation(Fraction::from_ratio(1.5)),
                ),
                (
                    "lumMod 60% + lumOff 40%",
                    accent
                        .clone()
                        .with_luminance_modulation(Fraction::from_ratio(0.6))
                        .with_luminance_offset(Fraction::from_ratio(0.4)),
                ),
                ("alpha 50%", accent.with_alpha(half)),
            ],
        ),
    ];

    for (top, swatches) in rows {
        for (column, (label, color)) in swatches.into_iter().enumerate() {
            let left = 0.35 + 2.12 * f64::from(u8::try_from(column)?);
            let swatch = deck
                .add_shape(
                    surface,
                    PresetShapeType::Rectangle,
                    ShapeBounds::from_inches(left, top, 2.0, 1.3),
                )
                .context("transform swatch")?;
            deck.set_shape_fill(surface, swatch.into(), &FillSpec::solid(color))
                .context("transform swatch fill")?;
            deck.set_shape_text_content(surface, swatch.into(), label)
                .context("transform swatch label")?;
            // The read-back is not an assertion about Office; it is what puts this library's own
            // answer in the harness output, so the reviewer knows what to compare the eyedropper
            // against before opening the file. `V-PPTX-02.4` names this exact call.
            let _resolved = deck
                .effective_shape_fill(surface, swatch.into())
                .context("effective shape fill")?;
        }
    }
    Ok(())
}

/// `V-PPTX-02` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_shape_appearance() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_shape_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-02` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_shape_appearance(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_shape_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-03 — tables
// -------------------------------------------------------------------------------------------

/// The table style id every table in this area points at. A GUID because that is what `a:tableStyle`
/// ids are; this one is not a built-in and is authored by `create_table_style` below.
const TABLE_STYLE_ID: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";

fn write_table_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    let table = deck
        .add_table(surface, 3, 3, ShapeBounds::from_inches(0.5, 0.5, 8.0, 2.5))
        .context("add table")?;
    for (row, cells) in [
        ["Region", "Revenue", "Growth"],
        ["North", "4.2", "+12%"],
        ["South", "3.1", "+8%"],
    ]
    .iter()
    .enumerate()
    {
        for (column, text) in cells.iter().enumerate() {
            deck.set_cell_text(
                surface,
                table.into(),
                u32::try_from(row)?,
                u32::try_from(column)?,
                0,
                text,
            )?;
        }
    }

    deck.create_table_style(TABLE_STYLE_ID, "mjx validation")?;
    deck.format_table_style_part(
        TABLE_STYLE_ID,
        TableStylePart::FirstRow,
        &TableStyleFormat::new()
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into())))
            .with_text_color(ColorSpec::Srgb("FFFFFF".into()))
            .with_border(
                TableStyleBorder::Bottom,
                LineSpec::solid(
                    LineWidth::from_points(2.0),
                    ColorSpec::Srgb("FFFFFF".into()),
                ),
            ),
    )?;
    deck.set_table_style(surface, table.into(), TABLE_STYLE_ID)?;

    deck.format_cells(
        surface,
        table.into(),
        Cells::row(0),
        &CellFormat::new()
            .with_anchor(TextAnchoring::Center)
            .with_margins(CellMargins::uniform(Emu::from_points(6.0))),
    )?;
    deck.set_cell_border(
        surface,
        table.into(),
        2,
        0,
        CellBorder::Top,
        &LineSpec::solid(
            LineWidth::from_points(1.0),
            ColorSpec::Srgb("C00000".into()),
        ),
    )?;
    deck.merge_cells(surface, table.into(), Cells::rectangle(2..3, 1..3))?;
    Ok(())
}

/// `V-PPTX-03` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_tables() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_table_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-03` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_tables(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_table_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-04 — charts
// -------------------------------------------------------------------------------------------

/// The chart every format's chart area draws, so a reviewer comparing a deck, a document and a
/// workbook side by side is comparing the same numbers.
pub(crate) fn quarterly_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25])
        .series("2025", [10.5, 13.0, 13.75, 16.0])
}

fn write_chart_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    let chart = deck
        .add_chart(
            surface,
            &quarterly_chart(),
            ShapeBounds::from_inches(0.5, 0.5, 8.0, 4.5),
        )
        .context("add chart")?;
    deck.set_chart_title(surface, chart.into(), Some("Revenue by quarter"))?;
    deck.set_chart_legend(surface, chart.into(), Some(LegendPosition::Bottom))?;
    deck.set_chart_axis_title(surface, chart.into(), 0, Some("Quarter"))?;
    deck.set_chart_axis_title(surface, chart.into(), 1, Some("Revenue"))?;
    deck.set_chart_series_fill(
        surface,
        chart.into(),
        0,
        &FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent1)),
    )?;
    deck.add_chart_trendline(
        surface,
        chart.into(),
        0,
        &TrendlineSpec::new(TrendlineKind::Linear),
    )?;
    Ok(())
}

/// `V-PPTX-04` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_charts() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_chart_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-04` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_charts(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_chart_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-05 — pictures
// -------------------------------------------------------------------------------------------

fn write_picture_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    deck.add_picture(
        surface,
        DEFAULT_PLACEHOLDER_IMAGE,
        ShapeBounds::from_inches(0.5, 0.5, 2.0, 2.0),
    )
    .context("add picture")?;
    let rel = deck
        .add_image(surface, DEFAULT_PLACEHOLDER_IMAGE)
        .context("add image part")?;
    let filled = deck
        .add_shape(
            surface,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(3.0, 0.5, 3.0, 2.0),
        )
        .context("picture-filled shape")?;
    deck.set_shape_fill(
        surface,
        filled.into(),
        &FillSpec::Picture {
            rel_id: rel,
            mode: PictureFillMode::Stretch,
        },
    )?;
    Ok(())
}

/// `V-PPTX-05` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_pictures() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_picture_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-05` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_pictures(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_picture_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-06 — notes and links
// -------------------------------------------------------------------------------------------

fn write_notes_and_link_areas(deck: &mut Deck, surface: Surface, slide_idx: u32) -> Result<()> {
    let linked = deck
        .add_text_box(
            surface,
            "The whole shape is a link",
            ShapeBounds::from_inches(0.5, 0.5, 4.0, 0.8),
        )
        .context("linked text box")?;
    deck.set_shape_hyperlink(
        surface,
        linked.into(),
        &Hyperlink::Url("https://example.com/investors".to_owned()),
    )?;

    let run_linked = deck
        .add_text_box(
            surface,
            "Only this run is a link",
            ShapeBounds::from_inches(0.5, 1.6, 4.0, 0.8),
        )
        .context("run-linked text box")?;
    deck.set_run_hyperlink(
        surface,
        run_linked.into(),
        0,
        0,
        &Hyperlink::Url("https://example.com/report".to_owned()),
    )?;

    deck.set_notes_text(
        slide_idx,
        "Lead with the revenue number, then the regional split.",
    )
    .context("notes")?;
    Ok(())
}

/// `V-PPTX-06` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_notes_and_links() -> Result<Vec<u8>> {
    let mut deck = Deck::blank(SlideSize::widescreen()).context("blank deck")?;
    let slide = deck.add_slide().context("add slide")?;
    write_notes_and_link_areas(&mut deck, slide.into(), slide)?;
    Ok(deck.save()?)
}

/// `V-PPTX-06` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_notes_and_links(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_notes_and_link_areas(&mut deck, slide, 0)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-07 — geometry, on a 4:3 deck
// -------------------------------------------------------------------------------------------

/// The guide-driven triangle: its apex is placed by the guide `apex = */ w 1 2` rather than by a
/// number, so the apex sits at the horizontal centre of whatever box the shape is given. It also
/// carries the four auxiliary lists a hand-written `a:custGeom` has — `a:avLst`, `a:gdLst`,
/// `a:cxnLst` and `a:rect` — because a `custGeom` that has only a path list exercises a quarter of
/// the element.
fn guide_driven_triangle() -> CustomGeometrySpec {
    let guide = |name: &str| AdjustCoordinate::Guide(name.to_owned());
    CustomGeometrySpec {
        adjust_values: vec![GuideSpec {
            name: "adj".to_owned(),
            formula: "val 50000".to_owned(),
        }],
        guides: vec![GuideSpec {
            name: "apex".to_owned(),
            formula: "*/ w 1 2".to_owned(),
        }],
        connection_sites: vec![
            ConnectionSite {
                angle: AdjustAngle::Angle(Angle::from_degrees(270.0)),
                position: Point {
                    x: guide("apex"),
                    y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                },
            },
            ConnectionSite {
                angle: AdjustAngle::Angle(Angle::from_degrees(0.0)),
                position: Point {
                    x: guide("r"),
                    y: guide("b"),
                },
            },
            ConnectionSite {
                angle: AdjustAngle::Angle(Angle::from_degrees(180.0)),
                position: Point {
                    x: guide("l"),
                    y: guide("b"),
                },
            },
        ],
        text_rectangle: Some(Rectangle {
            left: guide("l"),
            top: guide("vc"),
            right: guide("r"),
            bottom: guide("b"),
        }),
        paths: vec![Path2DSpec {
            commands: vec![
                DrawCommand::MoveTo(Point {
                    x: guide("apex"),
                    y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                }),
                DrawCommand::LineTo(Point {
                    x: guide("r"),
                    y: guide("b"),
                }),
                DrawCommand::LineTo(Point {
                    x: guide("l"),
                    y: guide("b"),
                }),
                DrawCommand::Close,
            ],
            ..Path2DSpec::default()
        }],
        ..CustomGeometrySpec::default()
    }
}

/// The four presets whose guide formulas take an arc-tangent argument through zero — the ones whose
/// evaluator was the point of MJXOFF-56.
const ARC_TANGENT_PRESETS: [PresetShapeType; 4] = [
    PresetShapeType::Moon,
    PresetShapeType::Arc,
    PresetShapeType::CircularArrow,
    PresetShapeType::Gear9,
];

fn write_geometry_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    // Two chevrons, one square and one 2:1. `shape_adjustments` resolves an adjustment's domain
    // against a *concrete* size, so the same preset answers a different maximum for each.
    let square_chevron = deck
        .add_shape(
            surface,
            PresetShapeType::Chevron,
            ShapeBounds::from_inches(0.4, 3.4, 2.5, 2.5),
        )
        .context("square chevron")?;
    let wide_chevron = deck
        .add_shape(
            surface,
            PresetShapeType::Chevron,
            ShapeBounds::from_inches(3.2, 3.4, 2.5, 1.25),
        )
        .context("2:1 chevron")?;

    // The four arc-tangent presets, in a row.
    for (position, preset) in ARC_TANGENT_PRESETS.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let left = 0.4 + 1.4 * position as f64;
        deck.add_shape(
            surface,
            preset,
            ShapeBounds::from_inches(left, 6.1, 1.2, 1.2),
        )
        .with_context(|| format!("preset {preset:?}"))?;
    }

    // The custom geometry, on a shape twice as wide as it is tall, so a guide-placed apex is
    // visibly *not* where a literal coordinate would have put it.
    let custom = deck
        .add_shape(
            surface,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(6.2, 3.4, 3.0, 1.5),
        )
        .context("custom-geometry shape")?;
    deck.set_shape_geometry(
        surface,
        custom.into(),
        Geometry::Custom(guide_driven_triangle()),
    )
    .context("custom geometry")?;
    deck.set_shape_fill(
        surface,
        custom.into(),
        &FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent2)),
    )?;

    // Read both chevrons' adjustment domains back, so the harness output carries what this library
    // believes before anybody drags a handle. Neither call is an assertion about PowerPoint, and
    // reading geometry does not dirty the part.
    let _square = deck
        .shape_adjustments(
            surface,
            square_chevron.into(),
            GuideContext::from_extents(Emu::from_emu(2_286_000), Emu::from_emu(2_286_000)),
        )
        .context("square chevron adjustments")?;
    let _wide = deck
        .shape_adjustments(
            surface,
            wide_chevron.into(),
            GuideContext::from_extents(Emu::from_emu(2_286_000), Emu::from_emu(1_143_000)),
        )
        .context("2:1 chevron adjustments")?;
    let _read_back = deck
        .shape_geometry(surface, custom.into())
        .context("reading the custom geometry back")?;

    // A rotated shape. Note what this is *not*: R7 asks about a transform naming a rotation and
    // neither `a:off` nor `a:ext`, and `set_shape_transform` is documented to write only the fields
    // its argument names — *an unset field means leave it alone, never clear it* — so no facade call
    // can author that shape. What this one gives the pass is the rotation itself, and the bounds
    // this library reports for it; R7's own case waits on an original that carries one
    // (`V-PPTX-07.6`).
    let rotated = deck
        .add_shape(
            surface,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(6.2, 5.2, 1.6, 1.0),
        )
        .context("rotation-only shape")?;
    deck.set_shape_transform(
        surface,
        rotated.into(),
        &Transform2D {
            rotation: Some(Angle::from_degrees(30.0)),
            ..Transform2D::default()
        },
    )
    .context("rotation-only transform")?;
    let _no_bounds = deck
        .effective_shape_bounds(surface, rotated.into())
        .context("effective bounds of the rotation-only shape")?;
    Ok(())
}

/// `V-PPTX-07` authored — **the one artefact built at 4:3**
/// (`9_144_000` x `6_858_000`, `SlideSize::standard`), so the master's placeholders arrive
/// rescaled rather than at the widescreen positions every other area shows.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_geometry() -> Result<Vec<u8>> {
    let mut deck = Deck::blank(SlideSize::standard()).context("blank 4:3 deck")?;
    // From the layout rather than `add_slide`, so the slide arrives with the master's title and
    // body placeholders on it — which is what makes the rescale visible.
    let slide = deck.add_slide_from_layout(0).context("add slide")?;
    deck.set_shape_text_content(slide.into(), 0u32.into(), "4:3 — 10 x 7.5 in")
        .context("title text")?;
    deck.set_shape_text_content(
        slide.into(),
        1u32.into(),
        "The two placeholders above and beside this one were placed by the master, not by this code.",
    )
    .context("body text")?;
    write_geometry_areas(&mut deck, slide.into())?;
    Ok(deck.save()?)
}

/// `V-PPTX-07` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_geometry(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_geometry_areas(&mut deck, slide)?;
    Ok(deck.save()?)
}

// -------------------------------------------------------------------------------------------
// V-PPTX-08 — chart decoration, and every plot type
// -------------------------------------------------------------------------------------------

/// The fifteen plot types that draw from one series, in the order `ChartKind` declares them.
/// `Stock` is the sixteenth and is authored separately, because `c:stockChart` is the one plot
/// type the schema will not accept with fewer than three series.
/// Each is paired with its `c:` element's local name, spelled out rather than read off the value,
/// because `ChartKind` publishes no name accessor through the bindings and the three languages
/// have to write the same title byte for byte.
const SINGLE_SERIES_KINDS: [(ChartKind, &str); 15] = [
    (ChartKind::Bar, "barChart"),
    (ChartKind::Bar3D, "bar3DChart"),
    (ChartKind::Line, "lineChart"),
    (ChartKind::Line3D, "line3DChart"),
    (ChartKind::Pie, "pieChart"),
    (ChartKind::Pie3D, "pie3DChart"),
    (ChartKind::OfPie, "ofPieChart"),
    (ChartKind::Area, "areaChart"),
    (ChartKind::Area3D, "area3DChart"),
    (ChartKind::Scatter, "scatterChart"),
    (ChartKind::Doughnut, "doughnutChart"),
    (ChartKind::Radar, "radarChart"),
    (ChartKind::Bubble, "bubbleChart"),
    (ChartKind::Surface, "surfaceChart"),
    (ChartKind::Surface3D, "surface3DChart"),
];

/// A quarter-inch-precise four-up grid, so sixteen charts read as a gallery rather than a pile.
fn gallery_bounds(position: usize) -> ShapeBounds {
    #[allow(clippy::cast_precision_loss)]
    let column = (position % 2) as f64;
    #[allow(clippy::cast_precision_loss)]
    let row = ((position % 4) / 2) as f64;
    ShapeBounds::from_inches(0.4 + column * 6.4, 0.4 + row * 3.4, 6.0, 3.0)
}

fn write_chart_decoration_areas(deck: &mut Deck, surface: Surface) -> Result<()> {
    // ---- The edited series -------------------------------------------------------------------
    // Authored at the fixture's own figures, then overwritten. Both the chart and its embedded
    // workbook must show the second set; the first must appear nowhere.
    let edited = deck
        .add_chart(
            surface,
            &ChartData::new(ChartKind::Bar)
                .categories(["Jan", "Feb", "Mar"])
                .series("Sales", [19.2, 21.4, 16.7]),
            ShapeBounds::from_inches(0.4, 0.4, 5.8, 2.6),
        )
        .context("edited-series chart")?;
    deck.set_chart_title(surface, edited.into(), Some("Series values, rewritten"))?;
    deck.set_chart_series_values(surface, edited.into(), 0, &[41.5, 42.5, 43.5])
        .context("rewriting the series values")?;

    // ---- The three label tiers ---------------------------------------------------------------
    let labelled = deck
        .add_chart(
            surface,
            &quarterly_chart(),
            ShapeBounds::from_inches(6.6, 0.4, 6.2, 2.6),
        )
        .context("labelled chart")?;
    deck.set_chart_title(surface, labelled.into(), Some("Labels, three tiers"))?;
    deck.set_chart_data_labels(
        surface,
        labelled.into(),
        ChartLabelScope::Plot { plot_index: 0 },
        &DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd)
            .separator("; ")
            .number_format("0.0"),
    )
    .context("plot-tier labels")?;
    deck.set_chart_data_labels(
        surface,
        labelled.into(),
        ChartLabelScope::Series { series_index: 0 },
        &DataLabelSpec::new().category_name(true),
    )
    .context("series-tier labels")?;
    deck.suppress_chart_data_labels(
        surface,
        labelled.into(),
        ChartLabelScope::Point {
            series_index: 0,
            point_index: 1,
        },
    )
    .context("point-tier suppression")?;
    deck.suppress_chart_data_labels(
        surface,
        labelled.into(),
        ChartLabelScope::Series { series_index: 1 },
    )
    .context("series-tier suppression")?;
    deck.add_chart_trendline(
        surface,
        labelled.into(),
        0,
        &TrendlineSpec::new(TrendlineKind::Polynomial)
            .polynomial_order(3)
            .projection(2.0, 0.0)
            .display(true, true),
    )
    .context("polynomial trendline")?;

    // ---- Per-point formatting on a pie -------------------------------------------------------
    let pie = deck
        .add_chart(
            surface,
            &ChartData::new(ChartKind::Pie)
                .categories(["North", "South", "East", "West"])
                .series("Share", [42.0, 28.0, 18.0, 12.0]),
            ShapeBounds::from_inches(0.4, 3.4, 5.8, 3.4),
        )
        .context("pie chart")?;
    deck.set_chart_title(
        surface,
        pie.into(),
        Some("Slice 1 exploded, slice 0 recoloured"),
    )?;
    deck.set_chart_point_explosion(surface, pie.into(), 0, 1, Some(25))
        .context("slice explosion")?;
    deck.set_chart_point_fill(
        surface,
        pie.into(),
        0,
        0,
        &FillSpec::solid(ColorSpec::Srgb("2E75B6".into())),
    )
    .context("slice fill")?;

    // ---- Two sets of error bars, one per axis ------------------------------------------------
    let scatter = deck
        .add_chart(
            surface,
            &ChartData::new(ChartKind::Scatter)
                .categories(["1", "2", "3", "4"])
                .series("Measured", [2.0, 4.5, 3.25, 6.0]),
            ShapeBounds::from_inches(6.6, 3.4, 6.2, 3.4),
        )
        .context("scatter chart")?;
    deck.set_chart_title(surface, scatter.into(), Some("Error bars on both axes"))?;
    deck.set_chart_error_bars(
        surface,
        scatter.into(),
        0,
        &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::Percentage, 5.0)
            .direction(ErrorBarDirection::X),
    )
    .context("x error bars")?;
    deck.set_chart_error_bars(
        surface,
        scatter.into(),
        0,
        &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 0.5)
            .direction(ErrorBarDirection::Y),
    )
    .context("y error bars")?;
    Ok(())
}

/// The axes-and-workbook chart, on a slide of its own: bounded, reversed, ruled, its two series
/// coloured — and then **detached** from its embedded workbook, so *Edit Data* has nothing to open.
fn write_axes_and_detached_workbook_area(deck: &mut Deck, surface: Surface) -> Result<()> {
    let axes = deck
        .add_chart(
            surface,
            &quarterly_chart(),
            ShapeBounds::from_inches(0.4, 0.4, 6.0, 3.0),
        )
        .context("axes chart")?;
    deck.set_chart_title(surface, axes.into(), Some("Bounded 0-25, reversed, ruled"))?;
    deck.set_chart_axis_title(surface, axes.into(), 0, Some("Quarter"))?;
    deck.set_chart_axis_title(surface, axes.into(), 1, Some("Revenue"))?;
    deck.set_chart_axis_scale(surface, axes.into(), 1, Some(0.0), Some(25.0))
        .context("axis scale")?;
    deck.set_chart_axis_orientation(surface, axes.into(), 1, AxisOrientation::MaximumToMinimum)
        .context("axis orientation")?;
    deck.set_chart_axis_gridlines(surface, axes.into(), 0, true, false)
        .context("category gridlines")?;
    deck.set_chart_axis_gridlines(surface, axes.into(), 1, true, true)
        .context("value gridlines")?;
    deck.set_chart_series_fill(
        surface,
        axes.into(),
        0,
        &FillSpec::solid(ColorSpec::Srgb("4472C4".into())),
    )
    .context("series 0 fill")?;
    deck.set_chart_series_line(
        surface,
        axes.into(),
        1,
        &LineSpec::solid(
            LineWidth::from_points(2.0),
            ColorSpec::Srgb("ED7D31".into()),
        ),
    )
    .context("series 1 outline")?;

    let detached = deck
        .add_chart(
            surface,
            &quarterly_chart(),
            ShapeBounds::from_inches(6.8, 0.4, 6.0, 3.0),
        )
        .context("detached chart")?;
    deck.set_chart_title(
        surface,
        detached.into(),
        Some("This chart has no embedded workbook"),
    )?;
    deck.detach_chart_workbook(surface, detached.into())
        .context("detaching the workbook")?;
    let _workbooks = deck
        .chart_workbooks(surface)
        .context("reading the workbook list back")?;
    Ok(())
}

/// The dangling-anchor case, which needs a slide of its own because it ends with a chart whose
/// `c:dPt` addresses a point the series no longer has.
fn write_dangling_point_area(deck: &mut Deck, surface: Surface) -> Result<()> {
    let shortened = deck
        .add_chart(
            surface,
            &ChartData::new(ChartKind::Bar)
                .categories(["Q1", "Q2", "Q3"])
                .series("2026", [4.0, 5.0, 6.0]),
            ShapeBounds::from_inches(0.4, 0.4, 6.0, 3.0),
        )
        .context("dangling-point chart")?;
    deck.set_chart_title(
        surface,
        shortened.into(),
        Some("A c:dPt left past the end of its series"),
    )?;
    // Format the last point, then shorten the series over it. `c:idx` is never renumbered by an
    // edit that changes a series' length, so the anchor stays at 2 and now addresses nothing.
    deck.set_chart_point_fill(
        surface,
        shortened.into(),
        0,
        2,
        &FillSpec::solid(ColorSpec::Srgb("C00000".into())),
    )
    .context("last-point fill")?;
    deck.set_chart_series_values(surface, shortened.into(), 0, &[4.0, 5.0])
        .context("shortening the series")?;
    let _dangling = deck
        .chart_dangling_decoration(surface, shortened.into(), 0)
        .context("reading the dangling decoration back")?;
    Ok(())
}

/// The sixteen plot types, four to a slide.
fn write_plot_type_gallery(deck: &mut Deck) -> Result<()> {
    let mut slide = None;
    for (position, (kind, name)) in SINGLE_SERIES_KINDS.into_iter().enumerate() {
        if position % 4 == 0 {
            slide = Some(deck.add_slide().context("gallery slide")?);
        }
        let surface: Surface = slide.expect("a gallery slide").into();
        let data = ChartData::new(kind)
            .categories(["A", "B", "C"])
            .series("S", [1.0, 2.0, 3.0]);
        let chart = deck
            .add_chart(surface, &data, gallery_bounds(position))
            .with_context(|| format!("gallery chart {kind:?}"))?;
        deck.set_chart_title(surface, chart.into(), Some(name))?;
    }
    // The sixteenth: `c:stockChart` is high-low-close, and the schema will not take fewer than
    // three series.
    let surface: Surface = slide.expect("a gallery slide").into();
    let stock = deck
        .add_chart(
            surface,
            &ChartData::new(ChartKind::Stock)
                .categories(["Mon", "Tue", "Wed"])
                .series("High", [7.0, 8.0, 9.0])
                .series("Low", [3.0, 4.0, 5.0])
                .series("Close", [5.0, 6.0, 7.0]),
            gallery_bounds(3),
        )
        .context("stock chart")?;
    deck.set_chart_title(surface, stock.into(), Some("stockChart"))?;
    Ok(())
}

/// `V-PPTX-08` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_chart_decoration() -> Result<Vec<u8>> {
    let (mut deck, slide) = one_slide_deck()?;
    write_chart_decoration_areas(&mut deck, slide)?;
    let axes = deck.add_slide().context("axes slide")?;
    write_axes_and_detached_workbook_area(&mut deck, axes.into())?;
    let dangling = deck.add_slide().context("dangling-point slide")?;
    write_dangling_point_area(&mut deck, dangling.into())?;
    write_plot_type_gallery(&mut deck)?;
    Ok(deck.save()?)
}

/// `V-PPTX-08` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_chart_decoration(original: &[u8]) -> Result<Vec<u8>> {
    let (mut deck, slide) = opened(original)?;
    write_chart_decoration_areas(&mut deck, slide)?;
    let axes = deck.add_slide().context("axes slide")?;
    write_axes_and_detached_workbook_area(&mut deck, axes.into())?;
    let dangling = deck.add_slide().context("dangling-point slide")?;
    write_dangling_point_area(&mut deck, dangling.into())?;
    write_plot_type_gallery(&mut deck)?;
    Ok(deck.save()?)
}
