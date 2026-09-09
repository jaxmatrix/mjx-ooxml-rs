// The guide's **What is preserved rather than modelled** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — it only reads, which is the point — so there is nothing for the harness to compare and
// it skips that half by construction.
//
// Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
// bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const original = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "preserved_parts.xlsx"));

// guide-example:start
import { Workbook } from "@mjx/ooxml";

const workbook = Workbook.open(original);

// Inventoried from the relationships, with no markup parsed at all.
const summary = workbook.preservedParts();
if (summary.pivotTables.length === 0 || summary.pivotCacheDefinitions.length === 0) {
  throw new Error("this workbook carries pivot tables and the caches they read");
}

// And resolved far enough to say which tab each one sits on.
for (const table of workbook.pivotTables()) {
  if (table.sheetName === "") {
    throw new Error("every pivot table names the tab it sits on");
  }
  table.free();
}

// a wasm handle owns memory the garbage collector cannot see
summary.free();
workbook.free();
// guide-example:end
