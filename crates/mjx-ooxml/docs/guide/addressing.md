# Addressing

Nothing on this surface is handed out. There is no `Slide`, no `Paragraph` and no `Sheet` to hold —
you name what you want on every call. **Each of the three surfaces has its own vocabulary for
naming, and each one is the shape of the thing it addresses**, which is why they differ.

| Surface | What names the part | What names the thing on it |
|---|---|---|
| [`Deck`] | [`Surface`] — `Slide(0)`, `Layout(1)`, `Master(0)`, `Notes(0)`, `NotesMaster` | [`ShapePath`] — `2`, or `[2, 1]` into a group |
| [`Document`] | nothing: a Word document has one body | [`BlockPath`] + [`RunPath`] — a paragraph, then a run in it |
| [`Workbook`] | a `u32` tab index | A1 text — `"B7"`, `"A1:C3"` — or an anchor index |

## `Deck`: a surface and a path

A [`Surface`] says which shape-bearing part. All five carry the same `p:cSld > p:spTree`, so every
shape method applies to each equally — and editing a **layout or master** is how one change reaches
many slides.

A [`ShapePath`] says which shape on it. A surface's shapes share **one index space covering every
kind** — autoshapes, pictures, groups, graphic frames, connectors — in document order. A group is
one entry on that space; its members are reached by descending into it, as deep as the groups nest.

Both convert from a bare `u32`, so `slide.into()` is the whole ceremony for the common case.

<!-- guide-example: addressing_a_deck rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Deck, PresetShapeType, ShapeBounds, ShapePath, SlideSize, Surface};

let mut deck = Deck::blank(SlideSize::widescreen())?;
// `add_slide_from_layout` would copy the layout's placeholders too.
let slide = Surface::Slide(deck.add_slide()?);
let rectangle = ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0);
let ellipse = ShapeBounds::from_inches(4.0, 1.0, 2.0, 1.0);
deck.add_shape(slide, PresetShapeType::Rectangle, rectangle)?;
deck.add_shape(slide, PresetShapeType::Ellipse, ellipse)?;
assert_eq!(deck.shape_count(slide)?, 2);

// The group itself is one entry on the surface's index space.
let group: ShapePath = deck.group_shapes(slide, &[0.into(), 1.into()])?;
assert!(group.is_top_level());

// Member 1 of that group, one step deeper.
let member = group.child(1);
assert_eq!(member.depth(), 2);
assert_eq!(member.indices().len(), 2);
assert!(!member.is_top_level());
assert_eq!(member.parent(), Some(group.clone()));

let saved = deck.save()?;
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: addressing_a_deck python -->
```python
from mjx_ooxml import Deck, PresetShapeType, ShapeBounds, SlideSize, Surface

deck = Deck.blank(SlideSize.widescreen())
# `add_slide_from_layout` would copy the layout's placeholders too.
slide = Surface.slide(deck.add_slide())
rectangle = ShapeBounds.from_inches(1.0, 1.0, 2.0, 1.0)
ellipse = ShapeBounds.from_inches(4.0, 1.0, 2.0, 1.0)
deck.add_shape(slide, PresetShapeType.Rectangle, rectangle)
deck.add_shape(slide, PresetShapeType.Ellipse, ellipse)
assert deck.shape_count(slide) == 2

# The group itself is one entry on the surface's index space.
group = deck.group_shapes(slide, [0, 1])
assert group.is_top_level

# Member 1 of that group, one step deeper.
member = group.child(1)
assert member.depth == 2
assert len(member.indices) == 2
assert not member.is_top_level
assert member.parent == group

saved = deck.save()
```
<!-- guide-example end -->

<!-- guide-example: addressing_a_deck js -->
```js
import { Deck, PresetShapeType, ShapeBounds, SlideSize, Surface } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
// `addSlideFromLayout` would copy the layout's placeholders too.
const slide = Surface.slide(deck.addSlide());
const rectangle = ShapeBounds.fromInches(1.0, 1.0, 2.0, 1.0);
const ellipse = ShapeBounds.fromInches(4.0, 1.0, 2.0, 1.0);
deck.addShape(slide, PresetShapeType.Rectangle, rectangle);
deck.addShape(slide, PresetShapeType.Ellipse, ellipse);
if (deck.shapeCount(slide) !== 2) {
  throw new Error("the slide should carry two shapes");
}

// The group itself is one entry on the surface's index space.
const group = deck.groupShapes(slide, [0, 1]);
// Member 1 of that group, one step deeper.
const member = group.child(1);
const parent = member.parent;
if (!group.isTopLevel || member.isTopLevel) {
  throw new Error("the group is top level and its member is not");
}
if (member.depth !== 2 || member.indices.length !== 2) {
  throw new Error("a group member's address is two indices deep");
}
if (!parent.equals(group)) {
  throw new Error("a member's parent is the group it belongs to");
}

const saved = deck.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [deck, slide, rectangle, ellipse, group, member, parent]) {
  handle.free();
}
```
<!-- guide-example end -->

Three differences between those blocks are the languages rather than this library, and they recur on
every page below. A Rust enumeration variant carrying a payload — `Surface::Slide(0)` — is a static
constructor in both bindings, because neither Python nor JavaScript has one; a Rust method that
answers a question, `depth()`, is a property, `depth` and `.depth`; and JavaScript has no operator
overloading, so two addresses are compared with `equals` where the other two use `==`. Every call in
front of those is the same call with the same arguments, which is exactly what these blocks being
copies of three running files is here to keep true.

A top-level path — one index, no descent — is stored inline and **never allocates**; only a path
that descends into a group allocates, once, on its way down. That matters because these values are
built on every call.

`Surface::from(3)` is `Surface::Slide(3)`, because a bare index almost always means a slide, and
`Surface::NotesMaster` is the one member with no index of its own —
[`Surface::index`](crate::Surface::index) reports `0` for it. [`Surface::is_master_like`] is the
question the inheritance readers ask: does this surface stand at the head of its own chain?

## `Document`: a block and a run

A Word document has one body, so nothing names the part. A [`BlockPath`] names a paragraph and a
[`RunPath`] names a run inside it. Both convert from a bare index the same way, and both descend:
a paragraph inside a table cell, inside a content control, inside another table, is a path with a
segment per level.

<!-- guide-example: addressing_a_document rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.append_paragraph()?;
document.append_run(0, "Quarterly ")?;
document.append_run(0, "results")?;
assert_eq!(document.run_count(0)?, 2);
assert_eq!(document.run_text(0, 1)?, "results");
assert_eq!(document.paragraph_text(0)?, "Quarterly results");

let saved = document.save()?;
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: addressing_a_document python -->
```python
from mjx_ooxml import Document, PageSize

document = Document.blank(PageSize.a4())
document.append_paragraph()
document.append_run(0, "Quarterly ")
document.append_run(0, "results")
assert document.run_count(0) == 2
assert document.run_text(0, 1) == "results"
assert document.paragraph_text(0) == "Quarterly results"

saved = document.save()
```
<!-- guide-example end -->

<!-- guide-example: addressing_a_document js -->
```js
import { Document, PageSize } from "@mjx/ooxml";

const document = Document.blank(PageSize.a4());
document.appendParagraph();
document.appendRun(0, "Quarterly ");
document.appendRun(0, "results");
if (document.runCount(0) !== 2) {
  throw new Error("the paragraph should carry two runs");
}
if (document.runText(0, 1) !== "results") {
  throw new Error("run 1 is the second run of paragraph 0");
}
if (document.paragraphText(0) !== "Quarterly results") {
  throw new Error("a paragraph's text is its runs, concatenated");
}

const saved = document.save();
document.free(); // a wasm handle owns memory the garbage collector cannot see
```
<!-- guide-example end -->

**Twenty-three of the `Document` methods take `impl Into<BlockPath>` rather than the concrete path**,
which is how `document.append_run(0, "…")` above compiles with a bare integer where the `Deck`
equivalent needs `slide.into()`. It is a deliberate difference from this crate's own stated
translation rule, recorded in `crates/mjx-ooxml/src/document.rs`'s module documentation; it costs
the bindings nothing, because a `BlockPath` satisfies `Into<BlockPath>`.

## `Workbook`: a tab index and A1 text

The one address that is **text**. `mjx_sml::CellReference` and `mjx_sml::CellRange` are the parsed
forms, and both are re-exported here — but nothing on the [`Workbook`] surface takes one. Every
address argument is the A1 string a spreadsheet user already spells, because an eight-byte address
value would have to become a class in Python and a class in TypeScript for no gain.

```
use mjx_ooxml::{CellInput, CellWrite, Workbook};

# fn main() -> Result<(), mjx_ooxml::Error> {
let mut workbook = Workbook::blank()?;
workbook.write_cells(0, &[
    CellWrite::new("A1", CellInput::SharedText("Region".into())),
    CellWrite::new("B1", CellInput::Number(12.5)),
    CellWrite::new("$B$2", CellInput::Number(18.0)),   // the anchoring is data; both spell one cell
])?;

let block = workbook.read_range(0, "A1:B2")?;
assert_eq!(block.first_row(), 0, "A1 is row 0, column 0");
assert_eq!(block.first_column(), 0);
assert_eq!(block.value(0, 0)?.text(), Some("Region"));
assert_eq!(block.value(1, 1)?.number(), Some(18.0));
assert_eq!(block.range().as_deref(), Some("A1:B2"));
assert_eq!(workbook.used_range(0)?.as_deref(), Some("A1:B2"));
# Ok(())
# }
```

A [`CellBlock`] is **row-major over the whole requested rectangle, blanks included**, and its
`row`/`column` arguments are offsets *into the block* rather than sheet coordinates —
[`CellBlock::first_row`] and [`CellBlock::first_column`] say where the block sits. The two open-ended
range forms `"A:C"` and `"1:3"` are clamped to the sheet's populated extent before anything is
allocated, so `"A:A"` reads the column a file actually has rather than reserving 1,048,576 slots for
one it does not.

## Row first, except where the file says otherwise

Forty-seven methods across these three surfaces take `(row, column)` — [`CellBlock::value`],
`Deck::cell_text`, `Document::set_cell_text`, [`ErrorDetail`]'s own two fields — because each indexes
a two-dimensional *body* of cells, where row-major is the ordinary convention.

**Four take the column first**, and it is the same four:
[`Workbook::add_chart`](crate::Workbook::add_chart),
[`Workbook::add_range_chart`](crate::Workbook::add_range_chart),
[`Workbook::add_one_cell_anchored_picture`] and [`Workbook::add_two_cell_anchored_picture`]. Each
flattens an `xdr` anchor marker into plain numbers, and a marker is
`<xdr:col><xdr:colOff><xdr:row><xdr:rowOff>` in the file — its two offsets interleave with its two
indices, so taking the row first would put each offset beside the wrong one. The order reads straight
down the element it writes, which is the same reason `mjx_sml::CellReference::relative` takes
`(column, row)`.

```
use mjx_ooxml::{ChartData, ChartKind, ResizingBehavior, Workbook};

# fn main() -> Result<(), mjx_ooxml::Error> {
let mut workbook = Workbook::blank()?;
let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2"])
    .series("North", [12.5, 18.0]);
// from column 1, row 1 (B2) to column 7, row 16 (H17) — column first, both times.
let anchor = workbook.add_chart(0, &chart, 1, 1, 7, 16, "Revenue", ResizingBehavior::MoveAndResizeWithAnchorCells)?;
assert_eq!(workbook.chart_anchor_indices(0)?, vec![anchor]);
# Ok(())
# }
```

There is no compiler that will catch a caller who transposes those, because both arguments are
`u32`. That is the cost of the asymmetry, it is recorded rather than papered over, and changing it is
the user's call rather than this library's.
