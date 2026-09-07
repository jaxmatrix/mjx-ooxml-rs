//! The SpreadsheetML areas — `V-XLSX-01` … `V-XLSX-06`.
//!
//! Every function here is a `mjx_ooxml::Workbook` caller and names no crate below the facade. See
//! [`super::presentation`] for the shape both halves of every area take.
//!
//! **Every cell is written in one [`Workbook::write_cells`](mjx_ooxml::Workbook::write_cells)
//! call.** That is not an optimisation of the generator; it is the only shape this facade offers,
//! because a per-cell call costs a whole-worksheet parse each time it is made.

use anyhow::{Context, Result};
use mjx_ooxml::{
    BorderEdgeSpec, BorderSpec, BorderStyle, CellFormatSpec, CellFormatTarget, CellInput,
    CellWrite, ChartKind, ChartRangeSeries, Color, FontProperties, LegendPosition, PatternFillSpec,
    ResizingBehavior, Workbook, DEFAULT_PLACEHOLDER_IMAGE,
};

/// One inch, in EMU — the unit the anchored-picture calls take.
const INCH: i64 = 914_400;

fn blank() -> Result<Workbook> {
    Workbook::blank().context("blank workbook")
}

fn opened(original: &[u8]) -> Result<Workbook> {
    Workbook::open(original).context("opening the Office-authored original")
}

/// The figures every workbook area writes into its first sheet, so a reviewer comparing two
/// artefacts is comparing the same numbers.
fn quarterly_cells() -> Vec<CellWrite> {
    vec![
        CellWrite::new("A1", CellInput::SharedText("Region".to_owned())),
        CellWrite::new("B1", CellInput::SharedText("Revenue".to_owned())),
        CellWrite::new("C1", CellInput::SharedText("Growth".to_owned())),
        CellWrite::new("A2", CellInput::SharedText("North America".to_owned())),
        CellWrite::new("B2", CellInput::Number(1_250_000.0)),
        CellWrite::new("C2", CellInput::Number(0.125)),
        CellWrite::new("A3", CellInput::SharedText("EMEA".to_owned())),
        CellWrite::new("B3", CellInput::Number(980_000.0)),
        CellWrite::new("C3", CellInput::Number(0.061)),
        CellWrite::new("A4", CellInput::SharedText("Asia Pacific".to_owned())),
        CellWrite::new("B4", CellInput::Number(1_410_000.0)),
        CellWrite::new("C4", CellInput::Number(0.198)),
    ]
}

// -------------------------------------------------------------------------------------------
// V-XLSX-01 — cell values
// -------------------------------------------------------------------------------------------

fn write_value_areas(workbook: &mut Workbook) -> Result<()> {
    let mut cells = quarterly_cells();
    cells.push(CellWrite::new(
        "A6",
        CellInput::SharedText("Audited".to_owned()),
    ));
    cells.push(CellWrite::new("B6", CellInput::Boolean(false)));
    cells.push(CellWrite::new(
        "A7",
        CellInput::SharedText("Deliberately in error".to_owned()),
    ));
    cells.push(CellWrite::new("B7", CellInput::Error("#N/A".to_owned())));
    cells.push(CellWrite::new(
        "A8",
        CellInput::InlineText(
            "Inline, not shared: this string is written into the cell".to_owned(),
        ),
    ));
    cells.push(CellWrite::new("B8", CellInput::Blank));
    workbook.write_cells(0, &cells).context("write cells")?;
    let notes = workbook.add_sheet("Notes").context("add sheet")?;
    workbook
        .write_cells(
            notes,
            &[CellWrite::new(
                "A1",
                CellInput::InlineText("Figures are unaudited and subject to revision.".to_owned()),
            )],
        )
        .context("write notes")?;
    Ok(())
}

/// `V-XLSX-01` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_cell_values() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Summary")?;
    write_value_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-01` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_cell_values(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_value_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

// -------------------------------------------------------------------------------------------
// V-XLSX-02 — the two style layers
// -------------------------------------------------------------------------------------------

/// Builds the two-layer arrangement `docs/EFFECTIVE_CELL_FORMAT_HANDOFF.md` asks about: a
/// `cellStyleXfs` record beneath a `cellXfs` record, each naming a *different* font, fill and
/// border, so which one Excel renders is visible rather than inferred.
///
/// **Nothing here decides the answer.** The table in that hand-off has an empty Excel column on
/// purpose, and it stays empty; this function only produces the file that column will be filled
/// from.
fn write_format_areas(workbook: &mut Workbook) -> Result<()> {
    workbook
        .write_cells(0, &quarterly_cells())
        .context("write cells")?;

    // The lower layer — a named style's format.
    let style_font = workbook.append_font(&FontProperties {
        font_name: Some("Times New Roman".to_owned()),
        italic: Some(true),
        size_in_points: Some(13.0),
        ..FontProperties::default()
    })?;
    let style_fill = workbook.append_pattern_fill(&PatternFillSpec::solid("445566"))?;
    let style_border = workbook.append_border(&BorderSpec {
        right: Some(BorderEdgeSpec {
            style: Some(BorderStyle::Thick),
            color: Some(Color::from_opaque_rgb("000000")),
        }),
        ..BorderSpec::skeleton_border()
    })?;
    let beneath = workbook.append_cell_format(
        CellFormatTarget::CellStyleFormats,
        &CellFormatSpec {
            font_index: Some(style_font),
            applies_font: Some(true),
            fill_index: Some(style_fill),
            applies_fill: Some(true),
            border_index: Some(style_border),
            applies_border: Some(true),
            ..CellFormatSpec::skeleton_cell_style_format()
        },
    )?;

    // The upper layer — a direct format that names the record beneath it and overrides its font.
    let direct_font = workbook.append_font(&FontProperties {
        font_name: Some("Arial".to_owned()),
        bold: Some(true),
        size_in_points: Some(12.0),
        ..FontProperties::default()
    })?;
    let direct_fill = workbook.append_pattern_fill(&PatternFillSpec::solid("112233"))?;
    let direct_border = workbook.append_border(&BorderSpec {
        left: Some(BorderEdgeSpec {
            style: Some(BorderStyle::Thin),
            color: Some(Color::from_opaque_rgb("000000")),
        }),
        ..BorderSpec::skeleton_border()
    })?;
    let direct = workbook.append_cell_format(
        CellFormatTarget::CellFormats,
        &CellFormatSpec {
            cell_style_format_index: Some(beneath),
            font_index: Some(direct_font),
            applies_font: Some(true),
            fill_index: Some(direct_fill),
            applies_fill: Some(true),
            border_index: Some(direct_border),
            applies_border: Some(true),
            ..CellFormatSpec::skeleton_cell_format()
        },
    )?;
    workbook.set_cell_style(0, "A1", Some(direct))?;
    workbook.set_cell_style(0, "B1", Some(direct))?;
    workbook.set_cell_style(0, "C1", Some(direct))?;

    // What this library says the cell resolves to. Recorded here so the artefact and the answer
    // travel together; it is not a claim about what Excel renders.
    let _resolved = workbook
        .effective_cell_format(0, "A1")
        .context("effective cell format")?;
    Ok(())
}

/// `V-XLSX-02` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_cell_formats() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Formats")?;
    write_format_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-02` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_cell_formats(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_format_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

// -------------------------------------------------------------------------------------------
// V-XLSX-03 — the grid
// -------------------------------------------------------------------------------------------

fn write_grid_areas(workbook: &mut Workbook) -> Result<()> {
    workbook
        .write_cells(0, &quarterly_cells())
        .context("write cells")?;
    workbook.merge_cells(0, "A6:C6")?;
    workbook.set_row_height(0, 0, Some(30.0), true)?;
    workbook.set_row_hidden(0, 4, true)?;
    workbook.set_row_outline_level(0, 2, 1)?;
    workbook.set_row_outline_level(0, 3, 1)?;
    workbook.set_column_width(0, 0, 0, Some(24.0), true)?;
    workbook.set_column_width(0, 1, 2, Some(14.0), true)?;
    workbook.set_column_hidden(0, 5, 5, true)?;
    Ok(())
}

/// `V-XLSX-03` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_grid() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Grid")?;
    write_grid_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-03` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_grid(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_grid_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

// -------------------------------------------------------------------------------------------
// V-XLSX-04 — charts
// -------------------------------------------------------------------------------------------

fn write_chart_areas(workbook: &mut Workbook) -> Result<()> {
    workbook
        .write_cells(0, &quarterly_cells())
        .context("write cells")?;
    let sheet_name = workbook.sheet(0)?.name;
    let anchor = workbook
        .add_range_chart(
            0,
            ChartKind::Bar,
            Some(&format!("{sheet_name}!$A$2:$A$4")),
            &[
                ChartRangeSeries::new("Revenue", format!("{sheet_name}!$B$2:$B$4"))
                    .named_by_cell(format!("{sheet_name}!$B$1")),
            ],
            4,
            1,
            11,
            16,
            "Revenue by region",
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .context("range chart")?;
    workbook.set_chart_title(0, anchor, Some("Revenue by region"))?;
    workbook.set_chart_legend(0, anchor, Some(LegendPosition::Bottom))?;
    workbook.set_chart_axis_title(0, anchor, 0, Some("Region"))?;
    Ok(())
}

/// `V-XLSX-04` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_charts() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Charts")?;
    write_chart_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-04` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_charts(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_chart_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

// -------------------------------------------------------------------------------------------
// V-XLSX-05 — drawings
// -------------------------------------------------------------------------------------------

fn write_drawing_areas(workbook: &mut Workbook) -> Result<()> {
    workbook
        .write_cells(0, &quarterly_cells())
        .context("write cells")?;
    workbook
        .add_two_cell_anchored_picture(
            0,
            DEFAULT_PLACEHOLDER_IMAGE,
            "Two-cell anchored",
            4,
            0,
            1,
            0,
            7,
            0,
            8,
            0,
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .context("two-cell anchored picture")?;
    workbook
        .add_one_cell_anchored_picture(
            0,
            DEFAULT_PLACEHOLDER_IMAGE,
            "One-cell anchored",
            4,
            0,
            10,
            0,
            2 * INCH,
            INCH,
        )
        .context("one-cell anchored picture")?;
    Ok(())
}

/// `V-XLSX-05` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_drawings() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Drawings")?;
    write_drawing_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-05` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_drawings(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_drawing_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

// -------------------------------------------------------------------------------------------
// V-XLSX-06 — comments and links
// -------------------------------------------------------------------------------------------

fn write_comment_areas(workbook: &mut Workbook) -> Result<()> {
    workbook
        .write_cells(0, &quarterly_cells())
        .context("write cells")?;
    workbook
        .add_cell_comment(
            0,
            "B2",
            "Reviewer",
            "Confirm the North America figure before publishing.",
        )
        .context("cell comment")?;
    workbook
        .add_cell_comment(0, "C4", "Reviewer", "Growth restated in March.")
        .context("second cell comment")?;
    workbook
        .set_cell_hyperlink_url(0, "A1", "https://example.com/investors")
        .context("cell hyperlink")?;
    Ok(())
}

/// `V-XLSX-06` authored.
///
/// # Errors
/// If the facade refuses any call.
pub(crate) fn authored_comments_and_links() -> Result<Vec<u8>> {
    let mut workbook = blank()?;
    workbook.rename_sheet(0, "Comments")?;
    write_comment_areas(&mut workbook)?;
    Ok(workbook.save()?)
}

/// `V-XLSX-06` edited.
///
/// # Errors
/// If the original cannot be opened or edited.
pub(crate) fn edit_comments_and_links(original: &[u8]) -> Result<Vec<u8>> {
    let mut workbook = opened(original)?;
    write_comment_areas(&mut workbook)?;
    Ok(workbook.save()?)
}
