//! **No layout path panics**, held two ways: by scanning the source for the constructs that would,
//! and by laying out worksheets that say things no well-formed file says.
//!
//! A worksheet comes from an untrusted file, and `CLAUDE.md`'s rule is that library code returns
//! typed errors rather than unwrapping. The scan is the strong half — it catches a panic on a path
//! no fixture happens to reach — and the adversarial sheets are the half that proves the scan is
//! measuring something real.

mod support;

use mjx_layout::BoxModel;

use support::{grid_from, model, styles, viewport};

/// The constructs that panic, as they appear in code rather than in prose.
const PANICKING: &[&str] = &[
    ".unwrap()",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "assert!(",
    "assert_eq!(",
];

fn source_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|suffix| suffix == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn no_source_line_can_panic() {
    let mut offences = Vec::new();
    for file in rust_files(&source_root()) {
        let text = std::fs::read_to_string(&file).expect("a readable source file");
        for (number, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            for construct in PANICKING {
                if line.contains(construct) {
                    offences.push(format!(
                        "{}:{}: `{construct}`\n    {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        offences.is_empty(),
        "a worksheet comes from an untrusted file, so no layout path may panic:\n{}",
        offences.join("\n")
    );
}

/// A sheet that says something no well-formed file says, laid out at a normal viewport.
fn survives(name: &str, body: &str) {
    let grid = grid_from(body, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    // Every one of these must produce a page rather than a crash. A page that looks wrong is the
    // correct outcome; a page that does not exist is not.
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let _ = model.estimate_extent(&grid, &constraints);
    println!("{name}: laid out");
}

#[test]
fn a_col_run_whose_bounds_are_inverted_still_lays_out() {
    survives(
        "an inverted col run",
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<cols><col min="9" max="3" width="12"/></cols>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
    );
}

#[test]
fn a_col_run_at_column_zero_still_lays_out() {
    // `@min="0"` names no column: the wire is one-based and `A` is 1.
    survives(
        "a col run at zero",
        r#"<sheetFormatPr defaultRowHeight="15"/>
<cols><col min="0" max="0" width="12"/></cols>
<sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>"#,
    );
}

#[test]
fn negative_and_absurd_measurements_still_lay_out() {
    survives(
        "a negative row height and a colossal column width",
        r#"<sheetFormatPr defaultRowHeight="-4"/>
<cols><col min="1" max="3" width="1000000"/></cols>
<sheetData><row r="1" ht="-9" customHeight="true"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
    );
}

#[test]
fn a_sheet_with_no_format_properties_at_all_still_lays_out() {
    // `@defaultRowHeight` is `use="required"`, so a sheet with no `sheetFormatPr` is malformed — and
    // a renderer must still draw it.
    survives(
        "no sheetFormatPr",
        r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
    );
}

#[test]
fn overlapping_col_runs_still_lay_out() {
    survives(
        "two col runs claiming the same columns",
        r#"<sheetFormatPr defaultRowHeight="15"/>
<cols><col min="1" max="10" width="12"/><col min="5" max="8" width="30"/></cols>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
    );
}

#[test]
fn rows_out_of_order_and_a_duplicated_row_still_lay_out() {
    survives(
        "rows the file wrote in the wrong order",
        r#"<sheetFormatPr defaultRowHeight="15"/>
<sheetData>
<row r="5"><c r="A5" t="inlineStr"><is><t>five</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>two</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>two again</t></is></c></row>
</sheetData>"#,
    );
}

#[test]
fn a_pane_frozen_past_the_edge_of_the_grid_still_lays_out() {
    survives(
        "a pane frozen at column forty thousand",
        r#"<sheetFormatPr defaultRowHeight="15"/>
<sheetViews><sheetView workbookViewId="0"><pane xSplit="40000" ySplit="2000000" state="frozen"/></sheetView></sheetViews>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
    );
}

#[test]
fn a_rotation_the_schema_does_not_allow_still_lays_out() {
    let odd = styles(
        &[
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1"><alignment textRotation="200" indent="60000"/></xf>"#,
        ],
        "",
    );
    let grid = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15"/>
<sheetData><row r="1"><c r="A1" s="1" t="inlineStr"><is><t>rotated</t></is></c></row></sheetData>"#,
        &odd,
    );
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
}

#[test]
fn a_viewport_with_no_height_is_an_error_rather_than_a_hang() {
    use mjx_layout::{Constraints, LayoutRect, LayoutSize, PageIndex};
    use mjx_ooxml_core::measure::Emu;

    let grid = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15"/>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    let mut constraints = Constraints::single_column(
        LayoutSize {
            width: Emu::from_inches(6.0),
            height: Emu::from_inches(3.0),
        },
        Emu::ZERO,
    );
    constraints.content =
        LayoutRect::from_edges(Emu::ZERO, Emu::ZERO, Emu::from_inches(6.0), Emu::ZERO);
    let mut model = model();
    let error = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect_err("a zero-height band is refused rather than looped over");
    assert!(
        matches!(error, mjx_layout_xlsx::SheetLayoutError::Layout(_)),
        "{error}"
    );
}
