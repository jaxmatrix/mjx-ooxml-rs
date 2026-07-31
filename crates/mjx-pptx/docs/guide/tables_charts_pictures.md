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

// Note the order: `cell_span` answers `(columns, rows)`, unlike `table_dimensions`, which
// answers `(rows, columns)`.
let (column_span, row_span) = deck.cell_span(0, table, 0, 0)?;
# let _ = (column_span, row_span);
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

Six kinds are authored: `Bar`, `Line`, `Pie`, `Area`, `Scatter`, `Doughnut`.

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

Editing rewrites the chart's **cache** — `c:numCache` and `c:strCache` — which is what actually
renders. A chart whose plot type this library does not model reads as an empty series list rather than
failing; it still round-trips untouched.

Two limits to plan around, both on [the gaps page](fidelity_and_gaps): an authored chart has no
embedded workbook, and editing an existing chart leaves any workbook it *does* have stale.

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
