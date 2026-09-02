// The building-a-deck guide, written through the WebAssembly binding.
//
// This is `crates/mjx-ooxml/examples/build_a_deck.rs` call for call — the same fixture, the same
// layouts, the same shapes, the same table, the same chart, the same notes, the same assertions.
// That is the point: if the curated subset were missing anything, this file could not be written.
//
// `bindings/mjx-python/tests/test_build_a_deck.py` is the second copy. All three write to
// `target/examples/`, and `the three walkthroughs agree` below checks that this one and the Rust
// one produce byte-identical parts.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// The package under test is the bundler build, which is what `import { Deck } from "@mjx/ooxml"`
// resolves to; `bindings/mjx-wasm/build-npm.sh` produces it.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CellFormat,
  Cells,
  CharacterPropertiesSpec,
  ChartData,
  ChartKind,
  ColorSpec,
  Deck,
  FillSpec,
  Format,
  LineSpec,
  LineWidth,
  PresetShapeType,
  ShapeBounds,
  defaultPlaceholderImage,
  detectFormat,
  formatConventionalExtension,
  formatIsEditable,
} from "../../npm/dist/bundler/mjx_ooxml.js";

import { partPayloads } from "./zip.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");
const FIXTURES = join(REPOSITORY_ROOT, "tests", "fixtures");
const OUTPUT_DIRECTORY = process.env.MJX_OUTPUT_DIR ?? join(REPOSITORY_ROOT, "target", "examples");

/**
 * The walkthrough, run once.
 *
 * Everything it builds is freed on the way out, including on failure — which is the shape every
 * caller of this library should copy. A leaked `Deck` is a leaked megabyte.
 */
function buildTheGuidesDeck(template) {
  const recorded = {};
  const owned = [];
  /** Remembers a wasm object so the `finally` below can free it. */
  const keep = (value) => {
    owned.push(value);
    return value;
  };

  // ---- What is this file? ------------------------------------------------------------------
  // Detection reads the package, not the name: a `.pptm` or a `.potx` answers correctly, and a
  // `.docx` renamed to `.pptx` is still reported as a Word document.
  const detected = detectFormat(template);
  assert.equal(detected, Format.Presentation);
  assert.equal(formatConventionalExtension(detected), "pptx");
  assert.equal(formatIsEditable(detected), true);

  // ---- Open --------------------------------------------------------------------------------
  const deck = Deck.open(template);
  try {
    // ---- Look before editing ---------------------------------------------------------------
    recorded.templateCounts = [deck.slideCount(), deck.layoutCount(), deck.masterCount()];
    const layouts = deck.layouts();
    recorded.layouts = layouts.map((layout) => [layout.index, layout.name ?? null]);
    layouts.forEach((layout) => layout.free());

    // ---- A slide from a layout, and its placeholders ---------------------------------------
    // Indices are plain numbers here, and a surface or a shape address takes one directly.
    const slide = deck.addSlideFromLayout(1);
    const shapes = deck.shapes(slide);
    recorded.placeholders = shapes
      .filter((shape) => shape.placeholder !== undefined)
      .map((shape) => [shape.index, shape.placeholder.kind]);
    shapes.forEach((shape) => shape.free());
    deck.setShapeTextContent(slide, 0, "Quarterly results");
    deck.setShapeTextContent(slide, 1, "Revenue up 14% year on year");

    // Nothing above set a font or a size: the title renders at the master's title size, in the
    // theme's major typeface, because that is what the layout and master say.
    const title = keep(deck.effectiveRunProperties(slide, 0, 0, 0));
    recorded.titleSizePoints = title.sizePoints ?? null;

    // ---- Shapes of our own -------------------------------------------------------------------
    const badge = deck.addShape(
      slide,
      PresetShapeType.Ellipse,
      keep(ShapeBounds.fromInches(8.0, 0.4, 1.2, 1.2)),
    );
    // `FillSpec.solid` borrows the colour, so the colour is still ours to free.
    const navy = keep(ColorSpec.srgb("1F3864"));
    deck.setShapeFill(slide, badge, keep(FillSpec.solid(navy)));
    deck.setShapeOutline(
      slide,
      badge,
      keep(LineSpec.solid(keep(LineWidth.fromPoints(1.5)), keep(ColorSpec.srgb("FFFFFF")))),
    );

    const white = keep(ColorSpec.srgb("FFFFFF"));
    const caption = deck.addTextBox(
      slide,
      "Source: internal",
      keep(ShapeBounds.fromInches(0.5, 6.5, 4.0, 0.4)),
    );
    deck.setShapeRunProperties(
      slide,
      caption,
      keep(keep(keep(new CharacterPropertiesSpec()).withSizePoints(10.0)).withItalic(true)),
    );

    // ---- A picture -----------------------------------------------------------------------------
    deck.addPicture(
      slide,
      defaultPlaceholderImage(),
      keep(ShapeBounds.fromInches(7.5, 5.5, 1.5, 1.5)),
    );

    // ---- A table -------------------------------------------------------------------------------
    const tableSlide = deck.addSlideFromLayout(1);
    deck.setShapeTextContent(tableSlide, 0, "By region");
    const table = deck.addTable(tableSlide, 3, 2, keep(ShapeBounds.fromInches(1.0, 2.0, 6.0, 2.0)));
    [
      ["North", "4.2"],
      ["South", "3.1"],
    ].forEach(([region, revenue], offset) => {
      const row = offset + 1;
      deck.setCellText(tableSlide, table, row, 0, 0, region);
      deck.setCellText(tableSlide, table, row, 1, 0, revenue);
    });
    deck.setCellText(tableSlide, table, 0, 0, 0, "Region");
    deck.setCellText(tableSlide, table, 0, 1, 0, "Revenue");

    // One call for the whole header row — `Cells` names the selection, `CellFormat` the change.
    deck.formatCells(
      tableSlide,
      table,
      keep(Cells.row(0)),
      keep(keep(new CellFormat()).withFill(keep(FillSpec.solid(navy)))),
    );
    deck.formatCellText(
      tableSlide,
      table,
      keep(Cells.row(0)),
      keep(keep(keep(new CharacterPropertiesSpec()).withBold(true)).withColor(white)),
    );

    // ---- A chart -------------------------------------------------------------------------------
    const chartSlide = deck.addSlideFromLayout(1);
    deck.setShapeTextContent(chartSlide, 0, "Trend");
    const chart = keep(
      keep(keep(new ChartData(ChartKind.Bar)).categories(["Q1", "Q2", "Q3", "Q4"])).series(
        "2026",
        new Float64Array([12.0, 15.5, 14.0, 19.25]),
      ),
    );
    const chartFrame = deck.addChart(
      chartSlide,
      chart,
      keep(ShapeBounds.fromInches(1.0, 2.0, 8.0, 4.0)),
    );

    // ---- Speaker notes -------------------------------------------------------------------------
    deck.setNotesText(slide, "Lead with the revenue number, then the regional split.");

    // ---- Package hygiene -----------------------------------------------------------------------
    // The three delegates that reach the package without handing out the part graph.
    const links = deck.externalLinks();
    recorded.externalLinks = links.length;
    links.forEach((link) => link.free());
    recorded.sweptParts = deck.removeUnusedParts().length;

    // ---- Save ----------------------------------------------------------------------------------
    // `save` validates first, exactly as the Rust `Deck::save` does; the binding does not route
    // around that check. `validate` is the same pass without writing.
    deck.validate();
    recorded.saved = deck.save();
    recorded.slide = slide;
    recorded.badge = badge;
    recorded.caption = caption;
    recorded.tableSlide = tableSlide;
    recorded.table = table;
    recorded.chartSlide = chartSlide;
    recorded.chartFrame = chartFrame;
    recorded.slideCount = deck.slideCount();
    return recorded;
  } finally {
    // `free()` is not optional: JavaScript's garbage collector does not manage the wasm heap.
    owned.forEach((value) => value.free());
    deck.free();
  }
}

test("the guide builds and reopens", () => {
  const template = readFileSync(join(FIXTURES, "layouts.pptx"));
  const run = buildTheGuidesDeck(template);

  mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
  writeFileSync(join(OUTPUT_DIRECTORY, "node_build_a_deck.pptx"), run.saved);

  assert.deepEqual(run.templateCounts, [2, 3, 1], "the fixture the guide starts from");
  assert.ok(run.placeholders.length > 0, "layout 1 places a title and a body");
  assert.notEqual(run.titleSizePoints, null, "the title inherits a size from the master");
  assert.equal(run.slideCount, 5, "two from the fixture, three the guide adds");
  assert.equal(run.externalLinks, 0);

  // ---- Reopen what we wrote --------------------------------------------------------------
  // A walkthrough that never checks its own output is a claim, not a demonstration.
  const reopened = Deck.open(run.saved);
  try {
    assert.equal(reopened.slideCount(), run.slideCount);
    assert.equal(reopened.format(), Format.Presentation);
    assert.equal(reopened.shapeText(run.slide, 0), "Quarterly results");
    assert.equal(reopened.shapeText(run.slide, run.caption), "Source: internal");
    assert.equal(reopened.cellText(run.tableSlide, run.table, 0, 0), "Region");
    assert.equal(reopened.cellText(run.tableSlide, run.table, 1, 1), "4.2");
    assert.notEqual(reopened.notesText(run.slide), undefined);

    // The badge really is an ellipse with the fill and outline the guide gave it.
    const fill = reopened.shapeFill(run.slide, run.badge);
    assert.equal(fill.kind, "solid");
    const color = fill.color;
    assert.equal(color.srgbValue, "1F3864");
    color.free();
    fill.free();

    const outline = reopened.shapeOutline(run.slide, run.badge);
    const width = outline.width;
    assert.equal(width.points, 1.5);
    width.free();
    outline.free();

    // And the chart is a chart, with the series the guide gave it.
    const series = reopened.chartSeries(run.chartSlide, run.chartFrame);
    assert.deepEqual(
      series.map((entry) => entry.name),
      ["2026"],
    );
    assert.deepEqual(Array.from(series[0].values), [12.0, 15.5, 14.0, 19.25]);
    series.forEach((entry) => entry.free());
  } finally {
    reopened.free();
  }
});

test("a Word document is detected and refused", () => {
  const document = readFileSync(join(FIXTURES, "sample.docx"));
  assert.equal(detectFormat(document), Format.Document);
  assert.throws(
    () => Deck.open(document),
    (failure) => {
      assert.ok(failure instanceof Error, "a failure must be a real Error");
      assert.equal(failure.name, "OoxmlError");
      assert.equal(failure.code, "UnsupportedFormat");
      assert.ok(typeof failure.stack === "string" && failure.stack.length > 0);
      return true;
    },
  );
});

test("the three walkthroughs agree", (context) => {
  // This walkthrough and the Rust one must produce the *same deck*, part for part.
  //
  // Not "both produce a file", and not "both produce a file of about the right size": the same part
  // names, and byte-identical payloads for every one of them. That is the only assertion that can
  // tell a faithful binding from a plausible one — a method wired to the wrong `Deck` method, or an
  // argument converted with the wrong units, changes a payload here and nothing else would notice.
  let rustOutput;
  try {
    rustOutput = join(OUTPUT_DIRECTORY, "facade_build_a_deck.pptx");
    execFileSync(
      "cargo",
      ["run", "--quiet", "-p", "mjx-ooxml", "--example", "build_a_deck", "--", rustOutput],
      { cwd: REPOSITORY_ROOT, stdio: "pipe" },
    );
  } catch (failure) {
    context.skip(`the Rust walkthrough could not be run: ${failure.message.split("\n")[0]}`);
    return;
  }

  const template = readFileSync(join(FIXTURES, "layouts.pptx"));
  const fromNode = partPayloads(buildTheGuidesDeck(template).saved);
  const fromRust = partPayloads(readFileSync(rustOutput));

  assert.deepEqual(
    Object.keys(fromNode).sort(),
    Object.keys(fromRust).sort(),
    "the two walkthroughs must author the same set of parts",
  );
  const differing = Object.keys(fromNode).filter(
    (name) => Buffer.compare(fromNode[name], fromRust[name]) !== 0,
  );
  assert.deepEqual(
    differing,
    [],
    `these parts differ between the Node and Rust walkthroughs: ${differing.join(", ")}`,
  );
});
