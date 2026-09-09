//! The guide's **The round trip** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/opening_and_saving.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/the_round_trip.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/the_round_trip.mjs`. `cargo run -p xtask --
//! guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! # Why this one is not `guide_saving_validates` again
//!
//! That example authors every part it compares, from [`mjx_ooxml::Deck::blank`]. This one **opens a
//! committed fixture**, so every part in the package it saves was written by somebody else and came
//! back through `mjx_opc`'s copy-on-write part graph and `mjx_xml::fidelity`'s byte-preserving
//! reader. The assertion inside the region is the round-trip contract itself, stated in all three
//! languages: same part names, and byte-identical payloads for every one of them. Nothing else in
//! the tri-language mechanism exercises preservation at all.
//!
//! The comparison the two binding harnesses then make is a second, different fact: that all three
//! languages re-emitted *the same* preserved bytes, and not merely each their own self-consistent
//! ones.
//!
//! # Where the bytes come from
//!
//! Between the two hidden-prelude sentinels below, so the doctest binds `original` and the rendered
//! block does not show it. Reading a file is the caller's job — this library is
//! bytes in and bytes out and never touches a filesystem, which is the same reason
//! `crates/mjx-ooxml/examples/build_a_deck.rs` keeps its own `std::fs::read` out of the guide.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_the_round_trip -- out.xlsx
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let original = mjx_fixtures::fixture("sample.xlsx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::Workbook;

    // `original` is the file's bytes. Reading them is the caller's job in every one of the three
    // languages: this library is bytes in and bytes out and never touches a filesystem.
    let saved = Workbook::open(&original)?.save()?;

    // Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
    // `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
    let before = Workbook::open(&original)?;
    let after = Workbook::open(&saved)?;
    assert_eq!(before.part_names(), after.part_names());
    for part in before.part_names() {
        let was = before.part_bytes(&part)?;
        let now = after.part_bytes(&part)?;
        assert_eq!(was, now, "{part} changed");
    }
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
            directory.join("facade_guide_the_round_trip.xlsx")
        }
    };
    std::fs::write(&path, saved)?;
    println!("wrote {} ({} bytes)", path.display(), saved.len());
    Ok(())
}
