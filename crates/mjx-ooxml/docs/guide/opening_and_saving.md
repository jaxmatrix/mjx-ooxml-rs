# Opening and saving

The whole lifecycle, once: what a package *is*, which of the three surfaces opens it, what saving
checks, and what refusing looks like.

## Detection reads the package, not the filename

[`detect_format`] opens the container, follows the root `officeDocument` relationship and reads the
**content type** of the part it lands on. So a `.pptm` and a `.potx` are recognised as
presentations, a `.docx` renamed to `.pptx` is recognised as a Word document, and a ZIP that is not
an OPC package at all is refused by name rather than by a parse failure three layers down.

**The three differ in shape here.** A [`Format`]'s accessors are methods in Rust, plain attributes
in Python, and **free functions** in JavaScript — `formatFamily(format)` rather than
`format.family()` — because a `#[wasm_bindgen]` enumeration is a number on that side and a number
cannot carry a getter. Nothing about the value differs; only how each language spells reaching into
it. See [Where the three languages differ in shape](crate::guide#where-the-three-languages-differ-in-shape).

<!-- guide-example: detecting_a_format rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let bytes = mjx_fixtures::fixture("sample.docx");
use mjx_ooxml::{detect_format, Format, FormatFamily};

// `bytes` is a Word document. Nothing here looks at a filename: detection opens the container,
// follows the root `officeDocument` relationship and reads the content type it lands on.
let format = detect_format(&bytes)?;
assert_eq!(format, Format::Document);
assert_eq!(format.family(), FormatFamily::WordProcessing);
assert_eq!(format.conventional_extension(), "docx");
assert!(format.is_editable());
assert!(!format.is_macro_enabled());
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: detecting_a_format python -->
```python
from mjx_ooxml import Format, FormatFamily, detect_format

# `data` is a Word document. Nothing here looks at a filename: detection opens the container,
# follows the root `officeDocument` relationship and reads the content type it lands on.
format = detect_format(data)
assert format == Format.Document
assert format.family == FormatFamily.WordProcessing
assert format.conventional_extension == "docx"
assert format.is_editable
assert not format.is_macro_enabled
```
<!-- guide-example end -->

<!-- guide-example: detecting_a_format js -->
```js
import {
  Format,
  FormatFamily,
  detectFormat,
  formatConventionalExtension,
  formatFamily,
  formatIsEditable,
  formatIsMacroEnabled,
} from "@mjx/ooxml";

// `data` is a Word document. Nothing here looks at a filename: detection opens the container,
// follows the root `officeDocument` relationship and reads the content type it lands on.
const format = detectFormat(data);
if (format !== Format.Document) {
  throw new Error("these bytes are a Word document");
}
if (formatFamily(format) !== FormatFamily.WordProcessing) {
  throw new Error("and its family is WordProcessing");
}
if (formatConventionalExtension(format) !== "docx") {
  throw new Error("whose conventional extension is docx");
}
if (!formatIsEditable(format) || formatIsMacroEnabled(format)) {
  throw new Error("this build can edit it, and it carries no macros");
}
```
<!-- guide-example end -->

[`Format`] has **fifteen** members across three [`FormatFamily`] values — six PowerPoint spellings,
four Word, five Excel — and `crates/mjx-ooxml/tests/format_detection.rs` is what holds the table to
real packages rather than to this sentence. Fourteen of the fifteen open. The one that does not is
[`Format::WorkbookBinary`] (`.xlsb`): a conforming OPC package whose main part is the MS-XLSB binary
record stream and **not SpreadsheetML at all**, so there is no markup in it for this library to read.
It is refused by design and not by schedule, with a message that says so in different words from the
one a `.pptx` handed to [`Workbook::open`] gets, because it is a different fact.

## Opening detects first, then parses once

You do not have to call [`detect_format`] yourself. Each of [`Deck::open`], [`Document::open`] and
[`Workbook::open`] detects before it parses, so handing a Word document to [`Deck::open`] gets
[`ErrorCode::UnsupportedFormat`] naming the format it actually is — and pointing at the constructor
that would have worked — rather than a `MalformedDocument` about a `presentation.xml` that was never
there. The package is read exactly once either way.

**The three differ in shape here.** The eleven [`ErrorCode`] values are one enumeration in Rust,
**eleven exception classes** in Python — so `except` can select one — and **eleven strings** on a
JavaScript `Error`'s `code` property, because `catch` selects on nothing there. All three branch on
the same classification; only catching is spelled differently. See
[Where the three languages differ in shape](crate::guide#where-the-three-languages-differ-in-shape),
and [Errors](errors) for the whole of both projections.

<!-- guide-example: opening_the_wrong_surface rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let workbook_bytes = mjx_fixtures::fixture("sample.xlsx");
use mjx_ooxml::{Deck, ErrorCode};

// `workbook_bytes` is a spreadsheet, and `Deck::open` detects that before it parses anything.
let failure = Deck::open(&workbook_bytes).expect_err("a workbook is not a deck");
assert_eq!(failure.code(), ErrorCode::UnsupportedFormat);

// The message names the constructor that would have worked, rather than complaining about a
// `presentation.xml` that was never there.
assert!(failure.message().contains("Workbook"));
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: opening_the_wrong_surface python -->
```python
from mjx_ooxml import Deck, UnsupportedFormatError

# `workbook_bytes` is a spreadsheet, and `Deck.open` detects that before it parses anything.
try:
    Deck.open(workbook_bytes)
    raise AssertionError("a workbook is not a deck")
except UnsupportedFormatError as failure:
    assert failure.code == "UnsupportedFormat"

    # The message names the constructor that would have worked, rather than complaining about a
    # `presentation.xml` that was never there.
    assert "Workbook" in str(failure), failure
```
<!-- guide-example end -->

<!-- guide-example: opening_the_wrong_surface js -->
```js
import { Deck } from "@mjx/ooxml";

// `workbookBytes` is a spreadsheet, and `Deck.open` detects that before it parses anything.
let failure;
try {
  Deck.open(workbookBytes);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "UnsupportedFormat") {
  throw new Error("a workbook is not a deck");
}

// The message names the constructor that would have worked, rather than complaining about a
// `presentation.xml` that was never there.
if (!failure.message.includes("Workbook")) {
  throw new Error(failure.message);
}
```
<!-- guide-example end -->

## Authoring from nothing

[`Deck::blank`], [`Document::blank`] and [`Workbook::blank`] build a package part by part from this
library's own element builders. **No template is embedded and nothing is read from disk**, which is
what makes a document buildable in a browser, or from a `pip install`, with no input file.

<!-- guide-example: authoring_from_nothing rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Deck, Document, PageSize, SlideSize, Workbook};

// One master, one layout, a theme — and no slides yet.
let deck = Deck::blank(SlideSize::widescreen())?;
assert_eq!(deck.slide_count(), 0);
assert_eq!(deck.master_count(), 1);

// One empty paragraph, because a `w:body` needs one.
let mut document = Document::blank(PageSize::a4())?;
assert_eq!(document.paragraph_count()?, 1);

// One empty worksheet, named Sheet1.
let workbook = Workbook::blank()?;
assert_eq!(workbook.sheet_count(), 1);
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: authoring_from_nothing python -->
```python
from mjx_ooxml import Deck, Document, PageSize, SlideSize, Workbook

# One master, one layout, a theme — and no slides yet.
deck = Deck.blank(SlideSize.widescreen())
assert deck.slide_count() == 0
assert deck.master_count() == 1

# One empty paragraph, because a `w:body` needs one.
document = Document.blank(PageSize.a4())
assert document.paragraph_count() == 1

# One empty worksheet, named Sheet1.
workbook = Workbook.blank()
assert workbook.sheet_count() == 1
```
<!-- guide-example end -->

<!-- guide-example: authoring_from_nothing js -->
```js
import { Deck, Document, PageSize, SlideSize, Workbook } from "@mjx/ooxml";

// One master, one layout, a theme — and no slides yet.
const deck = Deck.blank(SlideSize.widescreen());
if (deck.slideCount() !== 0 || deck.masterCount() !== 1) {
  throw new Error("a blank deck is one master, one layout and no slides");
}

// One empty paragraph, because a `w:body` needs one.
const document = Document.blank(PageSize.a4());
if (document.paragraphCount() !== 1) {
  throw new Error("a blank document has one empty paragraph");
}

// One empty worksheet, named Sheet1.
const workbook = Workbook.blank();
if (workbook.sheetCount() !== 1) {
  throw new Error("a blank workbook has one worksheet");
}

deck.free(); // a wasm handle owns memory the garbage collector cannot see
document.free();
workbook.free();
```
<!-- guide-example end -->

This example saves nothing, so there is no package for the two binding harnesses to compare —
what it demonstrates is precisely that none of the three needed an input file. The three halves
agreeing about producing nothing is itself checked, so a half that quietly started saving would not
pass unnoticed.

Each writes a theme part. That was not always true, and the reason it is now is worth knowing before
you author a chart: a chart series states no explicit fill, so its colour comes from the theme's
`accent1…accent6`, and a Word document or a workbook with no theme rendered a chart with a title,
axes, labels, a legend — and **no bars**. See [Fidelity and the known gaps](fidelity_and_gaps).

**What none of the three can do is set the document properties.** `mjx_pptx::Presentation`,
`mjx_docx::Document` and `mjx_xlsx::Workbook` each carry a `blank_with_properties` taking a
`mjx_opc::doc_props::CoreProperties` and an `ExtendedProperties`, so an authored file can name a
title, a creator and a created time. Nothing on this facade projects it, and neither binding can
reach it — the one gap on [The curated surface](the_curated_surface) that is a gap rather than a
decision. `blank` writes both `docProps` parts with this library's own defaults, and a file you
opened keeps the ones it came with, untouched.

## Saving validates

[`Deck::save`], [`Document::save`] and [`Workbook::save`] each run their own [`validate`](Deck::validate)
first and **refuse** to write a package that breaks a packaging invariant or a format one — a
relationship pointing at a part that is not there, a content-type override for a part that does not
exist, a `sheetId` two tabs share. The refusal is [`ErrorCode::InvalidDocument`], and
[`Error::detail`] names where.

The same three calls in each of the three languages the API ships in. **None of these blocks was
typed here.** Each is a copy of a sentinel-delimited region of a file a test runner executes —
`crates/mjx-ooxml/examples/guide_saving_validates.rs` under `cargo run`,
`bindings/mjx-python/tests/guide_examples/saving_validates.py` under `pytest`, and
`bindings/mjx-wasm/tests/node/guide_examples/saving_validates.mjs` under `node --test` — copied in
by `cargo run -p xtask -- guide-examples` and held to its source by `xtask/tests/guide_examples.rs`.
The three runs are compared to each other part by part, so a block that is out of date, or a
language that has quietly stopped agreeing with the other two, is a test failure rather than a
paragraph somebody has to notice.

<!-- guide-example: saving_validates rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{detect_format, Deck, Format, SlideSize};

let deck = Deck::blank(SlideSize::widescreen())?;
deck.validate()?; // the same check `save` runs
let saved = deck.save()?;
assert_eq!(detect_format(&saved)?, Format::Presentation);
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: saving_validates python -->
```python
from mjx_ooxml import Deck, Format, SlideSize, detect_format

deck = Deck.blank(SlideSize.widescreen())
deck.validate()  # the same check `save` runs
saved = deck.save()
assert detect_format(saved) == Format.Presentation
```
<!-- guide-example end -->

<!-- guide-example: saving_validates js -->
```js
import { Deck, Format, SlideSize, detectFormat } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
deck.validate(); // the same check `save` runs
const saved = deck.save();
if (detectFormat(saved) !== Format.Presentation) {
  throw new Error("the saved package is not a presentation");
}
deck.free(); // a wasm handle owns memory the garbage collector cannot see
```
<!-- guide-example end -->

The JavaScript block is one line longer than the other two, and the extra line is not decoration: a
wasm handle owns memory on the WebAssembly heap that the JavaScript garbage collector cannot see, so
a caller frees it. That is the one shape difference between the three surfaces here, and it is
visible precisely because these blocks are copies of files rather than translations of each other.

[`Deck::save_unchecked`], [`Document::save_unchecked`] and [`Workbook::save_unchecked`] are the
deliberate escape hatch, and there is exactly one situation they are for: **writing back a container
that arrived broken.** A file that failed `validate` on the way in will fail it on the way out, and
refusing to hand a caller their own bytes back would make this library the thing that lost them.

## The round trip

Opening a file and saving it without touching anything gives back **every part's decompressed
payload byte for byte**, plus a structurally identical container. Not identical ZIP bytes: the
compression level and the entry order are the container's business, not the document's.

The three blocks below are copies of three files a test runner executes, exactly as *Saving
validates* above is — and this is the example where that arrangement earns its keep. **Nothing here
is authored.** Every part of the package these three save was written by whoever produced
`tests/fixtures/sample.xlsx`, so what each block asserts is preservation itself: the same part names,
and byte-identical payloads for every one of them. `original` is that file's bytes, read by each
runner before the block starts, because reading a file is the caller's job in all three languages —
this library is bytes in and bytes out and never touches a filesystem.

<!-- guide-example: the_round_trip rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let original = mjx_fixtures::fixture("sample.xlsx");
use mjx_ooxml::Workbook;

// `original` is the file's bytes. Reading them is the caller's job in every one of the three
// languages: this library is bytes in and bytes out and never touches a filesystem.
let saved = Workbook::open(&original)?.save()?;

// Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
// `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
let before = Workbook::open(&original)?;
let after = Workbook::open(&saved)?;
assert_eq!(before.part_names(), after.part_names());
for part in before.part_names() {
    let was = before.part_bytes(&part)?;
    let now = after.part_bytes(&part)?;
    assert_eq!(was, now, "{part} changed");
}
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: the_round_trip python -->
```python
from mjx_ooxml import Workbook

# `original` is the file's bytes. Reading them is the caller's job in every one of the three
# languages: this library is bytes in and bytes out and never touches a filesystem.
saved = Workbook.open(original).save()

# Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
# `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
before = Workbook.open(original)
after = Workbook.open(saved)
assert before.part_names() == after.part_names()
for part in before.part_names():
    was = before.part_bytes(part)
    now = after.part_bytes(part)
    assert was == now, f"{part} changed"
```
<!-- guide-example end -->

<!-- guide-example: the_round_trip js -->
```js
import { Workbook } from "@mjx/ooxml";

// `original` is the file's bytes. Reading them is the caller's job in every one of the three
// languages: this library is bytes in and bytes out and never touches a filesystem.
const opened = Workbook.open(original);
const saved = opened.save();
opened.free();

// Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
// `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
const before = Workbook.open(original);
const after = Workbook.open(saved);
const names = before.partNames();
if (names.join("\n") !== after.partNames().join("\n")) {
  throw new Error("a part appeared or vanished");
}
for (const part of names) {
  const was = before.partBytes(part);
  const now = after.partBytes(part);
  if (was.length !== now.length || was.some((byte, at) => byte !== now[at])) {
    throw new Error(`${part} changed`);
  }
}
before.free(); // a wasm handle owns memory the garbage collector cannot see
after.free();
```
<!-- guide-example end -->

The JavaScript block is the longest, and both reasons are the language rather than this library: a
wasm handle owns memory the garbage collector cannot see, so each of the three is freed by hand, and
JavaScript has no structural equality, so two byte arrays are compared element by element where Rust
and Python compare them with `==`. Every call in front of those differences is the same call with the
same arguments.

The two binding harnesses then compare all three saved packages against each other part by part,
which is a **second** fact and not a restatement of the first: each block on its own proves that its
language preserved the fixture, and the comparison proves the three preserved it *the same way*.

That contract is not this crate's doing — it is `mjx_opc`'s copy-on-write part graph and
`mjx_xml::fidelity`'s byte-preserving reader, inherited whole. What *is* this crate's doing is that
it adds no re-serialisation of its own, and
`crates/mjx-ooxml/tests/preservation/main.rs` is the gate that proves it: every committed fixture,
crossed with every mutating method of all three surfaces.
