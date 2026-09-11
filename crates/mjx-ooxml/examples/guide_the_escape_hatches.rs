//! The guide's **The three escape hatches** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/the_curated_surface.md`][guide] shows this code, and the block a
//! reader sees is a *copy* of this file — `cargo run -p xtask -- guide-examples` does the copying
//! and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # This one is Rust-only, and it is the only kind of example that may be
//!
//! Every other guide example exists three times: here, under
//! `bindings/mjx-python/tests/guide_examples/` and under
//! `bindings/mjx-wasm/tests/node/guide_examples/`. This one cannot, and the marker beside its block
//! says so *and says why*: [`mjx_ooxml::Deck::presentation_mut`],
//! [`mjx_ooxml::Document::document_mut`] and [`mjx_ooxml::Workbook::workbook_mut`] are declared by
//! neither binding, and `xtask/tests/guide_examples.rs` reads both binding surfaces on every run to
//! confirm that is still true (MJXOFF-261, MJXOFF-257). The day one of them is projected, the
//! declaration reddens and this example owes two more halves.
//!
//! **A hatch hands back the real value, not a copy.** That is the whole claim, and the assertion in
//! the region below is what states it: an edit made through `mjx_xlsx::Workbook` is visible through
//! the facade's own reader immediately afterwards, with nothing re-opened and no bytes round
//! tripped in between.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_the_escape_hatches
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

// guide-example:packages none
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let original = mjx_fixtures::fixture("sample.xlsx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::Workbook;

    let mut workbook = Workbook::open(&original)?;

    // The per-sheet editing loop the facade deliberately does not offer: hold the parsed worksheet
    // yourself between one read and one write, and pay for the parse once.
    let inner = workbook.workbook_mut();
    let mut markup = inner.worksheet_markup(0)?.expect("a worksheet part");
    let cell = mjx_sml::CellReference::parse("A1")?;
    markup.set_cell_value(cell, mjx_sml::CellValue::Number(1.0))?;
    inner.write_worksheet_markup(0, &markup)?;

    // The hatch handed back the real value rather than a copy, so the facade sees the edit with
    // nothing re-opened in between. That is the whole of what a hatch promises.
    let block = workbook.read_range(0, "A1")?;
    assert_eq!(block.value(0, 0)?.number(), Some(1.0));
    // guide-example:end

    println!("A1 is now {:?}", block.value(0, 0)?.number());
    Ok(())
}
