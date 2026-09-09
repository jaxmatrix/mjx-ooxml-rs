// The guide's **Authoring from nothing** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved`, because what it demonstrates is that a package can be built with no template and no input
// file at all — so there is nothing for the harness to compare, and it skips that half of its work
// by construction.

// guide-example:start
import { Deck, Document, PageSize, SlideSize, Workbook } from "@mjx/ooxml";

// One master, one layout, a theme — and no slides yet.
const deck = Deck.blank(SlideSize.widescreen());
if (deck.slideCount() !== 0 || deck.masterCount() !== 1) {
  throw new Error("a blank deck is one master, one layout and no slides");
}

// One empty paragraph, because a `w:body` needs one.
const document = Document.blank(PageSize.a4());
if (document.paragraphCount() !== 1) {
  throw new Error("a blank document has one empty paragraph");
}

// One empty worksheet, named Sheet1.
const workbook = Workbook.blank();
if (workbook.sheetCount() !== 1) {
  throw new Error("a blank workbook has one worksheet");
}

deck.free(); // a wasm handle owns memory the garbage collector cannot see
document.free();
workbook.free();
// guide-example:end
