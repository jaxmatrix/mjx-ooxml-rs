// The guide's **Detection reads the package, not the filename** example, through the WebAssembly
// binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — it only reads a format out of bytes — so there is nothing for the harness to compare
// and it skips that half by construction.
//
// **A `Format`'s five accessors are free functions here and methods in Rust**, because a
// `#[wasm_bindgen]` enumeration is a number in JavaScript and a number cannot carry a getter. The
// guide says so above the blocks; the difference is the projection's, not this file's.
//
// Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
// bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const data = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "sample.docx"));

// guide-example:start
import {
  Format,
  FormatFamily,
  detectFormat,
  formatConventionalExtension,
  formatFamily,
  formatIsEditable,
  formatIsMacroEnabled,
} from "@mjx/ooxml";

// `data` is a Word document. Nothing here looks at a filename: detection opens the container,
// follows the root `officeDocument` relationship and reads the content type it lands on.
const format = detectFormat(data);
if (format !== Format.Document) {
  throw new Error("these bytes are a Word document");
}
if (formatFamily(format) !== FormatFamily.WordProcessing) {
  throw new Error("and its family is WordProcessing");
}
if (formatConventionalExtension(format) !== "docx") {
  throw new Error("whose conventional extension is docx");
}
if (!formatIsEditable(format) || formatIsMacroEnabled(format)) {
  throw new Error("this build can edit it, and it carries no macros");
}
// guide-example:end
