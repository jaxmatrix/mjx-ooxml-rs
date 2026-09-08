// The parts of the binding the walkthrough does not reach: addressing, errors, `free()`, the
// enumerations, and the mis-wiring guards.
//
//     node --test bindings/mjx-wasm/tests/node/

import assert from "node:assert/strict";
import test from "node:test";

import { partPayloads } from "./zip.mjs";

import {
  CellBorder,
  ChartData,
  ChartKind,
  ColorSpec,
  ColorTransform,
  ColorTransformKind,
  Deck,
  Emu,
  FillSpec,
  Format,
  Fraction,
  Angle,
  Geometry,
  GuideContext,
  Hyperlink,
  LineSpec,
  LineWidth,
  PresetShapeType,
  SchemeColor,
  ShapeBounds,
  ShapeGeometry,
  ShapeKind,
  ShapePath,
  SlideSize,
  Surface,
  TextUnderline,
  formatContentType,
  formatFamily,
  FormatFamily,
  version,
} from "../../npm/dist/bundler/mjx_ooxml.js";

/** A blank deck with one slide, plus the freeing every caller owes. */
function withDeck(body) {
  const size = SlideSize.widescreen();
  const deck = Deck.blank(size);
  size.free();
  try {
    deck.addSlide();
    return body(deck);
  } finally {
    deck.free();
  }
}

/** A `ShapeBounds`, used and freed. */
function bounds(x, y, width, height, body) {
  const value = ShapeBounds.fromInches(x, y, width, height);
  try {
    return body(value);
  } finally {
    value.free();
  }
}

test("the package states its version", () => {
  assert.match(version(), /^\d+\.\d+\.\d+$/);
});

test("every spelling of an address reaches the same shape, and none is consumed", () => {
  withDeck((deck) => {
    const first = bounds(1, 1, 1, 1, (b) => deck.addShape(0, PresetShapeType.Ellipse, b));
    const second = bounds(3, 1, 1, 1, (b) => deck.addShape(0, PresetShapeType.Ellipse, b));
    const group = deck.groupShapes(0, [first, second]);
    try {
      assert.equal(deck.shapeKind(0, group.indices[0]), ShapeKind.GroupShape);
      assert.equal(deck.shapeKind(0, [group.indices[0]]), ShapeKind.GroupShape);
      assert.equal(deck.shapeKind(0, group), ShapeKind.GroupShape);
      // …and again, with the *same* `ShapePath` object: an argument conversion that consumed it
      // would have freed it on the line above, and this line would throw.
      assert.equal(deck.shapeKind(0, group), ShapeKind.GroupShape);
      assert.equal(deck.shapeMemberCount(0, group), 2);
      assert.equal(deck.shapeKind(0, [group.indices[0], 0]), ShapeKind.Shape);
    } finally {
      group.free();
    }

    const surface = Surface.slide(0);
    try {
      assert.equal(deck.shapeCount(surface), deck.shapeCount(0));
      assert.equal(deck.shapeCount(surface), deck.shapeCount(0), "reusable, so not consumed");
      assert.equal(surface.kind, "slide");
      assert.equal(surface.index, 0);
    } finally {
      surface.free();
    }

    const master = Surface.master(0);
    try {
      assert.ok(deck.shapeCount(master) >= 0);
      assert.equal(master.isMasterLike, true);
    } finally {
      master.free();
    }
  });
});

test("a shape path knows where it is", () => {
  const top = ShapePath.top(2);
  const member = ShapePath.of(new Uint32Array([2, 1]));
  const child = top.child(1);
  const parent = member.parent;
  try {
    assert.deepEqual(Array.from(top.indices), [2]);
    assert.deepEqual(Array.from(member.indices), [2, 1]);
    assert.equal(top.depth, 1);
    assert.equal(member.depth, 2);
    assert.equal(top.isTopLevel, true);
    assert.equal(member.isTopLevel, false);
    assert.equal(child.equals(member), true);
    assert.equal(parent.equals(top), true);
    assert.equal(top.parent, undefined);
    assert.equal(String(top), "2");
    assert.equal(String(member), "[2, 1]");
  } finally {
    [top, member, child, parent].forEach((value) => value.free());
  }
});

test("an address of the wrong shape is refused with a message that names what is wanted", () => {
  withDeck((deck) => {
    assert.throws(() => deck.shapeCount("nonsense"), /surface/);
    assert.throws(() => deck.shapeCount(1.5), /whole number/);
    assert.throws(() => deck.shapeCount(-1), /whole number/);
    assert.throws(() => deck.shapeKind(0, {}), /shape address/);
    assert.throws(() => deck.shapeKind(0, []), /at least one index/);
    assert.throws(() => deck.shapeKind(0, ["a"]), /whole, non-negative/);
  });
});

test("a failure is a real Error with a code and coordinates", () => {
  withDeck((deck) => {
    assert.throws(
      () => deck.shapeText(0, [4, 2]),
      (failure) => {
        assert.ok(failure instanceof Error);
        assert.equal(failure.name, "OoxmlError");
        assert.equal(failure.code, "IndexOutOfRange");
        assert.equal(failure.detail.surface.kind, "slide");
        assert.deepEqual(Array.from(failure.detail.shape.indices), [4, 2]);
        assert.equal(failure.detail.row, undefined, "an unused coordinate is simply absent");
        assert.ok(failure.message.length > 0);
        failure.detail.surface.free();
        failure.detail.shape.free();
        return true;
      },
    );

    const table = bounds(1, 1, 4, 2, (b) => deck.addTable(0, 2, 2, b));
    assert.throws(
      () => deck.cellText(0, table, 5, 9),
      (failure) => {
        assert.equal(failure.code, "IndexOutOfRange");
        assert.equal(failure.detail.row, 5);
        assert.equal(failure.detail.column, 9);
        return true;
      },
    );

    // A different code for a different mistake, on the same deck.
    const shape = bounds(1, 1, 2, 2, (b) => deck.addShape(0, PresetShapeType.Rectangle, b));
    assert.throws(
      () => deck.tableDimensions(0, shape),
      (failure) => failure.code === "WrongKind",
    );

    // And the deck survives every one of them.
    assert.equal(deck.slideCount(), 1);
  });
});

test("opening bytes that are not a package fails with `Io`", () => {
  assert.throws(
    () => Deck.open(new Uint8Array([1, 2, 3])),
    (failure) => {
      assert.equal(failure.name, "OoxmlError");
      assert.equal(failure.code, "Io");
      assert.deepEqual(Object.keys(failure.detail), [], "an I/O failure names no coordinates");
      return true;
    },
  );
});

test("a freed deck throws rather than corrupting anything", () => {
  const size = SlideSize.widescreen();
  const deck = Deck.blank(size);
  size.free();
  deck.addSlide();
  deck.free();
  assert.throws(() => deck.slideCount(), /null pointer/);
  // Freeing twice is also refused, rather than double-freeing the wasm heap.
  assert.throws(() => deck.free(), /null pointer/);
});

test("enumerations keep their Rust member names, `None` included", () => {
  assert.equal(typeof TextUnderline.None, "number", "`None` needs no rename in TypeScript");
  assert.notEqual(TextUnderline.None, TextUnderline.Single);
  assert.equal(Object.keys(PresetShapeType).filter((k) => Number.isNaN(Number(k))).length, 187);
  assert.equal(Object.keys(ChartKind).filter((k) => Number.isNaN(Number(k))).length, 16);
  assert.equal(Object.keys(CellBorder).filter((k) => Number.isNaN(Number(k))).length, 6);
});

test("a format's accessors are functions, because an enumeration is a number here", () => {
  assert.equal(typeof Format.Presentation, "number");
  assert.equal(formatFamily(Format.Presentation), FormatFamily.Presentation);
  assert.equal(formatFamily(Format.Workbook), FormatFamily.Spreadsheet);
  assert.match(formatContentType(Format.Presentation), /presentationml/);
});

test("a value written through one method comes back only from its own reader", () => {
  withDeck((deck) => {
    // Two adjacent setters with identical shapes: mis-wire either and the two assertions swap.
    const table = bounds(1, 1, 6, 3, (b) => deck.addTable(0, 3, 3, b));
    const wide = Emu.fromInches(2.5);
    const tall = Emu.fromInches(0.75);
    deck.setColumnWidth(0, table, 1, wide);
    deck.setRowHeight(0, table, 2, tall);
    wide.free();
    tall.free();

    const width = deck.columnWidth(0, table, 1);
    const height = deck.rowHeight(0, table, 2);
    assert.equal(width.inches, 2.5);
    assert.equal(height.inches, 0.75);
    width.free();
    height.free();

    // A run hyperlink and a shape hyperlink live on the same shape and must not answer for
    // each other.
    const shape = bounds(1, 5, 4, 1, (b) => deck.addTextBox(0, "click", b));
    const toRun = Hyperlink.url("https://example.invalid/run");
    const toShape = Hyperlink.url("https://example.invalid/shape");
    deck.setRunHyperlink(0, shape, 0, 0, toRun);
    deck.setShapeHyperlink(0, shape, toShape);
    toRun.free();
    toShape.free();

    const runLink = deck.runHyperlink(0, shape, 0, 0);
    const shapeLink = deck.shapeHyperlink(0, shape);
    assert.equal(runLink.target, "https://example.invalid/run");
    assert.equal(shapeLink.target, "https://example.invalid/shape");
    runLink.free();
    shapeLink.free();
  });
});

test("the preset geometry table works in both directions and keeps its units", () => {
  withDeck((deck) => {
    const shape = bounds(1, 1, 3, 2, (b) =>
      deck.addShape(0, PresetShapeType.RoundedRectangle, b),
    );
    assert.deepEqual(ShapeGeometry.adjustmentNames(PresetShapeType.RoundedRectangle), [
      "corner_radius",
    ]);

    const radius = Fraction.of(0.4);
    const geometry = ShapeGeometry.of(PresetShapeType.RoundedRectangle, {
      corner_radius: radius,
    });
    const wrapped = Geometry.preset(geometry);
    deck.setShapeGeometry(0, shape, wrapped);
    radius.free();
    geometry.free();
    wrapped.free();

    const read = deck.shapeGeometry(0, shape);
    const preset = read.presetGeometry;
    assert.equal(preset.preset, PresetShapeType.RoundedRectangle);
    const corner = preset.adjustments.corner_radius;
    assert.equal(corner.ratio, 0.4);
    corner.free();
    preset.free();
    read.free();

    // The writing half arrives as two parallel arrays, because a wasm-bindgen signature cannot
    // carry a list of pairs without serde. `12345` is not a value the typed `Fraction` path would
    // produce from a round ratio, so a method wired to `setShapeGeometry` could not leave it there.
    deck.setShapeAdjustments(0, shape, ["adj"], [12345]);
    const restated = deck.shapeAdjustments(
      0,
      shape,
      GuideContext.fromExtents(Emu.fromInches(4), Emu.fromInches(1)),
    );
    assert.equal(restated[0].value, 12345);
    for (const entry of restated) entry.free();
    assert.throws(() => deck.setShapeAdjustments(0, shape, ["adj", "adj2"], [1]), /name/);

    // The unit is part of the contract: an angle is not a proportion.
    const angle = Angle.fromDegrees(30);
    assert.throws(
      () => ShapeGeometry.of(PresetShapeType.RoundedRectangle, { corner_radius: angle }),
      /proportion/,
    );
    angle.free();
    assert.throws(() => ShapeGeometry.of(PresetShapeType.RoundedRectangle, {}), /needs an adjustment/);
    assert.throws(
      () => ShapeGeometry.of(PresetShapeType.RoundedRectangle, { nonsense: Fraction.of(0.1) }),
      /no adjustment called/,
    );
  });
});

test("a specification can be applied to many shapes, because nothing consumes it", () => {
  withDeck((deck) => {
    const navy = ColorSpec.srgb("1F3864");
    const fill = FillSpec.solid(navy);
    const outline = LineSpec.solid(LineWidth.fromPoints(1), navy);
    const shapes = [0, 1, 2].map((offset) =>
      bounds(1 + offset * 2, 1, 1.5, 1.5, (b) =>
        deck.addShape(0, PresetShapeType.Ellipse, b),
      ),
    );
    for (const shape of shapes) {
      deck.setShapeFill(0, shape, fill);
      deck.setShapeOutline(0, shape, outline);
    }
    for (const shape of shapes) {
      const written = deck.shapeFill(0, shape);
      const color = written.color;
      assert.equal(color.srgbValue, "1F3864");
      color.free();
      written.free();
    }
    navy.free();
    fill.free();
    outline.free();
  });
});

test("a chart round-trips through the binding", () => {
  withDeck((deck) => {
    const empty = new ChartData(ChartKind.Bar);
    const categorised = empty.categories(["a", "b", "c"]);
    const chart = categorised.series("one", new Float64Array([1, 2, 3]));
    empty.free();
    categorised.free();

    const frame = bounds(1, 1, 8, 4, (b) => deck.addChart(0, chart, b));
    chart.free();

    assert.deepEqual(Array.from(deck.chartKinds(0, frame)), [ChartKind.Bar]);
    const series = deck.chartSeries(0, frame);
    assert.equal(series.length, 1);
    assert.equal(series[0].name, "one");
    assert.deepEqual(Array.from(series[0].values), [1, 2, 3]);
    series.forEach((entry) => entry.free());

    deck.setChartTitle(0, frame, "Trend");
    assert.equal(deck.chartTitle(0, frame), "Trend");

    // What the caches hold versus what the references name — the second half of the pair, and the
    // one MJXOFF-118 found bound on `Workbook` alone.
    const references = deck.chartSeriesReferences(0, frame);
    assert.equal(references.length, 1);
    assert.notEqual(references[0].values, undefined);
    references.forEach((entry) => entry.free());

    deck.validate();
  });
});

test("removing a deck chart binding is caught by this suite", () => {
  // The parity clause's own guard: **remove one binding and this case goes red.** The `Document`
  // and `Workbook` suites have carried this guard since their own children; MJXOFF-118 found the
  // `Deck` pair — the one the ticket names — had none, which is how `Deck.chartSeriesReferences`
  // came to be missing from both bindings while the chart case above stayed green. Delete
  // `Deck::chart_series_references` from `bindings/mjx-wasm/src/deck.rs` and this fails.
  withDeck((deck) => {
    for (const method of [
      "addChart",
      "chartPartBytes",
      "chartKinds",
      "chartSeries",
      "chartSeriesReferences",
      "chartAxes",
      "chartTitle",
      "chartLegend",
      "chartWorkbooks",
      "refreshChartWorkbook",
      "regenerateChartWorkbook",
      "detachChartWorkbook",
      "setChartSeriesValues",
      "setChartSeriesCategories",
      "setChartDataLabels",
      "addChartTrendline",
      "setChartErrorBars",
      "setChartPointFill",
      "chartDanglingDecoration",
    ]) {
      assert.equal(typeof deck[method], "function", `Deck.${method} is not bound`);
    }
  });
});

test("every colour transform in the group reaches TypeScript", () => {
  // `EG_ColorTransform` has twenty-eight members and a wasm enumeration is a number in JavaScript,
  // so the group arrives as `ColorTransformKind` plus three constructors that say what the member
  // carries (MJXOFF-219). This asserts the join: every kind is built by exactly one of the three,
  // and none of them invents a transform for a kind that does not take that value.
  const kinds = Object.keys(ColorTransformKind)
    .filter((name) => Number.isNaN(Number(name)) && name !== "Other")
    .map((name) => ColorTransformKind[name]);
  assert.equal(kinds.length, 28, "EG_ColorTransform has twenty-eight members");

  for (const kind of kinds) {
    const half = Fraction.of(0.5);
    const thirty = Angle.fromDegrees(30.0);
    const built = [
      ColorTransform.percentage(kind, half),
      ColorTransform.angle(kind, thirty),
      ColorTransform.marker(kind),
    ].filter((candidate) => candidate !== undefined);
    half.free();
    thirty.free();
    assert.equal(built.length, 1, `kind ${kind} is built by ${built.length} constructors, not one`);
    assert.equal(built[0].kind, kind);
    assert.ok(built[0].name.length > 0, "every member names an element");
    built[0].free();
  }

  // `Other` is not a member of the group: none of the three build it, and the bucket that does
  // keeps the element name and the raw value a file carried.
  const one = Fraction.of(1.0);
  assert.equal(ColorTransform.percentage(ColorTransformKind.Other, one), undefined);
  one.free();
  assert.equal(ColorTransform.marker(ColorTransformKind.Other), undefined);
  const kept = ColorTransform.other("futureTransform", "3");
  assert.equal(kept.kind, ColorTransformKind.Other);
  assert.equal(kept.name, "futureTransform");
  assert.equal(kept.value, "3");
  assert.equal(kept.percentageValue, undefined);
  assert.equal(kept.angleValue, undefined);
  kept.free();
});

test("a transformed colour is still the colour underneath", () => {
  // The builders append and `base` sees through them (MJXOFF-219).
  const accent = ColorSpec.scheme(SchemeColor.Accent1);
  const sixty = Fraction.of(0.6);
  const forty = Fraction.of(0.4);
  const modulated = accent.withLuminanceModulation(sixty);
  const lighter = modulated.withLuminanceOffset(forty);

  const base = lighter.base;
  assert.ok(base.equals(accent));
  assert.equal(lighter.schemeColor, SchemeColor.Accent1);
  base.free();

  const transforms = lighter.transforms;
  assert.deepEqual(
    transforms.map((transform) => transform.kind),
    [ColorTransformKind.LuminanceModulation, ColorTransformKind.LuminanceOffset],
  );
  const first = transforms[0].percentageValue;
  assert.ok(Math.abs(first.ratio - 0.6) < 1e-9);
  first.free();
  for (const transform of transforms) {
    transform.free();
  }

  // Order is part of the markup: the same two transforms the other way round are another colour.
  const offsetFirst = accent.withLuminanceOffset(forty);
  const reversed = offsetFirst.withLuminanceModulation(sixty);
  assert.equal(reversed.equals(lighter), false);
  assert.equal(accent.transforms.length, 0);

  // The generic builder reaches what the six conveniences do not — including the four
  // `V-PPTX-02.4` names, none of which any convenience covers.
  const literal = ColorSpec.srgb("4472C4");
  const gamma = ColorTransform.marker(ColorTransformKind.InverseGamma);
  const inverted = literal.withTransform(gamma);
  assert.equal(inverted.srgbValue, "4472C4");
  const invertedTransforms = inverted.transforms;
  assert.deepEqual(
    invertedTransforms.map((transform) => transform.kind),
    [ColorTransformKind.InverseGamma],
  );
  for (const transform of invertedTransforms) {
    transform.free();
  }

  const thirty = Angle.fromDegrees(30.0);
  const hueOffset = ColorTransform.angle(ColorTransformKind.HueOffset, thirty);
  const turned = literal.withTransform(hueOffset);
  const [only] = turned.transforms;
  const degrees = only.angleValue;
  assert.ok(Math.abs(degrees.degrees - 30.0) < 1e-6);
  degrees.free();
  only.free();

  for (const owned of [
    accent,
    sixty,
    forty,
    modulated,
    lighter,
    offsetFirst,
    reversed,
    literal,
    gamma,
    inverted,
    thirty,
    hueOffset,
    turned,
  ]) {
    owned.free();
  }
});

test("a colour transform reaches the file and comes back", () => {
  // The surface is not decorative: a transform authored through the facade is in the saved part,
  // and reading the deck back answers it (MJXOFF-219).
  withDeck((deck) => {
    const bounds = ShapeBounds.fromInches(1, 1, 2, 2);
    const shape = deck.addShape(0, PresetShapeType.Rectangle, bounds);
    bounds.free();

    const accent = ColorSpec.scheme(SchemeColor.Accent1);
    const half = Fraction.of(0.5);
    const tinted = accent.withTint(half);
    const fill = FillSpec.solid(tinted);
    deck.setShapeFill(0, shape, fill);
    for (const owned of [accent, half, tinted, fill]) {
      owned.free();
    }

    const parts = partPayloads(deck.save());
    const slide = parts["ppt/slides/slide1.xml"].toString("utf8");
    assert.ok(
      slide.includes('<a:schemeClr val="accent1"><a:tint val="50000"/></a:schemeClr>'),
      "the authored tint is in the saved slide",
    );

    const read = deck.shapeFill(0, shape);
    const color = read.color;
    const transforms = color.transforms;
    assert.deepEqual(
      transforms.map((transform) => transform.kind),
      [ColorTransformKind.Tint],
    );
    for (const owned of [...transforms, color, read]) {
      owned.free();
    }
  });
});
