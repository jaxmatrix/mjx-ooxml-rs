//! Creating a document from nothing — the runnable version of
//! [the building-a-document guide](mjx_docx::guide::building_a_document).
//!
//! No template, no fixture, no file read at all: `Document::blank` authors `[Content_Types].xml`,
//! the package-root relationships, `word/document.xml` and both `docProps` parts, and this example
//! fills the body in and writes it out.
//!
//! ```sh
//! cargo run -p mjx-docx --example blank_document -- out.docx
//! ```
//!
//! Then it reopens what it wrote and asserts on it — the only file the library itself touches is the
//! one `main` hands it.

use anyhow::{Context, Result};
use mjx_docx::{Document, PageSize};
use mjx_opc::doc_props::{CoreProperties, DocumentTimestamp, ExtendedProperties};

/// Where this example writes: its first argument, or `blank_document.docx` under the target
/// directory — the same convention `mjx-pptx`'s examples use.
fn output_path() -> std::path::PathBuf {
    match std::env::args().nth(1) {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let dir =
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/examples");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("blank_document.docx")
        }
    }
}

fn main() -> Result<()> {
    let out = output_path();

    // ---- A document from nothing, with document properties set ------------------------------
    // Every part below is authored in memory. `DocumentTimestamp` has no `now()` — a value can only
    // be built from caller-supplied fields, so this example (and `Document::blank` itself) never
    // depends on the wall clock.
    let created = DocumentTimestamp::new(2024, 1, 1, 0, 0, 0)?;
    let mut document = Document::blank_with_properties(
        PageSize::a4(),
        &CoreProperties {
            title: Some("Built from nothing".to_owned()),
            creator: Some("mjx-ooxml-rs".to_owned()),
            created: Some(created),
            modified: Some(created),
        },
        &ExtendedProperties {
            application: Some("mjx-ooxml-rs".to_owned()),
        },
    )
    .context("building a blank document")?;
    println!(
        "blank document: {} paragraph(s) to start",
        document.paragraph_count()?
    );

    // ---- Fill in the one paragraph a blank document starts with -----------------------------
    // `Document::blank`'s paragraph is genuinely empty (`<w:p/>`, no run at all yet), so this is a
    // run to *insert*, not text to *set* on one that already exists.
    document
        .insert_run(0, 0, "This document did not exist a moment ago.")
        .context("filling the first paragraph")?;

    // ---- Append a second paragraph -------------------------------------------------------------
    document.append_paragraph().context("append a paragraph")?;
    document
        .append_run(
            1,
            "Every part was authored in memory, in twips A4 can express.",
        )
        .context("append a run")?;

    // ---- Save -----------------------------------------------------------------------------------
    let bytes = document.save()?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    // ---- Reopen what was written, and check it ---------------------------------------------------
    let mut reopened = Document::open(&bytes).context("reopening the document just written")?;
    anyhow::ensure!(reopened.paragraph_count()? == 2, "expected two paragraphs");
    anyhow::ensure!(
        reopened.paragraph_text(0)? == "This document did not exist a moment ago.",
        "the first paragraph did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.paragraph_text(1)?
            == "Every part was authored in memory, in twips A4 can express.",
        "the second paragraph did not survive the round trip"
    );
    println!(
        "reopened: {} paragraph(s), text intact",
        reopened.paragraph_count()?
    );

    Ok(())
}
