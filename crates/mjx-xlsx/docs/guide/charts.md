# Charts on a sheet

A chart is the same part in every format this library reads: `c:chartSpace`, modelled once by
`mjx-chart`. What differs is only *how the part is reached* — a `p:graphicFrame` on a slide, a
`w:drawing` in a run, and here an `xdr:graphicFrame` on the sheet's own drawing part. So every method
below has a namesake on [`Deck`](https://docs.rs/mjx-ooxml) and `Document` that takes the same
vocabulary and answers the same thing. The address is the only difference: `(sheet, anchor)`, where
`anchor` is the object's position in the drawing part — the same address
[`Workbook::sheet_drawing`] already reports and
[`Workbook::remove_sheet_drawing_object`] already takes.

## …and one thing that exists nowhere else

Everywhere else in this library a chart's data is a **cache plus an embedded workbook**: the part
carries a `c:externalData` naming a whole `.xlsx` inside the package, and the caches beside each
`c:f` are what draws.

A chart on a worksheet usually has neither. Its `c:numRef`/`c:strRef` carry a `c:f` naming a **live
range in the sheets the chart already lives among**, and the cells *are* the source:

```xml
<c:val>
  <c:numRef>
    <c:f>Data!$B$2:$B$4</c:f>            <!-- where the numbers live -->
    <c:numCache>                          <!-- what was drawn last -->
      <c:ptCount val="3"/>
      <c:pt idx="0"><c:v>10</c:v></c:pt>
      …
```

Two sources, and they can disagree. This page is mostly about that.

## Reading a chart

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("../../tests/fixtures/chart_in_sheet.xlsx")?;
let mut workbook = mjx_xlsx::Workbook::open(&bytes)?;

// The anchors that frame a chart, in paint order.
assert_eq!(workbook.chart_anchor_indices(0)?, vec![0]);

// What the chart draws — from its caches.
let series = workbook.chart_series(0, 0)?;
assert_eq!(series[0].name.as_deref(), Some("Revenue"));
assert_eq!(series[0].values, [10.0, 20.0, 30.0]);

// Where it says that data lives — the `c:f`, exactly as the producer wrote it.
let references = workbook.chart_series_references(0, 0)?;
assert_eq!(references[0].values.as_deref(), Some("Data!$B$2:$B$4"));
# Ok(())
# }
```

## Cache versus cells: both are reported, and each is named

[`Workbook::chart_series`] answers from the caches. [`Workbook::chart_series_from_cells`] answers
from the cells. [`Workbook::chart_series_freshness`] answers with both, and says whether they agree.

**Neither is silently preferred, and the reason is that neither is wrong.** The cache is what a
consumer draws *until it recalculates*; the cells are what it would recalculate *from*. A library
that answered only the cache would hide a chart that has gone stale, and one that answered only the
cells would claim numbers no consumer has drawn yet.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
// This fixture's caches and cells disagree on every point: LibreOffice wrote the chart over
// `North/South/East = 10/20/30`, and the sheet was then given `Alpha/Beta/Gamma = 111/222/333`.
let bytes = std::fs::read("../../tests/fixtures/chart_stale_cache.xlsx")?;
let mut workbook = mjx_xlsx::Workbook::open(&bytes)?;

let freshness = workbook.chart_series_freshness(0, 0)?;
assert_eq!(freshness[0].cached.values, [10.0, 20.0, 30.0]);      // what draws
assert_eq!(freshness[0].from_cells.values, [111.0, 222.0, 333.0]); // what the sheet says
assert_eq!(freshness[0].values_agree, Some(false));
# Ok(())
# }
```

`values_agree` has **three** answers, not two. `None` means *cannot say*: the series' values are a
literal (there are no cells behind them at all), or its reference names something this workbook does
not have. "The cells disagree" and "there are no cells" are different facts, and a caller acting on
the first must not be told it by the second — the reason is on
[`Workbook::chart_series_freshness`].

### Writing a cell leaves the cache alone

This library recalculates nothing — a formula is text, and so is a chart's `c:f`. So
[`Workbook::set_cell_value`] touches no chart, exactly as adding a picture writes no cell. A chart
whose range you have just edited is **stale**, and `chart_series_freshness` is how you find out.

If you want the sheet's answer to win, ask for it:
[`Workbook::refresh_chart_cache_from_cells`] rewrites the caches from the cells and answers how many
series changed. It is the exact counterpart of [`Workbook::refresh_chart_workbook`] pointing the
other way — that one makes the workbook say what the chart draws; this one makes the chart draw what
the sheet says. Nothing calls either for you.

## Resolving a reference is not evaluating a formula

[`Workbook::resolve_range_reference`] takes any reference of the shape a `c:f` has and answers with
the cells it reaches. It handles what Excel writes: a quoted sheet name, absolute markers, a
multi-area union, a 3-D span, and a defined name (followed through to the range it stands for,
sheet-scoped names winning over workbook-scoped ones as §18.2.6 says).

It does **not** evaluate anything. Working out that `Data!$B$2:$B$4` names three cells is address
arithmetic; working out what `=SUM(B2:B4)` comes to is a calculation engine, and this project does
not have one and will not grow one. A cell holding a formula answers with its **cached value**,
which is what [`Workbook::cell_text`] does too.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let bytes = std::fs::read("../../tests/fixtures/chart_in_sheet.xlsx")?;
let mut workbook = mjx_xlsx::Workbook::open(&bytes)?;

let resolved = workbook.resolve_range_reference(0, "Data!$B$2:$B$4")?;
assert!(resolved.is_fully_resolved());
assert_eq!(resolved.addressed_cells, 3);
assert_eq!(resolved.numbers(3), [Some(10.0), Some(20.0), Some(30.0)]);

// A reference this workbook cannot satisfy is a *report*, not an error: a chart whose second
// series points at a deleted sheet still has a first series worth reading.
let ghost = workbook.resolve_range_reference(0, "NoSuchSheet!$A$1")?;
assert!(!ghost.is_fully_resolved());
assert!(ghost.problem().is_some());
# Ok(())
# }
```

**A blank cell is absent, not zero.** `ResolvedRange::cells` carries only the cells that hold
something, each with the `offset` it sits at within the reference — the same shape a `c:numCache`
has, which writes a `c:pt` for the points it has and omits the rest. That is what lets a cache and a
resolution be compared index for index without padding either, and it is why a reference naming a
whole column costs what its populated cells cost rather than what it addresses.

### What it costs

Two things are true of every resolution, and `examples/chart_range_cost.rs` measures both with a
counting allocator rather than describing them:

* **each sheet is parsed at most once per call**, however many series or areas name it;
* **the walk is `min(rows the range spans, rows the sheet has)`** — a three-row range on a
  300,000-cell sheet looks three rows up by number, and a whole-column reference walks the sheet's
  populated rows instead of a million addressable ones.

On a 30,000-cell sheet the example measures a three-cell range at **2,109 bytes** beyond the sheet's
own read, and a three-thousand-cell one at a megabyte: the figure tracks the range, not the sheet.
The sheet's own read is a different cost and a known one — see
[the large-workbooks page](large_workbooks).

## Authoring: two calls, because there are two kinds of chart

[`Workbook::add_chart`] takes the same `ChartData` a slide and a document take and writes the
**embedded workbook** beside it. A caller who built a chart description for a deck adds it to a
workbook unchanged; the chart's `c:f` name that workbook, exactly as they do in a `.pptx`.

[`Workbook::add_range_chart`] writes the case Excel itself writes: the `c:f` name **cells in this
workbook**, the caches are seeded from what those cells say right now, and **no embedded workbook is
written at all**.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_chart::ChartKind;
use mjx_dml::spreadsheet_drawing::CellMarker;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::{SheetChartSeries, SheetChartSource};

let mut workbook = mjx_xlsx::Workbook::blank()?;
workbook.rename_sheet(0, "Data")?;
for (address, value) in [("A1", 10.0), ("A2", 20.0), ("A3", 30.0)] {
    workbook.set_cell_value(0, CellReference::parse(address)?, CellValue::Number(value))?;
}

let anchor = workbook.add_range_chart(
    0,
    ChartKind::Line,
    &SheetChartSource {
        categories: None,
        series: vec![SheetChartSeries {
            name_cell: None,
            name: "Revenue".to_owned(),
            values: "Data!$A$1:$A$3".to_owned(),
        }],
    },
    CellMarker::new(2, 0, 1, 0),
    CellMarker::new(8, 0, 15, 0),
    "Revenue",
    ResizingBehavior::MoveWithCellsButDoNotResize,
)?;

// Seeded from the cells…
assert_eq!(workbook.chart_series(0, anchor)?[0].values, [10.0, 20.0, 30.0]);
// …and no workbook was written, so there is none to refresh.
assert!(!workbook.refresh_chart_workbook(0, anchor)?);
# Ok(())
# }
```

Every reference is resolved **before anything is written**, so a range naming a sheet this workbook
does not have is refused rather than written as a `c:f` pointing at nothing. Reading is the opposite
and deliberately so: a chart already in a file whose `c:f` names a deleted tab is a fact about that
file, reported rather than raised.

## Removing one

A chart frame is an anchored object like any other, so
[`Workbook::remove_sheet_drawing_object`] removes it — the same call that removes a picture, taking
the index `add_chart` returned. The chart part stays in the package; sweep it with
[`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if you want it
gone.

[`Workbook::detach_chart_workbook`] is different, and it is the one place this surface's behaviour
differs from Word's: it removes the embedded workbook part as well as the reference, **unless another
chart still names it**. That is forced rather than chosen — [`Workbook::save`] runs
`Package::validate`, which refuses a package holding a SpreadsheetML part no relationship chain
reaches, and an embedded chart workbook is exactly such a part once detached. Leaving it behind would
hand back a workbook this library then declines to write.

## What is not here

* **Pivot charts** and the pivot cache. `sml.xsd`'s pivot cluster is preserved rather than modelled
  — see [the fidelity page](fidelity_and_the_part_graph) for why.
* **Evaluating anything.** See above, and
  [formulas and cached values](formulas_and_cached_values) for the same decision one door along.
* **A chart sheet's chart.** A `chartsheet` is one of the four sheet kinds
  ([that page](print_setup_and_sheet_kinds)); its chart hangs off the chartsheet part rather than off
  a worksheet drawing, and this surface addresses a chart by `(sheet, anchor)` on a worksheet's
  drawing part.
