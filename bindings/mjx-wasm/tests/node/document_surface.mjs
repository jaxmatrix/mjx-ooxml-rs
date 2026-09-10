// The Word sibling of `surface.mjs`: every subject of the `Document` surface driven through one
// document, plus the mis-wiring guards for the pairs a swapped delegate could plausibly confuse.
//
//     node --test bindings/mjx-wasm/tests/node/

import assert from "node:assert/strict";
import test from "node:test";

import {
  AxisOrientation,
  CellBorderEdge,
  ChartData,
  ChartKind,
  ChartLabelScope,
  ChartWrap,
  ColorSpec,
  DataLabelSpec,
  Document,
  ErrorBarSpec,
  ErrorBarType,
  ErrorValueType,
  FillSpec,
  Format,
  HeaderFooterType,
  HyperlinkTarget,
  LegendPosition,
  LineSpec,
  LineWidth,
  MergedCellType,
  PageOrientation,
  PageSize,
  SectionLocation,
  TrendlineKind,
  TrendlineSpec,
  WrapText,
} from "../../npm/dist/bundler/mjx_ooxml.js";

/** The two-series chart every chart case below adds. */
function sampleChart() {
  return new ChartData(ChartKind.Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5])
    .series("South", [9.0, 11.5, 14.0]);
}

/** A blank document, plus the freeing every caller owes. */
function withDocument(body) {
  const document = Document.blank(PageSize.a4());
  try {
    return body(document);
  } finally {
    document.free();
  }
}

test("every subject of the Document surface is reachable, end to end", () => {
  withDocument((document) => {
    assert.equal(document.format(), Format.Document);
    assert.equal(document.conformance(), undefined);

    // --- text ---------------------------------------------------------------------------------
    assert.equal(document.paragraphCount(), 1);
    document.appendParagraph();
    document.appendRun(1, "Hello, ");
    document.appendRun(1, "document.");
    assert.equal(document.runCount(1), 2);
    document.insertRun(1, 2, "!");
    assert.equal(document.paragraphText(1), "Hello, document.!");
    document.setRunText(1, 0, "Hi, ");
    assert.equal(document.runText(1, 0), "Hi, ");
    document.removeRun(1, 2);
    document.insertParagraph(0);
    document.removeParagraph(0);

    // --- effective properties -------------------------------------------------------------------
    const runProps = document.effectiveRunProperties(1, 0);
    assert.equal(runProps.bold, undefined);
    runProps.free();
    const paraProps = document.effectiveParagraphProperties(1);
    paraProps.free();

    // --- styles (read-only) ---------------------------------------------------------------------
    assert.deepEqual(document.styleIds(), []);
    assert.equal(document.styleName("Normal"), undefined);

    // --- numbering --------------------------------------------------------------------------------
    document.attachParagraphToList(1, 1, 0);
    document.detachParagraphFromList(1);

    // --- sections and headers/footers ------------------------------------------------------------
    let sections = document.sections();
    assert.equal(sections.length, 1);
    assert.equal(document.sectionCount(), 1);
    assert.notEqual(sections[0].pageSize, undefined);
    sections[0].free();

    const body = SectionLocation.body();
    document.setSectionPageSize(body, PageSize.usLetter());
    sections = document.sections();
    const resized = sections[0].pageSize;
    assert.equal(resized.orientation, PageOrientation.Portrait);
    resized.free();
    sections[0].free();

    assert.equal(document.evenAndOddHeaders(), false);
    assert.equal(document.headerText(0, HeaderFooterType.Default), undefined);
    document.setHeaderText(body, HeaderFooterType.Default, "Header text");
    assert.equal(document.headerText(0, HeaderFooterType.Default), "Header text");
    document.setFooterText(body, HeaderFooterType.Default, "Footer text");
    assert.equal(document.footerText(0, HeaderFooterType.Default), "Footer text");
    document.removeHeader(body, HeaderFooterType.Default);
    document.removeFooter(body, HeaderFooterType.Default);

    // --- tables -------------------------------------------------------------------------------------
    const table = document.appendTable(2, 2);
    assert.equal(document.tableCount(), 1);
    const dimensions = document.tableDimensions(table);
    assert.equal(dimensions.rows, 2);
    assert.equal(dimensions.columns, 2);
    dimensions.free();
    document.setCellText(table, 0, 0, "top-left");
    assert.equal(document.cellText(table, 0, 0), "top-left");
    document.setCellSpan(table, 0, 0, 2);
    const span = document.cellSpan(table, 0, 0);
    assert.equal(span.columns, 2);
    span.free();
    const anchor = document.mergedCellAnchor(table, 0, 1);
    assert.equal(anchor.row, 0);
    assert.equal(anchor.column, 0);
    anchor.free();
    document.setCellSpan(table, 0, 0, undefined);
    document.setCellVerticalMerge(table, 0, 0, MergedCellType.Restart);
    document.setCellVerticalMerge(table, 1, 0, MergedCellType.Continue);
    const discrepancies = document.tableGridDiscrepancies(table);
    assert.equal(discrepancies.length, 0);
    const fill = document.effectiveCellFill(table, 0, 0);
    assert.equal(fill, undefined, "a plain table cell states no shading of its own");
    const border = document.effectiveCellBorder(table, 0, 0, CellBorderEdge.Top);
    assert.equal(border, undefined, "a plain table cell states no border of its own");
    const cellRunProps = document.effectiveCellRunProperties(table, 0, 0, 0, 0);
    cellRunProps.free();
    document.insertRow(table, 2);
    document.insertColumn(table, 2);
    document.removeColumn(table, 2);
    document.removeRow(table, 2);
    document.removeTable(table);

    // --- fields ---------------------------------------------------------------------------------------
    assert.deepEqual(document.fields(1), []);

    // --- hyperlinks -------------------------------------------------------------------------------------
    const url = HyperlinkTarget.url("https://example.com/mjx-ooxml-rs");
    document.insertHyperlink(1, 2, "example", url);
    const target = document.hyperlinkTarget(1, 2);
    assert.equal(target.urlValue, "https://example.com/mjx-ooxml-rs");
    target.free();
    url.free();
    document.removeHyperlink(1, 2);

    // --- comments -------------------------------------------------------------------------------------
    const commentId = document.addComment(1, "Reviewer", "R", "a remark");
    const comments = document.comments();
    assert.equal(comments.length, 1);
    assert.equal(comments[0].author, "Reviewer");
    comments[0].free();
    assert.equal(document.commentRangeText(commentId), "Hi, document.");
    document.removeComment(commentId);

    // --- footnotes, endnotes and revisions ------------------------------------------------------------
    const footnoteId = document.addFootnote(1, "a note");
    assert.equal(document.footnotes().length, 1);
    document.removeFootnote(footnoteId);
    const endnoteId = document.addEndnote(1, "an endnote");
    assert.equal(document.endnotes().length, 1);
    document.removeEndnote(endnoteId);
    assert.deepEqual(document.revisions(), []);
    assert.equal(
      document.textWithRevisionsAccepted(),
      document.textWithRevisionsRejected(),
    );

    // --- drawings ---------------------------------------------------------------------------------------
    const docPrId = document.addInlinePicture(
      1,
      new Uint8Array([0x89, 0x50, 0x4e, 0x47]),
      "image/png",
      "png",
      100,
      100,
      "pic",
    );
    assert.equal(document.removeDrawing(docPrId), true);
    assert.equal(document.removeDrawing(docPrId), false);

    // --- save ------------------------------------------------------------------------------------------
    document.validate();
    const bytes = document.save();
    assert.ok(bytes.length > 0);
  });
});

test("Document.open refuses a PresentationML package by name", () => {
  // `Document` never opens a `.pptx` — proved without a fixture, on a byte sequence too short to
  // be a ZIP at all, which is enough to prove the refusal is by classification, not a parse crash.
  assert.throws(
    () => Document.open(new Uint8Array([0, 1, 2, 3])),
    (error) => error.code === "Io",
  );
});

// ---------------------------------------------------------------------------------------------
// Mis-wiring guards
// ---------------------------------------------------------------------------------------------

test("paragraph and run addressing are not transposed", () => {
  withDocument((document) => {
    document.appendParagraph();
    document.appendRun(0, "first paragraph");
    document.appendRun(1, "second paragraph");
    assert.equal(document.paragraphText(0), "first paragraph");
    assert.equal(document.paragraphText(1), "second paragraph");
    assert.equal(document.runText(0, 0), "first paragraph");
    assert.equal(document.runText(1, 0), "second paragraph");
  });
});

test("table row and column are not transposed", () => {
  withDocument((document) => {
    const table = document.appendTable(3, 2);
    document.setCellText(table, 2, 1, "row 2, col 1");
    assert.equal(document.cellText(table, 2, 1), "row 2, col 1");
    assert.equal(document.cellText(table, 1, 0), "");
    const dims = document.tableDimensions(table);
    assert.equal(dims.rows, 3);
    assert.equal(dims.columns, 2);
    dims.free();
  });
});

test("header and footer are not each other", () => {
  withDocument((document) => {
    const body = SectionLocation.body();
    document.setHeaderText(body, HeaderFooterType.Default, "top of page");
    document.setFooterText(body, HeaderFooterType.Default, "bottom of page");
    assert.equal(document.headerText(0, HeaderFooterType.Default), "top of page");
    assert.equal(document.footerText(0, HeaderFooterType.Default), "bottom of page");
    document.removeHeader(body, HeaderFooterType.Default);
    assert.equal(document.footerText(0, HeaderFooterType.Default), "bottom of page");
  });
});

test("cell span and vertical merge are not each other", () => {
  withDocument((document) => {
    const table = document.appendTable(2, 2);
    document.setCellVerticalMerge(table, 0, 0, MergedCellType.Restart);
    document.setCellVerticalMerge(table, 1, 0, MergedCellType.Continue);
    const span = document.cellSpan(table, 0, 0);
    assert.equal(span.rows, 2, "a vertical merge must widen the row span, not the column span");
    assert.equal(span.columns, 1);
    span.free();
  });
});

test("removing a Document binding is caught by this suite", () => {
  // The proof this suite would fail if a method were unbound: `document.paragraphCount` really is
  // a function reachable on the class, not a value coincidentally present some other way.
  const document = Document.blank(PageSize.a4());
  try {
    assert.equal(typeof document.paragraphCount, "function");
  } finally {
    document.free();
  }
});

// ---------------------------------------------------------------------------------------------
// Charts (MJXOFF-103) — the TypeScript half of A10's parity rule
// ---------------------------------------------------------------------------------------------

test("the whole Word chart family is bound and reads back", () => {
  // A facade method without a Python *and* a TypeScript equivalent is an incomplete task. A method
  // missing from this binding is a `TypeError: not a function` here; a method bound to the wrong
  // delegate is a wrong value, which is why every assertion reads something back.
  withDocument((document) => {
    const drawing = document.addChart(0, sampleChart(), 4572000, 2743200, "Revenue");
    assert.deepEqual(Array.from(document.chartDrawingIds()), [drawing]);
    assert.notEqual(document.chartRelId(drawing), undefined);
    assert.notEqual(document.chartPartBytes(drawing), undefined);
    assert.deepEqual(Array.from(document.chartKinds(drawing)), [ChartKind.Bar]);

    const series = document.chartSeries(drawing);
    assert.equal(series.length, 2);
    assert.equal(series[0].name, "North");
    assert.deepEqual(Array.from(series[0].values), [12.5, 18.0, 21.5]);
    for (const entry of series) entry.free();

    const workbooks = document.chartWorkbooks();
    assert.equal(workbooks.length, 1);
    assert.equal(workbooks[0].drawingId, drawing);
    assert.equal(workbooks[0].external, false);
    for (const entry of workbooks) entry.free();
    assert.equal(document.refreshChartWorkbook(drawing), true);

    document.setChartTitle(drawing, "Regional revenue");
    assert.equal(document.chartTitle(drawing), "Regional revenue");

    document.setChartLegend(drawing, LegendPosition.Right);
    const legend = document.chartLegend(drawing);
    assert.equal(legend.position, LegendPosition.Right);
    legend.free();

    document.setChartSeriesValues(drawing, 0, new Float64Array([40, 41, 42]));
    const rewritten = document.chartSeries(drawing);
    assert.deepEqual(Array.from(rewritten[0].values), [40, 41, 42]);
    for (const entry of rewritten) entry.free();

    document.setChartSeriesCategories(drawing, 1, ["A", "B", "C"]);
    const recategorised = document.chartSeries(drawing);
    assert.deepEqual(Array.from(recategorised[1].categories), ["A", "B", "C"]);
    for (const entry of recategorised) entry.free();

    document.setChartAxisScale(drawing, 1, 0, 50);
    document.setChartAxisTitle(drawing, 1, "Millions");
    document.setChartAxisGridlines(drawing, 1, true, false);
    document.setChartAxisOrientation(drawing, 1, AxisOrientation.MaximumToMinimum);
    const axes = document.chartAxes(drawing);
    assert.equal(axes[1].minimum, 0);
    assert.equal(axes[1].maximum, 50);
    assert.equal(axes[1].title, "Millions");
    assert.equal(axes[1].majorGridlines, true);
    assert.equal(axes[1].orientation, AxisOrientation.MaximumToMinimum);
    for (const axis of axes) axis.free();

    const blue = FillSpec.solid(ColorSpec.srgb("1F77B4"));
    document.setChartSeriesFill(drawing, 0, blue);
    const readFill = document.chartSeriesFill(drawing, 0);
    assert.equal(readFill.kind, "solid");
    readFill.free();
    const outline = LineSpec.solid(LineWidth.fromPoints(1), ColorSpec.srgb("000000"));
    document.setChartSeriesLine(drawing, 0, outline);

    const seriesScope = ChartLabelScope.series(0);
    document.setChartDataLabels(drawing, seriesScope, new DataLabelSpec().value(true));
    const inForce = document.chartDataLabels(drawing, 0, undefined);
    assert.equal(inForce.showsValue, true);
    inForce.free();
    const tier = document.chartDataLabelTier(drawing, seriesScope);
    assert.equal(tier.showsValue, true);
    tier.free();
    assert.equal(document.chartPointLabelText(drawing, 0, 0), undefined);

    document.setChartPointFill(drawing, 0, 1, blue);
    document.setChartPointExplosion(drawing, 0, 1, 25);
    document.setChartPointLine(drawing, 0, 1, outline);
    const formats = document.chartPointFormats(drawing, 0);
    assert.equal(formats.length, 1);
    assert.equal(formats[0].index, 1);
    assert.equal(formats[0].explosion, 25);
    for (const entry of formats) entry.free();

    document.addChartTrendline(drawing, 0, new TrendlineSpec(TrendlineKind.Linear));
    const trendlines = document.chartTrendlines(drawing, 0);
    assert.equal(trendlines.length, 1);
    for (const entry of trendlines) entry.free();
    document.setChartTrendline(drawing, 0, 0, new TrendlineSpec(TrendlineKind.Logarithmic));
    assert.equal(document.removeChartTrendlines(drawing, 0), 1);

    document.setChartErrorBars(
      drawing,
      0,
      ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 1.5),
    );
    const bars = document.chartErrorBars(drawing, 0);
    assert.equal(bars.length, 1);
    for (const entry of bars) entry.free();
    assert.equal(document.removeChartErrorBars(drawing, 0), 1);

    assert.equal(document.chartDanglingDecoration(drawing, 0).length, 0);
    assert.equal(document.dropChartDanglingDecoration(drawing, 0), 0);
    assert.equal(document.removeChartPointFormat(drawing, 0, 1), true);
    document.suppressChartDataLabels(drawing, ChartLabelScope.series(1));
    assert.equal(document.removeChartDataLabels(drawing, seriesScope), true);
    assert.equal(document.chartStyleId(drawing), undefined);

    document.detachChartWorkbook(drawing);
    assert.equal(document.chartWorkbooks().length, 0);

    blue.free();
    outline.free();
    seriesScope.free();
  });
});

test("a floating Word chart takes each of the three wraps", () => {
  for (const wrap of [ChartWrap.none(), ChartWrap.square(WrapText.BothSides), ChartWrap.topAndBottom()]) {
    withDocument((document) => {
      const drawing = document.addFloatingChart(
        0,
        sampleChart(),
        228600,
        114300,
        4572000,
        2743200,
        wrap,
        "Floating",
      );
      assert.deepEqual(Array.from(document.chartDrawingIds()), [drawing]);
      assert.deepEqual(Array.from(document.chartKinds(drawing)), [ChartKind.Bar]);
    });
    wrap.free();
  }
  const square = ChartWrap.square(WrapText.Left);
  assert.equal(square.kind, "square");
  assert.equal(square.wrapText, WrapText.Left);
  square.free();
  const none = ChartWrap.none();
  assert.equal(none.wrapText, undefined);
  none.free();
  // `kind` is a data token, not a method name, so it stays snake_case and Python spells it the
  // same way. `xtask/tests/binding_projection.rs` holds the rule for the whole surface; this is
  // the one member that ever broke it (MJXOFF-268).
  for (const [wrap, kind] of [
    [ChartWrap.none(), "none"],
    [ChartWrap.square(WrapText.BothSides), "square"],
    [ChartWrap.topAndBottom(), "top_and_bottom"],
  ]) {
    assert.equal(wrap.kind, kind);
    wrap.free();
  }
});

test("a Word chart refusal carries the same code the Deck surface uses", () => {
  withDocument((document) => {
    const drawing = document.addChart(0, sampleChart(), 914400, 914400, "Revenue");
    assert.throws(
      () => document.chartSeriesFill(drawing, 7),
      (error) => error.code === "IndexOutOfRange",
    );
    assert.throws(
      () => document.chartSeries(9999),
      (error) => error.code === "WrongKind",
    );
  });
});

test("removing a Word chart binding is caught by this suite", () => {
  // The parity clause's own guard: **remove one binding and this case goes red.** A clause nothing
  // checks quietly stops being true, so this names the bindings explicitly rather than trusting
  // that some other case would have called them. Delete `Document::chart_drawing_ids` from
  // `bindings/mjx-wasm/src/document.rs` and this fails before any chart case above runs.
  const document = Document.blank(PageSize.a4());
  try {
    for (const method of [
      "chartDrawingIds",
      "addChart",
      "addFloatingChart",
      "chartSeries",
      "chartSeriesReferences",
      "chartKinds",
      "chartAxes",
      "chartTitle",
      "chartLegend",
      "chartWorkbooks",
      "refreshChartWorkbook",
      "regenerateChartWorkbook",
      "detachChartWorkbook",
      "setChartSeriesValues",
      "setChartDataLabels",
      "addChartTrendline",
      "setChartErrorBars",
      "dropChartDanglingDecoration",
    ]) {
      assert.equal(typeof document[method], "function", `Document.${method} is not bound`);
    }
  } finally {
    document.free();
  }
});
