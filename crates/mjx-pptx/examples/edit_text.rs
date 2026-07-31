//! Change one word, and see exactly what that cost.
//!
//! ```sh
//! cargo run -p mjx-pptx --example edit_text -- out.pptx
//! ```
//!
//! The point of this example is the report at the end: retitling a slide dirties **one** part. The
//! theme, the master, every layout and every other slide come back byte-for-byte as they arrived.
//! That is the guarantee the whole library is built around, and it is cheap to check.

use std::collections::BTreeMap;

use anyhow::Result;
use mjx_opc::Package;
use mjx_pptx::Presentation;

mod support;

fn main() -> Result<()> {
    let out = support::output_path("edit_text.pptx");
    let original = support::template()?;

    let mut deck = Presentation::open(&original)?;
    println!("before: {:?}", deck.shape_text(0, 0)?);

    // Replace one run's text. Its formatting is untouched — `set_shape_text` edits the text only.
    deck.set_shape_text(0, 0, 0, "Edited title")?;

    // Editing part of a paragraph splits runs so the range can carry its own formatting.
    use mjx_dml::{CharacterPropertiesSpec, ColorSpec};
    deck.set_text_range_properties(
        0,
        1,
        0,
        0..4,
        &CharacterPropertiesSpec::new()
            .with_bold(true)
            .with_color(ColorSpec::Srgb("C00000".into())),
    )?;
    println!("after:  {:?}", deck.shape_text(0, 0)?);
    println!("body now has {} runs", deck.run_count(0, 1, 0)?);

    // Repeated range edits fragment a paragraph; coalescing merges back what is now identical.
    let merged = deck.coalesce_shape_runs(0, 1)?;
    println!("coalescing merged {merged} run(s)");

    let saved = deck.save()?;
    std::fs::write(&out, &saved)?;

    // ---- What did that actually touch? -----------------------------------------------------
    let before = byte_map(&Package::open(&original)?);
    let after = byte_map(&Package::open(&saved)?);

    let mut changed: Vec<&String> = before
        .iter()
        .filter(|(name, bytes)| after.get(*name) != Some(*bytes))
        .map(|(name, _)| name)
        .collect();
    changed.sort();

    println!("\n{} parts total, {} changed:", before.len(), changed.len());
    for name in &changed {
        println!("  {name}");
    }
    anyhow::ensure!(
        changed.len() == 1,
        "editing one slide should dirty exactly one part"
    );
    println!("\nwrote {}", out.display());

    Ok(())
}

fn byte_map(package: &Package) -> BTreeMap<String, Vec<u8>> {
    package
        .entries()
        .iter()
        .filter_map(|entry| entry.bytes().map(|b| (entry.name.clone(), b.to_vec())))
        .collect()
}
