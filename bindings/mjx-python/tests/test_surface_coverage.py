"""One deck driven through every subject of the surface — and proof each method is wired to *its*
model method.

`test_build_a_deck.py` proves the whole binding agrees with the Rust walkthrough part for part.
This file is the complement: it reaches into the corners the walkthrough does not — geometry, 3-D,
lists, hyperlinks, chart decoration, legacy content, package hygiene — and, for the pairs where a
mis-wiring would be *plausible*, it writes a distinctive value through one method and checks that
the value comes back only from the method that should see it.
"""

from __future__ import annotations

import pytest

import mjx_ooxml
from mjx_ooxml import (
    Angle,
    Bullet,
    CellBorder,
    CellFormat,
    CellMargins,
    Cells,
    CharacterPropertiesSpec,
    ChartData,
    ChartKind,
    ChartLabelScope,
    ColorSpec,
    DataLabelSpec,
    Deck,
    EffectListSpec,
    Emu,
    ErrorBarSpec,
    ErrorBarType,
    ErrorValueType,
    FillSpec,
    Fraction,
    Geometry,
    GlowEffect,
    GuideContext,
    Hyperlink,
    IndentLevel,
    LegendPosition,
    LineSpec,
    LineWidth,
    ParagraphPropertiesSpec,
    PresetShapeType,
    ShapeBounds,
    ShapeGeometry,
    ShapeKind,
    SlideSize,
    Surface,
    TablePart,
    TextAlignment,
    TextAnchoring,
    TrendlineKind,
    TrendlineSpec,
)


@pytest.fixture
def deck() -> Deck:
    """A blank deck with one slide."""
    built = Deck.blank(SlideSize.widescreen())
    built.add_slide()
    return built


# ---------------------------------------------------------------------------------------------
# Mis-wiring guards: a distinctive value must come back from one reader and not from its neighbour
# ---------------------------------------------------------------------------------------------


def test_a_column_width_is_not_a_row_height(deck: Deck) -> None:
    """`set_column_width` and `set_row_height` are adjacent, take the same shapes, and differ.

    Wire either delegate to the other and the two assertions below swap; nothing about the types
    would complain, because both are `(surface, shape, index, Emu)`.
    """
    table = deck.add_table(0, 3, 3, ShapeBounds.from_inches(1, 1, 6, 3))
    deck.set_column_width(0, table, 1, Emu.from_inches(2.5))
    deck.set_row_height(0, table, 2, Emu.from_inches(0.75))

    width = deck.column_width(0, table, 1)
    height = deck.row_height(0, table, 2)
    assert width is not None and width.inches == pytest.approx(2.5)
    assert height is not None and height.inches == pytest.approx(0.75)

    # The other column and the other row did not move.
    other_width = deck.column_width(0, table, 0)
    assert other_width is None or other_width.inches != pytest.approx(2.5)
    assert deck.row_height(0, table, 0) != height


def test_shape_text_and_notes_text_do_not_share_a_body(deck: Deck) -> None:
    """A slide's own text and its notes are different parts, reached by different methods."""
    shape = deck.add_text_box(0, "on the slide", ShapeBounds.from_inches(1, 1, 4, 1))
    deck.set_notes_text(0, "in the notes")

    assert deck.shape_text(0, shape) == "on the slide"
    assert deck.notes_text(0) == "in the notes"
    # The notes body is a different surface, and it holds the notes text, not the slide's.
    notes_shapes = deck.shapes(Surface.notes(0))
    assert notes_shapes, "a notes slide exists once notes text is set"
    on_the_notes = []
    for info in notes_shapes:
        try:
            on_the_notes.append(deck.shape_text(Surface.notes(0), info.index))
        except mjx_ooxml.NothingToReadError:
            pass  # the slide-image placeholder carries no text body
    assert "in the notes" in "".join(on_the_notes)


def test_a_run_hyperlink_is_not_a_shape_hyperlink(deck: Deck) -> None:
    """Two hyperlink families on the same shape, and neither may answer for the other."""
    shape = deck.add_text_box(0, "click", ShapeBounds.from_inches(1, 1, 4, 1))
    deck.set_run_hyperlink(0, shape, 0, 0, Hyperlink.url("https://example.invalid/run"))
    deck.set_shape_hyperlink(0, shape, Hyperlink.url("https://example.invalid/shape"))

    run_link = deck.run_hyperlink(0, shape, 0, 0)
    shape_link = deck.shape_hyperlink(0, shape)
    assert run_link is not None and run_link.target == "https://example.invalid/run"
    assert shape_link is not None and shape_link.target == "https://example.invalid/shape"

    deck.clear_run_hyperlink(0, shape, 0, 0)
    assert deck.run_hyperlink(0, shape, 0, 0) is None
    assert deck.shape_hyperlink(0, shape) is not None, "clearing one must not clear the other"


def test_a_fill_is_not_an_outline_and_neither_is_an_effect(deck: Deck) -> None:
    """Three appearance families, three readers, one shape."""
    shape = deck.add_shape(
        0, PresetShapeType.Rectangle, ShapeBounds.from_inches(1, 1, 2, 2)
    )
    deck.set_shape_fill(0, shape, FillSpec.solid(ColorSpec.srgb("AABBCC")))
    deck.set_shape_outline(
        0, shape, LineSpec.solid(LineWidth.from_points(3.0), ColorSpec.srgb("112233"))
    )
    deck.set_shape_effects(
        0, shape, EffectListSpec().with_glow(GlowEffect(ColorSpec.srgb("FFEE00")))
    )

    fill = deck.shape_fill(0, shape)
    outline = deck.shape_outline(0, shape)
    effects = deck.shape_effects(0, shape)
    assert fill is not None and fill.color is not None
    assert fill.color.srgb_value == "AABBCC"
    assert outline is not None and outline.fill is not None
    assert outline.fill.color is not None and outline.fill.color.srgb_value == "112233"
    assert effects is not None and effects.glow is not None
    assert effects.glow.color.srgb_value == "FFEE00"

    deck.set_shape_no_fill(0, shape)
    assert deck.shape_outline(0, shape) is not None, "clearing the fill must not clear the outline"


def test_a_cell_border_edge_is_the_edge_it_was_given(deck: Deck) -> None:
    """Six edges, one setter, one reader — and the edge argument must reach the model."""
    table = deck.add_table(0, 2, 2, ShapeBounds.from_inches(1, 1, 4, 2))
    deck.set_cell_border(
        0, table, 0, 0, CellBorder.Left,
        LineSpec.solid(LineWidth.from_points(2.0), ColorSpec.srgb("FF0000")),
    )
    left = deck.cell_border(0, table, 0, 0, CellBorder.Left)
    right = deck.cell_border(0, table, 0, 0, CellBorder.Right)
    assert left is not None and left.width is not None
    assert left.width.points == pytest.approx(2.0)
    assert right is None or right.width != left.width


# ---------------------------------------------------------------------------------------------
# Breadth: every subject of the surface, exercised through this binding alone
# ---------------------------------------------------------------------------------------------


def test_slides_layouts_masters_and_the_theme(deck: Deck) -> None:
    """The document-level readers."""
    assert deck.slide_count() == 1
    assert deck.master_count() == 1
    assert deck.layout_count() >= 1
    assert deck.layouts()[0].master_index == 0
    assert deck.layout_master(0) == 0
    assert deck.slide_layout(0) is not None
    assert deck.slide_size().width_emu > 0
    assert deck.layout_kind(0) is not None
    theme = deck.theme(Surface.slide(0))
    assert theme is not None and theme.fill_styles
    colors = deck.color_map(Surface.master(0))
    assert colors is not None and colors.text1 is not None
    assert deck.master_name(0) is None or isinstance(deck.master_name(0), str)


def test_shapes_groups_and_addresses(deck: Deck) -> None:
    """Adding, grouping, ungrouping and moving between groups."""
    first = deck.add_shape(0, PresetShapeType.Ellipse, ShapeBounds.from_inches(1, 1, 1, 1))
    second = deck.add_shape(0, PresetShapeType.Ellipse, ShapeBounds.from_inches(3, 1, 1, 1))
    loose = deck.add_shape(0, PresetShapeType.Rectangle, ShapeBounds.from_inches(5, 1, 1, 1))

    group = deck.group_shapes(0, [first, second])
    assert deck.shape_kind(0, group) == ShapeKind.GroupShape
    assert deck.shape_member_count(0, group) == 2
    # Grouping rewrites the index space: the two members left it, and the group took their place,
    # so the shape that was at `loose` has moved down.
    assert deck.shape_count(0) == 2
    loose = 1 if group.indices[0] == 0 else 0

    moved = deck.move_shape_into_group(0, loose, group)
    assert moved.depth == 2
    assert deck.shape_member_count(0, group) == 3
    back = deck.move_shape_out_of_group(0, moved)
    assert back.is_top_level

    members = deck.ungroup(0, group)
    assert len(members) == 2
    assert deck.shape_count(0) == 3


def test_text_paragraphs_runs_and_list_styles(deck: Deck) -> None:
    """The text subject, from whole-body writes down to a character range."""
    shape = deck.add_text_box(0, "Hello world", ShapeBounds.from_inches(1, 1, 6, 2))
    assert deck.paragraph_count(0, shape) == 1
    assert deck.run_count(0, shape, 0) == 1
    assert deck.paragraph_text(0, shape, 0) == "Hello world"
    assert deck.run_text(0, shape, 0, 0) == "Hello world"

    deck.set_paragraph_properties(
        0, shape, 0, ParagraphPropertiesSpec().with_alignment(TextAlignment.Center)
    )
    paragraph = deck.paragraph_properties(0, shape, 0)
    assert paragraph is not None and paragraph.alignment == TextAlignment.Center

    deck.set_text_range_properties(
        0, shape, 0, range(0, 5), CharacterPropertiesSpec().with_bold(True)
    )
    assert deck.run_count(0, shape, 0) == 2, "the range split the run"
    first_run = deck.run_properties(0, shape, 0, 0)
    assert first_run is not None and first_run.is_bold is True
    assert deck.coalesce_paragraph_runs(0, shape, 0) >= 0

    deck.set_shape_list_style_level(
        0,
        shape,
        IndentLevel(1),
        ParagraphPropertiesSpec().with_bullet(Bullet.character(mjx_ooxml.BulletCharacter("•"))),
    )
    level = deck.shape_list_style_level(0, shape, IndentLevel(1))
    assert level is not None and level.bullet is not None
    assert level.bullet.kind == "character"
    assert deck.clear_shape_list_style_level(0, shape, IndentLevel(1)) is True

    deck.set_end_run_properties(0, shape, 0, CharacterPropertiesSpec().with_italic(True))
    end = deck.end_run_properties(0, shape, 0)
    assert end is not None and end.is_italic is True


def test_bounds_transforms_and_geometry(deck: Deck) -> None:
    """The geometry subject, including the 117-shape preset table."""
    shape = deck.add_shape(
        0, PresetShapeType.RoundedRectangle, ShapeBounds.from_inches(1, 1, 3, 2)
    )
    deck.set_shape_bounds(0, shape, ShapeBounds.from_inches(2, 2, 4, 1))
    bounds = deck.shape_bounds(0, shape)
    assert bounds is not None and bounds.width_emu == ShapeBounds.from_inches(0, 0, 4, 0).width_emu

    transform = deck.shape_transform(0, shape)
    assert transform is not None and transform.size is not None
    deck.set_shape_transform(
        0, shape, mjx_ooxml.Transform2D(rotation=Angle.from_degrees(45))
    )
    rotated = deck.shape_transform(0, shape)
    assert rotated is not None and rotated.rotation is not None
    assert rotated.rotation.degrees == pytest.approx(45)

    geometry = deck.shape_geometry(0, shape)
    assert geometry.kind == "preset"
    assert geometry.preset_geometry is not None
    assert geometry.preset_geometry.preset == PresetShapeType.RoundedRectangle

    # And the table both ways: name the adjustment, set it, read it back.
    assert ShapeGeometry.adjustment_names(PresetShapeType.RoundedRectangle) == ["corner_radius"]
    deck.set_shape_geometry(
        0,
        shape,
        Geometry.preset(
            ShapeGeometry.of(
                PresetShapeType.RoundedRectangle, {"corner_radius": Fraction.of(0.4)}
            )
        ),
    )
    written = deck.shape_geometry(0, shape).preset_geometry
    assert written is not None
    corner = written.adjustments["corner_radius"]
    assert isinstance(corner, Fraction) and corner.ratio == pytest.approx(0.4)

    adjustments = deck.shape_adjustments(
        0, shape, GuideContext.from_extents(Emu.from_inches(4), Emu.from_inches(1))
    )
    assert adjustments and adjustments[0].spec.wire_name == "adj"


def test_an_angle_adjustment_is_refused_where_a_proportion_was_wanted() -> None:
    """The preset table keeps the units: an `Angle` cannot stand in for a `Fraction`."""
    with pytest.raises(TypeError, match="proportion"):
        ShapeGeometry.of(
            PresetShapeType.RoundedRectangle, {"corner_radius": Angle.from_degrees(30)}
        )
    with pytest.raises(KeyError, match="no adjustment called"):
        ShapeGeometry.of(PresetShapeType.RoundedRectangle, {"nonsense": Fraction.of(0.1)})
    with pytest.raises(KeyError, match="needs an adjustment"):
        ShapeGeometry.of(PresetShapeType.RoundedRectangle, {})


def test_tables_cells_and_styles(deck: Deck) -> None:
    """The table subject: dimensions, spans, merges, per-cell formatting and styles."""
    table = deck.add_table(0, 3, 3, ShapeBounds.from_inches(1, 1, 6, 3))
    assert deck.table_dimensions(0, table) == (3, 3)

    deck.set_cell_text(0, table, 0, 0, 0, "corner")
    assert deck.cell_text(0, table, 0, 0) == "corner"
    assert deck.visible_cell_text(0, table, 0, 0) == "corner"
    assert deck.cell_paragraph_count(0, table, 0, 0) == 1
    assert deck.cell_run_count(0, table, 0, 0, 0) == 1
    assert deck.cell_paragraph_text(0, table, 0, 0, 0) == "corner"
    assert deck.cell_run_text(0, table, 0, 0, 0, 0) == "corner"

    deck.merge_cells(0, table, Cells.rectangle(range(1, 3), range(1, 3)))
    assert deck.cell_span(0, table, 1, 1) == (2, 2)
    assert deck.merged_cell_anchor(0, table, 2, 2) == (1, 1)
    deck.unmerge_cells(0, table, 1, 1)
    assert deck.cell_span(0, table, 1, 1) == (1, 1)

    deck.set_cell_margins(0, table, 0, 0, CellMargins.uniform(Emu.from_points(6)))
    margins = deck.cell_margins(0, table, 0, 0)
    assert margins.left is not None and margins.left.points == pytest.approx(6)
    deck.set_cell_anchor(0, table, 0, 0, TextAnchoring.Center)
    assert deck.cell_anchor(0, table, 0, 0) == TextAnchoring.Center

    deck.format_cells(
        0, table, Cells.row(0), CellFormat().with_fill(FillSpec.solid(ColorSpec.srgb("102030")))
    )
    fill = deck.cell_fill(0, table, 0, 1)
    assert fill is not None and fill.color is not None
    assert fill.color.srgb_value == "102030"

    deck.set_table_part(0, table, TablePart.FirstRow, True)
    assert deck.table_part(0, table, TablePart.FirstRow) is True

    style_id = "{11111111-2222-3333-4444-555555555555}"
    deck.create_table_style(style_id, "Coverage")
    deck.set_table_style(0, table, style_id)
    assert deck.table_style_id(0, table) == style_id

    deck.insert_row(0, table, 1)
    assert deck.table_dimensions(0, table)[0] == 4
    deck.remove_row(0, table, 1)
    deck.insert_column(0, table, 0)
    assert deck.table_dimensions(0, table)[1] == 4
    deck.remove_column(0, table, 0)
    assert deck.table_dimensions(0, table) == (3, 3)


def test_charts_and_their_decoration(deck: Deck) -> None:
    """The chart subject, from authoring through to trendlines and error bars."""
    chart = (
        ChartData(ChartKind.Bar)
        .categories(["a", "b", "c"])
        .series("one", [1.0, 2.0, 3.0])
        .series("two", [3.0, 2.0, 1.0])
        .legend(LegendPosition.Bottom)
    )
    chart.validate()
    frame = deck.add_chart(0, chart, ShapeBounds.from_inches(1, 1, 8, 4))
    assert deck.chart_part_bytes(0, frame) is not None, "the authored chart part is readable"

    assert deck.chart_kinds(0, frame) == [ChartKind.Bar]
    series = deck.chart_series(0, frame)
    assert [entry.name for entry in series] == ["one", "two"]
    deck.set_chart_series_values(0, frame, 0, [9.0, 8.0, 7.0])
    assert deck.chart_series(0, frame)[0].values == [9.0, 8.0, 7.0]
    deck.set_chart_series_categories(0, frame, 0, ["x", "y", "z"])
    assert deck.chart_series(0, frame)[0].categories == ["x", "y", "z"]

    deck.set_chart_title(0, frame, "Coverage")
    assert deck.chart_title(0, frame) == "Coverage"
    legend = deck.chart_legend(0, frame)
    assert legend is not None and legend.position == LegendPosition.Bottom

    axes = deck.chart_axes(0, frame)
    assert len(axes) == 2
    deck.set_chart_axis_scale(0, frame, 1, 0.0, 10.0)
    assert deck.chart_axes(0, frame)[1].maximum == pytest.approx(10.0)
    deck.set_chart_axis_title(0, frame, 0, "Category")
    assert deck.chart_axes(0, frame)[0].title == "Category"
    deck.set_chart_axis_gridlines(0, frame, 1, True, False)
    assert deck.chart_axes(0, frame)[1].major_gridlines is True

    deck.set_chart_series_fill(0, frame, 0, FillSpec.solid(ColorSpec.srgb("445566")))
    series_fill = deck.chart_series_fill(0, frame, 0)
    assert series_fill is not None and series_fill.color is not None
    assert series_fill.color.srgb_value == "445566"

    deck.set_chart_data_labels(
        0, frame, ChartLabelScope.series(0), DataLabelSpec().value(True)
    )
    labels = deck.chart_data_labels(0, frame, 0, None)
    assert labels.shows_value is True

    deck.add_chart_trendline(0, frame, 0, TrendlineSpec(TrendlineKind.Linear))
    assert len(deck.chart_trendlines(0, frame, 0)) == 1
    assert deck.remove_chart_trendlines(0, frame, 0) == 1

    deck.set_chart_error_bars(
        0, frame, 0, ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 0.5)
    )
    assert len(deck.chart_error_bars(0, frame, 0)) == 1
    assert deck.remove_chart_error_bars(0, frame, 0) == 1

    deck.set_chart_point_fill(0, frame, 0, 1, FillSpec.solid(ColorSpec.srgb("998877")))
    formats = deck.chart_point_formats(0, frame, 0)
    assert any(entry.index == 1 for entry in formats)
    assert deck.chart_dangling_decoration(0, frame, 0) == []
    assert deck.chart_workbooks(Surface.slide(0))


def test_pictures_media_and_the_effective_readers(deck: Deck) -> None:
    """Images, and the inheritance ladders the `effective_…` readers walk."""
    picture = deck.add_picture(
        0, mjx_ooxml.DEFAULT_PLACEHOLDER_IMAGE, ShapeBounds.from_inches(1, 1, 2, 2)
    )
    assert deck.shape_kind(0, picture) == ShapeKind.Picture
    assert deck.picture_image_bytes(0, picture) == mjx_ooxml.DEFAULT_PLACEHOLDER_IMAGE
    assert deck.picture_image_link_target(0, picture) is None
    assert deck.linked_images(Surface.slide(0)) == []
    assert deck.media_references(Surface.slide(0)) == []

    rel_id = deck.add_image(Surface.slide(0), mjx_ooxml.DEFAULT_PLACEHOLDER_IMAGE)
    assert rel_id.startswith("rId")

    shape = deck.add_text_box(0, "inherited", ShapeBounds.from_inches(1, 4, 4, 1))
    assert deck.effective_run_properties(0, shape, 0, 0) is not None
    assert deck.effective_paragraph_properties(0, shape, 0) is not None
    assert deck.effective_shape_bounds(0, shape) is not None
    # A text box states no fill of its own, and inherits none either — which is what "no fill"
    # means for a text box, and is a different answer from "the reader is not wired".
    assert deck.effective_shape_fill(0, shape) is None
    assert deck.effective_shape_transform(0, shape) is not None


def test_package_hygiene_and_the_legacy_windows(deck: Deck) -> None:
    """The three hygiene delegates, and the part-addressed byte windows."""
    assert deck.external_links() == []
    swept = deck.remove_unused_parts()
    assert isinstance(swept, list)

    assert deck.ink_part_names() == []
    assert deck.ink_references(Surface.slide(0)) == []
    assert deck.ole_objects(Surface.slide(0)) == []
    assert deck.activex_control_count(Surface.slide(0)) == 0

    inkml = (
        b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        b'<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML">'
        b"<inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"
    )
    ink_shape = deck.add_ink(Surface.slide(0), inkml)
    assert deck.ink_part_names(), "adding ink adds a part"
    part = deck.ink_part_for_shape(Surface.slide(0), ink_shape)
    assert part is not None
    assert deck.ink_part_bytes(part) == inkml
    assert deck.shape_for_ink_part(Surface.slide(0), part) == ink_shape

    diagram = mjx_ooxml.DiagramContent.vertical_list(["one", "two"])
    frame = deck.add_diagram(Surface.slide(0), diagram, ShapeBounds.from_inches(1, 1, 4, 3))
    ids = deck.diagram_relationship_ids(0, frame)
    assert ids is not None and ids.data is not None
    parts = deck.diagram_parts(0, frame)
    assert parts is not None and parts.layout is not None
    assert deck.diagram_part_bytes(parts.layout) is not None

    deck.validate()
    assert deck.save()[:2] == b"PK"


def test_the_three_dimensional_properties(deck: Deck) -> None:
    """`a:scene3d` and `a:sp3d`, which table cells share."""
    shape = deck.add_shape(
        0, PresetShapeType.Rectangle, ShapeBounds.from_inches(1, 1, 2, 2)
    )
    scene = mjx_ooxml.Scene3DSpec(
        mjx_ooxml.Camera(mjx_ooxml.PresetCamera.OrthographicFront),
        mjx_ooxml.LightRig(
            mjx_ooxml.LightRigType.ThreePoint, mjx_ooxml.LightRigDirection.Top
        ),
    )
    deck.set_shape_scene_3d(0, shape, scene)
    read_back = deck.shape_scene_3d(0, shape)
    assert read_back is not None
    assert read_back.camera.preset == mjx_ooxml.PresetCamera.OrthographicFront

    properties = mjx_ooxml.Shape3DSpec(extrusion_height=Emu.from_points(12))
    deck.set_shape_3d_properties(0, shape, properties)
    written = deck.shape_3d_properties(0, shape)
    assert written is not None and written.extrusion_height is not None
    assert written.extrusion_height.points == pytest.approx(12)

    deck.clear_shape_scene_3d(0, shape)
    assert deck.shape_scene_3d(0, shape) is None
    assert deck.shape_3d_properties(0, shape) is not None, (
        "clearing the scene must not clear the shape's own 3-D properties"
    )
