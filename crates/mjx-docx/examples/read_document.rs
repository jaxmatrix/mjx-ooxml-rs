//! Reading a document somebody else wrote, and proving the round trip does not disturb it — the
//! runnable version of [the fidelity page](mjx_docx::guide::fidelity_and_gaps)'s first half.
//!
//! ```sh
//! cargo run -p mjx-docx --example read_document -- out.docx
//! ```
//!
//! Reading is the whole point here, so the assertion at the end is the interesting one: this example
//! reads `sample.docx` exhaustively — every part it relates to, every paragraph, every style — saves
//! it back, and then compares **the decompressed payload of every part** of what it wrote against
//! the same part of what it opened. Reading materialises trees; it must not dirty a byte. An example
//! that only printed what it found would not notice if it did.

use anyhow::{Context, Result};
use mjx_docx::{Document, HeaderFooterType};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("read_document.docx");
    let original = support::template().context("reading the template")?;
    let mut document = Document::open(&original).context("opening the template")?;

    // ---- The part graph ---------------------------------------------------------------------
    // `parts()` is the resolved relationship graph of the main document part: what this document
    // actually relates to, not what a `.docx` may relate to in principle.
    let parts = document.parts().clone();
    println!("part graph:");
    for (label, present) in [
        ("styles.xml", parts.styles.is_some()),
        ("numbering.xml", parts.numbering.is_some()),
        ("settings.xml", parts.settings.is_some()),
        ("webSettings.xml", parts.web_settings.is_some()),
        ("fontTable.xml", parts.font_table.is_some()),
        ("theme", parts.theme.is_some()),
        ("footnotes.xml", parts.footnotes.is_some()),
        ("endnotes.xml", parts.endnotes.is_some()),
        ("comments.xml", parts.comments.is_some()),
        ("glossary/document.xml", parts.glossary_document.is_some()),
    ] {
        println!("  {label:<22} {}", if present { "yes" } else { "—" });
    }
    println!("  headers                {}", parts.headers.len());
    println!("  footers                {}", parts.footers.len());

    // ---- The body ----------------------------------------------------------------------------
    let paragraphs = document.paragraph_count()?;
    let tables = document.table_count()?;
    println!("body: {paragraphs} paragraph(s), {tables} table(s)");
    for index in 0..paragraphs.min(5) {
        let text = document.paragraph_text(index)?;
        let runs = document.run_count(index)?;
        println!("  [{index}] {runs} run(s): {text:?}");
    }

    // ---- Sections, and the headers they resolve to --------------------------------------------
    // A section's properties live at the *end* of the range it governs — see `SectionSpan`.
    let section_count = document.sections(|spans, _| spans.len())?;
    println!("sections: {section_count}");
    for index in 0..section_count {
        let header = document.resolve_header(index, HeaderFooterType::Default)?;
        let footer = document.resolve_footer(index, HeaderFooterType::Default)?;
        println!(
            "  [{index}] default header {}, default footer {}",
            header.map_or_else(|| "—".to_owned(), |part| part.as_str().to_owned()),
            footer.map_or_else(|| "—".to_owned(), |part| part.as_str().to_owned()),
        );
    }

    // ---- Styles -------------------------------------------------------------------------------
    // `style_sheet` answers `None` — the closure is never called — for a document with no
    // `word/styles.xml` at all, which is what a `Document::blank` document is.
    let style_ids = document.style_sheet(|sheet, interner| {
        sheet
            .styles()
            .filter_map(|style| style.style_id(interner).ok().flatten())
            .map(|id| id.into_owned())
            .collect::<Vec<_>>()
    })?;
    match &style_ids {
        Some(ids) => println!("styles: {}", ids.join(", ")),
        None => println!("styles: no word/styles.xml"),
    }

    // ---- Save, and check that reading changed nothing -------------------------------------------
    // The round-trip contract is per-part decompressed-payload byte identity plus structural
    // container identity — deliberately *not* identical ZIP bytes, because deflate parameters vary
    // by encoder. So the comparison below is part by part, never archive against archive.
    let saved = document.save().context("saving the document back")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    let before = mjx_opc::Package::open(&original).context("reopening the original package")?;
    let after = mjx_opc::Package::open(&saved).context("opening the saved package")?;

    let mut before_names: Vec<_> = before.part_names().collect();
    let mut after_names: Vec<_> = after.part_names().collect();
    before_names.sort();
    after_names.sort();
    anyhow::ensure!(
        before_names == after_names,
        "the saved package does not hold the same parts as the original"
    );

    for name in &before_names {
        let original_bytes = before
            .part_bytes(name)
            .with_context(|| format!("original {}", name.as_str()))?;
        let saved_bytes = after
            .part_bytes(name)
            .with_context(|| format!("saved {}", name.as_str()))?;
        anyhow::ensure!(
            original_bytes == saved_bytes,
            "{} changed across a round trip that only read",
            name.as_str()
        );
    }
    println!(
        "round trip: {} part(s), every decompressed payload byte-identical",
        before_names.len()
    );

    Ok(())
}
