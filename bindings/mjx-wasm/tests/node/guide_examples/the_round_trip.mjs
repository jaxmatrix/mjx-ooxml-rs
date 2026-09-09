// The guide's **The round trip** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_the_round_trip.rs` part by part.
//
// Unlike `saving_validates`, **this example authors nothing.** Every part of the package it saves
// was written by whoever produced `tests/fixtures/sample.xlsx`, so the checks below are the
// round-trip contract — copy-on-write and verbatim re-emission — rather than a check that this
// library agrees with itself. Reading the file is above the sentinel because it is the caller's
// job: the library is bytes in and bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const original = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "sample.xlsx"));

// guide-example:start
import { Workbook } from "@mjx/ooxml";

// `original` is the file's bytes. Reading them is the caller's job in every one of the three
// languages: this library is bytes in and bytes out and never touches a filesystem.
const opened = Workbook.open(original);
const saved = opened.save();
opened.free();

// Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
// `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
const before = Workbook.open(original);
const after = Workbook.open(saved);
const names = before.partNames();
if (names.join("\n") !== after.partNames().join("\n")) {
  throw new Error("a part appeared or vanished");
}
for (const part of names) {
  const was = before.partBytes(part);
  const now = after.partBytes(part);
  if (was.length !== now.length || was.some((byte, at) => byte !== now[at])) {
    throw new Error(`${part} changed`);
  }
}
before.free(); // a wasm handle owns memory the garbage collector cannot see
after.free();
// guide-example:end

export { saved };
