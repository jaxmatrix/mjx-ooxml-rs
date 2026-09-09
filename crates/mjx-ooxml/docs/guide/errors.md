# Errors

One error type, on all three surfaces, in all three languages. [`Error`] carries three things: a
stable [`ErrorCode`] a caller branches on, a human message, and an [`ErrorDetail`] saying *where*.
The typed cause is still under it.

## Eleven codes, and what each one means you should do

The three crates below this one raise `mjx_pptx::PptxError`, `mjx_docx::DocxError` and
`mjx_xlsx::XlsxError`, and between them they have well over a hundred variants. **A binding cannot
project a hundred exception classes**, and a caller does not want them: what a caller wants is the
handful of decisions the failure implies. So every variant maps onto one of eleven codes, and the
mapping is by *what the caller should do*, not by which crate raised it.

| Code | What it means | What to do |
|---|---|---|
| [`ErrorCode::Io`] | the bytes are not a readable container | it is not an OPC package; nothing here can help |
| [`ErrorCode::MalformedDocument`] | the bytes are a package, but its markup is not what the schema requires | the file is broken on the way in |
| [`ErrorCode::InvalidDocument`] | a packaging or format invariant is broken | `save` refused; `save_unchecked` is the escape hatch |
| [`ErrorCode::IndexOutOfRange`] | a slide, shape, row, sheet or run index is past the end | ask for the count first |
| [`ErrorCode::WrongKind`] | the thing at that address is not the kind you asked about | a picture is not a chart; check the kind |
| [`ErrorCode::NotFound`] | a named thing — a style, a bookmark, a defined name — is not there | it is a lookup miss, not a structural failure |
| [`ErrorCode::NothingToRead`] | the target exists and is of the right kind, but states nothing for this call | a chartsheet has no cells; a shape may have no text body |
| [`ErrorCode::InvalidArgument`] | the argument itself cannot be represented | a `NaN`, an unparsable A1 reference, a slide size out of range |
| [`ErrorCode::StructureConflict`] | the edit would break a structure that is currently valid | a merge crossing a merge, a shape that would contain itself |
| [`ErrorCode::UnsupportedContent`] | the document uses a construct this build does not model, or asks for one it cannot write | not a defect — see [Fidelity and the known gaps](fidelity_and_gaps) |
| [`ErrorCode::UnsupportedFormat`] | the package is a different format, or `.xlsb` | open it with the surface that matches |

[`ErrorCode::as_str`] gives each one a stable spelling, which is what the two bindings key their own
exception classes on.

**The three differ in shape here**, in two ways at once: an [`ErrorCode`] is an enumeration, eleven
exception classes and eleven strings; and a [`CellInput`] passed to [`CellWrite::new`] becomes a
static [`CellWrite`] constructor per kind in both bindings. Neither difference changes what is
being asked — three refusals, and a workbook still untouched. See
[Where the three languages differ in shape](crate::guide#where-the-three-languages-differ-in-shape).

<!-- guide-example: branching_on_an_error_code rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{CellInput, CellWrite, ErrorCode, Workbook};

let mut workbook = Workbook::blank()?;

// An address that does not parse: refused before the worksheet is even opened.
let bad_address = workbook
    .write_cells(0, &[CellWrite::new("not-a-cell", CellInput::Number(1.0))])
    .expect_err("`not-a-cell` is not an A1 reference");
assert_eq!(bad_address.code(), ErrorCode::InvalidArgument);

// A value SpreadsheetML has no spelling for.
let unrepresentable = workbook
    .write_cells(0, &[CellWrite::new("A1", CellInput::Number(f64::NAN))])
    .expect_err("SpreadsheetML cannot spell NaN");
assert_eq!(unrepresentable.code(), ErrorCode::InvalidArgument);

// A tab that is not there.
let missing = workbook.read_range(9, "A1").expect_err("one sheet");
assert_eq!(missing.code(), ErrorCode::IndexOutOfRange);
assert_eq!(missing.detail().index, Some(9));

// And the contract worth knowing: a batch that would fail halfway is refused before the
// package is touched at all, so nothing above wrote anything.
assert!(workbook.read_sheet(0)?.is_empty());
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: branching_on_an_error_code python -->
```python
from mjx_ooxml import CellWrite, IndexOutOfRangeError, InvalidArgumentError, Workbook

workbook = Workbook.blank()

# An address that does not parse: refused before the worksheet is even opened.
try:
    workbook.write_cells(0, [CellWrite.number("not-a-cell", 1.0)])
    raise AssertionError("`not-a-cell` is not an A1 reference")
except InvalidArgumentError as failure:
    assert failure.code == "InvalidArgument"

# A value SpreadsheetML has no spelling for.
try:
    workbook.write_cells(0, [CellWrite.number("A1", float("nan"))])
    raise AssertionError("SpreadsheetML cannot spell NaN")
except InvalidArgumentError as failure:
    assert failure.code == "InvalidArgument"

# A tab that is not there.
try:
    workbook.read_range(9, "A1")
    raise AssertionError("there is one sheet")
except IndexOutOfRangeError as failure:
    assert failure.index == 9

# And the contract worth knowing: a batch that would fail halfway is refused before the
# package is touched at all, so nothing above wrote anything.
assert workbook.read_sheet(0).is_empty
```
<!-- guide-example end -->

<!-- guide-example: branching_on_an_error_code js -->
```js
import { CellWrite, Workbook } from "@mjx/ooxml";

/** The code a call raised, or `undefined` if it did not raise. */
function codeOf(call) {
  try {
    call();
  } catch (raised) {
    return raised.code;
  }
  return undefined;
}

const workbook = Workbook.blank();

// An address that does not parse: refused before the worksheet is even opened.
const badAddress = codeOf(() => workbook.writeCells(0, [CellWrite.number("not-a-cell", 1.0)]));
if (badAddress !== "InvalidArgument") {
  throw new Error("`not-a-cell` is not an A1 reference");
}

// A value SpreadsheetML has no spelling for.
const unrepresentable = codeOf(() => workbook.writeCells(0, [CellWrite.number("A1", NaN)]));
if (unrepresentable !== "InvalidArgument") {
  throw new Error("SpreadsheetML cannot spell NaN");
}

// A tab that is not there.
if (codeOf(() => workbook.readRange(9, "A1")) !== "IndexOutOfRange") {
  throw new Error("there is one sheet");
}

// And the contract worth knowing: a batch that would fail halfway is refused before the
// package is touched at all, so nothing above wrote anything.
const sheet = workbook.readSheet(0);
if (!sheet.isEmpty) {
  throw new Error("three refusals wrote nothing");
}

// a wasm handle owns memory the garbage collector cannot see
sheet.free();
workbook.free();
```
<!-- guide-example end -->

That last assertion is the contract worth knowing: **a batch that would fail halfway is refused
before the package is touched at all.** Every reference in a [`Workbook::write_cells`] call is parsed
before the worksheet is opened, so a failed write leaves the workbook exactly as it was.

## `detail` says where

[`ErrorDetail`] has five optional fields — [`surface`](ErrorDetail::surface),
[`shape`](ErrorDetail::shape), [`row`](ErrorDetail::row), [`column`](ErrorDetail::column) and
[`index`](ErrorDetail::index) — and carries whichever the failure actually knew. That is what lets a
caller point at the thing that failed rather than re-deriving it from a message string, which is the
one thing a message must never be parsed for.

**The three differ in shape here.** The detail is a value behind `detail()` in Rust, **five
attributes on the exception itself** in Python — always present, `None` where the failure knew no
such coordinate — and a **plain `detail` object** in JavaScript carrying only the coordinates it
did have, so a caller there reads `detail.row ?? null` rather than expecting a key. See
[Where the three languages differ in shape](crate::guide#where-the-three-languages-differ-in-shape).

<!-- guide-example: an_error_says_where rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Deck, ErrorCode, ShapePath, SlideSize, Surface};

let mut deck = Deck::blank(SlideSize::widescreen())?;
let slide = Surface::Slide(deck.add_slide_from_layout(0)?);

// The slide carries the layout's placeholders and nothing at index 4.
let failure = deck.shape_bounds(slide, 4.into()).expect_err("no shape 4");
assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);

// The failure says *where*, in the same addressing the call used to get there.
assert_eq!(failure.detail().surface, Some(slide));
let shape = failure.detail().shape.as_ref().map(ShapePath::indices);
assert_eq!(shape, Some(&[4][..]));
assert!(!failure.detail().is_empty());
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: an_error_says_where python -->
```python
from mjx_ooxml import Deck, IndexOutOfRangeError, SlideSize, Surface

deck = Deck.blank(SlideSize.widescreen())
slide = Surface.slide(deck.add_slide_from_layout(0))

# The slide carries the layout's placeholders and nothing at index 4.
try:
    deck.shape_bounds(slide, 4)
    raise AssertionError("no shape 4")
except IndexOutOfRangeError as failure:
    assert failure.code == "IndexOutOfRange"

    # The failure says *where*, in the same addressing the call used to get there.
    assert failure.surface == slide
    assert failure.shape.indices == [4]
    assert failure.row is None and failure.column is None
```
<!-- guide-example end -->

<!-- guide-example: an_error_says_where js -->
```js
import { Deck, SlideSize, Surface } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
const slide = Surface.slide(deck.addSlideFromLayout(0));

// The slide carries the layout's placeholders and nothing at index 4.
let failure;
try {
  deck.shapeBounds(slide, 4);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "IndexOutOfRange") {
  throw new Error("no shape 4");
}

// The failure says *where*, in the same addressing the call used to get there.
if (failure.detail.surface.kind !== "slide" || failure.detail.surface.index !== slide.index) {
  throw new Error("the failure names the surface the call named");
}
if (failure.detail.shape.indices.join() !== "4") {
  throw new Error("and the shape address it was asked for");
}
if (failure.detail.row !== undefined || failure.detail.column !== undefined) {
  throw new Error("a coordinate the failure did not have is simply not a key");
}

// a wasm handle owns memory the garbage collector cannot see
failure.detail.surface.free();
failure.detail.shape.free();
slide.free();
deck.free();
```
<!-- guide-example end -->

## The typed cause is still there

Collapsing to eleven codes loses nothing for a Rust caller. [`Error`] implements
[`std::error::Error`], and its [`source`](std::error::Error::source) is the original
`mjx_pptx::PptxError`, `mjx_docx::DocxError` or `mjx_xlsx::XlsxError` — downcast it when you want the
variant rather than the code.

**There is one block below, not three, and that is this section's whole subject.** Every other
example in this guide is shown in Rust, Python and JavaScript. This one cannot be: `PptxError` is
declared by neither binding and neither language has `downcast_ref`, so the marker beside it
declares the block **Rust-only, naming those two** — and `xtask/tests/guide_examples.rs` reads both
binding surfaces on every run to check the claim is still true. The move a binding caller makes
instead is the one [§ Eleven codes](#eleven-codes-and-what-each-one-means-you-should-do) already
shows in all three languages: branch on the code. Repeating it here would make this section about
the thing a binding caller *can* do, which is the opposite of what it is for.

<!-- guide-example: downcasting_to_the_typed_cause rust-only PptxError downcast_ref -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Deck, ErrorCode, PptxError, SlideSize};
use std::error::Error as _;

let mut deck = Deck::blank(SlideSize::widescreen())?;

// A blank deck has no slides at all, so slide 7 is past the end.
let failure = deck.shape_count(7.into()).expect_err("no slide 7");
assert_eq!(failure.code(), ErrorCode::IndexOutOfRange);

// Collapsing to eleven codes loses nothing here: the variant the crate below raised is still
// underneath, reachable by downcasting the source.
let cause = failure.source().expect("a typed cause");
let pptx = cause.downcast_ref::<PptxError>().expect("a PptxError");
assert!(matches!(pptx, PptxError::SlideIndexOutOfRange { .. }));
# Ok(())
# }
```
<!-- guide-example end -->

Only [`mjx_pptx::PptxError`] is re-exported here by name, because it is the one a `Deck` caller is
most likely to want; a Word or Excel caller who wants the same reaches for `mjx_docx` or `mjx_xlsx`
directly, which is a Rust-only move either way — neither binding can carry a typed cause across the
boundary, and neither tries.

## In the two bindings

The two projections take opposite decisions from the same eleven codes, and each is right for its
language. **Python raises one exception class per code** — `IndexOutOfRangeError`,
`InvalidArgumentError` and nine more — all subclasses of `OoxmlError`, so `except OoxmlError` catches
everything and a narrower `except` catches one kind; `IndexOutOfRangeError` is also a Python
`IndexError`, so code already guarding a lookup keeps working when the lookup is a slide index. The
five [`ErrorDetail`] fields arrive as attributes, always present and `None` where the failure carried
no such coordinate.

**TypeScript throws a real `Error`** — `instanceof Error` is true and it has a `stack` — named
`OoxmlError`, with two own properties: `code`, the [`ErrorCode::as_str`] spelling, and `detail`, a
plain object carrying only the coordinates the failure had. `except` selects on the class in Python
and on nothing in JavaScript, which is the whole reason the two differ.

**[`ErrorCode`] is `#[non_exhaustive]`, and each binding absorbs a new code differently.**
`bindings/mjx-wasm/src/errors.rs` needs no arm at all — it writes [`ErrorCode::as_str`] straight into
the `code` property, so a twelfth code arrives in JavaScript the day it is added.
`bindings/mjx-python/src/errors.rs` maps each code to a class and carries a wildcard arm that raises
the root `OoxmlError`, deliberately: *a code this build does not know is still a failure, and
reporting it as the root class is strictly better than losing it.* So a twelfth code is never a
silent gap — but it is a Python class somebody has to remember to add, which is why it is written
down here.
