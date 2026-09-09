"""The guide's **The authoring vocabulary is one vocabulary** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md` shows as its `python` block —
literally, because `cargo run -p xtask -- guide-examples` copies it there and
`xtask/tests/guide_examples.rs` proves the copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the **workbook**, compared
against `crates/mjx-ooxml/examples/guide_one_authoring_vocabulary.rs` part by part — the deck this
example also authors is covered by `tests/test_build_a_deck.py`, and `set_chart_series_fill` is
covered by no walkthrough at all.
"""

# guide-example:start
from mjx_ooxml import ChartData, ChartKind, ColorSpec, Deck, FillSpec
from mjx_ooxml import PresetShapeType, ResizingBehavior, ShapeBounds, SlideSize, Surface, Workbook

navy = FillSpec.solid(ColorSpec.srgb("1F3864"))

# On a shape in a deck.
deck = Deck.blank(SlideSize.widescreen())
slide = Surface.slide(deck.add_slide_from_layout(0))
bounds = ShapeBounds.from_inches(1.0, 1.0, 2.0, 1.0)
shape = deck.add_shape(slide, PresetShapeType.Rectangle, bounds)
deck.set_shape_fill(slide, shape, navy)
assert deck.shape_fill(slide, shape) is not None

# The same value, on a chart series in a workbook.
workbook = Workbook.blank()
chart = ChartData(ChartKind.Bar).categories(["Q1"]).series("North", [12.5])
resizing = ResizingBehavior.MoveAndResizeWithAnchorCells
anchor = workbook.add_chart(0, chart, 1, 1, 7, 16, "Revenue", resizing)
workbook.set_chart_series_fill(0, anchor, 0, navy)
assert workbook.chart_series_fill(0, anchor, 0) is not None

saved = workbook.save()
# guide-example:end
