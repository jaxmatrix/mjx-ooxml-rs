# Authoring a chart

[`ChartData`](crate::ChartData) is a description of a chart, in plain Rust, with no interner and no
package behind it. It builds by chaining, validates on demand, and turns into the bytes of a
`c:chartSpace` part.

```rust
use mjx_chart::{ChartData, ChartKind, LegendPosition};

let chart = ChartData::new(ChartKind::Bar)
    .title("Quarterly revenue")
    .legend(LegendPosition::Bottom)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [10.0, 12.0, 14.0])
    .series("South", [8.0, 9.0, 11.0]);

chart.validate().expect("two series over three categories is a legal bar chart");
assert_eq!(chart.category_count(), 3);
assert_eq!(chart.series_names().collect::<Vec<_>>(), ["North", "South"]);

let part = chart.to_part_bytes();
assert!(part.starts_with(b"<?xml"));
```

Any of the sixteen [`ChartKind`](crate::ChartKind)s can be written, not only the six with typed
scalars of their own.

## `validate` is the whole refusal surface, and it runs before anything is written

[`ChartDataError`](crate::ChartDataError) has nine variants and every one is a property of the
description rather than of a file:

| Variant | What it means |
|---|---|
| [`NoData`](crate::ChartDataError::NoData) | no series, or every series empty |
| [`SeriesCount`](crate::ChartDataError::SeriesCount) | the plot type admits a different number — a `c:stockChart` needs four |
| [`DecorationNotAllowed`](crate::ChartDataError::DecorationNotAllowed) | the series type declares no such child (see [Axes, titles and decoration](axes_titles_and_decoration)) |
| [`DataPointOutOfRange`](crate::ChartDataError::DataPointOutOfRange) | a per-point edit named a point the series does not have |
| [`SettingNotAtThisTier`](crate::ChartDataError::SettingNotAtThisTier) | `c:showLeaderLines` asked of one point's `c:dLbl`, which only the container form admits |
| [`TrendlineOrderOutOfRange`](crate::ChartDataError::TrendlineOrderOutOfRange) | outside `ST_Order`'s 2–6 |
| [`TrendlinePeriodOutOfRange`](crate::ChartDataError::TrendlinePeriodOutOfRange) | below `ST_Period`'s 2 |
| [`NonFiniteMeasure`](crate::ChartDataError::NonFiniteMeasure) | a `NaN` or an infinity, which `xsd:double` has no OOXML spelling for |
| [`CustomErrorBarsNeedValues`](crate::ChartDataError::CustomErrorBarsNeedValues) | `cust` error bars with neither `c:plus` nor `c:minus` |

Each of them is a piece of markup that would have failed schema validation, refused at the point the
caller can still do something about it.

## Two ways a chart says where its data lives, and they are different charts

A chart's `c:f` formulas name cells. **Which workbook those cells are in is the difference between
the two ways to author one**, and choosing is the first decision, not a detail:

### Its own embedded workbook — what a chart on a slide or in a document does

[`embedded_workbook_for_chart_data`](crate::embedded_workbook_for_chart_data) writes a real `.xlsx`
package holding the chart's numbers, laid out on the grid the synthesized `c:f` formulas name:
series names across the header row, categories down column `A`, values from `B2`. That package is
what PowerPoint's **Edit Data** opens, and
[`to_part_bytes_linking_workbook`](crate::ChartData::to_part_bytes_linking_workbook) writes the
`c:externalData` that points at it.

```rust
use mjx_chart::{embedded_workbook_for_chart_data, ChartData, ChartKind};

# fn main() -> Result<(), mjx_sml::SmlError> {
let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2"])
    .series("Revenue", [10.0, 20.0]);
let workbook = embedded_workbook_for_chart_data(&chart)?;
assert_eq!(&workbook[..2], b"PK", "a workbook is a ZIP package of its own");
# Ok(())
# }
```

**A chart does not have to have one.** A chart with no workbook renders perfectly, because the caches
are what draw; what it loses is Edit Data having anything to show.

### A host workbook's own cells — what a chart on a worksheet does

[`ChartData::ranges`](crate::ChartData::ranges) takes [`ChartRanges`](crate::ChartRanges) and
[`ChartSeriesRange`](crate::ChartSeriesRange) and points the `c:f` formulas at cells in the sheets the
chart lives among. There is then no embedded copy at all: the worksheet is the source, and Excel
recalculates the chart from it.

Resolving such a reference against a package is `mjx-xlsx`'s job, because only a package knows which
sheet is which. **This crate writes the reference text and reads it back, and has never resolved one
in its life** — which is also why [`ChartSeriesReferences`](crate::ChartSeriesReferences) hands back
`c:f` as text.

## The workbook is written by `mjx-sml`, and there is exactly one SpreadsheetML writer

`mjx_chart::embedding` knows one thing — *which cell a chart's data belongs in*. Every cell, part,
content type and relationship of the package it hands back comes from
[`mjx_sml::write::WorkbookPackage`](mjx_sml::write::WorkbookPackage), and
`mjx_chart::author` reaches the same crate for `column_letters` so that a chart's `c:f` and its
workbook's cells cannot disagree about which column is which.

Until MJXOFF-99 this crate carried a minimal spreadsheet writer of its own, because `mjx-sml` did not
exist and `mjx-chart → mjx-xlsx` would have been an **upward** edge (2.2 → 3.0) the layering forbids.
`mjx-sml` is rank 2.1, so `mjx-chart → mjx-sml` points down; the duplicate was deleted and **exactly
one SpreadsheetML writer ships**. `xtask/tests/layering.rs` checks the edge rather than trusting the
sentence.

## The chart part is not the whole job

Authoring a chart into a real file means a part, a content type, a relationship from the frame that
draws it, the embedded workbook and *its* relationship — and a theme for the chart's colours to
resolve against. All of that belongs to the format crate: `mjx_pptx::Presentation::add_chart`,
`mjx_docx::Document::add_chart` and `mjx_xlsx::Workbook::add_chart` each build the part graph and call
in here for the markup. A caller wanting a chart in a file should be there, not here.
