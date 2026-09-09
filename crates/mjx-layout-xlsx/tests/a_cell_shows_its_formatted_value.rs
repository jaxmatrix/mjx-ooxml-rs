//! The engine, reached the way a reader reaches it: through a worksheet, a `numFmtId` and a band.
//!
//! # Why this suite is not the conformance table
//!
//! `tests/the_format_language_is_evaluated.rs` asks *does this code render this value correctly*.
//! This one asks the questions that only a real sheet can answer: does the `numFmtId` on the `xf`
//! actually reach the evaluator, does an id with **no `numFmt` element anywhere in the file** still
//! format (which is what "the built-in table" means in practice), does `[Red]` reach a decoration,
//! does the cache hit across a band, and does the workbook's own epoch reach the date.
//!
//! The last of those is the one a table cannot ask at all: `workbookPr@date1904` lives in a
//! different part from every cell it moves.

mod support;

use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_xlsx::DateSystem;

use support::{grid_from, model, styles, viewport};

/// The text every cell of band zero displayed, as `(row, column, text)`.
fn displayed(model: &mut SheetBoxModel, grid: &SheetGrid) -> Vec<(u32, u16, String)> {
    let constraints = viewport(8.0, 4.0);
    let _ = support::lay_out(model, grid, &constraints, 0);
    model
        .catalogue()
        .cells()
        .iter()
        .map(|report| (report.row, report.column, report.text.clone()))
        .collect()
}

/// What one cell displayed.
fn cell_text(model: &mut SheetBoxModel, grid: &SheetGrid, row: u32, column: u16) -> String {
    displayed(model, grid)
        .into_iter()
        .find(|(found_row, found_column, _)| *found_row == row && *found_column == column)
        .map(|(_, _, text)| text)
        .unwrap_or_default()
}

/// ⚠ The failure this child exists to end: a date shown as its serial.
///
/// `numFmtId="14"` writes **no `numFmt` element at all** — §18.8.30's table is in the specification,
/// not in the file — so a renderer with a short built-in table falls back to `General` here and
/// shows `45719`, which looks plausible and is the defect the ticket names.
#[test]
fn a_date_is_a_date_and_not_its_serial() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>45719</v></c></row>
</sheetData>"#;
    let grid = grid_from(
        sheet,
        &styles(
            &[r#"<xf numFmtId="14" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
            "",
        ),
    );
    let mut model = model();
    assert_eq!(cell_text(&mut model, &grid, 0, 0), "03-03-25");
}

/// A `numFmts` entry the workbook declares itself beats — and reaches — the built-in table.
#[test]
fn a_declared_format_code_reaches_the_evaluator() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>1234.5</v></c><c r="B1" s="1"><v>-1234.5</v></c></row>
</sheetData>"#;
    let styles_markup = declared_format(r#"&quot;$&quot;#,##0.00;[Red](&quot;$&quot;#,##0.00)"#);
    let grid = grid_from(sheet, &styles_markup);
    let mut model = model();
    let shown = displayed(&mut model, &grid);
    let text = |column: u16| {
        shown
            .iter()
            .find(|(_, found, _)| *found == column)
            .map(|(_, _, text)| text.clone())
            .unwrap_or_default()
    };
    assert_eq!(text(0), "$1,234.50");
    assert_eq!(text(1), "($1,234.50)");
}

/// `[Red]` reaches the decoration, and only the cell whose section named it.
///
/// The whole reason [`mjx_layout_xlsx::Decoration::text_colour`] exists: two cells with the **same**
/// effective format take different colours, because which section runs depends on the value.
#[test]
fn a_red_negative_does_not_share_its_neighbours_decoration() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>5</v></c><c r="B1" s="1"><v>-5</v></c><c r="C1" s="1"><v>7</v></c></row>
</sheetData>"#;
    let grid = grid_from(sheet, &declared_format("#,##0.00;[Red]#,##0.00"));
    let mut model = model();
    let constraints = viewport(8.0, 4.0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let catalogue = model.catalogue();
    let handle_of = |column: u16| {
        catalogue
            .cells()
            .iter()
            .find(|report| report.column == column)
            .map(|report| report.decoration)
            .expect("the cell was laid out")
    };
    let colour_of = |column: u16| {
        catalogue
            .decoration(handle_of(column))
            .and_then(|decoration| decoration.text_colour)
    };
    assert_eq!(colour_of(0), None, "the positive cell is not coloured");
    assert_eq!(colour_of(1), Some(2), "the negative cell is red");
    assert_eq!(colour_of(2), None, "and the next positive one is not");
    assert_ne!(
        handle_of(0),
        handle_of(1),
        "a coloured cell cannot share a decoration with an uncoloured one"
    );
    assert_eq!(
        handle_of(0),
        handle_of(2),
        "two cells with the same format and the same colour still share one"
    );
}

/// The workbook's own epoch reaches the date, from a part no cell mentions.
#[test]
fn the_workbook_states_which_epoch_its_dates_count_from() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>45719</v></c></row>
</sheetData>"#;
    let styles_markup = styles(
        &[r#"<xf numFmtId="14" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
        "",
    );
    let grid = grid_from(sheet, &styles_markup);
    assert_eq!(
        grid.date_system(),
        DateSystem::Windows1900,
        "a workbook with no `workbookPr` is on the schema default"
    );

    // The same serial, four years and a day apart, with nothing else changed.
    let macintosh = grid_from(sheet, &styles_markup).with_date_system(DateSystem::Macintosh1904);
    let mut windows = model();
    assert_eq!(cell_text(&mut windows, &grid, 0, 0), "03-03-25");
    let mut apple = model();
    assert_eq!(cell_text(&mut apple, &macintosh, 0, 0), "03-04-29");
}

/// The cache hits, measured rather than assumed.
///
/// A band of forty cells sharing two format codes must compile **two** codes plus `General`, and it
/// must render fewer values than it is asked for, because a column of repeated dates repeats.
#[test]
fn the_format_cache_hits_across_a_band() {
    let mut rows = String::new();
    for row in 1..=20 {
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" s="1"><v>45719</v></c><c r="B{row}" s="2"><v>1234.5</v></c></row>"#
        ));
    }
    let sheet = format!(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>{rows}</sheetData>"#
    );
    let styles_markup = styles(
        &[
            r#"<xf numFmtId="14" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#,
            r#"<xf numFmtId="4" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#,
        ],
        "",
    );
    let grid = grid_from(&sheet, &styles_markup);
    let mut model = model();
    let constraints = viewport(8.0, 6.0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);

    let formats = model.formats();
    assert!(
        formats.requests() >= 40,
        "the band asked for {} values",
        formats.requests()
    );
    assert_eq!(
        formats.compilations(),
        2,
        "two distinct codes on the band, compiled once each — not once per cell"
    );
    assert_eq!(
        formats.evaluations(),
        2,
        "two distinct (code, value) pairs, rendered once each"
    );
    assert!(
        formats.evaluations() < formats.requests(),
        "the result cache did nothing: {} evaluations for {} requests",
        formats.evaluations(),
        formats.requests()
    );
    assert_eq!(formats.compiled_count(), 2);
    assert_eq!(formats.result_count(), 2);
}

/// The cache survives a scroll, which is the whole reason it lives on the box model.
#[test]
fn the_cache_is_held_across_bands() {
    let mut rows = String::new();
    for row in 1..=200 {
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" s="1"><v>{row}</v></c></row>"#
        ));
    }
    let sheet = format!(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>{rows}</sheetData>"#
    );
    let grid = grid_from(
        &sheet,
        &styles(
            &[r#"<xf numFmtId="4" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
            "",
        ),
    );
    let mut model = model();
    let constraints = viewport(8.0, 2.0);
    for band in 0..4 {
        let _ = support::lay_out(&mut model, &grid, &constraints, band);
    }
    assert_eq!(
        model.formats().compilations(),
        1,
        "one code, compiled once, across four bands"
    );
    assert!(
        model.formats().requests() > model.formats().evaluations(),
        "the second visit to a band renders nothing again"
    );

    model.clear_format_cache();
    assert_eq!(model.formats().compilations(), 0);
    assert_eq!(model.formats().result_count(), 0);
}

/// A format that renders a value as nothing produces no glyphs, and does not stop an overflow.
///
/// `;;;` is how a person hides a column without hiding it. The cell is still **occupied** — that is
/// a question about the stored value — so its neighbour's text still stops at it.
#[test]
fn a_hidden_value_draws_nothing_and_still_blocks() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="8"/>
<sheetData>
<row r="1">
<c r="A1" t="inlineStr"><is><t>a very long label indeed</t></is></c>
<c r="B1" s="1"><v>1</v></c>
</row>
</sheetData>"#;
    let grid = grid_from(sheet, &declared_format(";;;"));
    let mut model = model();
    let constraints = viewport(8.0, 4.0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let catalogue = model.catalogue();
    let hidden = catalogue.cells().iter().find(|report| report.column == 1);
    assert!(
        hidden.is_none(),
        "a cell that renders as nothing produces no report and no glyphs"
    );
    let label = catalogue
        .cells()
        .iter()
        .find(|report| report.column == 0)
        .expect("the label was laid out");
    assert!(
        !matches!(label.overflow, mjx_layout_xlsx::Overflow::Spills { .. }),
        "the hidden cell still stops the overflow: {:?}",
        label.overflow
    );
}

/// A boolean ignores its number format, and an error shows its code.
#[test]
fn a_boolean_and_an_error_ignore_the_format() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1" t="b"><v>1</v></c><c r="B1" s="1" t="e"><v>#DIV/0!</v></c></row>
</sheetData>"#;
    let grid = grid_from(sheet, &declared_format("\"n=\"0.00"));
    let mut model = model();
    let shown = displayed(&mut model, &grid);
    let text = |column: u16| {
        shown
            .iter()
            .find(|(_, found, _)| *found == column)
            .map(|(_, _, text)| text.clone())
            .unwrap_or_default()
    };
    assert_eq!(text(0), "TRUE");
    assert_eq!(text(1), "#DIV/0!");
}

/// The committed corpus, formatted — every cell whose display text is not what the file stored.
///
/// The rows here were **not** authored for this suite: they are what four workbooks in
/// `tests/fixtures/` already say, and the expectations were read off the corpus once and then
/// written down. That makes them a change detector over real files rather than over a fixture built
/// to agree with the engine, which is the closest this child gets to an outside opinion.
///
/// The fourth one is the interesting one. `[$-409]#,##0.00"  USD"\;;[Red]-#,##0.00` carries an
/// **escaped** semicolon inside its first section and a real one after it, so it is a two-section
/// format and not a three-section one — and a splitter that did not honour `\` would put the
/// currency label in a section no positive value ever reaches.
#[test]
fn the_committed_corpus_formats_what_it_stores() {
    let expectations: &[(&str, u32, u16, &str, &str)] = &[
        // file, row, column, stored, displayed
        ("sheet_grid.xlsx", 1, 1, "1200", "1200.00"),
        ("sheet_grid.xlsx", 9, 3, "7", "7.00"),
        ("effective_cell_format.xlsx", 0, 0, "1", "100.000%"),
        ("effective_cell_format.xlsx", 0, 1, "2", "2.00  USD;"),
        ("effective_cell_format.xlsx", 1, 2, "7", "7.00  USD;"),
        ("style_resources.xlsx", 0, 0, "42", "42.000m"),
        // A formula cell whose cached value is a boolean: the format is `General` and the value
        // ignores it, which is why `1` reads as `TRUE`.
        ("formulas.xlsx", 5, 6, "1", "TRUE"),
    ];
    for (file, row, column, stored, displayed) in expectations {
        let grid = support::grid_of(file, 0);
        let mut model = model();
        let constraints = viewport(8.0, 5.0);
        let _ = support::lay_out(&mut model, &grid, &constraints, 0);
        let raw = grid
            .cell(*row, *column)
            .and_then(|cell| grid.cell_text(&cell))
            .unwrap_or_default();
        assert_eq!(
            &raw, stored,
            "{file} r{row} c{column}: the fixture no longer stores what this suite was written against"
        );
        let shown = model
            .catalogue()
            .cells()
            .iter()
            .find(|report| report.row == *row && report.column == *column)
            .map(|report| report.text.clone())
            .unwrap_or_default();
        assert_eq!(&shown, displayed, "{file} r{row} c{column}");
    }
}

/// `sample.xlsx` and `shared_strings_rich_text.xlsx` are all text and must be untouched.
///
/// The other half of the corpus assertion: a format engine that changed a shared string would be
/// visible everywhere, and this is what says it does not.
#[test]
fn a_sheet_of_text_is_not_reformatted() {
    for file in ["sample.xlsx", "shared_strings_rich_text.xlsx"] {
        let grid = support::grid_of(file, 0);
        let mut model = model();
        let constraints = viewport(8.0, 5.0);
        let _ = support::lay_out(&mut model, &grid, &constraints, 0);
        for report in model.catalogue().cells() {
            let raw = grid
                .cell(report.row, report.column)
                .and_then(|cell| grid.cell_text(&cell))
                .unwrap_or_default();
            assert_eq!(
                raw, report.text,
                "{file} r{} c{}: a text cell was reformatted",
                report.row, report.column
            );
        }
    }
}

/// The locale-dependent half of §18.8.30, which no file can answer for.
///
/// Ids 27–36, 50–58 and the Thai block have a **different** format code per UI language: id 30 is
/// `m/d/yy` in `zh-tw`, `m-d-yy` in `zh-cn` and `mm-dd-yy` in `ko-kr`. Nothing in a `.xlsx` states
/// which language a consumer is running in, so `mjx_sml::builtin_format_code` answers `None` for
/// every one of them and this crate resolves them to `General` — visibly wrong, honestly wrong, and
/// fixed by a *host* saying which language it is.
///
/// The three answers below are three different strings for one `numFmtId` and one serial, which is
/// what says the language reached the resolver rather than being ignored.
#[test]
fn a_locale_dependent_builtin_id_needs_a_language_and_says_so() {
    let sheet = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="18"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>45719</v></c></row>
</sheetData>"#;
    let styles_markup = styles(
        &[r#"<xf numFmtId="30" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
        "",
    );

    let plain = grid_from(sheet, &styles_markup);
    assert_eq!(plain.number_format_language(), None);
    let mut unlocalised = model();
    assert_eq!(
        cell_text(&mut unlocalised, &plain, 0, 0),
        "45719",
        "with no UI language, id 30 has no code and the cell falls back to `General`"
    );

    let korean = grid_from(sheet, &styles_markup)
        .with_number_format_language(Some(mjx_sml::NumberFormatLanguage::Korean));
    let mut in_korean = model();
    assert_eq!(
        cell_text(&mut in_korean, &korean, 0, 0),
        "03-03-25",
        "ko-kr: mm-dd-yy"
    );

    let chinese = grid_from(sheet, &styles_markup)
        .with_number_format_language(Some(mjx_sml::NumberFormatLanguage::ChineseChina));
    let mut in_chinese = model();
    assert_eq!(
        cell_text(&mut in_chinese, &chinese, 0, 0),
        "3-3-25",
        "zh-cn: m-d-yy"
    );
}

/// A `styles.xml` whose only `xf` beyond the default names a declared `numFmt` with `code`.
fn declared_format(code: &str) -> Vec<u8> {
    let base = styles(
        &[r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
        "",
    );
    let text = String::from_utf8(base).expect("the support markup is UTF-8");
    let numfmts =
        format!(r#"<numFmts count="1"><numFmt numFmtId="164" formatCode="{code}"/></numFmts>"#);
    text.replace(
        "<fonts count=\"1\">",
        &format!("{numfmts}<fonts count=\"1\">"),
    )
    .into_bytes()
}
