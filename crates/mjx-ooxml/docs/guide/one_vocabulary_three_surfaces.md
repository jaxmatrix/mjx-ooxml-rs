# One vocabulary, three surfaces

This crate is the only place in the workspace that may hold a [`Deck`], a [`Document`] and a
[`Workbook`] at once. `mjx-pptx`, `mjx-docx` and `mjx-xlsx` are all rank 3.0 in the layering table
`CLAUDE.md` states, so none of them may reach another — which means **every claim on this page can
only be checked here**, and `crates/mjx-ooxml/tests/chart_surface_parity.rs` is where it is.

## Charts: the same fifty method names on all three

A chart in a slide, a chart in a Word document and a chart on a worksheet are the same ChartML part
with three different owners. The facade spells them identically, so a caller who learned charts on
one surface has learned them on all three. Only the **address** in front differs, and it differs
because the containers do: a slide has a shape tree, a Word drawing has a `wp:docPr` id, and a
worksheet has neither — a chart there is the *n*th anchor of the sheet's drawing part.

| Surface | Address | Reading a title | Retitling it |
|---|---|---|---|
| [`Deck`] | [`Surface`] + [`ShapePath`] | `chart_title(surface, shape)` | `set_chart_title(surface, shape, text)` |
| [`Document`] | one `drawing_id` | `chart_title(drawing_id)` | `set_chart_title(drawing_id, text)` |
| [`Workbook`] | `sheet` + `anchor` | `chart_title(sheet, anchor)` | `set_chart_title(sheet, anchor, text)` |

Everything after the address is identical, argument for argument — `crate::Deck::chart_title`,
`crate::Document::chart_title` and `crate::Workbook::chart_title` differ only in what comes before
`text`, and so do `crate::Deck::chart_series`, `crate::Deck::chart_axes`,
`crate::Deck::chart_legend`, `crate::Deck::chart_data_labels`, `crate::Deck::add_chart_trendline`
and their forty-odd siblings on each surface.

<!-- guide-example: the_same_chart_on_all_three rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{ChartData, ChartKind, Deck, Document, PageSize};
use mjx_ooxml::{ShapeBounds, SlideSize, Surface};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5])
    .title("Quarterly revenue".to_owned());

// A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
let mut deck = Deck::blank(SlideSize::widescreen())?;
deck.add_slide()?;
let slide = Surface::Slide(0);
let bounds = ShapeBounds::from_inches(1.0, 1.0, 5.0, 3.0);
let shape = deck.add_chart(slide, &chart, bounds)?;

// A Word drawing is inline in a paragraph, so it takes a width and a height.
let mut document = Document::blank(PageSize::a4())?;
let drawing = document.add_chart(0.into(), &chart, 4_572_000, 2_743_200, "Revenue")?;

// Everything after the address is identical: the same question, the same answer.
let on_slide = deck.chart_title(slide, shape.into())?;
let in_document = document.chart_title(drawing)?;
assert_eq!(on_slide.as_deref(), Some("Quarterly revenue"));
assert_eq!(on_slide, in_document);
assert_eq!(deck.chart_series(slide, shape.into())?.len(), 1);
assert_eq!(document.chart_series(drawing)?.len(), 1);

let saved = deck.save()?;
let saved_document = document.save()?;
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: the_same_chart_on_all_three python -->
```python
from mjx_ooxml import ChartData, ChartKind, Deck, Document, PageSize
from mjx_ooxml import ShapeBounds, SlideSize, Surface

chart = (
    ChartData(ChartKind.Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5])
    .title("Quarterly revenue")
)

# A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
deck = Deck.blank(SlideSize.widescreen())
deck.add_slide()
slide = Surface.slide(0)
bounds = ShapeBounds.from_inches(1.0, 1.0, 5.0, 3.0)
shape = deck.add_chart(slide, chart, bounds)

# A Word drawing is inline in a paragraph, so it takes a width and a height.
document = Document.blank(PageSize.a4())
drawing = document.add_chart(0, chart, 4_572_000, 2_743_200, "Revenue")

# Everything after the address is identical: the same question, the same answer.
on_slide = deck.chart_title(slide, shape)
in_document = document.chart_title(drawing)
assert on_slide == "Quarterly revenue"
assert on_slide == in_document
assert len(deck.chart_series(slide, shape)) == 1
assert len(document.chart_series(drawing)) == 1

saved = deck.save()
saved_document = document.save()
```
<!-- guide-example end -->

<!-- guide-example: the_same_chart_on_all_three js -->
```js
import { ChartData, ChartKind, Deck, Document, PageSize } from "@mjx/ooxml";
import { ShapeBounds, SlideSize, Surface } from "@mjx/ooxml";

const chart = new ChartData(ChartKind.Bar)
  .categories(["Q1", "Q2", "Q3"])
  .series("North", [12.5, 18.0, 21.5])
  .title("Quarterly revenue");

// A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
const deck = Deck.blank(SlideSize.widescreen());
deck.addSlide();
const slide = Surface.slide(0);
const bounds = ShapeBounds.fromInches(1.0, 1.0, 5.0, 3.0);
const shape = deck.addChart(slide, chart, bounds);

// A Word drawing is inline in a paragraph, so it takes a width and a height.
const document = Document.blank(PageSize.a4());
const drawing = document.addChart(0, chart, 4_572_000, 2_743_200, "Revenue");

// Everything after the address is identical: the same question, the same answer.
const onSlide = deck.chartTitle(slide, shape);
const inDocument = document.chartTitle(drawing);
if (onSlide !== "Quarterly revenue" || onSlide !== inDocument) {
  throw new Error("both surfaces answer the same title for the same chart");
}
if (deck.chartSeries(slide, shape).length !== 1) {
  throw new Error("the slide's chart holds one series");
}
if (document.chartSeries(drawing).length !== 1) {
  throw new Error("the document's chart holds one series");
}

const saved = deck.save();
const savedDocument = document.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [chart, deck, document, slide, bounds]) {
  handle.free();
}
```
<!-- guide-example end -->

The example authors **two** packages, because that is the claim — one chart description, two owners,
one vocabulary — and both are compared part by part in all three languages.

`chart_surface_parity.rs` checks two separate things, and the second is the one that would have been
easy to skip: that the names exist on every surface with matching argument order after the address —
a compile-time fact — and that they **answer the same thing about the same bytes**. Giving all three
byte-identical chart parts and comparing every reader pairwise is what catches a Word method wired to
the wrong `mjx_chart::chart_ops` function, which "the method exists" cannot see and which neither
format crate's own suite can see either, because each only ever compares a surface against itself.
Every reader on all three answers in the same shared types — `mjx_chart::ChartSeriesData`,
`mjx_chart::ChartAxisData`, `mjx_chart::ChartLegendData`, `mjx_chart::ChartTrendlineData` — because
`mjx-chart` is where they live and all three format crates sit above it.

**One parameter is spelled differently on `Workbook`**, and it is worth knowing before you write
Python: the chart decoration calls name their series `series_idx`, `point_idx`, `axis_idx` and
`trendline_idx` on [`Deck`] and [`Document`], and `series`, `point`, `axis` and `trendline` on
[`Workbook`]. Twenty-five methods, positional in Rust and in TypeScript, **keyword-visible in
Python**. Recorded rather than corrected, because either spelling is a rename in three languages
(MJXOFF-214).

## Three calls each surface has, meaning three different things

Not everything shared is shared all the way down, and the differences are the containers speaking:

* **`add_chart`** takes [`ShapeBounds`] on a deck (a slide is a canvas in EMU), a width and a height
  on a document (a Word drawing is inline in a paragraph), and four cell indices plus a
  [`ResizingBehavior`] on a workbook (a sheet anchor moves with its cells). See
  [Addressing](addressing) for why the last of those takes the column first.
* **`chart_workbooks`** is on all three, and answers a different question on the third.
  `crate::Workbook::add_range_chart` makes the one chart in this library whose data source is a
  **live range in the same workbook** rather than an embedded copy — so
  `crate::Workbook::chart_series_freshness` and `crate::Workbook::refresh_chart_cache_from_cells`
  exist there and nowhere else, and `mjx_xlsx::ChartSeriesFreshness` is the answer they give.
* **The escape hatch** is `crate::Deck::presentation_mut`, `crate::Document::document_mut` and
  `crate::Workbook::workbook_mut`, handing back `mjx_pptx::Presentation`, `mjx_docx::Document` and
  `mjx_xlsx::Workbook` respectively — one name each, in the shape of the crate below. See
  [The curated surface](the_curated_surface).

## Shared markup, and which surface can reach what

Five crates in this workspace hold markup that is no single format's — `mjx-dml` (DrawingML, whose
`mjx_dml::Theme` and `mjx_dml::ShapeGeometry` every format resolves against), `mjx-sml`
(SpreadsheetML markup, `mjx_sml::WorksheetPart`), `mjx-chart` (ChartML, `mjx_chart::ChartData`),
`mjx-vml` (`mjx_vml::Drawing`) and `mjx-omml` (`mjx_omml::Math`). The three
surfaces do **not** reach the same amount of each, and the asymmetries are real rather than
oversights: a PowerPoint shape has an outline and a Word drawing does not expose one, an embedded
workbook is reachable from a chart on any surface, and only a `.pptx` has ink.

[Shared-markup reachability](crate::shared_markup_reachability) is the table, with a written reason
beside every asymmetry, and `crates/mjx-ooxml/tests/shared_markup_reachability.rs` is what fails when
the table and the code disagree.

## The authoring vocabulary is one vocabulary

The specs are shared too, and this is where the payoff shows: `mjx_dml::FillSpec`,
`mjx_dml::ColorSpec`, `mjx_dml::LineSpec`, `mjx_dml::CharacterPropertiesSpec`,
`mjx_dml::ParagraphPropertiesSpec`, `mjx_dml::EffectListSpec` and the DrawingML enumerations behind
them are the same types whichever surface you hand them to — re-exported here as [`FillSpec`],
[`ColorSpec`], [`LineSpec`] and the rest, so no caller ever names `mjx-dml`.

<!-- guide-example: one_authoring_vocabulary rust -->
```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_ooxml::{ChartData, ChartKind, ColorSpec, Deck, FillSpec};
use mjx_ooxml::{PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook};

let navy = FillSpec::solid(ColorSpec::Srgb("1F3864".into()));

// On a shape in a deck.
let mut deck = Deck::blank(SlideSize::widescreen())?;
let slide = Surface::Slide(deck.add_slide_from_layout(0)?);
let bounds = ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0);
let shape = deck.add_shape(slide, PresetShapeType::Rectangle, bounds)?;
deck.set_shape_fill(slide, shape.into(), &navy)?;
assert!(deck.shape_fill(slide, shape.into())?.is_some());

// The same value, on a chart series in a workbook.
let mut workbook = Workbook::blank()?;
let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1"])
    .series("North", [12.5]);
let resizing = ResizingBehavior::MoveAndResizeWithAnchorCells;
let anchor = workbook.add_chart(0, &chart, 1, 1, 7, 16, "Revenue", resizing)?;
workbook.set_chart_series_fill(0, anchor, 0, &navy)?;
assert!(workbook.chart_series_fill(0, anchor, 0)?.is_some());

let saved = workbook.save()?;
let saved_deck = deck.save()?;
# Ok(())
# }
```
<!-- guide-example end -->

<!-- guide-example: one_authoring_vocabulary python -->
```python
from mjx_ooxml import ChartData, ChartKind, ColorSpec, Deck, FillSpec
from mjx_ooxml import PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook

navy = FillSpec.solid(ColorSpec.srgb("1F3864"))

# On a shape in a deck.
deck = Deck.blank(SlideSize.widescreen())
slide = Surface.slide(deck.add_slide_from_layout(0))
bounds = ShapeBounds.from_inches(1.0, 1.0, 2.0, 1.0)
shape = deck.add_shape(slide, PresetShapeType.Rectangle, bounds)
deck.set_shape_fill(slide, shape, navy)
assert deck.shape_fill(slide, shape) is not None

# The same value, on a chart series in a workbook.
workbook = Workbook.blank()
chart = ChartData(ChartKind.Bar).categories(["Q1"]).series("North", [12.5])
resizing = ResizingBehavior.MoveAndResizeWithAnchorCells
anchor = workbook.add_chart(0, chart, 1, 1, 7, 16, "Revenue", resizing)
workbook.set_chart_series_fill(0, anchor, 0, navy)
assert workbook.chart_series_fill(0, anchor, 0) is not None

saved = workbook.save()
saved_deck = deck.save()
```
<!-- guide-example end -->

<!-- guide-example: one_authoring_vocabulary js -->
```js
import { ChartData, ChartKind, ColorSpec, Deck, FillSpec } from "@mjx/ooxml";
import { PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook } from "@mjx/ooxml";

const navy = FillSpec.solid(ColorSpec.srgb("1F3864"));

// On a shape in a deck.
const deck = Deck.blank(SlideSize.widescreen());
const slide = Surface.slide(deck.addSlideFromLayout(0));
const bounds = ShapeBounds.fromInches(1.0, 1.0, 2.0, 1.0);
const shape = deck.addShape(slide, PresetShapeType.Rectangle, bounds);
deck.setShapeFill(slide, shape, navy);
if (deck.shapeFill(slide, shape) === undefined) {
  throw new Error("the shape should carry the fill just set");
}

// The same value, on a chart series in a workbook.
const workbook = Workbook.blank();
const chart = new ChartData(ChartKind.Bar).categories(["Q1"]).series("North", [12.5]);
const resizing = ResizingBehavior.MoveAndResizeWithAnchorCells;
const anchor = workbook.addChart(0, chart, 1, 1, 7, 16, "Revenue", resizing);
workbook.setChartSeriesFill(0, anchor, 0, navy);
if (workbook.chartSeriesFill(0, anchor, 0) === undefined) {
  throw new Error("the series should carry the fill just set");
}

const saved = workbook.save();
const savedDeck = deck.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [navy, deck, slide, bounds, workbook, chart]) {
  handle.free();
}
```
<!-- guide-example end -->

Two packages again, and again both are compared part by part in all three languages.

One vocabulary is also what makes the two bindings possible at all: `bindings/mjx-python` and
`bindings/mjx-wasm` each wrap **this** list of types once, not three times.
