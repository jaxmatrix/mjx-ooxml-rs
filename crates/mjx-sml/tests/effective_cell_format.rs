//! **MJXOFF-108's gate.** The `xf` indirection resolved, against a fixture whose two layers
//! deliberately disagree.
//!
//! # Why `tests/fixtures/effective_cell_format.xlsx` is shaped the way it is
//!
//! Six Phase A children in a row shipped a test that could not fail, and the commonest cause was a
//! fixture that agreed with the implementation by construction. For a *two-layer* resolver that trap
//! has one specific shape, and the ticket for this child named it: **a fixture whose `cellXfs` and
//! `cellStyleXfs` entries state the same values tests nothing at all.** Every assertion passes
//! whichever layer is read, including by an implementation that never looks at the second one.
//!
//! So `cellXfs[1]` and `cellStyleXfs[1]` disagree on **all six aspects at once**:
//!
//! | aspect | `cellXfs[1]` (direct) | `cellStyleXfs[1]` (beneath) |
//! |---|---|---|
//! | number format | `numFmtId="164"` — the custom `USD` code | `numFmtId="165"` — `0.000%` |
//! | font | `fontId="1"` — `DirectFont`, bold, 12pt | `fontId="2"` — `StyleFont`, italic, 13pt |
//! | fill | `fillId="2"` — solid `FF112233` | `fillId="3"` — solid `FF445566` |
//! | border | `borderId="1"` — a thin **left** edge | `borderId="2"` — a thick **right** edge |
//! | alignment | `horizontal="left" vertical="bottom" wrapText="0"` | `horizontal="right" vertical="top" wrapText="1"` |
//! | protection | `locked="1" hidden="0"` | `locked="0" hidden="1"` |
//!
//! [`the_fixture_disagrees_with_itself_on_every_aspect`] asserts that table against the file before
//! anything else runs, so that a later edit which quietly made the two layers agree fails *here*,
//! naming the aspect, rather than turning the whole suite into a tautology.
//!
//! # And why there are five `cellXfs` records over the same pair of layers
//!
//! Records 1 to 4 name **the same** `xfId="1"` and state **the same** four indices; they differ only
//! in their `applyX` attributes. That is the only way to see the three states apart:
//!
//! | record | `applyFont` | `applyFill` | expected font | expected fill |
//! |---|---|---|---|---|
//! | 1 | `"0"` | `"0"` | the style layer | the style layer |
//! | 2 | *absent* | *absent* | the direct layer | the direct layer |
//! | 3 | `"1"` | `"1"` | the direct layer | the direct layer |
//! | 4 | `"0"` | `"1"` | the style layer | the direct layer |
//!
//! Records 1 and 2 are the mutation gate: an implementation that treats *absent* as *false* answers
//! `StyleFont` for both, and record 2's assertion is what says so. Record 4 is the other half — a
//! resolver that chose one layer for the whole record rather than per aspect passes 1, 2 and 3.
//!
//! Record 5 names `xfId="2"`, a style record that suppresses `applyFont` itself, so **both** layers
//! are off and nothing supplies the font.
//!
//! # The worksheet
//!
//! `cols` gives columns B–D `style="7"` (`ColumnFont`); row 2 writes `customFormat="1" s="6"`
//! (`RowFont`); row 3 writes `s="6"` **without** `customFormat`. So:
//!
//! | cell | writes `@s` | row | column | resolves through | font |
//! |---|---|---|---|---|---|
//! | `C2` | yes, `3` | `customFormat`, `s=6` | `style=7` | the **cell** | `DirectFont` |
//! | `B2` | no | `customFormat`, `s=6` | `style=7` | the **row** | `RowFont` |
//! | `B3` | no | `s=6`, no `customFormat` | `style=7` | the **column** | `ColumnFont` |
//! | `F4` | no | nothing | no run | the **default** record | `Calibri` |
//!
//! Three different fonts on one cell (`C2`), which is what the ticket asks for; and `B3` is what
//! makes `customFormat` load-bearing rather than decorative.

use std::borrow::Cow;

use mjx_ooxml_core::{Interner, RawDocument};
use mjx_ooxml_types::spreadsheetml::{BorderStyle, HorizontalAlignment, VerticalAlignment};
use mjx_opc::{Package, PartName};
use mjx_sml::styles::effective::{CellFormatResolver, FormatLayer, StyleIndexSource};
use mjx_sml::{
    builtin_cell_style_name, builtin_format_code, cell_style_index, column_style_index, ApplyFlag,
    BuiltInCellStyleName, CellReference, ColumnBlock, EffectiveCellFormat, FontPropertyOwner,
    FormatAspect, SheetData, StylesheetPart, WorksheetPart,
};

/// The fixture this whole suite is written against.
const FIXTURE: &str = "effective_cell_format.xlsx";

/// The bytes of one part of one committed fixture.
fn part_bytes(fixture: &str, part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The fixture's styles part, parsed and modelled.
fn stylesheet() -> (RawDocument, StylesheetPart) {
    let bytes = part_bytes(FIXTURE, "/xl/styles.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the styles part parses");
    let part = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    (document, part)
}

/// The fixture's worksheet, as both the cell store and the spine — the store holds the cells and
/// rows, the spine holds the `cols` blocks.
fn worksheet() -> (SheetData, WorksheetPart) {
    let bytes = part_bytes(FIXTURE, "/xl/worksheets/sheet1.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the worksheet parses");
    let cells = SheetData::read_worksheet(&document)
        .expect("the sheet data reads")
        .expect("the root is an x:worksheet");
    let spine = WorksheetPart::read_part(&bytes)
        .expect("the spine reads")
        .expect("the root is an x:worksheet");
    (cells, spine)
}

/// The typeface name of the font `format` resolves to.
fn font_name(
    resolver: &CellFormatResolver<'_>,
    interner: &Interner,
    format: &EffectiveCellFormat,
) -> Option<String> {
    resolver.font(format)?.properties(interner).font_name
}

/// The effective format of one cell, walking cell → row → column → the default record.
fn resolve_cell(
    resolver: &CellFormatResolver<'_>,
    cells: &SheetData,
    spine: &WorksheetPart,
    reference: &str,
) -> EffectiveCellFormat {
    let address = CellReference::parse(reference).expect("a cell reference");
    let blocks: Vec<ColumnBlock> = spine.column_blocks().cloned().collect();
    let column = u32::from(address.column()) + 1;
    let column_style =
        column_style_index(&blocks, spine.interner(), column).expect("the runs read");
    let cell = cells.cell(address);
    let row = cells.row(address.row() + 1);
    resolver
        .effective_cell_format(cell.as_ref(), row.as_ref(), column_style)
        .expect("the cell resolves")
}

/// **The fixture is discriminating, and this is what says so.**
///
/// Asserted against the *file* rather than against the resolver: it reads `cellXfs[1]` and
/// `cellStyleXfs[1]` directly and requires the two to differ on every one of the six aspects. An
/// edit that made them agree would leave every other case in this suite green while testing
/// nothing, which is exactly the failure this repo has shipped six times.
#[test]
fn the_fixture_disagrees_with_itself_on_every_aspect() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let direct = part
        .cell_formats()
        .expect("the fixture writes cellXfs")
        .get(1)
        .expect("cellXfs[1]");
    let beneath = part
        .cell_style_formats()
        .expect("the fixture writes cellStyleXfs")
        .get(1)
        .expect("cellStyleXfs[1]");

    assert_eq!(
        direct.cell_style_format_index(interner).expect("@xfId"),
        Some(1),
        "cellXfs[1] must sit on cellStyleXfs[1], or the two are not layers of one cell"
    );

    for aspect in [
        FormatAspect::NumberFormat,
        FormatAspect::Font,
        FormatAspect::Fill,
        FormatAspect::Border,
    ] {
        let above = direct.resource_index(interner, aspect).expect("an index");
        let below = beneath.resource_index(interner, aspect).expect("an index");
        assert!(
            above.is_some() && below.is_some() && above != below,
            "the two layers state the same {aspect:?} ({above:?}); a fixture where they agree \
             cannot tell a resolver that reads the wrong layer from one that reads the right one"
        );
    }

    let above = direct.alignment().expect("cellXfs[1] states an alignment");
    let below = beneath
        .alignment()
        .expect("cellStyleXfs[1] states an alignment");
    assert_ne!(
        above.horizontal_alignment(interner).expect("@horizontal"),
        below.horizontal_alignment(interner).expect("@horizontal"),
        "the two layers agree on @horizontal"
    );
    assert_ne!(
        above.vertical_alignment(interner).expect("@vertical"),
        below.vertical_alignment(interner).expect("@vertical"),
        "the two layers agree on @vertical"
    );
    assert_ne!(
        above.wraps_text(interner).expect("@wrapText"),
        below.wraps_text(interner).expect("@wrapText"),
        "the two layers agree on @wrapText"
    );

    let above = direct.protection().expect("cellXfs[1] states protection");
    let below = beneath
        .protection()
        .expect("cellStyleXfs[1] states protection");
    assert_ne!(
        above.locked(interner).expect("@locked"),
        below.locked(interner).expect("@locked"),
        "the two layers agree on @locked"
    );
    assert_ne!(
        above.formula_hidden(interner).expect("@hidden"),
        below.formula_hidden(interner).expect("@hidden"),
        "the two layers agree on @hidden"
    );
}

/// **Gate 1.** One cell, six independent assertions: every aspect of a record whose `applyX` are all
/// `"0"` comes from the `cellStyleXfs` record beneath, and the answer is the one that layer states
/// rather than the one the direct record does.
#[test]
fn every_aspect_of_a_suppressed_record_comes_from_the_layer_beneath() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");
    let format = resolver
        .resolve(1, StyleIndexSource::Cell)
        .expect("cellXfs[1] exists");

    for aspect in FormatAspect::ALL {
        let resolved = format.aspect(aspect);
        assert_eq!(
            resolved.apply_flag,
            ApplyFlag::Suppressed,
            "cellXfs[1] writes @{}=\"0\"",
            aspect.apply_attribute()
        );
        assert_eq!(
            resolved.layer,
            FormatLayer::CellStyle,
            "{aspect:?} must come from cellStyleXfs[1]"
        );
        assert_eq!(resolved.format_index, Some(1));
    }

    // 1 — number format. The direct record says 164 (the custom USD code); the style says 165.
    assert_eq!(format.number_format().resource_index, Some(165));
    assert_eq!(
        resolver
            .format_code(&format)
            .expect("the code reads")
            .as_deref(),
        Some("0.000%"),
        "reading the direct layer would answer the custom USD code"
    );

    // 2 — font. DirectFont above, StyleFont beneath.
    assert_eq!(format.font().resource_index, Some(2));
    assert_eq!(
        font_name(&resolver, interner, &format).as_deref(),
        Some("StyleFont")
    );

    // 3 — fill. FF112233 above, FF445566 beneath.
    assert_eq!(format.fill().resource_index, Some(3));
    let fill = resolver.fill(&format).expect("fill 3");
    let pattern = fill.pattern().expect("a pattern fill");
    assert_eq!(
        pattern
            .foreground_colour(interner)
            .and_then(|color| color.rgb),
        Some("FF445566".to_owned())
    );

    // 4 — border. A thin *left* edge above, a thick *right* edge beneath.
    assert_eq!(format.border().resource_index, Some(2));
    let border = resolver.border(&format).expect("border 2");
    assert_eq!(
        border
            .right_edge()
            .expect("a right edge")
            .style(interner)
            .expect("@style"),
        BorderStyle::Thick
    );
    assert_eq!(
        border
            .left_edge()
            .expect("a left edge")
            .style(interner)
            .expect("@style"),
        BorderStyle::None,
        "border 1's thin left edge belongs to the layer that was suppressed"
    );

    // 5 — alignment. left/bottom/no-wrap above, right/top/wrap beneath.
    let alignment = resolver.alignment(&format).expect("an alignment");
    assert_eq!(
        alignment
            .horizontal_alignment(interner)
            .expect("@horizontal"),
        Some(HorizontalAlignment::Right)
    );
    assert_eq!(
        alignment.vertical_alignment(interner).expect("@vertical"),
        VerticalAlignment::Top
    );
    assert_eq!(
        alignment.wraps_text(interner).expect("@wrapText"),
        Some(true)
    );

    // 6 — protection. locked/not-hidden above, unlocked/hidden beneath.
    let protection = resolver.protection(&format).expect("a protection");
    assert_eq!(protection.locked(interner).expect("@locked"), Some(false));
    assert_eq!(
        protection.formula_hidden(interner).expect("@hidden"),
        Some(true)
    );
}

/// **Gate 2.** Absent, `"1"` and `"0"` are three different answers — asserted for `applyFont` and
/// `applyFill` separately, over records that are otherwise byte for byte the same.
///
/// This is the case the mutation "treat absent `applyFont` as false" turns red: record 2 answers
/// `StyleFont` instead of `DirectFont`.
#[test]
fn absent_true_and_false_apply_flags_are_three_different_answers() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");

    // Record 1 — both flags written false.
    let suppressed = resolver
        .resolve(1, StyleIndexSource::Cell)
        .expect("cellXfs[1]");
    assert_eq!(suppressed.font().apply_flag, ApplyFlag::Suppressed);
    assert_eq!(suppressed.font().layer, FormatLayer::CellStyle);
    assert_eq!(
        font_name(&resolver, interner, &suppressed).as_deref(),
        Some("StyleFont")
    );
    assert_eq!(suppressed.fill().apply_flag, ApplyFlag::Suppressed);
    assert_eq!(suppressed.fill().resource_index, Some(3));

    // Record 2 — both flags absent. Absent is NOT false.
    let unstated = resolver
        .resolve(2, StyleIndexSource::Cell)
        .expect("cellXfs[2]");
    assert_eq!(unstated.font().apply_flag, ApplyFlag::Unstated);
    assert_eq!(
        unstated.font().layer,
        FormatLayer::Direct,
        "an absent applyFont does not suppress: §18.8.9's 0th record expresses no apply attributes \
         and is applied"
    );
    assert_eq!(
        font_name(&resolver, interner, &unstated).as_deref(),
        Some("DirectFont")
    );
    assert_eq!(unstated.fill().apply_flag, ApplyFlag::Unstated);
    assert_eq!(unstated.fill().resource_index, Some(2));

    // Record 3 — both flags written true.
    let applied = resolver
        .resolve(3, StyleIndexSource::Cell)
        .expect("cellXfs[3]");
    assert_eq!(applied.font().apply_flag, ApplyFlag::Applied);
    assert_eq!(applied.font().layer, FormatLayer::Direct);
    assert_eq!(
        font_name(&resolver, interner, &applied).as_deref(),
        Some("DirectFont")
    );
    assert_eq!(applied.fill().apply_flag, ApplyFlag::Applied);
    assert_eq!(applied.fill().resource_index, Some(2));

    // The three states are three states, and two of them agree on the *answer* while differing on
    // the flag — which is why the flag is reported beside it.
    assert_ne!(suppressed.font().apply_flag, unstated.font().apply_flag);
    assert_ne!(unstated.font().apply_flag, applied.font().apply_flag);
    assert_ne!(suppressed.font().layer, unstated.font().layer);
    assert_eq!(unstated.font().layer, applied.font().layer);

    // Record 4 — applyFont="0" beside applyFill="1" on one record. The six aspects resolve
    // independently; a resolver that picked one layer per record passes every case above and fails
    // here.
    let mixed = resolver
        .resolve(4, StyleIndexSource::Cell)
        .expect("cellXfs[4]");
    assert_eq!(mixed.font().layer, FormatLayer::CellStyle);
    assert_eq!(
        font_name(&resolver, interner, &mixed).as_deref(),
        Some("StyleFont")
    );
    assert_eq!(mixed.fill().layer, FormatLayer::Direct);
    assert_eq!(mixed.fill().resource_index, Some(2));
    // And the four aspects it says nothing about are still `Unstated`, hence still direct.
    for aspect in [
        FormatAspect::NumberFormat,
        FormatAspect::Border,
        FormatAspect::Alignment,
        FormatAspect::Protection,
    ] {
        assert_eq!(mixed.aspect(aspect).apply_flag, ApplyFlag::Unstated);
        assert_eq!(mixed.aspect(aspect).layer, FormatLayer::Direct);
    }
}

/// When the direct record suppresses an aspect **and** the record beneath suppresses it too, nothing
/// supplies it — which is this crate's answer, not the specification's, and is documented as such.
#[test]
fn both_layers_suppressing_an_aspect_leaves_it_unsupplied() {
    let (document, part) = stylesheet();
    let resolver = CellFormatResolver::new(&part, &document.interner).expect("the resolver builds");
    let format = resolver
        .resolve(5, StyleIndexSource::Cell)
        .expect("cellXfs[5]");

    assert_eq!(format.cell_style_format_index(), Some(2));
    let font = format.font();
    assert_eq!(font.apply_flag, ApplyFlag::Suppressed);
    assert_eq!(
        font.supplying_apply_flag,
        ApplyFlag::Suppressed,
        "cellStyleXfs[2] writes applyFont=\"0\" itself"
    );
    assert_eq!(font.layer, FormatLayer::Neither);
    assert_eq!(font.resource_index, None);
    assert!(resolver.font(&format).is_none());

    // Every other aspect is untouched by that: `Neither` is per aspect, not per record.
    assert_eq!(format.fill().layer, FormatLayer::Direct);
    assert_eq!(format.number_format().layer, FormatLayer::Direct);
}

/// **Gate 3.** A cell with no `@s`, in a row with `customFormat`, in a column with a `@style`
/// resolves through the right layer — with all three set to a different font.
#[test]
fn a_cell_resolves_through_cell_then_row_then_column_then_the_default_record() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");
    let (cells, spine) = worksheet();

    let expected = [
        ("C2", StyleIndexSource::Cell, 3, "DirectFont"),
        ("B2", StyleIndexSource::Row, 6, "RowFont"),
        ("B3", StyleIndexSource::Column, 7, "ColumnFont"),
        ("F4", StyleIndexSource::Default, 0, "Calibri"),
    ];
    let mut names = Vec::new();
    for (reference, source, style_index, font) in expected {
        let format = resolve_cell(&resolver, &cells, &spine, reference);
        assert_eq!(
            format.style_index_source(),
            source,
            "{reference} must resolve through {source:?}"
        );
        assert_eq!(
            format.style_index(),
            style_index,
            "{reference}'s style index"
        );
        assert_eq!(
            font_name(&resolver, interner, &format).as_deref(),
            Some(font),
            "{reference}'s effective font"
        );
        names.push(font);
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        4,
        "the four layers must name four different fonts, or three of these cases prove nothing"
    );

    // B3 is the case that makes `customFormat` load-bearing. Row 3 writes `s="6"` and no
    // `customFormat`, so §18.3.1.73's gate says the row style is not applied — and the answer must
    // be the column's, not the row's.
    let row = cells.row(3).expect("row 3 is written");
    assert_eq!(row.style(), 6, "row 3 does write @s");
    assert!(
        !row.uses_custom_format(),
        "row 3 must not write customFormat, or this case cannot tell the gate from its absence"
    );
    assert_eq!(
        cell_style_index(None, Some(&row), Some(7)),
        (7, StyleIndexSource::Column)
    );

    // And row 2 writes both, so the row layer wins over the column's.
    let row = cells.row(2).expect("row 2 is written");
    assert!(row.uses_custom_format());
    assert_eq!(
        cell_style_index(None, Some(&row), Some(7)),
        (6, StyleIndexSource::Row)
    );
}

/// **Gate 4.** A custom `formatCode` comes back character for character — a locale prefix, a quoted
/// literal holding a *double* space, and an escaped semicolon.
///
/// This is the case the mutation "normalise a custom format code's whitespace" turns red.
#[test]
fn a_custom_format_code_round_trips_character_for_character() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");

    // Record 2 applies its own number format, which is the custom one.
    let format = resolver
        .resolve(2, StyleIndexSource::Cell)
        .expect("cellXfs[2]");
    assert_eq!(format.number_format().resource_index, Some(164));
    let code = resolver
        .format_code(&format)
        .expect("the code reads")
        .expect("164 is declared");

    const EXPECTED: &str = "[$-409]#,##0.00\"  USD\"\\;;[Red]-#,##0.00";
    assert_eq!(code.as_ref(), EXPECTED);
    assert!(
        code.contains("[$-409]"),
        "the locale prefix must survive: {code}"
    );
    assert!(
        code.contains("\"  USD\""),
        "the quoted literal's two spaces must survive: {code:?}"
    );
    assert!(
        code.contains("\\;"),
        "the escaped semicolon must survive: {code:?}"
    );
    assert_eq!(
        code.chars().filter(|c| *c == ' ').count(),
        2,
        "exactly the two spaces the file wrote, neither collapsed nor trimmed"
    );
    assert!(
        matches!(code, Cow::Owned(_)),
        "the file spells the quotes as &quot;, so decoding them is the one allocation this resolver \
         makes — and the value is the decoded one, not the escaped one"
    );

    // A declared code beats §18.8.30's implied table, and an id nobody declares falls back to it.
    let default = resolver
        .resolve(0, StyleIndexSource::Default)
        .expect("cellXfs[0]");
    assert_eq!(default.number_format().resource_index, Some(0));
    assert_eq!(
        resolver
            .format_code(&default)
            .expect("the code reads")
            .as_deref(),
        Some("General"),
        "id 0 is implied by §18.8.30 and written nowhere in the file"
    );
    assert_eq!(builtin_format_code(0), Some("General"));
}

/// Reading an effective format cannot mark a part dirty — the package saves back byte for byte.
///
/// Not a courtesy: a read that triggered a reserialise would break edit isolation for every caller
/// of this crate, and the resolver takes `&self` throughout precisely so it cannot.
#[test]
fn reading_an_effective_format_leaves_the_package_byte_identical() {
    let original = mjx_fixtures::fixture(FIXTURE);
    let package = Package::open(&original).expect("the fixture opens");
    let styles_name = PartName::new("/xl/styles.xml").expect("a part name");
    let before = package
        .part_bytes(&styles_name)
        .expect("the styles part is there")
        .to_vec();

    let document = mjx_xml::fidelity::parse(&before).expect("the part parses");
    let part = StylesheetPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:styleSheet");
    let resolver = CellFormatResolver::new(&part, &document.interner).expect("the resolver builds");
    let (cells, spine) = worksheet();
    for reference in ["A1", "B1", "C1", "D1", "E1", "B2", "C2", "B3", "F4"] {
        let _ = resolve_cell(&resolver, &cells, &spine, reference);
    }

    let after = package
        .part_bytes(&styles_name)
        .expect("the styles part is still there");
    assert_eq!(after, before.as_slice(), "resolving edited the styles part");
    let saved = package.save().expect("the package saves");
    let reopened = Package::open(&saved).expect("the saved package opens");
    assert_eq!(
        reopened
            .part_bytes(&styles_name)
            .expect("the styles part survives"),
        before.as_slice()
    );
    let sheet_name = PartName::new("/xl/worksheets/sheet1.xml").expect("a part name");
    assert_eq!(
        reopened
            .part_bytes(&sheet_name)
            .expect("the worksheet survives"),
        package.part_bytes(&sheet_name).expect("it was there")
    );
}

/// The `cellStyleXfs` record beneath a cell has a **name**, and `builtinId` is what identifies it.
#[test]
fn the_named_style_beneath_a_cell_is_reachable_by_its_builtin_id() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");
    let format = resolver
        .resolve(1, StyleIndexSource::Cell)
        .expect("cellXfs[1]");

    let style = resolver
        .named_style(&format)
        .expect("cellStyleXfs[1] is named by a cellStyle");
    assert_eq!(
        style.style_name(interner).expect("@name").as_deref(),
        Some("Explanatory Text")
    );
    assert_eq!(style.builtin_id(interner).expect("@builtinId"), Some(53));
    assert_eq!(
        builtin_cell_style_name(53),
        Some(BuiltInCellStyleName::Fixed("Explanatory Text")),
        "Annex G.2's invariant name for builtinId 53"
    );

    // The default record sits on cellStyleXfs[0], which is Normal.
    let default = resolver
        .resolve(0, StyleIndexSource::Default)
        .expect("cellXfs[0]");
    assert_eq!(
        resolver
            .named_style(&default)
            .and_then(|style| style.builtin_id(interner).ok().flatten()),
        Some(0)
    );
}

/// The resolver reads a `styles.xml` it has never seen and answers, for every committed fixture.
///
/// A sweep over the corpus rather than a list in this file, so that an `.xlsx` added by a later
/// child joins it by existing. It asserts nothing about *what* the answers are — it cannot, for a
/// file it does not know — only that every `cellXfs` index in every fixture resolves without error,
/// which is what says the decoder handles the shapes real parts have.
#[test]
fn every_committed_fixture_resolves_every_one_of_its_cell_formats() {
    let mut examined = 0usize;
    for name in mjx_fixtures::all_fixture_files() {
        if !name.ends_with(".xlsx") {
            continue;
        }
        let bytes = mjx_fixtures::fixture(&name);
        let package = Package::open(&bytes).expect("a committed fixture opens");
        let parts: Vec<PartName> = package
            .part_names()
            .filter(|part| part.as_str().ends_with("/styles.xml"))
            .collect();
        for part_name in parts {
            let part_bytes = package.part_bytes(&part_name).expect("the part is there");
            let document = mjx_xml::fidelity::parse(part_bytes).expect("the part parses");
            let Some(stylesheet) = StylesheetPart::read_part(&document).expect("the part reads")
            else {
                continue;
            };
            let resolver = CellFormatResolver::new(&stylesheet, &document.interner)
                .unwrap_or_else(|error| panic!("{name}::{}: {error}", part_name.as_str()));
            for index in 0..resolver.cell_format_count() {
                let index = u32::try_from(index).expect("a table this size");
                let format = resolver
                    .resolve(index, StyleIndexSource::Cell)
                    .unwrap_or_else(|error| {
                        panic!("{name}::{} xf {index}: {error}", part_name.as_str())
                    });
                // Every answer names a layer, and a `Direct` answer names its own index.
                for aspect in FormatAspect::ALL {
                    let resolved = format.aspect(aspect);
                    if resolved.layer == FormatLayer::Direct {
                        assert_eq!(resolved.format_index, Some(index));
                    }
                }
                let _ = resolver.format_code(&format).expect("the code reads");
                examined += 1;
            }
        }
    }
    assert!(
        examined >= 10,
        "only {examined} cell formats across the whole corpus — a sweep that finds nothing passes \
         every assertion in it"
    );
}

/// The font a resolved format names is the same value a rich-text run's `rPr` decodes to.
///
/// MJXOFF-97 modelled that family once and MJXOFF-105 reused it whole; this child adds no font type
/// of its own, and this is the assertion that says so.
#[test]
fn a_resolved_font_is_the_shared_font_property_family() {
    let (document, part) = stylesheet();
    let interner = &document.interner;
    let resolver = CellFormatResolver::new(&part, interner).expect("the resolver builds");
    let format = resolver
        .resolve(3, StyleIndexSource::Cell)
        .expect("cellXfs[3]");
    let font = resolver.font(&format).expect("font 1");

    let properties = font.properties(interner);
    assert_eq!(properties.font_name.as_deref(), Some("DirectFont"));
    assert_eq!(properties.bold, Some(true));
    assert_eq!(properties.size_in_points, Some(12.0));

    let same = mjx_sml::FontProperties::read(
        &font.as_raw_element(),
        interner,
        FontPropertyOwner::FontTableEntry,
    );
    assert_eq!(properties, same);
}
