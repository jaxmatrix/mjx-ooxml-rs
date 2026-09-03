//! The large-file corpus and its peak-RSS harness (MJXOFF-147).
//!
//! `cargo run -p xtask -- corpus` is the one documented command MJXOFF-147 promises: it (re)builds
//! three files into `target/corpus/` (git-ignored — `target/` already is, so nothing new needs
//! adding to `.gitignore`) and prints their size, element count and (for the workbook) cell count.
//! `cargo run -p xtask -- corpus --mem <pptx|docx|xlsx>` runs the peak-resident-set checkpoints for
//! one format in this process (see [`memory`]); run it three times, once per format, for a clean
//! reading of each.
//!
//! **This is a performance instrument, not a fidelity fixture.** It is not a substitute for
//! MJXOFF-130's Office-authored corpus and does not claim to be: nothing here is validated by the
//! ECMA-376 schema gate, and it is generated fresh into a git-ignored directory precisely so it
//! never joins `tests/fixtures/` by accident.

mod common;
mod docx;
mod edit;
mod memory;
mod pptx;
mod xlsx;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use mjx_ooxml_core::{RawElement, RawNode};
use mjx_opc::{Package, PartName};

/// Top-level dispatch for `cargo run -p xtask -- corpus [--mem <format>]`.
pub fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None => generate_all(),
        Some("--mem") => {
            let format = args
                .get(1)
                .map(String::as_str)
                .ok_or_else(|| anyhow::anyhow!("usage: corpus --mem <pptx|docx|xlsx>"))?;
            run_membench(format)
        }
        Some(other) => bail!(
            "unknown `corpus` option {other:?}. Available: (none) to generate, \
             --mem <pptx|docx|xlsx> for peak-RSS checkpoints"
        ),
    }
}

/// `target/corpus/` — inside the already-git-ignored build directory, so the corpus never needs its
/// own `.gitignore` entry and `cargo clean` is exactly "make me regenerate it."
fn output_dir() -> PathBuf {
    mjx_fixtures::workspace_root().join("target").join("corpus")
}

fn generate_all() -> Result<()> {
    let dir = output_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    println!("MJXOFF-147 large-file corpus -> {}\n", dir.display());

    let deck = pptx::build_large_deck().context("building the pptx corpus")?;
    let pptx_bytes = deck.save().context("saving the pptx corpus")?;
    drop(deck);
    report("deck_large.pptx", &dir, &pptx_bytes)?;
    println!("    {} slides", pptx::SLIDE_COUNT);

    let docx_bytes = docx::build_long_document().context("building the docx corpus")?;
    report("document_long.docx", &dir, &docx_bytes)?;
    println!("    {} paragraphs", docx::PARAGRAPH_COUNT);

    let xlsx_bytes = xlsx::build_large_workbook().context("building the xlsx corpus")?;
    report("workbook_large.xlsx", &dir, &xlsx_bytes)?;
    println!(
        "    {} populated cells ({} rows x {} columns)",
        xlsx::CELL_COUNT,
        xlsx::ROW_COUNT,
        xlsx::COLUMN_COUNT
    );

    Ok(())
}

/// Writes `bytes` to `dir/name`, then re-opens the file fresh (a clean read, the same as any later
/// consumer's) to count its elements across every XML part, and prints one summary line.
fn report(name: &str, dir: &Path, bytes: &[u8]) -> Result<()> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
    let mut package =
        Package::open(bytes).with_context(|| format!("re-opening {name} to count elements"))?;
    let elements = count_elements(&mut package)?;
    println!(
        "  {name:<22} {kib:>9} KiB   {elements:>9} elements",
        kib = bytes.len() / 1024,
    );
    Ok(())
}

/// Every element across every XML part of `package` — the corpus's element count, on the same
/// definition `crates/mjx-xml/examples/mjx248_measure.rs` uses for its synthetic slide (elements
/// only; text/comment/PI nodes are not counted, since only elements carry a byte-range span).
fn count_elements(package: &mut Package) -> Result<usize> {
    let mut total = 0usize;
    let names: Vec<PartName> = package.part_names().collect();
    for name in names {
        if name.extension().as_deref() != Some("xml") {
            continue;
        }
        let tree = package
            .part_tree(&name)
            .with_context(|| format!("parsing {}", name.as_str()))?;
        total += count_recursive(&tree.root);
    }
    Ok(total)
}

fn count_recursive(element: &RawElement) -> usize {
    1 + element
        .children
        .iter()
        .map(|node| match node {
            RawNode::Element(child) => count_recursive(child),
            _ => 0,
        })
        .sum::<usize>()
}

/// Runs the four peak-RSS checkpoints (open / first-mutation materialisation / edit / save) for one
/// format, generating its corpus file first if it is not already on disk.
fn run_membench(format: &str) -> Result<()> {
    let dir = output_dir();
    let (file_name, target) = match format {
        "pptx" => ("deck_large.pptx", pptx::representative_slide_part()?),
        "docx" => (
            "document_long.docx",
            PartName::new("/word/document.xml").context("document part name")?,
        ),
        "xlsx" => (
            "workbook_large.xlsx",
            PartName::new("/xl/worksheets/sheet1.xml").context("worksheet part name")?,
        ),
        other => bail!("unknown format {other:?}. Available: pptx, docx, xlsx"),
    };
    let path = dir.join(file_name);
    if !path.exists() {
        generate_all()?;
    }
    membench_package(format, &path, &target)
}

/// The four checkpoints, for one already-on-disk package. Reads its bytes fresh from disk as the
/// very first thing this process does with them, so the "open" checkpoint is not inflated by
/// whatever building the corpus in-process would have cost (see [`memory`]'s module docs).
fn membench_package(label: &str, path: &Path, target: &PartName) -> Result<()> {
    println!(
        "\n{label} ({}) — peak RSS, cumulative since process start:",
        path.display()
    );
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut package = Package::open(&bytes).context("Package::open")?;
    memory::checkpoint("open")?;

    package
        .part_tree_mut(target)
        .context("first-mutation materialisation")?;
    memory::checkpoint("first-mutation materialisation")?;

    {
        let tree = package
            .part_tree_mut(target)
            .context("re-borrowing the already-materialised tree")?;
        edit::representative_edit(tree)?;
    }
    memory::checkpoint("edit (one attribute, already materialised)")?;

    let saved = package.save().context("save")?;
    memory::checkpoint("save")?;
    println!("  (saved {} bytes, discarded)", saved.len());
    Ok(())
}
