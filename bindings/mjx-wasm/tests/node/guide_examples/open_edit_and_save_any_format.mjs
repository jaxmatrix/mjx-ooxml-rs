// The guide's **Bytes in, bytes out** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/README.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_open_edit_and_save_any_format.rs` part by part.
//
// **`formatFamily` is a free function here**, because a `#[wasm_bindgen]` enumeration is a number
// in JavaScript and a number cannot carry a getter — and the dispatch is a `switch` rather than a
// `match`. The guide says so above the blocks.
//
// Reading and writing the file are the caller's job, so both are outside the block: the library is
// bytes in and bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const data = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "sample.pptx"));

// guide-example:start
import { Deck, Document, FormatFamily, Workbook, detectFormat, formatFamily } from "@mjx/ooxml";

// `data` is whatever the caller read. Which of the three surfaces opens it is the package's
// answer, not the filename's.
let opened;
switch (formatFamily(detectFormat(data))) {
  case FormatFamily.Presentation:
    opened = Deck.open(data);
    break;
  case FormatFamily.WordProcessing:
    opened = Document.open(data);
    break;
  case FormatFamily.Spreadsheet:
    opened = Workbook.open(data);
    break;
  default:
    // A fourth family would be another case here, not a broken program.
    throw new Error("unhandled format family");
}
const saved = opened.save();
opened.free(); // a wasm handle owns memory the garbage collector cannot see
if (saved.length === 0) {
  throw new Error("a saved package is never empty");
}
// guide-example:end

export { saved };
