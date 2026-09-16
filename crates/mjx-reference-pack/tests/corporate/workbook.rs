//! The corporate workbook: one sheet shaped like a report somebody would actually send
//! (MJXOFF-300).

use mjx_chart::{ChartData, ChartKind};
use mjx_ooxml_types::spreadsheetml::{BorderStyle, ConditionalFormatValueObjectType, IconSetType};
use mjx_sml::{
    BorderEdgeSpec, BorderSpec, CellFormatSpec, CellFormatTarget, CellRange, CellRangeList,
    CellReference, CellSpan, CellValue, ColorScaleSpec, ColumnWidth, ConditionalRuleSpec,
    ConditionalRuleSpecKind, ConditionalValueObjectSpec, DataBarSpec, IconSetSpec,
    TableStyleReferenceSpec, WorksheetTableSpec,
};
use mjx_xlsx::drawing_geometry::{CellMarker, ResizingBehavior};
use mjx_xlsx::{Package, Workbook};

use super::{part_text, set_part_text, splice, LOGO_PNG};

/// The parts a blank workbook writes, and the drawing this one adds.
const SHEET: &str = "/xl/worksheets/sheet1.xml";
const STYLES: &str = "/xl/styles.xml";
const DRAWING: &str = "/xl/drawings/drawing1.xml";

/// The cell whose inline string becomes rich text, and the text the splice looks for.
const RICH_CELL: &str = "F2";
const RICH_MARKER: &str = "RICHTEXTCELL";

/// The accounting and date formats. Custom ids start at 164; 0-163 are the built-in table.
const ACCOUNTING_FORMAT: u32 = 164;
const DATE_FORMAT: u32 = 165;

/// The two custom number formats, as `xl/styles.xml` states them. `numFmts` is `styleSheet`'s
/// first child, before `fonts`.
const NUMBER_FORMATS: &str = concat!(
    "<numFmts count=\"2\">",
    "<numFmt numFmtId=\"164\" formatCode=\"_(&quot;$&quot;* #,##0.00_);_(&quot;$&quot;* \\(#,##0.00\\);",
    "_(&quot;$&quot;* &quot;-&quot;??_);_(@_)\"/>",
    "<numFmt numFmtId=\"165\" formatCode=\"[$-409]d\\-mmm\\-yy;@\"/>",
    "</numFmts>"
);

/// Gridlines on, and the pane frozen below the header row.
const SHEET_VIEWS: &str = concat!(
    "<sheetViews><sheetView tabSelected=\"true\" showGridLines=\"true\" workbookViewId=\"0\">",
    "<pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/>",
    "<selection pane=\"bottomLeft\" activeCell=\"A2\" sqref=\"A2\"/>",
    "</sheetView></sheetViews>"
);

/// The rich-text cell: two runs, one of which states its own font.
///
/// Neither run's text begins or ends with a space, and that is a schema requirement rather than a
/// style choice: `sml.xsd` types `t` as `ST_Xstring`, a simple type that can carry **no attribute
/// at all**, so the `xml:space="preserve"` every other producer writes is refused. The committed
/// fixtures that carry one are third-party files the gate tolerates; a part this workspace authors
/// has to be valid, so the run boundary falls on a comma instead of on a space.
const RICH_TEXT: &str = concat!(
    "<is><r><rPr><b/><sz val=\"11\"/><color rgb=\"FFC00000\"/><rFont val=\"Calibri\"/></rPr>",
    "<t>Audited</t></r><r><t>, signed off by finance</t></r></is>"
);

/// The text-box shape, anchored beside the picture in the same drawing part.
const TEXT_BOX: &str = concat!(
    "<xdr:twoCellAnchor editAs=\"twoCell\">",
    "<xdr:from><xdr:col>8</xdr:col><xdr:colOff>0</xdr:colOff>",
    "<xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>",
    "<xdr:to><xdr:col>11</xdr:col><xdr:colOff>0</xdr:colOff>",
    "<xdr:row>4</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>",
    "<xdr:sp><xdr:nvSpPr><xdr:cNvPr id=\"40\" name=\"Commentary\"/><xdr:cNvSpPr/></xdr:nvSpPr>",
    "<xdr:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></a:xfrm>",
    "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></xdr:spPr>",
    "<xdr:txBody><a:bodyPr anchor=\"t\" rtlCol=\"false\"/><a:lstStyle/>",
    "<a:p><a:r><a:rPr lang=\"en-US\"/><a:t>Trend is ahead of plan in every region.</a:t></a:r></a:p>",
    "</xdr:txBody></xdr:sp><xdr:clientData/></xdr:twoCellAnchor>"
);

/// The corporate workbook, as bytes.
pub(crate) fn corporate_workbook() -> Vec<u8> {
    let mut book = Workbook::blank().expect("a blank workbook");
    book.rename_sheet(0, "Summary").expect("the tab's name");
    author_cells(&mut book);
    author_table(&mut book);
    author_conditional_formats(&mut book);
    author_borders(&mut book);
    author_columns(&mut book);
    author_drawing(&mut book);
    let saved = book.save().expect("the authored workbook saves");
    finish(splice_the_unwritable(saved))
}

/// One cell reference, parsed.
fn at(reference: &str) -> CellReference {
    CellReference::parse(reference).expect("a cell reference")
}

/// The headings, the regions, the money, the dates and the two measures.
fn author_cells(book: &mut Workbook) {
    let headings = ["Region", "Revenue", "Booked", "Trend", "Score"];
    for (column, heading) in headings.iter().enumerate() {
        let reference = format!("{}1", (b'A' + column as u8) as char);
        book.set_cell_value(0, at(&reference), CellValue::InlineString(heading))
            .expect("a heading");
    }

    // Revenue is money, Booked is a date serial, Trend and Score are what the rules read.
    let rows = [
        ("EMEA", 1_204_000.0, 46_023.0, 0.62, 3.0),
        ("APAC", 987_500.0, 46_031.0, 0.31, 2.0),
        ("North America", 1_410_250.0, 46_038.0, 0.88, 3.0),
        ("LATAM", 412_000.0, 46_045.0, 0.12, 1.0),
    ];
    for (index, (region, revenue, booked, trend, score)) in rows.iter().enumerate() {
        let row = index + 2;
        book.set_cell_value(0, at(&format!("A{row}")), CellValue::InlineString(region))
            .expect("a region");
        book.set_cell_value(0, at(&format!("B{row}")), CellValue::Number(*revenue))
            .expect("a revenue");
        book.set_cell_value(0, at(&format!("C{row}")), CellValue::Number(*booked))
            .expect("a date");
        book.set_cell_value(0, at(&format!("D{row}")), CellValue::Number(*trend))
            .expect("a trend");
        book.set_cell_value(0, at(&format!("E{row}")), CellValue::Number(*score))
            .expect("a score");
    }

    // The cell the rich-text splice replaces, marked with a string nothing else writes.
    book.set_cell_value(0, at(RICH_CELL), CellValue::InlineString(RICH_MARKER))
        .expect("the rich-text cell");
    book.set_cell_value(
        0,
        at("G1"),
        CellValue::InlineString("Diagonal and dashed borders"),
    )
    .expect("the border label");
}

/// The worksheet table, wearing the style MJXOFF-300 names.
fn author_table(book: &mut Workbook) {
    let mut spec = WorksheetTableSpec::new(
        "SummaryTable",
        CellRange::parse("A1:E5").expect("a range"),
        &["Region", "Revenue", "Booked", "Trend", "Score"],
    );
    spec.style = Some(TableStyleReferenceSpec::named("TableStyleMedium2"));
    book.add_table(0, &spec).expect("the worksheet table");
}

/// A colour scale, a data bar and an icon set, each over its own column.
fn author_conditional_formats(book: &mut Workbook) {
    let colour_scale = ConditionalRuleSpec {
        kind: ConditionalRuleSpecKind::ColorScale(ColorScaleSpec::two_color(
            "FFF8696B", "FF63BE7B",
        )),
        priority: 1,
        stops_lower_priority_rules: None,
        differential_format_index: None,
    };
    book.add_conditional_formatting(
        0,
        &CellRangeList::parse("D2:D5").expect("a range list"),
        &[colour_scale],
    )
    .expect("the colour scale");

    let data_bar = ConditionalRuleSpec {
        kind: ConditionalRuleSpecKind::DataBar(DataBarSpec::spanning_the_range("638EC6")),
        priority: 2,
        stops_lower_priority_rules: None,
        differential_format_index: None,
    };
    book.add_conditional_formatting(
        0,
        &CellRangeList::parse("B2:B5").expect("a range list"),
        &[data_bar],
    )
    .expect("the data bar");

    let icon_set = ConditionalRuleSpec {
        kind: ConditionalRuleSpecKind::IconSet(IconSetSpec {
            icons: Some(IconSetType::ThreeArrows),
            thresholds: vec![
                ConditionalValueObjectSpec::with_value(
                    ConditionalFormatValueObjectType::Percent,
                    "0",
                ),
                ConditionalValueObjectSpec::with_value(
                    ConditionalFormatValueObjectType::Percent,
                    "33",
                ),
                ConditionalValueObjectSpec::with_value(
                    ConditionalFormatValueObjectType::Percent,
                    "67",
                ),
            ],
            ..IconSetSpec::default()
        }),
        priority: 3,
        stops_lower_priority_rules: None,
        differential_format_index: None,
    };
    book.add_conditional_formatting(
        0,
        &CellRangeList::parse("E2:E5").expect("a range list"),
        &[icon_set],
    )
    .expect("the icon set");
}

/// A cell wearing a diagonal and two dashed edges — the one border a `Decoration` cannot carry.
fn author_borders(book: &mut Workbook) {
    let border = book
        .append_border(&BorderSpec {
            left: Some(BorderEdgeSpec::styled(BorderStyle::Dashed)),
            right: Some(BorderEdgeSpec::styled(BorderStyle::Dashed)),
            top: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            bottom: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            diagonal: Some(BorderEdgeSpec::styled(BorderStyle::Thin)),
            diagonal_up: Some(true),
            diagonal_down: Some(false),
            ..BorderSpec::skeleton_border()
        })
        .expect("the border");
    let style = book
        .append_cell_format(
            CellFormatTarget::CellFormats,
            &CellFormatSpec {
                border_index: Some(border),
                applies_border: Some(true),
                ..CellFormatSpec::skeleton_cell_format()
            },
        )
        .expect("the bordered format");
    book.set_cell_style(0, at("G2"), Some(style))
        .expect("the bordered cell");
}

/// The money and date columns are narrow, which is what makes their formats worth reading.
fn author_columns(book: &mut Workbook) {
    book.set_column_width(
        0,
        CellSpan::new(1, 2).expect("a column span"),
        Some(ColumnWidth::Custom(9.0)),
    )
    .expect("the narrow columns");
    book.set_column_width(
        0,
        CellSpan::new(0, 0).expect("a column span"),
        Some(ColumnWidth::Custom(18.0)),
    )
    .expect("the region column");
}

/// The chart and the picture. The text box joins them in the same drawing part, spliced.
fn author_drawing(book: &mut Workbook) {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["EMEA", "APAC", "North America", "LATAM"])
        .series("Revenue", [1_204_000.0, 987_500.0, 1_410_250.0, 412_000.0]);
    book.add_chart(
        0,
        &chart,
        CellMarker::new(0, 0, 7, 0),
        CellMarker::new(6, 0, 20, 0),
        "Revenue",
        ResizingBehavior::MoveWithCellsButDoNotResize,
    )
    .expect("the chart");
    book.add_two_cell_anchored_picture(
        0,
        LOGO_PNG,
        "Logo",
        CellMarker::new(8, 0, 6, 0),
        CellMarker::new(10, 0, 9, 0),
        ResizingBehavior::MoveWithCellsButDoNotResize,
    )
    .expect("the picture");
}

/// The four elements this workspace has a reader for and no writer.
fn splice_the_unwritable(saved: Vec<u8>) -> Package {
    let mut package = Package::open(&saved).expect("the authored workbook reopens");

    let mut styles = part_text(&package, STYLES);
    splice(&mut styles, "<fonts", &format!("{NUMBER_FORMATS}<fonts"));
    set_part_text(&mut package, STYLES, styles);

    let mut sheet = part_text(&package, SHEET);
    // `sheetViews` is rank 2, so it goes before whichever of its successors comes first.
    let successor = ["<sheetFormatPr", "<cols", "<sheetData"]
        .into_iter()
        .filter_map(|name| sheet.find(name).map(|at| (at, name)))
        .min()
        .map(|(_, name)| name)
        .expect("the worksheet states a child sheetViews must precede");
    splice(&mut sheet, successor, &format!("{SHEET_VIEWS}{successor}"));
    splice(
        &mut sheet,
        &format!("<is><t>{RICH_MARKER}</t></is>"),
        RICH_TEXT,
    );
    set_part_text(&mut package, SHEET, sheet);

    let mut drawing = part_text(&package, DRAWING);
    splice(
        &mut drawing,
        "</xdr:wsDr>",
        &format!("{TEXT_BOX}</xdr:wsDr>"),
    );
    set_part_text(&mut package, DRAWING, drawing);

    package
}

/// The cell formats that point at the two spliced number formats, written through the typed writer.
fn finish(package: Package) -> Vec<u8> {
    let mut book = Workbook::from_package(package).expect("the spliced workbook reopens");
    for (format, column) in [(ACCOUNTING_FORMAT, 'B'), (DATE_FORMAT, 'C')] {
        let style = book
            .append_cell_format(
                CellFormatTarget::CellFormats,
                &CellFormatSpec {
                    number_format_id: Some(format),
                    applies_number_format: Some(true),
                    ..CellFormatSpec::skeleton_cell_format()
                },
            )
            .expect("a number format");
        for row in 2..=5 {
            book.set_cell_style(0, at(&format!("{column}{row}")), Some(style))
                .expect("a formatted cell");
        }
    }
    book.save().expect("the corporate workbook saves")
}
