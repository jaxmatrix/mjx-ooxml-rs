"""The exception hierarchy: what is raised, what it carries, and what catches it."""

from __future__ import annotations

import pytest

import mjx_ooxml
from mjx_ooxml import Deck, OoxmlError, PresetShapeType, ShapeBounds, ShapePath, SlideSize, Surface

CODES = [
    ("IoError", "Io"),
    ("MalformedDocumentError", "MalformedDocument"),
    ("InvalidDocumentError", "InvalidDocument"),
    ("IndexOutOfRangeError", "IndexOutOfRange"),
    ("WrongKindError", "WrongKind"),
    ("NotFoundError", "NotFound"),
    ("NothingToReadError", "NothingToRead"),
    ("InvalidArgumentError", "InvalidArgument"),
    ("StructureConflictError", "StructureConflict"),
    ("UnsupportedContentError", "UnsupportedContent"),
    ("UnsupportedFormatError", "UnsupportedFormat"),
]


@pytest.mark.parametrize("name,_code", CODES)
def test_every_code_has_a_class_rooted_at_ooxml_error(name: str, _code: str) -> None:
    """Eleven classes, one per stable code, all catchable as one."""
    cls = getattr(mjx_ooxml, name)
    assert issubclass(cls, OoxmlError)
    assert cls.__module__ == "mjx_ooxml"


def test_index_out_of_range_is_also_an_index_error() -> None:
    """Code that already guards a lookup with `except IndexError` keeps working."""
    assert issubclass(mjx_ooxml.IndexOutOfRangeError, IndexError)
    assert issubclass(mjx_ooxml.IndexOutOfRangeError, OoxmlError)
    # The order matters for `except`: the more specific class must come first in the MRO.
    mro = mjx_ooxml.IndexOutOfRangeError.__mro__
    assert mro.index(OoxmlError) < mro.index(IndexError)

    deck = Deck.blank(SlideSize.widescreen())
    caught: Exception | None = None
    try:
        deck.shape_count(99)
    except IndexError as failure:  # deliberately the *built-in*
        caught = failure
    assert isinstance(caught, mjx_ooxml.IndexOutOfRangeError)


def test_a_failure_carries_the_coordinates_it_had() -> None:
    """`.surface` and `.shape` come back in the classes the caller passed, not as prose."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    with pytest.raises(mjx_ooxml.IndexOutOfRangeError) as failure:
        deck.shape_text(slide, [4, 2])
    assert failure.value.code == "IndexOutOfRange"
    assert failure.value.surface == Surface.slide(slide)
    assert failure.value.shape == ShapePath.of([4, 2])
    assert failure.value.row is None and failure.value.column is None
    assert str(failure.value)


def test_a_cell_failure_carries_a_row_and_a_column() -> None:
    """A different code carries different coordinates, and the unused ones are `None`."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    table = deck.add_table(slide, 2, 2, ShapeBounds.from_inches(1, 1, 4, 2))
    with pytest.raises(mjx_ooxml.IndexOutOfRangeError) as failure:
        deck.cell_text(slide, table, 5, 9)
    assert failure.value.row == 5
    assert failure.value.column == 9
    assert failure.value.index is None


def test_every_coordinate_attribute_exists_even_when_empty() -> None:
    """A handler reading `.row` on an I/O failure must get `None`, not `AttributeError`."""
    with pytest.raises(OoxmlError) as failure:
        Deck.open(b"not a zip archive at all")
    for attribute in ("code", "surface", "shape", "row", "column", "index"):
        assert hasattr(failure.value, attribute)
    assert failure.value.code == "Io"
    assert failure.value.surface is None


def test_the_wrong_kind_of_shape_is_a_different_class() -> None:
    """Asking a rectangle for its table dimensions is `WrongKindError`, not an index failure."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    shape = deck.add_shape(
        slide, PresetShapeType.Rectangle, ShapeBounds.from_inches(1, 1, 2, 2)
    )
    with pytest.raises(mjx_ooxml.WrongKindError) as failure:
        deck.table_dimensions(slide, shape)
    assert failure.value.code == "WrongKind"


def test_an_argument_refused_before_anything_is_written() -> None:
    """A slide size outside what `p:sldSz` can express fails before a deck exists."""
    with pytest.raises(mjx_ooxml.InvalidArgumentError) as failure:
        Deck.blank(SlideSize.from_emu(10, 10))
    assert failure.value.code == "InvalidArgument"


def test_a_structure_conflict_is_its_own_class() -> None:
    """Grouping shapes that are not siblings conflicts with the tree, and says so."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    only = deck.add_shape(
        slide, PresetShapeType.Rectangle, ShapeBounds.from_inches(1, 1, 1, 1)
    )
    with pytest.raises(OoxmlError) as failure:
        deck.group_shapes(slide, [only])
    assert failure.value.code in ("InvalidArgument", "StructureConflict")


def test_a_deck_survives_the_failures_it_reports() -> None:
    """A raised exception releases the deck's borrow: the next call works."""
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    for _ in range(5):
        with pytest.raises(OoxmlError):
            deck.shape_text(slide, 42)
    shape = deck.add_text_box(slide, "still here", ShapeBounds.from_inches(1, 1, 3, 1))
    assert deck.shape_text(slide, shape) == "still here"
