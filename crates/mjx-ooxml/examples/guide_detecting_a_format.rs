//! The guide's **Detection reads the package, not the filename** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/opening_and_saving.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/detecting_a_format.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/detecting_a_format.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # The three differ in shape, and the page says why
//!
//! A [`mjx_ooxml::Format`] carries five accessors, and in JavaScript they are **free functions**
//! rather than methods: a `#[wasm_bindgen]` enumeration is a number on that side and a number
//! cannot carry a getter. Python has them as attributes rather than as calls, because that is what
//! a Python enumeration member's derived values look like. The guide states both differences above
//! the blocks rather than leaving a reader to conclude that one of them is a typo.
//!
//! This example **saves nothing** — it only reads a format out of bytes — so the two binding
//! harnesses have no package to compare and skip that half of their work by construction.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_detecting_a_format
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let bytes = mjx_fixtures::fixture("sample.docx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::{detect_format, Format, FormatFamily};

    // `bytes` is a Word document. Nothing here looks at a filename: detection opens the container,
    // follows the root `officeDocument` relationship and reads the content type it lands on.
    let detected = detect_format(&bytes)?;
    assert_eq!(detected, Format::Document);
    assert_eq!(detected.family(), FormatFamily::WordProcessing);
    assert_eq!(detected.conventional_extension(), "docx");
    assert!(detected.is_editable());
    assert!(!detected.is_macro_enabled());
    // guide-example:end

    println!("{detected:?} ({})", detected.conventional_extension());
    Ok(())
}
