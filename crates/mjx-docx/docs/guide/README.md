# Guide

Five pages, in reading order. Each is written to be read start to finish once, then returned to as a
reference.

| Page | Read it when |
|---|---|
| [Building a document](building_a_document) | You want the whole story once: start blank or from a file, add paragraphs and runs, save |
| [Text, runs and annotations](text_and_formatting) | You need to address a particular run, edit text precisely, or attach a comment, a note or a bookmark |
| [Tables, sections, headers and structured content](tables_sections_and_headers) | You are placing structured content, or a property is not where a section put it |
| [Styles, numbering and inheritance](styles_and_inheritance) | A run renders in a way nothing in the paragraph explains |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on something in production |

The [effective-properties guide](crate::effective_properties) is the deep reference behind page 4 —
what a file *states* versus what Word *renders*, with the ECMA-376 clauses each rung comes from.

Every snippet on every page is a compiled doctest that `cargo test` runs, and every one asserts on a
value it computed. That is what keeps this guide from drifting away from the API: a rename in a later
change breaks `cargo test` until the guide is fixed.

## The shape of the API, in one page

Three facts explain most of it, and all three are deliberate mirrors of `mjx-pptx`'s own guide — a
reviewer who knows one knows the shape of the other.

**Bytes in, bytes out.** [`Document::open`] takes `&[u8]` and [`save`](Document::save) returns
`Vec<u8>`; [`Document::blank`] takes no bytes at all and builds a document from nothing. The library
never touches a filesystem, a network, or a clock. Whoever calls it owns the file handle — which is
also why the same code compiles to WebAssembly and runs in a browser unchanged.

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

**Reads take `&mut self`.** Not because reading changes the document — it does not — but because a
part is raw bytes until something needs it parsed. The first read of `word/styles.xml` materialises
its tree; the document you save is identical to the document you opened, and
`examples/read_document.rs` checks exactly that, part by part.

## Units

Page geometry ([`PageSize`], [`PageMargins`]) is in **twips**: 1440 to the inch, the unit
`w:pgSz`/`w:pgMar` are wire-typed in (`s:ST_TwipsMeasure`). [`PageSize::a4`] and
[`PageSize::us_letter`] are the two named defaults; [`PageSize::landscape`] swaps width and height
rather than leaving them at their portrait values, matching how Word itself writes a rotated page.

Font sizes are in **half-points** (`w:sz`, `s:ST_HpsMeasure`), so 24 is twelve point. Drawing
extents are in **EMU** (English Metric Units), 914 400 to the inch, because that side of the file is
DrawingML rather than WordprocessingML — the two units meet at [`add_inline_picture`](Document::add_inline_picture).

## The examples

Nine, one per shape of work, each writing a file and then reopening it to check what it wrote:

```sh
cargo run -p mjx-docx --example blank_document        # a document from nothing
cargo run -p mjx-docx --example read_document         # read a file exhaustively; prove the round trip
cargo run -p mjx-docx --example edit_text             # edit runs, and only what was addressed
cargo run -p mjx-docx --example build_table           # rows, columns and both kinds of merge
cargo run -p mjx-docx --example styles_and_numbering  # author styles.xml and numbering.xml
cargo run -p mjx-docx --example sections_and_headers  # sections, page geometry, header inheritance
cargo run -p mjx-docx --example fields_and_hyperlinks # nested fields, links, bookmarks, form fields
cargo run -p mjx-docx --example annotations           # comments, footnotes, endnotes
cargo run -p mjx-docx --example structured_content    # content controls, custom XML, altChunk, a picture
```

Each takes an optional output path and otherwise writes under `target/examples/`. An example that
never checks its own output is a claim, not a demonstration, so every one of them reopens the bytes
it wrote and asserts on what came back.
