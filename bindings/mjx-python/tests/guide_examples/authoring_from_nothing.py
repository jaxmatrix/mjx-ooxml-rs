"""The guide's **Authoring from nothing** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved`, because
what it demonstrates is that a package can be built with no template and no input file at all — so
there is nothing for the harness to compare, and it skips that half of its work by construction.
"""

# guide-example:start
from mjx_ooxml import Deck, Document, PageSize, SlideSize, Workbook

# One master, one layout, a theme — and no slides yet.
deck = Deck.blank(SlideSize.widescreen())
assert deck.slide_count() == 0
assert deck.master_count() == 1

# One empty paragraph, because a `w:body` needs one.
document = Document.blank(PageSize.a4())
assert document.paragraph_count() == 1

# One empty worksheet, named Sheet1.
workbook = Workbook.blank()
assert workbook.sheet_count() == 1
# guide-example:end
