//! The guide's **`detail` says where** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/errors.md`][guide] shows this code in three languages, and every
//! one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/an_error_says_where.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/an_error_says_where.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # The three differ in shape, and the page says why
//!
//! An [`mjx_ooxml::ErrorDetail`] is a value behind `detail()` here; in Python its five coordinates
//! are attributes on the exception itself, always present and `None` where the failure knew no such
//! coordinate; in JavaScript it is a plain `detail` object carrying **only** the coordinates the
//! failure had, so a caller reads `detail.row ?? null`.
//!
//! Why any of it matters is the same in all three: a caller points at the thing that failed rather
//! than re-deriving it from a message string, which is the one thing a message must never be parsed
//! for.
//!
//! This example **saves nothing** — it asks a slide for a shape it has not got — so the two binding
//! harnesses have no package to compare and skip that half of their work by construction.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_an_error_says_where
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

// guide-example:packages none
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Deck, ErrorCode, ShapePath, SlideSize, Surface};

    let mut deck = Deck::blank(SlideSize::widescreen())?;
    let slide = Surface::Slide(deck.add_slide_from_layout(0)?);

    // The slide carries the layout's placeholders and nothing at index 4.
    let failure = deck.shape_bounds(slide, 4.into()).expect_err("no shape 4");
    assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);

    // The failure says *where*, in the same addressing the call used to get there.
    assert_eq!(failure.detail().surface, Some(slide));
    let shape = failure.detail().shape.as_ref().map(ShapePath::indices);
    assert_eq!(shape, Some(&[4][..]));
    assert!(!failure.detail().is_empty());
    // guide-example:end

    println!("{}", failure.message());
    Ok(())
}
