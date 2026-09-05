//! **MJXOFF-102's markup gate.** `CT_Worksheet`'s thirty-nine slots: read, held, placed by generated
//! rank, and written back byte for byte.
//!
//! # The fixture, and why it is authored the way it is
//!
//! Six Phase A children in a row shipped a test that could not fail, and **twice** the cause was a
//! fixture written in the order the writer emits, so a broken writer looked correct. This child is
//! entirely about schema order, so that failure mode would be fatal here. `worksheet_spine.xlsx` is
//! authored against it:
//!
//! * **Two `<cols>` blocks, not one.** `CT_Worksheet` declares the slot `maxOccurs="unbounded"`
//!   (`sml.xsd:2176`), so two blocks of one run each and one block of two runs are the same column
//!   widths and different bytes. A model that merged them would pass a "widths came back" assertion
//!   and fail [`the_two_cols_blocks_stay_two`].
//! * **`mergeCells` present and `autoFilter` absent.** `autoFilter` is rank 10 and `mergeCells` rank
//!   14, so a writer that emitted "in the order we happen to know them" rather than by rank puts
//!   them the wrong way round — and a fixture carrying both, or neither, could not tell.
//! * **One child from every later Phase D child's territory**, none of them modelled here:
//!   `sheetProtection`, `mergeCells`, `phoneticPr`, `conditionalFormatting`, `dataValidations`,
//!   `hyperlinks`, `printOptions`, `pageMargins`, `pageSetup`, `headerFooter`, `rowBreaks`,
//!   `colBreaks`, `ignoredErrors`, `tableParts`, `extLst`. **An unmodelled child is not a dropped
//!   child**, and [`every_unmodelled_slot_survives_an_edit_in_its_schema_position`] is what says so.
//! * **Two spaces between two attributes**, on one modelled slot (`sheetFormatPr`) and one
//!   unmodelled one (`pageSetup`). A decomposed attribute list does not record the whitespace
//!   *between* attributes, so nothing but the verbatim source range reproduces it. That is what
//!   makes the copy-on-write assertions here a comparison against **the file** rather than against a
//!   second run of the same writer — the exact shape `cell_store_fidelity.rs` had to reach for one
//!   layer down.
//! * A comment between two slots, a single-quoted attribute, an element written `<x></x>` rather
//!   than `<x/>`, a cell-level `extLst` in a foreign namespace, and a `headerFooter` whose character
//!   data carries `&amp;` and `&quot;`.
//!
//! # No `mjx_opc` in this file's models
//!
//! The suite reaches a package only to *get the bytes of a part*. Every assertion after that is made
//! against [`WorksheetPart`], which has never heard of a [`PartName`](mjx_opc::PartName): the
//! `drawing`, `hyperlinks` and `tableParts` slots hold relationship ids as the strings the file
//! wrote, and resolving one is `mjx-xlsx`'s. That is the layering rule stated as a test rather than
//! as a comment.

use std::collections::BTreeSet;

use mjx_ooxml_types::child_order::WORKSHEET;
use mjx_opc::{Package, PartName};
use mjx_sml::{CellReference, CellValue, WorksheetPart};

/// One committed worksheet part: a label, and its bytes.
struct WorksheetSource {
    label: String,
    bytes: Vec<u8>,
}

/// Every worksheet part of every committed `.xlsx` fixture, derived from the corpus directory rather
/// than from a list in this file.
///
/// `_rels` streams live under `/xl/worksheets/` too and are not worksheets; a part directly under
/// `worksheets/` is a sheet and one under its `_rels/` never is.
fn worksheet_sources() -> Vec<WorksheetSource> {
    let mut found = Vec::new();
    for name in mjx_fixtures::all_fixture_files() {
        if !name.ends_with(".xlsx") {
            continue;
        }
        let bytes = mjx_fixtures::fixture(&name);
        let package = Package::open(&bytes).expect("a committed fixture opens");
        let parts: Vec<PartName> = package
            .part_names()
            .filter(|part| {
                part.as_str().starts_with("/xl/worksheets/")
                    && !part.as_str().starts_with("/xl/worksheets/_rels/")
            })
            .collect();
        for part in parts {
            found.push(WorksheetSource {
                label: format!("{name}::{}", part.as_str()),
                bytes: package
                    .part_bytes(&part)
                    .expect("the worksheet part is there")
                    .to_vec(),
            });
        }
    }
    assert!(
        found.len() >= 5,
        "only {} worksheet part(s) found in the committed corpus — a sweep that finds nothing \
         passes every assertion below",
        found.len()
    );
    found
}

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

/// Reads a worksheet part, insisting that it is one.
fn read(bytes: &[u8]) -> WorksheetPart {
    WorksheetPart::read_part(bytes)
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// The spine fixture's own worksheet.
fn spine() -> Vec<u8> {
    part_bytes("worksheet_spine.xlsx", "/xl/worksheets/sheet1.xml")
}

// -------------------------------------------------------------------------------------------
// Round-trip
// -------------------------------------------------------------------------------------------

/// Every committed worksheet part re-emits byte for byte, untouched **and** after an edit has forced
/// the frame off its whole-part shortcut.
///
/// The second half is the one that measures anything. Until something is edited the writer is one
/// `extend_from_slice` of the part's own buffer, which no defect in the slot walk could disturb; the
/// edited pass runs the walk over every slot and still has to reproduce the file.
#[test]
fn every_committed_worksheet_re_emits_byte_for_byte_before_and_after_an_edit() {
    for source in worksheet_sources() {
        let sheet = read(&source.bytes);
        assert!(
            sheet.is_verbatim(),
            "{}: a part just read owns no bytes of its own",
            source.label
        );
        assert_eq!(
            sheet.to_markup(),
            source.bytes,
            "{}: the untouched part must re-emit verbatim",
            source.label
        );

        // Reach a slot mutably without changing it. That is enough to give up the whole-part
        // shortcut, so every slot is written through the walk.
        let mut edited = read(&source.bytes);
        let touched = edited.dimension_mut().is_some()
            || edited.sheet_data_mut().is_some()
            || edited.sheet_views_mut().is_some();
        assert!(
            touched,
            "{}: every committed worksheet has at least one of dimension, sheetData or sheetViews",
            source.label
        );
        assert!(
            !edited.is_verbatim(),
            "{}: reaching a slot mutably must give up the whole-part shortcut",
            source.label
        );
        assert_eq!(
            edited.to_markup(),
            source.bytes,
            "{}: the slot walk must reproduce the file",
            source.label
        );
    }
}

/// A part whose root is not an `x:worksheet` is a question, not an error.
#[test]
fn a_part_that_is_not_a_worksheet_reads_as_none() {
    let workbook = part_bytes("sample.xlsx", "/xl/workbook.xml");
    assert!(WorksheetPart::read_part(&workbook)
        .expect("no error")
        .is_none());

    // …and so is an element merely *named* worksheet in somebody else's namespace.
    let foreign = br#"<worksheet xmlns="urn:not-spreadsheetml"><sheetData/></worksheet>"#;
    assert!(WorksheetPart::read_part(foreign)
        .expect("no error")
        .is_none());
}

// -------------------------------------------------------------------------------------------
// `sample.xlsx` — the counted facts
// -------------------------------------------------------------------------------------------

/// Every attribute of `sample.xlsx`'s `sheetView` comes back, **counted from the file**.
///
/// MJXOFF-102's own ticket says "sixteen"; the orchestrator's pre-dispatch check said fifteen. Both
/// are claims about a committed file, so neither is taken on trust: the expected count is derived
/// here by counting `="` in the part's own `<sheetView …>` start tag, and the model's count is
/// compared against that. A wrong number in a ticket cannot make this pass or fail.
#[test]
fn every_sheet_view_attribute_of_sample_xlsx_comes_back_counted_from_the_file() {
    let bytes = part_bytes("sample.xlsx", "/xl/worksheets/sheet1.xml");
    let text = core::str::from_utf8(&bytes).expect("the part is UTF-8");
    let start = text.find("<sheetView ").expect("a sheetView start tag");
    let end = start + text[start..].find('>').expect("the tag closes");
    let in_the_file = text[start..end].matches("=\"").count();
    assert_eq!(
        in_the_file, 15,
        "counted from the file: sample.xlsx's sheetView carries fifteen attributes, not the \
         sixteen the ticket claims"
    );

    let sheet = read(&bytes);
    let views = sheet.sheet_views().expect("a sheetViews element");
    let view = views.views().next().expect("one sheetView");
    assert_eq!(
        view.attribute_count(),
        in_the_file,
        "the model must carry every attribute the file wrote"
    );

    // …and they are readable through their typed accessors, so the count is not met by a bag of
    // bytes nothing can interpret.
    let interner = sheet.interner();
    assert!(view.tab_selected(interner).expect("tabSelected"));
    assert!(view.shows_grid_lines(interner).expect("showGridLines"));
    assert!(!view.shows_formulas(interner).expect("showFormulas"));
    assert_eq!(view.zoom_scale(interner).expect("zoomScale"), 100);
    assert_eq!(
        view.zoom_scale_page_layout_view(interner)
            .expect("zoomScalePageLayoutView"),
        100
    );
    assert_eq!(
        view.workbook_view_index(interner).expect("workbookViewId"),
        0
    );
    assert_eq!(
        view.top_left_cell(interner).expect("topLeftCell"),
        Some(CellReference::parse("A1").expect("A1"))
    );

    // An attribute the file does **not** write reads as its schema default and is not written back.
    assert!(
        view.shows_ruler(interner).expect("showRuler"),
        "default true"
    );
    assert!(!text[start..end].contains("showRuler"));

    // The one `<selection>` the file writes.
    let selection = view.selections().next().expect("a selection");
    assert_eq!(
        selection.active_cell(interner).expect("activeCell"),
        Some(CellReference::parse("A1").expect("A1"))
    );
    assert_eq!(view.selections().count(), 1);
    assert!(view.pane().is_none(), "sample.xlsx freezes nothing");
}

/// `sample.xlsx`'s three `<col>` runs come back, in one block, with their widths.
#[test]
fn the_three_col_runs_of_sample_xlsx_come_back() {
    let bytes = part_bytes("sample.xlsx", "/xl/worksheets/sheet1.xml");
    let text = core::str::from_utf8(&bytes).expect("UTF-8");
    assert_eq!(text.matches("<col ").count(), 3, "counted from the file");
    assert_eq!(text.matches("<cols>").count(), 1);

    let sheet = read(&bytes);
    let blocks: Vec<usize> = sheet
        .column_blocks()
        .map(|block| block.run_count())
        .collect();
    assert_eq!(blocks, vec![3], "one block of three runs");

    let interner = sheet.interner();
    let widths: Vec<(u32, u32, Option<f64>)> = sheet
        .column_blocks()
        .flat_map(mjx_sml::ColumnBlock::runs)
        .map(|run| {
            (
                run.first_column(interner).expect("min"),
                run.last_column(interner).expect("max"),
                run.width(interner).expect("width"),
            )
        })
        .collect();
    assert_eq!(
        widths,
        vec![(1, 1, Some(6.72)), (2, 2, Some(4.07)), (3, 3, Some(5.47))]
    );
}

// -------------------------------------------------------------------------------------------
// The spine fixture — thirty-nine slots
// -------------------------------------------------------------------------------------------

/// The frame emits its children in generated-rank order — asserted **against
/// `child_order::WORKSHEET`**, not against the input.
///
/// The part is written out and parsed again before the ranks are read, so what is checked is the
/// sequence the writer produced rather than the one the reader happened to be handed.
#[test]
fn the_emitted_children_are_in_generated_rank_order() {
    assert_eq!(WORKSHEET.symbol, "CT_Worksheet");
    assert_eq!(
        WORKSHEET.slots.len(),
        39,
        "CT_Worksheet is a thirty-nine slot sequence — the widest in the schema"
    );

    let bytes = spine();
    let mut sheet = read(&bytes);
    sheet
        .set_cell_value(
            CellReference::parse("B2").expect("B2"),
            CellValue::Number(42.0),
        )
        .expect("B2 is inside the grid");
    let emitted = sheet.to_markup();

    let reparsed = read(&emitted);
    let ranks: Vec<u16> = reparsed
        .child_element_locals()
        .filter_map(|local| WORKSHEET.rank_of(None, local))
        .collect();
    assert!(
        ranks.len() >= 20,
        "only {} rankable children — a sequence that short cannot show an ordering defect",
        ranks.len()
    );
    assert!(
        ranks.windows(2).all(|pair| pair[0] <= pair[1]),
        "the emitted children are out of CT_Worksheet's xsd:sequence: {ranks:?}"
    );

    // …and the rank list is not trivially satisfiable: it must contain the two repeated `cols`
    // ranks and skip rank 10 (`autoFilter`), which the fixture deliberately omits.
    let cols = WORKSHEET.rank_of(None, "cols").expect("cols is ranked");
    assert_eq!(
        ranks.iter().filter(|rank| **rank == cols).count(),
        2,
        "two cols blocks, so the repeated rank appears twice"
    );
    let auto_filter = WORKSHEET
        .rank_of(None, "autoFilter")
        .expect("autoFilter is ranked");
    assert!(
        !ranks.contains(&auto_filter),
        "the fixture omits autoFilter on purpose, so a writer that invented one would show here"
    );
    let merge_cells = WORKSHEET
        .rank_of(None, "mergeCells")
        .expect("mergeCells is ranked");
    assert!(ranks.contains(&merge_cells));
    assert!(
        auto_filter < merge_cells,
        "the omitted slot ranks *before* the present one, which is what makes their pairing \
         discriminating"
    );
}

/// The two `<cols>` blocks stay two. Merging them would describe the same widths and change the
/// file.
#[test]
fn the_two_cols_blocks_stay_two() {
    let bytes = spine();
    let text = core::str::from_utf8(&bytes).expect("UTF-8");
    assert_eq!(text.matches("<cols>").count(), 2, "counted from the file");

    let sheet = read(&bytes);
    let blocks: Vec<usize> = sheet
        .column_blocks()
        .map(|block| block.run_count())
        .collect();
    assert_eq!(
        blocks,
        vec![1, 1],
        "two blocks of one run each, never merged"
    );

    let interner = sheet.interner();
    let second = sheet.column_blocks().nth(1).expect("a second block");
    let run = second.runs().next().expect("its run");
    assert_eq!(run.first_column(interner).expect("min"), 4);
    assert_eq!(run.last_column(interner).expect("max"), 4);
    assert!(run.best_fit(interner).expect("bestFit"));
    assert_eq!(run.outline_level(interner).expect("outlineLevel"), 1);
}

/// **An unmodelled child is not a dropped child.** Every slot this child does not model survives an
/// edit elsewhere, byte for byte and in its schema position.
///
/// The comparison is against the *file's* bytes, and two of the slots checked carry whitespace no
/// rebuild reproduces (`pageSetup`'s doubled space) or character data a re-escaping writer would
/// change (`headerFooter`'s `&amp;` and `&quot;`), so this cannot pass by comparing two identical
/// rebuilds.
#[test]
fn every_unmodelled_slot_survives_an_edit_in_its_schema_position() {
    const HELD: &[&str] = &[
        "sheetProtection",
        "mergeCells",
        "phoneticPr",
        "conditionalFormatting",
        "dataValidations",
        "hyperlinks",
        "printOptions",
        "pageMargins",
        "pageSetup",
        "headerFooter",
        "rowBreaks",
        "colBreaks",
        "ignoredErrors",
        "tableParts",
        "extLst",
    ];

    let bytes = spine();
    let text = core::str::from_utf8(&bytes).expect("UTF-8");
    let mut sheet = read(&bytes);
    sheet
        .set_cell_value(
            CellReference::parse("B2").expect("B2"),
            CellValue::Number(99.0),
        )
        .expect("B2 is inside the grid");
    let emitted = sheet.to_markup();
    let emitted_text = core::str::from_utf8(&emitted).expect("UTF-8");

    for local in HELD {
        let opening = format!("<{local}");
        let start = text
            .find(&opening)
            .unwrap_or_else(|| panic!("the fixture writes <{local}>"));
        let end = closing_index(text, start, local);
        let original = &text[start..end];
        assert!(
            emitted_text.contains(original),
            "<{local}> did not survive an edit elsewhere byte for byte.\n  wanted: {original}"
        );
    }

    // The two discriminators: whitespace no rebuild records, and entity spellings a re-escaping
    // writer would normalise.
    assert!(
        emitted_text.contains(r#"<pageSetup paperSize="9"  orientation="landscape""#),
        "the doubled space in pageSetup's start tag is only reproducible from the source range"
    );
    assert!(
        emitted_text.contains("&amp;C&amp;&quot;Times New Roman,Regular&quot;&amp;12Spine"),
        "headerFooter's character data must come back in the entity spellings the file used"
    );
    assert!(
        emitted_text.contains(r#"<sheetFormatPr defaultColWidth="9.140625"  defaultRowHeight"#),
        "a *modelled* slot nobody touched keeps its verbatim bytes too"
    );

    // The edit itself landed.
    assert_eq!(
        sheet
            .cell(CellReference::parse("B2").expect("B2"))
            .expect("B2 is populated")
            .number(),
        Some(99.0)
    );
    // …and only one row of `sheetData` was rewritten.
    for row in sheet.rows() {
        if row.number() == Some(2) {
            continue;
        }
        assert!(
            row.is_verbatim(),
            "row {:?} was rewritten by an edit to row 2",
            row.number()
        );
    }
}

/// The end of the previous element, given the index its start tag begins at.
fn closing_index(text: &str, start: usize, local: &str) -> usize {
    let rest = &text[start..];
    let open_tag_end = rest.find('>').expect("the start tag closes") + 1;
    if rest[..open_tag_end].ends_with("/>") {
        return start + open_tag_end;
    }
    let close = format!("</{local}>");
    start + rest.find(&close).expect("an end tag") + close.len()
}

/// Every slot the generated table names is either modelled or held raw, and the modelled set is
/// exactly the thirteen this workspace claims.
///
/// MJXOFF-102 (D07) modelled the first seven ranks, and the fact that they were a **prefix** was
/// what let the frame hold everything else as unranked raw markup. MJXOFF-117 (D12) added six more —
/// ranks 7, 8, 9, 14, 23 and 24 — so the modelled and held slots now interleave, and the frame ranks
/// a held child through the same generated table rather than treating it as unrankable. The
/// assertion below is what stops that distinction being lost again:
/// `crates/mjx-sml/tests/sheet_grid.rs` pins the placement it makes possible.
#[test]
fn the_thirty_nine_slots_are_accounted_for() {
    const MODELLED: &[&str] = &[
        // MJXOFF-102 (D07) — ranks 0..=6.
        "sheetPr",
        "dimension",
        "sheetViews",
        "sheetFormatPr",
        "cols",
        "sheetData",
        "sheetCalcPr",
        // MJXOFF-117 (D12) — ranks 7, 8, 9, 14, 23, 24.
        "sheetProtection",
        "protectedRanges",
        "scenarios",
        "mergeCells",
        "rowBreaks",
        "colBreaks",
    ];

    let names: BTreeSet<&'static str> = WORKSHEET.slots.iter().map(|slot| slot.local).collect();
    assert_eq!(names.len(), 39, "thirty-nine distinct child names");
    for local in MODELLED {
        assert!(
            names.contains(local),
            "{local} is modelled here but is not a slot of CT_Worksheet"
        );
    }
    let mut modelled_ranks: Vec<u16> = MODELLED
        .iter()
        .map(|local| WORKSHEET.rank_of(None, local).expect("a rank"))
        .collect();
    modelled_ranks.sort_unstable();
    assert_eq!(
        modelled_ranks,
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 14, 23, 24]
    );
    assert_ne!(
        modelled_ranks,
        (0..modelled_ranks.len() as u16).collect::<Vec<_>>(),
        "the modelled slots are no longer a prefix of the sequence, which is why a held child has \
         to be ranked too — see `Slot::rank` in `crates/mjx-sml/src/worksheet/frame.rs`"
    );
}

/// The modelled slots of the spine fixture read back through their typed accessors.
#[test]
fn the_modelled_slots_read_back_through_their_accessors() {
    let bytes = spine();
    let sheet = read(&bytes);
    let interner = sheet.interner();

    // `sheetPr` and all three of its children.
    let properties = sheet.properties().expect("a sheetPr");
    assert_eq!(
        properties.code_name(interner).expect("codeName").as_deref(),
        Some("Spine")
    );
    assert!(!properties.filter_mode(interner).expect("filterMode"));
    let colour = properties.tab_colour(interner).expect("a tabColor");
    assert_eq!(colour.rgb.as_deref(), Some("FF00B050"));
    assert_eq!(colour.tint, Some(-0.25));
    let outline = properties.outline().expect("an outlinePr");
    assert!(!outline
        .summary_row_below_detail(interner)
        .expect("summaryBelow"));
    assert!(properties
        .page_setup()
        .expect("a pageSetUpPr")
        .fit_to_page(interner)
        .expect("fitToPage"));

    // `dimension`, reported as read.
    let dimension = sheet.dimension().expect("a dimension");
    assert_eq!(
        dimension.range(interner).expect("ref").text().as_str(),
        "A1:D6"
    );

    // The frozen pane and both selections.
    let view = sheet
        .sheet_views()
        .expect("sheetViews")
        .views()
        .next()
        .expect("a sheetView");
    let pane = view.pane().expect("a pane");
    assert_eq!(pane.horizontal_split(interner).expect("xSplit"), 1.0);
    assert_eq!(pane.vertical_split(interner).expect("ySplit"), 2.0);
    assert_eq!(
        pane.state(interner).expect("state"),
        mjx_ooxml_types::spreadsheetml::PaneState::Frozen
    );
    assert_eq!(
        pane.active_pane(interner).expect("activePane"),
        mjx_ooxml_types::spreadsheetml::Pane::BottomRight
    );
    let selections: Vec<_> = view.selections().collect();
    assert_eq!(selections.len(), 2, "one selection per visible pane");
    let ranges = selections[1]
        .selected_ranges(interner)
        .expect("sqref")
        .expect("a range list");
    assert_eq!(ranges.len(), 2, "`B3:C4 D6` is two ranges, not one");

    // `sheetFormatPr`.
    let format = sheet.format_properties().expect("a sheetFormatPr");
    assert_eq!(format.default_row_height(interner).expect("required"), 15.0);
    assert_eq!(
        format
            .deepest_row_outline_level(interner)
            .expect("outlineLevelRow"),
        1
    );

    // `sheetCalcPr` — reported, never acted on.
    assert!(sheet
        .calculation_properties()
        .expect("a sheetCalcPr")
        .full_calculation_on_load(interner)
        .expect("fullCalcOnLoad"));

    // The cell store, reached through the frame.
    assert_eq!(sheet.row_count(), 4);
    assert_eq!(sheet.cell_count(), 9);
    assert_eq!(
        sheet
            .cell(CellReference::parse("B3").expect("B3"))
            .expect("B3")
            .number(),
        Some(9.5)
    );

    // The relationship prefix, which is all this layer knows about `tableParts/@r:id`.
    assert_eq!(sheet.relationship_prefix(), Some("r"));
}

// -------------------------------------------------------------------------------------------
// Placement
// -------------------------------------------------------------------------------------------

/// A newly set child lands at its rank in the sequence, not at the end — and the unranked nodes
/// between the slots neither move nor stop the scan.
#[test]
fn a_new_child_is_placed_at_its_schema_rank() {
    // `sample.xlsx` writes no `sheetCalcPr` (rank 6), and does write `printOptions` (rank 19),
    // so a writer that appended would put it after everything.
    let bytes = part_bytes("sample.xlsx", "/xl/worksheets/sheet1.xml");
    let mut sheet = read(&bytes);
    assert!(sheet.calculation_properties().is_none());

    let calc = mjx_sml::SheetCalculationProperties::new(sheet.interner_mut(), None);
    sheet.set_calculation_properties(Some(calc));

    let emitted = sheet.to_markup();
    let text = core::str::from_utf8(&emitted).expect("UTF-8");
    let calc_at = text
        .find("<sheetCalcPr")
        .expect("the new child was written");
    let data_at = text.find("</sheetData>").expect("sheetData");
    let print_at = text.find("<printOptions").expect("printOptions");
    assert!(
        data_at < calc_at && calc_at < print_at,
        "sheetCalcPr ranks 6: after sheetData (5) and before printOptions (19)"
    );

    // Reparsing agrees, so the placement is a property of the bytes rather than of the model.
    let reparsed = read(&emitted);
    let ranks: Vec<u16> = reparsed
        .child_element_locals()
        .filter_map(|local| WORKSHEET.rank_of(None, local))
        .collect();
    assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
}

/// A slot removed by its setter is gone, and everything else keeps its place.
#[test]
fn a_slot_set_to_none_is_removed_and_nothing_else_moves() {
    let bytes = spine();
    let mut sheet = read(&bytes);
    sheet.set_calculation_properties(None);
    let emitted = sheet.to_markup();
    let text = core::str::from_utf8(&emitted).expect("UTF-8");
    assert!(!text.contains("<sheetCalcPr"));
    assert!(text.contains("<sheetProtection"));
    assert!(text.contains("</sheetData>"));

    let reparsed = read(&emitted);
    let ranks: Vec<u16> = reparsed
        .child_element_locals()
        .filter_map(|local| WORKSHEET.rank_of(None, local))
        .collect();
    assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
}

/// An unmodelled child is stepped over by placement rather than treated as a boundary.
///
/// The insertion point is **one past the last sibling that must precede** the new child, and a node
/// the generated table cannot rank is neither that sibling nor a stopping point. So `sheetViews`
/// lands immediately after `dimension` — the foreign node does not end the scan, and does not pull
/// the new child past itself. This is `child_order::insert_index_of_names`' own rule, which
/// `mjx_sml::WorkbookPart` is held to in the same words; it is not restated here.
///
/// Authored markup rather than a fixture, because a foreign element inside `x:worksheet` is markup
/// `sml.xsd` rejects — the frame preserves it, and a committed fixture carrying one could not be
/// schema-valid.
#[test]
fn an_unmodelled_child_is_stepped_over_rather_than_treated_as_a_boundary() {
    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let markup = format!(
        r#"<worksheet xmlns="{SML_NS}"><dimension ref="A1"/><q:note xmlns:q="urn:q"/><sheetData/><pageSetup/></worksheet>"#
    );
    let mut sheet = read(markup.as_bytes());
    let views = mjx_sml::SheetViews::new(sheet.interner_mut(), None);
    sheet.set_sheet_views(Some(views));

    let locals: Vec<&str> = sheet.child_element_locals().collect();
    assert_eq!(
        locals,
        vec!["dimension", "sheetViews", "note", "sheetData", "pageSetup"],
        "sheetViews ranks 2, so it goes one past dimension (1); the foreign node neither ends the \
         scan nor pulls the new child past itself, and sheetData (5) still follows it"
    );

    // The foreign node itself is untouched, prefix and all, and it is still between the two slots
    // it was written between.
    let emitted = sheet.to_markup();
    let text = core::str::from_utf8(&emitted).expect("UTF-8");
    assert!(text.contains(r#"<q:note xmlns:q="urn:q"/><sheetData/>"#));
    assert!(text.contains(r#"<dimension ref="A1"/><sheetViews/><q:note"#));
}

/// A `sheetData` this frame authors lands at rank 5, for a worksheet that had none.
#[test]
fn an_authored_sheet_data_lands_at_rank_five() {
    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let markup =
        format!(r#"<worksheet xmlns="{SML_NS}"><dimension ref="A1"/><pageSetup/></worksheet>"#);
    let mut sheet = read(markup.as_bytes());
    assert!(sheet.sheet_data().is_none());
    sheet
        .set_cell_value(
            CellReference::parse("A1").expect("A1"),
            CellValue::Number(1.0),
        )
        .expect("A1 is inside the grid");

    let emitted = sheet.to_markup();
    let text = core::str::from_utf8(&emitted).expect("UTF-8");
    let data_at = text.find("<sheetData").expect("an authored sheetData");
    let dimension_at = text.find("<dimension").expect("dimension");
    let page_at = text.find("<pageSetup").expect("pageSetup");
    assert!(dimension_at < data_at && data_at < page_at);
}

// -------------------------------------------------------------------------------------------
// `dimension` — a cached value, treated as one
// -------------------------------------------------------------------------------------------

/// Reading never recomputes the cached bounding box, even where the cells disagree with it.
#[test]
fn a_dimension_that_disagrees_with_the_cells_is_reported_not_repaired() {
    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let markup = format!(
        r#"<worksheet xmlns="{SML_NS}"><dimension ref="A1:B2"/><sheetData><row r="9"><c r="D9"><v>1</v></c></row></sheetData></worksheet>"#
    );
    let sheet = read(markup.as_bytes());
    assert_eq!(
        sheet
            .dimension()
            .expect("a dimension")
            .range(sheet.interner())
            .expect("ref")
            .text()
            .as_str(),
        "A1:B2",
        "the file's cached box is reported as it stands"
    );
    assert_eq!(sheet.to_markup(), markup.as_bytes());
}

/// A cell written outside the cached box widens it; one written inside leaves it byte-identical.
#[test]
fn writing_outside_the_cached_box_widens_it_and_writing_inside_does_not() {
    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let markup = format!(
        r#"<worksheet xmlns="{SML_NS}"><dimension ref="A1:B2"/><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#
    );

    let mut inside = read(markup.as_bytes());
    inside
        .set_cell_value(
            CellReference::parse("B2").expect("B2"),
            CellValue::Number(2.0),
        )
        .expect("inside the grid");
    let text = String::from_utf8(inside.to_markup()).expect("UTF-8");
    assert!(
        text.contains(r#"<dimension ref="A1:B2"/>"#),
        "a cell inside the box changes nothing about it: {text}"
    );

    let mut outside = read(markup.as_bytes());
    outside
        .set_cell_value(
            CellReference::parse("D9").expect("D9"),
            CellValue::Number(3.0),
        )
        .expect("inside the grid");
    let text = String::from_utf8(outside.to_markup()).expect("UTF-8");
    assert!(
        text.contains(r#"<dimension ref="A1:D9"/>"#),
        "a cell outside the box widens it, because the stale cache would then be this library's: \
         {text}"
    );
}

/// Recomputing is the caller's ask, and it reports what it wrote.
#[test]
fn recomputing_the_dimension_is_explicit() {
    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let markup = format!(
        r#"<worksheet xmlns="{SML_NS}"><dimension ref="A1:Z99"/><sheetData><row r="2"><c r="B2"><v>1</v></c><c r="C2"><v>2</v></c></row></sheetData></worksheet>"#
    );
    let mut sheet = read(markup.as_bytes());
    let written = sheet.recompute_dimension().expect("a range");
    assert_eq!(written.text().as_str(), "B2:C2");
    let text = String::from_utf8(sheet.to_markup()).expect("UTF-8");
    assert!(text.contains(r#"<dimension ref="B2:C2"/>"#), "{text}");
}

// -------------------------------------------------------------------------------------------
// Layering
// -------------------------------------------------------------------------------------------

/// The `tableParts` slot's relationship id is held as the string the file wrote, and this crate does
/// not resolve it.
///
/// The whole file names `mjx_opc` only to fetch a part's bytes; every model assertion above is made
/// without one, which is the layering rule stated as a test.
#[test]
fn a_relationship_id_is_held_as_text_and_never_resolved_here() {
    let bytes = spine();
    let sheet = read(&bytes);
    assert_eq!(sheet.relationship_prefix(), Some("r"));

    let held = sheet
        .children()
        .filter_map(|child| match child {
            mjx_sml::WorksheetContent::Raw(mjx_ooxml_core::RawNode::Element(element)) => {
                Some(element)
            }
            _ => None,
        })
        .find(|element| sheet.interner().resolve(element.name.local) == "tableParts")
        .expect("the fixture writes tableParts");
    let part = held
        .children
        .iter()
        .find_map(|node| match node {
            mjx_ooxml_core::RawNode::Element(element) => Some(element),
            _ => None,
        })
        .expect("a tablePart");
    let id = part
        .attributes
        .iter()
        .find(|attribute| sheet.interner().resolve(attribute.name.local) == "id")
        .expect("an r:id");
    assert_eq!(&*id.value, b"rId1");
    assert_eq!(
        sheet.interner().resolve(id.name.prefix.expect("a prefix")),
        "r"
    );
}
