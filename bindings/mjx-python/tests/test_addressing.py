"""`Surface` and `ShapePath`: the hand-written conversions, and what they refuse.

The whole ergonomic claim of this binding is that `deck.shape_kind(0, 2)` means what
`deck.shape_kind(slide.into(), 2.into())` means in Rust. These tests hold that claim to the same
standard the Rust `address.rs` unit tests hold theirs: every spelling reaches the *same shape*, and
every spelling that should not be accepted raises.
"""

from __future__ import annotations

import pytest

from mjx_ooxml import (
    Deck,
    IndexOutOfRangeError,
    PresetShapeType,
    ShapeBounds,
    ShapeKind,
    ShapePath,
    SlideSize,
    Surface,
)


def deck_with_a_group() -> tuple[Deck, int, int]:
    """A deck whose slide 0 holds a group of two shapes at top-level index 0."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    first = deck.add_shape(
        slide, PresetShapeType.Ellipse, ShapeBounds.from_inches(1, 1, 1, 1)
    )
    second = deck.add_shape(
        slide, PresetShapeType.Rectangle, ShapeBounds.from_inches(3, 1, 1, 1)
    )
    group = deck.group_shapes(slide, [first, second])
    assert group.is_top_level
    return deck, slide, group.indices[0]


def test_every_spelling_of_a_surface_reaches_the_same_part() -> None:
    """An int, a `Surface.slide`, and a `Surface` built from the same index are one address."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    deck.add_shape(slide, PresetShapeType.Ellipse, ShapeBounds.from_inches(1, 1, 1, 1))

    assert deck.shape_count(slide) == 1
    assert deck.shape_count(Surface.slide(slide)) == 1

    # And the other four kinds are reachable only by naming them, which is the point of the class.
    assert deck.shape_count(Surface.master(0)) >= 1
    assert deck.shape_count(Surface.layout(0)) >= 0
    assert Surface.notes_master().kind == "notes master"
    assert Surface.layout(1).index == 1
    assert Surface.master(0).is_master_like and not Surface.slide(0).is_master_like


def test_every_spelling_of_a_shape_address_reaches_the_same_shape() -> None:
    """An int, a one-element list, and a `ShapePath` all name the same top-level shape."""
    deck, slide, group = deck_with_a_group()

    by_int = deck.shape_kind(slide, group)
    by_list = deck.shape_kind(slide, [group])
    by_path = deck.shape_kind(slide, ShapePath.top(group))
    assert by_int == by_list == by_path == ShapeKind.GroupShape

    # A descent into the group is a list, exactly as it is a `[2, 1]` in Rust.
    assert deck.shape_kind(slide, [group, 0]) == ShapeKind.Shape
    assert deck.shape_kind(slide, ShapePath.of([group, 1])) == ShapeKind.Shape
    assert deck.shape_member_count(slide, group) == 2


def test_a_shape_path_says_where_it_is() -> None:
    """The address arithmetic — depth, child, parent — matches the Rust type exactly."""
    top = ShapePath.top(2)
    member = ShapePath.of([2, 1])
    assert top.indices == [2]
    assert member.indices == [2, 1]
    assert top.depth == 1 and member.depth == 2
    assert top.is_top_level and not member.is_top_level
    assert top.child(1) == member
    assert member.parent == top
    assert top.parent is None
    assert str(top) == "2" and str(member) == "[2, 1]"


def test_addresses_are_hashable_so_they_can_key_a_dictionary() -> None:
    """A caller tracking work by address needs these in a `dict` and a `set`."""
    seen = {Surface.slide(0): "first", Surface.layout(0): "layout"}
    assert seen[Surface.slide(0)] == "first"
    assert len({ShapePath.top(1), ShapePath.of([1]), ShapePath.of([1, 0])}) == 2


@pytest.mark.parametrize(
    "bad_surface",
    [True, "0", 1.5, None, [0], ShapePath.top(0)],
    ids=["bool", "str", "float", "None", "list", "ShapePath"],
)
def test_a_surface_that_is_neither_an_int_nor_a_surface_is_refused(bad_surface: object) -> None:
    """A `bool` is an `int` in Python; `deck.shape_count(True)` is never what anyone meant."""
    deck = Deck.blank(SlideSize.widescreen())
    with pytest.raises(TypeError, match="surface"):
        deck.shape_count(bad_surface)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    "bad_path",
    [True, "0", 1.5, None, Surface.slide(0), ["a"]],
    ids=["bool", "str", "float", "None", "Surface", "list-of-str"],
)
def test_a_shape_address_of_the_wrong_shape_is_refused(bad_path: object) -> None:
    """The message names what a shape address is, rather than complaining about an element."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    with pytest.raises(TypeError, match="shape address"):
        deck.shape_kind(slide, bad_path)  # type: ignore[arg-type]


def test_an_empty_shape_address_is_refused_by_value_not_by_type() -> None:
    """`[]` is a well-typed sequence of ints and still not an address: the tree is not a shape."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    with pytest.raises(ValueError, match="at least one index"):
        deck.shape_kind(slide, [])


def test_an_out_of_range_address_raises_where_it_happened() -> None:
    """The exception carries the address that failed, in the classes the caller passed."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    with pytest.raises(IndexOutOfRangeError) as failure:
        deck.shape_kind(slide, 7)
    assert failure.value.surface == Surface.slide(slide)
    assert failure.value.shape == ShapePath.top(7)
