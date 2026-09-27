//! Tracked changes, and **which text a display mode measures** — the half of revision marks that is
//! layout rather than decoration.
//!
//! # A deletion occupies space, or it does not, and the document paginates differently
//!
//! This is the whole reason revision marks are a layout subject at all. A reviewer's deletion shown
//! as struck-through text is **on the line**: it is measured, it takes width, it pushes the words
//! after it along, and a page that held forty lines with it holds forty-one without. The same
//! document, the same fonts, the same margins, and a different page for every paragraph after the
//! first deletion.
//!
//! So a renderer that drew deletions in red with a line through them and *measured the same text in
//! every mode* would be tinting rather than laying out — and every gate on colour would pass.
//! `tests/a_deletion_changes_the_page.rs` asserts the page **count**, which no amount of tinting can
//! change.
//!
//! # The four modes, as three subsets of one string
//!
//! `mjx_docx::ParagraphFormatting::text` is the **all-markup** view: everything the file holds,
//! insertions and deletions alike, with [`mjx_docx::RevisionSpan`]s saying which bytes are which.
//! Each of Word's display modes is that string with some spans dropped:
//!
//! | `w:revisionView` / Word's menu | this | drops |
//! |---|---|---|
//! | *All Markup* | [`RevisionView::AllMarkup`] | nothing |
//! | *Simple Markup* | [`RevisionView::SimpleMarkup`] | nothing — the change bar replaces the marks |
//! | *No Markup* (final) | [`RevisionView::NoMarkup`] | `w:del`, `w:moveFrom` |
//! | *Original* | [`RevisionView::Original`] | `w:ins`, `w:moveTo` |
//!
//! **Simple Markup drops nothing**, and that is the one a reader guesses wrong: Word's *Simple
//! Markup* shows the document as it would be *with the changes accepted* — so it drops the same
//! spans *No Markup* does — and marks the changed lines with a bar in the margin. It is
//! [`RevisionView::SimpleMarkup`]'s [`RevisionView::shows_change_bars`] that distinguishes it, not
//! its text. See that method for the citation and the `GUESS:` it rests on.
//!
//! # What this module is not
//!
//! It does not colour anything, and it does not draw a strike-through. Both are *paint*, and this
//! crate resolves no paint at all — the same line `w:pBdr` and a `bar` tab stop already sit on. What
//! travels to a scene companion is [`crate::generated::Composition`]'s per-piece
//! [`crate::generated::PieceRevision`], which says *this stretch of the line is a deletion by
//! Priya*, and drawing it is `mjx-scene-docx`'s.

use mjx_docx::{RevisionKind, RevisionSpan};

/// Which of Word's four review views a document is laid out in.
///
/// # Why the default is `NoMarkup` and not `AllMarkup`
///
/// **A file with no revisions in it lays out identically in all four**, so the default cannot be
/// chosen by testing. It is chosen by asking what a reader who opens a document expects to see, and
/// the answer is the document — not its edit history. `w:revisionView` in `word/settings.xml` states
/// which marks are *suppressed*, never which are shown, so a document that states nothing is
/// showing what Word's own default shows, and Word's default for a document received with tracked
/// changes is *Simple Markup*: the final text, with a bar in the margin.
///
/// So the default here is [`RevisionView::SimpleMarkup`] — which measures exactly what
/// [`RevisionView::NoMarkup`] measures and additionally reports where the change bars go. A caller
/// showing a review interface passes [`RevisionView::AllMarkup`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum RevisionView {
    /// Every insertion and deletion is shown, marked.
    AllMarkup,
    /// The text as it would be with every change accepted, with a change bar beside every line that
    /// carries one. Word's own default, and this crate's.
    #[default]
    SimpleMarkup,
    /// The text as it would be with every change accepted, unmarked.
    NoMarkup,
    /// The text as it was before any change was made.
    Original,
}

impl RevisionView {
    /// Whether a span of `kind` contributes its characters in this view.
    ///
    /// The **only** question this module answers about layout, and every other behaviour follows
    /// from it: a span that does not contribute is not measured, is not on a line, and does not push
    /// the page.
    #[must_use]
    pub fn shows(self, kind: RevisionKind) -> bool {
        match kind {
            // Content that was deleted or moved away is in the file and not in the finished
            // document. Only a markup view draws it.
            RevisionKind::Deleted | RevisionKind::MovedFromContent => self == Self::AllMarkup,
            // Content that was inserted or moved here is in the finished document and was not in
            // the original. Only the original view drops it.
            RevisionKind::Inserted | RevisionKind::MovedToContent => self != Self::Original,
            // Every other kind changes a *property* rather than a character — `w:rPrChange`,
            // `w:pPrChange`, a table grid change. None of them has a span in a paragraph's text, so
            // none reaches here; showing them is the safe answer if one ever does.
            _ => true,
        }
    }

    /// Whether this view draws a bar in the margin beside a changed line.
    ///
    /// **`GUESS:`** *Simple Markup* and *All Markup* draw one and the other two do not.
    /// `w:revisionView` in `word/settings.xml` says which *marks* a document suppresses and says
    /// nothing about a bar; the bar is Word's own interface rather than a document property, and the
    /// reason *Simple Markup* is a distinct view at all is that it is the one where the bar is the
    /// only mark there is.
    #[must_use]
    pub fn shows_change_bars(self) -> bool {
        matches!(self, Self::AllMarkup | Self::SimpleMarkup)
    }
}

/// Whether `spans` hide any part of the paragraph in `view` — that is, whether the paragraph's
/// layout text differs from its all-markup text.
///
/// Cheap, and worth having: a document with no tracked changes in it takes the same path in every
/// view, so nothing pays for the feature it does not use.
#[must_use]
pub fn changes_anything(spans: &[RevisionSpan], view: RevisionView) -> bool {
    spans
        .iter()
        .any(|span| !span.range.is_empty() && !view.shows(span.kind))
}

/// Whether any of `spans` is a *content* change — which is what a change bar is drawn for.
#[must_use]
pub fn has_content_change(spans: &[RevisionSpan]) -> bool {
    spans.iter().any(|span| {
        matches!(
            span.kind,
            RevisionKind::Inserted
                | RevisionKind::Deleted
                | RevisionKind::MovedFromContent
                | RevisionKind::MovedToContent
        )
    })
}

/// The innermost span covering `at`, if any.
///
/// Innermost, because the four containers nest: Word writes a `w:del` inside a `w:ins` for text one
/// reviewer added and another removed, and what a reader must see is that it was **deleted**. The
/// list is in document order with an outer span before the inner one it contains, so the last match
/// is the innermost.
#[must_use]
pub fn covering(spans: &[RevisionSpan], at: usize) -> Option<&RevisionSpan> {
    spans
        .iter()
        .rfind(|span| span.range.start <= at && at < span.range.end)
}
