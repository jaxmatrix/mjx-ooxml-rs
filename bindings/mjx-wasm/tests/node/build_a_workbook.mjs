// The building-a-workbook walkthrough, written through the WebAssembly binding.
//
// This is `crates/mjx-ooxml/examples/build_a_workbook.rs` call for call — the same tabs, the same
// cells in the same single batched write, the same style, the same geometry, the same hyperlink.
// `bindings/mjx-python/tests/test_build_a_workbook.py` is the second copy, and all three are
// compared against the Rust one part by part, byte for byte.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// The package under test is the bundler build; `bindings/mjx-wasm/build-npm.sh` produces it.
//
// # The one thing to notice
//
// Every cell is written in one `writeCells` call and read back in one `readRange`. That is the only
// shape this binding offers: `mjx_xlsx::Workbook` holds no parsed worksheet, so a per-cell call
// costs a whole-worksheet parse every time it is made — and a JavaScript caller has no escape hatch
// to reach the Rust answer with.

import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

import {
  BorderEdgeSpec,
  BorderSpec,
  BorderStyle,
  CellFormatSpec,
  CellFormatTarget,
  CellWrite,
  Color,
  ColorSchemeSlot,
  FontProperties,
  Format,
  PatternFillSpec,
  Workbook,
} from "../../npm/dist/bundler/mjx_ooxml.js";
import { partPayloads } from "./zip.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");
const OUTPUT_DIRECTORY = process.env.MJX_OUTPUT_DIR ?? join(REPOSITORY_ROOT, "target", "examples");
const FIXTURES = join(REPOSITORY_ROOT, "tests", "fixtures");

/** Builds the guide's workbook and returns its bytes, freeing every wasm handle it made. */
function buildTheGuidesWorkbook() {
  const owned = [];
  const keep = (value) => {
    owned.push(value);
    return value;
  };
  try {
    const workbook = keep(Workbook.blank());
    assert.equal(workbook.format(), Format.Workbook);
    assert.equal(workbook.sheetCount(), 1);

    // ---- Tabs ------------------------------------------------------------------------------
    workbook.renameSheet(0, "Summary");
    const notes = workbook.addSheet("Notes");
    assert.equal(notes, 1);
    assert.equal(workbook.sheetIndex("Summary"), 0);

    // ---- Cells: one call, every cell -----------------------------------------------------------
    // The entries are **moved into** `writeCells` — see its own documentation — so they are
    // deliberately not `keep`t for freeing here.
    workbook.writeCells(0, [
      CellWrite.sharedText("A1", "Region"),
      CellWrite.sharedText("B1", "Revenue"),
      CellWrite.sharedText("C1", "Growth"),
      CellWrite.sharedText("A2", "North America"),
      CellWrite.number("B2", 1250000),
      CellWrite.number("C2", 0.125),
      CellWrite.sharedText("A3", "Europe"),
      CellWrite.number("B3", 980000),
      CellWrite.number("C3", 0.061),
      CellWrite.sharedText("A4", "Asia Pacific"),
      CellWrite.number("B4", 1410000),
      CellWrite.number("C4", 0.198),
      CellWrite.sharedText("A5", "Audited"),
      CellWrite.boolean("B5", false),
    ]);
    workbook.writeCells(notes, [
      CellWrite.inlineText("A1", "Figures are unaudited and subject to revision."),
    ]);

    // ---- A style, built once and pointed at -----------------------------------------------------
    // Every `with…` returns a **new** value, so each intermediate is its own wasm handle.
    const white = keep(Color.fromOpaqueRgb("FFFFFF"));
    // The heading's fill is a theme slot, its text a literal — see the Rust walkthrough.
    const font = workbook.appendFont(
      keep(
        keep(keep(keep(new FontProperties()).withFontName("Calibri")).withBold(true))
          .withSizePoints(12)
          .withColor(white),
      ),
    );
    const fill = workbook.appendPatternFill(
      keep(PatternFillSpec.solidFromTheme(ColorSchemeSlot.Accent1)),
    );
    const mediumEdge = keep(BorderEdgeSpec.styled(BorderStyle.Medium));
    const border = workbook.appendBorder(keep(keep(new BorderSpec()).withBottom(mediumEdge)));
    const heading = workbook.appendCellFormat(
      CellFormatTarget.CellFormats,
      keep(
        keep(keep(CellFormatSpec.skeletonCellFormat()).withFontIndex(font))
          .withFillIndex(fill)
          .withBorderIndex(border),
      ),
    );
    for (const headingCell of ["A1", "B1", "C1"]) {
      workbook.setCellStyle(0, headingCell, heading);
    }

    // ---- Geometry ---------------------------------------------------------------------------------
    // Rows are one-based, as `row@r` is; columns are zero-based, as `A` is column 0.
    workbook.setRowHeight(0, 1, 22, true);
    workbook.setColumnWidth(0, 0, 0, 22, true);
    workbook.setColumnWidth(0, 1, 2, 14, true);

    // ---- A hyperlink -------------------------------------------------------------------------------
    workbook.setCellHyperlinkUrl(0, "A2", "https://example.org/north-america");

    // ---- Save ----------------------------------------------------------------------------------------
    workbook.validate();
    return workbook.save();
  } finally {
    for (const value of owned) {
      value.free();
    }
  }
}

test("the Excel walkthrough runs end to end through the wasm binding", () => {
  const bytes = buildTheGuidesWorkbook();
  assert.ok(bytes.length > 0);
  mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
  writeFileSync(join(OUTPUT_DIRECTORY, "wasm_build_a_workbook.xlsx"), bytes);

  const owned = [];
  const keep = (value) => {
    owned.push(value);
    return value;
  };
  try {
    const reopened = keep(Workbook.open(bytes));
    assert.equal(reopened.sheetCount(), 2);
    const sheets = reopened.sheets();
    assert.equal(sheets[0].name, "Summary");
    for (const sheet of sheets) {
      sheet.free();
    }
    assert.equal(reopened.usedRange(0), "A1:C5");

    // One read, every cell — the mirror image of the one write above.
    const block = keep(reopened.readRange(0, "A1:C5"));
    assert.equal(block.rowCount, 5);
    assert.equal(block.columnCount, 3);

    const rows = block.rows();
    assert.deepEqual(rows[0], ["Region", "Revenue", "Growth"]);
    assert.deepEqual(rows[1], ["North America", 1250000, 0.125]);
    assert.equal(rows[3][2], 0.198);
    assert.equal(rows[4][1], false);
    assert.deepEqual(block.kinds()[4], ["text", "boolean", "blank"]);

    const note = keep(reopened.readRange(1, "A1"));
    assert.deepEqual(note.rows(), [["Figures are unaudited and subject to revision."]]);

    const link = keep(reopened.cellHyperlink(0, "A2"));
    assert.equal(link.target, "https://example.org/north-america");
    const heading = keep(reopened.effectiveCellFormat(0, "B1"));
    assert.ok(heading.styleIndex > 0);
  } finally {
    for (const value of owned) {
      value.free();
    }
  }
});

test("Workbook.open refuses a presentation and a Word document by name", () => {
  for (const name of ["sample.pptx", "sample.docx"]) {
    const bytes = readFileSync(join(FIXTURES, name));
    assert.throws(
      () => Workbook.open(bytes),
      (failure) => {
        assert.ok(failure instanceof Error);
        assert.equal(failure.name, "OoxmlError");
        assert.equal(failure.code, "UnsupportedFormat");
        return true;
      },
    );
  }
});

test("a workbook opened and saved untouched is byte-identical part by part", () => {
  // A binding that quietly re-serialized a part it never touched would still produce a file Excel
  // opens, and every other assertion in this file would still pass. This is the one that would not.
  const original = readFileSync(join(FIXTURES, "sample.xlsx"));
  const workbook = Workbook.open(original);
  try {
    const saved = workbook.save();
    const before = partPayloads(original);
    const after = partPayloads(saved);
    assert.deepEqual(Object.keys(before).sort(), Object.keys(after).sort());
    for (const name of Object.keys(before)) {
      assert.deepEqual(after[name], before[name], `part ${name} changed on an untouched round trip`);
    }
  } finally {
    workbook.free();
  }
});

test("this walkthrough and the Rust one produce the same workbook, part for part", () => {
  // Not "both produce a file": the same part names, and byte-identical payloads for every one of
  // them. A method wired to the wrong `Workbook` method, or a cell written to the wrong address,
  // changes a payload here and nothing else would notice.
  const fromWasm = Buffer.from(buildTheGuidesWorkbook());
  const rustOutput = join(OUTPUT_DIRECTORY, "facade_build_a_workbook.xlsx");
  execFileSync(
    "cargo",
    ["run", "--quiet", "-p", "mjx-ooxml", "--example", "build_a_workbook", "--", rustOutput],
    { cwd: REPOSITORY_ROOT, stdio: "inherit" },
  );
  const fromRust = readFileSync(rustOutput);

  const wasmParts = partPayloads(fromWasm);
  const rustParts = partPayloads(fromRust);
  assert.deepEqual(
    Object.keys(wasmParts).sort(),
    Object.keys(rustParts).sort(),
    "the two walkthroughs must author the same set of parts",
  );
  for (const name of Object.keys(wasmParts)) {
    assert.deepEqual(wasmParts[name], rustParts[name], `part ${name} differs`);
  }
});
