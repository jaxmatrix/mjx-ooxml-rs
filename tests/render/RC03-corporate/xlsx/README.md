# RC03 — the corporate workbook (`tests/fixtures/corporate.xlsx`)

One worksheet shaped like a report somebody would actually send: a theme; a five-column table styled
`TableStyleMedium2` with one header row; accounting and date formats in narrow columns; a colour
scale, a data bar and an icon set; a rich-text cell; a chart; a picture and a text-box shape in one
drawing; a cell wearing diagonal and dashed borders; gridlines on; and the pane frozen below the
header row.

**How it is authored.** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_authored.rs`
builds it with this workspace's own writers and nothing else — no file was copied from
`tests/office-authored/` or `tests/office-exports/`, which agents may not fill. Re-derive it with:

```sh
MJX_AUTHOR_FIXTURES=1 cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_authored
```

That suite re-authors the workbook on every run and fails if the committed bytes differ, so the
fixture cannot drift from the code that claims to author it.

Four elements are **spliced as markup** rather than written by a typed writer, because this
workspace has a reader for each and no writer: the two custom `numFmt` codes, the `sheetViews`
carrying the frozen pane and the gridline flag, the rich-text cell's runs, and the `xdr:sp` text
box. The cell formats that *point at* the two number formats are written through the typed writer,
so only the format codes themselves are markup.

**Proved by** `crates/mjx-reference-pack/tests/the_corporate_fixtures_are_corporate.rs` and
`crates/mjx-reference-pack/tests/an_acceptance_render_of_xlsx.rs`.

**Run**:

```sh
cargo test -p mjx-reference-pack --test the_corporate_fixtures_are_corporate
cargo test -p mjx-reference-pack --test an_acceptance_render_of_xlsx -- --ignored
```

The plate is written to `target/rc03-acceptance/xlsx.png`, which is git-ignored.

## Decision D9 — one folder per test, a LibreOffice reference for this run

Each render test owns its folder, holding a synthetic input and the reference for **this** run, which
is LibreOffice's. LibreOffice is a change detector and **not** parity: whether these pixels look like
Excel's is a question only a person sitting against real Microsoft Office on Windows answers, and
that sitting re-runs these same cases later (`docs/validation/07-the-reference-pack.md`).
`reference/windows.png` stays empty until it happens.

## What the render loses today

The acceptance journey pins the whole loss vector. Today the band reaches pixels with four anchored
objects framed but not laid out — the chart, the picture and the text box among them (RC28) — one
thing the reader dropped, and three values the grid approximated. Five labelled placeholders are
drawn, and no draw falls back to stand-in geometry: a worksheet issues no outline handle at all.

## Checklist coverage

| features.json row | fixture element |
| --- | --- |
| xlsx-sheet-reaches-pixels | the first band of the sheet, carried to a display list and rasterised by the acceptance journey |
| xlsx-theme-colours | the workbook's own `xl/theme/theme1.xml`, which the palette resolves scheme colours against |
| xlsx-table-styles | the worksheet table, named `TableStyleMedium2`, with one header row |
| xlsx-pictures | the two-cell anchored picture in `xl/drawings/drawing1.xml` |
| xlsx-shapes-and-text-boxes | the `xdr:sp` text box anchored beside that picture |
| xlsx-rich-text-runs | the rich-text cell, whose two runs state their own `rPr` |
| xlsx-width-dependent-number-formats | the accounting format in a narrow column, where the code's width decides what shows |
| xlsx-localised-dates | the `[$-409]d\-mmm\-yy` date format on the booked column |
| xlsx-data-bars-and-icon-sets | the data bar over the revenue column and the three-arrow icon set over the score column |
| xlsx-diagonal-and-dashed-borders | the bordered cell's `diagonal` edge and its two `dashed` sides |
| xlsx-gridlines | the sheet view's `showGridLines` |
| xlsx-sheet-view-flags | the same `sheetView`, with its selection and active pane |
| xlsx-frozen-pane-drawings | the pane frozen below the header row, with the drawing anchored across it |
| xlsx-charts | the bar chart's `c:chartSpace` part and its embedded workbook |
| xlsx-render-losses | the journey's pinned loss vector and its five labelled placeholders |
| shared-chart-model | the chart's categories and one series, as `ChartData` wrote them |
| shared-chart-series-marks | the same series, whose bars the chart engine will place |
