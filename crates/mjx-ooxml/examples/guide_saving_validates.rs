//! The guide's **Saving validates** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/opening_and_saving.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/saving_validates.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/saving_validates.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done, so a
//! block a reader sees cannot differ from a file a harness ran.
//!
//! Everything between the two `guide-example` sentinels below is what the guide shows. Everything
//! outside them is this program's own scaffolding: reading an argument, writing a file, printing a
//! line. The library itself is bytes in and bytes out and never touches a filesystem.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_saving_validates -- out.pptx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

// guide-example:packages saved
fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:start
    use mjx_ooxml::{detect_format, Deck, Format, SlideSize};

    let deck = Deck::blank(SlideSize::widescreen())?;
    deck.validate()?; // the same check `save` runs
    let saved = deck.save()?;
    assert_eq!(detect_format(&saved)?, Format::Presentation);
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
            directory.join("facade_guide_saving_validates.pptx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
