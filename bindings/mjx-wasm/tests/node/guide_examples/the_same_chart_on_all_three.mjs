// The guide's **Charts: the same fifty method names on all three** example, through WebAssembly.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md` shows as its `js` block —
// literally, because `cargo run -p xtask -- guide-examples` copies it there and
// `xtask/tests/guide_examples.rs` proves the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the deck it
// produced, compared against `crates/mjx-ooxml/examples/guide_the_same_chart_on_all_three.rs` part
// by part. The Word document the example also authors is held by the readers the block calls; a Word
// chart is compared byte for byte by `tests/node/build_a_document.mjs`.

// guide-example:start
import { ChartData, ChartKind, Deck, Document, PageSize } from "@mjx/ooxml";
import { ShapeBounds, SlideSize, Surface } from "@mjx/ooxml";

const chart = new ChartData(ChartKind.Bar)
  .categories(["Q1", "Q2", "Q3"])
  .series("North", [12.5, 18.0, 21.5])
  .title("Quarterly revenue");

// A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
const deck = Deck.blank(SlideSize.widescreen());
deck.addSlide();
const slide = Surface.slide(0);
const bounds = ShapeBounds.fromInches(1.0, 1.0, 5.0, 3.0);
const shape = deck.addChart(slide, chart, bounds);

// A Word drawing is inline in a paragraph, so it takes a width and a height.
const document = Document.blank(PageSize.a4());
const drawing = document.addChart(0, chart, 4_572_000, 2_743_200, "Revenue");

// Everything after the address is identical: the same question, the same answer.
const onSlide = deck.chartTitle(slide, shape);
const inDocument = document.chartTitle(drawing);
if (onSlide !== "Quarterly revenue" || onSlide !== inDocument) {
  throw new Error("both surfaces answer the same title for the same chart");
}
if (deck.chartSeries(slide, shape).length !== 1) {
  throw new Error("the slide's chart holds one series");
}
if (document.chartSeries(drawing).length !== 1) {
  throw new Error("the document's chart holds one series");
}

const saved = deck.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [chart, deck, document, slide, bounds]) {
  handle.free();
}
// guide-example:end

export { saved };
