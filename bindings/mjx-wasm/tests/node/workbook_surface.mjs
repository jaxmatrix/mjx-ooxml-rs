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
  CellWrite,
  GeometrySource,
  ResizingBehavior,
  SheetKind,
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
