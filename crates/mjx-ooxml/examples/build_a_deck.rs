//! The building-a-deck guide, written entirely through the facade.
//!
//! This is [`mjx-pptx`'s `build_a_deck`](https://docs.rs/mjx-pptx) example line for line, with one
//! difference that is the whole point: **it names no crate below `mjx-ooxml`**. Every type it uses —
//! the fills, the colours, the preset shape type, the chart description, the cell selection — is
//! re-exported here, so the only dependency an application needs is this one.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example build_a_deck -- out.pptx
//! ```
//!
//! Note where the file I/O is: right here, in the caller. The library is bytes-in and bytes-out and
//! never touches a filesystem — which is exactly why the same calls work unchanged in a browser.

use std::path::PathBuf;

use mjx_ooxml::{
    CellFormat, Cells, CharacterPropertiesSpec, ChartData, ChartKind, ColorSpec, Deck, Error,
    FillSpec, Format, LineSpec, LineWidth, PresetShapeType, ShapeBounds, DEFAULT_PLACEHOLDER_IMAGE,
};

/// The repository fixture the guide starts from — a small multi-layout template.
fn template() -> std::io::Result<Vec<u8>> {
    match std::env::var_os("MJX_TEMPLATE") {
        Some(path) => std::fs::read(path),
        None => std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/layouts.pptx"),
        ),
    }
}

/// Where this example writes: its first argument, or `target/examples/` by default.
fn output_path() -> PathBuf {
    match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("facade_build_a_deck.pptx")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = output_path();
    let bytes = template()?;

    // ---- What is this file? ----------------------------------------------------------------
    // Detection reads the package, not the name: a `.pptm` or a `.potx` answers correctly, and a
    // `.docx` renamed to `.pptx` is still reported as a Word document.
    let format = mjx_ooxml::detect_format(&bytes)?;
    println!(
        "template is {:?} (.{}), editable: {}",
        format,
        format.conventional_extension(),
        format.is_editable()
    );
    assert_eq!(format, Format::Presentation);

    // ---- Open --------------------------------------------------------------------------------
    let mut deck = Deck::open(&bytes)?;

    // ---- Look before editing -----------------------------------------------------------------
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

    // ---- A slide from a layout, and its placeholders ------------------------------------------
    // Indices are `u32` here, and a surface or a shape address is a concrete type; `.into()` is the
    // whole ceremony a bare index needs.
    let slide = deck.add_slide_from_layout(1)?;
    for shape in deck.shapes(slide.into())? {
        if let Some(placeholder) = shape.placeholder {
            println!(
                "  new slide shape {}: {:?} placeholder",
                shape.index, placeholder.kind
            );
        }
    }
    deck.set_shape_text_content(slide.into(), 0.into(), "Quarterly results")?;
    deck.set_shape_text_content(slide.into(), 1.into(), "Revenue up 14% year on year")?;

    // Nothing above set a font or a size: the title renders at the master's title size, in the
    // theme's major typeface, because that is what the layout and master say.
    let title = deck.effective_run_properties(slide.into(), 0.into(), 0, 0)?;
    println!("  title resolves to {:?}pt", title.size_points());

    // ---- Shapes of our own ---------------------------------------------------------------------
    let badge = deck.add_shape(
        slide.into(),
        PresetShapeType::Ellipse,
        ShapeBounds::from_inches(8.0, 0.4, 1.2, 1.2),
    )?;
    deck.set_shape_fill(
        slide.into(),
        badge.into(),
        &FillSpec::solid(ColorSpec::Srgb("1F3864".into())),
    )?;
    deck.set_shape_outline(
        slide.into(),
        badge.into(),
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )?;

    let caption = deck.add_text_box(
        slide.into(),
        "Source: internal",
        ShapeBounds::from_inches(0.5, 6.5, 4.0, 0.4),
    )?;
    deck.set_shape_run_properties(
        slide.into(),
        caption.into(),
        &CharacterPropertiesSpec::new()
            .with_size_points(10.0)
            .with_italic(true),
    )?;

    // ---- A picture -----------------------------------------------------------------------------
    deck.add_picture(
        slide.into(),
        DEFAULT_PLACEHOLDER_IMAGE,
        ShapeBounds::from_inches(7.5, 5.5, 1.5, 1.5),
    )?;

    // ---- A table -------------------------------------------------------------------------------
    let table_slide = deck.add_slide_from_layout(1)?;
    deck.set_shape_text_content(table_slide.into(), 0.into(), "By region")?;
    let table = deck.add_table(
        table_slide.into(),
        3,
        2,
        ShapeBounds::from_inches(1.0, 2.0, 6.0, 2.0),
    )?;
    for (row, (region, revenue)) in [("North", "4.2"), ("South", "3.1")].iter().enumerate() {
        let row = u32::try_from(row)? + 1;
        deck.set_cell_text(table_slide.into(), table.into(), row, 0, 0, region)?;
        deck.set_cell_text(table_slide.into(), table.into(), row, 1, 0, revenue)?;
    }
    deck.set_cell_text(table_slide.into(), table.into(), 0, 0, 0, "Region")?;
    deck.set_cell_text(table_slide.into(), table.into(), 0, 1, 0, "Revenue")?;

    // One call for the whole header row — `Cells` names the selection, `CellFormat` the change.
    deck.format_cells(
        table_slide.into(),
        table.into(),
        Cells::row(0),
        &CellFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into()))),
    )?;
    deck.format_cell_text(
        table_slide.into(),
        table.into(),
        Cells::row(0),
        &CharacterPropertiesSpec::new()
            .with_bold(true)
            .with_color(ColorSpec::Srgb("FFFFFF".into())),
    )?;

    // ---- A chart -------------------------------------------------------------------------------
    let chart_slide = deck.add_slide_from_layout(1)?;
    deck.set_shape_text_content(chart_slide.into(), 0.into(), "Trend")?;
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25]);
    deck.add_chart(
        chart_slide.into(),
        &chart,
        ShapeBounds::from_inches(1.0, 2.0, 8.0, 4.0),
    )?;

    // ---- Speaker notes -------------------------------------------------------------------------
    deck.set_notes_text(
        slide,
        "Lead with the revenue number, then the regional split.",
    )?;

    // ---- Package hygiene -----------------------------------------------------------------------
    // The three delegates that reach the package without handing out the part graph.
    println!("  {} external link(s)", deck.external_links().len());
    println!(
        "  swept {} unused part(s)",
        deck.remove_unused_parts()?.len()
    );

    // ---- Save ----------------------------------------------------------------------------------
    // `save` validates first, exactly as `Presentation::save` does; the facade does not route
    // around that check. `validate` is the same pass without writing.
    deck.validate()?;
    let saved = deck.save()?;
    std::fs::write(&out, &saved)?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Reopen what we wrote --------------------------------------------------------------
    // An example that never checks its own output is a claim, not a demonstration.
    let mut reopened = Deck::open(&saved)?;
    assert_eq!(reopened.slide_count(), deck.slide_count());
    assert_eq!(reopened.format(), Format::Presentation);
    assert_eq!(
        reopened.shape_text(slide.into(), 0.into())?,
        "Quarterly results"
    );
    assert_eq!(
        reopened.cell_text(table_slide.into(), table.into(), 0, 0)?,
        "Region"
    );
    assert!(reopened.notes_text(slide)?.is_some());
    println!("reopened and verified");

    // ---- And what a Word document does ---------------------------------------------------------
    // Detection works before editing does, so the refusal names the format instead of failing to
    // parse it.
    let docx = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sample.docx"),
    )?;
    assert_eq!(mjx_ooxml::detect_format(&docx)?, Format::Document);
    let refused: Error = Deck::open(&docx).expect_err("a Word document is not a deck");
    println!("opening a .docx: {} — {}", refused.code(), refused);

    Ok(())
}
