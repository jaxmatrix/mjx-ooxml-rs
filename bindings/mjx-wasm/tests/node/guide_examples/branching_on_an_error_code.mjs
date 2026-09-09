// The guide's **Eleven codes, and what each one means you should do** example, through the
// WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/errors.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — nothing was written, which is its last check — so there is nothing for the harness to
// compare and it skips that half by construction.
//
// **One string per code, and no `CellInput`.** Both differences are the projection's, and both are
// stated in the guide above the blocks.

// guide-example:start
import { CellWrite, Workbook } from "@mjx/ooxml";

/** The code a call raised, or `undefined` if it did not raise. */
function codeOf(call) {
  try {
    call();
  } catch (raised) {
    return raised.code;
  }
  return undefined;
}

const workbook = Workbook.blank();

// An address that does not parse: refused before the worksheet is even opened.
const badAddress = codeOf(() => workbook.writeCells(0, [CellWrite.number("not-a-cell", 1.0)]));
if (badAddress !== "InvalidArgument") {
  throw new Error("`not-a-cell` is not an A1 reference");
}

// A value SpreadsheetML has no spelling for.
const unrepresentable = codeOf(() => workbook.writeCells(0, [CellWrite.number("A1", NaN)]));
if (unrepresentable !== "InvalidArgument") {
  throw new Error("SpreadsheetML cannot spell NaN");
}

// A tab that is not there.
if (codeOf(() => workbook.readRange(9, "A1")) !== "IndexOutOfRange") {
  throw new Error("there is one sheet");
}

// And the contract worth knowing: a batch that would fail halfway is refused before the
// package is touched at all, so nothing above wrote anything.
const sheet = workbook.readSheet(0);
if (!sheet.isEmpty) {
  throw new Error("three refusals wrote nothing");
}

// a wasm handle owns memory the garbage collector cannot see
sheet.free();
workbook.free();
// guide-example:end
