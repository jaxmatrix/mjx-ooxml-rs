//! The building-a-workbook walkthrough, written entirely through the facade.
//!
//! `bindings/mjx-python/tests/test_build_a_workbook.py` and
//! `bindings/mjx-wasm/tests/node/build_a_workbook.mjs` are the same walkthrough, call for call, in
//! the other two languages MJXOFF-137 curated `Workbook` for — and each compares its workbook
//! against the one this file writes **part by part, byte for byte**. A method wired to the wrong
//! `Workbook` method changes one payload and fails there.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example build_a_workbook -- out.xlsx
//! ```
//!
//! Note where the file I/O is: right here, in the caller. The library is bytes-in and bytes-out and
//! never touches a filesystem — which is exactly why the same calls work unchanged in a browser.
//!
//! # The one thing to notice
//!
//! **Every cell in this file is written in a single [`Workbook::write_cells`] call**, and every cell
//! read back comes from a single [`Workbook::read_range`]. That is not an optimisation of the
//! walkthrough; it is the only shape this facade offers, because a per-cell call costs a
//! whole-worksheet parse each time it is made — 387 ms per cell on a 300,000-cell sheet. See
//! [`mjx_ooxml::workbook`] and [*Large workbooks*](mjx_xlsx::guide::large_workbooks).

use std::path::PathBuf;

use mjx_ooxml::{
    BorderEdgeSpec, BorderSpec, BorderStyle, CellFormatSpec, CellFormatTarget, CellInput,
    CellWrite, Color, FontProperties, Format, PatternFillSpec, SpreadsheetPatternType, Workbook,
};

/// Where this example writes: its first argument, or `target/examples/` by default.
fn output_path() -> PathBuf {
    match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("facade_build_a_workbook.xlsx")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = output_path();

    // ---- Blank -------------------------------------------------------------------------------
    // Nothing is read from disk: every part is authored from this library's own element builders,
    // which is what makes a workbook buildable from a `pip install` or a browser tab with no input
    // file.
    let mut workbook = Workbook::blank()?;
    assert_eq!(workbook.format(), Format::Workbook);
    assert_eq!(workbook.sheet_count(), 1);

    // ---- Tabs --------------------------------------------------------------------------------
    workbook.rename_sheet(0, "Summary")?;
    let notes = workbook.add_sheet("Notes")?;
    assert_eq!(notes, 1);
    assert_eq!(workbook.sheet_index("Summary"), Some(0));

    // ---- Cells: one call, every cell -----------------------------------------------------------
    // `SharedText` interns into `xl/sharedStrings.xml` (what Excel writes when the same text repeats);
    // `InlineText` stores the string in the cell itself. Both read back as `CellData::Text`.
    workbook.write_cells(
        0,
        &[
            CellWrite::new("A1", CellInput::SharedText("Region".into())),
            CellWrite::new("B1", CellInput::SharedText("Revenue".into())),
            CellWrite::new("C1", CellInput::SharedText("Growth".into())),
            CellWrite::new("A2", CellInput::SharedText("North America".into())),
            CellWrite::new("B2", CellInput::Number(1_250_000.0)),
            CellWrite::new("C2", CellInput::Number(0.125)),
            CellWrite::new("A3", CellInput::SharedText("Europe".into())),
            CellWrite::new("B3", CellInput::Number(980_000.0)),
            CellWrite::new("C3", CellInput::Number(0.061)),
            CellWrite::new("A4", CellInput::SharedText("Asia Pacific".into())),
            CellWrite::new("B4", CellInput::Number(1_410_000.0)),
            CellWrite::new("C4", CellInput::Number(0.198)),
            CellWrite::new("A5", CellInput::SharedText("Audited".into())),
            CellWrite::new("B5", CellInput::Boolean(false)),
        ],
    )?;
    workbook.write_cells(
        notes,
        &[CellWrite::new(
            "A1",
            CellInput::InlineText("Figures are unaudited and subject to revision.".into()),
        )],
    )?;

    // ---- A style, built once and pointed at ----------------------------------------------------
    // The four `xl/styles.xml` tables are appended to, never deduplicated: an index handed back here
    // stays valid for the life of the workbook.
    let font = workbook.append_font(&FontProperties {
        font_name: Some("Calibri".into()),
        bold: Some(true),
        size_in_points: Some(12.0),
        color: Some(Color::from_opaque_rgb("FFFFFF")),
        ..FontProperties::default()
    })?;
    let fill = workbook.append_pattern_fill(&PatternFillSpec {
        pattern: Some(SpreadsheetPatternType::Solid),
        foreground: Some(Color::from_opaque_rgb("1F3864")),
        background: None,
    })?;
    let border = workbook.append_border(&BorderSpec {
        bottom: Some(BorderEdgeSpec::styled(BorderStyle::Medium)),
        ..BorderSpec::default()
    })?;
    let heading = workbook.append_cell_format(
        CellFormatTarget::CellFormats,
        &CellFormatSpec {
            font_index: Some(font),
            fill_index: Some(fill),
            border_index: Some(border),
            applies_font: Some(true),
            applies_fill: Some(true),
            applies_border: Some(true),
            ..CellFormatSpec::skeleton_cell_format()
        },
    )?;
    for heading_cell in ["A1", "B1", "C1"] {
        workbook.set_cell_style(0, heading_cell, Some(heading))?;
    }

    // ---- Geometry ------------------------------------------------------------------------------
    // Rows are one-based, as `row@r` is; columns are zero-based, as `A` is column 0.
    workbook.set_row_height(0, 1, Some(22.0), true)?;
    workbook.set_column_width(0, 0, 0, Some(22.0), true)?;
    workbook.set_column_width(0, 1, 2, Some(14.0), true)?;

    // ---- A hyperlink ---------------------------------------------------------------------------
    // The entry and its `External` relationship are written together; neither half exists alone.
    workbook.set_cell_hyperlink_url(0, "A2", "https://example.org/north-america")?;

    // ---- Save ----------------------------------------------------------------------------------
    workbook.validate()?;
    let bytes = workbook.save()?;
    std::fs::write(&out, &bytes)?;
    println!("wrote {} bytes to {}", bytes.len(), out.display());

    // ---- Reopen, to prove the bytes are a real workbook -----------------------------------------
    let reopened = Workbook::open(&bytes)?;
    assert_eq!(reopened.sheet_count(), 2);
    assert_eq!(reopened.sheets()[0].name, "Summary");
    assert_eq!(reopened.used_range(0)?.as_deref(), Some("A1:C5"));

    // One read, every cell — the mirror image of the one write above.
    let block = reopened.read_range(0, "A1:C5")?;
    assert_eq!(block.value(0, 0)?.text(), Some("Region"));
    assert_eq!(block.value(1, 0)?.text(), Some("North America"));
    assert_eq!(block.value(1, 1)?.number(), Some(1_250_000.0));
    assert_eq!(block.value(3, 2)?.number(), Some(0.198));
    assert_eq!(block.value(4, 1)?.boolean(), Some(false));

    assert_eq!(
        reopened.read_range(notes, "A1")?.value(0, 0)?.text(),
        Some("Figures are unaudited and subject to revision.")
    );
    assert_eq!(
        reopened
            .cell_hyperlink(0, "A2")?
            .and_then(|link| link.target),
        Some("https://example.org/north-america".to_owned())
    );
    assert_eq!(
        reopened
            .effective_cell_format(0, "B1")?
            .map(|f| f.style_index()),
        Some(heading)
    );

    Ok(())
}
