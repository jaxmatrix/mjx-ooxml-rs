//! Comments, footnotes and endnotes — the runnable version of
//! [the annotations section of the text-and-formatting guide](mjx_docx::guide::text_and_formatting).
//!
//! ```sh
//! cargo run -p mjx-docx --example annotations -- out.docx
//! ```
//!
//! Each of the three is *two* things: an entry in its own part, and a marker in the body that points
//! at it. Writing one without the other produces a file Word repairs, so every assertion below
//! checks both halves — the entry's own text through `word/comments.xml`/`footnotes.xml`, and the
//! range or reference in `word/document.xml` that names it.

use anyhow::{Context, Result};
use mjx_docx::{Document, FootnoteEndnote, PageSize};

mod support;

fn main() -> Result<()> {
    let out = support::output_path("annotations.docx");
    let mut document = Document::blank(PageSize::a4()).context("blank document")?;
    document.insert_run(0, 0, "Revenue grew across every region this quarter.")?;
    document.append_paragraph()?;
    document.append_run(1, "Figures are preliminary.")?;

    // ---- A blank document has none of the three parts ------------------------------------------
    anyhow::ensure!(
        document.comments(|_, _| ())?.is_none()
            && document.footnotes(|_, _| ())?.is_none()
            && document.endnotes(|_, _| ())?.is_none(),
        "a blank document relates to no comments, footnotes or endnotes part"
    );

    // ---- A comment ------------------------------------------------------------------------------
    // `add_comment` writes the entry in `word/comments.xml` *and* the
    // `w:commentRangeStart`/`w:commentRangeEnd`/`w:commentReference` triple around the paragraph it
    // anchors to, creating the part on the first call.
    let comment_id = document.add_comment(
        0,
        "Reviewer",
        Some("R"),
        "Confirm the North America figure before publishing.",
    )?;
    let covered = document
        .comment_range_text(comment_id)?
        .context("the comment range should cover the paragraph it was anchored to")?;
    println!("comment {comment_id} covers {covered:?}");
    anyhow::ensure!(
        covered == "Revenue grew across every region this quarter.",
        "the comment range should cover exactly the paragraph it was anchored to"
    );

    // ---- Footnotes and endnotes -------------------------------------------------------------------
    // Both parts are created with the two reserved `separator`/`continuationSeparator` entries every
    // real one carries — which is why the "how many notes are there" question has two answers, and
    // why `user_footnotes` exists beside `footnotes`.
    let footnote_id = document.add_footnote(1, "Unaudited and subject to revision.")?;
    let endnote_id = document.add_endnote(1, "Source: internal reporting, Q4.")?;
    println!("footnote {footnote_id}, endnote {endnote_id}");

    let (all_notes, user_notes) = document
        .footnotes(|footnotes, interner| {
            (
                footnotes.footnotes().count(),
                footnotes.user_footnotes(interner).count(),
            )
        })?
        .context("word/footnotes.xml exists by now")?;
    println!("footnotes part: {all_notes} entries, {user_notes} of them the author's");
    anyhow::ensure!(
        all_notes == 3 && user_notes == 1,
        "the two reserved separator entries are entries too, and are not the author's"
    );

    // ---- Save, reopen, and check both halves of each ------------------------------------------------
    let bytes = document.save().context("saving")?;
    std::fs::write(&out, &bytes).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} ({} bytes)", out.display(), bytes.len());

    let mut reopened = Document::open(&bytes).context("reopening")?;
    anyhow::ensure!(
        reopened.parts().comments.is_some()
            && reopened.parts().footnotes.is_some()
            && reopened.parts().endnotes.is_some(),
        "all three parts should be related from the reopened document"
    );

    // The entry, read from its own part …
    let comment_text = reopened
        .comments(|comments, interner| {
            comments
                .comment(interner, comment_id)
                .map(mjx_docx::Comment::text)
        })?
        .flatten()
        .context("the comment entry should be in word/comments.xml")?;
    anyhow::ensure!(
        comment_text == "Confirm the North America figure before publishing.",
        "the comment's own text did not survive the round trip"
    );
    // … and the marker in the body that names it.
    anyhow::ensure!(
        reopened.comment_range_text(comment_id)?.as_deref()
            == Some("Revenue grew across every region this quarter."),
        "the comment range markers did not survive the round trip"
    );

    let footnote_text = reopened
        .footnotes(|footnotes, interner| {
            footnotes
                .footnote(interner, footnote_id)
                .map(FootnoteEndnote::text)
        })?
        .flatten()
        .context("the footnote entry should be in word/footnotes.xml")?;
    anyhow::ensure!(
        footnote_text == "Unaudited and subject to revision.",
        "the footnote's own text did not survive the round trip"
    );
    let endnote_text = reopened
        .endnotes(|endnotes, interner| {
            endnotes
                .endnote(interner, endnote_id)
                .map(FootnoteEndnote::text)
        })?
        .flatten()
        .context("the endnote entry should be in word/endnotes.xml")?;
    anyhow::ensure!(
        endnote_text == "Source: internal reporting, Q4.",
        "the endnote's own text did not survive the round trip"
    );
    println!("reopened: comment, footnote and endnote all intact, entry and marker both");

    // ---- Removing a comment takes both halves with it -------------------------------------------------
    reopened.remove_comment(comment_id)?;
    anyhow::ensure!(
        reopened.comment_range_text(comment_id)?.is_none(),
        "removing a comment must remove its body markers too, not only its entry"
    );
    // It was the only comment, so `word/comments.xml` and its relationship are swept with it —
    // otherwise `Package::validate` would report an orphaned part and `save` would refuse.
    anyhow::ensure!(
        reopened.parts().comments.is_none() && reopened.comments(|_, _| ())?.is_none(),
        "removing the last comment must take word/comments.xml with it"
    );
    reopened.validate()?;
    println!("after removal: neither the entry, the range markers, nor the part itself remain");

    Ok(())
}
