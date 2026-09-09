//! The guide's **Bytes in, bytes out** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/README.md`][guide] shows this code in three languages, and every
//! one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/open_edit_and_save_any_format.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/open_edit_and_save_any_format.mjs`. `cargo run -p
//! xtask -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was
//! done.
//!
//! # It used to be the one block that never ran
//!
//! Until MJXOFF-254 finished the guide, this was the guide's only `no_run` doctest: it read
//! `in.pptx`, a file that does not exist, so it compiled and was never executed. The hidden prelude
//! is what fixed it — the bytes come from a committed fixture, out of sight, and everything a
//! reader sees runs. **Every code block in this guide is now executed by something.**
//!
//! Writing the result is not in the block for the same reason reading is not: a file handle is the
//! caller's, and this library is bytes in and bytes out.
//!
//! # The three differ in shape, and the page says why
//!
//! The [`mjx_ooxml::FormatFamily`] the dispatch branches on is a `match` here, an `if` chain in
//! both bindings, and the accessor that produces it is a method, an attribute and a free function
//! respectively.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_open_edit_and_save_any_format -- out.pptx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let bytes = mjx_fixtures::fixture("sample.pptx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::{detect_format, Deck, Document, FormatFamily, Workbook};

    // `bytes` is whatever the caller read. Which of the three surfaces opens it is the package's
    // answer, not the filename's.
    let saved = match detect_format(&bytes)?.family() {
        FormatFamily::Presentation => Deck::open(&bytes)?.save()?,
        FormatFamily::WordProcessing => Document::open(&bytes)?.save()?,
        FormatFamily::Spreadsheet => Workbook::open(&bytes)?.save()?,
        // `FormatFamily` is `#[non_exhaustive]`: a fourth family is an added arm, not a broken
        // build.
        other => return Err(format!("unhandled family {other:?}").into()),
    };
    assert!(!saved.is_empty());
    // guide-example:end

    write_output(&saved)
}

/// Where this example writes: its first argument, or `target/examples/` by default.
///
/// The two binding harnesses run this program with an explicit path and compare what lands there,
/// part by part, against what they produced themselves from the same fixture.
fn write_output(saved: &[u8]) -> Result<(), Box<dyn Error>> {
    let path = match std::env::args().nth(1) {
        Some(argument) => PathBuf::from(argument),
        None => {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            std::fs::create_dir_all(&directory)?;
            directory.join("facade_guide_open_edit_and_save_any_format.pptx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
