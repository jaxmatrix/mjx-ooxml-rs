# Opening and saving

The whole lifecycle, once: what a package *is*, which of the three surfaces opens it, what saving
checks, and what refusing looks like.

## Detection reads the package, not the filename

[`detect_format`] opens the container, follows the root `officeDocument` relationship and reads the
**content type** of the part it lands on. So a `.pptm` and a `.potx` are recognised as
presentations, a `.docx` renamed to `.pptx` is recognised as a Word document, and a ZIP that is not
an OPC package at all is refused by name rather than by a parse failure three layers down.

```
use mjx_ooxml::{detect_format, Format, FormatFamily};

# fn main() -> Result<(), mjx_ooxml::Error> {
let bytes = mjx_fixtures::fixture("sample.docx");
let format = detect_format(&bytes)?;
assert_eq!(format, Format::Document);
assert_eq!(format.family(), FormatFamily::WordProcessing);
assert_eq!(format.conventional_extension(), "docx");
assert!(format.is_editable() && !format.is_macro_enabled());
# Ok(())
# }
```

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

```
use mjx_ooxml::{Deck, ErrorCode};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let workbook_bytes = mjx_fixtures::fixture("sample.xlsx");
let failure = Deck::open(&workbook_bytes).expect_err("a workbook is not a deck");
assert_eq!(failure.code(), ErrorCode::UnsupportedFormat);
assert!(failure.message().contains("Workbook"), "{}", failure.message());
# Ok(())
# }
```

## Authoring from nothing

[`Deck::blank`], [`Document::blank`] and [`Workbook::blank`] build a package part by part from this
library's own element builders. **No template is embedded and nothing is read from disk**, which is
what makes a document buildable in a browser, or from a `pip install`, with no input file.

```
use mjx_ooxml::{Deck, Document, PageSize, SlideSize, Workbook};

# fn main() -> Result<(), mjx_ooxml::Error> {
let deck = Deck::blank(SlideSize::widescreen())?;
assert_eq!(deck.slide_count(), 0, "one master, one layout, a theme — and no slides yet");
assert_eq!(deck.master_count(), 1);

let mut document = Document::blank(PageSize::a4())?;
assert_eq!(document.paragraph_count()?, 1, "one empty paragraph, because a `w:body` needs one");

let workbook = Workbook::blank()?;
assert_eq!(workbook.sheet_count(), 1, "one empty worksheet named Sheet1");
# Ok(())
# }
```

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

```
use mjx_ooxml::{Deck, SlideSize};

# fn main() -> Result<(), mjx_ooxml::Error> {
let deck = Deck::blank(SlideSize::widescreen())?;
deck.validate()?;                      // the same check `save` runs
let bytes = deck.save()?;
assert_eq!(mjx_ooxml::detect_format(&bytes)?, mjx_ooxml::Format::Presentation);
# Ok(())
# }
```

[`Deck::save_unchecked`], [`Document::save_unchecked`] and [`Workbook::save_unchecked`] are the
deliberate escape hatch, and there is exactly one situation they are for: **writing back a container
that arrived broken.** A file that failed `validate` on the way in will fail it on the way out, and
refusing to hand a caller their own bytes back would make this library the thing that lost them.

## The round trip

Opening a file and saving it without touching anything gives back **every part's decompressed
payload byte for byte**, plus a structurally identical container. Not identical ZIP bytes: the
compression level and the entry order are the container's business, not the document's.

```
# fn main() -> Result<(), mjx_ooxml::Error> {
use mjx_ooxml::Workbook;

let original = mjx_fixtures::fixture("sample.xlsx");
let saved = Workbook::open(&original)?.save()?;

let before = Workbook::open(&original)?;
let after = Workbook::open(&saved)?;
assert_eq!(before.part_names(), after.part_names());
for part in before.part_names() {
    assert_eq!(before.part_bytes(&part)?, after.part_bytes(&part)?, "{part} changed");
}
# Ok(())
# }
```

That contract is not this crate's doing — it is `mjx_opc`'s copy-on-write part graph and
`mjx_xml::fidelity`'s byte-preserving reader, inherited whole. What *is* this crate's doing is that
it adds no re-serialisation of its own, and
`crates/mjx-ooxml/tests/preservation/main.rs` is the gate that proves it: every committed fixture,
crossed with every mutating method of all three surfaces.
