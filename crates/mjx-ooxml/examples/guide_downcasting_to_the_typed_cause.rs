//! The guide's **The typed cause is still there** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/errors.md`][guide] shows this code, and the block a reader sees is
//! a *copy* of this file — `cargo run -p xtask -- guide-examples` does the copying and
//! `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # Rust-only, and the alternative was considered rather than assumed
//!
//! MJXOFF-257 asked whether this block should instead *say in prose* that a binding caller branches
//! on `.code`, which would give it three real halves after all. It should not, and the reason is
//! that the page already has that block: § *Eleven codes, and what each one means you should do*
//! is `branching_on_an_error_code`, in all three languages. A second block making the same move
//! would not be this section's subject — this section's subject is the thing a binding caller
//! **cannot** do.
//!
//! So the marker beside the block declares it Rust-only and names [`mjx_ooxml::PptxError`] and
//! `downcast_ref`. Neither binding declares either, and `xtask/tests/guide_examples.rs` reads both
//! surfaces on every run to confirm it (MJXOFF-261).
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_downcasting_to_the_typed_cause
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Deck, ErrorCode, PptxError, SlideSize};
    use std::error::Error as _;

    let mut deck = Deck::blank(SlideSize::widescreen())?;

    // A blank deck has no slides at all, so slide 7 is past the end.
    let failure = deck.shape_count(7.into()).expect_err("no slide 7");
    assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);

    // Collapsing to eleven codes loses nothing here: the variant the crate below raised is still
    // underneath, reachable by downcasting the source.
    let cause = failure.source().expect("a typed cause");
    let pptx = cause.downcast_ref::<PptxError>().expect("a PptxError");
    assert!(matches!(pptx, PptxError::SlideIndexOutOfRange { .. }));
    // guide-example:end

    println!("the typed cause is {pptx:?}");
    Ok(())
}
