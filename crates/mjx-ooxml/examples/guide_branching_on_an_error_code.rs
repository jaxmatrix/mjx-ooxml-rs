//! The guide's **Eleven codes, and what each one means you should do** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/errors.md`][guide] shows this code in three languages, and every
//! one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/branching_on_an_error_code.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/branching_on_an_error_code.mjs`. `cargo run -p
//! xtask -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was
//! done.
//!
//! # The three differ in shape, and the page says why
//!
//! Both of this page's differences meet here: an [`mjx_ooxml::ErrorCode`] is one enumeration in
//! Rust, eleven exception classes in Python and eleven strings in JavaScript; and an
//! [`mjx_ooxml::CellInput`] becomes a static [`mjx_ooxml::CellWrite`] constructor in both bindings.
//!
//! The last assertion is the one worth keeping: **a batch that would fail halfway is refused before
//! the package is touched at all**, so three failed writes leave the workbook exactly as it was.
//! This example saves nothing precisely because nothing was written.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_branching_on_an_error_code
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{CellInput, CellWrite, ErrorCode, Workbook};

    let mut workbook = Workbook::blank()?;

    // An address that does not parse: refused before the worksheet is even opened.
    let bad_address = workbook
        .write_cells(0, &[CellWrite::new("not-a-cell", CellInput::Number(1.0))])
        .expect_err("`not-a-cell` is not an A1 reference");
    assert_eq!(bad_address.code(), ErrorCode::InvalidArgument);

    // A value SpreadsheetML has no spelling for.
    let unrepresentable = workbook
        .write_cells(0, &[CellWrite::new("A1", CellInput::Number(f64::NAN))])
        .expect_err("SpreadsheetML cannot spell NaN");
    assert_eq!(unrepresentable.code(), ErrorCode::InvalidArgument);

    // A tab that is not there.
    let missing = workbook.read_range(9, "A1").expect_err("one sheet");
    assert_eq!(missing.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(missing.detail().index, Some(9));

    // And the contract worth knowing: a batch that would fail halfway is refused before the
    // package is touched at all, so nothing above wrote anything.
    assert!(workbook.read_sheet(0)?.is_empty());
    // guide-example:end

    println!("three refusals, and the workbook is still empty");
    Ok(())
}
