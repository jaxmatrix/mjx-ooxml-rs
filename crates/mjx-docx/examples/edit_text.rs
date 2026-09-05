//! Editing the text of a document that already exists — the runnable version of
//! [the text-and-formatting guide](mjx_docx::guide::text_and_formatting).
//!
//! ```sh
//! cargo run -p mjx-docx --example edit_text -- out.docx
//! ```
//!
//! Two things are demonstrated and then checked on the reopened bytes: that an edit lands where it
//! was addressed, and that it lands *only* there. `word/styles.xml`, `word/settings.xml`,
//! `word/fontTable.xml` and the theme are never named by any call below, so all four must come back
//! byte-for-byte — that is part-level copy-on-write, and it is the reason this library exists.

use anyhow::{Context, Result};
use mjx_docx::Document;

mod support;

/// The parts this example never addresses. Each must survive byte-identically.
const UNTOUCHED: [&str; 4] = [
    "/word/styles.xml",
    "/word/settings.xml",
    "/word/fontTable.xml",
    "/word/theme/theme1.xml",
];

fn main() -> Result<()> {
    let out = support::output_path("edit_text.docx");
    let original = support::template().context("reading the template")?;
    let mut document = Document::open(&original).context("opening the template")?;

    // ---- What is there now ---------------------------------------------------------------------
    let before = document.paragraph_text(0)?;
    println!("paragraph 0 was: {before:?}");
    anyhow::ensure!(
        !before.is_empty(),
        "the template's first paragraph is empty; this example needs a run to edit"
    );

    // ---- Replace one run's text ------------------------------------------------------------------
    // `set_run_text` edits a run that already exists; `insert_run`/`append_run` are for a paragraph
    // that has none. Both address by position — no handle is held across calls, which is exactly
    // what lets the library know which single part it has to rewrite.
    document
        .set_run_text(0, 0, "Edited by the mjx-ooxml-rs example suite.")
        .context("setting paragraph 0's first run")?;

    // ---- Add a run to an existing paragraph -------------------------------------------------------
    document
        .append_run(0, " Appended in a second run.")
        .context("appending a run")?;
    anyhow::ensure!(
        document.run_count(0)? == 2,
        "paragraph 0 should have two runs"
    );

    // ---- Add a paragraph, and take one away --------------------------------------------------------
    let paragraphs_before = document.paragraph_count()?;
    document.append_paragraph()?;
    document.append_run(paragraphs_before, "A paragraph this example added.")?;
    document.insert_paragraph(0)?;
    document.insert_run(0, 0, "A paragraph inserted at the very top.")?;
    anyhow::ensure!(
        document.paragraph_count()? == paragraphs_before + 2,
        "two paragraphs should have been added"
    );
    // Positions shift as the body changes: the paragraph edited above started at index 0 and is now
    // at index 1, because a paragraph was inserted in front of it. The template's own second
    // paragraph — index 1 when this example started — is index 2 now, and it is the one removed.
    document.remove_paragraph(2)?;
    anyhow::ensure!(
        document.paragraph_count()? == paragraphs_before + 1,
        "one paragraph should have been removed again"
    );

    // ---- Tracked changes, read two ways ------------------------------------------------------------
    // Neither reader rewrites the document: both answer what the text *would* be. `sample.docx`
    // carries no revisions, so the two agree — which is itself the check that this path is being
    // exercised rather than skipped.
    let revisions = document.revisions()?;
    let accepted = document.text_with_revisions_accepted()?;
    let rejected = document.text_with_revisions_rejected()?;
    println!("revisions: {}", revisions.len());
    anyhow::ensure!(
        revisions.is_empty() == (accepted == rejected),
        "with no revisions the accepted and rejected texts must agree, and differ when there are any"
    );

    // ---- Save --------------------------------------------------------------------------------------
    let bytes = document.save().context("saving")?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    // ---- Reopen, and check both halves ---------------------------------------------------------------
    let mut reopened = Document::open(&bytes).context("reopening")?;
    anyhow::ensure!(
        reopened.paragraph_text(0)? == "A paragraph inserted at the very top.",
        "the inserted paragraph did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.paragraph_text(1)?
            == "Edited by the mjx-ooxml-rs example suite. Appended in a second run.",
        "the edited paragraph did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.paragraph_count()? == paragraphs_before + 1,
        "the paragraph count did not survive the round trip"
    );

    let source = mjx_opc::Package::open(&original)?;
    let written = mjx_opc::Package::open(&bytes)?;
    for name in UNTOUCHED {
        let part = mjx_opc::PartName::new(name).context("part name")?;
        let (Some(before), Some(after)) = (source.part_bytes(&part), written.part_bytes(&part))
        else {
            anyhow::bail!("{name} is missing from one of the two packages");
        };
        anyhow::ensure!(before == after, "{name} changed, and nothing addressed it");
    }
    println!(
        "untouched parts still byte-identical: {}",
        UNTOUCHED.join(", ")
    );

    Ok(())
}
