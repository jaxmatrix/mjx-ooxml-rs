"""The guide's **`Deck`: a surface and a path** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/addressing.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against `crates/mjx-ooxml/examples/guide_addressing_a_deck.rs`
part by part — so a grouping wired to the wrong shape indices changes `ppt/slides/slide1.xml` and
fails there.
"""

# guide-example:start
from mjx_ooxml import Deck, PresetShapeType, ShapeBounds, SlideSize, Surface

deck = Deck.blank(SlideSize.widescreen())
# `add_slide_from_layout` would copy the layout's placeholders too.
slide = Surface.slide(deck.add_slide())
rectangle = ShapeBounds.from_inches(1.0, 1.0, 2.0, 1.0)
ellipse = ShapeBounds.from_inches(4.0, 1.0, 2.0, 1.0)
deck.add_shape(slide, PresetShapeType.Rectangle, rectangle)
deck.add_shape(slide, PresetShapeType.Ellipse, ellipse)
assert deck.shape_count(slide) == 2

# The group itself is one entry on the surface's index space.
group = deck.group_shapes(slide, [0, 1])
assert group.is_top_level

# Member 1 of that group, one step deeper.
member = group.child(1)
assert member.depth == 2
assert len(member.indices) == 2
assert not member.is_top_level
assert member.parent == group

saved = deck.save()
# guide-example:end
