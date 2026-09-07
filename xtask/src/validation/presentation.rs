//! The PresentationML areas — `V-PPTX-01` … `V-PPTX-06`.
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
    Angle, CellBorder, CellFormat, CellMargins, Cells, CharacterPropertiesSpec, ChartData,
    ChartKind, ColorSpec, Deck, EffectListSpec, Emu, FillSpec, Fraction, GlowEffect,
    GradientStopSpec, Hyperlink, LegendPosition, LineSpec, LineWidth, OuterShadowEffect,
    ParagraphPropertiesSpec, PictureFillMode, PresetShapeType, SchemeColor, ShapeBounds, SlideSize,
    Surface, TableStyleBorder, TableStyleFormat, TableStylePart, TextAlignment, TextAnchoring,
    TrendlineKind, TrendlineSpec, DEFAULT_PLACEHOLDER_IMAGE,
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
