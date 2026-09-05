# Text, runs and annotations

[Building a document](building_a_document) got text into a document. This page is about addressing a
particular piece of it, editing it precisely, and the four things that hang off a piece of text
without being text: fields, comments, notes and tracked changes.

The runnable versions are `examples/edit_text.rs` and `examples/annotations.rs`:

```sh
cargo run -p mjx-docx --example edit_text -- out.docx
cargo run -p mjx-docx --example annotations -- out.docx
```

## One address space, two coordinates

A paragraph is named by a [`BlockPath`], a run within it by a [`RunPath`]. Both are sequences of
indices, and both accept a bare `usize` for the common case — the top level. Nothing is ever handed
out to hold: there is no `Paragraph` handle that stays valid across an edit, because a handle would
have to be invalidated by every insert, and an invalidated handle is a bug that compiles.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "First")?;
document.append_paragraph()?;
document.append_run(1, "Second")?;

assert_eq!(document.paragraph_count()?, 2);
assert_eq!(document.run_count(1)?, 1);
assert_eq!(document.paragraph_text(1)?, "Second");
assert_eq!(document.run_text(1, 0)?, "Second");
# Ok(())
# }
```

**Positions shift.** Inserting a paragraph in front of another moves it, and the index that named it
a moment ago now names its neighbour. That is not a wart of this library — it is what a positional
address means — but it is the mistake this API makes easiest to hit, so `examples/edit_text.rs`
demonstrates it deliberately rather than avoiding it.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Originally first")?;
document.insert_paragraph(0)?;
document.insert_run(0, 0, "Now first")?;

// The paragraph that was 0 is 1.
assert_eq!(document.paragraph_text(0)?, "Now first");
assert_eq!(document.paragraph_text(1)?, "Originally first");
# Ok(())
# }
```

## Text at three scopes

[`paragraph_text`](Document::paragraph_text) concatenates every run in a paragraph;
[`run_text`](Document::run_text) reads one run. Neither dirties a part —
but both take `&mut self`, because a part is raw bytes until something needs it parsed, and the first
read materialises the tree.

Writing has the same two scopes, plus the structural ones:

| To do this | Call |
|---|---|
| Change a run that already exists | [`set_run_text`](Document::set_run_text) |
| Add a run to a paragraph that has none, or one at a position | [`append_run`](Document::append_run) / [`insert_run`](Document::insert_run) |
| Take a run away | [`remove_run`](Document::remove_run) |
| Add or remove a whole paragraph | [`append_paragraph`](Document::append_paragraph) / [`insert_paragraph`](Document::insert_paragraph) / [`remove_paragraph`](Document::remove_paragraph) |

A blank document's one paragraph is genuinely empty — `<w:p/>`, with no run at all — so the first
call on a fresh document is `insert_run`, never `set_run_text`. `set_run_text` on a run that does not
exist is [`DocxError::AddressNotFound`], not a silent no-op.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, DocxError, PageSize};

let mut document = Document::blank(PageSize::a4())?;
assert!(matches!(
    document.set_run_text(0, 0, "there is no run here yet"),
    Err(DocxError::AddressNotFound(_))
));

document.insert_run(0, 0, "now there is")?;
document.set_run_text(0, 0, "and it can be edited")?;
assert_eq!(document.paragraph_text(0)?, "and it can be edited");
# Ok(())
# }
```

## Equations

An equation is not text with symbols in it: it is Office MathML (`m:oMath`), a vocabulary of its own
that [`mjx_omml`] models. [`append_math`](Document::append_math) hands you the part's own
[`Interner`](mjx_ooxml_core::Interner) and takes the [`Math`](mjx_omml::Math) you build with it —
the interner has to be the document's, because a value built against a throwaway one would carry
symbols that resolve to the wrong strings once written back.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};
use mjx_omml::{Argument, Fraction, Math, MathElement};

let mut document = Document::blank(PageSize::a4())?;
document.append_math(0, |interner| {
    let numerator = Argument::with_text(interner, "num", "a");
    let denominator = Argument::with_text(interner, "den", "b");
    let fraction = Fraction::new(interner, numerator, denominator);
    Math::with_elements(interner, &[MathElement::Fraction(fraction)])
})?;

// An equation's own text is not run text: `paragraph_text` reads `w:r/w:t`, and `m:t` is a
// different element in a different namespace. The equation is reached by naming its shape.
assert_eq!(document.paragraph_text(0)?, "");

// `["f", "num", "r", "t"]` is the chain of local names from `m:oMath` down to the numerator's own
// text — everything the edit does not name keeps its own bytes.
document.set_equation_run_text(0, 0, &["f", "num", "r", "t"], "x")?;

// A path that names nothing is an error, not a silent miss.
assert!(document
    .set_equation_run_text(0, 0, &["f", "den", "rad", "r", "t"], "y")
    .is_err());
# Ok(())
# }
```

## Run-level content that is not text

A run can hold a drawing, a legacy VML picture, an embedded object or an ActiveX control instead of
(or beside) its text. [`paragraph_run_content`](Document::paragraph_run_content) hands back every
run's content in document order so a caller can match on
[`RunInnerContent`](crate::RunInnerContent) without reaching into this crate's private machinery.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize, RunInnerContent};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Figure 1")?;
let picture_id = document.add_inline_picture(
    0,
    vec![0x89, b'P', b'N', b'G'],
    "image/png",
    "png",
    914_400,
    914_400,
    "Figure 1",
)?;

let drawings = document.paragraph_run_content(0, |content, _| {
    content
        .iter()
        .filter(|item| matches!(item, RunInnerContent::Drawing(_)))
        .count()
})?;
assert_eq!(drawings, 1);

// A drawing is addressed again by the `wp:docPr` id it was given.
assert!(document.remove_drawing(picture_id)?);
assert!(!document.remove_drawing(picture_id)?);
# Ok(())
# }
```

## Comments, footnotes and endnotes

Each of the three is **two** things: an entry in its own part, and a marker in the body pointing at
it. Writing one without the other is a file Word offers to repair, so every constructor here writes
both — and creates the part, its content-type registration and its relationship on the first call.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Revenue grew across every region.")?;

let comment = document.add_comment(0, "Reviewer", Some("R"), "Check this figure.")?;
let footnote = document.add_footnote(0, "Unaudited.")?;

// The range markers in the body …
assert_eq!(
    document.comment_range_text(comment)?.as_deref(),
    Some("Revenue grew across every region.")
);
// … and the entry in word/comments.xml.
let author = document
    .comments(|comments, interner| {
        comments
            .comment(interner, comment)
            .and_then(|entry| entry.author(interner))
    })?
    .flatten();
assert_eq!(author.as_deref(), Some("Reviewer"));

// A footnotes part always carries the two reserved separator entries beside the author's own.
let (all, mine) = document
    .footnotes(|notes, interner| (notes.footnotes().count(), notes.user_footnotes(interner).count()))?
    .unwrap_or((0, 0));
assert_eq!((all, mine), (3, 1));
let _ = footnote;
# Ok(())
# }
```

[`remove_comment`](Document::remove_comment) takes both halves — and, when it was the last comment,
`word/comments.xml` and its relationship as well, because an orphaned part is a packaging defect
[`validate`](Document::validate) would refuse to write.

## Bookmarks, and the hyperlinks that name them

A [`HyperlinkTarget`](crate::HyperlinkTarget) is either a `Url` (through the part's own
relationships) or an `Anchor` — a bookmark name, handed back **unresolved**, because a bookmark can
move independently of any link that names it. [`resolve_bookmark`](Document::resolve_bookmark) is the
second half of that walk, and it answers `None` for a name nothing defines rather than guessing.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{BookmarkResolution, Document, HyperlinkTarget, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Chapter Three")?;
document.append_paragraph()?;
document.append_run(1, "See ")?;
document.insert_hyperlink(1, 1, "chapter three", &HyperlinkTarget::Anchor("ch3".to_owned()))?;

// The link names a bookmark nothing has defined yet.
assert_eq!(
    document.hyperlink_target(1, 1)?,
    Some(HyperlinkTarget::Anchor("ch3".to_owned()))
);
assert!(document.resolve_bookmark("ch3")?.is_none());

// Define it, and the same call answers.
let id = document.add_bookmark(0, "ch3")?;
assert_eq!(
    document.resolve_bookmark("ch3")?,
    Some(BookmarkResolution::Resolved {
        id,
        text: "Chapter Three".to_owned()
    })
);
# Ok(())
# }
```

## Tracked changes

Revisions are read, never applied. [`revisions`](Document::revisions) reports every tracked change in
the document — the body, the headers and footers, the comments and both note parts —
and the two text readers answer what the document *would* say under each decision, without rewriting
anything:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Nothing here is tracked.")?;

assert!(document.revisions()?.is_empty());
// With no revisions the two answers agree — which is the check that both readers ran.
assert_eq!(
    document.text_with_revisions_accepted()?,
    document.text_with_revisions_rejected()?
);
# Ok(())
# }
```

Accepting or rejecting a revision — actually rewriting the document to match one of those two
answers — is not something this library does. See [fidelity and the known gaps](fidelity_and_gaps).

## What to read next

[Tables, sections and headers](tables_sections_and_headers) for the structures a run sits inside, and
[styles and inheritance](styles_and_inheritance) for where a run's formatting comes from when the run
itself states none.
