"""The guide's **One error type, eleven codes** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/README.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — it asks a
blank deck for a slide it does not have — so there is nothing for the harness to compare and it
skips that half by construction.

**The five `ErrorDetail` coordinates are attributes on the exception here**, always present and
`None` where the failure carried no such coordinate, so none of them ever raises `AttributeError`.
`IndexOutOfRangeError` is also a Python `IndexError`, which is why `except IndexError` would catch
this too. The guide says both above the blocks.
"""

# guide-example:start
from mjx_ooxml import Deck, IndexOutOfRangeError, SlideSize

deck = Deck.blank(SlideSize.widescreen())

# A blank deck has no slides at all, so slide 7 is past the end.
try:
    deck.shape_count(7)
    raise AssertionError("no slide 7")
except IndexOutOfRangeError as failure:
    assert failure.code == "IndexOutOfRange"
    assert failure.index == 7
    assert str(failure) == "slide index 7 out of range (0..0)"
# guide-example:end
