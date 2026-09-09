//! The guide's **Opening detects first, then parses once** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/opening_and_saving.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/opening_the_wrong_surface.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/opening_the_wrong_surface.mjs`. `cargo run -p xtask
//! -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # The three differ in shape, and the page says why
//!
//! A failure is one [`mjx_ooxml::Error`] with an [`mjx_ooxml::ErrorCode`] here; in Python it is one
//! of eleven exception classes, and in JavaScript a real `Error` carrying the code's stable
//! spelling as a string on `.code`. All three branch on the same eleven classifications — only the
//! spelling of *catching* differs, because `except` selects on the class in Python and on nothing
//! in JavaScript.
//!
//! This example **saves nothing** — it refuses to open something — so the two binding harnesses
//! have no package to compare and skip that half of their work by construction.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_opening_the_wrong_surface
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

// guide-example:packages none
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let workbook_bytes = mjx_fixtures::fixture("sample.xlsx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::{Deck, ErrorCode};

    // `workbook_bytes` is a spreadsheet, and `Deck::open` detects that before it parses anything.
    let failure = Deck::open(&workbook_bytes).expect_err("a workbook is not a deck");
    assert_eq!(failure.code(), ErrorCode::UnsupportedFormat);

    // The message names the constructor that would have worked, rather than complaining about a
    // `presentation.xml` that was never there.
    assert!(failure.message().contains("Workbook"));
    // guide-example:end

    println!("{}", failure.message());
    Ok(())
}
