# Charts

A chart in a Word document is the *same part* a chart in a PowerPoint deck is: `c:chartSpace`, in
its own `word/charts/chartN.xml`, with an embedded `.xlsx` workbook beside it holding the numbers
*Edit Data* opens. What differs is only how the document reaches it — a `c:chart` reference inside
`a:graphicData`, inside `wp:inline` or `wp:anchor`, inside a `w:drawing`, inside a run.

So the whole chart vocabulary is the vocabulary you already know from `mjx-pptx`, under the same
method names, taking the same arguments in the same order. The one difference is the address.

## Addressing a chart

A slide addresses a chart as `(surface, shape index)`. A document has no shape tree, so it addresses
one by the **`wp:docPr` id** every drawing carries — the same id
[`Document::add_inline_picture`] hands back and [`Document::remove_drawing`] takes.
[`Document::add_chart`] returns it, and every other chart method takes it.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_chart::{ChartData, ChartKind};
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5])
    .series("South", [9.0, 11.5, 14.0]);

// 4,572,000 × 2,743,200 EMU is 5 in × 3 in.
let drawing = document.add_chart(0, &chart, 4_572_000, 2_743_200, "Revenue")?;

assert_eq!(document.chart_drawing_ids()?, [drawing]);
assert_eq!(document.chart_kinds(drawing)?, [ChartKind::Bar]);
assert_eq!(
    document.chart_series(drawing)?[0].name.as_deref(),
    Some("North")
);
# Ok(())
# }
```

[`Document::chart_drawing_ids`] enumerates the drawings that frame a chart, in document order. That
is the Word equivalent of walking a slide's shapes looking for chart frames, and it is how you find
the charts in a file somebody else wrote — the ids are read from the file, never assumed. A document
written by another producer may well number its first drawing `0`.

## What is written

Adding one chart writes three parts and one run:

| Part | What it holds |
|---|---|
| `word/charts/chartN.xml` | the chart — the series' caches (what renders) and a `c:externalData` naming its workbook |
| `word/embeddings/Microsoft_Excel_SheetN.xlsx` | the embedded workbook, laid out to match the chart's own `c:f` formulas cell for cell |
| `word/charts/_rels/chartN.xml.rels` | the relationship binding the two |

Those are the directory and file names Word itself uses, so a document this library writes and one
Word writes are laid out alike. The workbook is composed by `mjx-chart`, which hands its rows to
`mjx-sml`'s workbook writer — one writer for every embedded workbook in the project, whichever
format carries it.

[`Document::remove_drawing`] takes the chart away again: the run, the chart part, and the workbook
that chart part alone referenced.

## Inline and floating

[`Document::add_chart`] places the chart **inline** — it flows with the text like an oversized
character, which is what Word inserts by default. [`Document::add_chart_placed`] takes a
[`ChartPlacement`] instead, so a chart can float at an offset with the text wrapping around it.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_chart::{ChartData, ChartKind};
use mjx_docx::{ChartPlacement, ChartWrap, Document, PageSize};
use mjx_ooxml_types::wordprocessingdrawing::WrapText;

let mut document = Document::blank(PageSize::a4())?;
let chart = ChartData::new(ChartKind::Pie)
    .categories(["North", "South"])
    .series("Share", [60.0, 40.0]);

let drawing = document.add_chart_placed(
    0,
    &chart,
    2_743_200,
    2_743_200,
    "Share",
    ChartPlacement::Floating {
        offset_x_emu: 228_600,
        offset_y_emu: 114_300,
        wrap: ChartWrap::Square(WrapText::BothSides),
    },
)?;

assert_eq!(document.chart_kinds(drawing)?, [ChartKind::Pie]);
# Ok(())
# }
```

[`ChartWrap`] offers three of `EG_WrapType`'s five members — `None`, `Square` and `TopAndBottom`.
The two outline wraps (`wp:wrapTight`, `wp:wrapThrough`) are **deliberately not** on this surface:
both need a `wp:wrapPolygon`, and the coordinate space that polygon is measured in is not something
this project will guess at. Reading a document that carries one is unaffected — `mjx-dml` models all
five and this crate preserves them — and a caller who needs to *write* one builds it through
`mjx_dml::wordprocessing_drawing::WrapOutline::new`, where the polygon is theirs to supply.

## Editing

Series values and categories, the axes, the title, the legend, the series' fill and line, and the
whole decoration tier (data labels, per-point formatting, trendlines, error bars) all have setters,
under the names the PowerPoint surface uses.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_chart::{ChartData, ChartKind, LegendPosition};
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5]);
let drawing = document.add_chart(0, &chart, 4_572_000, 2_743_200, "Revenue")?;

document.set_chart_title(drawing, Some("Regional revenue"))?;
document.set_chart_legend(drawing, Some(LegendPosition::Right))?;
document.set_chart_series_values(drawing, 0, &[40.0, 41.0, 42.0])?;
document.set_chart_axis_scale(drawing, 1, Some(0.0), Some(50.0))?;
document.set_chart_axis_title(drawing, 1, Some("Millions"))?;

assert_eq!(
    document.chart_title(drawing)?.as_deref(),
    Some("Regional revenue")
);
assert_eq!(document.chart_series(drawing)?[0].values, [40.0, 41.0, 42.0]);
assert_eq!(document.chart_axes(drawing)?[1].maximum, Some(50.0));
# Ok(())
# }
```

**A data edit brings the embedded workbook along in the same call**, so the numbers *Edit Data*
shows are the numbers the chart draws; they cannot go stale by forgetting a second call.
[`Document::refresh_chart_workbook`] is public for a caller who has edited a chart some other way,
and answers `false` — changing nothing — when there is no embedded workbook at all.

The workbook is **patched, not replaced** (MJXOFF-208). Each series' `c:f` says which cells its data
lives in, and only those cells are written; every other sheet, cell format, defined name, macro and
document property a producer's workbook carried survives byte for byte, and a cell that already holds
its value is not rewritten at all. A `c:f` this library will not write over — one naming another
workbook, several sheets, whole columns, a rectangle, or fewer cells than the data has points — is a
[`DocxError::ChartAccess`] rather than a quiet fall back to rebuilding, and the edit leaves both
parts as they were: the workbook is worked out before the chart is touched and written after, so a
data edit is all of it or none of it.

[`Document::regenerate_chart_workbook`] is the explicit opt-in that does replace it wholesale, and
**discards whatever it held**. So is [detaching it](Document::detach_chart_workbook), which drops the
reference instead of writing over the file, leaving the chart to render from its caches.

## What an edit touches

The three fidelity tiers hold here as they do everywhere else in this library, and edit isolation is
the one worth stating precisely:

* **Adding** a chart leaves every existing part byte-identical, except `word/document.xml` (which
  gains the run), the content types and the document's own `.rels` (which gain the new parts).
* **A data edit** touches the chart part and its workbook, and nothing else — no part is added and
  none is removed.
* **A styling edit** touches the chart part alone; the workbook is not rewritten, because the
  numbers did not change.

And within the chart part itself, an edit changes **only the markup it is about**. A chart part
another producer wrote comes back byte-for-byte apart from the element or attribute you edited —
including its own attribute order, its own boolean spellings (`val="false"` where this library
writes `val="0"`), and its own number formatting. That is not a general property of typed models: an
earlier tier of this project read a whole chart part into a model and wrote the whole part back, so
an edit re-flowed everything it touched and much it did not. Carrying the source span through the
model closed it, and `crates/mjx-docx/tests/charts.rs` measures it on a producer-written part rather
than asserting it.

## The same chart, from a deck or from a document

Every method above exists on `mjx_ooxml::Deck` too, with the same name and the same arguments after
the address. That is not a coincidence maintained by hand: since MJXOFF-103 both surfaces call the
*same* functions in `mjx_chart::chart_ops`, so a chart read from a document and the same chart read
from a deck answer the same thing by construction. `crates/mjx-ooxml/tests/chart_surface_parity.rs`
holds the two side by side and compares every reader, every setter and every refusal's error code —
down to the two chart parts coming out byte-identical.

Both bindings carry the family too: `document.add_chart(...)` in Python, `document.addChart(...)` in
TypeScript.

## What is not here

* **A chart on a worksheet**, and the live-range data source that reads a chart's numbers straight
  out of a workbook rather than from its caches.
* **`c:dPt > c:marker`** and **`c:pictureOptions`** stay in the raw bucket — preserved through a
  round trip, not typed.
* **`c:layout`** and **`c:trendlineLbl`** are preserved but not typed: both are manual-placement
  instructions, which is a rendering concern this library does not have.
* **The chart colour and style parts** (`colors1.xml`, `style1.xml`) are carried verbatim when a file
  has them and are never authored.
* An **image fill** on a series or a point is refused rather than written: an image fill names an
  image relationship, and a chart part relates to no images.
