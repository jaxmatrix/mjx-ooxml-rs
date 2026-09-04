# Guide

One page so far, growing with the crate.

| Page | Read it when |
|---|---|
| [Building a document](building_a_document) | You want the whole story once: start blank or from a file, add paragraphs and runs, save |

## The shape of the API, in one page

Two facts explain most of it, and both are deliberate mirrors of `mjx-pptx`'s own guide — a reviewer
who knows one knows the shape of the other.

**Bytes in, bytes out.** [`Document::open`] takes `&[u8]` and [`save`](Document::save) returns
`Vec<u8>`; [`Document::blank`] takes no bytes at all and builds a document from nothing. The library
never touches a filesystem, a network, or a clock. Whoever calls it owns the file handle.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("in.docx")?;
let mut document = mjx_docx::Document::open(&bytes)?;
// … edits …
std::fs::write("out.docx", document.save()?)?;
# Ok(())
# }
```

**Everything is addressed, nothing is handed out.** There is no `Paragraph` object and no `Run`
object to hold onto between calls — [`Paragraph`]/[`Run`] are values `Document`'s own accessors read
into and hand back, not live handles. You name what you want with a [`BlockPath`]/[`RunPath`] (a bare
`usize` addresses a top-level paragraph or run) and the document answers. That is what keeps a
`.docx`'s copy-on-write fidelity intact: the library knows exactly which part you touched, so every
part you did not touch is re-emitted byte-for-byte — see
`crates/mjx-docx/src/document/mod.rs`'s own `editing_one_run_retains_the_untouched_sibling_paragraphs_source_span`
test for that guarantee proved on bytes, not just asserted.

## Units

Page geometry ([`PageSize`]) is in **twips**: 1440 to the inch, the unit `w:pgSz`/`w:pgMar` are
wire-typed in (`s:ST_TwipsMeasure`). [`PageSize::a4`] and [`PageSize::us_letter`] are the two named
defaults; [`PageSize::landscape`] swaps width and height rather than leaving them at their portrait
values, matching how Word itself writes a rotated page.
