// The validation artefacts, written through the WebAssembly binding (MJXOFF-122).
//
// `xtask/src/validation/` is the Rust original: twenty areas across the three formats, each built
// from a `blank` document through the facade and nothing below it. This file is the same twenty,
// call for call, and every one is compared against the Rust output **part by part, byte for byte**.
//
// That comparison is the point. A binding method wired to the wrong facade method, or an argument
// converted with the wrong units, changes one part payload and nothing else in this repository
// would notice — least of all a human reading the file in Office, which is where these artefacts
// are going. `bindings/mjx-python/tests/test_validation_artefacts.py` is the second copy and is
// compared against the same reference.
//
// **Nothing here records a result.** See `docs/validation/00-method.md`: judging what Office
// renders needs a person with Office in front of them, and a passing comparison in this file says
// only that three languages agree about the bytes.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// The package under test is the bundler build; `bindings/mjx-wasm/build-npm.sh` produces it.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  AdjustAngle,
  AdjustCoordinate,
  Angle,
  AxisOrientation,
  BorderEdgeSpec,
  BorderSpec,
  BorderStyle,
  CellBorder,
  CellFormat,
  CellFormatSpec,
  CellFormatTarget,
  CellMargins,
  CellWrite,
  Cells,
  CharacterPropertiesSpec,
  ChartData,
  ChartKind,
  ChartLabelScope,
  ChartRangeSeries,
  ChartWrap,
  Color,
  ColorSpec,
  ConnectionSite,
  CustomGeometrySpec,
  DataLabelPosition,
  DataLabelSpec,
  Deck,
  Document,
  DrawCommand,
  EffectListSpec,
  Emu,
  ErrorBarDirection,
  ErrorBarSpec,
  ErrorBarType,
  ErrorValueType,
  FillSpec,
  FontProperties,
  Fraction,
  Geometry,
  GlowEffect,
  GradientStopSpec,
  GuideContext,
  GuideSpec,
  HeaderFooterType,
  Hyperlink,
  HyperlinkTarget,
  LegendPosition,
  LineSpec,
  LineWidth,
  MergedCellType,
  OuterShadowEffect,
  PageMargins,
  PageSize,
  ParagraphPropertiesSpec,
  Path2DSpec,
  PatternFillSpec,
  PictureFillMode,
  Point,
  PresetShapeType,
  Rectangle,
  ResizingBehavior,
  SchemeColor,
  SectionLocation,
  ShapeBounds,
  SlideSize,
  TableStyleBorder,
  TableStyleFormat,
  TableStylePart,
  TextAlignment,
  TextAnchoring,
  Transform2D,
  TrendlineKind,
  TrendlineSpec,
  Workbook,
  WrapText,
  defaultPlaceholderImage,
} from "../../npm/dist/bundler/mjx_ooxml.js";
import { partPayloads } from "./zip.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");

/** One inch in EMU — the unit `Document` and `Workbook`'s drawing calls take. */
const INCH = 914_400;

/** The table style `V-PPTX-03` authors and points at. Not a built-in. */
const TABLE_STYLE_ID = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";

const PLACEHOLDER_CONTENT_TYPE = "image/png";
const PLACEHOLDER_EXTENSION = "png";

/**
 * Runs `build` with a collector that frees every wasm handle it was handed, however it ends.
 *
 * The binding hands out owned handles and JavaScript has no destructors, so a generator that threw
 * halfway would leak the ones it had made. `keep` is the same arrangement the three walkthrough
 * files already use.
 */
function owning(build) {
  const owned = [];
  const keep = (value) => {
    owned.push(value);
    return value;
  };
  try {
    return build(keep);
  } finally {
    for (const value of owned.reverse()) {
      try {
        value.free();
      } catch {
        // Already freed, or moved into a call that consumed it.
      }
    }
  }
}

/** The chart every format's chart area draws, so the three artefacts are comparable. */
function quarterlyChart(keep) {
  return keep(
    keep(keep(new ChartData(ChartKind.Bar)).categories(["Q1", "Q2", "Q3", "Q4"]))
      .series("2026", [12.0, 15.5, 14.0, 19.25]),
  ).series("2025", [10.5, 13.0, 13.75, 16.0]);
}

/** The figures every workbook area writes into its first sheet. */
function quarterlyCells() {
  return [
    CellWrite.sharedText("A1", "Region"),
    CellWrite.sharedText("B1", "Revenue"),
    CellWrite.sharedText("C1", "Growth"),
    CellWrite.sharedText("A2", "North America"),
    CellWrite.number("B2", 1250000),
    CellWrite.number("C2", 0.125),
    CellWrite.sharedText("A3", "EMEA"),
    CellWrite.number("B3", 980000),
    CellWrite.number("C3", 0.061),
    CellWrite.sharedText("A4", "Asia Pacific"),
    CellWrite.number("B4", 1410000),
    CellWrite.number("C4", 0.198),
  ];
}

/** The first line of a document area: written into the blank paragraph, or appended. */
function appendHeading(document, text) {
  const existing = document.paragraphCount();
  let at;
  if (existing === 1 && document.runCount(0) === 0) {
    at = 0;
  } else {
    document.appendParagraph();
    at = existing;
  }
  document.appendRun(at, text);
  return at;
}

function appendLine(document, text) {
  const at = document.paragraphCount();
  document.appendParagraph();
  document.appendRun(at, text);
  return at;
}

// -------------------------------------------------------------------------------------------
// V-PPTX-01 … V-PPTX-06
// -------------------------------------------------------------------------------------------

const vPptx01 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    const stated = deck.addTextBox(
      slide,
      "Stated on the run: 24pt bold, accent 1",
      keep(ShapeBounds.fromInches(0.5, 0.5, 8.0, 0.8)),
    );
    deck.setShapeRunProperties(
      slide,
      stated,
      keep(
        keep(keep(new CharacterPropertiesSpec()).withSizePoints(24.0))
          .withBold(true)
          .withColor(keep(ColorSpec.scheme(SchemeColor.Accent1))),
      ),
    );
    const paragraph = deck.addTextBox(
      slide,
      "Stated on the paragraph: centred, 1.5 line spacing",
      keep(ShapeBounds.fromInches(0.5, 1.6, 8.0, 0.8)),
    );
    deck.setParagraphProperties(
      slide,
      paragraph,
      0,
      keep(
        keep(keep(new ParagraphPropertiesSpec()).withAlignment(TextAlignment.Center))
          .withLeftMarginPoints(18.0)
          .withIndentPoints(-18.0),
      ),
    );
    const inherited = deck.addTextBox(
      slide,
      "Nothing stated: this run resolves from the layout and the master",
      keep(ShapeBounds.fromInches(0.5, 2.7, 8.0, 0.8)),
    );
    keep(deck.effectiveRunProperties(slide, inherited, 0, 0));
    return deck.save();
  });

const vPptx02 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();

    const solid = deck.addShape(
      slide,
      PresetShapeType.Rectangle,
      keep(ShapeBounds.fromInches(0.5, 0.5, 2.5, 1.5)),
    );
    deck.setShapeFill(slide, solid, keep(FillSpec.solid(keep(ColorSpec.srgb("1F3864")))));
    deck.setShapeOutline(
      slide,
      solid,
      keep(
        LineSpec.solid(
          keep(LineWidth.fromPoints(3.0)),
          keep(ColorSpec.scheme(SchemeColor.Accent2)),
        ),
      ),
    );

    const gradient = deck.addShape(
      slide,
      PresetShapeType.Ellipse,
      keep(ShapeBounds.fromInches(3.5, 0.5, 2.5, 1.5)),
    );
    deck.setShapeFill(
      slide,
      gradient,
      keep(
        FillSpec.gradient(
          [
            new GradientStopSpec(keep(Fraction.of(0.0)), keep(ColorSpec.srgb("FFF2CC"))),
            new GradientStopSpec(keep(Fraction.of(1.0)), keep(ColorSpec.srgb("C00000"))),
          ],
          keep(Angle.fromDegrees(45.0)),
        ),
      ),
    );

    const effects = deck.addShape(
      slide,
      PresetShapeType.RoundedRectangle,
      keep(ShapeBounds.fromInches(6.5, 0.5, 2.5, 1.5)),
    );
    deck.setShapeFill(slide, effects, keep(FillSpec.solid(keep(ColorSpec.srgb("FFFFFF")))));
    deck.setShapeEffects(
      slide,
      effects,
      keep(
        new EffectListSpec(
          undefined,
          undefined,
          new GlowEffect(keep(ColorSpec.scheme(SchemeColor.Accent1)), keep(Emu.fromPoints(5.0))),
          undefined,
          new OuterShadowEffect(
            keep(ColorSpec.srgb("808080")),
            keep(Emu.fromPoints(4.0)),
            keep(Emu.fromPoints(3.0)),
            keep(Angle.fromDegrees(45.0)),
          ),
        ),
      ),
    );
    return deck.save();
  });

const vPptx03 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    const table = deck.addTable(slide, 3, 3, keep(ShapeBounds.fromInches(0.5, 0.5, 8.0, 2.5)));
    const rows = [
      ["Region", "Revenue", "Growth"],
      ["North", "4.2", "+12%"],
      ["South", "3.1", "+8%"],
    ];
    rows.forEach((cells, row) => {
      cells.forEach((text, column) => {
        deck.setCellText(slide, table, row, column, 0, text);
      });
    });
    deck.createTableStyle(TABLE_STYLE_ID, "mjx validation");
    deck.formatTableStylePart(
      TABLE_STYLE_ID,
      TableStylePart.FirstRow,
      keep(
        keep(keep(new TableStyleFormat()).withFill(keep(FillSpec.solid(keep(ColorSpec.srgb("1F3864"))))))
          .withTextColor(keep(ColorSpec.srgb("FFFFFF")))
          .withBorder(
            TableStyleBorder.Bottom,
            keep(
              LineSpec.solid(keep(LineWidth.fromPoints(2.0)), keep(ColorSpec.srgb("FFFFFF"))),
            ),
          ),
      ),
    );
    deck.setTableStyle(slide, table, TABLE_STYLE_ID);
    deck.formatCells(
      slide,
      table,
      keep(Cells.row(0)),
      keep(
        keep(keep(new CellFormat()).withAnchor(TextAnchoring.Center)).withMargins(
          keep(CellMargins.uniform(keep(Emu.fromPoints(6.0)))),
        ),
      ),
    );
    deck.setCellBorder(
      slide,
      table,
      2,
      0,
      CellBorder.Top,
      keep(LineSpec.solid(keep(LineWidth.fromPoints(1.0)), keep(ColorSpec.srgb("C00000")))),
    );
    // A range argument is two numbers here — the one shape difference `CLAUDE.md` records for this
    // binding, because a JavaScript caller has no `Range` to hand over.
    deck.mergeCells(slide, table, keep(Cells.rectangle(2, 3, 1, 3)));
    return deck.save();
  });

const vPptx04 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    const chart = deck.addChart(
      slide,
      quarterlyChart(keep),
      keep(ShapeBounds.fromInches(0.5, 0.5, 8.0, 4.5)),
    );
    deck.setChartTitle(slide, chart, "Revenue by quarter");
    deck.setChartLegend(slide, chart, LegendPosition.Bottom);
    deck.setChartAxisTitle(slide, chart, 0, "Quarter");
    deck.setChartAxisTitle(slide, chart, 1, "Revenue");
    deck.setChartSeriesFill(
      slide,
      chart,
      0,
      keep(FillSpec.solid(keep(ColorSpec.scheme(SchemeColor.Accent1)))),
    );
    deck.addChartTrendline(slide, chart, 0, keep(new TrendlineSpec(TrendlineKind.Linear)));
    return deck.save();
  });

const vPptx05 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    deck.addPicture(
      slide,
      defaultPlaceholderImage(),
      keep(ShapeBounds.fromInches(0.5, 0.5, 2.0, 2.0)),
    );
    const rel = deck.addImage(slide, defaultPlaceholderImage());
    const filled = deck.addShape(
      slide,
      PresetShapeType.Rectangle,
      keep(ShapeBounds.fromInches(3.0, 0.5, 3.0, 2.0)),
    );
    deck.setShapeFill(slide, filled, keep(FillSpec.picture(rel, PictureFillMode.Stretch)));
    return deck.save();
  });

const vPptx06 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    const linked = deck.addTextBox(
      slide,
      "The whole shape is a link",
      keep(ShapeBounds.fromInches(0.5, 0.5, 4.0, 0.8)),
    );
    deck.setShapeHyperlink(slide, linked, keep(Hyperlink.url("https://example.com/investors")));
    const runLinked = deck.addTextBox(
      slide,
      "Only this run is a link",
      keep(ShapeBounds.fromInches(0.5, 1.6, 4.0, 0.8)),
    );
    deck.setRunHyperlink(slide, runLinked, 0, 0, keep(Hyperlink.url("https://example.com/report")));
    deck.setNotesText(slide, "Lead with the revenue number, then the regional split.");
    return deck.save();
  });

// -------------------------------------------------------------------------------------------
// V-DOCX-01 … V-DOCX-06
// -------------------------------------------------------------------------------------------


// -------------------------------------------------------------------------------------------
// V-PPTX-07 … V-PPTX-08
// -------------------------------------------------------------------------------------------

/** The four presets whose guide formulas take an arc-tangent argument through zero. */
const ARC_TANGENT_PRESETS = [
  PresetShapeType.Moon,
  PresetShapeType.Arc,
  PresetShapeType.CircularArrow,
  PresetShapeType.Gear9,
];

/** The fifteen plot types that draw from one series, each with its `c:` element's local name. */
const SINGLE_SERIES_KINDS = [
  [ChartKind.Bar, "barChart"],
  [ChartKind.Bar3D, "bar3DChart"],
  [ChartKind.Line, "lineChart"],
  [ChartKind.Line3D, "line3DChart"],
  [ChartKind.Pie, "pieChart"],
  [ChartKind.Pie3D, "pie3DChart"],
  [ChartKind.OfPie, "ofPieChart"],
  [ChartKind.Area, "areaChart"],
  [ChartKind.Area3D, "area3DChart"],
  [ChartKind.Scatter, "scatterChart"],
  [ChartKind.Doughnut, "doughnutChart"],
  [ChartKind.Radar, "radarChart"],
  [ChartKind.Bubble, "bubbleChart"],
  [ChartKind.Surface, "surfaceChart"],
  [ChartKind.Surface3D, "surface3DChart"],
];

// The apex placed by the guide `apex` (formula `*/ w 1 2`) rather than by a number.
function guideDrivenTriangle(keep) {
  const guide = (name) => keep(AdjustCoordinate.guide(name));
  const zero = () => keep(AdjustCoordinate.emu(keep(Emu.fromEmu(0n))));
  return keep(
    new CustomGeometrySpec(
      [
        keep(
          new Path2DSpec(
            [
              keep(DrawCommand.moveTo(keep(new Point(guide("apex"), zero())))),
              keep(DrawCommand.lineTo(keep(new Point(guide("r"), guide("b"))))),
              keep(DrawCommand.lineTo(keep(new Point(guide("l"), guide("b"))))),
              keep(DrawCommand.close()),
            ],
            undefined,
            undefined,
            undefined,
            undefined,
            undefined,
          ),
        ),
      ],
      [keep(new GuideSpec("adj", "val 50000"))],
      [keep(new GuideSpec("apex", "*/ w 1 2"))],
      [],
      [
        keep(
          new ConnectionSite(
            keep(AdjustAngle.angle(keep(Angle.fromDegrees(270.0)))),
            keep(new Point(guide("apex"), zero())),
          ),
        ),
        keep(
          new ConnectionSite(
            keep(AdjustAngle.angle(keep(Angle.fromDegrees(0.0)))),
            keep(new Point(guide("r"), guide("b"))),
          ),
        ),
        keep(
          new ConnectionSite(
            keep(AdjustAngle.angle(keep(Angle.fromDegrees(180.0)))),
            keep(new Point(guide("l"), guide("b"))),
          ),
        ),
      ],
      keep(new Rectangle(guide("l"), guide("vc"), guide("r"), guide("b"))),
    ),
  );
}

function writeGeometryAreas(deck, surface, keep) {
  const squareChevron = deck.addShape(
    surface,
    PresetShapeType.Chevron,
    keep(ShapeBounds.fromInches(0.4, 3.4, 2.5, 2.5)),
  );
  const wideChevron = deck.addShape(
    surface,
    PresetShapeType.Chevron,
    keep(ShapeBounds.fromInches(3.2, 3.4, 2.5, 1.25)),
  );
  ARC_TANGENT_PRESETS.forEach((preset, position) => {
    deck.addShape(
      surface,
      preset,
      keep(ShapeBounds.fromInches(0.4 + 1.4 * position, 6.1, 1.2, 1.2)),
    );
  });
  const custom = deck.addShape(
    surface,
    PresetShapeType.Rectangle,
    keep(ShapeBounds.fromInches(6.2, 3.4, 3.0, 1.5)),
  );
  deck.setShapeGeometry(surface, custom, keep(Geometry.custom(guideDrivenTriangle(keep))));
  deck.setShapeFill(surface, custom, keep(FillSpec.solid(keep(ColorSpec.scheme(SchemeColor.Accent2)))));
  deck
    .shapeAdjustments(
      surface,
      squareChevron,
      keep(GuideContext.fromExtents(keep(Emu.fromEmu(2286000n)), keep(Emu.fromEmu(2286000n)))),
    )
    .forEach((value) => keep(value));
  deck
    .shapeAdjustments(
      surface,
      wideChevron,
      keep(GuideContext.fromExtents(keep(Emu.fromEmu(2286000n)), keep(Emu.fromEmu(1143000n)))),
    )
    .forEach((value) => keep(value));
  keep(deck.shapeGeometry(surface, custom));
  const rotated = deck.addShape(
    surface,
    PresetShapeType.Rectangle,
    keep(ShapeBounds.fromInches(6.2, 5.2, 1.6, 1.0)),
  );
  deck.setShapeTransform(
    surface,
    rotated,
    keep(
      new Transform2D(
        undefined,
        undefined,
        keep(Angle.fromDegrees(30.0)),
        undefined,
        undefined,
        undefined,
        undefined,
      ),
    ),
  );
  keep(deck.effectiveShapeBounds(surface, rotated));
}

const vPptx07 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.standard())));
    const slide = deck.addSlideFromLayout(0);
    deck.setShapeTextContent(slide, 0, "4:3 — 10 x 7.5 in");
    deck.setShapeTextContent(
      slide,
      1,
      "The two placeholders above and beside this one were placed by the master, not by this code.",
    );
    writeGeometryAreas(deck, slide, keep);
    return deck.save();
  });

function galleryBounds(position) {
  const column = position % 2;
  const row = Math.floor((position % 4) / 2);
  return ShapeBounds.fromInches(0.4 + column * 6.4, 0.4 + row * 3.4, 6.0, 3.0);
}

function writeChartDecorationAreas(deck, surface, keep) {
  const edited = deck.addChart(
    surface,
    keep(
      keep(keep(new ChartData(ChartKind.Bar)).categories(["Jan", "Feb", "Mar"])).series(
        "Sales",
        [19.2, 21.4, 16.7],
      ),
    ),
    keep(ShapeBounds.fromInches(0.4, 0.4, 5.8, 2.6)),
  );
  deck.setChartTitle(surface, edited, "Series values, rewritten");
  deck.setChartSeriesValues(surface, edited, 0, [41.5, 42.5, 43.5]);

  const labelled = deck.addChart(
    surface,
    keep(quarterlyChart(keep)),
    keep(ShapeBounds.fromInches(6.6, 0.4, 6.2, 2.6)),
  );
  deck.setChartTitle(surface, labelled, "Labels, three tiers");
  deck.setChartDataLabels(
    surface,
    labelled,
    keep(ChartLabelScope.plot(0)),
    keep(
      keep(
        keep(keep(new DataLabelSpec()).value(true)).position(DataLabelPosition.OutsideEnd),
      ).separator("; "),
    ).numberFormat("0.0"),
  );
  deck.setChartDataLabels(
    surface,
    labelled,
    keep(ChartLabelScope.series(0)),
    keep(keep(new DataLabelSpec()).categoryName(true)),
  );
  deck.suppressChartDataLabels(surface, labelled, keep(ChartLabelScope.point(0, 1)));
  deck.suppressChartDataLabels(surface, labelled, keep(ChartLabelScope.series(1)));
  deck.addChartTrendline(
    surface,
    labelled,
    0,
    keep(
      keep(
        keep(keep(new TrendlineSpec(TrendlineKind.Polynomial)).polynomialOrder(3)).projection(
          2.0,
          0.0,
        ),
      ).display(true, true),
    ),
  );

  const pie = deck.addChart(
    surface,
    keep(
      keep(
        keep(new ChartData(ChartKind.Pie)).categories(["North", "South", "East", "West"]),
      ).series("Share", [42.0, 28.0, 18.0, 12.0]),
    ),
    keep(ShapeBounds.fromInches(0.4, 3.4, 5.8, 3.4)),
  );
  deck.setChartTitle(surface, pie, "Slice 1 exploded, slice 0 recoloured");
  deck.setChartPointExplosion(surface, pie, 0, 1, 25);
  deck.setChartPointFill(surface, pie, 0, 0, keep(FillSpec.solid(keep(ColorSpec.srgb("2E75B6")))));

  const scatter = deck.addChart(
    surface,
    keep(
      keep(keep(new ChartData(ChartKind.Scatter)).categories(["1", "2", "3", "4"])).series(
        "Measured",
        [2.0, 4.5, 3.25, 6.0],
      ),
    ),
    keep(ShapeBounds.fromInches(6.6, 3.4, 6.2, 3.4)),
  );
  deck.setChartTitle(surface, scatter, "Error bars on both axes");
  deck.setChartErrorBars(
    surface,
    scatter,
    0,
    keep(
      keep(ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.Percentage, 5.0)).direction(
        ErrorBarDirection.X,
      ),
    ),
  );
  deck.setChartErrorBars(
    surface,
    scatter,
    0,
    keep(
      keep(ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 0.5)).direction(
        ErrorBarDirection.Y,
      ),
    ),
  );
}

function writeAxesAndDetachedWorkbookArea(deck, surface, keep) {
  const axes = deck.addChart(
    surface,
    keep(quarterlyChart(keep)),
    keep(ShapeBounds.fromInches(0.4, 0.4, 6.0, 3.0)),
  );
  deck.setChartTitle(surface, axes, "Bounded 0-25, reversed, ruled");
  deck.setChartAxisTitle(surface, axes, 0, "Quarter");
  deck.setChartAxisTitle(surface, axes, 1, "Revenue");
  deck.setChartAxisScale(surface, axes, 1, 0.0, 25.0);
  deck.setChartAxisOrientation(surface, axes, 1, AxisOrientation.MaximumToMinimum);
  deck.setChartAxisGridlines(surface, axes, 0, true, false);
  deck.setChartAxisGridlines(surface, axes, 1, true, true);
  deck.setChartSeriesFill(surface, axes, 0, keep(FillSpec.solid(keep(ColorSpec.srgb("4472C4")))));
  deck.setChartSeriesLine(
    surface,
    axes,
    1,
    keep(LineSpec.solid(keep(LineWidth.fromPoints(2.0)), keep(ColorSpec.srgb("ED7D31")))),
  );
  const detached = deck.addChart(
    surface,
    keep(quarterlyChart(keep)),
    keep(ShapeBounds.fromInches(6.8, 0.4, 6.0, 3.0)),
  );
  deck.setChartTitle(surface, detached, "This chart has no embedded workbook");
  deck.detachChartWorkbook(surface, detached);
  deck.chartWorkbooks(surface).forEach((value) => keep(value));
}

function writeDanglingPointArea(deck, surface, keep) {
  const shortened = deck.addChart(
    surface,
    keep(
      keep(keep(new ChartData(ChartKind.Bar)).categories(["Q1", "Q2", "Q3"])).series(
        "2026",
        [4.0, 5.0, 6.0],
      ),
    ),
    keep(ShapeBounds.fromInches(0.4, 0.4, 6.0, 3.0)),
  );
  deck.setChartTitle(surface, shortened, "A c:dPt left past the end of its series");
  deck.setChartPointFill(
    surface,
    shortened,
    0,
    2,
    keep(FillSpec.solid(keep(ColorSpec.srgb("C00000")))),
  );
  deck.setChartSeriesValues(surface, shortened, 0, [4.0, 5.0]);
  deck.chartDanglingDecoration(surface, shortened, 0).forEach((value) => keep(value));
}

function writePlotTypeGallery(deck, keep) {
  let slide = null;
  SINGLE_SERIES_KINDS.forEach(([kind, name], position) => {
    if (position % 4 === 0) {
      slide = deck.addSlide();
    }
    const data = keep(
      keep(keep(new ChartData(kind)).categories(["A", "B", "C"])).series("S", [1.0, 2.0, 3.0]),
    );
    const chart = deck.addChart(slide, data, keep(galleryBounds(position)));
    deck.setChartTitle(slide, chart, name);
  });
  const stock = deck.addChart(
    slide,
    keep(
      keep(
        keep(
          keep(keep(new ChartData(ChartKind.Stock)).categories(["Mon", "Tue", "Wed"])).series(
            "High",
            [7.0, 8.0, 9.0],
          ),
        ).series("Low", [3.0, 4.0, 5.0]),
      ).series("Close", [5.0, 6.0, 7.0]),
    ),
    keep(galleryBounds(3)),
  );
  deck.setChartTitle(slide, stock, "stockChart");
}

const vPptx08 = () =>
  owning((keep) => {
    const deck = keep(Deck.blank(keep(SlideSize.widescreen())));
    const slide = deck.addSlide();
    writeChartDecorationAreas(deck, slide, keep);
    writeAxesAndDetachedWorkbookArea(deck, deck.addSlide(), keep);
    writeDanglingPointArea(deck, deck.addSlide(), keep);
    writePlotTypeGallery(deck, keep);
    return deck.save();
  });


const vDocx01 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    appendHeading(document, "Quarterly Review");
    appendLine(
      document,
      "This paragraph states nothing: every property it renders with is inherited.",
    );
    const stated = appendLine(document, "North America: +12%");
    appendLine(document, "EMEA: +8%");
    keep(document.effectiveRunProperties(stated, 0));
    return document.save();
  });

const vDocx02 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    appendHeading(document, "Sections, headers and footers");
    document.setSectionPageSize(keep(SectionLocation.body()), keep(keep(PageSize.usLetter()).landscape()));
    document.setSectionPageMargins(keep(SectionLocation.body()), keep(PageMargins.normal()));
    document.setHeaderText(
      keep(SectionLocation.body()),
      HeaderFooterType.Default,
      "Quarterly Review — Internal",
    );
    document.setFooterText(
      keep(SectionLocation.body()),
      HeaderFooterType.Default,
      "Page footer, default type",
    );
    document.setHeaderText(
      keep(SectionLocation.body()),
      HeaderFooterType.First,
      "First page header",
    );
    return document.save();
  });

const vDocx03 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    appendHeading(document, "Tables");
    const table = document.appendTable(3, 3);
    const rows = [
      ["Region", "Revenue", "Growth"],
      ["North America", "4.2", "+12%"],
      ["EMEA", "3.1", "+8%"],
    ];
    rows.forEach((cells, row) => {
      cells.forEach((text, column) => {
        document.setCellText(table, row, column, text);
      });
    });
    document.setCellSpan(table, 2, 1, 2);
    document.setCellVerticalMerge(table, 1, 0, MergedCellType.Restart);
    document.setCellVerticalMerge(table, 2, 0, MergedCellType.Continue);
    return document.save();
  });

const vDocx04 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    const anchor = appendHeading(document, "Charts");
    const chart = document.addChart(anchor, quarterlyChart(keep), 6 * INCH, 3 * INCH, "Inline chart");
    document.setChartTitle(chart, "Revenue by quarter");
    document.setChartLegend(chart, LegendPosition.Bottom);
    document.setChartAxisTitle(chart, 0, "Quarter");
    const floatingAnchor = appendLine(document, "A floating chart follows this paragraph.");
    const floating = document.addFloatingChart(
      floatingAnchor,
      quarterlyChart(keep),
      INCH / 2,
      INCH / 2,
      4 * INCH,
      2 * INCH,
      keep(ChartWrap.square(WrapText.BothSides)),
      "Floating chart",
    );
    document.setChartTitle(floating, "The same numbers, wrapped square");
    return document.save();
  });

const vDocx05 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    const anchor = appendHeading(document, "Pictures");
    document.addInlinePicture(
      anchor,
      defaultPlaceholderImage(),
      PLACEHOLDER_CONTENT_TYPE,
      PLACEHOLDER_EXTENSION,
      2 * INCH,
      2 * INCH,
      "Placeholder",
    );
    const second = appendLine(document, "A second picture, half the size:");
    document.addInlinePicture(
      second,
      defaultPlaceholderImage(),
      PLACEHOLDER_CONTENT_TYPE,
      PLACEHOLDER_EXTENSION,
      INCH,
      INCH,
      "Placeholder, small",
    );
    return document.save();
  });

const vDocx06 = () =>
  owning((keep) => {
    const document = keep(Document.blank(keep(PageSize.a4())));
    const anchor = appendHeading(document, "Notes, comments and links");
    document.addFootnote(anchor, "Figures are unaudited and subject to revision.");
    document.addEndnote(anchor, "Prepared by the validation harness.");
    document.addComment(anchor, "Reviewer", "R", "Confirm the North America figure before publishing.");
    const linkParagraph = appendLine(document, "Full figures: ");
    document.insertHyperlink(
      linkParagraph,
      1,
      "investor relations page",
      keep(HyperlinkTarget.url("https://example.com/investors")),
    );
    return document.save();
  });

// -------------------------------------------------------------------------------------------
// V-XLSX-01 … V-XLSX-06
// -------------------------------------------------------------------------------------------

const vXlsx01 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Summary");
    const cells = quarterlyCells();
    cells.push(CellWrite.sharedText("A6", "Audited"));
    cells.push(CellWrite.boolean("B6", false));
    cells.push(CellWrite.sharedText("A7", "Deliberately in error"));
    cells.push(CellWrite.error("B7", "#N/A"));
    cells.push(
      CellWrite.inlineText("A8", "Inline, not shared: this string is written into the cell"),
    );
    cells.push(CellWrite.blank("B8"));
    workbook.writeCells(0, cells);
    const notes = workbook.addSheet("Notes");
    workbook.writeCells(notes, [
      CellWrite.inlineText("A1", "Figures are unaudited and subject to revision."),
    ]);
    return workbook.save();
  });

const vXlsx02 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Formats");
    workbook.writeCells(0, quarterlyCells());

    const styleFont = workbook.appendFont(
      keep(
        keep(keep(new FontProperties()).withFontName("Times New Roman")).withItalic(true).withSizePoints(13.0),
      ),
    );
    const styleFill = workbook.appendPatternFill(keep(PatternFillSpec.solid("445566")));
    const plain = () => keep(new BorderEdgeSpec());
    const styleBorder = workbook.appendBorder(
      keep(
        keep(
          keep(keep(keep(new BorderSpec()).withLeft(plain())).withRight(
            keep(keep(BorderEdgeSpec.styled(BorderStyle.Thick)).withColor(keep(Color.fromOpaqueRgb("000000")))),
          )).withTop(plain()),
        ).withBottom(plain()),
      ).withDiagonal(plain()),
    );
    const beneath = workbook.appendCellFormat(
      CellFormatTarget.CellStyleFormats,
      keep(
        keep(keep(keep(CellFormatSpec.skeletonCellStyleFormat()).withFontIndex(styleFont)).withFillIndex(styleFill))
          .withBorderIndex(styleBorder),
      ),
    );

    const directFont = workbook.appendFont(
      keep(keep(keep(new FontProperties()).withFontName("Arial")).withBold(true).withSizePoints(12.0)),
    );
    const directFill = workbook.appendPatternFill(keep(PatternFillSpec.solid("112233")));
    const directBorder = workbook.appendBorder(
      keep(
        keep(
          keep(
            keep(
              keep(new BorderSpec()).withLeft(
                keep(keep(BorderEdgeSpec.styled(BorderStyle.Thin)).withColor(keep(Color.fromOpaqueRgb("000000")))),
              ),
            ).withRight(plain()),
          ).withTop(plain()),
        ).withBottom(plain()),
      ).withDiagonal(plain()),
    );
    const direct = workbook.appendCellFormat(
      CellFormatTarget.CellFormats,
      keep(
        keep(
          keep(
            keep(keep(CellFormatSpec.skeletonCellFormat()).withCellStyleFormatIndex(beneath)).withFontIndex(directFont),
          ).withFillIndex(directFill),
        ).withBorderIndex(directBorder),
      ),
    );
    for (const reference of ["A1", "B1", "C1"]) {
      workbook.setCellStyle(0, reference, direct);
    }
    const resolved = workbook.effectiveCellFormat(0, "A1");
    if (resolved !== undefined) {
      keep(resolved);
    }
    return workbook.save();
  });

const vXlsx03 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Grid");
    workbook.writeCells(0, quarterlyCells());
    workbook.mergeCells(0, "A6:C6");
    workbook.setRowHeight(0, 0, 30.0, true);
    workbook.setRowHidden(0, 4, true);
    workbook.setRowOutlineLevel(0, 2, 1);
    workbook.setRowOutlineLevel(0, 3, 1);
    workbook.setColumnWidth(0, 0, 0, 24.0, true);
    workbook.setColumnWidth(0, 1, 2, 14.0, true);
    workbook.setColumnHidden(0, 5, 5, true);
    return workbook.save();
  });

const vXlsx04 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Charts");
    workbook.writeCells(0, quarterlyCells());
    const summary = keep(workbook.sheet(0));
    const sheetName = summary.name;
    const anchor = workbook.addRangeChart(
      0,
      ChartKind.Bar,
      `${sheetName}!$A$2:$A$4`,
      [
        keep(new ChartRangeSeries("Revenue", `${sheetName}!$B$2:$B$4`)).namedByCell(
          `${sheetName}!$B$1`,
        ),
      ],
      4,
      1,
      11,
      16,
      "Revenue by region",
      ResizingBehavior.MoveAndResizeWithAnchorCells,
    );
    workbook.setChartTitle(0, anchor, "Revenue by region");
    workbook.setChartLegend(0, anchor, LegendPosition.Bottom);
    workbook.setChartAxisTitle(0, anchor, 0, "Region");
    return workbook.save();
  });

const vXlsx05 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Drawings");
    workbook.writeCells(0, quarterlyCells());
    workbook.addTwoCellAnchoredPicture(
      0,
      defaultPlaceholderImage(),
      "Two-cell anchored",
      4,
      0,
      1,
      0,
      7,
      0,
      8,
      0,
      ResizingBehavior.MoveAndResizeWithAnchorCells,
    );
    workbook.addOneCellAnchoredPicture(
      0,
      defaultPlaceholderImage(),
      "One-cell anchored",
      4,
      0,
      10,
      0,
      2 * INCH,
      INCH,
    );
    return workbook.save();
  });

const vXlsx06 = () =>
  owning((keep) => {
    const workbook = keep(Workbook.blank());
    workbook.renameSheet(0, "Comments");
    workbook.writeCells(0, quarterlyCells());
    workbook.addCellComment(0, "B2", "Reviewer", "Confirm the North America figure before publishing.");
    workbook.addCellComment(0, "C4", "Reviewer", "Growth restated in March.");
    workbook.setCellHyperlinkUrl(0, "A1", "https://example.com/investors");
    return workbook.save();
  });

/**
 * Every area, keyed by the artefact file name the Rust generator writes. The key is what binds this
 * file to the index: a name that stops matching fails the comparison below rather than drifting.
 */
const GENERATORS = {
  "v-pptx-01-authored.pptx": vPptx01,
  "v-pptx-02-authored.pptx": vPptx02,
  "v-pptx-03-authored.pptx": vPptx03,
  "v-pptx-04-authored.pptx": vPptx04,
  "v-pptx-05-authored.pptx": vPptx05,
  "v-pptx-06-authored.pptx": vPptx06,
  "v-pptx-07-authored.pptx": vPptx07,
  "v-pptx-08-authored.pptx": vPptx08,
  "v-docx-01-authored.docx": vDocx01,
  "v-docx-02-authored.docx": vDocx02,
  "v-docx-03-authored.docx": vDocx03,
  "v-docx-04-authored.docx": vDocx04,
  "v-docx-05-authored.docx": vDocx05,
  "v-docx-06-authored.docx": vDocx06,
  "v-xlsx-01-authored.xlsx": vXlsx01,
  "v-xlsx-02-authored.xlsx": vXlsx02,
  "v-xlsx-03-authored.xlsx": vXlsx03,
  "v-xlsx-04-authored.xlsx": vXlsx04,
  "v-xlsx-05-authored.xlsx": vXlsx05,
  "v-xlsx-06-authored.xlsx": vXlsx06,
};

/** Runs the Rust generator into a temporary directory and returns it. */
function rustArtefacts() {
  const directory = mkdtempSync(join(tmpdir(), "mjx-validation-"));
  execFileSync(
    "cargo",
    ["run", "--quiet", "-p", "xtask", "--", "validation-artefacts", "--out", directory],
    { cwd: REPOSITORY_ROOT, stdio: ["ignore", "pipe", "pipe"] },
  );
  return directory;
}

test("the WebAssembly generators and the Rust ones agree, part by part", () => {
  const directory = rustArtefacts();
  try {
    const produced = readdirSync(directory).sort();
    assert.ok(produced.length >= 20, `only ${produced.length} artefact(s); the catalogue has shrunk`);
    // Stated over the filesystem rather than over the list above, so an area added in Rust and not
    // here fails instead of being silently skipped.
    assert.deepEqual(
      produced,
      Object.keys(GENERATORS).sort(),
      "the WebAssembly generators and the Rust ones name different artefacts",
    );

    for (const name of produced) {
      const fromWasm = partPayloads(GENERATORS[name]());
      const fromRust = partPayloads(readFileSync(join(directory, name)));
      assert.deepEqual(
        Object.keys(fromWasm).sort(),
        Object.keys(fromRust).sort(),
        `${name}: the two generators author different parts`,
      );
      const differing = Object.keys(fromWasm).filter(
        (part) => Buffer.compare(Buffer.from(fromWasm[part]), Buffer.from(fromRust[part])) !== 0,
      );
      assert.deepEqual(differing, [], `${name}: these parts differ between WebAssembly and Rust`);
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
