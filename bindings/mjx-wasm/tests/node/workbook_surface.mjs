// The Excel sibling of `surface.mjs` and `document_surface.mjs`: mis-wiring guards for the argument
// pairs a swapped delegate could plausibly confuse, plus the corners `build_a_workbook.mjs` does not
// reach.
//
// See `crates/mjx-ooxml/tests/workbook_public_paths.rs` (the Rust original these guards mirror) for
// why every assertion here is asymmetric on purpose: a test that writes "Region" into A1 and reads
// "Region" back out of A1 passes against a surface that ignores both arguments and holds one value.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  AxisOrientation,
  CellWrite,
  ChartData,
  ChartKind,
  ChartLabelScope,
  ChartRangeSeries,
  ColorSpec,
  DataLabelSpec,
  ErrorBarSpec,
  ErrorBarType,
  ErrorValueType,
  FillSpec,
  GeometrySource,
  LegendPosition,
  LineSpec,
  LineWidth,
  ResizingBehavior,
  SheetKind,
  TrendlineKind,
  TrendlineSpec,
  Workbook,
} from "../../npm/dist/bundler/mjx_ooxml.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(resolve(HERE, "../../../.."), "tests", "fixtures");

/** A blank workbook with a 3x2 block of mixed cell kinds, written in one call. */
function filled(owned) {
  const workbook = owned.keep(Workbook.blank());
  // The entries are **moved into** `writeCells`, so they are deliberately not kept for freeing.
  workbook.writeCells(0, [
    CellWrite.sharedText("A1", "Region"),
    CellWrite.sharedText("B1", "Growth"),
    CellWrite.inlineText("A2", "North America"),
    CellWrite.number("B2", 12.5),
    CellWrite.boolean("A3", true),
    CellWrite.error("B3", "#DIV/0!"),
  ]);
  return workbook;
}

/** A scope that frees every wasm handle it was handed, whatever the test did. */
function scope(body) {
  const values = [];
  const owned = {
    keep(value) {
      values.push(value);
      return value;
    },
  };
  try {
    body(owned);
  } finally {
    for (const value of values) {
      value.free();
    }
  }
}

test("block rows and columns are not transposed", () => {
  scope((owned) => {
    const block = owned.keep(filled(owned).readRange(0, "A1:B3"));
    assert.equal(block.rowCount, 3);
    assert.equal(block.columnCount, 2);

    // Six different values at six different addresses; the two columns hold different kinds.
    assert.deepEqual(block.rows(), [
      ["Region", "Growth"],
      ["North America", 12.5],
      [true, "#DIV/0!"],
    ]);
    assert.equal(block.range, "A1:B3");
  });
});

test("a text cell and an error cell are told apart by kind", () => {
  // `rows()` cannot distinguish them — both arrive as a string — and `kinds()` is why that is fine.
  scope((owned) => {
    const block = owned.keep(filled(owned).readRange(0, "A1:B3"));
    assert.deepEqual(block.kinds(), [
      ["text", "text"],
      ["text", "number"],
      ["boolean", "error"],
    ]);
    const one = owned.keep(block.value(2, 1));
    assert.equal(one.kind, "error");
    assert.equal(one.errorCode, "#DIV/0!");
    assert.equal(one.text, undefined);
  });
});

test("an offset outside the block throws an OoxmlError naming the offsets", () => {
  scope((owned) => {
    const block = owned.keep(filled(owned).readRange(0, "A1:B3"));
    assert.throws(
      () => block.value(3, 0),
      (failure) => {
        assert.equal(failure.name, "OoxmlError");
        assert.equal(failure.code, "IndexOutOfRange");
        assert.equal(failure.detail.row, 3);
        assert.equal(failure.detail.column, 0);
        return true;
      },
    );
  });
});

test("a batch with a bad address writes nothing", () => {
  scope((owned) => {
    const workbook = filled(owned);
    const before = workbook.save();
    assert.throws(
      () =>
        workbook.writeCells(0, [
          CellWrite.number("C1", 1),
          CellWrite.number("not a cell", 2),
        ]),
      (failure) => failure.code === "InvalidArgument",
    );
    assert.deepEqual(workbook.save(), before);
  });
});

test("a merge is found from a covered cell and removed by its own reference", () => {
  scope((owned) => {
    const workbook = filled(owned);
    workbook.mergeCells(0, "A1:B1");
    assert.deepEqual(workbook.mergedRanges(0), ["A1:B1"]);
    assert.equal(workbook.mergedRangeContaining(0, "B1"), "A1:B1");
    assert.equal(workbook.mergedRangeContaining(0, "A3"), undefined);
    assert.equal(workbook.unmergeCells(0, "A1:B1"), true);
    assert.deepEqual(workbook.mergedRanges(0), []);
  });
});

test("an external link and an internal jump are not each other", () => {
  scope((owned) => {
    const workbook = filled(owned);
    workbook.setCellHyperlinkUrl(0, "A2", "https://example.org/north-america");
    workbook.setCellHyperlinkLocation(0, "A3", "Sheet1!B2");

    const external = owned.keep(workbook.cellHyperlink(0, "A2"));
    assert.equal(external.target, "https://example.org/north-america");
    assert.equal(external.location, undefined);

    const internal = owned.keep(workbook.cellHyperlink(0, "A3"));
    assert.equal(internal.location, "Sheet1!B2");
    assert.equal(internal.relationshipId, undefined, "an internal jump writes no relationship");

    assert.equal(workbook.removeCellHyperlink(0, "A2"), true);
    const remaining = workbook.sheetHyperlinks(0);
    assert.equal(remaining.length, 1);
    for (const link of remaining) {
      link.free();
    }
    assert.equal(workbook.removeCellHyperlink(0, "A2"), false);
  });
});

test("a comment writes both halves and a delete takes both away", () => {
  // Asymmetric on purpose: the two cells hold different text and different visibility, so a
  // delegate that ignored `reference` and held one comment would fail. The fixture is
  // LibreOffice's, so the producer's own two comments are there to be counted against.
  scope((owned) => {
    const workbook = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "cell_comments.xlsx"))),
    );
    const all = workbook.sheetComments(0);
    assert.equal(all.length, 2);
    for (const comment of all) {
      comment.free();
    }

    const checked = owned.keep(workbook.cellComment(0, "A2"));
    assert.equal(checked.text, "Checked against the ledger.\nSecond line.");
    assert.equal(checked.author, "Unknown Author");
    const checkedBox = owned.keep(checked.commentBox);
    assert.equal(checkedBox.isVisible, true);
    assert.equal(checkedBox.row, 1);
    assert.equal(checkedBox.column, 0);
    // Read, never inferred: the anchor is the producer's own string.
    assert.equal(checkedBox.anchorText, "1, 23, 0, 0, 2, 47, 3, 1");

    const spend = owned.keep(workbook.cellComment(0, "B1"));
    assert.equal(spend.text, "Spend is in thousands.");
    const spendBox = owned.keep(spend.commentBox);
    assert.equal(spendBox.isVisible, false);

    assert.equal(workbook.addCellComment(0, "C3", "Jai Shukla", "A fresh note."), 1025);
    const fresh = owned.keep(workbook.cellComment(0, "C3"));
    assert.equal(fresh.author, "Jai Shukla");
    assert.equal(fresh.shapeId, 1025);
    const freshBox = owned.keep(fresh.commentBox);
    assert.equal(freshBox.identifier, "_x0000_s1025");

    assert.equal(workbook.setCellCommentText(0, "C3", "Rewritten."), true);
    assert.equal(workbook.setCellCommentText(0, "Z9", "nobody"), false);
    const rewritten = owned.keep(workbook.cellComment(0, "C3"));
    assert.equal(rewritten.text, "Rewritten.");

    assert.equal(workbook.removeCellComment(0, "C3"), true);
    assert.equal(workbook.removeCellComment(0, "C3"), false);
    assert.equal(workbook.cellComment(0, "C3"), undefined);
    // Both halves went: saving would refuse if either were left behind.
    workbook.save();
  });
});

test("a form control resolves to its legacy shape and an OLE object does not", () => {
  // The `shapeId` hop, and the two lists it reads from told apart. LibreOffice's fixture lists a
  // form control and no OLE object, so the two methods must answer differently — a delegate wired
  // to the wrong list would answer the same thing twice.
  scope((owned) => {
    const workbook = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "legacy_form_control.xlsx"))),
    );
    assert.equal(workbook.vmlShapeIdForFormControl(0, 0), "AcceptTerms");
    assert.equal(workbook.vmlShapeIdForFormControl(0, 7), undefined);
    assert.equal(workbook.vmlShapeIdForOleObject(0, 0), undefined);

    const vml = workbook.sheetVmlPartBytes(0);
    assert.ok(vml instanceof Uint8Array);
    const text = new TextDecoder().decode(vml);
    assert.ok(
      text.includes('o:spid="_x0000_s1001"'),
      "the producer's own VML comes through verbatim",
    );
  });
});

test("a tab that is not there and a tab with no cells are different failures", () => {
  scope((owned) => {
    const workbook = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "print_and_sheet_kinds.xlsx"))),
    );
    const sheets = workbook.sheets();
    const others = sheets
      .map((sheet, index) => [sheet.kind, index])
      .filter(([kind]) => kind !== SheetKind.Worksheet)
      .map(([, index]) => index);
    for (const sheet of sheets) {
      sheet.free();
    }
    assert.ok(others.length > 0, "the fixture must hold a tab that is not a worksheet");

    assert.throws(
      () => workbook.readRange(others[0], "A1"),
      (failure) => {
        assert.equal(failure.code, "NothingToRead");
        assert.equal(failure.detail.index, others[0]);
        return true;
      },
    );
    assert.throws(
      () => workbook.readRange(workbook.sheetCount(), "A1"),
      (failure) => failure.code === "IndexOutOfRange",
    );
  });
});

test("the feature reports answer from files that hold the features", () => {
  scope((owned) => {
    const conditional = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "conditional_formatting.xlsx"))),
    );
    assert.ok(conditional.conditionalFormattingRanges(0).length > 0);
    assert.ok(conditional.conditionalFormattingRuleCount(0, 0) > 0);

    const filters = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "validation_and_filters.xlsx"))),
    );
    assert.notEqual(filters.autoFilterRange(0), undefined);
    assert.ok(filters.dataValidationRanges(0).length > 0);

    const tables = owned.keep(Workbook.open(readFileSync(join(FIXTURES, "worksheet_tables.xlsx"))));
    const sheetTables = tables.sheetTables(0);
    assert.ok(sheetTables.length > 0);
    for (const table of sheetTables) {
      const columns = table.columns;
      assert.ok(columns.length > 0);
      for (const column of columns) {
        column.free();
      }
      table.free();
    }

    const preserved = owned.keep(Workbook.open(readFileSync(join(FIXTURES, "preserved_parts.xlsx"))));
    const summary = owned.keep(preserved.preservedParts());
    assert.equal(summary.isEmpty, false);
    for (const entry of summary.all()) {
      assert.ok(entry.part.length > 0);
      entry.free();
    }
  });
});

test("the workbook metadata and part graph are reachable", () => {
  scope((owned) => {
    const workbook = owned.keep(Workbook.open(readFileSync(join(FIXTURES, "sample.xlsx"))));

    for (const name of workbook.definedNames()) {
      name.free();
    }
    assert.equal(workbook.definedName("Nothing"), undefined);
    workbook.printArea(0);
    workbook.dateSystem();
    owned.keep(workbook.calculationSettings());
    for (const view of workbook.windowViews()) {
      view.free();
    }
    workbook.activeSheet();
    workbook.sheetPrinterSettings(0);
    workbook.sheetBackgroundImage(0);
    for (const anomaly of workbook.gridAnomalies(0)) {
      anomaly.free();
    }
    workbook.nextTableId();
    workbook.tableStyleOrigin("TableStyleMedium2");
    for (const list of [
      workbook.pivotTables(),
      workbook.externalLinks(),
      workbook.connections(),
      workbook.queryTables(),
    ]) {
      for (const entry of list) {
        entry.free();
      }
    }
    const maps = workbook.xmlMaps();
    if (maps !== undefined) {
      maps.free();
    }
    owned.keep(workbook.revisionState());

    assert.ok(workbook.workbookPart().endsWith("workbook.xml"));
    assert.ok(workbook.partNames().length > 1);
    assert.notEqual(workbook.contentTypeOf(workbook.workbookPart()), undefined);
    assert.ok(workbook.partBytes(workbook.workbookPart()).length > 0);
    assert.throws(
      () => workbook.partBytes("/xl/nothing.xml"),
      (failure) => failure.code === "NotFound",
    );

    workbook.validate();
  });
});

// A 1x1 PNG — the smallest thing the image sniffer calls a PNG.
const PNG = new Uint8Array([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
  0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
  0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
  0x44, 0xae, 0x42, 0x60, 0x82,
]);

test("the three anchor modes are not each other", () => {
  scope((owned) => {
    const workbook = filled(owned);
    // Every number differs from every other, so a wrapper that crossed a column with a row, or a
    // `from` offset with a `to` offset, fails here rather than in markup nobody reads.
    const two = workbook.addTwoCellAnchoredPicture(
      0, PNG, "two-cell", 1, 190500, 2, 47625, 3, 95250, 5, 19050,
      ResizingBehavior.MoveWithCellsButDoNotResize,
    );
    const one = workbook.addOneCellAnchoredPicture(
      0, PNG, "one-cell", 4, 76200, 1, 38100, 914400, 457200,
    );
    const absolute = workbook.addAbsoluteAnchoredPicture(
      0, PNG, "absolute", 1905000, 952500, 685800, 342900,
    );
    assert.deepEqual([two, one, absolute], [0, 1, 2]);

    const drawing = owned.keep(workbook.sheetDrawing(0));
    assert.ok(drawing.part.endsWith("drawing1.xml"));
    const objects = drawing.objects;
    assert.deepEqual(
      objects.map((object) => object.anchor),
      ["twoCellAnchor", "oneCellAnchor", "absoluteAnchor"],
    );
    assert.deepEqual(
      objects.map((object) => object.name),
      ["two-cell", "one-cell", "absolute"],
    );
    // The `@editAs` the caller asked for, read back off the file — and the two anchors that carry
    // none answer from what they are, so the three do not agree.
    assert.deepEqual(
      objects.map((object) => object.resizing),
      [
        ResizingBehavior.MoveWithCellsButDoNotResize,
        ResizingBehavior.MoveWithCellsButDoNotResize,
        ResizingBehavior.DoNotMoveOrResizeWithRowsOrColumns,
      ],
    );
    // One image, stored once, shown by all three.
    assert.deepEqual(
      [...new Set(objects.map((object) => object.image))],
      ["/xl/media/image1.png"],
    );
    assert.ok(objects.every((object) => object.printsWithSheet));
    for (const object of objects) {
      object.free();
    }

    workbook.save();
  });
});

test("an anchor a blank sheet cannot place and one it can are different answers", () => {
  scope((owned) => {
    const workbook = filled(owned);
    workbook.addOneCellAnchoredPicture(0, PNG, "one-cell", 4, 76200, 1, 38100, 914400, 457200);
    workbook.addAbsoluteAnchoredPicture(0, PNG, "absolute", 1905000, 952500, 685800, 342900);

    // `Workbook.blank` writes no `x:sheetFormatPr`, so `@defaultRowHeight` is stated nowhere and a
    // cell-anchored object cannot be placed. The honest answer is `undefined`, not Excel's own 15.
    assert.equal(workbook.sheetAnchorBounds(0, 0, 7.0, 96.0), undefined);

    // The absolute anchor names no cell, so it is placeable on the very same sheet.
    const bounds = owned.keep(workbook.sheetAnchorBounds(0, 1, 7.0, 96.0));
    assert.equal(bounds.xEmu, 1905000n);
    assert.equal(bounds.yEmu, 952500n);
    assert.equal(bounds.widthEmu, 685800n);
    assert.equal(bounds.heightEmu, 342900n);
    assert.equal(bounds.rowSource, GeometrySource.Stated);
    assert.equal(bounds.columnSource, GeometrySource.Stated);
    assert.equal(bounds.maximumDigitWidthPixels, 7.0);
    assert.equal(bounds.pixelsPerInch, 96.0);
  });
});

test("a producer drawing resolves and the three modes shift differently", () => {
  scope((owned) => {
    const workbook = owned.keep(
      Workbook.open(readFileSync(join(FIXTURES, "worksheet_drawings.xlsx"))),
    );

    // The extent Apache POI computed for the same column widths, reached through the binding alone.
    const bounds = owned.keep(workbook.sheetAnchorBounds(0, 0, 7.0, 96.0));
    assert.equal(bounds.widthEmu, 2085975n);
    assert.equal(bounds.heightEmu, 885825n);
    assert.equal(bounds.rowSource, GeometrySource.SheetDefault);
    assert.equal(bounds.columnSource, GeometrySource.Stated);

    const report = workbook.insertRowsIntoDrawing(0, 0, 3);
    assert.equal(report.length, 4);
    assert.ok(report[0].moved && !report[0].resized);
    assert.ok(report[2].moved && !report[2].resized);
    assert.ok(!report[3].moved && !report[3].resized);
    assert.equal(report[3].promise, ResizingBehavior.DoNotMoveOrResizeWithRowsOrColumns);
    assert.ok(report.every((shift) => shift.promiseKept));
    for (const shift of report) {
      shift.free();
    }

    // A row inserted *inside* the first anchor resizes it against its own `@editAs`, which the
    // report says rather than silently leaving wrong.
    const inside = workbook.insertRowsIntoDrawing(0, 7, 1);
    assert.ok(inside[0].resized && !inside[0].promiseKept);
    assert.ok(inside[1].promiseKept);
    for (const shift of inside) {
      shift.free();
    }

    for (const shifted of [
      workbook.removeRowsFromDrawing(0, 0, 1),
      workbook.insertColumnsIntoDrawing(0, 0, 1),
      workbook.removeColumnsFromDrawing(0, 0, 1),
    ]) {
      assert.equal(shifted.length, 4);
      for (const shift of shifted) {
        shift.free();
      }
    }

    assert.equal(workbook.removeSheetDrawingObject(0, 3), true);
    assert.equal(workbook.removeSheetDrawingObject(0, 9), false);
    const drawing = owned.keep(workbook.sheetDrawing(0));
    const objects = drawing.objects;
    assert.equal(objects.length, 3);
    for (const object of objects) {
      object.free();
    }
    workbook.save();
  });
});

test("bytes that are not an image are refused", () => {
  scope((owned) => {
    const workbook = filled(owned);
    assert.throws(
      () => workbook.addAbsoluteAnchoredPicture(0, new Uint8Array([1, 2, 3]), "nope", 0, 0, 1, 1),
      (failure) => failure.code === "InvalidArgument",
    );
    assert.equal(workbook.sheetDrawing(0), undefined);
  });
});

// ---------------------------------------------------------------------------------------------
// Charts (MJXOFF-111) — the TypeScript half of A10's parity rule, from the third surface
// ---------------------------------------------------------------------------------------------

/** The two-series chart every chart case below adds. */
function sampleChart() {
  return new ChartData(ChartKind.Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", new Float64Array([12.5, 18.0, 21.5]))
    .series("South", new Float64Array([9.0, 11.5, 14.0]));
}

test("the whole Excel chart family is bound and reads back", () => {
  // MJXOFF-80's own gate, from the third surface: the same names, taking the same vocabulary. What
  // differs is only the address — `(sheet, anchor)` where a document takes a drawing id — and every
  // assertion reads back the value it just set, so a method wired to the wrong delegate answers
  // something else rather than succeeding quietly.
  scope((owned) => {
    const workbook = filled(owned);
    const anchor = workbook.addChart(
      0,
      sampleChart(),
      4,
      1,
      10,
      16,
      "Revenue",
      ResizingBehavior.MoveWithCellsButDoNotResize,
    );
    assert.deepEqual(Array.from(workbook.chartAnchorIndices(0)), [anchor]);
    assert.notEqual(workbook.chartRelId(0, anchor), undefined);
    assert.notEqual(workbook.chartPartBytes(0, anchor), undefined);
    assert.deepEqual(Array.from(workbook.chartKinds(0, anchor)), [ChartKind.Bar]);

    const series = workbook.chartSeries(0, anchor);
    assert.equal(series.length, 2);
    assert.equal(series[0].name, "North");
    assert.deepEqual(Array.from(series[1].values), [9.0, 11.5, 14.0]);
    for (const entry of series) entry.free();

    const workbooks = workbook.chartWorkbooks();
    assert.equal(workbooks.length, 1);
    assert.equal(workbooks[0].sheet, 0);
    assert.equal(workbooks[0].anchor, anchor);
    assert.equal(workbooks[0].external, false);
    for (const entry of workbooks) entry.free();
    assert.equal(workbook.refreshChartWorkbook(0, anchor), true);

    workbook.setChartTitle(0, anchor, "Regional revenue");
    assert.equal(workbook.chartTitle(0, anchor), "Regional revenue");

    workbook.setChartLegend(0, anchor, LegendPosition.Right);
    const legend = workbook.chartLegend(0, anchor);
    assert.equal(legend.position, LegendPosition.Right);
    legend.free();

    workbook.setChartSeriesValues(0, anchor, 0, new Float64Array([40, 41, 42]));
    const rewritten = workbook.chartSeries(0, anchor);
    assert.deepEqual(Array.from(rewritten[0].values), [40, 41, 42]);
    for (const entry of rewritten) entry.free();

    workbook.setChartSeriesCategories(0, anchor, 1, ["A", "B", "C"]);
    const recategorised = workbook.chartSeries(0, anchor);
    assert.deepEqual(Array.from(recategorised[1].categories), ["A", "B", "C"]);
    for (const entry of recategorised) entry.free();

    workbook.setChartAxisScale(0, anchor, 1, 0, 50);
    workbook.setChartAxisTitle(0, anchor, 1, "Millions");
    workbook.setChartAxisGridlines(0, anchor, 1, true, false);
    workbook.setChartAxisOrientation(0, anchor, 1, AxisOrientation.MaximumToMinimum);
    const axes = workbook.chartAxes(0, anchor);
    assert.equal(axes[1].minimum, 0);
    assert.equal(axes[1].maximum, 50);
    assert.equal(axes[1].title, "Millions");
    for (const axis of axes) axis.free();

    const blue = FillSpec.solid(ColorSpec.srgb("1F77B4"));
    workbook.setChartSeriesFill(0, anchor, 0, blue);
    const readFill = workbook.chartSeriesFill(0, anchor, 0);
    assert.equal(readFill.kind, "solid");
    readFill.free();
    const outline = LineSpec.solid(LineWidth.fromPoints(1), ColorSpec.srgb("000000"));
    workbook.setChartSeriesLine(0, anchor, 0, outline);

    const seriesScope = ChartLabelScope.series(0);
    workbook.setChartDataLabels(0, anchor, seriesScope, new DataLabelSpec().value(true));
    const inForce = workbook.chartDataLabels(0, anchor, 0, undefined);
    assert.equal(inForce.showsValue, true);
    inForce.free();
    const tier = workbook.chartDataLabelTier(0, anchor, seriesScope);
    assert.equal(tier.showsValue, true);
    tier.free();
    assert.equal(workbook.chartPointLabelText(0, anchor, 0, 0), undefined);

    workbook.setChartPointFill(0, anchor, 0, 1, blue);
    workbook.setChartPointExplosion(0, anchor, 0, 1, 25);
    workbook.setChartPointLine(0, anchor, 0, 1, outline);
    const formats = workbook.chartPointFormats(0, anchor, 0);
    assert.equal(formats.length, 1);
    assert.equal(formats[0].explosion, 25);
    for (const entry of formats) entry.free();

    workbook.addChartTrendline(0, anchor, 0, new TrendlineSpec(TrendlineKind.Linear));
    const trendlines = workbook.chartTrendlines(0, anchor, 0);
    assert.equal(trendlines.length, 1);
    for (const entry of trendlines) entry.free();
    workbook.setChartTrendline(0, anchor, 0, 0, new TrendlineSpec(TrendlineKind.Logarithmic));
    assert.equal(workbook.removeChartTrendlines(0, anchor, 0), 1);

    workbook.setChartErrorBars(
      0,
      anchor,
      0,
      ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 1.5),
    );
    const bars = workbook.chartErrorBars(0, anchor, 0);
    assert.equal(bars.length, 1);
    for (const entry of bars) entry.free();
    assert.equal(workbook.removeChartErrorBars(0, anchor, 0), 1);

    assert.equal(workbook.chartDanglingDecoration(0, anchor, 0).length, 0);
    assert.equal(workbook.dropChartDanglingDecoration(0, anchor, 0), 0);
    assert.equal(workbook.removeChartPointFormat(0, anchor, 0, 1), true);
    workbook.suppressChartDataLabels(0, anchor, ChartLabelScope.series(1));
    assert.equal(workbook.removeChartDataLabels(0, anchor, seriesScope), true);
    assert.equal(workbook.chartStyleId(0, anchor), undefined);

    workbook.detachChartWorkbook(0, anchor);
    assert.equal(workbook.chartWorkbooks().length, 0);
    assert.equal(workbook.refreshChartWorkbook(0, anchor), false);
    workbook.save();

    blue.free();
    outline.free();
    seriesScope.free();
  });
});

test("a chart over a live range reads the cells and reports a stale cache", () => {
  // The half of the chart family only this surface has, and the trap MJXOFF-111 names: the cache
  // and the cells are made to **disagree** before anything is asserted about which one a reader
  // answered. A chart whose cached values equalled its cell values would let a reader wired to
  // either source pass.
  scope((owned) => {
    const workbook = filled(owned);
    workbook.writeCells(0, [CellWrite.number("B4", 7.25), CellWrite.number("B5", 9.5)]);
    const anchor = workbook.addRangeChart(
      0,
      ChartKind.Line,
      "Sheet1!$A$1:$A$3",
      [new ChartRangeSeries("Growth", "Sheet1!$B$4:$B$5").namedByCell("Sheet1!$B$1")],
      4,
      1,
      10,
      16,
      "Live",
      ResizingBehavior.MoveWithCellsButDoNotResize,
    );

    // No embedded workbook, and asking for one does not make one.
    assert.equal(workbook.refreshChartWorkbook(0, anchor), false);
    assert.equal(workbook.chartWorkbooks().length, 0);

    // The series took its name from the header cell, and its caches were seeded from the cells.
    const series = workbook.chartSeries(0, anchor);
    assert.equal(series[0].name, "Growth");
    assert.deepEqual(Array.from(series[0].values), [7.25, 9.5]);
    for (const entry of series) entry.free();

    const references = workbook.chartSeriesReferences(0, anchor);
    assert.equal(references[0].values, "Sheet1!$B$4:$B$5");
    assert.equal(references[0].name, "Sheet1!$B$1");
    for (const entry of references) entry.free();

    // The resolver, reached directly. A blank cell is absent rather than zero, and an error code is
    // reported as itself.
    const resolved = owned.keep(workbook.resolveRangeReference(0, "Sheet1!$B$2:$B$5"));
    assert.equal(resolved.fullyResolved, true);
    assert.equal(resolved.problem, undefined);
    assert.equal(resolved.addressedCells, 4n);
    const cells = resolved.cells;
    assert.deepEqual(
      cells.map((cell) => cell.reference),
      ["B2", "B3", "B4", "B5"],
    );
    assert.equal(cells[0].number, 12.5);
    assert.equal(cells[1].number, undefined);
    assert.equal(cells[1].label, "#DIV/0!");
    for (const cell of cells) cell.free();

    const missing = owned.keep(workbook.resolveRangeReference(0, "Ghost!$A$1"));
    assert.equal(missing.fullyResolved, false);
    assert.ok(missing.problem.includes("Ghost"));

    // Writing a cell leaves the cache alone — and the freshness report is how a caller finds out.
    const before = workbook.chartSeriesFreshness(0, anchor);
    assert.equal(before[0].valuesAgree, true);
    for (const entry of before) entry.free();

    workbook.writeCells(0, [CellWrite.number("B4", 99.0)]);
    const stillCached = workbook.chartSeries(0, anchor);
    assert.deepEqual(Array.from(stillCached[0].values), [7.25, 9.5]);
    for (const entry of stillCached) entry.free();

    const after = workbook.chartSeriesFreshness(0, anchor);
    assert.equal(after[0].valuesAgree, false);
    const cached = after[0].cached;
    const fromCells = after[0].fromCells;
    assert.deepEqual(Array.from(cached.values), [7.25, 9.5]);
    assert.deepEqual(Array.from(fromCells.values), [99.0, 9.5]);
    cached.free();
    fromCells.free();
    for (const entry of after) entry.free();

    const readFromCells = workbook.chartSeriesFromCells(0, anchor);
    assert.deepEqual(Array.from(readFromCells[0].values), [99.0, 9.5]);
    for (const entry of readFromCells) entry.free();

    // …and the opt-in repair brings the two back into step.
    assert.equal(workbook.refreshChartCacheFromCells(0, anchor), 1);
    const repaired = workbook.chartSeries(0, anchor);
    assert.deepEqual(Array.from(repaired[0].values), [99.0, 9.5]);
    for (const entry of repaired) entry.free();
    workbook.save();
  });
});

test("removing an Excel chart binding is caught by this suite", () => {
  // The parity clause's own guard: **remove one binding and this case goes red.** A clause nothing
  // checks quietly stops being true, so this names the bindings explicitly rather than trusting
  // that some other case would have called them. Delete `Workbook::chart_anchor_indices` from
  // `bindings/mjx-wasm/src/workbook.rs` and this fails before any chart case above runs.
  const workbook = Workbook.blank();
  try {
    for (const method of [
      "chartAnchorIndices",
      "chartRelId",
      "chartPartBytes",
      "addChart",
      "addRangeChart",
      "chartSeries",
      "chartKinds",
      "chartAxes",
      "chartTitle",
      "chartLegend",
      "chartWorkbooks",
      "refreshChartWorkbook",
      "detachChartWorkbook",
      "chartSeriesReferences",
      "chartSeriesFromCells",
      "chartSeriesFreshness",
      "refreshChartCacheFromCells",
      "resolveRangeReference",
      "setChartSeriesValues",
      "setChartDataLabels",
      "addChartTrendline",
      "setChartErrorBars",
      "dropChartDanglingDecoration",
    ]) {
      assert.equal(typeof workbook[method], "function", `Workbook.${method} is not bound`);
    }
  } finally {
    workbook.free();
  }
});
