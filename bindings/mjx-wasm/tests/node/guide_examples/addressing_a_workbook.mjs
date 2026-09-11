// The guide's **`Workbook`: a tab index and A1 text** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/addressing.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_addressing_a_workbook.rs` part by part.
//
// **There is no `CellInput` here.** An enumeration carrying a payload has no projection, so
// `CellWrite` has one static constructor per kind instead — the same information with the variant
// folded into the function name. The guide says so above the blocks.

// guide-example:start
import { CellWrite, Workbook } from "@mjx/ooxml";

const workbook = Workbook.blank();
workbook.writeCells(0, [
  CellWrite.sharedText("A1", "Region"),
  CellWrite.number("B1", 12.5),
  // The anchoring is data, not address: `$B$2` and `B2` spell one cell.
  CellWrite.number("$B$2", 18.0),
]);

// A block is row-major over the whole requested rectangle, blanks included, and its two
// arguments are offsets *into the block* rather than sheet coordinates.
const block = workbook.readRange(0, "A1:B2");
if (block.firstRow !== 0 || block.firstColumn !== 0) {
  throw new Error("A1 is row 0, column 0");
}
if (block.value(0, 0).text !== "Region" || block.value(1, 1).number !== 18.0) {
  throw new Error("the block holds what was written into it");
}
if (block.range !== "A1:B2" || workbook.usedRange(0) !== "A1:B2") {
  throw new Error("and it covers exactly the range that was asked for");
}

const saved = workbook.save();

// a wasm handle owns memory the garbage collector cannot see
block.free();
workbook.free();
// guide-example:end

export { saved };
