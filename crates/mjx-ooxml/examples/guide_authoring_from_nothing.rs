//! The guide's **Authoring from nothing** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/opening_and_saving.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/authoring_from_nothing.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/authoring_from_nothing.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! This example **saves nothing**: what it demonstrates is that all three surfaces build a package
//! from this library's own element builders with no template and no file on disk. So its two
//! binding halves have no package to compare against this one, and the harnesses skip that half of
//! their work by construction — the three halves agreeing about producing nothing is itself checked,
//! by `the_three_halves_of_an_example_agree_about_whether_it_produces_a_package`.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_authoring_from_nothing
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

// guide-example:packages none
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Deck, Document, PageSize, SlideSize, Workbook};

    // One master, one layout, a theme — and no slides yet.
    let deck = Deck::blank(SlideSize::widescreen())?;
    assert_eq!(deck.slide_count(), 0);
    assert_eq!(deck.master_count(), 1);

    // One empty paragraph, because a `w:body` needs one.
    let mut document = Document::blank(PageSize::a4())?;
    assert_eq!(document.paragraph_count()?, 1);

    // One empty worksheet, named Sheet1.
    let workbook = Workbook::blank()?;
    assert_eq!(workbook.sheet_count(), 1);
    // guide-example:end

    println!("three blank packages, none of them read from disk");
    Ok(())
}
