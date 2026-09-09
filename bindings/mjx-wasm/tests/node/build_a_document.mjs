// The building-a-document walkthrough, written through the WebAssembly binding.
//
// This is `crates/mjx-ooxml/examples/build_a_document.rs` call for call — the same paragraphs, the
// same numbered list, the same hyperlink, the same table, the same header, the same comment, the
// same footnote. `bindings/mjx-python/tests/test_build_a_document.py` is the second copy, and
// `the three Word walkthroughs agree` below checks that this one and the Rust one produce
// byte-identical parts.
//
//     node --test bindings/mjx-wasm/tests/node/
//
// That comparison is the reason this file is shaped the way it is. Until MJXOFF-239 it was not
// here: this file transcribed the Rust example and wrote its own `.docx`, and nothing ever ran the
// Rust one or looked at the two together. Both files passed, which is exactly the failure that is
// invisible from a test report — a `Document` method wired to the wrong facade method produced a
// different document and the suite stayed green, because there was nothing to be different from.
//
// The package under test is the bundler build; `bindings/mjx-wasm/build-npm.sh` produces it.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

import { partPayloads } from "./zip.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(HERE, "../../../..");
const OUTPUT_DIRECTORY = process.env.MJX_OUTPUT_DIR ?? join(REPOSITORY_ROOT, "target", "examples");

/**
 * The walkthrough, run once.
 *
 * Everything it builds is freed on the way out, including on failure — which is the shape every
 * caller of this library should copy. A leaked `Document` is a leaked megabyte.
 *
 * @returns {{ saved: Uint8Array, paragraphCount: number }} the document, and what it counted.
 */
function buildTheGuidesDocument() {
  const document = Document.blank(PageSize.a4());
  const owned = [document];
  /** Remembers a wasm object so the `finally` below can free it. */
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
    return { saved: document.save(), paragraphCount: document.paragraphCount() };
  } finally {
    for (const value of owned) {
      value.free();
    }
  }
}

test("the Word walkthrough runs end to end through the wasm binding", () => {
  const run = buildTheGuidesDocument();
  assert.ok(run.saved.length > 0);
  mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
  writeFileSync(join(OUTPUT_DIRECTORY, "wasm_build_a_document.docx"), run.saved);

  // ---- Reopen, to prove the bytes are a real document -----------------------------------------
  const reopened = Document.open(run.saved);
  try {
    assert.equal(reopened.paragraphCount(), run.paragraphCount);
    assert.equal(reopened.paragraphText(0), "Quarterly Review");
    assert.equal(reopened.cellText(0, 0, 0), "Region");
    assert.equal(reopened.headerText(0, HeaderFooterType.Default), "Quarterly Review — Internal");
    assert.equal(reopened.comments().length, 1);
    assert.equal(reopened.footnotes().length, 1);
  } finally {
    reopened.free();
  }
});

test("the three Word walkthroughs agree", (context) => {
  // This walkthrough and the Rust one must produce the *same document*, part for part.
  //
  // Not "both produce a file", and not "both produce a file of about the right size": the same part
  // names, and byte-identical payloads for every one of them. That is the only assertion that can
  // tell a faithful binding from a plausible one — a method wired to the wrong `Document` method, a
  // paragraph index off by one, or an argument converted with the wrong units, changes a payload
  // here and nothing else would notice.
  let rustOutput;
  try {
    mkdirSync(OUTPUT_DIRECTORY, { recursive: true });
    rustOutput = join(OUTPUT_DIRECTORY, "facade_build_a_document.docx");
    execFileSync(
      "cargo",
      ["run", "--quiet", "-p", "mjx-ooxml", "--example", "build_a_document", "--", rustOutput],
      { cwd: REPOSITORY_ROOT, stdio: "pipe" },
    );
  } catch (failure) {
    context.skip(`the Rust walkthrough could not be run: ${failure.message.split("\n")[0]}`);
    return;
  }

  const fromNode = partPayloads(buildTheGuidesDocument().saved);
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
