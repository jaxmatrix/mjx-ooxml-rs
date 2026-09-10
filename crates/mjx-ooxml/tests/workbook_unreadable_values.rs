//! A value the file states and this library cannot read is **not** a blank (MJXOFF-285).
//!
//! `<c t="n"><v>not-a-number</v></c>` is markup no ECMA-376 simple type admits and that real
//! producers nevertheless write. Until this suite existed the facade read it back as
//! [`CellData::Blank`] — the same answer it gives a cell that holds nothing at all — so a caller saw
//! an empty cell where the file held data, with nothing anywhere to say a value had been dropped.
//!
//! Every case below therefore asserts **the discrimination**, not just the new variant: the
//! unreadable cell and the empty cell beside it are read in the same call, from the same row, and
//! compared against each other. A reader that answered `Blank` for both would pass an assertion that
//! only looked at one of them.
//!
//! Like `format_detection.rs`, this file names a crate below the facade — and only to *build* the
//! input. No authoring call in this workspace will write a token its own schema refuses, which is
//! the point: the input has to come from bytes.

use mjx_ooxml::{CellData, Workbook};
use mjx_opc::{Package, PartName};

/// A one-sheet workbook whose first row is exactly `cells`.
///
/// Authored by rewriting the worksheet payload of a real package, so everything else about the file
/// — its content types, its relationships, its workbook part — is what this library itself writes.
fn workbook_whose_first_row_is(cells: &str) -> Vec<u8> {
    let saved = Workbook::blank()
        .expect("a blank workbook")
        .save()
        .expect("saving it");
    let part = Workbook::open(&saved)
        .expect("reopening it")
        .sheet(0)
        .expect("the first tab")
        .part
        .expect("its worksheet part");

    let markup = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n<worksheet \
         xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData><row \
         r=\"1\">{cells}</row></sheetData></worksheet>"
    );

    let mut package = Package::open(&saved).expect("the package");
    package
        .replace_part_bytes(
            &PartName::new(&part).expect("the worksheet part name"),
            markup.into_bytes(),
        )
        .expect("rewriting the worksheet");
    package.save_unchecked().expect("re-saving the package")
}

/// The row every case reads: six cells, six different answers, and the two that matter sit side by
/// side.
const ROW: &str = concat!(
    // A number the file states and no reader can parse.
    r#"<c r="A1" t="n"><v>not-a-number</v></c>"#,
    // A cell that states nothing at all. This is what a blank *is*.
    r#"<c r="B1"/>"#,
    // A number that reads, so a suite cannot pass by reporting every cell as unreadable.
    r#"<c r="C1" t="n"><v>42</v></c>"#,
    // A boolean whose `<v>` is neither `1` nor `0`.
    r#"<c r="D1" t="b"><v>maybe</v></c>"#,
    // A shared string whose `<v>` is not an index into anything.
    r#"<c r="E1" t="s"><v>seven</v></c>"#,
    // An `inlineStr` that wrote a `<v>` where its `<is>` belongs.
    r#"<c r="F1" t="inlineStr"><v>loose</v></c>"#,
);

#[test]
fn a_number_the_file_states_and_we_cannot_parse_is_not_a_blank() {
    let bytes = workbook_whose_first_row_is(ROW);
    let workbook = Workbook::open(&bytes).expect("opening the workbook");
    let block = workbook.read_range(0, "A1:B1").expect("the two cells");

    let stated = block.value(0, 0).expect("A1");
    let empty = block.value(0, 1).expect("B1");

    // The claim, in the only form that means anything: a caller can tell the two apart.
    assert_ne!(
        stated, empty,
        "a cell holding an unparseable number must not read the same as a cell holding nothing"
    );
    assert_eq!(stated, &CellData::Unreadable("not-a-number".to_owned()));
    assert_eq!(empty, &CellData::Blank);

    // And apart through every accessor, not only through `PartialEq`.
    assert!(!stated.is_blank(), "the file states a value here");
    assert!(empty.is_blank(), "the file states nothing here");
    assert_eq!(stated.unreadable_text(), Some("not-a-number"));
    assert_eq!(empty.unreadable_text(), None);

    // It is not smuggled in as one of the readable kinds either: `number` would be a repair and
    // `text` would be a claim about a kind of cell this is not.
    assert_eq!(stated.number(), None);
    assert_eq!(stated.text(), None);
    assert_eq!(stated.error_code(), None);
}

/// The three other cell types that can state a token their own `c@t` cannot read, and the readable
/// number that proves the reader has not simply given up on the row.
#[test]
fn every_cell_type_that_can_state_an_unreadable_token_reports_it() {
    let bytes = workbook_whose_first_row_is(ROW);
    let workbook = Workbook::open(&bytes).expect("opening the workbook");
    let block = workbook.read_range(0, "A1:F1").expect("the whole row");

    assert_eq!(
        block.value(0, 2).expect("C1"),
        &CellData::Number(42.0),
        "a number that reads must still read"
    );
    assert_eq!(
        block.value(0, 3).expect("D1"),
        &CellData::Unreadable("maybe".to_owned()),
        "a `t=\"b\"` whose value is neither 1 nor 0"
    );
    assert_eq!(
        block.value(0, 4).expect("E1"),
        &CellData::Unreadable("seven".to_owned()),
        "a `t=\"s\"` whose value is not an index"
    );
    assert_eq!(
        block.value(0, 5).expect("F1"),
        &CellData::Unreadable("loose".to_owned()),
        "an `inlineStr` that wrote a `<v>` instead of an `<is>`"
    );
}

/// Reading is not an edit: reporting the token changes nothing about the bytes it was read from.
///
/// The reason the reader is allowed to *report* rather than refuse or repair — the file keeps saying
/// exactly what it said, and a caller who never touches this cell writes it back verbatim.
#[test]
fn reporting_the_token_does_not_rewrite_it() {
    let bytes = workbook_whose_first_row_is(ROW);
    let workbook = Workbook::open(&bytes).expect("opening the workbook");
    let read = workbook.read_range(0, "A1:F1").expect("the whole row");
    assert!(read.value(0, 0).expect("A1").unreadable_text().is_some());

    let saved = workbook.save().expect("saving without editing anything");
    let reopened = Workbook::open(&saved).expect("reopening");
    let part = reopened
        .sheet(0)
        .expect("the first tab")
        .part
        .expect("its worksheet part");
    let markup = String::from_utf8(reopened.part_bytes(&part).expect("the worksheet bytes"))
        .expect("the worksheet is utf-8");

    assert!(
        markup.contains("<v>not-a-number</v>"),
        "the token must survive a save that did not edit it: {markup}"
    );
    assert_eq!(
        reopened
            .read_range(0, "A1")
            .expect("A1 again")
            .value(0, 0)
            .expect("A1"),
        &CellData::Unreadable("not-a-number".to_owned())
    );
}
