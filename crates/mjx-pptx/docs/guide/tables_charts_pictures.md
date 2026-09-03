# Tables, charts and pictures

The three kinds of structured content a deck usually carries. Tables and charts both live inside a
`p:graphicFrame`, so [`graphic_frame_kind`](Presentation::graphic_frame_kind) is how you tell them
apart — and how you find out that a frame holds SmartArt, which this library recognises but does not
model.

## Tables

### Create and size

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::Emu;
use mjx_pptx::ShapeBounds;

let table = deck.add_table(0, 4, 3, ShapeBounds::from_inches(1.0, 1.5, 8.0, 3.0))?;
let (rows, columns) = deck.table_dimensions(0, table)?;

deck.set_column_width(0, table, 0, Emu::from_points(216.0))?;   // 3 inches
deck.set_row_height(0, table, 0, Emu::from_emu(500_000))?;
# let _ = (rows, columns);
# Ok(())
# }
```

Rows and columns can be inserted and removed after the fact —
[`insert_row`](Presentation::insert_row), [`remove_row`](Presentation::remove_row) and their column
counterparts — which fixes up merges and widths as it goes.

### Cell text

A cell holds a full text body, so the addressing goes one level deeper than a shape: `(row, column)`
then paragraph then run.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
# let table = 0usize;
deck.set_cell_text(0, table, 0, 0, 0, "Region")?;
let text = deck.cell_text(0, table, 0, 0)?;
# let _ = text;
# Ok(())
# }
```

### Formatting a selection

This is where the API earns its keep. [`Cells`] names a selection and [`CellFormat`] names what to do
to it — so styling a header row is one call, not a loop.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
# let table = 0usize;
use mjx_dml::{CellBorder, ColorSpec, FillSpec, LineSpec, LineWidth, CharacterPropertiesSpec};
use mjx_pptx::{CellFormat, Cells};

deck.format_cells(
    0,
    table,
    Cells::row(0),
    &CellFormat::new()
        .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into())))
        .with_border(CellBorder::Bottom, LineSpec::solid(LineWidth::from_points(1.0), ColorSpec::Srgb("FFFFFF".into()))),
)?;

deck.format_cell_text(
    0,
    table,
    Cells::row(0),
    &CharacterPropertiesSpec::new().with_bold(true),
)?;
# Ok(())
# }
```

`Cells::one(r, c)`, `Cells::row(r)`, `Cells::column(c)`, `Cells::rectangle(0..2, 1..3)` and
`Cells::all()` cover every shape of selection — and because every selection is a rectangle, the same
type describes a merge.

### Merging

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
# let table = 0usize;
use mjx_pptx::Cells;

deck.merge_cells(0, table, Cells::rectangle(0..1, 0..3))?;

// `(rows, columns)`, the order `table_dimensions` answers in and the order every address
// on this surface is written in.
let (row_span, column_span) = deck.cell_span(0, table, 0, 0)?;
# let _ = (row_span, column_span);
# Ok(())
# }
```

A merged region keeps its covered cells in the file — that is how OOXML models it, and it is what lets
[`unmerge_cells`](Presentation::unmerge_cells) put the grid back. Two consequences worth knowing:
[`cell_text`](Presentation::cell_text) on a covered cell returns text that nothing renders (use
[`visible_cell_text`](Presentation::visible_cell_text) when you want what a reader sees), and
[`merged_cell_anchor`](Presentation::merged_cell_anchor) maps any covered position back to the cell
that actually shows.

### Table styles

Two routes. A **style id** points at a definition in the deck's `tableStyles.xml`; an **inline style**
carries its definition on the table itself, which is self-contained and travels well.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
# let table = 0usize;
use mjx_dml::{ColorSpec, FillSpec, OnOffStyle, TableStylePart};
use mjx_pptx::{TableStyleDefinition, TableStyleFormat};

deck.set_inline_table_style(
    0,
    table,
    &TableStyleDefinition::new()
        .with_name("Report")
        .with_part(
            TableStylePart::FirstRow,
            TableStyleFormat::new()
                .with_fill(FillSpec::solid(ColorSpec::Srgb("1F3864".into())))
                .with_bold(OnOffStyle::On),
        ),
)?;
# Ok(())
# }
```

Which style parts apply to a given cell depends on its position and on the table's `a:tblPr` flags
(first row, banded rows, last column, …), toggled with
[`set_table_part`](Presentation::set_table_part). The resolution order —
corner cells beat first/last column, which beat first/last row, which beat banding, which beats the
whole-table part — is documented in [the effective-properties guide](crate::effective_properties).

## Charts

### Authoring

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::{ChartData, ChartKind, ShapeBounds};

let chart = ChartData::new(ChartKind::Line)
    .categories(["Jan", "Feb", "Mar"])
    .series("North", [3.0, 4.5, 4.0])
    .series("South", [2.0, 2.5, 3.5]);

deck.add_chart(0, &chart, ShapeBounds::from_inches(1.0, 1.5, 8.0, 4.0))?;
# Ok(())
# }
```

Every plot type `CT_PlotArea` admits can be authored and read: `Bar`, `Bar3D`, `Line`, `Line3D`,
`Pie`, `Pie3D`, `OfPie`, `Area`, `Area3D`, `Scatter`, `Doughnut`, `Radar`, `Bubble`, `Stock`,
`Surface` and `Surface3D`. `Stock` is the one with a shape requirement of its own — `CT_StockChart`
declares three or four series (open, high, low, close), and `add_chart` refuses anything else rather
than writing markup that fails validation.

### The embedded workbook

`add_chart` writes three parts, not one: the chart, a `/ppt/embeddings/Microsoft_Excel_SheetN.xlsx`
**workbook**, and the relationship binding them. The workbook is laid out to match the chart's `c:f`
formulas cell for cell — column `A` the categories, `B` onwards one per series — so PowerPoint's
*Edit Data* opens on exactly the numbers the chart draws.

[`set_chart_series_values`](Presentation::set_chart_series_values) and
[`set_chart_series_categories`](Presentation::set_chart_series_categories) refresh that workbook in
the same call, so it never goes stale.
[`refresh_chart_workbook`](Presentation::refresh_chart_workbook) does it on demand for a chart edited
some other way. The workbook is *regenerated*, not patched — a chart's embedded workbook is a
chart-private artefact whose content is the chart's data — so formatting or extra sheets a
third-party workbook carried do not survive a data edit. Detach it first
([`chart_workbooks`](Presentation::chart_workbooks),
[`detach_chart_workbook`](Presentation::detach_chart_workbook)) if you would rather keep it.

### Reading and editing an existing chart

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
for series in deck.chart_series(0, 3)? {
    println!("{:?}: {:?}", series.name, series.values);
}
deck.set_chart_series_values(0, 3, 0, &[10.0, 12.0, 11.5])?;
# Ok(())
# }
```

Editing rewrites whichever source the series names — a `c:numRef`'s cache or a `c:numLit` literal —
and that is what renders. Multi-level categories (`c:multiLvlStrRef`) read too, level by level; a
chart whose category source is numeric or multi-level answers
[`ChartSeriesNotEditable`](PptxError::ChartSeriesNotEditable) to a label rewrite rather than
inventing a cache.

### Axes, legend, title and series styling

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::{ColorSpec, FillSpec};
use mjx_pptx::{AxisOrientation, LegendPosition};

for axis in deck.chart_axes(0, 3)? {
    println!("{:?} axis, {:?} to {:?}", axis.kind, axis.minimum, axis.maximum);
}

deck.set_chart_title(0, 3, Some("Revenue by quarter"))?;
deck.set_chart_legend(0, 3, Some(LegendPosition::Bottom))?;
deck.set_chart_axis_title(0, 3, 1, Some("Millions"))?;
deck.set_chart_axis_scale(0, 3, 1, Some(0.0), Some(25.0))?;
deck.set_chart_axis_orientation(0, 3, 1, AxisOrientation::MinimumToMaximum)?;
deck.set_chart_axis_gridlines(0, 3, 1, true, false)?;
deck.set_chart_series_fill(0, 3, 0, &FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned())))?;
# Ok(())
# }
```

[`chart_axes`](Presentation::chart_axes) returns one [`ChartAxisData`] per axis in document order —
its kind, id, position, bounds, orientation, title, gridlines, tick marks and number format. A field
is `None` when the axis does not declare that setting: the axis inherits it, and the reader says so
rather than guessing what Office would draw. Passing `None` to
[`set_chart_title`](Presentation::set_chart_title) or
[`set_chart_legend`](Presentation::set_chart_legend) removes the element entirely.

An image fill is refused on a series
([`ChartFillNotSupported`](PptxError::ChartFillNotSupported)): it would name an image relationship,
and a chart part relates to no images.

### Data labels — the three tiers

Data labels are the part of a chart a reader actually reads, and `c:dLbls` is the same element at two
tiers: ECMA-376 §21.2.2.49 calls it "a root element that specifies the settings for the data labels
for an entire series **or the entire chart**". A `c:dLbl` inside a series' container overrides them
for one point. So a label's settings resolve over **three tiers**, most specific first — the point,
the series, then the plot — and [`ChartLabelScope`] is how each is addressed.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::{ChartLabelScope, DataLabelPosition, DataLabelSpec};

// Every series of the plot shows its value, outside the end of the bar.
deck.set_chart_data_labels(
    0,
    3,
    ChartLabelScope::Plot { plot_idx: 0 },
    &DataLabelSpec::new()
        .value(true)
        .position(DataLabelPosition::OutsideEnd)
        .number_format("#,##0"),
)?;

// …except this series, which shows the share of the total instead.
deck.set_chart_data_labels(
    0,
    3,
    ChartLabelScope::Series { series_idx: 1 },
    &DataLabelSpec::new().value(false).percentage(true),
)?;

// …and this one point, which is silenced entirely.
deck.suppress_chart_data_labels(
    0,
    3,
    ChartLabelScope::Point { series_idx: 1, point_idx: 2 },
)?;

// What is actually in force for one point, merged across all three tiers.
let settings = deck.chart_data_labels(0, 3, 1, Some(0))?;
println!("{:?} {:?}", settings.shows_percentage, settings.position);
# Ok(())
# }
```

The merge is **per setting**, not per tier: a series that only says "show the percentage" still takes
its plot's position and number format. A field that is still `None` in the resolved
[`DataLabelSettings`] is one no tier states, which the application fills in from the chart style —
the reader says so rather than guessing.

A [`DataLabelSpec`] states only what you set. Writing one whose sole `Some` is `value` turns the
value on and leaves the position, the separator and the format exactly as they were, so one setting
of a label Office wrote can be changed without flattening the rest. Three verbs separate what are
genuinely three different intentions:
[`set_chart_data_labels`](Presentation::set_chart_data_labels) states settings,
[`suppress_chart_data_labels`](Presentation::suppress_chart_data_labels) says *draw nothing here* (a
`c:delete`), and [`remove_chart_data_labels`](Presentation::remove_chart_data_labels) says *say
nothing here*, returning the tier to what it inherits.
[`chart_data_label_tier`](Presentation::chart_data_label_tier) reads one tier in isolation, which is
how you find out which tier a setting is coming from.

A surface plot declares no `c:dLbls` at all, and one point's label declares no leader lines — both
are refused with a typed error before anything is written, rather than emitted as markup that fails
validation.

### Per-point formatting, trendlines and error bars

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_dml::{ColorSpec, FillSpec};
use mjx_pptx::{ErrorBarType, ErrorValueType, ErrorBarSpec, TrendlineKind, TrendlineSpec};

// One column in its own colour, one slice pulled out of its pie.
deck.set_chart_point_fill(0, 3, 0, 2, &FillSpec::Solid(ColorSpec::Srgb("C00000".to_owned())))?;
deck.set_chart_point_explosion(0, 3, 0, 2, Some(25))?;

// A fitted curve, extended two categories past the last point.
deck.add_chart_trendline(
    0,
    3,
    0,
    &TrendlineSpec::new(TrendlineKind::Polynomial)
        .polynomial_order(3)
        .projection(2.0, 0.0)
        .display(true, true),
)?;

// Uncertainty, either as one figure or per point.
deck.set_chart_error_bars(
    0,
    3,
    0,
    &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::Percentage, 5.0),
)?;
deck.set_chart_error_bars(
    0,
    3,
    0,
    &ErrorBarSpec::custom(ErrorBarType::Both, vec![1.0, 2.0, 3.0], vec![0.5, 0.5, 0.5]),
)?;
# Ok(())
# }
```

**A `c:dPt` is anchored by index into its series, and nothing here ever renumbers one.** That matters
the moment a series changes length: renumbering would move one point's colour silently onto a
different point, so an edit that shortens a series leaves every `c:idx` naming exactly what it named
before. The ones that now address past the end are *reported* by
[`chart_dangling_decoration`](Presentation::chart_dangling_decoration) and removed only when you ask,
by [`drop_chart_dangling_decoration`](Presentation::drop_chart_dangling_decoration). A `c:idx` in a
file that is not a number at all addresses no point, is never matched by a lookup, and rides through
a round-trip untouched.

Writing past the end is the other half of the same rule: `set_chart_point_fill(…, 9, …)` on a
three-point series answers
[`DataPointOutOfRange`](ChartDataError::DataPointOutOfRange) rather than writing an anchor that names
nothing.

What a series may carry comes from the schema, not from a list here. `CT_PieSer` declares no
`c:trendline` and no `c:errBars`, and `CT_SurfaceSer` declares no decoration at all, so asking for
one is [`DecorationNotAllowed`](ChartDataError::DecorationNotAllowed) — and a scatter, area or bubble
series admits *two* sets of error bars, one per axis, which
[`set_chart_error_bars`](Presentation::set_chart_error_bars) keeps apart by `c:errDir`. A polynomial
order outside 2–6 and a moving-average period below 2 are refused for the same reason: `ST_Order` and
`ST_Period` do not admit them.

A chart can also label itself the moment it is authored, with no edit step:

```
use mjx_pptx::{ChartData, ChartKind, DataLabelPosition, DataLabelSpec};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("Revenue", [10.0, 20.0, 15.0])
    .data_labels(
        DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd)
            .number_format("#,##0"),
    );
assert!(chart.validate().is_ok());
```

## Pictures

### Adding

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
use mjx_pptx::ShapeBounds;

let bytes = std::fs::read("chart.png")?;
deck.add_picture(0, &bytes, ShapeBounds::from_inches(1.0, 1.0, 4.0, 3.0))?;
# Ok(())
# }
```

The format is determined by sniffing magic bytes; unrecognised bytes are rejected before anything is
edited. **Nothing is ever decoded or re-encoded** — the bytes you hand over are the bytes in the file.

[`add_image`](Presentation::add_image) is the lower half of that, when you want the media part and its
relationship id without a shape. Both deduplicate by content, so the same logo on twenty slides is one
part.

### Reading and replacing

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
if let Some(bytes) = deck.picture_image_bytes(0, 1)? {
    println!("{} bytes", bytes.len());
}
deck.set_picture_image(0, 1, &std::fs::read("new.png")?)?;
# Ok(())
# }
```

### Linked images

A picture can reference an image by URL instead of embedding it (`a:blip@r:link`). Those render as
nothing when the target is unreachable, which is a common way for a deck to arrive broken.

```no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# use mjx_pptx::Presentation;
# let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
for linked in deck.linked_images(0)? {
    println!("shape {} links to {}", linked.shape_index, linked.target);
    deck.replace_linked_image_with_placeholder(0, linked.shape_index, None)?;
}
# Ok(())
# }
```

The library performs **no network access** — it reports what is linked and lets you decide. Passing
`None` uses a built-in neutral placeholder; passing your own bytes uses those. The same
discovery-then-replace pattern covers audio and video
([`media_references`](Presentation::media_references)), OLE objects
([`ole_objects`](Presentation::ole_objects)) and chart workbooks
([`chart_workbooks`](Presentation::chart_workbooks)).
