// The guide's **Saving validates** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_saving_validates.rs` part by part.
//
// `@mjx/ooxml` resolves here to `bindings/mjx-wasm/npm/`, which `build-npm.sh` links into
// `tests/node/node_modules/` exactly as `npm link` would — so the specifier the guide shows is the
// specifier a consumer writes, and it is the one this file really imports.

// guide-example:start
import { Deck, Format, SlideSize, detectFormat } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
deck.validate(); // the same check `save` runs
const saved = deck.save();
if (detectFormat(saved) !== Format.Presentation) {
  throw new Error("the saved package is not a presentation");
}
deck.free(); // a wasm handle owns memory the garbage collector cannot see
// guide-example:end

export { saved };
