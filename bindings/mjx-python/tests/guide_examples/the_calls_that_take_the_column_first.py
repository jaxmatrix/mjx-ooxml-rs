"""The guide's **Row first, except where the file says otherwise** example, through Python.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/addressing.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which the harness compares against
`crates/mjx-ooxml/examples/guide_the_calls_that_take_the_column_first.rs` part by part — and that
comparison is the point of this particular example, because a transposed `(column, row)` pair is
still four valid numbers. It changes `xl/drawings/drawing1.xml` and nothing else would notice.
"""

# guide-example:start
from mjx_ooxml import ChartData, ChartKind, ResizingBehavior, Workbook

workbook = Workbook.blank()
chart = ChartData(ChartKind.Bar).categories(["Q1", "Q2"]).series("North", [12.5, 18.0])

# From column 1, row 1 (B2) to column 7, row 16 (H17) — column first, both times.
resizing = ResizingBehavior.MoveAndResizeWithAnchorCells
anchor = workbook.add_chart(0, chart, 1, 1, 7, 16, "Revenue", resizing)
assert workbook.chart_anchor_indices(0) == [anchor]

saved = workbook.save()
# guide-example:end
