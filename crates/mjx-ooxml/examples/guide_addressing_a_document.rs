//! The guide's **`Document`: a block and a run** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/addressing.md`][guide] shows this code in three languages, and
//! every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/addressing_a_document.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/addressing_a_document.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! A Word document has one body, so nothing names the part: a paragraph is addressed by a
//! [`mjx_ooxml::BlockPath`] and a run inside it by a [`mjx_ooxml::RunPath`], and both convert from a
//! bare index.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_addressing_a_document -- out.docx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

// guide-example:packages saved
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{Document, PageSize};

    let mut document = Document::blank(PageSize::a4())?;
    document.append_paragraph()?;
    document.append_run(0, "Quarterly ")?;
    document.append_run(0, "results")?;
    assert_eq!(document.run_count(0)?, 2);
    assert_eq!(document.run_text(0, 1)?, "results");
    assert_eq!(document.paragraph_text(0)?, "Quarterly results");

    let saved = document.save()?;
    // guide-example:end

    write_output(&saved)
}

/// Where this example writes: its first argument, or `target/examples/` by default.
///
/// The two binding harnesses run this program with an explicit path and compare what lands there,
/// part by part, against what they produced themselves.
fn write_output(saved: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_addressing_a_document.docx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
