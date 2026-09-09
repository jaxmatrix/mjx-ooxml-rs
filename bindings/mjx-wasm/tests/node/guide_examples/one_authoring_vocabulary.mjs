// The guide's **The authoring vocabulary is one vocabulary** example, through the WebAssembly
// binding.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md` shows as its `js` block —
// literally, because `cargo run -p xtask -- guide-examples` copies it there and
// `xtask/tests/guide_examples.rs` proves the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the
// **workbook**, compared against `crates/mjx-ooxml/examples/guide_one_authoring_vocabulary.rs` part
// by part — the deck this example also authors is covered by `tests/node/build_a_deck.mjs`, and
// `setChartSeriesFill` is covered by no walkthrough at all.

// guide-example:start
import { ChartData, ChartKind, ColorSpec, Deck, FillSpec } from "@mjx/ooxml";
import { PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook } from "@mjx/ooxml";

const navy = FillSpec.solid(ColorSpec.srgb("1F3864"));

// On a shape in a deck.
const deck = Deck.blank(SlideSize.widescreen());
const slide = Surface.slide(deck.addSlideFromLayout(0));
const bounds = ShapeBounds.fromInches(1.0, 1.0, 2.0, 1.0);
const shape = deck.addShape(slide, PresetShapeType.Rectangle, bounds);
deck.setShapeFill(slide, shape, navy);
if (deck.shapeFill(slide, shape) === undefined) {
  throw new Error("the shape should carry the fill just set");
}

// The same value, on a chart series in a workbook.
const workbook = Workbook.blank();
const chart = new ChartData(ChartKind.Bar).categories(["Q1"]).series("North", [12.5]);
const resizing = ResizingBehavior.MoveAndResizeWithAnchorCells;
const anchor = workbook.addChart(0, chart, 1, 1, 7, 16, "Revenue", resizing);
workbook.setChartSeriesFill(0, anchor, 0, navy);
if (workbook.chartSeriesFill(0, anchor, 0) === undefined) {
  throw new Error("the series should carry the fill just set");
}

const saved = workbook.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [navy, deck, slide, bounds, workbook, chart]) {
  handle.free();
}
// guide-example:end

export { saved };
