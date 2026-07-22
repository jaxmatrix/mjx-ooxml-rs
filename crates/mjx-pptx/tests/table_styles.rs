//! Integration tests for table styles (`tableStyles.xml`) and the seven `a:tblPr` flags.
//!
//! The flags live on a table and mean nothing alone — they tell a *style* which parts to emphasize.
//! The style lives in the presentation's shared `tableStyles.xml` part, named by GUID. These tests
//! drive the whole loop: set the flags, author a style and format its parts, point a table at it, and
//! resolve it back — while holding to the round-trip contract (only the parts we touch change).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    ColorSpec, FillSpec, LineSpec, OnOffStyle, TablePart, TableStyleBorder, TableStylePart,
};
use mjx_opc::Package;
use mjx_pptx::{Presentation, ShapeBounds, TableStyleFormat};

const GUID: &str = "{7C9E6A1B-4D2F-4A55-9E3C-1122334455AA}";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn deck_with_table() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0))
        .expect("add table");
    (pres, table)
}

// ---------------------------------------------------------------------------------------------
// The seven a:tblPr flags
// ---------------------------------------------------------------------------------------------

#[test]
fn a_created_table_states_the_flags_powerpoint_writes() {
    let (mut pres, table) = deck_with_table();
    // `add_table` writes `firstRow="1" bandRow="1"`, as PowerPoint does for a new table.
    assert_eq!(
        pres.table_part(0, table, TablePart::FirstRow)
            .expect("read"),
        Some(true)
    );
    assert_eq!(
        pres.table_part(0, table, TablePart::BandedRows)
            .expect("read"),
        Some(true)
    );
    assert_eq!(
        pres.table_part(0, table, TablePart::LastRow).expect("read"),
        None,
        "an unstated flag reads as None, not false"
    );
}

#[test]
fn a_flag_can_be_turned_on_and_off() {
    let (mut pres, table) = deck_with_table();
    pres.set_table_part(0, table, TablePart::LastColumn, true)
        .expect("set");
    assert_eq!(
        pres.table_part(0, table, TablePart::LastColumn)
            .expect("read"),
        Some(true)
    );

    // Off removes the flag rather than writing "0".
    pres.set_table_part(0, table, TablePart::FirstRow, false)
        .expect("clear");
    assert_eq!(
        pres.table_part(0, table, TablePart::FirstRow)
            .expect("read"),
        None
    );
}

// ---------------------------------------------------------------------------------------------
// Assigning and authoring styles
// ---------------------------------------------------------------------------------------------

#[test]
fn a_table_can_be_pointed_at_a_style() {
    let (mut pres, table) = deck_with_table();
    assert_eq!(pres.table_style_id(0, table).expect("read"), None);

    pres.set_table_style(0, table, GUID).expect("assign");
    assert_eq!(
        pres.table_style_id(0, table).expect("read"),
        Some(GUID.to_owned())
    );
}

#[test]
fn a_style_can_be_authored_formatted_and_resolved() {
    let (mut pres, table) = deck_with_table();

    pres.create_table_style(GUID, "Report Style")
        .expect("create");
    pres.format_table_style_part(
        GUID,
        TableStylePart::FirstRow,
        &TableStyleFormat::new()
            .with_bold(OnOffStyle::On)
            .with_text_color(ColorSpec::Srgb("FFFFFF".to_owned()))
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".to_owned())))
            .with_border(TableStyleBorder::Bottom, LineSpec::default()),
    )
    .expect("format the header part");
    pres.set_table_style(0, table, GUID).expect("assign");

    // Survives a save and reopen, and resolves from the table back through tableStyles.xml.
    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");

    let resolved = reopened
        .with_table_style(0, table, |style, interner| {
            let header = style
                .part(interner, TableStylePart::FirstRow)
                .expect("firstRow part");
            let text = header.text_style(interner).expect("tcTxStyle");
            let cell = header.cell_style(interner).expect("tcStyle");
            Ok((
                text.bold(interner),
                text.color(interner).is_some(),
                cell.fill(interner).is_some(),
                cell.borders(interner)
                    .and_then(|b| b.border(interner, TableStyleBorder::Bottom))
                    .is_some(),
                style.style_name(interner).map(str::to_owned),
            ))
        })
        .expect("resolve")
        .expect("a style is resolved");

    assert_eq!(resolved.0, OnOffStyle::On, "header is bold");
    assert!(resolved.1, "header has a text colour");
    assert!(resolved.2, "header has a fill");
    assert!(resolved.3, "header has a bottom border");
    assert_eq!(resolved.4.as_deref(), Some("Report Style"));
}

#[test]
fn the_table_styles_part_is_created_once_and_shared() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.create_table_style(GUID, "First").expect("first");
    let other = "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}";
    pres.create_table_style(other, "Second").expect("second");

    let saved = pres.save().expect("save");
    let pkg = Package::open(&saved).expect("reopen package");
    let style_parts: Vec<_> = pkg
        .entries()
        .iter()
        .filter(|e| e.name.ends_with("tableStyles.xml"))
        .collect();
    assert_eq!(style_parts.len(), 1, "one shared part, not one per style");

    // Both styles are defined in the one shared part.
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let table = reopened
        .add_table(0, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 4.0, 2.0))
        .expect("add table");
    reopened
        .set_table_style(0, table, other)
        .expect("assign the second style");
    assert_eq!(
        reopened.table_style_id(0, table).expect("read"),
        Some(other.to_owned())
    );
}

#[test]
fn formatting_an_undefined_style_is_refused() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    // No tableStyles.xml at all.
    assert!(matches!(
        pres.format_table_style_part(GUID, TableStylePart::WholeTable, &TableStyleFormat::new()),
        Err(mjx_pptx::PptxError::TableStyleNotFound { .. })
    ));
    // A part exists, but not this GUID.
    pres.create_table_style("{00000000-0000-0000-0000-000000000000}", "Other")
        .expect("create other");
    assert!(matches!(
        pres.format_table_style_part(GUID, TableStylePart::WholeTable, &TableStyleFormat::new()),
        Err(mjx_pptx::PptxError::TableStyleNotFound { .. })
    ));
}

#[test]
fn resolving_a_table_that_names_no_style_is_none() {
    let (mut pres, table) = deck_with_table();
    assert!(
        pres.with_table_style(0, table, |_, _| Ok(()))
            .expect("resolve")
            .is_none(),
        "a table with no tableStyleId resolves to nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn authoring_a_style_leaves_unrelated_parts_untouched() {
    let before = byte_map(&Package::open(&fixture("sample.pptx")).expect("baseline"));

    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.create_table_style(GUID, "Report Style")
        .expect("create");
    pres.format_table_style_part(
        GUID,
        TableStylePart::WholeTable,
        &TableStyleFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("EEEEEE".to_owned()))),
    )
    .expect("format");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // Authoring a style touches exactly three things: the content-types manifest (a new Override),
    // the presentation's relationships (a new relationship to the part), and the new part itself.
    // Every other part the fixture already had — the slide, layout, master, theme — is byte-identical.
    for (name, original) in &before {
        if name == "[Content_Types].xml" || name.ends_with("presentation.xml.rels") {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "dirtied {name}");
    }
    assert!(
        after.keys().any(|n| n.ends_with("tableStyles.xml")),
        "the new part is present"
    );
    assert!(
        after.contains_key("[Content_Types].xml")
            && after.get("[Content_Types].xml") != before.get("[Content_Types].xml"),
        "the content-types manifest gained the tableStyles override"
    );
}

// ---------------------------------------------------------------------------------------------
// The committed fixture: a deck carrying a real tableStyles.xml + a table naming its style.
// ---------------------------------------------------------------------------------------------

const FIXTURE_STYLE: &str = "{2E9B8F31-6A47-4C8D-B0A2-9F4E7D3C1B65}";

/// Regenerates `tests/fixtures/tables.pptx`. Run with `cargo test -p mjx-pptx --test table_styles
/// -- --ignored generate_tables_fixture`. Kept as a test so the fixture is reproducible from source.
#[test]
#[ignore = "regenerates the committed tables.pptx fixture"]
fn generate_tables_fixture() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 3, 3, ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0))
        .expect("add table");
    for (row, column, text) in [
        (0, 0, "Region"),
        (0, 1, "Revenue"),
        (0, 2, "Change"),
        (1, 0, "North"),
        (1, 1, "1,204"),
        (1, 2, "+12%"),
        (2, 0, "South"),
        (2, 1, "987"),
        (2, 2, "-3%"),
    ] {
        pres.set_cell_text(0, table, row, column, 0, text)
            .expect("cell text");
    }

    pres.set_table_part(0, table, TablePart::FirstRow, true)
        .expect("firstRow flag");
    pres.set_table_part(0, table, TablePart::BandedRows, true)
        .expect("bandRow flag");

    pres.create_table_style(FIXTURE_STYLE, "Report Style")
        .expect("create style");
    pres.format_table_style_part(
        FIXTURE_STYLE,
        TableStylePart::WholeTable,
        &TableStyleFormat::new()
            .with_border(TableStyleBorder::InsideHorizontal, LineSpec::default()),
    )
    .expect("whole-table borders");
    pres.format_table_style_part(
        FIXTURE_STYLE,
        TableStylePart::FirstRow,
        &TableStyleFormat::new()
            .with_bold(OnOffStyle::On)
            .with_text_color(ColorSpec::Srgb("FFFFFF".to_owned()))
            .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".to_owned()))),
    )
    .expect("header style");
    pres.format_table_style_part(
        FIXTURE_STYLE,
        TableStylePart::Band1Horizontal,
        &TableStyleFormat::new().with_fill(FillSpec::solid(ColorSpec::Srgb("D9E1F2".to_owned()))),
    )
    .expect("banded style");
    pres.set_table_style(0, table, FIXTURE_STYLE)
        .expect("assign");

    let bytes = pres.save().expect("save");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tables.pptx");
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
}

#[test]
fn the_tables_fixture_resolves_its_style_and_reading_dirties_nothing() {
    let bytes = fixture("tables.pptx");
    let before = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let count = pres.shape_count(0).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(0, idx).is_ok())
        .expect("a table on slide 0");

    assert_eq!(
        pres.table_style_id(0, table).expect("id"),
        Some(FIXTURE_STYLE.to_owned())
    );
    assert_eq!(
        pres.table_part(0, table, TablePart::FirstRow)
            .expect("flag"),
        Some(true)
    );

    let (bold, header_fill, name) = pres
        .with_table_style(0, table, |style, interner| {
            let header = style.part(interner, TableStylePart::FirstRow);
            Ok((
                header
                    .as_ref()
                    .and_then(|p| p.text_style(interner))
                    .map(|t| t.bold(interner)),
                header
                    .and_then(|p| p.cell_style(interner))
                    .and_then(|c| c.fill(interner))
                    .is_some(),
                style.style_name(interner).map(str::to_owned),
            ))
        })
        .expect("resolve")
        .expect("style resolves");
    assert_eq!(bold, Some(OnOffStyle::On));
    assert!(header_fill, "the header part has a fill");
    assert_eq!(name.as_deref(), Some("Report Style"));

    // Reading resolved the style but must not have dirtied a single part.
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(after, before, "reading a styled table dirtied a part");
}
