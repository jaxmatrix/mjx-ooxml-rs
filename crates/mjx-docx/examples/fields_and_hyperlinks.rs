//! Fields, hyperlinks, bookmarks and form fields, read and edited on a document that already has
//! them — the runnable version of
//! [the fields section of the tables-and-structure guide](mjx_docx::guide::tables_sections_and_headers).
//!
//! ```sh
//! cargo run -p mjx-docx --example fields_and_hyperlinks -- out.docx
//! ```
//!
//! This one starts from `tests/fixtures/fields_and_hyperlinks.docx` rather than from nothing,
//! because the shape that matters cannot be built by accident: a `TOC` field whose cached result
//! holds two `PAGEREF` fields of its own. A reader that counts `begin`/`end` markers instead of
//! pairing them with a stack reports three top-level fields here and mis-scopes every instruction;
//! the assertion below — one top-level field, two nested inside it — is what tells the two apart.

use anyhow::{Context, Result};
use mjx_docx::{BookmarkResolution, Document, FieldForm, HyperlinkTarget};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("fields_and_hyperlinks.docx");
    let bytes = support::fixture("fields_and_hyperlinks.docx")?;
    let mut document = Document::open(&bytes).context("opening the fixture")?;

    // ---- Nesting, paired rather than counted ---------------------------------------------------
    let fields = document.fields(0)?;
    println!("paragraph 0: {} top-level field(s)", fields.len());
    for field in &fields {
        println!(
            "  {:?} {:?} → {:?} ({} nested)",
            field.form(),
            field.field_name(),
            field.cached_result(),
            field.nested_fields().len()
        );
    }
    anyhow::ensure!(fields.len() == 1, "the TOC is one field, not three");
    let toc = &fields[0];
    anyhow::ensure!(
        toc.form() == FieldForm::Complex,
        "the TOC is a complex field"
    );
    anyhow::ensure!(toc.field_name() == Some("TOC"), "the instruction keyword");
    anyhow::ensure!(
        toc.nested_fields().len() == 2,
        "both PAGEREF fields belong to the TOC's cached result"
    );
    anyhow::ensure!(
        toc.nested_fields()
            .iter()
            .all(|nested| nested.field_name() == Some("PAGEREF")),
        "the nested fields are the two PAGEREFs"
    );

    // An instruction split across two `w:instrText` runs is one instruction. Paragraph 1's HYPERLINK
    // field is written as ` HYPER` + `LINK "http://example.com" `, which is legal and common.
    let hyperlink_field = document.fields(1)?;
    anyhow::ensure!(
        hyperlink_field[0].field_name() == Some("HYPERLINK"),
        "an instruction split across runs must still read as one keyword"
    );

    // ---- Editing an instruction and a cached result ----------------------------------------------
    // Editing the instruction rewrites the field's own `w:instrText` runs and leaves the cached
    // result alone; the two are never the same accessor, because a field's code and its last
    // rendered value are different things and this library never evaluates one into the other.
    document.set_field_instruction(2, 0, " DATE \\@ \"yyyy-MM-dd\" ")?;
    document.set_field_cached_result_text(1, 0, "example.com (updated)")?;

    // ---- Hyperlinks: two kinds of target ------------------------------------------------------------
    // `r:id` resolves through the part's own relationships to a URL; `w:anchor` is a bookmark name in
    // this document, handed back unresolved because a bookmark can move independently of any link.
    let external = document.hyperlink_target(3, 0)?;
    let internal = document.hyperlink_target(4, 0)?;
    println!("hyperlink targets: {external:?} / {internal:?}");
    anyhow::ensure!(
        external == Some(HyperlinkTarget::Url("http://example.com/target".to_owned())),
        "the r:id hyperlink should resolve to its relationship target"
    );
    anyhow::ensure!(
        internal == Some(HyperlinkTarget::Anchor("chapter3".to_owned())),
        "the w:anchor hyperlink should hand back the bookmark name"
    );

    // ---- A bookmark, and resolving the anchor against it ----------------------------------------------
    // The fixture's second hyperlink points at `chapter3`, which nothing defines — so it resolves to
    // nothing until this example adds the bookmark. Same call, two answers.
    anyhow::ensure!(
        document.resolve_bookmark("chapter3")?.is_none(),
        "the fixture has no such bookmark yet"
    );
    document.append_paragraph()?;
    let target = document.paragraph_count()? - 1;
    document.append_run(target, "Chapter Three")?;
    let bookmark_id = document.add_bookmark(target, "chapter3")?;
    let resolved = document
        .resolve_bookmark("chapter3")?
        .context("the bookmark should resolve once it exists")?;
    println!("bookmark {bookmark_id}: {resolved:?}");
    anyhow::ensure!(
        resolved
            == BookmarkResolution::Resolved {
                id: bookmark_id,
                text: "Chapter Three".to_owned(),
            },
        "the bookmark should cover the run this example wrote"
    );

    // ---- A new hyperlink -------------------------------------------------------------------------------
    document.append_paragraph()?;
    let link_paragraph = document.paragraph_count()? - 1;
    document.append_run(link_paragraph, "Full figures: ")?;
    document.insert_hyperlink(
        link_paragraph,
        1,
        "investor relations",
        &HyperlinkTarget::Url("https://example.com/investors".to_owned()),
    )?;

    // ---- A form field's own data -------------------------------------------------------------------------
    let checkbox_name = document.form_field(5, 0, |data, interner| {
        data.and_then(|data| data.name(interner))
    })?;
    println!("form field in paragraph 5: {checkbox_name:?}");
    anyhow::ensure!(
        checkbox_name.as_deref() == Some("Approved"),
        "the checkbox form field's own w:name"
    );
    document.edit_form_field(5, 0, |data, interner| data.set_name(interner, "ApprovedBy"))??;

    // ---- Save, reopen, and check every edit --------------------------------------------------------------
    let saved = document.save().context("saving")?;
    std::fs::write(&out, &saved).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), saved.len());

    let mut reopened = Document::open(&saved).context("reopening")?;
    let reopened_date = reopened.fields(2)?;
    anyhow::ensure!(
        reopened_date[0].instruction() == " DATE \\@ \"yyyy-MM-dd\" ",
        "the edited instruction did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.fields(1)?[0].cached_result() == Some("example.com (updated)"),
        "the edited cached result did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.fields(0)?[0].nested_fields().len() == 2,
        "the TOC's nesting did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.resolve_bookmark("chapter3")?.is_some(),
        "the bookmark did not survive the round trip"
    );
    anyhow::ensure!(
        reopened.hyperlink_target(link_paragraph, 1)?
            == Some(HyperlinkTarget::Url(
                "https://example.com/investors".to_owned()
            )),
        "the inserted hyperlink did not survive the round trip"
    );
    anyhow::ensure!(
        reopened
            .form_field(5, 0, |data, interner| data
                .and_then(|data| data.name(interner)))?
            .as_deref()
            == Some("ApprovedBy"),
        "the renamed form field did not survive the round trip"
    );
    println!("reopened: fields, bookmark, hyperlink and form field all intact");

    Ok(())
}
