//! Reading formulas and the cached values beside them — and proving that an edit to a cell a formula
//! *depends on* leaves that cached value exactly as the file wrote it.
//!
//! ```sh
//! cargo run -p mjx-xlsx --example read_formulas -- out.xlsx
//! ```
//!
//! The runnable version of [the formulas page](mjx_xlsx::guide::formulas_and_cached_values). The
//! stale cached value is the single most likely thing on this crate's surface to be reported as a
//! bug, so this example asserts it rather than describing it: `B2` holds `=A2*2` with a cached `2`,
//! `A2` is changed from `1` to `50`, and after a save-and-reopen the cached value is still `2`.
//! There is no calculation engine here and there will not be one — see
//! [the deliberate-limitations page](mjx_xlsx::guide::deliberate_limitations).

use anyhow::{Context, Result};
use mjx_opc::Package;
use mjx_sml::{CellReference, CellValue, FormulaKind};
use mjx_xlsx::Workbook;

mod support;

fn main() -> Result<()> {
    let out = support::output_path("read_formulas.xlsx");
    let original = support::fixture("formulas.xlsx").context("reading the fixture")?;
    let mut workbook = Workbook::open(&original).context("opening the fixture")?;

    // ---- Every formula on the sheet, with its kind and its cached value -------------------------
    // `worksheet_markup` takes `&self`: reading parses the part's bytes into a model that outlives
    // the tree and leaves the package holding the bytes it arrived with.
    let markup = workbook
        .worksheet_markup(0)?
        .context("the first tab reaches a worksheet part")?;
    let sheet = markup.sheet_data().context("the sheet has a sheetData")?;

    let mut formula_cells = 0usize;
    let mut text_carrying = 0usize;
    println!("formulas:");
    for cell in sheet.cells() {
        let Some(formula) = cell.formula() else {
            continue;
        };
        formula_cells += 1;
        let text = formula.text().context("decoding the formula text")?;
        if !text.is_empty() {
            text_carrying += 1;
        }
        let cached = cell.value().context("decoding the cached value")?;
        println!(
            "  {:<4} {:<10} {:<24} cached={:?}",
            cell.reference().text().as_str(),
            format!("{:?}", formula.kind().context("the formula's @t")?),
            if text.is_empty() {
                "(no text of its own)".to_owned()
            } else {
                format!("={text}")
            },
            cached.as_deref(),
        );
    }
    // Eleven `<f>` elements: five shared (one host plus four text-less members), four normal, one
    // array and one data-table. Six of the eleven carry text — the four text-less shared members and
    // the self-closing data-table formula carry none.
    anyhow::ensure!(
        formula_cells == 11,
        "expected eleven formula cells, found {formula_cells}"
    );

    // ---- A shared group keeps its text on one cell, and only one -------------------------------
    // Five cells, one of them carrying the text. Expanding a group into per-cell text on write is a
    // corruption, not an optimisation, so the distribution is asserted rather than assumed.
    let groups = sheet
        .shared_formula_groups()
        .context("indexing the groups")?;
    anyhow::ensure!(
        groups.len() == 1,
        "expected one shared group, found {}",
        groups.len()
    );
    let group = groups.get(0).context("group 0 exists")?;
    println!(
        "shared group {}: host {:?}, range {:?}, {} cell(s)",
        group.index(),
        group.host().map(|host| host.text().as_str().to_owned()),
        group.range().map(|range| range.text().as_str().to_owned()),
        group.cell_count(),
    );
    anyhow::ensure!(
        group.cell_count() == 5,
        "the group should have five members"
    );
    anyhow::ensure!(
        group.host_count() == 1,
        "exactly one member carries the text"
    );
    anyhow::ensure!(
        group
            .host_formula()
            .context("the host has a formula")?
            .kind()?
            == FormulaKind::Shared,
    );
    // The distribution is the assertion: a writer that expanded the group would push this to ten.
    anyhow::ensure!(
        text_carrying == 6,
        "expected six formula cells to carry text, found {text_carrying}"
    );

    // ---- The edit that would tempt a library to recalculate --------------------------------------
    let reference = |text: &str| CellReference::parse(text).expect("a literal reference parses");
    let (a2, b2) = (reference("A2"), reference("B2"));
    anyhow::ensure!(
        workbook.cell_text(0, b2)?.as_deref() == Some("2"),
        "B2's cached value should start at 2"
    );
    drop(markup);
    workbook
        .set_cell_value(0, a2, CellValue::Number(50.0))
        .context("setting A2, which B2 references")?;

    let saved = workbook.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    // ---- Assert: stale, deliberately ------------------------------------------------------------
    let reopened = Workbook::open(&saved).context("reopening")?;
    anyhow::ensure!(
        reopened.cell_text(0, a2)?.as_deref() == Some("50"),
        "the edit did not land"
    );
    anyhow::ensure!(
        reopened.cell_text(0, b2)?.as_deref() == Some("2"),
        "B2's cached value changed — something recalculated, or blanked, a value it was asked to \
         preserve"
    );
    let reopened_markup = reopened
        .worksheet_markup(0)?
        .context("the reopened tab reaches a worksheet part")?;
    let reopened_sheet = reopened_markup
        .sheet_data()
        .context("the reopened sheet has a sheetData")?;
    let b2_formula = reopened_sheet
        .cell(b2)
        .context("B2 survived")?
        .formula()
        .context("B2 still has a formula")?;
    anyhow::ensure!(
        b2_formula.text()? == "A2*2",
        "the formula text was rewritten by a save that had no business touching it"
    );
    println!("A2 = 50, and B2's formula still reads =A2*2 with its cached 2 — stale by design");

    // ---- And `calcChain.xml` is left exactly as found ---------------------------------------------
    // Maintaining a calculation chain means knowing what depends on what, which is a calculation
    // engine under another name; dropping it would be an edit nobody asked for.
    let before = Package::open(&original)?;
    let after = Package::open(&saved)?;
    let chain = mjx_opc::PartName::new("/xl/calcChain.xml")?;
    let chain_before = before
        .part_bytes(&chain)
        .context("the fixture has a calcChain")?;
    let chain_after = after
        .part_bytes(&chain)
        .context("the saved file kept its calcChain")?;
    anyhow::ensure!(
        chain_before == chain_after,
        "xl/calcChain.xml was rewritten"
    );
    println!("xl/calcChain.xml: byte-identical, neither maintained nor dropped");

    Ok(())
}
