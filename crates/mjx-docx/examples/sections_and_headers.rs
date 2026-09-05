//! Sections, page geometry, and the headers a section resolves to — the runnable version of
//! [the sections-and-headers guide](mjx_docx::guide::tables_sections_and_headers).
//!
//! ```sh
//! cargo run -p mjx-docx --example sections_and_headers -- out.docx
//! ```
//!
//! The check worth reading is the last one: a section that names no header of its own **inherits**
//! the previous section's, per ECMA-376 Part 1 §17.10.1, and `resolve_header` answers with the part
//! that actually applies rather than with the reference the section happens to carry. Asserting that
//! section 1 resolves to the *same part* section 0 created is what distinguishes a real resolution
//! walk from a lookup that only reads the section in front of it.

use anyhow::{Context, Result};
use mjx_docx::{
    Document, HeaderFooterType, PageMargins, PageSize, Paragraph, Run, SectionLocation,
};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("sections_and_headers.docx");
    let mut document = Document::blank(PageSize::a4()).context("blank document")?;

    // ---- One section to start with ------------------------------------------------------------
    // A section's properties live at the *end* of the range it governs: the body-level `w:sectPr`
    // is the last section's, and every earlier section ends at a paragraph that carries its own.
    document.insert_run(0, 0, "Portrait section")?;
    anyhow::ensure!(
        document.sections(|spans, _| spans.len())? == 1,
        "a blank document has exactly one section"
    );

    // ---- A second section, landscape and wider-margined ------------------------------------------
    // Putting a `w:sectPr` in paragraph 0's own properties *ends* a section there — so paragraph 0
    // becomes section 0, and everything after it falls into the body-level section.
    document.append_paragraph()?;
    document.append_run(1, "Landscape section")?;
    document.edit_section_properties(
        SectionLocation::Paragraph(0.into()),
        |properties, interner| {
            properties.set_page_size(interner, Some(PageSize::a4()));
            properties.set_page_margins(interner, Some(PageMargins::NORMAL));
        },
    )?;
    document.edit_section_properties(SectionLocation::Body, |properties, interner| {
        properties.set_page_size(interner, Some(PageSize::a4().landscape()));
        properties.set_page_margins(
            interner,
            Some(PageMargins {
                left: 2880,
                right: 2880,
                ..PageMargins::NORMAL
            }),
        );
    })?;

    let spans = document.sections(|spans, interner| {
        spans
            .iter()
            .map(|span| {
                let size = span
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.page_size(interner).ok().flatten());
                (span.first_paragraph, span.last_paragraph, size)
            })
            .collect::<Vec<_>>()
    })?;
    println!("sections: {}", spans.len());
    for (index, (first, last, size)) in spans.iter().enumerate() {
        println!("  [{index}] paragraphs {first}..={last:?}, page {size:?}");
    }
    anyhow::ensure!(
        spans.len() == 2,
        "the document should now have two sections"
    );

    // ---- A header on the first section ------------------------------------------------------------
    // `create_header` writes the part, registers its content type, relates it from the main document
    // part and wires `w:headerReference` into that section's own `w:sectPr` — all four, because three
    // of them produce a file Word repairs.
    let header = document.create_header(
        SectionLocation::Paragraph(0.into()),
        HeaderFooterType::Default,
    )?;
    // A new part is created holding one *empty* paragraph, so this fills that one in rather than
    // appending a second and leaving a blank line above it.
    document.edit_header_footer(&header, |content, interner| {
        if let Some(paragraph) = content.paragraph_mut(0) {
            paragraph.append_run(Run::with_text(interner, "Quarterly Review — Internal"));
        }
    })?;
    let footer = document.create_footer(SectionLocation::Body, HeaderFooterType::Default)?;
    document.edit_header_footer(&footer, |content, interner| {
        if let Some(paragraph) = content.paragraph_mut(0) {
            paragraph.append_run(Run::with_text(interner, "Page footer"));
        }
    })?;
    println!("header part: {}", header.as_str());
    println!("footer part: {}", footer.as_str());

    // ---- Save, reopen, and resolve --------------------------------------------------------------
    let bytes = document.save().context("saving")?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    let mut reopened = Document::open(&bytes).context("reopening")?;
    anyhow::ensure!(
        reopened.sections(|spans, _| spans.len())? == 2,
        "both sections should survive the round trip"
    );
    anyhow::ensure!(
        !reopened.even_and_odd_headers()?,
        "nothing set w:evenAndOddHeaders, so it must read false"
    );

    let first = reopened
        .resolve_header(0, HeaderFooterType::Default)?
        .context("section 0 should resolve a default header")?;
    let second = reopened
        .resolve_header(1, HeaderFooterType::Default)?
        .context("section 1 should inherit section 0's default header")?;
    anyhow::ensure!(
        first == second,
        "a section that names no header of its own must inherit the previous section's"
    );
    let header_text = reopened.header_footer(&first, |content, _| {
        content
            .paragraphs()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    anyhow::ensure!(
        header_text == "Quarterly Review — Internal",
        "the header text did not survive the round trip"
    );
    println!(
        "reopened: section 1 inherits {} — {header_text:?}",
        second.as_str()
    );

    // ---- Removing a reference ----------------------------------------------------------------------
    // Removing section 0's header leaves section 1 with nothing to inherit, so the resolution that
    // succeeded a moment ago now answers `None`. Same call, different answer, because the state it
    // reads really changed.
    reopened.remove_header(
        SectionLocation::Paragraph(0.into()),
        HeaderFooterType::Default,
    )?;
    anyhow::ensure!(
        reopened
            .resolve_header(1, HeaderFooterType::Default)?
            .is_none(),
        "with the only header removed, section 1 has nothing left to inherit"
    );
    println!("after removing section 0's header, section 1 resolves to nothing");

    Ok(())
}
