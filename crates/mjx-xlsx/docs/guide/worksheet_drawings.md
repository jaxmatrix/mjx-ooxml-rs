# Worksheet drawings

Everything on a sheet that is **not a cell** — a picture, a shape, a chart's frame, the box a comment
draws in — hangs off `xdr:wsDr` in a drawing part the worksheet relates to. It is Excel's equivalent
of PowerPoint's shape tree, with one difference that changes everything about it: a slide's shape
tree positions a shape in slide coordinates, and this one positions it against a **grid that moves**.

That makes a drawing six things that have to agree:

| thing | where it lives |
|---|---|
| the anchors | `xl/drawings/drawingN.xml`, an `xdr:wsDr` |
| its content type | an `Override` in `[Content_Types].xml` |
| the edge to it | a `drawing` relationship in `xl/worksheets/_rels/sheetN.xml.rels` |
| the sheet's claim on it | `x:drawing@r:id`, rank 29 of `CT_Worksheet` |
| a picture's image | `xl/media/imageN.png`, bytes this library never decodes |
| the edge to *that* | an `image` relationship in `xl/drawings/_rels/drawingN.xml.rels` |

The last row is the one worth reading twice. An `a:blip@r:embed` is resolved against **the part that
contains it**, so an image related from the *sheet* names nothing at all. [`Workbook::add_two_cell_anchored_picture`]
and its two siblings write all six in one call, and create the drawing part on demand for a sheet
that has none.

## Reading what is on a sheet

[`Workbook::sheet_drawing`] answers with owned values — no closure, no borrow, no interner:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("../../tests/fixtures/worksheet_drawings.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;

let drawing = workbook.sheet_drawing(0)?.expect("sheet 1 has a drawing");
assert_eq!(drawing.part.as_str(), "/xl/drawings/drawing1.xml");

// Four objects, in the order the file lists them — which is the order a consumer paints them.
let shapes: Vec<(&str, Option<&str>)> = drawing
    .objects
    .iter()
    .map(|object| (object.anchor, object.object))
    .collect();
assert_eq!(
    shapes,
    vec![
        ("twoCellAnchor", Some("pic")),
        ("twoCellAnchor", Some("sp")),
        ("oneCellAnchor", Some("pic")),
        ("absoluteAnchor", Some("pic")),
    ],
);

// A picture's image is resolved through the **drawing part's** own relationships.
assert_eq!(
    drawing.objects[3].image.as_ref().map(|part| part.as_str()),
    Some("/xl/media/image2.jpeg"),
);
# Ok(())
# }
```

Everything the report does not carry is reached through [`Workbook::drawing_markup`], which hands you
the [`mjx_dml::spreadsheet_drawing::WorksheetDrawing`] **and the interner it was parsed with**. That
second argument is not decoration: an anchor's names are symbols of the drawing part's own interner,
and resolving one against the worksheet's does not fail loudly — it answers whatever string sits at
that index.

## Three anchors, and what each one promises

| element | pinned to | its size | when rows or columns move |
|---|---|---|---|
| `xdr:twoCellAnchor` | two cells | **the cells** — it has no extent of its own | moves *and* resizes |
| `xdr:oneCellAnchor` | one cell, plus an `xdr:ext` | its own | moves, keeps its size |
| `xdr:absoluteAnchor` | the sheet, in EMU | its own | does neither |

There is a fourth statement, and it is an *attribute* rather than an element: `twoCellAnchor@editAs`.
It defaults to `twoCell`, but a producer is free to write `editAs="oneCell"` on a two-cell anchor,
and Apache POI writes exactly that for every move-but-do-not-resize picture. **The element says how
the geometry is stated; the attribute says what should happen to it.** They are different questions:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
let bytes = std::fs::read("../../tests/fixtures/worksheet_drawings.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;
let drawing = workbook.sheet_drawing(0)?.expect("a drawing");

// A `twoCellAnchor` whose own attribute says otherwise…
assert_eq!(drawing.objects[0].anchor, "twoCellAnchor");
assert_eq!(
    drawing.objects[0].resizing,
    ResizingBehavior::MoveWithCellsButDoNotResize,
);
// …beside one that says nothing, and so means what its element means.
assert_eq!(
    drawing.objects[1].resizing,
    ResizingBehavior::MoveAndResizeWithAnchorCells,
);
# Ok(())
# }
```

[`Workbook::insert_rows_into_drawing`] and its three siblings move every anchor the way its own mode
promises, and hand back a report saying per anchor what happened. **They move the drawing, not the
cells**: nothing in this library inserts a row into a sheet, so a caller doing that itself calls this
so the objects over those rows travel with them.

One promise the markers alone cannot keep — a `twoCellAnchor` resized while its `@editAs` forbids it
— comes back with `promise_kept: false`, naming itself, rather than being left silently wrong.

## Where an anchor actually is, and which half of that is a measurement

[`Workbook::sheet_anchor_bounds`] resolves an anchor to a rectangle in EMU. The two axes are **not
equally answerable**, and the difference is not a matter of effort:

* **A row height is exact.** `row@ht` and `sheetFormatPr@defaultRowHeight` are in points, and a point
  is 12,700 EMU by definition.
* **A column width is not, and cannot be.** `col@width` is *"the number of characters of the maximum
  digit width of the numbers 0, 1, 2, …, 9 as rendered in the normal style's font"* (ECMA-376 Part 1
  §18.3.1.13). Turning that into a length needs a **font measurement this library never makes**.

So the column half is computed through a [`mjx_sml::ColumnMetrics`] the caller supplies, every answer
carries the metrics it was computed with, and each axis reports whether its number came from a stated
`ht`/`width`, from the sheet's own default, or from `sheetFormatPr@baseColWidth`:

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_sml::{ColumnMetrics, GeometrySource};
let bytes = std::fs::read("../../tests/fixtures/worksheet_drawings.xlsx")?;
let workbook = mjx_xlsx::Workbook::open(&bytes)?;

let bounds = workbook
    .sheet_anchor_bounds(0, 0, ColumnMetrics::CALIBRI_11_AT_96_DPI)?
    .expect("the sheet states enough to place it");

// Apache POI wrote these two numbers into this fixture's own `a:ext`, from the same column widths.
assert_eq!(bounds.size.width.emu(), 2_085_975);
assert_eq!(bounds.size.height.emu(), 885_825);

// …and the answer says what it rests on. Every column crossed states its own width; the rows fell
// back to the sheet's `defaultRowHeight`.
assert_eq!(bounds.column_source, GeometrySource::Stated);
assert_eq!(bounds.row_source, GeometrySource::SheetDefault);
assert!(!bounds.is_fully_stated());
# Ok(())
# }
```

**Where the sheet states nothing that could place the object, the answer is `None`.**
`sheetFormatPr@defaultRowHeight` is `use="required"`, so a worksheet with no `x:sheetFormatPr` states
no default row height at all — and a row that writes no `@ht` on such a sheet has no height this
library can report. Answering `0`, or Excel's own 15 points, would be presenting a guess as a
measurement. It is the same answer `mjx_pptx`'s `effective_shape_bounds` gives for a transform naming
a rotation but not both `a:off` and `a:ext`.

A hidden row or column has **no** height or width: `row@hidden`, `col@hidden` and
`sheetFormatPr@zeroHeight` are layout, not decoration, and every object below a hidden row moves up.

## Adding a picture

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_dml::spreadsheet_drawing::CellMarker;
# use mjx_dml::Size;
# use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
# const PNG: &[u8] = &[
#     0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
#     b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
#     0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8,
#     0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
#     0x00, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82,
# ];
let mut workbook = mjx_xlsx::Workbook::blank()?;

// Between B3 and D6, starting a little way into each — the offsets are EMU **into** the cell, not
// from the sheet origin, which is what lets the anchor survive the column being widened.
let at = workbook.add_two_cell_anchored_picture(
    0,
    PNG,
    "logo",
    CellMarker::new(1, 190_500, 2, 47_625),
    CellMarker::new(3, 95_250, 5, 19_050),
    ResizingBehavior::MoveAndResizeWithAnchorCells,
)?;
assert_eq!(at, 0);

// …and one anchored to a single cell, carrying its own size.
workbook.add_one_cell_anchored_picture(
    0,
    PNG,
    "badge",
    CellMarker::new(4, 76_200, 1, 38_100),
    Size::from_emu(914_400, 457_200),
)?;

let drawing = workbook.sheet_drawing(0)?.expect("the sheet gained one");
assert_eq!(drawing.objects.len(), 2);
// The same bytes twice: one image part, one relationship.
assert_eq!(drawing.objects[0].image, drawing.objects[1].image);

let saved = workbook.save()?;
assert!(saved.len() > 1_000);
# Ok(())
# }
```

Identical images are stored once, `cNvPr@id` is one past the highest the drawing already uses and
never reused, and bytes that match no image format this build recognises are refused **before
anything is written** — so a refusal leaves the workbook exactly as it was.

## What this page does not cover

**Charts on a drawing part** are MJXOFF-111's: an `xdr:graphicFrame` is reported here, with its
transform and its `a:graphic` reference, and its content is not decomposed. **Cell comments and the
legacy VML box they draw in** are MJXOFF-114's; `x:legacyDrawing` at rank 30 is still held as the
markup the file wrote.

And nothing here **renders**. A shape's preset geometry is a named outline this library reports and
never evaluates into a path, exactly as it is for PowerPoint.
