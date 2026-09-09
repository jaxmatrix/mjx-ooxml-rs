//! The guide's **What is preserved rather than modelled** example, as a program.
//!
//! [`crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md`][guide] shows this code in three languages,
//! and every one of the three blocks is a *copy* of a file a test runner executes — this one, plus
//! `bindings/mjx-python/tests/guide_examples/preserved_rather_than_modelled.py` and
//! `bindings/mjx-wasm/tests/node/guide_examples/preserved_rather_than_modelled.mjs`. `cargo run -p
//! xtask -- guide-examples` does the copying and `xtask/tests/guide_examples.rs` proves it was done.
//!
//! A large amount of every format is recognised, reported and deliberately not modelled. **Preserved
//! is not ignored**: each cluster has a reader here that says what is in the package without a model
//! behind it, and every one of those parts comes back byte for byte through an unrelated edit.
//!
//! This example **saves nothing** — it only reads, which is the point — so the two binding harnesses
//! have no package to compare and skip that half of their work by construction.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example guide_preserved_rather_than_modelled
//! ```
//!
//! [guide]: https://docs.rs/mjx-ooxml

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // guide-example:prelude-start
    let original = mjx_fixtures::fixture("preserved_parts.xlsx");
    // guide-example:prelude-end

    // guide-example:start
    use mjx_ooxml::Workbook;

    let workbook = Workbook::open(&original)?;

    // Inventoried from the relationships, with no markup parsed at all.
    let summary = workbook.preserved_parts()?;
    assert!(!summary.pivot_tables.is_empty());
    assert!(!summary.pivot_cache_definitions.is_empty());

    // And resolved far enough to say which tab each one sits on.
    for table in workbook.pivot_tables()? {
        assert!(!table.sheet_name.is_empty());
    }
    // guide-example:end

    println!("{} pivot table(s) preserved", summary.pivot_tables.len());
    Ok(())
}
