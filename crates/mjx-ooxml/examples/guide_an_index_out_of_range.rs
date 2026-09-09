//! The guide's **One error type, eleven codes** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/README.md`][guide] shows this code in three languages, and every
//! one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/an_index_out_of_range.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/an_index_out_of_range.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # The three differ in shape, and the page says why
//!
//! Two of the differences meet here at once: an [`mjx_ooxml::ErrorCode`] is an enumeration in Rust,
//! a class in Python and a string in JavaScript; and an [`mjx_ooxml::ErrorDetail`] is a value
//! behind `detail()` in Rust, five attributes on the exception itself in Python, and a plain
//! `detail` object in JavaScript carrying only the coordinates the failure actually had.
//!
//! This example **saves nothing** — it asks a blank deck for a slide it does not have — so the two
//! binding harnesses have no package to compare and skip that half of their work by construction.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_an_index_out_of_range
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

// guide-example:packages none
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Deck, ErrorCode, SlideSize};

    let mut deck = Deck::blank(SlideSize::widescreen())?;

    // A blank deck has no slides at all, so slide 7 is past the end.
    let failure = deck.shape_count(7.into()).expect_err("no slide 7");
    assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(failure.detail().index, Some(7));
    assert_eq!(failure.message(), "slide index 7 out of range (0..0)");
    // guide-example:end

    println!("{}", failure.message());
    Ok(())
}
