# Building a document

One continuous story: get a document, add paragraphs, put runs in them, save. Everything else this
crate does so far is a closer look at a step you meet here.

The runnable version of this page is `examples/blank_document.rs`:

```sh
cargo run -p mjx-docx --example blank_document -- out.docx
```

## Where the first document comes from

Two ways: from nothing, or from a file.

**From nothing.** [`Document::blank`] builds a complete document in memory — `word/document.xml`
with one empty paragraph and a page size, plus both `docProps` parts — with **no styles, settings,
fonts or theme related to it**. Nothing on disk is consulted, and no template is unpacked: every part
is markup this library writes and validates against the ECMA-376 schemas, which is why you can trust
what is in it.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
# let _ = document.paragraph_count()?;
# Ok(())
# }
```

The page is not free-form in the way you might expect: this crate fixes the margins at Word's own
"Normal" template default (1 inch on every side, ½ inch header/footer) regardless of page size, and
refuses a page those margins do not fit inside with [`DocxError::InvalidPageSize`] rather than
writing a `w:sectPr` Word offers to repair. `PageSize::a4()` and `PageSize::us_letter()` are the two
named defaults; call [`landscape`](PageSize::landscape) on either to rotate it. `Document::blank`'s
own module doc (`crates/mjx-docx/src/blank.rs`) explains exactly which optional parts a blank
document does and does not get, and why the answer is different from `mjx_pptx::Presentation::blank`'s
— a document with no related `styles.xml` is not the same kind of unusable a deck with no master is.

**From a file.** [`Document::open`] takes any `.docx` you supply. Reach for it when you want
somebody *else's* styles and settings — a corporate template is the usual reason.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::Document;

let template = std::fs::read("template.docx")?;
let mut document = Document::open(&template)?;
# Ok(())
# }
```

## Paragraphs and runs

A blank document starts with exactly one paragraph, and it is genuinely empty — `<w:p/>`, no run at
all — so the first thing most callers do is [`insert_run`](Document::insert_run) rather than
[`set_run_text`](Document::set_run_text), which edits a run that already exists:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::us_letter())?;
document.insert_run(0, 0, "The first paragraph's first run.")?;
document.append_paragraph()?;
document.append_run(1, "A second paragraph.")?;

assert_eq!(document.paragraph_count()?, 2);
assert_eq!(document.paragraph_text(1)?, "A second paragraph.");
# Ok(())
# }
```

[`insert_paragraph`](Document::insert_paragraph) / [`remove_paragraph`](Document::remove_paragraph)
and [`insert_run`](Document::insert_run) / [`remove_run`](Document::remove_run) round out the set —
every one addresses by position ([`BlockPath`]/[`RunPath`]), never by holding a reference across
calls, which is what lets each edit dirty only the one part (`word/document.xml`) and leave
everything else byte-identical.

## What survives, what does not

A blank document's `word/document.xml` is the only WordprocessingML markup it ships — see
`crates/mjx-docx/src/blank.rs`'s module doc for the full "deliberately absent" list and the reasoning
behind it. Everything that module omits ([`Document::parts`]'s `styles` / `numbering` / `settings` /
`web_settings` / `font_table` / `theme` / `headers` / `footers` / `footnotes` / `endnotes` /
`comments` / `glossary_document` fields) stays `None` on a document built this way, exactly as it
would on a hand-crafted `.docx` that never related one — nothing about `Document::blank` narrows what
those fields can hold once you relate a part of your own to the document.
