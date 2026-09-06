//! Editing a workbook somebody else wrote, and proving the edit touched **exactly** the parts it
//! had to — the runnable version of [the fidelity page](mjx_xlsx::guide::fidelity_and_the_part_graph).
//!
//! ```sh
//! cargo run -p mjx-xlsx --example edit_a_workbook -- out.xlsx
//! ```
//!
//! The interesting assertion is the last one. This example opens `sample.xlsx`, changes one cell and
//! renames the tab, saves, and then compares **the decompressed payload of every part** of what it
//! wrote against the same part of what it opened. Exactly two payloads may differ — the worksheet
//! the cell is on, and `xl/workbook.xml`, which is where a tab's name lives. An example that only
//! printed the new value would not notice a save that quietly rewrote the theme, the styles or the
//! shared strings.

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use mjx_opc::Package;
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

mod support;

fn main() -> Result<()> {
    let out = support::output_path("edit_a_workbook.xlsx");
    let original = support::template().context("reading the template")?;
    let mut workbook = Workbook::open(&original).context("opening the template")?;

    // ---- What is in there ---------------------------------------------------------------------
    // `sheets()` is the workbook part's own list, in tab order. `Sheet::part` is `None` for a tab
    // whose `r:id` reaches nothing — a report about the file, never a repair of it.
    println!("tabs:");
    for (index, sheet) in workbook.sheets().iter().enumerate() {
        println!(
            "  [{index}] {:<12} visible={} part={}",
            sheet.name,
            sheet.is_visible(),
            sheet.part.as_ref().map_or("—", |part| part.as_str()),
        );
    }

    let reference = |text: &str| CellReference::parse(text).expect("a literal reference parses");
    let (b2, c2, a1) = (reference("B2"), reference("C2"), reference("A1"));

    println!("before:");
    println!("  A1 = {:?}", workbook.cell_text(0, a1)?);
    println!("  B2 = {:?}", workbook.cell_text(0, b2)?);
    println!("  C2 = {:?}", workbook.cell_text(0, c2)?);

    // ---- The edit ------------------------------------------------------------------------------
    // Two mutations, on two different parts on purpose: a cell (the worksheet) and a tab name
    // (`xl/workbook.xml`, because a tab's name is *not* in its own markup).
    workbook
        .set_cell_value(0, b2, CellValue::Number(42.0))
        .context("setting B2")?;
    workbook.rename_sheet(0, "edited").context("renaming")?;

    let saved = workbook.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Assert: the value came back, through a reopened file rather than the model -------------
    let reopened = Workbook::open(&saved).context("reopening what we wrote")?;
    anyhow::ensure!(
        reopened.cell_text(0, b2)?.as_deref() == Some("42"),
        "B2 did not come back as 42"
    );
    anyhow::ensure!(
        reopened.sheets()[0].name == "edited",
        "the tab did not come back renamed"
    );
    // The neighbours are untouched: a shared string still resolves, and the number beside the edit
    // is exactly the text the file wrote — `9.99`, not a reformatted `9.9900000000000002`.
    anyhow::ensure!(
        reopened.cell_text(0, a1)?.as_deref() == Some("name"),
        "A1's shared string did not survive"
    );
    anyhow::ensure!(
        reopened.cell_text(0, c2)?.as_deref() == Some("9.99"),
        "C2 was reformatted by a save that had no business touching it"
    );

    // ---- Assert: exactly two parts changed ------------------------------------------------------
    // The round-trip contract is per-part decompressed-payload byte identity plus structural
    // container identity — deliberately *not* identical ZIP bytes, because deflate parameters vary
    // by encoder. So this compares part by part, never archive against archive.
    let before = Package::open(&original).context("reopening the original package")?;
    let after = Package::open(&saved).context("opening the saved package")?;

    let before_names: BTreeSet<_> = before
        .part_names()
        .map(|name| name.as_str().to_owned())
        .collect();
    let after_names: BTreeSet<_> = after
        .part_names()
        .map(|name| name.as_str().to_owned())
        .collect();
    anyhow::ensure!(
        before_names == after_names,
        "the saved package does not hold the same parts as the original"
    );

    let mut changed = Vec::new();
    for name in before.part_names() {
        let original_bytes = before
            .part_bytes(&name)
            .with_context(|| format!("original {}", name.as_str()))?;
        let saved_bytes = after
            .part_bytes(&name)
            .with_context(|| format!("saved {}", name.as_str()))?;
        if original_bytes != saved_bytes {
            changed.push(name.as_str().to_owned());
        }
    }
    changed.sort();
    println!("parts whose payload changed: {changed:?}");
    anyhow::ensure!(
        changed == ["/xl/workbook.xml", "/xl/worksheets/sheet1.xml"],
        "an edit to one cell and one tab name changed {changed:?} — copy-on-write is not holding"
    );
    println!(
        "round trip: {} part(s), {} changed, the rest byte-identical",
        before_names.len(),
        changed.len(),
    );

    Ok(())
}
