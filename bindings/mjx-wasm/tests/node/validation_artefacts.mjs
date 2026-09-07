// The validation artefacts, written through the WebAssembly binding (MJXOFF-122).
//
// `xtask/src/validation/` is the Rust original: eighteen areas across the three formats, each built
// from a `blank` document through the facade and nothing below it. This file is the same eighteen,
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
  Angle,
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
  ChartRangeSeries,
  ChartWrap,
  Color,
  ColorSpec,
  Deck,
  Document,
  EffectListSpec,
  Emu,
  FillSpec,
  FontProperties,
  Fraction,
  GlowEffect,
  GradientStopSpec,
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
  PatternFillSpec,
  PictureFillMode,
  PresetShapeType,
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
    assert.ok(produced.length >= 18, `only ${produced.length} artefact(s); the catalogue has shrunk`);
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
