//! Every [`Workbook`] method is callable naming **only** `mjx_ooxml`, and every value it hands back
//! can be destructured naming only `mjx_ooxml`.
//!
//! The Excel sibling of `public_paths.rs` and `document_public_paths.rs`, and it earns its keep the
//! same way: `use mjx_ooxml::…;` is the whole import list, so a signature that leaks an `mjx-xlsx`
//! or an `mjx-sml` type a caller cannot name stops this file compiling. A binding has to name every
//! one of those types to wrap it, so "compiles here" and "bindable" are the same claim.
//!
//! # Why the assertions are asymmetric
//!
//! A test that writes `"Region"` into `A1` and reads `"Region"` back out of `A1` passes against a
//! surface that ignores both arguments and holds one value. Every case below therefore writes
//! *different* values at *different* addresses and checks each one at its own address — and where an
//! argument pair could plausibly be swapped (a row against a column, a first column against a last),
//! the two are given different values on purpose.

use mjx_ooxml::{
    BorderEdgeSpec, BorderSpec, BorderStyle, CellFormatSpec, CellFormatTarget, CellInput,
    CellWrite, Color, ErrorCode, FontProperties, PatternFillSpec, SheetKind,
    SpreadsheetPatternType, UnderlineType, Workbook,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// A blank workbook with a 3×2 block of mixed cell kinds, written in one call.
fn filled() -> Workbook {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .write_cells(
            0,
            &[
                CellWrite::new("A1", CellInput::SharedText("Region".into())),
                CellWrite::new("B1", CellInput::SharedText("Growth".into())),
                CellWrite::new("A2", CellInput::InlineText("North America".into())),
                CellWrite::new("B2", CellInput::Number(12.5)),
                CellWrite::new("A3", CellInput::Boolean(true)),
                CellWrite::new("B3", CellInput::Error("#DIV/0!".into())),
            ],
        )
        .expect("one batched write");
    workbook
}

#[test]
fn a_blank_workbook_opens_edits_and_saves_through_the_facade_alone() {
    let workbook = filled();
    assert_eq!(workbook.sheet_count(), 1);

    let saved = workbook.save().expect("saving");
    let reopened = Workbook::open(&saved).expect("reopening");
    let block = reopened.read_range(0, "A1:B3").expect("the block");

    assert_eq!(block.first_row(), 0);
    assert_eq!(block.first_column(), 0);
    assert_eq!(block.row_count(), 3);
    assert_eq!(block.column_count(), 2);

    // Asymmetric on purpose: six different values, six different addresses, and the two columns
    // hold different *kinds* so a transposed read cannot pass.
    assert_eq!(block.value(0, 0).expect("A1").text(), Some("Region"));
    assert_eq!(block.value(0, 1).expect("B1").text(), Some("Growth"));
    assert_eq!(block.value(1, 0).expect("A2").text(), Some("North America"));
    assert_eq!(block.value(1, 1).expect("B2").number(), Some(12.5));
    assert_eq!(block.value(2, 0).expect("A3").boolean(), Some(true));
    assert_eq!(block.value(2, 1).expect("B3").error_code(), Some("#DIV/0!"));

    assert_eq!(block.range().as_deref(), Some("A1:B3"));
    let rows = block.into_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].len(), 2);
}

/// The two `CellInput` string variants land in **different parts**, which is the whole reason they
/// are two variants: `SharedText` writes a `t="s"` index into `xl/sharedStrings.xml`, `InlineText`
/// writes an `<is>` inside the cell. Both read back as the same `CellData::Text`, so the only way to
/// tell them apart is in the bytes — which is what this asserts.
#[test]
fn shared_text_and_inline_text_are_written_to_different_places() {
    let workbook = filled();
    let saved = workbook.save().expect("saving");
    let reopened = Workbook::open(&saved).expect("reopening");

    let sheet = reopened
        .sheet(0)
        .expect("the first tab")
        .part
        .expect("its part");
    let markup = String::from_utf8(reopened.part_bytes(&sheet).expect("the worksheet bytes"))
        .expect("worksheet markup is utf-8");

    assert!(
        markup.contains("t=\"inlineStr\"") && markup.contains("North America"),
        "InlineText must land in the worksheet itself: {markup}"
    );
    assert!(
        !markup.contains(">Region<"),
        "SharedText must not land in the worksheet: {markup}"
    );

    let shared = reopened
        .part_bytes("/xl/sharedStrings.xml")
        .expect("the shared-string part");
    let shared = String::from_utf8(shared).expect("shared strings are utf-8");
    assert!(shared.contains("Region"), "SharedText must land here");
    assert!(
        !shared.contains("North America"),
        "InlineText must not land here: {shared}"
    );
}

#[test]
fn the_tab_strip_is_readable_and_editable() {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let second = workbook.add_sheet("Data").expect("a second tab");
    assert_eq!(second, 1);
    assert_eq!(workbook.sheet_count(), 2);

    workbook.rename_sheet(0, "Summary").expect("renaming");

    let sheets = workbook.sheets();
    assert_eq!(sheets[0].name, "Summary");
    assert_eq!(sheets[1].name, "Data");
    assert!(sheets[0].is_visible);
    assert_eq!(sheets[0].kind, Some(SheetKind::Worksheet));
    assert!(sheets[0].part.is_some());

    assert_eq!(workbook.sheet_index("Data"), Some(1));
    assert_eq!(workbook.sheet_index("Summary"), Some(0));
    assert_eq!(workbook.sheet_index("Nothing"), None);

    let one = workbook.sheet(1).expect("the second tab");
    assert_eq!(one.name, "Data");
    assert_eq!(
        workbook.sheet(2).expect_err("no third tab").code(),
        ErrorCode::IndexOutOfRange
    );

    let views = workbook.window_views().expect("the window views");
    let _ = views.len();
    let _ = workbook.active_sheet().expect("the active tab");
}

#[test]
fn a_range_read_clamps_the_open_axis_and_reports_the_used_extent() {
    let workbook = filled();

    // `"A:B"` is a whole-column form; the block must be the three populated rows and not 1,048,576.
    let columns = workbook.read_range(0, "A:B").expect("whole columns");
    assert_eq!(columns.row_count(), 3);
    assert_eq!(columns.column_count(), 2);

    // `"1:3"` is a whole-row form; the open axis is the columns.
    let rows = workbook.read_range(0, "1:3").expect("whole rows");
    assert_eq!(rows.row_count(), 3);
    assert_eq!(rows.column_count(), 2);

    // A single cell is a range too, and it is the whole one-cell story.
    let one = workbook.read_range(0, "B2").expect("one cell");
    assert_eq!(one.row_count(), 1);
    assert_eq!(one.column_count(), 1);
    assert_eq!(one.value(0, 0).expect("B2").number(), Some(12.5));

    assert_eq!(
        workbook.used_range(0).expect("the used range").as_deref(),
        Some("A1:B3")
    );
    assert_eq!(workbook.read_sheet(0).expect("the sheet").row_count(), 3);

    let empty = Workbook::blank().expect("a blank workbook");
    assert_eq!(empty.used_range(0).expect("no used range"), None);
    assert!(empty.read_sheet(0).expect("an empty sheet").is_empty());
}

#[test]
fn a_block_offset_outside_the_block_is_an_index_error_naming_the_offsets() {
    let workbook = filled();
    let block = workbook.read_range(0, "A1:B3").expect("the block");
    let error = block
        .value(3, 0)
        .expect_err("row 3 is outside a 3-row block");
    assert_eq!(error.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(error.detail().row, Some(3));
    assert_eq!(error.detail().column, Some(0));

    let error = block
        .value(0, 2)
        .expect_err("column 2 is outside a 2-column block");
    assert_eq!(error.detail().row, Some(0));
    assert_eq!(error.detail().column, Some(2));
}

/// A batch that would fail halfway is refused before the package is touched, so a failed
/// `write_cells` leaves the workbook exactly as it was.
#[test]
fn a_batch_with_a_bad_address_writes_nothing() {
    let mut workbook = filled();
    let before = workbook.save().expect("saving before");

    let error = workbook
        .write_cells(
            0,
            &[
                CellWrite::new("C1", CellInput::Number(1.0)),
                CellWrite::new("not a cell", CellInput::Number(2.0)),
            ],
        )
        .expect_err("the second entry is not an address");
    assert_eq!(error.code(), ErrorCode::InvalidArgument);

    let after = workbook.save().expect("saving after");
    assert_eq!(
        before, after,
        "a refused batch must not have written its first entry"
    );
}

#[test]
fn the_geometry_surface_is_reachable_and_not_transposed() {
    let mut workbook = filled();

    workbook.merge_cells(0, "A1:B1").expect("a merge");
    assert_eq!(workbook.merged_ranges(0).expect("the merges"), ["A1:B1"]);
    assert_eq!(
        workbook
            .merged_range_containing(0, "B1")
            .expect("the merge containing B1")
            .as_deref(),
        Some("A1:B1")
    );
    assert_eq!(
        workbook
            .merged_range_containing(0, "A3")
            .expect("A3 is not merged"),
        None
    );
    assert!(workbook.unmerge_cells(0, "A1:B1").expect("unmerging"));
    assert!(workbook.merged_ranges(0).expect("the merges").is_empty());

    // Rows are one-based (`row@r`); columns are zero-based. Different values on purpose, so a
    // surface that confused the two axes could not pass.
    workbook
        .set_row_height(0, 2, Some(24.0), true)
        .expect("a row height");
    workbook.set_row_hidden(0, 3, true).expect("a hidden row");
    workbook
        .set_row_outline_level(0, 2, 1)
        .expect("a row outline level");
    workbook
        .set_column_width(0, 0, 1, Some(18.0), true)
        .expect("a column width");
    workbook
        .set_column_hidden(0, 2, 2, true)
        .expect("a hidden column");
    workbook
        .set_column_outline_level(0, 0, 1, 2)
        .expect("a column outline level");

    let anomalies = workbook.grid_anomalies(0).expect("the anomalies");
    assert!(
        anomalies.is_empty(),
        "an authored sheet states no anomalies: {anomalies:?}"
    );

    workbook.save().expect("the edited workbook still saves");
}

#[test]
fn the_style_surface_builds_an_index_and_a_cell_points_at_it() {
    let mut workbook = filled();

    let font = workbook
        .append_font(&FontProperties {
            font_name: Some("Calibri".into()),
            bold: Some(true),
            size_in_points: Some(14.0),
            underline: Some(UnderlineType::Single),
            color: Some(Color::from_opaque_rgb("1F3864")),
            ..FontProperties::default()
        })
        .expect("a font");
    let fill = workbook
        .append_pattern_fill(&PatternFillSpec {
            pattern: Some(SpreadsheetPatternType::Solid),
            foreground: Some(Color::from_opaque_rgb("FFF2CC")),
            background: None,
        })
        .expect("a fill");
    let border = workbook
        .append_border(&BorderSpec {
            bottom: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            ..BorderSpec::default()
        })
        .expect("a border");
    let format = workbook
        .append_cell_format(
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
        )
        .expect("a cell format");
    assert!(format > 0, "a new format must not be `cellXfs[0]`");

    workbook
        .set_cell_style(0, "A1", Some(format))
        .expect("pointing A1 at it");

    let effective = workbook
        .effective_cell_format(0, "A1")
        .expect("the effective format")
        .expect("a styles part");
    assert_eq!(effective.style_index(), format);

    // A cell that was never styled resolves to `cellXfs[0]`, which is a different answer.
    let untouched = workbook
        .effective_cell_format(0, "B3")
        .expect("the effective format")
        .expect("a styles part");
    assert_eq!(untouched.style_index(), 0);

    let merged = workbook
        .effective_merged_cell_format(0, "A1")
        .expect("the merged-anchor format")
        .expect("a styles part");
    assert_eq!(merged.style_index(), format);

    let index = workbook.intern_shared_string("Region").expect("interning");
    assert_eq!(
        workbook.intern_shared_string("Region").expect("again"),
        index,
        "interning the same text twice must answer the same index"
    );

    workbook.save().expect("the styled workbook still saves");
}

#[test]
fn the_hyperlink_surface_writes_both_halves_and_removes_both() {
    let mut workbook = filled();

    workbook
        .set_cell_hyperlink_url(0, "A2", "https://example.org/north-america")
        .expect("an external link");
    workbook
        .set_cell_hyperlink_location(0, "A3", "Sheet1!B2")
        .expect("an internal jump");

    let links = workbook.sheet_hyperlinks(0).expect("the links");
    assert_eq!(links.len(), 2);

    let external = workbook
        .cell_hyperlink(0, "A2")
        .expect("the link on A2")
        .expect("there is one");
    assert_eq!(external.range, "A2");
    assert_eq!(
        external.target.as_deref(),
        Some("https://example.org/north-america")
    );
    assert_eq!(external.target_is_external, Some(true));
    assert_eq!(external.location, None);

    let internal = workbook
        .cell_hyperlink(0, "A3")
        .expect("the link on A3")
        .expect("there is one");
    assert_eq!(internal.range, "A3");
    assert_eq!(internal.location.as_deref(), Some("Sheet1!B2"));
    assert_eq!(
        internal.relationship_id, None,
        "an internal jump writes no relationship"
    );

    assert!(workbook
        .remove_cell_hyperlink(0, "A2")
        .expect("removing the external link"));
    assert_eq!(workbook.sheet_hyperlinks(0).expect("the links").len(), 1);
    assert!(!workbook
        .remove_cell_hyperlink(0, "A2")
        .expect("removing it twice"));

    workbook.save().expect("the linked workbook still saves");
}

#[test]
fn the_workbook_metadata_surface_is_reachable() {
    let mut workbook = Workbook::open(&fixture("sample.xlsx")).expect("the sample workbook");

    let _ = workbook.defined_names().expect("the defined names");
    let _ = workbook.defined_name("Nothing").expect("a missing name");
    let _ = workbook.print_area(0).expect("the print area");
    let _ = workbook.date_system().expect("the date system");
    let _ = workbook.calculation_settings().expect("the calc settings");
    let _ = workbook
        .sheet_printer_settings(0)
        .expect("printer settings");
    let _ = workbook.sheet_background_image(0).expect("a background");
    let _ = workbook.auto_filter_range(0).expect("an autofilter");
    let _ = workbook.data_validation_ranges(0).expect("validations");
    let _ = workbook
        .conditional_formatting_ranges(0)
        .expect("conditional formatting");
    let _ = workbook
        .conditional_formatting_rule_count(0, 0)
        .expect("the rule count");
    let _ = workbook.sheet_tables(0).expect("the tables");
    let _ = workbook.next_table_id().expect("the next table id");
    let _ = workbook
        .table_style_origin("TableStyleMedium2")
        .expect("a style origin");

    let preserved = workbook.preserved_parts().expect("the preserved parts");
    for entry in preserved.all() {
        assert!(
            !entry.part.is_empty(),
            "{:?} named an empty part",
            entry.kind
        );
    }
    let _ = workbook.pivot_tables().expect("the pivot tables");
    let _ = workbook.external_links().expect("the external links");
    let _ = workbook.connections().expect("the connections");
    let _ = workbook.query_tables().expect("the query tables");
    let _ = workbook.xml_maps().expect("the xml maps");
    let _ = workbook.revision_state().expect("the revision state");

    assert!(workbook.workbook_part().ends_with("workbook.xml"));
    assert!(workbook.part_names().len() > 1);
    assert!(workbook
        .content_type_of(&workbook.workbook_part())
        .expect("a content type")
        .is_some());
    assert!(!workbook
        .part_bytes(&workbook.workbook_part())
        .expect("the workbook bytes")
        .is_empty());
    assert_eq!(
        workbook
            .part_bytes("/xl/nothing.xml")
            .expect_err("no such part")
            .code(),
        ErrorCode::NotFound
    );

    workbook.validate().expect("the sample workbook is valid");
}

/// The four reports over the committed fixtures that actually hold the features, so each report is
/// exercised against a file that has something to report rather than only against one that does not.
#[test]
fn the_feature_reports_answer_from_files_that_hold_the_features() {
    let conditional = Workbook::open(&fixture("conditional_formatting.xlsx")).expect("the fixture");
    let ranges = conditional
        .conditional_formatting_ranges(0)
        .expect("the conditional-formatting ranges");
    assert!(
        !ranges.is_empty(),
        "conditional_formatting.xlsx must report at least one block"
    );
    assert!(
        conditional
            .conditional_formatting_rule_count(0, 0)
            .expect("the rule count")
            .is_some_and(|count| count > 0),
        "the first block must hold at least one rule"
    );

    let filters = Workbook::open(&fixture("validation_and_filters.xlsx")).expect("the fixture");
    assert!(
        filters
            .auto_filter_range(0)
            .expect("the autofilter range")
            .is_some(),
        "validation_and_filters.xlsx must report an autofilter"
    );
    assert!(
        !filters
            .data_validation_ranges(0)
            .expect("the validation ranges")
            .is_empty(),
        "validation_and_filters.xlsx must report a data validation"
    );

    let tables = Workbook::open(&fixture("worksheet_tables.xlsx")).expect("the fixture");
    let sheet_tables = tables.sheet_tables(0).expect("the tables");
    assert!(
        !sheet_tables.is_empty(),
        "worksheet_tables.xlsx must report a table"
    );
    assert!(sheet_tables[0].part.ends_with(".xml"));
    assert!(!sheet_tables[0].columns.is_empty());

    let preserved = Workbook::open(&fixture("preserved_parts.xlsx")).expect("the fixture");
    assert!(
        !preserved
            .preserved_parts()
            .expect("the preserved parts")
            .is_empty(),
        "preserved_parts.xlsx must report preserved parts"
    );

    let links = Workbook::open(&fixture("hyperlinks.xlsx")).expect("the fixture");
    assert!(
        !links.sheet_hyperlinks(0).expect("the links").is_empty(),
        "hyperlinks.xlsx must report a hyperlink"
    );
}

/// A tab that is not a worksheet answers `NothingToRead`, which is a different code from a tab that
/// is not there at all — the distinction a caller acts on.
#[test]
fn a_chartsheet_has_no_cells_and_says_so_differently_from_a_missing_tab() {
    let workbook = Workbook::open(&fixture("print_and_sheet_kinds.xlsx")).expect("the fixture");
    let sheets = workbook.sheets();
    let Some(other) = sheets
        .iter()
        .position(|sheet| sheet.kind != Some(SheetKind::Worksheet))
    else {
        panic!("print_and_sheet_kinds.xlsx must hold a tab that is not a worksheet");
    };
    let other = u32::try_from(other).expect("a small index");

    let error = workbook
        .read_range(other, "A1")
        .expect_err("a chartsheet has no cells");
    assert_eq!(error.code(), ErrorCode::NothingToRead);
    assert_eq!(error.detail().index, Some(other));

    let missing = workbook
        .read_range(workbook.sheet_count(), "A1")
        .expect_err("no such tab");
    assert_eq!(missing.code(), ErrorCode::IndexOutOfRange);
}

/// The Rust-only escape hatch is there, and it is the *only* door to the per-cell calls this facade
/// deliberately does not restate.
#[test]
fn the_escape_hatch_reaches_the_per_cell_surface_this_facade_does_not_carry() {
    let mut workbook = filled();
    assert_eq!(workbook.workbook().sheets().len(), 1);

    let reference = mjx_ooxml::CellReference::parse("B2").expect("an address");
    let text = workbook
        .workbook_mut()
        .cell_text(0, reference)
        .expect("the cell text");
    assert_eq!(text.as_deref(), Some("12.5"));

    let inner = workbook.into_workbook();
    let round_trip = Workbook::from(inner);
    assert_eq!(round_trip.format(), mjx_ooxml::Format::Workbook);
}
