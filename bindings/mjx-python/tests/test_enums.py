"""The seventy-four enumerations: complete, correctly renamed, and round-tripping through a deck."""

from __future__ import annotations

import inspect

import pytest

import mjx_ooxml
from mjx_ooxml import (
    CellBorder,
    Deck,
    PresetShapeType,
    SchemeColor,
    ShapeBounds,
    SlideSize,
    TextUnderline,
)

# Every enumeration whose Rust `None` variant had to be renamed, because `None` is a Python keyword
# and `TextUnderline.None` is a syntax error rather than a lookup.
RENAMED_NONE = [
    "FontCollectionIndex",
    "LineEndType",
    "PathFillMode",
    "PictureFillMode",
    "ScatterStyle",
    "TextCapitalization",
    "TextUnderline",
    "TickLabelPosition",
    "TickMark",
]

# The member counts that must not silently shrink. Each is the number of variants the model states,
# so a projection that dropped one would be caught here rather than at a caller's call site.
MEMBER_COUNTS = {
    "PresetShapeType": 187,
    "AutonumberScheme": 41,
    "PatternType": 54,
    "PresetCamera": 62,
    "SlideLayoutKind": 36,
    "LightRigType": 27,
    "PresetShadow": 20,
    "TextUnderline": 18,
    "SchemeColor": 17,
    "ChartKind": 16,
    "PlaceholderType": 16,
    "SlideSizeKind": 16,
    "PresetMaterial": 15,
    "Format": 15,
    "TableStylePart": 13,
    "BevelPreset": 12,
    "ColorSchemeSlot": 12,
    "PresetLineDash": 11,
    "DataLabelPosition": 9,
    "RectangleAlignment": 9,
    "TableStyleBorder": 8,
    "LightRigDirection": 8,
    "TablePart": 7,
    "ColorKind": 7,
    "TextAlignment": 7,
    "TextDirection": 7,
    "CellBorder": 6,
    "LineEndType": 6,
    "PathFillMode": 6,
    "ScatterStyle": 6,
    "ShapeKind": 6,
    "TrendlineKind": 6,
    "FontSlot": 4,
    "AxisKind": 4,
    "AdjustmentAxis": 4,
    "FormatFamily": 3,
    "MediaKind": 3,
    "TargetMode": 2,
    "FontSchemeSlot": 2,
    "OfPieType": 2,
}


def members(cls: type) -> list[str]:
    """The enumeration's members — the class attributes that are instances of it."""
    return [name for name in dir(cls) if not name.startswith("_") and isinstance(getattr(cls, name), cls)]


@pytest.mark.parametrize("name,count", sorted(MEMBER_COUNTS.items()))
def test_an_enumeration_states_every_member_the_model_has(name: str, count: int) -> None:
    """A projection that quietly dropped a variant would be a value a caller cannot express."""
    cls = getattr(mjx_ooxml, name)
    assert len(members(cls)) == count, f"{name} projects {len(members(cls))} of {count} members"


@pytest.mark.parametrize("name", RENAMED_NONE)
def test_the_none_variant_is_spelled_in_capitals(name: str) -> None:
    """The one renamed member, and the only rename in the whole binding."""
    cls = getattr(mjx_ooxml, name)
    assert "NONE" in members(cls), f"{name} lost its `NONE` member"
    assert "None" not in members(cls)


def test_no_other_member_was_renamed() -> None:
    """Everything except `None` keeps its Rust spelling, which is what "identity mapping" means."""
    renamed = set(RENAMED_NONE)
    for name in mjx_ooxml.__all__:
        cls = getattr(mjx_ooxml, name)
        if not inspect.isclass(cls):
            continue
        found = [member for member in members(cls) if member.isupper() and len(member) > 2]
        if found:
            assert name in renamed, f"{name} has an all-capitals member {found}, which is unexpected"


def test_members_compare_by_identity_and_by_value() -> None:
    """`eq_int` enumerations must compare to themselves, to each other, and to nothing else."""
    assert SchemeColor.Accent1 == SchemeColor.Accent1
    assert SchemeColor.Accent1 != SchemeColor.Accent2
    assert SchemeColor.Accent1 is SchemeColor.Accent1
    assert int(SchemeColor.Accent1) != int(SchemeColor.Accent2)
    # Deliberately across two enumerations: mypy is right that this can never be equal, and that
    # is exactly the property being asserted.
    assert SchemeColor.Accent1 != TextUnderline.Single  # type: ignore[comparison-overlap]


def test_an_enumeration_survives_the_round_trip_through_a_document() -> None:
    """Written into markup and read back, a member must come back as the same member.

    This is the assertion a wire-token mistake would fail: a projection that mapped
    `CellBorder.Left` onto the model's `Right` would still compare equal to itself in Python, and
    only a trip through the document would notice.
    """
    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    shape = deck.add_shape(
        slide, PresetShapeType.SmileyFace, ShapeBounds.from_inches(1, 1, 2, 2)
    )
    geometry = deck.shape_geometry(slide, shape)
    assert geometry.preset_geometry is not None
    assert geometry.preset_geometry.preset == PresetShapeType.SmileyFace

    deck.set_shape_run_properties(
        slide,
        deck.add_text_box(slide, "underlined", ShapeBounds.from_inches(1, 4, 4, 1)),
        mjx_ooxml.CharacterPropertiesSpec().with_underline(TextUnderline.DoubleWavy),
    )
    reopened = Deck.open(deck.save())
    properties = reopened.run_properties(slide, 1, 0, 0)
    assert properties is not None
    assert properties.underline == TextUnderline.DoubleWavy


def test_every_cell_border_edge_is_distinct_in_the_markup() -> None:
    """Six edges, six different lines: the enumeration must not collapse two of them."""
    from mjx_ooxml import ColorSpec, LineSpec, LineWidth

    deck = Deck.blank(SlideSize.widescreen())
    slide = deck.add_slide()
    table = deck.add_table(slide, 2, 2, ShapeBounds.from_inches(1, 1, 4, 2))
    edges = [
        CellBorder.Left,
        CellBorder.Right,
        CellBorder.Top,
        CellBorder.Bottom,
        CellBorder.TopLeftToBottomRight,
        CellBorder.BottomLeftToTopRight,
    ]
    for index, edge in enumerate(edges):
        deck.set_cell_border(
            slide,
            table,
            0,
            0,
            edge,
            LineSpec.solid(LineWidth.from_points(index + 1), ColorSpec.srgb("000000")),
        )
    widths = []
    for edge in edges:
        line = deck.cell_border(slide, table, 0, 0, edge)
        assert line is not None and line.width is not None
        widths.append(line.width.points)
    assert widths == [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (
        "each edge must carry the line it was given, not another edge's"
    )


def test_format_reports_what_a_format_is() -> None:
    """`Format`'s five accessors, which no other class provides."""
    assert mjx_ooxml.Format.Presentation.conventional_extension == "pptx"
    assert mjx_ooxml.Format.PresentationMacroEnabled.is_macro_enabled
    assert not mjx_ooxml.Format.Presentation.is_macro_enabled
    assert mjx_ooxml.Format.Presentation.is_editable
    assert not mjx_ooxml.Format.Workbook.is_editable
    assert mjx_ooxml.Format.Workbook.family == mjx_ooxml.FormatFamily.Spreadsheet
    assert mjx_ooxml.Format.Document.family == mjx_ooxml.FormatFamily.WordProcessing
    assert "presentationml" in mjx_ooxml.Format.Presentation.content_type
