// The guide's **`Deck`: a surface and a path** example, through the WebAssembly binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/addressing.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which is what the harness compares against
// `crates/mjx-ooxml/examples/guide_addressing_a_deck.rs` part by part.

// guide-example:start
import { Deck, PresetShapeType, ShapeBounds, SlideSize, Surface } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
// `addSlideFromLayout` would copy the layout's placeholders too.
const slide = Surface.slide(deck.addSlide());
const rectangle = ShapeBounds.fromInches(1.0, 1.0, 2.0, 1.0);
const ellipse = ShapeBounds.fromInches(4.0, 1.0, 2.0, 1.0);
deck.addShape(slide, PresetShapeType.Rectangle, rectangle);
deck.addShape(slide, PresetShapeType.Ellipse, ellipse);
if (deck.shapeCount(slide) !== 2) {
  throw new Error("the slide should carry two shapes");
}

// The group itself is one entry on the surface's index space.
const group = deck.groupShapes(slide, [0, 1]);
// Member 1 of that group, one step deeper.
const member = group.child(1);
const parent = member.parent;
if (!group.isTopLevel || member.isTopLevel) {
  throw new Error("the group is top level and its member is not");
}
if (member.depth !== 2 || member.indices.length !== 2) {
  throw new Error("a group member's address is two indices deep");
}
if (!parent.equals(group)) {
  throw new Error("a member's parent is the group it belongs to");
}

const saved = deck.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [deck, slide, rectangle, ellipse, group, member, parent]) {
  handle.free();
}
// guide-example:end

export { saved };
