// The building-a-document walkthrough, written through the WebAssembly binding.
//
// This is `crates/mjx-ooxml/examples/build_a_document.rs` call for call — the same paragraphs, the
// same numbered list, the same hyperlink, the same table, the same header, the same comment, the
// same footnote. `bindings/mjx-python/tests/test_build_a_document.py` is the second copy.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// The package under test is the bundler build; `bindings/mjx-wasm/build-npm.sh` produces it.

import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  Document,
  Format,
  HeaderFooterType,
  HyperlinkTarget,
  PageSize,
  SectionLocation,
} from "../../npm/dist/bundler/mjx_ooxml.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");
const OUTPUT_DIRECTORY = process.env.MJX_OUTPUT_DIR ?? join(REPOSITORY_ROOT, "target", "examples");

test("the Word walkthrough runs end to end through the wasm binding", () => {
  const document = Document.blank(PageSize.a4());
  const owned = [document];
  const keep = (value) => {
    owned.push(value);
    return value;
  };
  try {
    assert.equal(document.format(), Format.Document);
    assert.equal(document.paragraphCount(), 1);

    // ---- Paragraphs and runs -----------------------------------------------------------------
    document.appendRun(0, "Quarterly Review");
    document.appendParagraph();
    document.appendRun(1, "Prepared by the mjx-ooxml-rs example suite.");
    document.appendParagraph();
    document.appendRun(2, "Highlights");
    document.appendParagraph();
    document.appendRun(3, "Revenue grew across every region this quarter.");

    document.appendParagraph();
    document.appendRun(4, "North America: +12%");
    document.attachParagraphToList(4, 1, 0);
    document.appendParagraph();
    document.appendRun(5, "EMEA: +8%");
    document.attachParagraphToList(5, 1, 0);

    // ---- A hyperlink ---------------------------------------------------------------------------
    document.appendParagraph();
    document.appendRun(6, "Full figures: ");
    const url = keep(HyperlinkTarget.url("https://example.com/investors"));
    document.insertHyperlink(6, 1, "investor relations page", url);

    // ---- A table -------------------------------------------------------------------------------
    const table = document.appendTable(2, 2);
    document.setCellText(table, 0, 0, "Region");
    document.setCellText(table, 0, 1, "Growth");
    document.setCellText(table, 1, 0, "North America");
    document.setCellText(table, 1, 1, "+12%");
    const dimensions = keep(document.tableDimensions(table));
    assert.equal(dimensions.rows, 2);
    assert.equal(dimensions.columns, 2);

    // ---- A header and a comment -----------------------------------------------------------------
    const body = keep(SectionLocation.body());
    document.setHeaderText(body, HeaderFooterType.Default, "Quarterly Review — Internal");
    const commentId = document.addComment(
      0,
      "Reviewer",
      "R",
      "Confirm the North America figure before publishing.",
    );
    assert.notEqual(document.commentRangeText(commentId), undefined);

    // ---- A footnote ----------------------------------------------------------------------------
    document.addFootnote(3, "Figures are unaudited and subject to revision.");

    // ---- Save ----------------------------------------------------------------------------------
    document.validate();
    const bytes = document.save();
    assert.ok(bytes.length > 0);
    mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
    writeFileSync(join(OUTPUT_DIRECTORY, "wasm_build_a_document.docx"), bytes);

    // ---- Reopen, to prove the bytes are a real document -----------------------------------------
    const reopened = keep(Document.open(bytes));
    assert.equal(reopened.paragraphCount(), document.paragraphCount());
    assert.equal(reopened.paragraphText(0), "Quarterly Review");
    assert.equal(reopened.cellText(0, 0, 0), "Region");
    assert.equal(
      reopened.headerText(0, HeaderFooterType.Default),
      "Quarterly Review — Internal",
    );
    assert.equal(reopened.comments().length, 1);
    assert.equal(reopened.footnotes().length, 1);
  } finally {
    for (const value of owned) {
      value.free();
    }
  }
});
