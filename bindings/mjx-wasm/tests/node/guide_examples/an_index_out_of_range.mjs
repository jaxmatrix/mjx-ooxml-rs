// The guide's **One error type, eleven codes** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/README.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — it asks a blank deck for a slide it does not have — so there is nothing for the harness
// to compare and it skips that half by construction.
//
// **The five `ErrorDetail` coordinates are keys on a plain `detail` object here**, and only the
// ones the failure actually carried are present — a caller reads `detail.index ?? null`. The guide
// says so above the blocks.

// guide-example:start
import { Deck, SlideSize } from "@mjx/ooxml";

const size = SlideSize.widescreen();
const deck = Deck.blank(size);

// A blank deck has no slides at all, so slide 7 is past the end.
let failure;
try {
  deck.shapeCount(7);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "IndexOutOfRange" || failure.detail.index !== 7) {
  throw new Error("slide 7 is out of range, and the failure says which index");
}
if (failure.message !== "slide index 7 out of range (0..0)") {
  throw new Error(failure.message);
}

// a wasm handle owns memory the garbage collector cannot see
size.free();
deck.free();
// guide-example:end
