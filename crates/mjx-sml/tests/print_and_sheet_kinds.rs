//! **MJXOFF-129's markup gate.** The print block every sheet *kind* shares, custom sheet views, and
//! the three sheet kinds that are not worksheets.
//!
//! # The fixture is authored to make specific wrong answers visible
//!
//! `tests/fixtures/print_and_sheet_kinds.xlsx` is written by hand, and every value in its print
//! block is one a plausible implementation gets wrong:
//!
//! * **`printOptions@gridLinesSet="0"`** — the *non-default* of an attribute whose schema default is
//!   `true`. A reader that answered the default rather than the file passes on every other flag in
//!   the element and fails on this one;
//! * **`printOptions@verticalCentered="0"`** — explicitly `false` where `false` is already the
//!   default, so a writer that "tidied" a redundant attribute away changes the bytes;
//! * **`pageSetup@errors="NA"`**, whose generated variant is [`PrintError::NotAvailable`] and not
//!   `Na`. A test written from the wire token does not compile;
//! * **`pageSetup@cellComments="atEnd"`** and **`@pageOrder="overThenDown"`**, both the non-default
//!   member of their enumerations;
//! * **`pageSetup` carries `@paperSize`, `@paperHeight` *and* `@paperWidth`** — a producer stating
//!   both the index and the measurement, which nothing here reconciles;
//! * **`pageSetup` carries `@scale` *and* `@fitToWidth`/`@fitToHeight`** — the two scaling modes at
//!   once, with `sheetPr/pageSetUpPr/@fitToPage` deciding which a consumer honours from a
//!   *different slot*. A model that cleared one because the other was set would lose a value;
//! * **`pageMargins` are six different numbers** (`0.7`, `0.75`, `1`, `1.25`, `0.3`, `0.45`), so a
//!   reader that crossed two accessors fails rather than coincidentally agreeing;
//! * **the six header/footer strings use six different shapes** — see below;
//! * **the `customSheetView` carries all nine of its children**, including an `extLst` after the
//!   `autoFilter`, so a new child appended rather than placed lands after the `extLst` and the
//!   ordering gate sees it;
//! * **`phoneticPr` (rank 15) stands between two typed slots**, so the modelled and held slots
//!   genuinely interleave — the property MJXOFF-117 had to fix placement for.
//!
//! # The six header strings, and what each one is for
//!
//! | element | text | the wrong answer it catches |
//! |---|---|---|
//! | `oddHeader` | `&L&"Arial,Bold"Q1&C&G&R&P of &N` | a quoted font name, a `&G`, and three sections |
//! | `oddFooter` | `&CSmith && Sons&R&D &T` | **`&&` is a literal ampersand**, not a section code and not a drawing reference |
//! | `evenHeader` | `&LEven&RPage &P` | two sections, no center |
//! | `evenFooter` | `&C&F` | a section whose whole run is one code |
//! | `firstHeader` | `&C&#65;lpha&LFirst` | a **character reference** in the source, and `&C` *before* `&L` |
//! | `firstFooter` | a **CDATA section** holding `&Rdraft` | character data that is not a text node at all |
//!
//! The last two are the ones the *never re-serialise* rule is really about: `&#65;` decodes to `A`
//! and a CDATA section decodes to its contents, so a model that rebuilt its own text node would
//! write `Alpha` and `&amp;Rdraft` — the same strings, different bytes, and a subtree that has lost
//! its verbatim source range.
//!
//! # Why the re-emission tests are not byte-identity tests
//!
//! The same reason MJXOFF-120, MJXOFF-123, MJXOFF-125 and MJXOFF-127 each recorded: **a part nobody
//! edited is one `extend_from_slice` of its own buffer, and after an edit elsewhere every other slot
//! still writes from its own stored bytes.** No byte-identity gate ever reaches this model's writer.
//!
//! For a header string there is a *second* door of the same kind, and MJXOFF-123 found it:
//! [`HeaderFooterText`] read from a file **replays its stored children** rather than re-rendering
//! its text, so even forcing the slot to rebuild does not exercise the escaping path. Both doors are
//! opened here, by two different cases:
//!
//! * [`every_new_model_survives_a_rebuild_from_the_model_alone`] drops the slot's verbatim bytes and
//!   rebuilds — the **replay** path;
//! * [`a_replaced_header_string_writes_freshly_escaped_text`] calls `set_text` — the **escape** path,
//!   which is the only way a `HeaderFooterText` ever renders its own string.

use mjx_ooxml_core::{Interner, ToXml};
use mjx_ooxml_types::spreadsheetml::{
    CellComments, PageOrder, PrintError, PrintOrientation, SheetState, SheetViewType,
};
use mjx_opc::{Package, PartName};
use mjx_sml::sheets::{ChartSheetPart, DialogSheetPart, MacroSheetPart};
use mjx_sml::{HeaderFooterSection, HeaderFooterSlot, HeaderFooterText, WorksheetPart};

/// The fixture this suite is written against.
const FIXTURE: &str = "print_and_sheet_kinds.xlsx";

/// The bytes of one part of the fixture.
fn part_bytes(part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let package = Package::open(&bytes).expect("the fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .expect("the part is there")
        .to_vec()
}

/// The fixture's worksheet, read.
fn sheet() -> WorksheetPart {
    WorksheetPart::read_part(&part_bytes("/xl/worksheets/sheet1.xml"))
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

/// The fixture's dialogsheet, read.
fn dialog_sheet() -> DialogSheetPart {
    DialogSheetPart::read_part(&part_bytes("/xl/dialogsheets/sheet1.xml"))
        .expect("the dialogsheet reads")
        .expect("the root is an x:dialogsheet")
}

/// One value serialized to bytes, **rebuilt from the model** — no verbatim source range anywhere.
fn rebuilt<T: ToXml>(value: &T, interner: &mut Interner) -> String {
    let element = value.to_xml(interner);
    let mut out = Vec::new();
    mjx_xml::fidelity::serialize_element(&element, interner, None, &mut out);
    String::from_utf8(out).expect("UTF-8")
}

// -------------------------------------------------------------------------------------------
// The print block on a worksheet
// -------------------------------------------------------------------------------------------

#[test]
fn print_options_report_the_file_rather_than_the_schema_defaults() {
    let sheet = sheet();
    let interner = sheet.interner();
    let options = sheet
        .print_options()
        .expect("the sheet writes x:printOptions at rank 19");

    assert_eq!(options.centred_horizontally(interner), Ok(true));
    assert_eq!(options.centred_vertically(interner), Ok(false));
    assert_eq!(options.prints_row_and_column_headings(interner), Ok(true));
    assert_eq!(options.prints_grid_lines(interner), Ok(true));
    assert_eq!(
        options.grid_lines_preference_set(interner),
        Ok(false),
        "the schema default is `true`; a reader answering the default rather than the file passes \
         the other four flags and fails here"
    );
}

#[test]
fn the_six_page_margins_are_six_different_numbers_in_inches() {
    let sheet = sheet();
    let interner = sheet.interner();
    let margins = sheet
        .page_margins()
        .expect("the sheet writes x:pageMargins at rank 20");

    assert_eq!(margins.left_inches(interner), Ok(0.7));
    assert_eq!(margins.right_inches(interner), Ok(0.75));
    assert_eq!(margins.top_inches(interner), Ok(1.0));
    assert_eq!(margins.bottom_inches(interner), Ok(1.25));
    assert_eq!(margins.header_inches(interner), Ok(0.3));
    assert_eq!(margins.footer_inches(interner), Ok(0.45));
}

#[test]
fn page_setup_reports_both_scaling_modes_and_never_reconciles_them() {
    let sheet = sheet();
    let interner = sheet.interner();
    let setup = sheet
        .page_setup()
        .expect("the sheet writes x:pageSetup at rank 21");

    assert_eq!(setup.paper_size_index(interner), Ok(9));
    assert_eq!(
        setup.paper_height(interner).expect("ok").as_deref(),
        Some("297mm")
    );
    assert_eq!(
        setup.paper_width(interner).expect("ok").as_deref(),
        Some("210mm")
    );

    // Both scaling modes at once, and both reported. `sheetPr/pageSetUpPr/@fitToPage` — a different
    // slot — is what decides which a consumer honours, and it is `1` in this fixture.
    assert_eq!(setup.scale_percentage(interner), Ok(85));
    assert_eq!(setup.pages_wide(interner), Ok(2));
    assert_eq!(
        setup.pages_tall(interner),
        Ok(0),
        "`fitToHeight=\"0\"` means \"as many pages tall as it takes\" and is not the schema default"
    );
    assert_eq!(
        sheet
            .properties()
            .expect("x:sheetPr")
            .page_setup()
            .expect("x:pageSetUpPr")
            .fit_to_page(interner),
        Ok(true)
    );

    assert_eq!(setup.first_page_number(interner), Ok(7));
    assert_eq!(setup.uses_first_page_number(interner), Ok(true));
    assert_eq!(setup.page_order(interner), Ok(PageOrder::OverThenDown));
    assert_eq!(setup.orientation(interner), Ok(PrintOrientation::Landscape));
    assert_eq!(setup.uses_printer_defaults(interner), Ok(false));
    assert_eq!(setup.prints_black_and_white(interner), Ok(true));
    assert_eq!(setup.prints_draft_quality(interner), Ok(true));
    assert_eq!(
        setup.cell_comment_printing(interner),
        Ok(CellComments::AtEnd)
    );
    assert_eq!(
        setup.error_printing(interner),
        Ok(PrintError::NotAvailable),
        "the wire token is `NA`; the generated variant is `NotAvailable`"
    );
    assert_eq!(setup.horizontal_dots_per_inch(interner), Ok(1200));
    assert_eq!(setup.vertical_dots_per_inch(interner), Ok(1200));
    assert_eq!(setup.copies(interner), Ok(3));

    // The printer-settings relationship: the identifier the file wrote, and nothing more. Resolving
    // it to a part is `mjx-xlsx`'s.
    assert_eq!(
        setup
            .relationship_id(interner, sheet.relationship_prefix())
            .expect("ok")
            .as_deref(),
        Some("rId1")
    );
}

#[test]
fn a_default_page_setup_reports_the_schema_defaults_rather_than_zero() {
    // The other half of `print_options_report_the_file_rather_than_the_schema_defaults`: an absent
    // attribute answers with the schema's default, which for `@scale` and `@copies` is not zero and
    // for `@usePrinterDefaults` is not `false`.
    let dialog = dialog_sheet();
    let interner = dialog.interner();
    let setup = dialog.page_setup().expect("x:pageSetup");

    assert_eq!(setup.scale_percentage(interner), Ok(100));
    assert_eq!(setup.pages_wide(interner), Ok(1));
    assert_eq!(setup.pages_tall(interner), Ok(1));
    assert_eq!(setup.copies(interner), Ok(1));
    assert_eq!(setup.uses_printer_defaults(interner), Ok(true));
    assert_eq!(setup.horizontal_dots_per_inch(interner), Ok(600));
    assert_eq!(setup.paper_height(interner).expect("ok"), None);
    assert_eq!(setup.error_printing(interner), Ok(PrintError::Displayed));
}

// -------------------------------------------------------------------------------------------
// Header and footer strings: opaque, and read-only through the section accessors
// -------------------------------------------------------------------------------------------

#[test]
fn the_six_header_and_footer_strings_come_back_exactly_as_written() {
    let sheet = sheet();
    let interner = sheet.interner();
    let header_footer = sheet
        .header_footer()
        .expect("the sheet writes x:headerFooter at rank 22");

    assert_eq!(header_footer.odd_and_even_pages_differ(interner), Ok(true));
    assert_eq!(header_footer.first_page_differs(interner), Ok(true));
    assert_eq!(header_footer.scales_with_document(interner), Ok(false));
    assert_eq!(header_footer.aligns_with_page_margins(interner), Ok(false));

    let text = |slot| {
        header_footer
            .string(slot)
            .unwrap_or_else(|| panic!("{slot:?} is written"))
            .text()
    };

    assert_eq!(
        text(HeaderFooterSlot::OddHeader),
        r#"&L&"Arial,Bold"Q1&C&G&R&P of &N"#
    );
    assert_eq!(text(HeaderFooterSlot::OddFooter), "&CSmith && Sons&R&D &T");
    assert_eq!(text(HeaderFooterSlot::EvenHeader), "&LEven&RPage &P");
    assert_eq!(text(HeaderFooterSlot::EvenFooter), "&C&F");
    assert_eq!(
        text(HeaderFooterSlot::FirstHeader),
        "&CAlpha&LFirst",
        "`&#65;` is a character reference and decodes to `A`; the *bytes* keep the reference"
    );
    assert_eq!(
        text(HeaderFooterSlot::FirstFooter),
        "&Rdraft",
        "a CDATA section decodes to its contents"
    );

    // Every one of the six is in the element, in `CT_HeaderFooter`'s own sequence order.
    let present: Vec<HeaderFooterSlot> = HeaderFooterSlot::ALL
        .into_iter()
        .filter(|slot| header_footer.string(*slot).is_some())
        .collect();
    assert_eq!(present.len(), 6);
}

#[test]
fn the_section_accessors_read_the_one_stored_string_and_understand_the_escape() {
    let sheet = sheet();
    let header_footer = sheet.header_footer().expect("x:headerFooter");
    let runs = |slot, section| {
        header_footer
            .string(slot)
            .expect("written")
            .section_runs(section)
            .collect::<Vec<_>>()
    };

    // `&L&"Arial,Bold"Q1&C&G&R&P of &N` — the quoted font name stays inside the left run, and the
    // `&G` is the whole of the center run.
    assert_eq!(
        runs(HeaderFooterSlot::OddHeader, HeaderFooterSection::Left),
        vec![r#"&"Arial,Bold"Q1"#]
    );
    assert_eq!(
        runs(HeaderFooterSlot::OddHeader, HeaderFooterSection::Center),
        vec!["&G"]
    );
    assert_eq!(
        runs(HeaderFooterSlot::OddHeader, HeaderFooterSection::Right),
        vec!["&P of &N"]
    );

    // `&CSmith && Sons&R&D &T` — `&&` is a literal ampersand. A scan that split on `&R` textually
    // would be right here; one that split on `&&`'s second `&` as a code would not.
    assert_eq!(
        runs(HeaderFooterSlot::OddFooter, HeaderFooterSection::Center),
        vec!["Smith && Sons"]
    );
    assert_eq!(
        runs(HeaderFooterSlot::OddFooter, HeaderFooterSection::Right),
        vec!["&D &T"]
    );
    assert!(runs(HeaderFooterSlot::OddFooter, HeaderFooterSection::Left).is_empty());

    // `&C&#65;lpha&LFirst` — the sections are written center-then-left, which §18.3.1.46 permits:
    // "the section specifiers for a header or footer can appear in any order".
    assert_eq!(
        runs(HeaderFooterSlot::FirstHeader, HeaderFooterSection::Center),
        vec!["Alpha"]
    );
    assert_eq!(
        runs(HeaderFooterSlot::FirstHeader, HeaderFooterSection::Left),
        vec!["First"]
    );

    // Only the string that carries a `&G` reports one. `&&` is not a code at all.
    assert!(header_footer
        .string(HeaderFooterSlot::OddHeader)
        .expect("written")
        .contains_drawing_reference());
    assert!(!header_footer
        .string(HeaderFooterSlot::OddFooter)
        .expect("written")
        .contains_drawing_reference());
}

#[test]
fn several_runs_for_one_section_are_reported_as_several_and_never_joined() {
    // ECMA-376 Part 1 §18.3.1.46's own example: an implementation *can* concatenate like specifiers.
    // "Can", not "shall" — so this crate reports what the file wrote, and joining is the caller's.
    let mut interner = Interner::default();
    let value = HeaderFooterText::odd_header(&mut interner, None, "&LA&CD&RG&LB&CE&RH");

    assert_eq!(
        value
            .section_runs(HeaderFooterSection::Left)
            .collect::<Vec<_>>(),
        vec!["A", "B"]
    );
    assert_eq!(
        value
            .section_runs(HeaderFooterSection::Center)
            .collect::<Vec<_>>(),
        vec!["D", "E"]
    );
    assert_eq!(
        value
            .section_runs(HeaderFooterSection::Right)
            .collect::<Vec<_>>(),
        vec!["G", "H"]
    );
    assert_eq!(value.unsectioned_text(), "");
}

#[test]
fn text_before_the_first_section_code_belongs_to_no_section() {
    let mut interner = Interner::default();

    // §18.3.1.46 requires a section specifier to *begin* with its code, so leading text is outside
    // what the spec describes. It is reported rather than assigned to a section on a guess.
    let stray = HeaderFooterText::odd_footer(&mut interner, None, "intro&Rtail");
    assert_eq!(stray.unsectioned_text(), "intro");
    assert_eq!(
        stray
            .section_runs(HeaderFooterSection::Right)
            .collect::<Vec<_>>(),
        vec!["tail"]
    );

    // A string with no section code at all is entirely unsectioned.
    let plain = HeaderFooterText::odd_footer(&mut interner, None, "plain");
    assert_eq!(plain.unsectioned_text(), "plain");
    assert_eq!(
        plain
            .section_runs(HeaderFooterSection::Center)
            .collect::<Vec<_>>(),
        Vec::<&str>::new()
    );

    // And an empty string has no runs and no unsectioned text.
    let empty = HeaderFooterText::first_footer(&mut interner, None, "");
    assert_eq!(empty.unsectioned_text(), "");
    assert!(empty
        .section_runs(HeaderFooterSection::Left)
        .next()
        .is_none());
}

#[test]
fn an_unterminated_quoted_font_name_swallows_the_rest_rather_than_finding_codes_in_it() {
    // The file's defect, reported as one run rather than as three sections invented out of a font
    // name. Nothing here repairs the string.
    let mut interner = Interner::default();
    let value = HeaderFooterText::odd_header(&mut interner, None, r#"&L&"Arial,Bold&CX"#);
    assert_eq!(
        value
            .section_runs(HeaderFooterSection::Left)
            .collect::<Vec<_>>(),
        vec![r#"&"Arial,Bold&CX"#]
    );
    assert!(value
        .section_runs(HeaderFooterSection::Center)
        .next()
        .is_none());
}

// -------------------------------------------------------------------------------------------
// Custom sheet views
// -------------------------------------------------------------------------------------------

#[test]
fn a_custom_sheet_view_reaches_four_other_clusters_rather_than_re_modelling_them() {
    let sheet = sheet();
    let interner = sheet.interner();
    let views = sheet
        .custom_sheet_views()
        .expect("the sheet writes x:customSheetViews at rank 13");
    assert_eq!(views.len(), 1);

    let view = views.views().next().expect("one saved view");
    assert_eq!(
        view.guid(interner).as_deref(),
        Ok("{2F1C7B44-9E30-4D6A-8B15-A7C3E0D95F82}")
    );
    assert_eq!(view.zoom_scale(interner), Ok(75));
    assert_eq!(view.grid_colour_index(interner), Ok(12));
    assert_eq!(view.shows_page_breaks(interner), Ok(true));
    assert_eq!(view.shows_grid_lines(interner), Ok(false));
    assert_eq!(view.fits_to_page(interner), Ok(true));
    assert_eq!(view.has_hidden_rows(interner), Ok(true));
    assert_eq!(view.filters_unique_values_only(interner), Ok(true));
    assert_eq!(view.sheet_state(interner), Ok(SheetState::Hidden));
    assert_eq!(
        view.view_type(interner),
        Ok(SheetViewType::PageBreakPreview)
    );
    assert_eq!(view.shows_ruler(interner), Ok(true), "the schema default");

    // The four clusters, each reached as the type that already models it.
    let pane = view.pane().expect("MJXOFF-102's CT_Pane");
    assert_eq!(pane.horizontal_split(interner), Ok(1.0));
    let selection = view.selection().expect("MJXOFF-102's CT_Selection");
    assert_eq!(
        selection
            .active_cell(interner)
            .map(|cell| cell.map(|c| c.to_string())),
        Ok(Some("C4".to_owned()))
    );
    assert_eq!(
        view.row_breaks()
            .expect("MJXOFF-117's CT_PageBreak")
            .manual_count(interner),
        1
    );
    assert_eq!(
        view.column_breaks()
            .expect("the same type, the other axis")
            .manual_count(interner),
        1
    );
    assert_eq!(
        view.page_margins()
            .expect("this child's CT_PageMargins")
            .left_inches(interner),
        Ok(0.25)
    );
    assert_eq!(
        view.print_options()
            .expect("this child's CT_PrintOptions")
            .prints_grid_lines(interner),
        Ok(true)
    );
    assert_eq!(
        view.page_setup()
            .expect("this child's CT_PageSetup — the full one, not CT_CsPageSetup")
            .scale_percentage(interner),
        Ok(60)
    );
    assert_eq!(
        view.header_footer()
            .expect("this child's CT_HeaderFooter")
            .string(HeaderFooterSlot::OddHeader)
            .expect("an oddHeader")
            .text(),
        r#"&C&"Arial,Bold"Saved view"#
    );
    assert_eq!(
        view.auto_filter()
            .expect("MJXOFF-123's CT_AutoFilter")
            .columns()
            .count(),
        1,
        "recorded, never applied: no row's @hidden is set from it"
    );

    // …and the view's `@hiddenRows` says nothing about the sheet's actual rows.
    assert!(
        sheet.rows().all(|row| !row.is_hidden()),
        "a saved view's memory of hidden rows is not the sheet's hidden rows"
    );
}

#[test]
fn a_new_child_of_a_custom_sheet_view_is_placed_before_the_extlst_it_already_carries() {
    // The fixture's view carries an `extLst` after its `autoFilter`, which is the whole reason it
    // is there: a `pageMargins` appended rather than *placed* lands after the `extLst` and the
    // element is out of `xsd:sequence` order.
    let mut sheet = sheet();
    let prefix = sheet.element_prefix().map(str::to_owned);
    let margins = mjx_sml::PageMargins::new(sheet.interner_mut(), prefix.as_deref());

    let views = sheet.custom_sheet_views_mut().expect("x:customSheetViews");
    let view = views.view_mut(0).expect("one view");
    view.set_page_margins(None);
    view.set_page_margins(Some(margins));

    let locals: Vec<&str> = view
        .content()
        .iter()
        .filter_map(|child| match child {
            mjx_sml::CustomSheetViewContent::Raw(mjx_ooxml_core::RawNode::Element(_)) => {
                Some("extLst")
            }
            mjx_sml::CustomSheetViewContent::Raw(_) => None,
            mjx_sml::CustomSheetViewContent::Pane(_) => Some("pane"),
            mjx_sml::CustomSheetViewContent::Selection(_) => Some("selection"),
            mjx_sml::CustomSheetViewContent::RowBreaks(_) => Some("rowBreaks"),
            mjx_sml::CustomSheetViewContent::ColumnBreaks(_) => Some("colBreaks"),
            mjx_sml::CustomSheetViewContent::PageMargins(_) => Some("pageMargins"),
            mjx_sml::CustomSheetViewContent::PrintOptions(_) => Some("printOptions"),
            mjx_sml::CustomSheetViewContent::PageSetup(_) => Some("pageSetup"),
            mjx_sml::CustomSheetViewContent::HeaderFooter(_) => Some("headerFooter"),
            mjx_sml::CustomSheetViewContent::AutoFilter(_) => Some("autoFilter"),
        })
        .collect();
    assert_eq!(
        locals,
        vec![
            "pane",
            "selection",
            "rowBreaks",
            "colBreaks",
            "pageMargins",
            "printOptions",
            "pageSetup",
            "headerFooter",
            "autoFilter",
            "extLst",
        ],
        "the re-inserted pageMargins goes back at rank 4, not after the extLst"
    );
}

// -------------------------------------------------------------------------------------------
// The whole part, and the two re-emission doors
// -------------------------------------------------------------------------------------------

#[test]
fn an_untouched_worksheet_writes_back_its_own_bytes() {
    let original = part_bytes("/xl/worksheets/sheet1.xml");
    let sheet = sheet();
    assert!(sheet.is_verbatim());
    assert_eq!(sheet.to_markup(), original);
}

#[test]
fn every_slot_this_child_typed_is_in_the_part_in_schema_order() {
    let sheet = sheet();
    let locals: Vec<&str> = sheet.child_element_locals().collect();
    assert_eq!(
        locals,
        vec![
            "sheetPr",
            "dimension",
            "sheetViews",
            "sheetFormatPr",
            "sheetData",
            "customSheetViews",
            "phoneticPr",
            "printOptions",
            "pageMargins",
            "pageSetup",
            "headerFooter",
            "rowBreaks",
            "picture",
        ],
        "every child, modelled or held, in the order the file wrote them"
    );

    // `phoneticPr` (rank 15) is the only one still held, and it stands *between* typed slots — so
    // the modelled and held slots interleave, which is the case placement has to get right.
    let held: Vec<&str> = sheet
        .children()
        .filter_map(|child| match child {
            mjx_sml::WorksheetContent::Raw(mjx_ooxml_core::RawNode::Element(element)) => {
                Some(sheet.interner().resolve(element.name.local))
            }
            _ => None,
        })
        .collect();
    assert_eq!(held, vec!["phoneticPr"]);
}

#[test]
fn an_edit_to_one_slot_leaves_every_other_slot_byte_identical() {
    let original = String::from_utf8(part_bytes("/xl/worksheets/sheet1.xml")).expect("UTF-8");
    let mut sheet = sheet();

    // Change one slot, through the one door. The clone is taken first because a setter needs the
    // part's **own** interner — an attribute name is a symbol, and a symbol interned anywhere else
    // resolves to nothing here — and the part cannot lend it while a child is borrowed.
    let mut options = sheet.print_options().expect("x:printOptions").clone();
    options.set_prints_grid_lines(sheet.interner_mut(), Some(false));
    sheet.set_print_options(Some(options));
    let written = String::from_utf8(sheet.to_markup()).expect("UTF-8");

    assert_ne!(written, original, "the edited slot must have changed");
    for untouched in [
        r#"<pageMargins left="0.7" right="0.75" top="1" bottom="1.25" header="0.3" footer="0.45"/>"#,
        r#"<oddFooter>&amp;CSmith &amp;&amp; Sons&amp;R&amp;D &amp;T</oddFooter>"#,
        r#"<firstHeader>&amp;C&#65;lpha&amp;LFirst</firstHeader>"#,
        r#"<firstFooter><![CDATA[&Rdraft]]></firstFooter>"#,
        r#"<phoneticPr fontId="1" type="noConversion"/>"#,
        r#"<picture r:id="rId2"/>"#,
    ] {
        assert!(
            written.contains(untouched),
            "an edit to x:printOptions must leave {untouched} byte-identical"
        );
    }
}

/// **The first door.** Every new model survives a rebuild *from the model alone*, with no verbatim
/// range anywhere.
///
/// An unedited part is one `memcpy` and an edited one still writes every other slot from its own
/// bytes, so no byte-identity gate ever executes an `as_raw_element`. This one does, for all six new
/// worksheet slots at once — and for a header string that means the **replay** path, which is what a
/// value read from a file uses.
#[test]
fn every_new_model_survives_a_rebuild_from_the_model_alone() {
    let mut sheet = sheet();
    // Cloned out first: `interner_mut` needs the part mutably and every child below borrows it.
    let options = sheet.print_options().expect("x:printOptions").clone();
    let margins = sheet.page_margins().expect("x:pageMargins").clone();
    let setup = sheet.page_setup().expect("x:pageSetup").clone();
    let header_footer = sheet.header_footer().expect("x:headerFooter").clone();
    let picture = sheet.background_picture().expect("x:picture").clone();
    let views = sheet
        .custom_sheet_views()
        .expect("x:customSheetViews")
        .clone();
    let interner = sheet.interner_mut();

    assert_eq!(
        rebuilt(&options, interner),
        r#"<printOptions horizontalCentered="1" verticalCentered="0" headings="1" gridLines="1" gridLinesSet="0"/>"#
    );
    assert_eq!(
        rebuilt(&margins, interner),
        r#"<pageMargins left="0.7" right="0.75" top="1" bottom="1.25" header="0.3" footer="0.45"/>"#
    );
    assert_eq!(
        rebuilt(&setup, interner),
        r#"<pageSetup paperSize="9" paperHeight="297mm" paperWidth="210mm" scale="85" firstPageNumber="7" fitToWidth="2" fitToHeight="0" pageOrder="overThenDown" orientation="landscape" usePrinterDefaults="0" blackAndWhite="1" draft="1" cellComments="atEnd" useFirstPageNumber="1" errors="NA" horizontalDpi="1200" verticalDpi="1200" copies="3" r:id="rId1"/>"#
    );
    assert_eq!(
        rebuilt(&header_footer, interner),
        concat!(
            r#"<headerFooter differentOddEven="1" differentFirst="1" scaleWithDoc="0" alignWithMargins="0">"#,
            r#"<oddHeader>&amp;L&amp;&quot;Arial,Bold&quot;Q1&amp;C&amp;G&amp;R&amp;P of &amp;N</oddHeader>"#,
            r#"<oddFooter>&amp;CSmith &amp;&amp; Sons&amp;R&amp;D &amp;T</oddFooter>"#,
            r#"<evenHeader>&amp;LEven&amp;RPage &amp;P</evenHeader>"#,
            r#"<evenFooter>&amp;C&amp;F</evenFooter>"#,
            r#"<firstHeader>&amp;C&#65;lpha&amp;LFirst</firstHeader>"#,
            r#"<firstFooter><![CDATA[&Rdraft]]></firstFooter>"#,
            "</headerFooter>"
        ),
        "the character reference and the CDATA section are replayed, not re-escaped"
    );
    assert_eq!(rebuilt(&picture, interner), r#"<picture r:id="rId2"/>"#);
    assert!(
        rebuilt(&views, interner).starts_with(
            r#"<customSheetViews><customSheetView guid="{2F1C7B44-9E30-4D6A-8B15-A7C3E0D95F82}""#
        ),
        "the saved view rebuilds with its own attributes"
    );
    assert!(rebuilt(&views, interner).ends_with("</customSheetView></customSheetViews>"));
}

/// **The second door**, and the only path on which a header string renders its own text.
///
/// MJXOFF-123 found this shape: a value read from a file **replays its stored children**, so even
/// the rebuild above never reaches the escaping branch. `set_text` is the one thing that does.
#[test]
fn a_replaced_header_string_writes_freshly_escaped_text() {
    let mut sheet = sheet();
    let mut header_footer = sheet.header_footer().expect("x:headerFooter").clone();
    header_footer
        .string_mut(HeaderFooterSlot::EvenFooter)
        .expect("an evenFooter")
        .set_text(r#"&L&"Arial,Bold"A && B&R<3"#);
    let interner = sheet.interner_mut();

    let written = rebuilt(&header_footer, interner);
    assert!(
        written.contains(
            r#"<evenFooter>&amp;L&amp;"Arial,Bold"A &amp;&amp; B&amp;R&lt;3</evenFooter>"#
        ),
        "a replaced string writes **minimally** escaped markup — `&` and `<` and nothing else, so \
         the double quotes stand as themselves — with every formatting code intact. That the \
         minimal spelling differs from the one the file used for the same string is exactly why an \
         untouched value replays its own bytes instead: {written}"
    );
    // …and the five the caller did not touch still replay their own bytes.
    assert!(written.contains(r#"<firstFooter><![CDATA[&Rdraft]]></firstFooter>"#));
    assert!(written.contains(r#"<firstHeader>&amp;C&#65;lpha&amp;LFirst</firstHeader>"#));
}

#[test]
fn a_header_footer_element_places_a_new_string_at_its_rank() {
    // All six children are the same complex type and differ only by element name, so the sequence
    // position is the *only* thing that distinguishes `oddHeader` from `firstFooter`. The rank comes
    // from the generated `HEADER_FOOTER` table.
    let mut interner = Interner::default();
    let mut header_footer = mjx_sml::HeaderFooter::new(&mut interner, None);
    for slot in [
        HeaderFooterSlot::FirstFooter,
        HeaderFooterSlot::OddFooter,
        HeaderFooterSlot::EvenHeader,
        HeaderFooterSlot::OddHeader,
    ] {
        let value = HeaderFooterText::named(&mut interner, None, slot, "x");
        header_footer.set_string(slot, Some(value));
    }

    let written = rebuilt(&header_footer, &mut interner);
    assert_eq!(
        written,
        "<headerFooter><oddHeader>x</oddHeader><oddFooter>x</oddFooter><evenHeader>x</evenHeader>\
         <firstFooter>x</firstFooter></headerFooter>",
        "inserted out of order, emitted in schema order"
    );
}

// -------------------------------------------------------------------------------------------
// The three sheet kinds that are not worksheets
// -------------------------------------------------------------------------------------------

#[test]
fn a_dialogsheet_reads_its_slots_and_writes_back_its_own_bytes() {
    let original = part_bytes("/xl/dialogsheets/sheet1.xml");
    let dialog = dialog_sheet();
    let interner = dialog.interner();

    assert!(dialog.is_verbatim());
    assert_eq!(dialog.to_markup(), original);

    assert_eq!(
        dialog
            .properties()
            .expect("x:sheetPr")
            .tab_color_element()
            .expect("a tabColor")
            .color(interner)
            .theme,
        Some(4),
        "a SpreadsheetML colour: a theme slot, which no RGB triple can express"
    );
    assert_eq!(
        dialog.sheet_views().expect("x:sheetViews").views().count(),
        1
    );
    assert_eq!(
        dialog
            .format_properties()
            .expect("x:sheetFormatPr")
            .default_row_height(interner),
        Ok(15.0)
    );
    assert_eq!(
        dialog
            .protection()
            .expect("x:sheetProtection")
            .is_protected(interner),
        Ok(true)
    );
    assert_eq!(
        dialog
            .custom_sheet_views()
            .expect("x:customSheetViews")
            .len(),
        1
    );
    assert_eq!(
        dialog
            .print_options()
            .expect("x:printOptions")
            .centred_vertically(interner),
        Ok(true)
    );
    assert_eq!(
        dialog
            .page_margins()
            .expect("x:pageMargins")
            .header_inches(interner),
        Ok(0.3)
    );
    assert_eq!(
        dialog
            .page_setup()
            .expect("x:pageSetup")
            .relationship_id(interner, dialog.relationship_prefix())
            .expect("ok")
            .as_deref(),
        Some("rId1")
    );
    assert_eq!(
        dialog
            .header_footer()
            .expect("x:headerFooter")
            .string(HeaderFooterSlot::OddHeader)
            .expect("an oddHeader")
            .text(),
        "&LDialog && Controls&R&A"
    );
    assert!(
        dialog.drawing().is_none(),
        "the fixture's dialogsheet has no x:drawing, and that is a `None` rather than an error"
    );

    assert_eq!(
        dialog.child_element_locals().collect::<Vec<_>>(),
        vec![
            "sheetPr",
            "sheetViews",
            "sheetFormatPr",
            "sheetProtection",
            "customSheetViews",
            "printOptions",
            "pageMargins",
            "pageSetup",
            "headerFooter",
        ]
    );
}

/// A chartsheet: read, reported, and written back byte for byte.
///
/// Assembled from markup here rather than committed, and deliberately: a legal `CT_Chartsheet`
/// declares `drawing` `minOccurs="1"` (`sml.xsd:2965`), so a schema-valid chartsheet drags a
/// `dml-spreadsheetDrawing` part into the corpus — and that namespace has no arm in
/// `mjx_schema_gate::categories`, whose owner is MJXOFF-107 (E3). The *markup* is what is under test
/// here, and the markup needs no package.
#[test]
fn a_chartsheet_reads_its_slots_and_writes_back_its_own_bytes() {
    const CHARTSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetPr published="0" codeName="Chart1"><tabColor indexed="13"/></sheetPr><sheetViews><sheetView tabSelected="1" zoomScale="120" workbookViewId="0" zoomToFit="1"/></sheetViews><sheetProtection algorithmName="SHA-512" hashValue="cXVpZXQ=" saltValue="c2FsdA==" spinCount="100000" content="1" objects="1"/><customSheetViews><customSheetView guid="{9B2E4D07-15CA-4F38-8E60-D3A7C1B54290}" scale="70" state="veryHidden" zoomToFit="1"><pageMargins left="0.1" right="0.1" top="0.2" bottom="0.2" header="0.05" footer="0.05"/><pageSetup paperSize="5" orientation="landscape" copies="2"/><headerFooter><oddHeader>&amp;CSaved chart view</oddHeader></headerFooter></customSheetView></customSheetViews><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><pageSetup paperSize="9" paperWidth="210mm" firstPageNumber="3" orientation="landscape" usePrinterDefaults="0" blackAndWhite="1" draft="1" useFirstPageNumber="1" horizontalDpi="300" verticalDpi="300" copies="4" r:id="rId2"/><headerFooter differentFirst="1"><oddHeader>&amp;L&amp;&quot;Arial,Bold&quot;Chart&amp;R&amp;G</oddHeader><firstHeader>&amp;C&amp;&amp;</firstHeader></headerFooter><drawing r:id="rId1"/><legacyDrawing r:id="rId3"/><picture r:id="rId4"/><webPublishItems count="1"><webPublishItem id="7" divId="chart" sourceType="chart" destinationFile="out.htm"/></webPublishItems><extLst><ext uri="{5C1D7E20-8A44-4B96-9F3D-6E0B27A4C815}"><foo xmlns="urn:example:chartsheet"/></ext></extLst></chartsheet>"#;

    let chart = ChartSheetPart::read_part(CHARTSHEET)
        .expect("the chartsheet reads")
        .expect("the root is an x:chartsheet");
    let interner = chart.interner();

    assert!(chart.is_verbatim());
    assert_eq!(chart.to_markup(), CHARTSHEET);

    let properties = chart.properties().expect("x:sheetPr");
    assert_eq!(properties.published(interner), Ok(false));
    assert_eq!(
        properties.code_name(interner).expect("ok").as_deref(),
        Some("Chart1")
    );
    assert_eq!(
        properties
            .tab_color_element()
            .expect("a tabColor")
            .color(interner)
            .indexed,
        Some(13),
        "an indexed palette slot — a SpreadsheetML colour, not a DrawingML one"
    );

    let view = chart
        .sheet_views()
        .expect("x:sheetViews")
        .views()
        .next()
        .expect("one sheetView");
    assert_eq!(view.tab_selected(interner), Ok(true));
    assert_eq!(view.zoom_scale(interner), Ok(120));
    assert_eq!(view.workbook_view_index(interner), Ok(0));
    assert_eq!(view.zooms_to_fit_window(interner), Ok(true));

    let protection = chart.protection().expect("x:sheetProtection");
    assert_eq!(
        protection
            .hash_algorithm_name(interner)
            .expect("ok")
            .as_deref(),
        Some("SHA-512")
    );
    assert_eq!(
        protection.password_hash(interner).expect("ok").as_deref(),
        Some("cXVpZXQ="),
        "a preserved hash: never verified, never recomputed, never cleared"
    );
    assert_eq!(protection.protects_content(interner), Ok(true));
    assert_eq!(protection.protects_objects(interner), Ok(true));

    let saved = chart
        .custom_sheet_views()
        .expect("x:customSheetViews")
        .views()
        .next()
        .expect("one CT_CustomChartsheetView");
    assert_eq!(saved.zoom_scale(interner), Ok(70));
    assert_eq!(saved.sheet_state(interner), Ok(SheetState::VeryHidden));
    assert_eq!(
        saved
            .page_setup()
            .expect("a CT_CsPageSetup")
            .copies(interner),
        Ok(2)
    );
    assert!(
        saved.header_footer().is_some() && saved.page_margins().is_some(),
        "all three of CT_CustomChartsheetView's children are reachable"
    );

    // A chartsheet's own `pageSetup` is `CT_CsPageSetup`: it has no `@scale`, no `@fitToWidth` and
    // no `@errors`, because a chart has no grid — so those accessors do not exist on this type.
    let setup = chart.page_setup().expect("x:pageSetup");
    assert_eq!(setup.paper_size_index(interner), Ok(9));
    assert_eq!(
        setup.paper_width(interner).expect("ok").as_deref(),
        Some("210mm")
    );
    assert_eq!(setup.first_page_number(interner), Ok(3));
    assert_eq!(setup.copies(interner), Ok(4));
    assert_eq!(
        setup
            .relationship_id(interner, chart.relationship_prefix())
            .expect("ok")
            .as_deref(),
        Some("rId2")
    );

    assert_eq!(
        chart
            .header_footer()
            .expect("x:headerFooter")
            .string(HeaderFooterSlot::FirstHeader)
            .expect("a firstHeader")
            .text(),
        "&C&&",
        "a section whose entire run is one literal ampersand"
    );

    // The relationship the chart hangs off — reported, never opened.
    assert_eq!(
        chart
            .drawing()
            .expect("x:drawing is minOccurs=1")
            .relationship_id(interner, chart.relationship_prefix())
            .expect("ok")
            .as_deref(),
        Some("rId1")
    );
    assert_eq!(
        chart
            .background_picture()
            .expect("x:picture")
            .relationship_id(interner, chart.relationship_prefix())
            .expect("ok")
            .as_deref(),
        Some("rId4")
    );
    assert_eq!(
        chart.web_publish_items().expect("x:webPublishItems").len(),
        1
    );

    // `legacyDrawing` (rank 8) and `extLst` (rank 13) are held, in position.
    assert_eq!(
        chart.child_element_locals().collect::<Vec<_>>(),
        vec![
            "sheetPr",
            "sheetViews",
            "sheetProtection",
            "customSheetViews",
            "pageMargins",
            "pageSetup",
            "headerFooter",
            "drawing",
            "legacyDrawing",
            "picture",
            "webPublishItems",
            "extLst",
        ]
    );
}

#[test]
fn an_edit_to_a_chartsheet_slot_leaves_every_other_slot_byte_identical() {
    const CHARTSHEET: &[u8] = br#"<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetViews><sheetView workbookViewId="0"/></sheetViews><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><headerFooter><oddHeader>&amp;C&amp;#65;lpha</oddHeader></headerFooter><drawing r:id="rId1"/><extLst><ext uri="{5C1D7E20-8A44-4B96-9F3D-6E0B27A4C815}"><foo xmlns="urn:example:chartsheet"/></ext></extLst></chartsheet>"#;

    let mut chart = ChartSheetPart::read_part(CHARTSHEET)
        .expect("reads")
        .expect("an x:chartsheet");
    let mut margins = chart.page_margins().expect("x:pageMargins").clone();
    margins.set_left_inches(chart.interner_mut(), 0.25);
    chart.set_page_margins(Some(margins));
    let written = String::from_utf8(chart.to_markup()).expect("UTF-8");

    assert!(written.contains(r#"left="0.25""#), "{written}");
    for untouched in [
        r#"<oddHeader>&amp;C&amp;#65;lpha</oddHeader>"#,
        r#"<drawing r:id="rId1"/>"#,
        r#"<ext uri="{5C1D7E20-8A44-4B96-9F3D-6E0B27A4C815}"><foo xmlns="urn:example:chartsheet"/></ext>"#,
    ] {
        assert!(
            written.contains(untouched),
            "an edit to x:pageMargins must leave {untouched} byte-identical: {written}"
        );
    }
}

/// A macrosheet reads, reports its twenty typed slots, holds its `sheetData` and round-trips.
///
/// `sml.xsd` declares no global element for `CT_Macrosheet`, so this part is reached by its **root
/// element** rather than by a content type — see [`mjx_sml::sheets::macrosheet`].
#[test]
fn a_macrosheet_reads_holds_its_sheet_data_and_writes_back_its_own_bytes() {
    const MACROSHEET: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<macrosheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetPr codeName="Macro1"/><dimension ref="A1:A3"/><sheetViews><sheetView workbookViewId="0"/></sheetViews><sheetData><row r="1"><c r="A1" t="str"><f>RESULT(1)</f><v>1</v></c></row></sheetData><customSheetViews><customSheetView guid="{4E7A0C31-B268-49D5-A03F-8C51E6B27D94}" scale="90"/></customSheetViews><printOptions headings="1"/><pageMargins left="0.5" right="0.5" top="0.5" bottom="0.5" header="0.2" footer="0.2"/><pageSetup orientation="portrait" fitToWidth="3"/><headerFooter><oddHeader>&amp;LMacro &amp;&amp; XLM</oddHeader></headerFooter><rowBreaks count="1" manualBreakCount="1"><brk id="2" max="16383" man="1"/></rowBreaks><picture r:id="rId1"/></macrosheet>"#;

    let macro_sheet = MacroSheetPart::read_part(MACROSHEET)
        .expect("the macrosheet reads")
        .expect("the root is a macrosheet");
    let interner = macro_sheet.interner();

    assert!(macro_sheet.is_verbatim());
    assert_eq!(macro_sheet.to_markup(), MACROSHEET);

    assert_eq!(
        macro_sheet
            .properties()
            .expect("x:sheetPr")
            .code_name(interner)
            .expect("ok")
            .as_deref(),
        Some("Macro1")
    );
    assert_eq!(
        macro_sheet
            .page_setup()
            .expect("x:pageSetup")
            .pages_wide(interner),
        Ok(3),
        "a macrosheet's pageSetup is the full CT_PageSetup, unlike a chartsheet's"
    );
    assert_eq!(
        macro_sheet
            .header_footer()
            .expect("x:headerFooter")
            .string(HeaderFooterSlot::OddHeader)
            .expect("an oddHeader")
            .text(),
        "&LMacro && XLM"
    );
    assert_eq!(
        macro_sheet
            .row_breaks()
            .expect("x:rowBreaks")
            .manual_count(interner),
        1
    );
    assert_eq!(macro_sheet.custom_sheet_views().expect("views").len(), 1);

    // `sheetData` is held, not stored: there is no cell accessor on this type at all.
    let held: Vec<&str> = macro_sheet
        .children()
        .filter_map(|child| match child {
            mjx_sml::MacroSheetContent::Raw(mjx_ooxml_core::RawNode::Element(element)) => {
                Some(interner.resolve(element.name.local))
            }
            _ => None,
        })
        .collect();
    assert_eq!(held, vec!["sheetData"]);
}

#[test]
fn a_part_of_the_wrong_kind_is_a_question_rather_than_an_error() {
    // Every reader answers `Ok(None)` for a root it does not recognise, exactly as
    // `WorksheetPart::read_document` does — the caller handed over a different part.
    let worksheet = part_bytes("/xl/worksheets/sheet1.xml");
    assert!(ChartSheetPart::read_part(&worksheet)
        .expect("no error")
        .is_none());
    assert!(DialogSheetPart::read_part(&worksheet)
        .expect("no error")
        .is_none());
    assert!(MacroSheetPart::read_part(&worksheet)
        .expect("no error")
        .is_none());

    let dialogsheet = part_bytes("/xl/dialogsheets/sheet1.xml");
    assert!(WorksheetPart::read_part(&dialogsheet)
        .expect("no error")
        .is_none());
    assert!(ChartSheetPart::read_part(&dialogsheet)
        .expect("no error")
        .is_none());
}
