//! The building-a-document walkthrough, written entirely through the facade.
//!
//! `bindings/mjx-python/tests/test_build_a_document.py` and
//! `bindings/mjx-wasm/tests/node/build_a_document.mjs` are the same walkthrough, call for call, in
//! the other two languages MJXOFF-139 curated `Document` for — proof that the curated surface is
//! actually enough to author a real document, not merely to open one.
//!
//! ```sh
//! cargo run -p mjx-ooxml --example build_a_document -- out.docx
//! ```
//!
//! Note where the file I/O is: right here, in the caller. The library is bytes-in and bytes-out and
//! never touches a filesystem — which is exactly why the same calls work unchanged in a browser.

use std::path::PathBuf;

use mjx_ooxml::{Document, Format, HeaderFooterType, HyperlinkTarget, PageSize, SectionLocation};

/// Where this example writes: its first argument, or `target/examples/` by default.
fn output_path() -> PathBuf {
    match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("facade_build_a_document.docx")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = output_path();

    // ---- Blank -------------------------------------------------------------------------------
    // Nothing is read from disk: every part is authored from this library's own element builders,
    // which is what makes a document buildable from a `pip install` or a browser tab with no input
    // file.
    let mut document = Document::blank(PageSize::a4())?;
    assert_eq!(document.format(), Format::Document);
    assert_eq!(document.paragraph_count()?, 1);

    // ---- Paragraphs and runs ------------------------------------------------------------------
    // `Document::blank` writes one *empty* paragraph — no run to edit yet, so the first line is
    // authored with `append_run` rather than `set_run_text`.
    document.append_run(0, "Quarterly Review")?;
    document.append_paragraph()?;
    document.append_run(1, "Prepared by the mjx-ooxml-rs example suite.")?;
    document.append_paragraph()?;
    document.append_run(2, "Highlights")?;
    document.append_paragraph()?;
    document.append_run(3, "Revenue grew across every region this quarter.")?;

    // A numbered list: two more paragraphs, attached to numbering instance 1 at level 0. (No
    // `word/numbering.xml` exists yet on a blank document; `attach_paragraph_to_list` writes the
    // reference regardless — resolving it against a real list definition is a step a caller adds
    // through `Document::document_mut` when one exists.)
    document.append_paragraph()?;
    document.append_run(4, "North America: +12%")?;
    document.attach_paragraph_to_list(4, 1, 0)?;
    document.append_paragraph()?;
    document.append_run(5, "EMEA: +8%")?;
    document.attach_paragraph_to_list(5, 1, 0)?;

    // ---- A hyperlink ---------------------------------------------------------------------------
    document.append_paragraph()?;
    document.append_run(6, "Full figures: ")?;
    document.insert_hyperlink(
        6,
        1,
        "investor relations page",
        &HyperlinkTarget::Url("https://example.com/investors".to_owned()),
    )?;

    // ---- A table ---------------------------------------------------------------------------------
    let table = document.append_table(2, 2)?;
    document.set_cell_text(table, 0, 0, "Region")?;
    document.set_cell_text(table, 0, 1, "Growth")?;
    document.set_cell_text(table, 1, 0, "North America")?;
    document.set_cell_text(table, 1, 1, "+12%")?;
    let (rows, columns) = document.table_dimensions(table)?;
    assert_eq!((rows, columns), (2, 2));

    // ---- A header and a comment -------------------------------------------------------------------
    document.set_header_text(
        SectionLocation::Body,
        HeaderFooterType::Default,
        "Quarterly Review — Internal",
    )?;
    let comment_id = document.add_comment(
        0,
        "Reviewer",
        Some("R"),
        "Confirm the North America figure before publishing.",
    )?;
    assert!(document.comment_range_text(comment_id)?.is_some());

    // ---- A footnote --------------------------------------------------------------------------------
    document.add_footnote(3, "Figures are unaudited and subject to revision.")?;

    // ---- Save --------------------------------------------------------------------------------------
    document.validate()?;
    let bytes = document.save()?;
    std::fs::write(&out, &bytes)?;
    println!("wrote {} bytes to {}", bytes.len(), out.display());

    // ---- Reopen, to prove the bytes are a real document --------------------------------------------
    let mut reopened = Document::open(&bytes)?;
    assert_eq!(reopened.paragraph_count()?, document.paragraph_count()?);
    assert_eq!(reopened.paragraph_text(0)?, "Quarterly Review");
    assert_eq!(reopened.cell_text(0, 0, 0)?, "Region");
    assert_eq!(
        reopened
            .header_text(0, HeaderFooterType::Default)?
            .as_deref(),
        Some("Quarterly Review — Internal")
    );
    assert_eq!(reopened.comments()?.len(), 1);
    assert_eq!(reopened.footnotes()?.len(), 1);

    Ok(())
}
