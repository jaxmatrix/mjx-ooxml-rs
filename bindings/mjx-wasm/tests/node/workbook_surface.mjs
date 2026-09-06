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

import { CellWrite, SheetKind, Workbook } from "../../npm/dist/bundler/mjx_ooxml.js";

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
