"""The guide's **Saving validates** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against `crates/mjx-ooxml/examples/guide_saving_validates.rs`
part by part.
"""

# guide-example:start
from mjx_ooxml import Deck, Format, SlideSize, detect_format

deck = Deck.blank(SlideSize.widescreen())
deck.validate()  # the same check `save` runs
saved = deck.save()
assert detect_format(saved) == Format.Presentation
# guide-example:end
