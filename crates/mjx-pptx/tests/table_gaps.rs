//! Integration tests for the smaller table gaps (MJX-43): accessibility `headers`, `visible_cell_text`,
//! and `graphic_frame_kind`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_opc::Package;
use mjx_pptx::{Cells, GraphicFrameKind, Presentation, ShapeBounds};

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

fn deck_with_table(rows: usize, columns: usize) -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(
            0,
            rows,
            columns,
            ShapeBounds::from_inches(0.5, 1.0, 8.0, 3.0),
        )
        .expect("add table");
    (pres, table)
}

// ---------------------------------------------------------------------------------------------
// headers (accessibility)
// ---------------------------------------------------------------------------------------------

#[test]
fn cell_header_associations_round_trip() {
    let (mut pres, table) = deck_with_table(2, 2);
    assert!(
        pres.cell_headers(0, table, 1, 1).expect("read").is_empty(),
        "a fresh cell names no headers"
    );

    pres.set_cell_headers(0, table, 1, 1, &["hRegion", "hYear"])
        .expect("set headers");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.cell_headers(0, table, 1, 1).expect("read"),
        vec!["hRegion".to_owned(), "hYear".to_owned()]
    );

    // Clearing removes the association.
    reopened
        .set_cell_headers(0, table, 1, 1, &[])
        .expect("clear");
    assert!(reopened
        .cell_headers(0, table, 1, 1)
        .expect("read")
        .is_empty());
}

#[test]
fn setting_headers_dirties_only_the_slide() {
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("baseline"));
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    // The fixture's slide 2 holds a one-cell table.
    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("a table on slide 2");
    pres.set_cell_headers(1, table, 0, 0, &["h1"]).expect("set");

    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for (name, original) in &before {
        if name == "ppt/slides/slide2.xml" {
            assert_ne!(after.get(name), Some(original), "the slide changed");
        } else {
            assert_eq!(after.get(name), Some(original), "dirtied {name}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// visible_cell_text
// ---------------------------------------------------------------------------------------------

#[test]
fn visible_cell_text_follows_a_merge_to_its_anchor() {
    let (mut pres, table) = deck_with_table(3, 3);
    for row in 0..3 {
        for column in 0..3 {
            pres.set_cell_text(0, table, row, column, 0, &format!("{row}{column}"))
                .expect("text");
        }
    }
    pres.merge_cells(0, table, Cells::rectangle(0..1, 0..2))
        .expect("merge the first two header cells");

    // The covered cell keeps its own (hidden) text, but what renders there is the anchor's.
    assert_eq!(pres.cell_text(0, table, 0, 1).expect("own"), "01");
    assert_eq!(
        pres.visible_cell_text(0, table, 0, 1).expect("visible"),
        "00",
        "the covered cell shows the anchor's text"
    );
    // An unmerged cell shows its own text.
    assert_eq!(
        pres.visible_cell_text(0, table, 2, 2).expect("visible"),
        "22"
    );
}

// ---------------------------------------------------------------------------------------------
// graphic_frame_kind
// ---------------------------------------------------------------------------------------------

#[test]
fn graphic_frame_kind_reports_a_table_and_none_for_a_shape() {
    let (mut pres, table) = deck_with_table(2, 2);
    assert_eq!(
        pres.graphic_frame_kind(0, table).expect("kind"),
        Some(GraphicFrameKind::Table)
    );

    // A text box is a `p:sp`, not a graphic frame at all.
    let text_box = pres
        .add_text_box(0, "hi", ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0))
        .expect("add text box");
    assert_eq!(
        pres.graphic_frame_kind(0, text_box).expect("kind"),
        None,
        "a non-frame shape has no graphic-frame kind"
    );
}
