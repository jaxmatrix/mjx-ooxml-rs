//! Table properties, table styles and conditional formatting (MJXOFF-119).
//!
//! Every fixture here is built through this crate's own public API (`Document::blank`,
//! `edit_style_sheet`, `append_table`, `edit_table`/`edit_cell`) — no committed Word fixture in this
//! workspace carries a table style's conditional formatting before this file, the same "authored, not
//! templated" approach MJXOFF-116's `ragged_table.docx` and MJXOFF-113's `header_watermark.docx` used
//! for their own new markup shapes.

use mjx_docx::{
    CellBorderEdge, CellProperties, ConditionalFormatRegion, Document, PageSize, Row,
    RowProperties, Shading, StyleDefinition, TableAlignment, TableExceptionProperties, TableLook,
    TableProperties, TableStyleOverride, TableWidth, TableWidthMeasure,
};
use mjx_ooxml_core::Interner;
use mjx_ooxml_types::wordprocessingml::{
    HexadecimalColor, ShadingPattern, StyleType, TableJustification, TableWidthUnit,
};
use mjx_opc::{Package, PartName};

/// A `w:shd` of `pattern="clear"` and the given fill colour.
fn shading(interner: &mut Interner, fill: &str) -> Shading {
    let mut shading = Shading::new(interner, ShadingPattern::Clear);
    shading.set_fill_color(interner, Some(HexadecimalColor::from_wire(fill)));
    shading
}

// -------------------------------------------------------------------------------------------
// The four-answer conditional-formatting fixture: a table style whose `wholeTable`, `firstRow`,
// `band1Horz`, `band2Horz` and `topLeftCell` regions each shade differently, and a `w:tblLook`
// enabling `firstRow`/`firstColumn`/horizontal banding.
// -------------------------------------------------------------------------------------------

const WHOLE_TABLE_FILL: &str = "FFFFFF";
const FIRST_ROW_FILL: &str = "FF0000";
const BAND1_FILL: &str = "00FF00";
const BAND2_FILL: &str = "0000FF";
const TOP_LEFT_FILL: &str = "FFFF00";

/// Builds a document with one table style (`"Banded"`) declaring five conditionally-formatted
/// regions with five distinct fill colours, and a 4-row x 3-column table referencing it. `tblLook`
/// enables `firstRow`, `firstColumn` and horizontal banding, and disables vertical banding (so a
/// column-0 cell's own `firstColumn` region is the only column-direction effect in play — nothing
/// here also exercises column banding, which would otherwise compound with the row direction and
/// make the fixture's own geometry harder to reason about).
fn four_region_document() -> Document {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");

    document
        .edit_style_sheet(|sheet, interner| {
            let mut style = StyleDefinition::new(interner, StyleType::Table, "Banded");

            let mut whole_table_properties = TableProperties::new(interner);
            whole_table_properties.set_shading(Some(shading(interner, WHOLE_TABLE_FILL)));
            style.set_table_properties(Some(whole_table_properties));

            for (region, fill) in [
                (ConditionalFormatRegion::FirstRow, FIRST_ROW_FILL),
                (ConditionalFormatRegion::Band1Horizontal, BAND1_FILL),
                (ConditionalFormatRegion::Band2Horizontal, BAND2_FILL),
                (ConditionalFormatRegion::TopLeftCell, TOP_LEFT_FILL),
            ] {
                let mut override_ = TableStyleOverride::new(interner, region);
                let mut cell_properties = CellProperties::new(interner);
                cell_properties.set_shading(Some(shading(interner, fill)));
                override_.set_cell_properties(Some(cell_properties));
                style.push_table_style_override(override_);
            }

            sheet.add_style(style);
        })
        .expect("edit style sheet");

    let table = document.append_table(4, 3).expect("append 4x3 table");
    document
        .edit_table(table, |table, interner| {
            let properties = table
                .properties_mut()
                .expect("Table::new always writes a w:tblPr");
            properties.set_style_id(interner, Some("Banded"));
            let mut look = TableLook::new(interner);
            look.set_first_row(interner, Some(true));
            look.set_first_column(interner, Some(true));
            look.set_no_horizontal_band(interner, Some(false));
            look.set_no_vertical_band(interner, Some(true));
            properties.set_look(Some(look));
        })
        .expect("set table style and tblLook");

    document
}

/// Would this pass if `w:tblStylePr` were never consulted? No — a resolver that only ever reads the
/// cell's own (absent) `w:tcPr/w:shd` would answer `None` for all four cells; a resolver that reads
/// only the style's own base `w:tblPr/w:shd` would answer `Some(WHOLE_TABLE_FILL)` for all four,
/// never four distinct colours. **Proved by mutation**: temporarily replacing `fill_tier`'s call to
/// `fold_overwrite` with `Ok(None)` (so no region ever contributes) turns this test red — every one
/// of the four assertions below fails, each expecting the whole-table fallback rather than its own
/// region's colour:
///
/// ```text
/// ---- four_regions_of_a_table_style_resolve_to_four_different_fills stdout ----
/// thread 'four_regions_of_a_table_style_resolve_to_four_different_fills' panicked:
/// assertion `left == right` failed: the first-row cell must show the firstRow region's own fill
///   left: None
///  right: Some("FF0000")
/// ```
///
/// (and three further failures for the band-1, band-2 and corner cells) — restored by re-editing.
#[test]
fn four_regions_of_a_table_style_resolve_to_four_different_fills() {
    let mut document = four_region_document();

    let top_left = document
        .effective_cell_fill(0, 0, 0)
        .expect("top-left cell")
        .map(|fill| fill.fill.map(hex));
    let first_row = document
        .effective_cell_fill(0, 0, 1)
        .expect("first-row, non-corner cell")
        .map(|fill| fill.fill.map(hex));
    let band1 = document
        .effective_cell_fill(0, 1, 1)
        .expect("first band-1 data cell")
        .map(|fill| fill.fill.map(hex));
    let band2 = document
        .effective_cell_fill(0, 2, 1)
        .expect("first band-2 data cell")
        .map(|fill| fill.fill.map(hex));

    assert_eq!(
        top_left,
        Some(Some(TOP_LEFT_FILL.to_owned())),
        "the top-left cell must show the topLeftCell region's own fill"
    );
    assert_eq!(
        first_row,
        Some(Some(FIRST_ROW_FILL.to_owned())),
        "the first-row cell must show the firstRow region's own fill"
    );
    assert_eq!(
        band1,
        Some(Some(BAND1_FILL.to_owned())),
        "the first data row must show the band1Horz region's own fill"
    );
    assert_eq!(
        band2,
        Some(Some(BAND2_FILL.to_owned())),
        "the second data row must show the band2Horz region's own fill"
    );

    // All four answers are genuinely different from one another and from the whole-table default —
    // the fixture's own point ("would exercise no conditional formatting at all" is exactly what
    // this refutes).
    let answers = [&top_left, &first_row, &band1, &band2];
    for (index, a) in answers.iter().enumerate() {
        for b in &answers[index + 1..] {
            assert_ne!(a, b, "the four regions must resolve to four distinct fills");
        }
    }
}

/// Flipping `w:tblLook/@firstRow` off changes the first-row cell's answer: with the flag off, row 0
/// is no longer excluded from banding, so it becomes an ordinary data row (band 1) instead of the
/// `firstRow` region. Would this pass if `w:tblLook` were ignored? No — an implementation that always
/// treats row 0 as the header (regardless of the flag) would keep answering `FIRST_ROW_FILL`.
#[test]
fn turning_off_first_row_in_tbllook_changes_exactly_the_first_row_cells() {
    let mut document = four_region_document();

    document
        .edit_table(0, |table, interner| {
            let properties = table.properties_mut().expect("w:tblPr");
            let mut look = TableLook::new(interner);
            look.set_first_row(interner, Some(false));
            look.set_first_column(interner, Some(true));
            look.set_no_horizontal_band(interner, Some(false));
            look.set_no_vertical_band(interner, Some(true));
            properties.set_look(Some(look));
        })
        .expect("turn off firstRow");

    let first_row_cell = document
        .effective_cell_fill(0, 0, 1)
        .expect("cell (0, 1)")
        .and_then(|fill| fill.fill)
        .map(hex);
    assert_eq!(
        first_row_cell,
        Some(BAND1_FILL.to_owned()),
        "with firstRow off, row 0 is an ordinary data row — band 1, not the firstRow fill"
    );

    // The (0, 0) cell changes too, and for the same reason: with `firstRow` off, it is no longer a
    // corner (a corner needs *both* edge flags — `topLeftCell` never applies without `firstRow`),
    // so it falls through to the region that *is* still defined for it — this fixture states no
    // `firstColumn` override at all, so column 0's own `firstColumn` region contributes nothing and
    // the band-1 fill underneath it shows through instead.
    let corner = document
        .effective_cell_fill(0, 0, 0)
        .expect("cell (0, 0)")
        .and_then(|fill| fill.fill)
        .map(hex);
    assert_eq!(
        corner,
        Some(BAND1_FILL.to_owned()),
        "without firstRow, (0, 0) is no longer a corner — band 1 shows through the undefined firstColumn region"
    );
}

fn hex(color: mjx_docx::EffectiveColor) -> String {
    match color {
        mjx_docx::EffectiveColor::Auto => "auto".to_owned(),
        mjx_docx::EffectiveColor::Hex(hex) => hex,
    }
}

// -------------------------------------------------------------------------------------------
// First row vs. first column: no corner override defined, so the two edge regions' own precedence
// decides — column must win, per ECMA-376 Part 1 §17.7.6.6's own stated order ("First row, last
// row" applied *before* "First column, last column").
// -------------------------------------------------------------------------------------------

/// Would this pass if the region-application order were reversed (row applied after column)? No —
/// it would then read the *row*'s fill at `(0, 0)`, not the column's. **Proved by mutation**:
/// swapping the push order of the `FirstRow`/`FirstColumn` entries in
/// `table_regions::applicable_regions` (so column is pushed before row) turns this red:
/// `left: Some("FF00FF") /* the row's own fill */, right: Some("00FFFF") /* expected: the column's */`.
/// Restored by re-editing.
#[test]
fn first_column_wins_over_first_row_when_both_regions_disagree() {
    const ROW_FILL: &str = "FF00FF";
    const COLUMN_FILL: &str = "00FFFF";

    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    document
        .edit_style_sheet(|sheet, interner| {
            let mut style = StyleDefinition::new(interner, StyleType::Table, "EdgeConflict");

            let mut row_override =
                TableStyleOverride::new(interner, ConditionalFormatRegion::FirstRow);
            let mut row_cell_properties = CellProperties::new(interner);
            row_cell_properties.set_shading(Some(shading(interner, ROW_FILL)));
            row_override.set_cell_properties(Some(row_cell_properties));
            style.push_table_style_override(row_override);

            let mut column_override =
                TableStyleOverride::new(interner, ConditionalFormatRegion::FirstColumn);
            let mut column_cell_properties = CellProperties::new(interner);
            column_cell_properties.set_shading(Some(shading(interner, COLUMN_FILL)));
            column_override.set_cell_properties(Some(column_cell_properties));
            style.push_table_style_override(column_override);

            // Deliberately no `topLeftCell` override — the point is that the edge regions'
            // *own* precedence, not a corner override, decides this cell's answer.
            sheet.add_style(style);
        })
        .expect("edit style sheet");

    let table = document.append_table(3, 3).expect("append 3x3 table");
    document
        .edit_table(table, |table, interner| {
            let properties = table.properties_mut().expect("w:tblPr");
            properties.set_style_id(interner, Some("EdgeConflict"));
            let mut look = TableLook::new(interner);
            look.set_first_row(interner, Some(true));
            look.set_first_column(interner, Some(true));
            look.set_no_horizontal_band(interner, Some(true));
            look.set_no_vertical_band(interner, Some(true));
            properties.set_look(Some(look));
        })
        .expect("set table style and tblLook");

    let corner = document
        .effective_cell_fill(0, 0, 0)
        .expect("cell (0, 0)")
        .and_then(|fill| fill.fill)
        .map(hex);
    assert_eq!(
        corner,
        Some(COLUMN_FILL.to_owned()),
        "first column must beat first row when both regions apply and neither corner override exists"
    );
}

// -------------------------------------------------------------------------------------------
// The table tier joins the toggle XOR (MJXOFF-106's forward note).
// -------------------------------------------------------------------------------------------

/// A table style states `w:b` (bold) `true` in its own base `w:rPr`, and the cell's paragraph is
/// styled with a paragraph style that *also* states `w:b` `true`. Per ECMA-376 Part 1 §17.7.3, the
/// twelve toggle properties combine by XOR across the style hierarchy — `true XOR true` cancels to
/// `false`.
///
/// Would this pass if the table tier were folded in as a plain override rung instead of a fourth XOR
/// term? No — an override rung would report `Some(true)` (whichever of the two tiers is read last
/// would simply win outright). **Proved by mutation**: temporarily changing this test's own
/// `run_properties_tier` call site to `.merge_under(&table_tier)` *before* `recombine_toggles`
/// without threading `table_tier` into the XOR (i.e. dropping the `table` argument from
/// `recombine_toggles`/`combine_toggle`, letting the toggle answer come only from the merge_under
/// fold) turns this red:
///
/// ```text
/// ---- a_bold_table_style_over_a_bold_paragraph_style_cancels_to_not_bold stdout ----
/// assertion `left == right` failed: true XOR true must cancel to false, not stay true
///   left: Some(true)
///  right: Some(false)
/// ```
///
/// Restored by re-editing.
#[test]
fn a_bold_table_style_over_a_bold_paragraph_style_cancels_to_not_bold() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");

    document
        .edit_style_sheet(|sheet, interner| {
            let mut table_style = StyleDefinition::new(interner, StyleType::Table, "BoldTable");
            table_style
                .run_properties_or_insert(interner)
                .set_bold(interner, Some(true));
            sheet.add_style(table_style);

            let mut paragraph_style =
                StyleDefinition::new(interner, StyleType::Paragraph, "BoldPara");
            paragraph_style
                .run_properties_or_insert(interner)
                .set_bold(interner, Some(true));
            sheet.add_style(paragraph_style);
        })
        .expect("edit style sheet");

    let table = document.append_table(1, 1).expect("append 1x1 table");
    document
        .edit_table(table, |table, interner| {
            table
                .properties_mut()
                .expect("w:tblPr")
                .set_style_id(interner, Some("BoldTable"));
        })
        .expect("set table style");
    document
        .edit_cell(table, 0, 0, |cell, interner| {
            let paragraph = cell
                .paragraph_mut(0)
                .expect("cell starts with one paragraph");
            paragraph
                .properties_or_insert(interner)
                .set_style(Some(mjx_docx::ParagraphStyle::new(interner, "BoldPara")));
        })
        .expect("style the cell's paragraph");
    document
        .set_cell_text(table, 0, 0, "hi")
        .expect("give the cell a run to address");

    let effective = document
        .effective_cell_run_properties(table, 0, 0, 0, 0)
        .expect("cell (0, 0), paragraph 0, run 0");
    assert_eq!(
        effective.bold,
        Some(false),
        "true XOR true must cancel to false, not stay true"
    );
}

/// The XOR-with-one-term control: only the table style states bold (no paragraph style on this
/// cell's own paragraph). A single `true` term XORs to `true` — proving the fixture's `false` answer
/// above comes from genuine cancellation, not from some unrelated bug that always reports `false`.
#[test]
fn a_single_bold_table_tier_alone_stays_bold() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    document
        .edit_style_sheet(|sheet, interner| {
            let mut table_style = StyleDefinition::new(interner, StyleType::Table, "BoldTable");
            table_style
                .run_properties_or_insert(interner)
                .set_bold(interner, Some(true));
            sheet.add_style(table_style);
        })
        .expect("edit style sheet");

    let table = document.append_table(1, 1).expect("append 1x1 table");
    document
        .edit_table(table, |table, interner| {
            table
                .properties_mut()
                .expect("w:tblPr")
                .set_style_id(interner, Some("BoldTable"));
        })
        .expect("set table style");
    document
        .set_cell_text(table, 0, 0, "hi")
        .expect("give the cell a run to address");

    let effective = document
        .effective_cell_run_properties(table, 0, 0, 0, 0)
        .expect("cell (0, 0), paragraph 0, run 0");
    assert_eq!(effective.bold, Some(true));
}

// -------------------------------------------------------------------------------------------
// w:tblPrEx — the row-level exception set overrides the table's own properties for that row alone.
// -------------------------------------------------------------------------------------------

/// The table's own `w:tblPr/w:jc` states `center`; row 0's own `w:tblPrEx/w:jc` states `right`; row 1
/// has no `w:tblPrEx` at all. Would this pass if `w:tblPrEx` were never read back (e.g. round-tripped
/// only as an opaque node)? No — `Row::exception_properties` would answer `None` for row 0 too.
#[test]
fn tblprex_on_one_row_overrides_the_tables_own_property_for_that_row_only() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let table = document.append_table(2, 2).expect("append 2x2 table");

    document
        .edit_table(table, |table, interner| {
            table
                .properties_mut()
                .expect("w:tblPr")
                .set_justification(Some(TableAlignment::new(
                    interner,
                    TableJustification::Center,
                )));

            let row0 = table.row_mut(0).expect("row 0");
            let mut exception = TableExceptionProperties::new(interner);
            exception.set_justification(Some(TableAlignment::new(
                interner,
                TableJustification::Right,
            )));
            row0.set_exception_properties(Some(exception));
        })
        .expect("set table jc and row 0's tblPrEx");

    let (table_justification, row0_exception, row1_exception) = document
        .edit_table(table, |table, interner| {
            let table_justification = table
                .properties()
                .and_then(TableProperties::justification)
                .map(|alignment| alignment.value(interner).expect("valid jc"));
            let row0_exception = table
                .row(0)
                .and_then(Row::exception_properties)
                .and_then(TableExceptionProperties::justification)
                .map(|alignment| alignment.value(interner).expect("valid jc"));
            let row1_exception = table.row(1).and_then(Row::exception_properties);
            (
                table_justification,
                row0_exception,
                row1_exception.is_some(),
            )
        })
        .expect("read back");

    assert_eq!(table_justification, Some(TableJustification::Center));
    assert_eq!(
        row0_exception,
        Some(TableJustification::Right),
        "row 0's own tblPrEx must override the table's justification for that row"
    );
    assert!(
        !row1_exception,
        "row 1 states no tblPrEx at all — it defers entirely to the table's own w:tblPr"
    );
}

// -------------------------------------------------------------------------------------------
// CT_TblPrBase / CT_TrPrBase / CT_TcPrBase round-trip — every child at least once.
// -------------------------------------------------------------------------------------------

/// Builds a table stating every `CT_TblPrBase`, `CT_TrPrBase` and `CT_TcPrBase` member at least
/// once, saves it, reopens it, and asserts the reread values match — proving both directions
/// (`FromXml` and `ToXml`) for the full property surface this ticket adds, not merely that it
/// compiles.
#[test]
fn every_tblprbase_trprbase_tcprbase_member_round_trips() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let table = document.append_table(1, 1).expect("append 1x1 table");

    document
        .edit_table(table, |table, interner| {
            let properties = table.properties_mut().expect("w:tblPr");
            properties.set_style_id(interner, Some("SomeStyle"));
            properties.set_bidi_visual(interner, Some(true));
            properties.set_row_band_size(interner, Some(2));
            properties.set_column_band_size(interner, Some(3));
            let mut width = TableWidth::new(interner);
            width.set_measure(
                interner,
                Some(TableWidthMeasure {
                    unit: TableWidthUnit::Twips,
                    value: mjx_ooxml_types::wordprocessingml::MeasurementOrPercentage::from_wire(
                        "5000",
                    ),
                }),
            );
            properties.set_width(interner, Some(width));
            properties.set_justification(Some(TableAlignment::new(
                interner,
                TableJustification::Center,
            )));
            let mut cell_spacing = TableWidth::new(interner);
            cell_spacing.set_measure(
                interner,
                Some(TableWidthMeasure {
                    unit: TableWidthUnit::Twips,
                    value: mjx_ooxml_types::wordprocessingml::MeasurementOrPercentage::from_wire(
                        "10",
                    ),
                }),
            );
            properties.set_cell_spacing(interner, Some(cell_spacing));
            properties.set_look(Some(TableLook::new(interner)));

            let row0 = table.row_mut(0).expect("row 0");
            let mut row_properties = RowProperties::new(interner);
            row_properties.set_cant_split(interner, Some(true));
            row_properties.set_table_header(interner, Some(true));
            row_properties.set_hidden(interner, Some(false));
            row0.set_properties(Some(row_properties));

            let mut exception = TableExceptionProperties::new(interner);
            exception.set_justification(Some(TableAlignment::new(
                interner,
                TableJustification::Right,
            )));
            row0.set_exception_properties(Some(exception));

            let cell = row0.cell_mut(0).expect("row 0's own cell");
            let cell_properties = cell.properties_or_insert(interner);
            cell_properties.set_no_wrap(interner, Some(true));
            cell_properties.set_hide_mark(interner, Some(true));
            cell_properties.set_fit_text(interner, Some(true));
        })
        .expect("author every member");

    let bytes = document.save().expect("save");
    let mut reopened = Document::open(&bytes).expect("reopen");

    let (
        style_id,
        bidi_visual,
        row_band,
        column_band,
        table_justification,
        row_can_split,
        row_header,
        row_hidden,
        row_exception_justification,
        cell_no_wrap,
        cell_hide_mark,
        cell_fit_text,
    ) = reopened
        .edit_table(table, |table, interner| {
            let properties = table.properties().expect("w:tblPr");
            let style_id = properties.style_id(interner).expect("style id").unwrap();
            let bidi_visual = properties.bidi_visual(interner).expect("bidi").unwrap();
            let row_band = properties
                .effective_row_band_size(interner)
                .expect("row band");
            let column_band = properties
                .effective_column_band_size(interner)
                .expect("col band");
            let table_justification = properties
                .justification()
                .map(|alignment| alignment.value(interner).expect("valid jc"));

            let row0 = table.row(0).expect("row 0");
            let row_properties = row0.properties().expect("w:trPr");
            let row_can_split = row_properties.cant_split(interner).expect("cantSplit");
            let row_header = row_properties.table_header(interner).expect("tblHeader");
            let row_hidden = row_properties.hidden(interner).expect("hidden");
            let row_exception_justification = row0
                .exception_properties()
                .and_then(TableExceptionProperties::justification)
                .map(|alignment| alignment.value(interner).expect("valid jc"));

            let cell = row0.cell(0).expect("row 0's own cell");
            let cell_properties = cell.properties().expect("w:tcPr");
            let cell_no_wrap = cell_properties.no_wrap(interner).expect("noWrap");
            let cell_hide_mark = cell_properties.hide_mark(interner).expect("hideMark");
            let cell_fit_text = cell_properties.fit_text(interner).expect("tcFitText");

            (
                style_id,
                bidi_visual,
                row_band,
                column_band,
                table_justification,
                row_can_split,
                row_header,
                row_hidden,
                row_exception_justification,
                cell_no_wrap,
                cell_hide_mark,
                cell_fit_text,
            )
        })
        .expect("read back");

    assert_eq!(style_id, "SomeStyle");
    assert!(bidi_visual);
    assert_eq!(row_band, 2);
    assert_eq!(column_band, 3);
    assert_eq!(table_justification, Some(TableJustification::Center));
    assert_eq!(row_can_split, Some(true));
    assert_eq!(row_header, Some(true));
    assert_eq!(row_hidden, Some(false));
    assert_eq!(row_exception_justification, Some(TableJustification::Right));
    assert_eq!(cell_no_wrap, Some(true));
    assert_eq!(cell_hide_mark, Some(true));
    assert_eq!(cell_fit_text, Some(true));
}

// -------------------------------------------------------------------------------------------
// Editing one cell's shading leaves every other part byte-identical, and every other cell's own
// content unaffected.
// -------------------------------------------------------------------------------------------

#[test]
fn editing_one_cells_shading_leaves_every_other_part_and_every_other_cell_untouched() {
    let mut document = four_region_document();
    document
        .set_cell_text(0, 1, 1, "band-1 cell")
        .expect("give the band-1 cell a run");
    document
        .set_cell_text(0, 2, 1, "band-2 cell")
        .expect("give the band-2 cell a run");
    let original_bytes = document.save().expect("save the baseline");
    let original_package = Package::open(&original_bytes).expect("open original package");

    let mut edited = Document::open(&original_bytes).expect("reopen baseline");
    edited
        .edit_cell(0, 1, 1, |cell, interner| {
            cell.properties_or_insert(interner)
                .set_shading(Some(shading(interner, "123456")));
        })
        .expect("edit the band-1 cell's own shading");
    let edited_bytes = edited.save().expect("save the edit");
    let edited_package = Package::open(&edited_bytes).expect("open edited package");

    let document_part = PartName::new("/word/document.xml").expect("part name");
    let mut any_other_part = false;
    for part in original_package.part_names() {
        if part == document_part {
            continue;
        }
        any_other_part = true;
        assert_eq!(
            original_package.part_bytes(&part),
            edited_package.part_bytes(&part),
            "part {part:?} must be byte-identical — only word/document.xml was edited"
        );
    }
    assert!(any_other_part, "the fixture must relate other parts too");

    // The edited cell's own fill changed …
    let mut reopened = Document::open(&edited_bytes).expect("reopen the edited document");
    let edited_fill = reopened
        .effective_cell_fill(0, 1, 1)
        .expect("band-1 cell")
        .and_then(|fill| fill.fill)
        .map(hex);
    assert_eq!(edited_fill, Some("123456".to_owned()));

    // … but every other cell's own text and effective fill are exactly what they were before the
    // edit.
    for (row, column) in [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 2),
        (2, 0),
        (2, 2),
        (3, 0),
        (3, 1),
        (3, 2),
    ] {
        let original_text = Document::open(&original_bytes)
            .expect("reopen original")
            .cell_text(0, row, column)
            .expect("original cell text");
        let edited_text = reopened
            .cell_text(0, row, column)
            .expect("edited cell text");
        assert_eq!(
            original_text, edited_text,
            "cell ({row}, {column})'s own text must be untouched by the shading edit"
        );

        let original_fill = Document::open(&original_bytes)
            .expect("reopen original")
            .effective_cell_fill(0, row, column)
            .expect("original cell fill")
            .and_then(|fill| fill.fill)
            .map(hex);
        let edited_fill = reopened
            .effective_cell_fill(0, row, column)
            .expect("edited cell fill")
            .and_then(|fill| fill.fill)
            .map(hex);
        assert_eq!(
            original_fill, edited_fill,
            "cell ({row}, {column})'s own effective fill must be untouched by the shading edit"
        );
    }
}

// -------------------------------------------------------------------------------------------
// effective_cell_border — a smoke test proving the reader wires cell -> region -> style-base
// resolution for borders too, not just fill.
// -------------------------------------------------------------------------------------------

#[test]
fn effective_cell_border_reads_a_cells_own_border_over_the_table_styles() {
    let mut document = Document::blank(PageSize::a4()).expect("blank a4 document");
    let table = document.append_table(1, 1).expect("append 1x1 table");

    document
        .edit_cell(table, 0, 0, |cell, interner| {
            let cell_properties = cell.properties_or_insert(interner);
            let mut borders = mjx_docx::CellBorders::new(interner);
            let mut top = mjx_docx::Border::new(
                interner,
                mjx_ooxml_types::wordprocessingml::BorderStyle::Single,
            );
            top.set_width_eighths_of_a_point(interner, Some(8));
            borders.set_top(interner, Some(top));
            cell_properties.set_borders(Some(borders));
        })
        .expect("set the cell's own top border");

    let border = document
        .effective_cell_border(0, 0, 0, CellBorderEdge::Top)
        .expect("cell (0, 0)'s own top border")
        .expect("a border was set");
    assert_eq!(
        border.style,
        mjx_ooxml_types::wordprocessingml::BorderStyle::Single
    );
}
