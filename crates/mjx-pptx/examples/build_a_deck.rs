//! The runnable version of [the building-a-deck guide](mjx_pptx::guide::building_a_deck).
//!
//! Opens a template, adds a slide from a layout, fills its placeholders, adds shapes and styles
//! them, adds a picture, a table and a chart, writes speaker notes, and saves.
//!
//! ```sh
//! cargo run -p mjx-pptx --example build_a_deck -- out.pptx
//! ```
//!
//! Note where the file I/O is: right here, in the caller. The library is bytes-in and bytes-out and
//! never touches a filesystem — which is exactly why the same calls work unchanged in a browser.

use anyhow::{Context, Result};
use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_pptx::{
    CellFormat, Cells, ChartData, ChartKind, Presentation, ShapeBounds, DEFAULT_PLACEHOLDER_IMAGE,
};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("build_a_deck.pptx");

    // ---- Open a template -------------------------------------------------------------------
    // `open` is the only constructor: a deck starts from a file, which is also where its theme,
    // master and layouts come from.
    let mut deck = Presentation::open(&support::template()?).context("opening the template")?;

    // ---- Look before editing ---------------------------------------------------------------
    println!(
        "template: {} slides, {} layouts, {} masters",
        deck.slide_count(),
        deck.layout_count(),
        deck.master_count()
    );
    for layout in deck.layouts()? {
        println!(
            "  layout {}: {:?} ({:?})",
            layout.index, layout.name, layout.kind
        );
    }

    // ---- A slide from a layout, and its placeholders ---------------------------------------
    let slide = deck.add_slide_from_layout(1)?;
    for shape in deck.shapes(slide)? {
        if let Some(placeholder) = shape.placeholder {
            println!(
                "  new slide shape {}: {:?} placeholder",
                shape.index, placeholder.kind
            );
        }
    }
    deck.set_shape_text_content(slide, 0, "Quarterly results")?;
    deck.set_shape_text_content(slide, 1, "Revenue up 14% year on year")?;

    // Nothing above set a font or a size: the title renders at the master's title size, in the
    // theme's major typeface, because that is what the layout and master say.
    let title = deck.effective_run_properties(slide, 0, 0, 0)?;
    println!("  title resolves to {:?}pt", title.size_points());

    // ---- Shapes of our own -----------------------------------------------------------------
    let badge = deck.add_shape(
        slide,
        PresetShapeType::Ellipse,
        ShapeBounds::from_inches(8.0, 0.4, 1.2, 1.2),
    )?;
    deck.set_shape_fill(
        slide,
        badge,
        &FillSpec::solid(ColorSpec::Srgb("1F3864".into())),
    )?;
    deck.set_shape_outline(
        slide,
        badge,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )?;

    let caption = deck.add_text_box(
        slide,
        "Source: internal",
        ShapeBounds::from_inches(0.5, 6.5, 4.0, 0.4),
    )?;
    deck.set_shape_run_properties(
        slide,
        caption,
        &CharacterPropertiesSpec::new()
            .with_size_points(10.0)
            .with_italic(true),
    )?;

    // ---- A picture -------------------------------------------------------------------------
    // Any image bytes will do; the format is sniffed and the bytes are stored untouched.
    deck.add_picture(
        slide,
        DEFAULT_PLACEHOLDER_IMAGE,
        ShapeBounds::from_inches(7.5, 5.5, 1.5, 1.5),
    )?;

    // ---- A table ---------------------------------------------------------------------------
    let table_slide = deck.add_slide_from_layout(1)?;
    deck.set_shape_text_content(table_slide, 0, "By region")?;
    let table = deck.add_table(
        table_slide,
        3,
        2,
        ShapeBounds::from_inches(1.0, 2.0, 6.0, 2.0),
    )?;
    for (row, (region, revenue)) in [("North", "4.2"), ("South", "3.1")].iter().enumerate() {
        deck.set_cell_text(table_slide, table, row + 1, 0, 0, region)?;
        deck.set_cell_text(table_slide, table, row + 1, 1, 0, revenue)?;
    }
    deck.set_cell_text(table_slide, table, 0, 0, 0, "Region")?;
    deck.set_cell_text(table_slide, table, 0, 1, 0, "Revenue")?;

    // One call for the whole header row — `Cells` names the selection, `CellFormat` the change.
    deck.format_cells(
        table_slide,
        table,
        Cells::row(0),
        &CellFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into()))),
    )?;
    deck.format_cell_text(
        table_slide,
        table,
        Cells::row(0),
        &CharacterPropertiesSpec::new()
            .with_bold(true)
            .with_color(ColorSpec::Srgb("FFFFFF".into())),
    )?;

    // ---- A chart ---------------------------------------------------------------------------
    let chart_slide = deck.add_slide_from_layout(1)?;
    deck.set_shape_text_content(chart_slide, 0, "Trend")?;
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25]);
    deck.add_chart(
        chart_slide,
        &chart,
        ShapeBounds::from_inches(1.0, 2.0, 8.0, 4.0),
    )?;

    // ---- Speaker notes ---------------------------------------------------------------------
    deck.set_notes_text(
        slide,
        "Lead with the revenue number, then the regional split.",
    )?;

    // ---- Save ------------------------------------------------------------------------------
    let bytes = deck.save()?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    // ---- Reopen what we wrote --------------------------------------------------------------
    // An example that never checks its own output is a claim, not a demonstration.
    let mut reopened = Presentation::open(&bytes)?;
    assert_eq!(reopened.slide_count(), deck.slide_count());
    assert_eq!(reopened.shape_text(slide, 0)?, "Quarterly results");
    assert_eq!(reopened.cell_text(table_slide, table, 0, 0)?, "Region");
    assert!(reopened.notes_text(slide)?.is_some());
    println!("reopened and verified");

    Ok(())
}
