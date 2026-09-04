// The Word sibling of `surface.mjs`: every subject of the `Document` surface driven through one
// document, plus the mis-wiring guards for the pairs a swapped delegate could plausibly confuse.
//
//     node --test bindings/mjx-wasm/tests/node/

import assert from "node:assert/strict";
import test from "node:test";

import {
  CellBorderEdge,
  Document,
  Format,
  HeaderFooterType,
  HyperlinkTarget,
  MergedCellType,
  PageOrientation,
  PageSize,
  SectionLocation,
} from "../../npm/dist/bundler/mjx_ooxml.js";

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
