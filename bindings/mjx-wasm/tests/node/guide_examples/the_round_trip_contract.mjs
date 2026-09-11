// The guide's **The contract** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_the_round_trip_contract.rs` part by part.
//
// **This is the project's central claim, checked inside the block a reader sees.** Not "three
// languages agree with each other" — each half is checked against the input file, so three
// languages agreeing on a wrong answer would not pass. The harness comparison on top is a second,
// different fact: that all three preserved it the same way.
//
// Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
// bytes out and never touches a filesystem.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../../..");
const original = readFileSync(join(REPOSITORY_ROOT, "tests", "fixtures", "sample.xlsx"));

// guide-example:start
import { Workbook } from "@mjx/ooxml";

// One edit: the first tab's name, which `xl/workbook.xml` states and no other part does.
const workbook = Workbook.open(original);
workbook.renameSheet(0, "Revised");
const saved = workbook.save();

const before = Workbook.open(original);
const after = Workbook.open(saved);
const names = before.partNames();
if (names.join("\n") !== after.partNames().join("\n")) {
  throw new Error("no part appeared or vanished");
}

const changed = names.filter((part) => {
  const was = before.partBytes(part);
  const now = after.partBytes(part);
  return was.length !== now.length || was.some((byte, at) => byte !== now[at]);
});
// Everything else came back byte for byte — the whole of the contract.
if (changed.join() !== "/xl/workbook.xml") {
  throw new Error(`one edit touched ${changed.join(", ")}`);
}

// a wasm handle owns memory the garbage collector cannot see
workbook.free();
before.free();
after.free();
// guide-example:end

export { saved };
