// The guide's **`Document`: a block and a run** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/addressing.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_addressing_a_document.rs` part by part.

// guide-example:start
import { Document, PageSize } from "@mjx/ooxml";

const document = Document.blank(PageSize.a4());
document.appendParagraph();
document.appendRun(0, "Quarterly ");
document.appendRun(0, "results");
if (document.runCount(0) !== 2) {
  throw new Error("the paragraph should carry two runs");
}
if (document.runText(0, 1) !== "results") {
  throw new Error("run 1 is the second run of paragraph 0");
}
if (document.paragraphText(0) !== "Quarterly results") {
  throw new Error("a paragraph's text is its runs, concatenated");
}

const saved = document.save();
document.free(); // a wasm handle owns memory the garbage collector cannot see
// guide-example:end

export { saved };
