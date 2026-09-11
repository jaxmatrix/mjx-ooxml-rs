// The guide's **Opening detects first, then parses once** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — it refuses to open something — so there is nothing for the harness to compare and it
// skips that half by construction.
//
// **The eleven `ErrorCode` values are eleven strings here**, on the `code` property of a real
// `Error` named `OoxmlError`. There is no class to select on, because `catch` selects on nothing in
// JavaScript. The guide says so above the blocks.
//
// Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
// bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const workbookBytes = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "sample.xlsx"));

// guide-example:start
import { Deck } from "@mjx/ooxml";

// `workbookBytes` is a spreadsheet, and `Deck.open` detects that before it parses anything.
let failure;
try {
  Deck.open(workbookBytes);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "UnsupportedFormat") {
  throw new Error("a workbook is not a deck");
}

// The message names the constructor that would have worked, rather than complaining about a
// `presentation.xml` that was never there.
if (!failure.message.includes("Workbook")) {
  throw new Error(failure.message);
}
// guide-example:end
