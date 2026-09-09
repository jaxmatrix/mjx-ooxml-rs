# Guide

Six pages plus this index, in reading order — the same shape `mjx_pptx::guide`, `mjx_docx::guide` and
`mjx_xlsx::guide` carry, and deliberately not a fourth copy of any of them. **Those three describe
one format each. This one describes the surface all three are reached through**, and it is written
to answer the questions that only have an answer here: which of the three a package *is*, how each
of them is addressed, which vocabulary is shared across all three, what an error means whichever one
raised it, and what this crate deliberately leaves to the crate below it.

| Page | Read it when |
|---|---|
| [Opening and saving](opening_and_saving) | You have bytes and you do not yet know what they are |
| [Addressing](addressing) | You know which surface you want and not yet how to name a thing on it |
| [One vocabulary, three surfaces](one_vocabulary_three_surfaces) | You learned something on one format and want it on another |
| [Errors](errors) | A call failed and you want to know what to do about it |
| [The curated surface](the_curated_surface) | A method exists one layer down and not here |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on any of this in production |

Per-format detail lives with the format. Start here, then go to
[the PowerPoint guide](mjx_pptx::guide), [the Word guide](mjx_docx::guide) or
[the Excel guide](mjx_xlsx::guide) — each is written against its own crate's type, and
[One vocabulary, three surfaces](one_vocabulary_three_surfaces) is the table that translates a call
in either direction. The three runnable walkthroughs written *through this crate*, naming no lower
crate at all, are `crates/mjx-ooxml/examples/build_a_deck.rs`,
`crates/mjx-ooxml/examples/build_a_document.rs` and
`crates/mjx-ooxml/examples/build_a_workbook.rs`; CI runs every one on every push, and each exists a
second time in Python and a third in TypeScript, compared against the Rust one part by part, byte
for byte — and `xtask/tests/walkthrough_triples.rs` derives that set from the examples directory, so
a fourth walkthrough cannot arrive with no comparison the way the Word one did (MJXOFF-239).

```sh
cargo run -p mjx-ooxml --example build_a_deck
cargo run -p mjx-ooxml --example build_a_document
cargo run -p mjx-ooxml --example build_a_workbook
```

Every snippet on every page here is a compiled doctest that `cargo test` runs, and every one asserts
on a value it computed — the same rule the other three guides are held to, and what keeps a page
from drifting away from the API it describes.

## Where the three languages differ in shape

Almost every code example on these pages is shown three times — Rust, Python, JavaScript — and none
of the three is a transcription: each is a copy of a file a test runner executes, so they cannot
drift apart. `xtask/tests/guide_examples.rs` is what makes that a test failure rather than a habit.

What they can still do is **differ**, because a few things this API states in a Rust type have no
Rust type to be stated in on the other side. Wherever one of those applies, the blocks are preceded
by a line beginning **The three differ in shape here**, saying which and why. That line always comes
*before* the blocks, never after: a reader who meets three structurally different blocks with no
explanation has already concluded that one of them is a typo, and no note underneath will undo it.

| What differs | Rust | Python | JavaScript |
|---|---|---|---|
| A [`Format`] accessor | `format.family()`, a method | `format.family`, an attribute | `formatFamily(format)`, a free function — a wasm enumeration is a number in JavaScript and cannot carry a getter |
| An [`ErrorCode`] | `failure.code() == ErrorCode::IndexOutOfRange` | one exception class per code, and `failure.code` as its string spelling | `failure.code`, a string on a real `Error` |
| An [`ErrorDetail`] | `failure.detail().index` | `failure.index`, an attribute on the exception itself | `failure.detail.index`, a key present only when the failure had that coordinate |
| A [`CellInput`] | `CellWrite::new("A1", CellInput::Number(1.0))` | `CellWrite.number("A1", 1.0)` | `CellWrite.number("A1", 1.0)` |

One more difference exists that no example here reaches: a Rust range argument becomes two numbers
in JavaScript, because `wasm-bindgen` has no range — see `bindings/mjx-wasm/src/tables.rs`.

And a **handful of blocks are shown once, in Rust alone**, because the behaviour they describe is
Rust-only by decision: the three escape hatches on [The curated surface](the_curated_surface), and
the typed cause on [Errors](errors). Each says so in the prose beside it, and each carries a marker
naming the symbols that make the claim true — which the same gate checks against both binding
surfaces on every run, so a claim that stops being true is a test failure rather than a stale
sentence.

## The shape of the API, in one page

Four facts explain most of it.

**Bytes in, bytes out, and nothing else.** [`Deck::open`], [`Document::open`] and [`Workbook::open`]
each take `&[u8]`; [`Deck::save`], [`Document::save`] and [`Workbook::save`] each return `Vec<u8>`.
Nothing in this workspace touches a filesystem, a network, a clock, a thread or a random number
generator — which is why the same calls run unchanged in a browser through
`bindings/mjx-wasm`, and why the caller always owns the file handle.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{detect_format, Deck, Document, FormatFamily, Workbook};

let bytes = std::fs::read("in.pptx")?;
match detect_format(&bytes)?.family() {
    FormatFamily::Presentation => { let deck = Deck::open(&bytes)?; std::fs::write("out.pptx", deck.save()?)?; }
    FormatFamily::WordProcessing => { let doc = Document::open(&bytes)?; std::fs::write("out.docx", doc.save()?)?; }
    FormatFamily::Spreadsheet => { let wb = Workbook::open(&bytes)?; std::fs::write("out.xlsx", wb.save()?)?; }
    // `FormatFamily` is `#[non_exhaustive]`: a fourth family is an added arm, not a broken build.
    _ => unreachable!("three families today"),
}
# Ok(())
# }
```

**Everything is addressed, nothing is handed out.** There is no `Slide` object, no `Paragraph`
object and no `Sheet` object to hold. You name what you want on each call — a surface and a shape, a
block and a run, a tab and an A1 range — and the document answers. That is what keeps the
copy-on-write fidelity intact: the library knows exactly which part you touched, so every part you
did not touch is re-emitted byte for byte. It is also what makes the surface projectable: nothing
here hands back a view into the document and nothing here takes a callback, so a binding can never
have two live borrows. See [Addressing](addressing).

**Reads take `&mut self`.** Not because reading changes the document — it does not — but because a
part is raw bytes until something needs it parsed. The first read of a slide, a header or a
worksheet materialises its tree; what you save is identical to what you opened. Share a document
between threads by moving it, not by aliasing it.

**One error type, eleven codes.** Every call on all three surfaces returns
[`Result<_, Error>`](crate::Error). [`Error::code`] is a stable [`ErrorCode`] a caller can branch on
in any of the three languages, [`Error::detail`] says *where*, and the typed cause — a
[`mjx_pptx::PptxError`], a `mjx_docx::DocxError`, a `mjx_xlsx::XlsxError` — is still reachable by
downcasting [`source`](std::error::Error::source). See [Errors](errors).

**The three differ in shape here**, in both of the ways the table above names at once: the code is
an enumeration, a class and a string; and the five [`ErrorDetail`] coordinates are a value behind
`detail()`, five attributes on the exception itself, and the keys a plain `detail` object actually
carries.

<!-- guide-example: an_index_out_of_range rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Deck, ErrorCode, SlideSize};

let mut deck = Deck::blank(SlideSize::widescreen())?;

// A blank deck has no slides at all, so slide 7 is past the end.
let failure = deck.shape_count(7.into()).expect_err("no slide 7");
assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);
assert_eq!(failure.detail().index, Some(7));
assert_eq!(failure.message(), "slide index 7 out of range (0..0)");
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: an_index_out_of_range python -->
```python
from mjx_ooxml import Deck, IndexOutOfRangeError, SlideSize

deck = Deck.blank(SlideSize.widescreen())

# A blank deck has no slides at all, so slide 7 is past the end.
try:
    deck.shape_count(7)
    raise AssertionError("no slide 7")
except IndexOutOfRangeError as failure:
    assert failure.code == "IndexOutOfRange"
    assert failure.index == 7
    assert str(failure) == "slide index 7 out of range (0..0)"
```
<!-- guide-example end -->

<!-- guide-example: an_index_out_of_range js -->
```js
import { Deck, SlideSize } from "@mjx/ooxml";

const size = SlideSize.widescreen();
const deck = Deck.blank(size);

// A blank deck has no slides at all, so slide 7 is past the end.
let failure;
try {
  deck.shapeCount(7);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "IndexOutOfRange" || failure.detail.index !== 7) {
  throw new Error("slide 7 is out of range, and the failure says which index");
}
if (failure.message !== "slide index 7 out of range (0..0)") {
  throw new Error(failure.message);
}

// a wasm handle owns memory the garbage collector cannot see
size.free();
deck.free();
```
<!-- guide-example end -->

## Nothing downstream names a crate below this one

`crate::Deck`, `crate::Document` and `crate::Workbook` are the whole surface, and every type needed
to *state* an argument to any of them is re-exported here — [`FillSpec`], [`ColorSpec`],
[`ShapeBounds`], [`CharacterPropertiesSpec`], [`ChartData`], [`CellInput`], [`PageSize`],
[`PresetShapeType`] and some two hundred more. So an application depends on `mjx-ooxml` and on
nothing else in this workspace: not `mjx-dml` for a fill, not `mjx-chart` for a chart description,
not `mjx-ooxml-types` for a preset shape, not `mjx-pptx` for anything at all.

That claim is checked twice. `crates/mjx-ooxml/tests/public_paths.rs` drives one deck through every
subject of the `Deck` surface importing only `mjx_ooxml::…`, and
`crates/mjx-ooxml/tests/vocabulary_closure.rs` destructures and rebuilds the values whose *fields*
name further types — the ten a binding found reachable from the vocabulary but absent from it. If a
caller had to reach past this crate to state an argument, neither file would compile.
