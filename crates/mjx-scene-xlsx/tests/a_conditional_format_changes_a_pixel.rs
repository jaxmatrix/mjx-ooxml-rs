//! ⚠ The `dxf` is observed **changing a pixel**, not merely being selected.
//!
//! `mjx-layout-xlsx` can say which rule fired and what fill it imposed, and it does — but a rule
//! that fires and reaches no paint is indistinguishable, on a screen, from a rule that never fired.
//! MJXOFF-244 made the other half reachable: a worksheet's fragment tree now becomes a
//! [`DisplayList`], and a painter reads *that*. So the assertions here are taken at the **encoded
//! display list**, which is the last thing before pixels and the first thing a painter sees.
//!
//! Two behaviours could each have made the whole feature vacuous, and each is asserted here rather
//! than described:
//!
//! 1. **A `dxf`'s fill is written differently from a cell's.** Excel writes a highlight as
//!    `<fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill>` — no `@patternType`, and
//!    the colour in `bgColor` rather than `fgColor`. Read the way a cell's fill is read, that is a
//!    pattern of `none` with no foreground, which paints **nothing**: the rule fires, the report
//!    says so, and the sheet looks identical.
//! 2. **A colour scale's stops are not colours yet.** They are `CT_Color` — an `@rgb`, an
//!    `@indexed` row, or a `@theme` *position* with a `@tint` — so the box model hands over the two
//!    stops and a position between them and this crate does the blend. A midpoint is asserted, not
//!    only the two ends, because a resolver that returned its low stop always would pass an
//!    endpoint-only fixture.

mod support;

use mjx_layout::Fragment;
use mjx_scene::{
    build_scene, Color, Command, DisplayList, FillStyle, Geometry, Paint, SceneOptions,
};
use mjx_text::GlyphAtlas;
use mjx_xlsx::{PartName, Workbook};

use support::{resolve, styles, viewport, worksheet};

/// A workbook whose sheet holds `sheet`, whose styles carry `differentials` as its `<dxfs>` table.
fn book(sheet: &str, differentials: &[&str]) -> Workbook {
    book_with_formats(sheet, differentials, &[], "")
}

/// The same, plus `cellXfs` records and a `numFmts` block.
fn book_with_formats(
    sheet: &str,
    differentials: &[&str],
    formats: &[&str],
    number_formats: &str,
) -> Workbook {
    let base = styles("", "", "", formats);
    let text = String::from_utf8(base).expect("the support markup is UTF-8");
    let dxfs = format!(
        r#"<dxfs count="{}">{}</dxfs>"#,
        differentials.len(),
        differentials.concat()
    );
    // `dxfs` sits after `cellXfs` in `CT_Stylesheet`'s sequence.
    let markup = text
        .replace("</styleSheet>", &format!("{dxfs}</styleSheet>"))
        .replace(
            "<fonts count=\"9\">",
            &format!("{number_formats}<fonts count=\"9\">"),
        );
    assert!(markup.contains("<dxfs"), "the dxfs table was inserted");

    let bytes = Workbook::blank()
        .expect("a blank workbook")
        .save_unchecked()
        .expect("blank saves");
    let mut package = mjx_xlsx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new(support::SHEET_PART).expect("a part name"),
            worksheet(sheet),
        )
        .expect("the worksheet is replaceable");
    package
        .replace_part_bytes(
            &PartName::new(support::STYLES_PART).expect("a part name"),
            markup.into_bytes(),
        )
        .expect("the styles part is replaceable");
    Workbook::from_package(package).expect("the authored package resolves")
}

/// Builds the display list for the first band of `book`.
fn display_list(book: &mut Workbook) -> DisplayList {
    let constraints = viewport(6.0, 2.0);
    let mut resolved = resolve(book, 0, &constraints);
    let options = SceneOptions::new(constraints.page);
    let mut atlas = GlyphAtlas::new();
    build_scene(
        &resolved.tree,
        &resolved.resources,
        resolved.model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("the band becomes a display list")
}

/// Every solid fill the list encodes, as `(top edge, colour)`, ordered down the page.
///
/// Taken **out of the encoded list**: the commands, their paint indices and the geometry table, all
/// read back through `mjx-scene`'s own accessors, which is what a painter does. Ordering by the top
/// edge is what lets a fixture say *the first cell, the second cell* without re-deriving the scene
/// builder's coordinate conversion inside the test — the sheets here are one column, so down the
/// page is unambiguous.
fn fills(list: &DisplayList) -> Vec<(f32, Color)> {
    let mut out = Vec::new();
    for command in list.commands() {
        let Command::FillPath { geometry, paint } = command else {
            continue;
        };
        let Some(Geometry::Rectangle(rect)) = list.geometry(geometry) else {
            continue;
        };
        if let Some(Paint::Solid(colour)) = list.paint(paint) {
            out.push((rect.top, colour));
        }
    }
    out.sort_by(|left, right| left.0.total_cmp(&right.0));
    out
}

/// Excel's own *Light Red Fill with Dark Red Text*, spelled exactly as Excel spells it.
const HIGHLIGHT: &str = r#"<dxf><font><color rgb="FF9C0006"/></font><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf>"#;

/// Two cells, one above the threshold and one below it, in the same `xf`.
const TWO_CELLS: &str = r#"<dimension ref="A1:A2"/>
<sheetData>
<row r="1"><c r="A1"><v>10</v></c></row>
<row r="2"><c r="A2"><v>1</v></c></row>
</sheetData>
<conditionalFormatting sqref="A1:A2"><cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan"><formula>5</formula></cfRule></conditionalFormatting>"#;

#[test]
fn a_dxf_that_states_only_a_bg_colour_paints_the_cell_that_fired_and_not_its_neighbour() {
    let mut book = book(TWO_CELLS, &[HIGHLIGHT]);
    let painted = fills(&display_list(&mut book));

    // ⚠ **One** fill, not two. The cell below the threshold is not filled at all, so a fixture that
    // only checked the colour of the first would be green for an evaluator that fired on both.
    assert_eq!(painted.len(), 1, "exactly one cell was filled: {painted:?}");
    let (_, fired) = painted[0];
    assert_eq!(
        (fired.red, fired.green, fired.blue, fired.alpha),
        (0xFF, 0xC7, 0xCE, 0xFF),
        "the colour in `bgColor` reached the paint table"
    );
}

#[test]
fn the_same_sheet_without_the_rule_paints_neither_cell() {
    // The control. The rule is the only difference between this fixture and the one above, so if
    // both were green with an evaluator that fires on nothing, this one would be too — and the
    // assertion above would still be red.
    let plain = TWO_CELLS
        .split("<conditionalFormatting")
        .next()
        .expect("the sheet before the block");
    let mut book = book(plain, &[HIGHLIGHT]);
    assert!(fills(&display_list(&mut book)).is_empty());
}

#[test]
fn the_dxfs_font_colour_reaches_the_text_and_not_the_cell_below_it() {
    let mut book = book(TWO_CELLS, &[HIGHLIGHT]);
    let constraints = viewport(6.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);
    use mjx_scene::ResourceResolver;

    let colour_of = |row: u32| {
        resolved.tree.nodes().find_map(|(_, node)| {
            if !matches!(node.fragment(), Fragment::GlyphRun(_)) {
                return None;
            }
            if node.source().path().segments() != [row, 0] {
                return None;
            }
            resolved
                .resources
                .text_decoration(node.source())
                .map(|decoration| decoration.fill)
        })
    };
    assert_eq!(
        colour_of(0),
        Some(FillStyle::Solid(Color {
            red: 0x9C,
            green: 0x00,
            blue: 0x06,
            alpha: 0xFF,
        })),
        "the dark red the dxf named"
    );
    assert_eq!(
        colour_of(1),
        None,
        "and the cell that did not fire keeps the font's own colour, which is none"
    );
}

// -------------------------------------------------------------------------------------------
// Colour scales, which are interpolations and so need a midpoint
// -------------------------------------------------------------------------------------------

/// Black at the minimum, white at the maximum, over 0, 50 and 100.
const SCALE: &str = r#"<dimension ref="A1:A3"/>
<sheetData>
<row r="1"><c r="A1"><v>0</v></c></row>
<row r="2"><c r="A2"><v>50</v></c></row>
<row r="3"><c r="A3"><v>100</v></c></row>
</sheetData>
<conditionalFormatting sqref="A1:A3"><cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="max"/><color rgb="FF000000"/><color rgb="FFFFFFFF"/></colorScale></cfRule></conditionalFormatting>"#;

#[test]
fn a_colour_scale_blends_its_two_stops_and_the_middle_cell_is_neither_of_them() {
    let mut book = book(SCALE, &[]);
    let painted = fills(&display_list(&mut book));
    assert_eq!(painted.len(), 3, "every cell of the scale is filled");

    assert_eq!(
        (painted[0].1.red, painted[0].1.green, painted[0].1.blue),
        (0x00, 0x00, 0x00)
    );
    assert_eq!(
        (painted[2].1.red, painted[2].1.green, painted[2].1.blue),
        (0xFF, 0xFF, 0xFF)
    );

    // ⚠ The midpoint. `128` is `round(0 + 0.5 * 255)`, and it is the number a resolver that
    // answered its first stop, its last stop, or a fixed grey would all miss.
    let middle = painted[1].1;
    assert_eq!(
        (middle.red, middle.green, middle.blue),
        (128, 128, 128),
        "halfway between black and white, channel by channel"
    );
    assert_eq!(middle.alpha, 0xFF);
}

#[test]
fn the_scale_moves_when_the_data_does_and_not_only_when_the_stops_do() {
    // The same two stops over a *different* spread: 0, 25, 100. The middle cell is now a quarter of
    // the way up rather than halfway, so the colour must move too. A blend hard-coded at the mean of
    // its stops passes the case above and fails here.
    let skewed = SCALE.replace("<v>50</v>", "<v>25</v>");
    assert_ne!(skewed, SCALE, "the middle value was found");
    let mut book = book(&skewed, &[]);
    let painted = fills(&display_list(&mut book));
    assert_eq!(painted.len(), 3);
    let middle = painted[1].1;
    assert_eq!(
        (middle.red, middle.green, middle.blue),
        (64, 64, 64),
        "a quarter of the way from black to white"
    );
}

#[test]
fn a_scale_stop_that_names_a_palette_row_is_resolved_here_and_not_in_the_box_model() {
    // The whole reason the blend happens in this crate: `<color indexed="10"/>` is a **row of
    // `indexedColors`**, not a colour, and a box model that mixed two hex triplets would have been
    // resolving a palette it does not hold — the same division `[Red]` travels by. What is asserted
    // is that the answer is neither of the two literals the fixture could have fallen back to.
    let themed = SCALE.replace(r#"<color rgb="FFFFFFFF"/>"#, r#"<color indexed="10"/>"#);
    assert_ne!(themed, SCALE, "the high stop was found");
    let mut book = book(&themed, &[]);
    let painted = fills(&display_list(&mut book));
    assert_eq!(painted.len(), 3);
    let high = painted[2].1;
    assert_ne!(
        (high.red, high.green, high.blue),
        (0x00, 0x00, 0x00),
        "the themed stop resolved to something other than the low one"
    );
    let middle = painted[1].1;
    assert_eq!(
        (middle.red, middle.green, middle.blue),
        (
            (f32::from(high.red) / 2.0).round() as u8,
            (f32::from(high.green) / 2.0).round() as u8,
            (f32::from(high.blue) / 2.0).round() as u8
        ),
        "and the midpoint is halfway to it, which says the blend used the resolved colour"
    );
}

// -------------------------------------------------------------------------------------------
// Two colours claiming the same text, and which one wins
// -------------------------------------------------------------------------------------------

#[test]
fn a_dxf_font_colour_outranks_a_number_formats_own_colour() {
    // WARNING: two mechanisms reach the same glyph run and only one can win. `#,##0;[Red]#,##0`
    // colours a negative cell through `Decoration::text_colour` — a bare row of `indexedColors`,
    // which MJXOFF-172 made *win over the font* precisely because it is a statement about the value.
    // A conditional format's `dxf` says something about the value too, and it is the more specific
    // one: a person who writes *make overdue rows dark red* expects dark red.
    //
    // The fixture is a **negative** cell, so both mechanisms fire. If the number format won, the
    // text would be palette row 2 — Excel's own bright red, `FF0000` — rather than the dxf's
    // `9C0006`, and the highlight rule would look broken on exactly the cells it was written for.
    let sheet = r#"<dimension ref="A1:A2"/>
<sheetData>
<row r="1"><c r="A1" s="1"><v>-10</v></c></row>
<row r="2"><c r="A2" s="1"><v>-1</v></c></row>
</sheetData>
<conditionalFormatting sqref="A1:A2"><cfRule type="cellIs" dxfId="0" priority="1" operator="lessThan"><formula>-5</formula></cfRule></conditionalFormatting>"#;
    let numfmts =
        r##"<numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0;[Red]#,##0"/></numFmts>"##;
    let mut book = book_with_formats(
        sheet,
        &[HIGHLIGHT],
        &[r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
        numfmts,
    );
    let constraints = viewport(6.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);
    use mjx_scene::ResourceResolver;

    let colour_of = |row: u32| {
        resolved.tree.nodes().find_map(|(_, node)| {
            if !matches!(node.fragment(), Fragment::GlyphRun(_)) {
                return None;
            }
            if node.source().path().segments() != [row, 0] {
                return None;
            }
            resolved
                .resources
                .text_decoration(node.source())
                .map(|decoration| decoration.fill)
        })
    };

    let dark_red = FillStyle::Solid(Color {
        red: 0x9C,
        green: 0x00,
        blue: 0x06,
        alpha: 0xFF,
    });
    assert_eq!(
        colour_of(0),
        Some(dark_red.clone()),
        "the rule fired, so its dxf colour wins over `[Red]`"
    );

    // And the cell that did *not* fire keeps `[Red]`, which is what says the number format is still
    // working rather than having been switched off wholesale.
    let unfired = colour_of(1).expect("the second cell is still coloured by its format");
    assert_ne!(unfired, dark_red, "it is not the dxf's colour");
    let FillStyle::Solid(colour) = unfired else {
        panic!("a solid colour");
    };
    assert_eq!(
        (colour.red, colour.green, colour.blue),
        (0xFF, 0x00, 0x00),
        "it is row two of `indexedColors`, which is `[Red]`"
    );
}
