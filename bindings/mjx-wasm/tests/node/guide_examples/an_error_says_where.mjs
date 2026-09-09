// The guide's **`detail` says where** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/errors.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. This one exports no
// `saved` — it asks a slide for a shape it has not got — so there is nothing for the harness to
// compare and it skips that half by construction.
//
// **`detail` is a plain object here, carrying only the coordinates the failure had.** A caller
// reads `detail.row ?? null` rather than expecting a key. The guide says so above the blocks.

// guide-example:start
import { Deck, SlideSize, Surface } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
const slide = Surface.slide(deck.addSlideFromLayout(0));

// The slide carries the layout's placeholders and nothing at index 4.
let failure;
try {
  deck.shapeBounds(slide, 4);
} catch (raised) {
  failure = raised;
}
if (failure?.code !== "IndexOutOfRange") {
  throw new Error("no shape 4");
}

// The failure says *where*, in the same addressing the call used to get there.
if (failure.detail.surface.kind !== "slide" || failure.detail.surface.index !== slide.index) {
  throw new Error("the failure names the surface the call named");
}
if (failure.detail.shape.indices.join() !== "4") {
  throw new Error("and the shape address it was asked for");
}
if (failure.detail.row !== undefined || failure.detail.column !== undefined) {
  throw new Error("a coordinate the failure did not have is simply not a key");
}

// a wasm handle owns memory the garbage collector cannot see
failure.detail.surface.free();
failure.detail.shape.free();
slide.free();
deck.free();
// guide-example:end
