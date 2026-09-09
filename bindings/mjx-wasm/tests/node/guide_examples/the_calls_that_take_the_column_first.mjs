// The guide's **Row first, except where the file says otherwise** example, through WebAssembly.
//
// Everything between the two `guide-example` sentinels is what
// `crates/mjx-ooxml/docs/guide/addressing.md` shows as its `js` block — literally, because
// `cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves
// the copy is current.
//
// Importing this module *is* running the example: the code below is top level, so
// `tests/node/guide_examples.mjs` executes every check in it by importing it. `saved` is the package
// it produced, which the harness compares against
// `crates/mjx-ooxml/examples/guide_the_calls_that_take_the_column_first.rs` part by part — and that
// comparison is the point of this particular example, because a transposed `(column, row)` pair is
// still four valid numbers. It changes `xl/drawings/drawing1.xml` and nothing else would notice.

// guide-example:start
import { ChartData, ChartKind, ResizingBehavior, Workbook } from "@mjx/ooxml";

const workbook = Workbook.blank();
const chart = new ChartData(ChartKind.Bar).categories(["Q1", "Q2"]).series("North", [12.5, 18.0]);

// From column 1, row 1 (B2) to column 7, row 16 (H17) — column first, both times.
const resizing = ResizingBehavior.MoveAndResizeWithAnchorCells;
const anchor = workbook.addChart(0, chart, 1, 1, 7, 16, "Revenue", resizing);
const anchors = workbook.chartAnchorIndices(0);
if (anchors.length !== 1 || anchors[0] !== anchor) {
  throw new Error("the sheet should carry exactly the chart just added");
}

const saved = workbook.save();
// a wasm handle owns memory the garbage collector cannot see
for (const handle of [workbook, chart]) {
  handle.free();
}
// guide-example:end

export { saved };
