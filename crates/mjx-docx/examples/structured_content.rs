//! Content controls, custom XML, external content and an inline picture — the runnable version of
//! [the structured-content section of the tables-and-structure guide](mjx_docx::guide::tables_sections_and_headers).
//!
//! ```sh
//! cargo run -p mjx-docx --example structured_content -- out.docx
//! ```
//!
//! `tests/fixtures/structured_content.docx` is the input because the property that matters is
//! nesting: a block-level content control wrapping a table, a custom-XML wrapper around one row, a
//! cell-level control around one cell, and a run-level control inside that cell's paragraph. Row and
//! column addressing sees through the row- and cell-level wrappers — `(row, column)` reaches the same
//! cell whether or not one stands between the table and it — and asserting on the *text* of a
//! three-wrappers-deep cell is what proves the recursion happens rather than merely compiling.
//!
//! It also shows the boundary that recursion stops at, because a guide that only showed what works
//! would be an advertisement: a table wrapped in a **block-level** control is not a top-level table,
//! so `Document::table_count` does not count it. Reaching it is a walk down the wrapper's own
//! `content()`, and this example does that walk rather than pretending the shortcut exists.

use anyhow::{Context, Result};
use mjx_docx::{BlockContent, Document, MainDocument, Table};
use mjx_ooxml_core::FromXml;
use mjx_opc::{Package, PartName};

mod support;

/// The `ds:itemID` of the fixture's own Custom XML Data Storage part, and the XPath its innermost
/// content control binds to.
const STORE_ITEM_ID: &str = "{11111111-1111-1111-1111-111111111111}";
const XPATH: &str = "/ns0:customer[1]/ns0:name[1]";

/// A four-byte stand-in for image bytes: this library stores an image part exactly as given and
/// never decodes one, so an example does not need a real PNG to demonstrate the part graph.
const IMAGE_BYTES: [u8; 4] = [0x89, b'P', b'N', b'G'];

fn main() -> Result<()> {
    let out = support::output_path("structured_content.docx");
    let bytes = support::fixture("structured_content.docx")?;
    let mut document = Document::open(&bytes).context("opening the fixture")?;

    // ---- Where the shortcut stops -----------------------------------------------------------------
    // The fixture's only table is inside a block-level `w:sdt`, so it is not one of the body's own
    // top-level tables and `table_count` answers zero. That is the documented boundary, not a bug:
    // `Document`'s table methods address `w:body`'s own content.
    anyhow::ensure!(
        document.table_count()? == 0,
        "a table inside a block-level content control is not a top-level table"
    );

    // ---- Reaching it, and addressing through the wrappers below it -------------------------------
    // Row 0 is inside a `w:customXml`; its second cell is inside a cell-level `w:sdt`; and that
    // cell's paragraph holds a run-level `w:sdt` between two plain runs. None of *that* changes what
    // `(row, column)` means — `Table::row`/`Row::cell` recurse through each wrapper.
    let table = wrapped_table(&bytes).context("reaching the wrapped table")?;
    println!("wrapped table: {} row(s)", table.row_count());
    anyhow::ensure!(
        table.row_count() == 3,
        "the custom-XML-wrapped row and the repeating section's two are all rows"
    );
    let wrapped_cell = table
        .row(0)
        .and_then(|row| row.cell(1))
        .context("row 0, cell 1")?
        .text();
    println!("cell (0,1), three wrappers deep: {wrapped_cell:?}");
    anyhow::ensure!(
        wrapped_cell == "before-INNERMOST-after",
        "a run-level control's own content is part of the cell's text, in position"
    );
    anyhow::ensure!(
        table
            .row(2)
            .and_then(|row| row.cell(1))
            .context("row 2, cell 1")?
            .text()
            == "Rep2C1",
        "the repeating section's rows address like any other rows"
    );

    // ---- The data binding, resolved against the custom XML part ------------------------------------
    // The control above is bound to `/ns0:customer[1]/ns0:name[1]` in a store whose `ds:itemID` it
    // names. Resolving that is a walk across two parts, and the answer — "Jane Doe" — is the value
    // Word would push into the control, which is *not* the text the control currently displays.
    let stores = document.custom_xml_parts()?;
    println!("custom XML parts: {}", stores.len());
    anyhow::ensure!(stores.len() == 1, "the fixture relates one data store");
    let bound = document.resolve_data_binding(STORE_ITEM_ID, XPATH, |node, _| {
        node.children
            .iter()
            .find_map(|child| match child {
                mjx_ooxml_core::RawNode::Text(bytes) => {
                    Some(String::from_utf8_lossy(bytes).into_owned())
                }
                _ => None,
            })
            .unwrap_or_default()
    })?;
    println!("the binding resolves to {bound:?}");
    anyhow::ensure!(
        bound == "Jane Doe",
        "the data binding should resolve across both custom XML parts"
    );
    anyhow::ensure!(
        bound != wrapped_cell,
        "the bound value and the control's displayed text are different things, and this example \
         would not be showing anything if they happened to agree"
    );

    // ---- External content: imported, never parsed ------------------------------------------------------
    // A `w:altChunk` names a part holding a whole other document — HTML here, but equally RTF or a
    // nested `.docx`. This library stores it exactly as handed over and never converts it: Word does
    // the import when the file is opened.
    let html = b"<html><body><p>Imported by reference.</p></body></html>".to_vec();
    let chunk_id = document.add_alt_chunk(
        mjx_docx::constants::CONTENT_TYPE_ALT_CHUNK_HTML,
        html.clone(),
    )?;
    println!("alt chunk relationship: {chunk_id}");
    let (payload, content_type) = document.alt_chunk_payload(&chunk_id)?;
    anyhow::ensure!(
        payload == html.as_slice() && content_type == "text/html",
        "the payload is stored byte-for-byte, with the content type it was given"
    );

    // ---- An inline picture ------------------------------------------------------------------------------
    // The image part, its content-type registration, its relationship and the `w:drawing` that names
    // it: four things, because writing three of them is a file Word repairs. The `wp:docPr` id comes
    // back so the drawing can be addressed again later.
    document.append_paragraph()?;
    let picture_paragraph = document.paragraph_count()? - 1;
    let doc_pr_id = document.add_inline_picture(
        picture_paragraph,
        IMAGE_BYTES.to_vec(),
        "image/png",
        "png",
        914_400,
        914_400,
        "Chart snapshot",
    )?;
    println!("inline picture wp:docPr id {doc_pr_id}");

    // ---- Save, reopen, and check all three -----------------------------------------------------------------
    let saved = document.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    let mut reopened = Document::open(&saved).context("reopening")?;
    anyhow::ensure!(
        wrapped_table(&saved)
            .context("the wrapped table should still be reachable")?
            .row(0)
            .and_then(|row| row.cell(1))
            .context("row 0, cell 1")?
            .text()
            == "before-INNERMOST-after",
        "the nested content controls did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.alt_chunk_parts()?.len() == 1,
        "the imported chunk did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.alt_chunk_payload(&chunk_id)?.0 == html.as_slice(),
        "the imported chunk's bytes were altered"
    );
    let drawings = reopened.paragraph_run_content(picture_paragraph, |content, _| {
        content
            .iter()
            .filter(|item| matches!(item, mjx_docx::RunInnerContent::Drawing(_)))
            .count()
    })?;
    anyhow::ensure!(
        drawings == 1,
        "the inline picture's w:drawing did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.remove_drawing(doc_pr_id)?,
        "removing the drawing by its own wp:docPr id should report that it removed one"
    );
    anyhow::ensure!(
        !reopened.remove_drawing(doc_pr_id)?,
        "removing it a second time should report that there was nothing to remove"
    );
    println!("reopened: controls, imported chunk and picture all intact");

    Ok(())
}

/// The table inside `structured_content.docx`'s outer block-level content control.
///
/// This is the walk `Document`'s own table methods deliberately do not do: down `w:body`'s content
/// to the `w:sdt`, then down that control's own `w:sdtContent` to the `w:tbl`.
fn wrapped_table(bytes: &[u8]) -> Result<Table> {
    let mut package = Package::open(bytes).context("opening the package")?;
    let part = PartName::new("/word/document.xml").context("part name")?;
    let document = package.part_tree(&part).context("reading the main part")?;
    let main = MainDocument::from_xml(&document.root, &document.interner)
        .context("parsing the main document")?;
    let body = main.body().context("the document declares a body")?;
    let control = body
        .content()
        .iter()
        .find_map(|item| match item {
            BlockContent::StructuredDocumentTag(control) => Some(control),
            _ => None,
        })
        .context("an outer block-level content control")?;
    control
        .content_block()
        .context("the control has content")?
        .content()
        .iter()
        .find_map(|item| match item {
            BlockContent::Table(table) => Some(table.clone()),
            _ => None,
        })
        .context("a table inside the control")
}
