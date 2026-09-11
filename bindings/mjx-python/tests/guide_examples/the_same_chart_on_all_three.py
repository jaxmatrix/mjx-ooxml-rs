"""The guide's **Charts: the same fifty method names on all three** example, through Python.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/one_vocabulary_three_surfaces.md` shows as its `python` block —
literally, because `cargo run -p xtask -- guide-examples` copies it there and
`xtask/tests/guide_examples.rs` proves the copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. It offers **both** packages it
authors — `saved` is the deck and `saved_document` is the Word document — and each is compared
against `crates/mjx-ooxml/examples/guide_the_same_chart_on_all_three.rs` part by part (MJXOFF-260).
"""

# guide-example:start
from mjx_ooxml import ChartData, ChartKind, Deck, Document, PageSize
from mjx_ooxml import ShapeBounds, SlideSize, Surface

chart = (
    ChartData(ChartKind.Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("North", [12.5, 18.0, 21.5])
    .title("Quarterly revenue")
)

# A slide is a canvas in EMU, so a chart on one is laid out inside bounds.
deck = Deck.blank(SlideSize.widescreen())
deck.add_slide()
slide = Surface.slide(0)
bounds = ShapeBounds.from_inches(1.0, 1.0, 5.0, 3.0)
shape = deck.add_chart(slide, chart, bounds)

# A Word drawing is inline in a paragraph, so it takes a width and a height.
document = Document.blank(PageSize.a4())
drawing = document.add_chart(0, chart, 4_572_000, 2_743_200, "Revenue")

# Everything after the address is identical: the same question, the same answer.
on_slide = deck.chart_title(slide, shape)
in_document = document.chart_title(drawing)
assert on_slide == "Quarterly revenue"
assert on_slide == in_document
assert len(deck.chart_series(slide, shape)) == 1
assert len(document.chart_series(drawing)) == 1

saved = deck.save()
saved_document = document.save()
# guide-example:end
