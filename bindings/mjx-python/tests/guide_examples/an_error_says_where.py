"""The guide's **`detail` says where** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/errors.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — it asks a
slide for a shape it has not got — so there is nothing for the harness to compare and it skips that
half by construction.

**The five coordinates are attributes on the exception here**, always present and `None` where the
failure knew no such coordinate, so none of them ever raises `AttributeError`. The guide says so
above the blocks.
"""

# guide-example:start
from mjx_ooxml import Deck, IndexOutOfRangeError, SlideSize, Surface

deck = Deck.blank(SlideSize.widescreen())
slide = Surface.slide(deck.add_slide_from_layout(0))

# The slide carries the layout's placeholders and nothing at index 4.
try:
    deck.shape_bounds(slide, 4)
    raise AssertionError("no shape 4")
except IndexOutOfRangeError as failure:
    assert failure.code == "IndexOutOfRange"

    # The failure says *where*, in the same addressing the call used to get there.
    assert failure.surface == slide
    shape = failure.shape
    assert shape is not None and shape.indices == [4]
    assert failure.row is None and failure.column is None
# guide-example:end
