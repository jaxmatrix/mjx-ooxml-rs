"""The validation artefacts, written through the Python binding (MJXOFF-122).

`xtask/src/validation/` is the Rust original: twenty areas across the three formats, each built
from a `blank` document through the facade and nothing below it. This file is the same twenty,
call for call, and `test_every_artefact_matches_the_rust_one` compares each one against the Rust
output **part by part, byte for byte**.

That comparison is the point. A binding method wired to the wrong facade method, or an argument
converted with the wrong units, changes one part payload and nothing else in this repository would
notice — least of all a human reading the file in Office, which is where these artefacts are going.
`bindings/mjx-wasm/tests/node/validation_artefacts.mjs` is the third copy and is compared against
the same reference.

**Nothing here records a result.** See `docs/validation/00-method.md`: judging what Office renders
needs a person with Office in front of them, and a passing comparison in this file says only that
three languages agree about the bytes.
"""

from __future__ import annotations

import pathlib
import subprocess
from typing import Callable

import pytest

from mjx_ooxml import (
    Angle,
    AxisOrientation,
    BorderEdgeSpec,
    BorderSpec,
    BorderStyle,
    CellBorder,
    CellFormat,
    CellFormatSpec,
    CellFormatTarget,
    CellMargins,
    Cells,
    CellWrite,
    CharacterPropertiesSpec,
    ChartData,
    ChartKind,
    ChartLabelScope,
    ChartRangeSeries,
    ChartWrap,
    Color,
    ColorSpec,
    ColorTransform,
    ColorTransformKind,
    ConnectionSite,
    CustomGeometrySpec,
    DataLabelPosition,
    DataLabelSpec,
    Deck,
    DrawCommand,
    Document,
    AdjustAngle,
    AdjustCoordinate,
    EffectListSpec,
    Emu,
    ErrorBarDirection,
    ErrorBarSpec,
    ErrorBarType,
    ErrorValueType,
    FillSpec,
    FontProperties,
    Fraction,
    Geometry,
    GlowEffect,
    GradientStopSpec,
    GuideContext,
    GuideSpec,
    HeaderFooterType,
    Hyperlink,
    HyperlinkTarget,
    LegendPosition,
    LineSpec,
    LineWidth,
    MergedCellType,
    OuterShadowEffect,
    PageMargins,
    PageSize,
    ParagraphPropertiesSpec,
    Path2DSpec,
    PatternFillSpec,
    PictureFillMode,
    Point,
    PresetShapeType,
    Rectangle,
    ResizingBehavior,
    SchemeColor,
    SectionLocation,
    ShapeBounds,
    SlideSize,
    TableStyleBorder,
    TableStyleFormat,
    TableStylePart,
    TextAlignment,
    TextAnchoring,
    Transform2D,
    TrendlineKind,
    TrendlineSpec,
    WrapText,
    Workbook,
    DEFAULT_PLACEHOLDER_IMAGE,
)

from opc import part_payloads

REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[3]

#: One inch in EMU — the unit `Document` and `Workbook`'s drawing calls take.
INCH = 914_400

#: The table style `V-PPTX-03` authors and points at. A GUID because that is what `a:tableStyle`
#: ids are; it is not a built-in.
TABLE_STYLE_ID = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"

PLACEHOLDER_CONTENT_TYPE = "image/png"
PLACEHOLDER_EXTENSION = "png"


# ---------------------------------------------------------------------------------------------
# Shared pieces — the same values every language writes
# ---------------------------------------------------------------------------------------------


def quarterly_chart() -> ChartData:
    """The chart every format's chart area draws, so the three artefacts are comparable."""
    return (
        ChartData(ChartKind.Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25])
        .series("2025", [10.5, 13.0, 13.75, 16.0])
    )


def quarterly_cells() -> list[CellWrite]:
    """The figures every workbook area writes into its first sheet."""
    return [
        CellWrite.shared_text("A1", "Region"),
        CellWrite.shared_text("B1", "Revenue"),
        CellWrite.shared_text("C1", "Growth"),
        CellWrite.shared_text("A2", "North America"),
        CellWrite.number("B2", 1_250_000.0),
        CellWrite.number("C2", 0.125),
        CellWrite.shared_text("A3", "EMEA"),
        CellWrite.number("B3", 980_000.0),
        CellWrite.number("C3", 0.061),
        CellWrite.shared_text("A4", "Asia Pacific"),
        CellWrite.number("B4", 1_410_000.0),
        CellWrite.number("C4", 0.198),
    ]


def _one_slide_deck() -> tuple[Deck, int]:
    deck = Deck.blank(SlideSize.widescreen())
    return deck, deck.add_slide()


def _blank_document() -> Document:
    return Document.blank(PageSize.a4())


def _append_heading(document: Document, text: str) -> int:
    """The first line of an area: written into the blank document's empty paragraph, or appended."""
    existing = document.paragraph_count()
    if existing == 1 and document.run_count(0) == 0:
        at = 0
    else:
        document.append_paragraph()
        at = existing
    document.append_run(at, text)
    return at


def _append_line(document: Document, text: str) -> int:
    at = document.paragraph_count()
    document.append_paragraph()
    document.append_run(at, text)
    return at


# ---------------------------------------------------------------------------------------------
# V-PPTX-01 … V-PPTX-06
# ---------------------------------------------------------------------------------------------


def v_pptx_01() -> bytes:
    deck, slide = _one_slide_deck()
    stated = deck.add_text_box(
        slide,
        "Stated on the run: 24pt bold, accent 1",
        ShapeBounds.from_inches(0.5, 0.5, 8.0, 0.8),
    )
    deck.set_shape_run_properties(
        slide,
        stated,
        CharacterPropertiesSpec()
        .with_size_points(24.0)
        .with_bold(True)
        .with_color(ColorSpec.scheme(SchemeColor.Accent1)),
    )
    paragraph = deck.add_text_box(
        slide,
        "Stated on the paragraph: centred, 1.5 line spacing",
        ShapeBounds.from_inches(0.5, 1.6, 8.0, 0.8),
    )
    deck.set_paragraph_properties(
        slide,
        paragraph,
        0,
        ParagraphPropertiesSpec()
        .with_alignment(TextAlignment.Center)
        .with_left_margin_points(18.0)
        .with_indent_points(-18.0),
    )
    inherited = deck.add_text_box(
        slide,
        "Nothing stated: this run resolves from the layout and the master",
        ShapeBounds.from_inches(0.5, 2.7, 8.0, 0.8),
    )
    deck.effective_run_properties(slide, inherited, 0, 0)
    return deck.save()


def v_pptx_02() -> bytes:
    deck, slide = _one_slide_deck()
    solid = deck.add_shape(
        slide, PresetShapeType.Rectangle, ShapeBounds.from_inches(0.5, 0.5, 2.5, 1.5)
    )
    deck.set_shape_fill(slide, solid, FillSpec.solid(ColorSpec.srgb("1F3864")))
    deck.set_shape_outline(
        slide,
        solid,
        LineSpec.solid(LineWidth.from_points(3.0), ColorSpec.scheme(SchemeColor.Accent2)),
    )
    gradient = deck.add_shape(
        slide, PresetShapeType.Ellipse, ShapeBounds.from_inches(3.5, 0.5, 2.5, 1.5)
    )
    deck.set_shape_fill(
        slide,
        gradient,
        FillSpec.gradient(
            [
                GradientStopSpec(Fraction.of(0.0), ColorSpec.srgb("FFF2CC")),
                GradientStopSpec(Fraction.of(1.0), ColorSpec.srgb("C00000")),
            ],
            Angle.from_degrees(45.0),
        ),
    )
    effects = deck.add_shape(
        slide, PresetShapeType.RoundedRectangle, ShapeBounds.from_inches(6.5, 0.5, 2.5, 1.5)
    )
    deck.set_shape_fill(slide, effects, FillSpec.solid(ColorSpec.srgb("FFFFFF")))
    deck.set_shape_effects(
        slide,
        effects,
        EffectListSpec(
            glow=GlowEffect(ColorSpec.scheme(SchemeColor.Accent1), Emu.from_points(5.0)),
            outer_shadow=OuterShadowEffect(
                ColorSpec.srgb("808080"),
                blur_radius=Emu.from_points(4.0),
                distance=Emu.from_points(3.0),
                direction=Angle.from_degrees(45.0),
            ),
        ),
    )
    _write_colour_transform_areas(deck, slide)
    return deck.save()



def _write_colour_transform_areas(deck: Deck, surface: int) -> None:
    """`V-PPTX-02.4` — the swatches the human Office pass needs, in the order the Rust catalogue
    writes them (`xtask/src/validation/presentation.rs`). The top row is the four transforms the
    entry names plus `a:inv`, over a fixed `4472C4`; the bottom row is what a real file actually
    contains, over the theme's accent 1. The first swatch in each row carries no transform and is
    that row's baseline.
    """
    base = ColorSpec.srgb("4472C4")
    accent = ColorSpec.scheme(SchemeColor.Accent1)
    half = Fraction.of(0.5)
    rows: list[tuple[float, list[tuple[str, ColorSpec]]]] = [
        (
            2.4,
            [
                ("4472C4", base),
                ("comp", base.with_transform(_marker(ColorTransformKind.Complement))),
                ("gray", base.with_transform(_marker(ColorTransformKind.Grayscale))),
                ("gamma", base.with_transform(_marker(ColorTransformKind.Gamma))),
                ("invGamma", base.with_transform(_marker(ColorTransformKind.InverseGamma))),
                ("inv", base.with_transform(_marker(ColorTransformKind.Inverse))),
            ],
        ),
        (
            4.2,
            [
                ("accent 1", accent),
                ("tint 50%", accent.with_tint(half)),
                ("shade 50%", accent.with_shade(half)),
                ("satMod 150%", accent.with_saturation_modulation(Fraction.of(1.5))),
                (
                    "lumMod 60% + lumOff 40%",
                    accent.with_luminance_modulation(Fraction.of(0.6)).with_luminance_offset(
                        Fraction.of(0.4)
                    ),
                ),
                ("alpha 50%", accent.with_alpha(half)),
            ],
        ),
    ]
    for top, swatches in rows:
        for column, (label, color) in enumerate(swatches):
            swatch = deck.add_shape(
                surface,
                PresetShapeType.Rectangle,
                ShapeBounds.from_inches(0.35 + 2.12 * column, top, 2.0, 1.3),
            )
            deck.set_shape_fill(surface, swatch, FillSpec.solid(color))
            deck.set_shape_text_content(surface, swatch, label)
            deck.effective_shape_fill(surface, swatch)


def _marker(kind: ColorTransformKind) -> ColorTransform:
    """A valueless transform, or a failure here rather than a silently plain colour in the file."""
    transform = ColorTransform.marker(kind)
    assert transform is not None, f"{kind} is a valueless member"
    return transform


def v_pptx_03() -> bytes:
    deck, slide = _one_slide_deck()
    table = deck.add_table(slide, 3, 3, ShapeBounds.from_inches(0.5, 0.5, 8.0, 2.5))
    rows = [
        ["Region", "Revenue", "Growth"],
        ["North", "4.2", "+12%"],
        ["South", "3.1", "+8%"],
    ]
    for row, cells in enumerate(rows):
        for column, text in enumerate(cells):
            deck.set_cell_text(slide, table, row, column, 0, text)
    deck.create_table_style(TABLE_STYLE_ID, "mjx validation")
    deck.format_table_style_part(
        TABLE_STYLE_ID,
        TableStylePart.FirstRow,
        TableStyleFormat()
        .with_fill(FillSpec.solid(ColorSpec.srgb("1F3864")))
        .with_text_color(ColorSpec.srgb("FFFFFF"))
        .with_border(
            TableStyleBorder.Bottom,
            LineSpec.solid(LineWidth.from_points(2.0), ColorSpec.srgb("FFFFFF")),
        ),
    )
    deck.set_table_style(slide, table, TABLE_STYLE_ID)
    deck.format_cells(
        slide,
        table,
        Cells.row(0),
        CellFormat()
        .with_anchor(TextAnchoring.Center)
        .with_margins(CellMargins.uniform(Emu.from_points(6.0))),
    )
    deck.set_cell_border(
        slide,
        table,
        2,
        0,
        CellBorder.Top,
        LineSpec.solid(LineWidth.from_points(1.0), ColorSpec.srgb("C00000")),
    )
    deck.merge_cells(slide, table, Cells.rectangle(range(2, 3), range(1, 3)))
    return deck.save()


def v_pptx_04() -> bytes:
    deck, slide = _one_slide_deck()
    chart = deck.add_chart(slide, quarterly_chart(), ShapeBounds.from_inches(0.5, 0.5, 8.0, 4.5))
    deck.set_chart_title(slide, chart, "Revenue by quarter")
    deck.set_chart_legend(slide, chart, LegendPosition.Bottom)
    deck.set_chart_axis_title(slide, chart, 0, "Quarter")
    deck.set_chart_axis_title(slide, chart, 1, "Revenue")
    deck.set_chart_series_fill(slide, chart, 0, FillSpec.solid(ColorSpec.scheme(SchemeColor.Accent1)))
    deck.add_chart_trendline(slide, chart, 0, TrendlineSpec(TrendlineKind.Linear))
    return deck.save()


def v_pptx_05() -> bytes:
    deck, slide = _one_slide_deck()
    deck.add_picture(slide, DEFAULT_PLACEHOLDER_IMAGE, ShapeBounds.from_inches(0.5, 0.5, 2.0, 2.0))
    rel = deck.add_image(slide, DEFAULT_PLACEHOLDER_IMAGE)
    filled = deck.add_shape(
        slide, PresetShapeType.Rectangle, ShapeBounds.from_inches(3.0, 0.5, 3.0, 2.0)
    )
    deck.set_shape_fill(slide, filled, FillSpec.picture(rel, PictureFillMode.Stretch))
    return deck.save()


def v_pptx_06() -> bytes:
    deck, slide = _one_slide_deck()
    linked = deck.add_text_box(
        slide, "The whole shape is a link", ShapeBounds.from_inches(0.5, 0.5, 4.0, 0.8)
    )
    deck.set_shape_hyperlink(slide, linked, Hyperlink.url("https://example.com/investors"))
    run_linked = deck.add_text_box(
        slide, "Only this run is a link", ShapeBounds.from_inches(0.5, 1.6, 4.0, 0.8)
    )
    deck.set_run_hyperlink(slide, run_linked, 0, 0, Hyperlink.url("https://example.com/report"))
    deck.set_notes_text(slide, "Lead with the revenue number, then the regional split.")
    return deck.save()


# ---------------------------------------------------------------------------------------------
# V-DOCX-01 … V-DOCX-06
# ---------------------------------------------------------------------------------------------


#: The four presets whose guide formulas take an arc-tangent argument through zero.
ARC_TANGENT_PRESETS = [
    PresetShapeType.Moon,
    PresetShapeType.Arc,
    PresetShapeType.CircularArrow,
    PresetShapeType.Gear9,
]

#: The fifteen plot types that draw from one series, each with its `c:` element's local name.
SINGLE_SERIES_KINDS = [
    (ChartKind.Bar, "barChart"),
    (ChartKind.Bar3D, "bar3DChart"),
    (ChartKind.Line, "lineChart"),
    (ChartKind.Line3D, "line3DChart"),
    (ChartKind.Pie, "pieChart"),
    (ChartKind.Pie3D, "pie3DChart"),
    (ChartKind.OfPie, "ofPieChart"),
    (ChartKind.Area, "areaChart"),
    (ChartKind.Area3D, "area3DChart"),
    (ChartKind.Scatter, "scatterChart"),
    (ChartKind.Doughnut, "doughnutChart"),
    (ChartKind.Radar, "radarChart"),
    (ChartKind.Bubble, "bubbleChart"),
    (ChartKind.Surface, "surfaceChart"),
    (ChartKind.Surface3D, "surface3DChart"),
]


def _guide_driven_triangle() -> CustomGeometrySpec:
    """The apex placed by `apex = */ w 1 2` rather than by a number."""

    def guide(name: str) -> AdjustCoordinate:
        return AdjustCoordinate.guide(name)

    return CustomGeometrySpec(
        paths=[
            Path2DSpec(
                [
                    DrawCommand.move_to(
                        Point(guide("apex"), AdjustCoordinate.emu(Emu.from_emu(0)))
                    ),
                    DrawCommand.line_to(Point(guide("r"), guide("b"))),
                    DrawCommand.line_to(Point(guide("l"), guide("b"))),
                    DrawCommand.close(),
                ]
            )
        ],
        adjust_values=[GuideSpec("adj", "val 50000")],
        guides=[GuideSpec("apex", "*/ w 1 2")],
        connection_sites=[
            ConnectionSite(
                AdjustAngle.angle(Angle.from_degrees(270.0)),
                Point(guide("apex"), AdjustCoordinate.emu(Emu.from_emu(0))),
            ),
            ConnectionSite(
                AdjustAngle.angle(Angle.from_degrees(0.0)),
                Point(guide("r"), guide("b")),
            ),
            ConnectionSite(
                AdjustAngle.angle(Angle.from_degrees(180.0)),
                Point(guide("l"), guide("b")),
            ),
        ],
        text_rectangle=Rectangle(guide("l"), guide("vc"), guide("r"), guide("b")),
    )


def _write_geometry_areas(deck: Deck, surface: int) -> None:
    square_chevron = deck.add_shape(
        surface, PresetShapeType.Chevron, ShapeBounds.from_inches(0.4, 3.4, 2.5, 2.5)
    )
    wide_chevron = deck.add_shape(
        surface, PresetShapeType.Chevron, ShapeBounds.from_inches(3.2, 3.4, 2.5, 1.25)
    )
    for position, preset in enumerate(ARC_TANGENT_PRESETS):
        deck.add_shape(
            surface, preset, ShapeBounds.from_inches(0.4 + 1.4 * position, 6.1, 1.2, 1.2)
        )
    custom = deck.add_shape(
        surface, PresetShapeType.Rectangle, ShapeBounds.from_inches(6.2, 3.4, 3.0, 1.5)
    )
    deck.set_shape_geometry(surface, custom, Geometry.custom(_guide_driven_triangle()))
    deck.set_shape_fill(surface, custom, FillSpec.solid(ColorSpec.scheme(SchemeColor.Accent2)))
    deck.shape_adjustments(
        surface, square_chevron, GuideContext.from_extents(Emu.from_emu(2_286_000), Emu.from_emu(2_286_000))
    )
    deck.shape_adjustments(
        surface, wide_chevron, GuideContext.from_extents(Emu.from_emu(2_286_000), Emu.from_emu(1_143_000))
    )
    deck.shape_geometry(surface, custom)
    rotated = deck.add_shape(
        surface, PresetShapeType.Rectangle, ShapeBounds.from_inches(6.2, 5.2, 1.6, 1.0)
    )
    deck.set_shape_transform(surface, rotated, Transform2D(rotation=Angle.from_degrees(30.0)))
    deck.effective_shape_bounds(surface, rotated)


def v_pptx_07() -> bytes:
    deck = Deck.blank(SlideSize.standard())
    slide = deck.add_slide_from_layout(0)
    deck.set_shape_text_content(slide, 0, "4:3 — 10 x 7.5 in")
    deck.set_shape_text_content(
        slide,
        1,
        "The two placeholders above and beside this one were placed by the master, not by this code.",
    )
    _write_geometry_areas(deck, slide)
    return deck.save()


def _gallery_bounds(position: int) -> ShapeBounds:
    column = position % 2
    row = (position % 4) // 2
    return ShapeBounds.from_inches(0.4 + column * 6.4, 0.4 + row * 3.4, 6.0, 3.0)


def _write_chart_decoration_areas(deck: Deck, surface: int) -> None:
    edited = deck.add_chart(
        surface,
        ChartData(ChartKind.Bar).categories(["Jan", "Feb", "Mar"]).series("Sales", [19.2, 21.4, 16.7]),
        ShapeBounds.from_inches(0.4, 0.4, 5.8, 2.6),
    )
    deck.set_chart_title(surface, edited, "Series values, rewritten")
    deck.set_chart_series_values(surface, edited, 0, [41.5, 42.5, 43.5])

    labelled = deck.add_chart(
        surface, quarterly_chart(), ShapeBounds.from_inches(6.6, 0.4, 6.2, 2.6)
    )
    deck.set_chart_title(surface, labelled, "Labels, three tiers")
    deck.set_chart_data_labels(
        surface,
        labelled,
        ChartLabelScope.plot(0),
        DataLabelSpec()
        .value(True)
        .position(DataLabelPosition.OutsideEnd)
        .separator("; ")
        .number_format("0.0"),
    )
    deck.set_chart_data_labels(
        surface, labelled, ChartLabelScope.series(0), DataLabelSpec().category_name(True)
    )
    deck.suppress_chart_data_labels(surface, labelled, ChartLabelScope.point(0, 1))
    deck.suppress_chart_data_labels(surface, labelled, ChartLabelScope.series(1))
    deck.add_chart_trendline(
        surface,
        labelled,
        0,
        TrendlineSpec(TrendlineKind.Polynomial)
        .polynomial_order(3)
        .projection(2.0, 0.0)
        .display(True, True),
    )

    pie = deck.add_chart(
        surface,
        ChartData(ChartKind.Pie)
        .categories(["North", "South", "East", "West"])
        .series("Share", [42.0, 28.0, 18.0, 12.0]),
        ShapeBounds.from_inches(0.4, 3.4, 5.8, 3.4),
    )
    deck.set_chart_title(surface, pie, "Slice 1 exploded, slice 0 recoloured")
    deck.set_chart_point_explosion(surface, pie, 0, 1, 25)
    deck.set_chart_point_fill(surface, pie, 0, 0, FillSpec.solid(ColorSpec.srgb("2E75B6")))

    scatter = deck.add_chart(
        surface,
        ChartData(ChartKind.Scatter)
        .categories(["1", "2", "3", "4"])
        .series("Measured", [2.0, 4.5, 3.25, 6.0]),
        ShapeBounds.from_inches(6.6, 3.4, 6.2, 3.4),
    )
    deck.set_chart_title(surface, scatter, "Error bars on both axes")
    deck.set_chart_error_bars(
        surface,
        scatter,
        0,
        ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.Percentage, 5.0).direction(
            ErrorBarDirection.X
        ),
    )
    deck.set_chart_error_bars(
        surface,
        scatter,
        0,
        ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 0.5).direction(
            ErrorBarDirection.Y
        ),
    )


def _write_axes_and_detached_workbook_area(deck: Deck, surface: int) -> None:
    axes = deck.add_chart(surface, quarterly_chart(), ShapeBounds.from_inches(0.4, 0.4, 6.0, 3.0))
    deck.set_chart_title(surface, axes, "Bounded 0-25, reversed, ruled")
    deck.set_chart_axis_title(surface, axes, 0, "Quarter")
    deck.set_chart_axis_title(surface, axes, 1, "Revenue")
    deck.set_chart_axis_scale(surface, axes, 1, 0.0, 25.0)
    deck.set_chart_axis_orientation(surface, axes, 1, AxisOrientation.MaximumToMinimum)
    deck.set_chart_axis_gridlines(surface, axes, 0, True, False)
    deck.set_chart_axis_gridlines(surface, axes, 1, True, True)
    deck.set_chart_series_fill(surface, axes, 0, FillSpec.solid(ColorSpec.srgb("4472C4")))
    deck.set_chart_series_line(
        surface,
        axes,
        1,
        LineSpec.solid(LineWidth.from_points(2.0), ColorSpec.srgb("ED7D31")),
    )
    detached = deck.add_chart(
        surface, quarterly_chart(), ShapeBounds.from_inches(6.8, 0.4, 6.0, 3.0)
    )
    deck.set_chart_title(surface, detached, "This chart has no embedded workbook")
    deck.detach_chart_workbook(surface, detached)
    deck.chart_workbooks(surface)


def _write_dangling_point_area(deck: Deck, surface: int) -> None:
    shortened = deck.add_chart(
        surface,
        ChartData(ChartKind.Bar).categories(["Q1", "Q2", "Q3"]).series("2026", [4.0, 5.0, 6.0]),
        ShapeBounds.from_inches(0.4, 0.4, 6.0, 3.0),
    )
    deck.set_chart_title(surface, shortened, "A c:dPt left past the end of its series")
    deck.set_chart_point_fill(surface, shortened, 0, 2, FillSpec.solid(ColorSpec.srgb("C00000")))
    deck.set_chart_series_values(surface, shortened, 0, [4.0, 5.0])
    deck.chart_dangling_decoration(surface, shortened, 0)


def _write_plot_type_gallery(deck: Deck) -> None:
    slide = None
    for position, (kind, name) in enumerate(SINGLE_SERIES_KINDS):
        if position % 4 == 0:
            slide = deck.add_slide()
        assert slide is not None
        data = ChartData(kind).categories(["A", "B", "C"]).series("S", [1.0, 2.0, 3.0])
        chart = deck.add_chart(slide, data, _gallery_bounds(position))
        deck.set_chart_title(slide, chart, name)
    assert slide is not None
    stock = deck.add_chart(
        slide,
        ChartData(ChartKind.Stock)
        .categories(["Mon", "Tue", "Wed"])
        .series("High", [7.0, 8.0, 9.0])
        .series("Low", [3.0, 4.0, 5.0])
        .series("Close", [5.0, 6.0, 7.0]),
        _gallery_bounds(3),
    )
    deck.set_chart_title(slide, stock, "stockChart")


def v_pptx_08() -> bytes:
    deck, slide = _one_slide_deck()
    _write_chart_decoration_areas(deck, slide)
    _write_axes_and_detached_workbook_area(deck, deck.add_slide())
    _write_dangling_point_area(deck, deck.add_slide())
    _write_plot_type_gallery(deck)
    return deck.save()


def v_docx_01() -> bytes:
    document = _blank_document()
    _append_heading(document, "Quarterly Review")
    _append_line(
        document,
        "This paragraph states nothing: every property it renders with is inherited.",
    )
    stated = _append_line(document, "North America: +12%")
    _append_line(document, "EMEA: +8%")
    document.effective_run_properties(stated, 0)
    return document.save()


def v_docx_02() -> bytes:
    document = _blank_document()
    _append_heading(document, "Sections, headers and footers")
    document.set_section_page_size(SectionLocation.body(), PageSize.us_letter().landscape())
    document.set_section_page_margins(SectionLocation.body(), PageMargins.normal())
    document.set_header_text(
        SectionLocation.body(), HeaderFooterType.Default, "Quarterly Review — Internal"
    )
    document.set_footer_text(
        SectionLocation.body(), HeaderFooterType.Default, "Page footer, default type"
    )
    document.set_header_text(
        SectionLocation.body(), HeaderFooterType.First, "First page header"
    )
    return document.save()


def v_docx_03() -> bytes:
    document = _blank_document()
    _append_heading(document, "Tables")
    table = document.append_table(3, 3)
    rows = [
        ["Region", "Revenue", "Growth"],
        ["North America", "4.2", "+12%"],
        ["EMEA", "3.1", "+8%"],
    ]
    for row, cells in enumerate(rows):
        for column, text in enumerate(cells):
            document.set_cell_text(table, row, column, text)
    document.set_cell_span(table, 2, 1, 2)
    document.set_cell_vertical_merge(table, 1, 0, MergedCellType.Restart)
    document.set_cell_vertical_merge(table, 2, 0, MergedCellType.Continue)
    return document.save()


def v_docx_04() -> bytes:
    document = _blank_document()
    anchor = _append_heading(document, "Charts")
    chart = document.add_chart(anchor, quarterly_chart(), 6 * INCH, 3 * INCH, "Inline chart")
    document.set_chart_title(chart, "Revenue by quarter")
    document.set_chart_legend(chart, LegendPosition.Bottom)
    document.set_chart_axis_title(chart, 0, "Quarter")
    floating_anchor = _append_line(document, "A floating chart follows this paragraph.")
    floating = document.add_floating_chart(
        floating_anchor,
        quarterly_chart(),
        INCH // 2,
        INCH // 2,
        4 * INCH,
        2 * INCH,
        ChartWrap.square(WrapText.BothSides),
        "Floating chart",
    )
    document.set_chart_title(floating, "The same numbers, wrapped square")
    return document.save()


def v_docx_05() -> bytes:
    document = _blank_document()
    anchor = _append_heading(document, "Pictures")
    document.add_inline_picture(
        anchor,
        DEFAULT_PLACEHOLDER_IMAGE,
        PLACEHOLDER_CONTENT_TYPE,
        PLACEHOLDER_EXTENSION,
        2 * INCH,
        2 * INCH,
        "Placeholder",
    )
    second = _append_line(document, "A second picture, half the size:")
    document.add_inline_picture(
        second,
        DEFAULT_PLACEHOLDER_IMAGE,
        PLACEHOLDER_CONTENT_TYPE,
        PLACEHOLDER_EXTENSION,
        INCH,
        INCH,
        "Placeholder, small",
    )
    return document.save()


def v_docx_06() -> bytes:
    document = _blank_document()
    anchor = _append_heading(document, "Notes, comments and links")
    document.add_footnote(anchor, "Figures are unaudited and subject to revision.")
    document.add_endnote(anchor, "Prepared by the validation harness.")
    document.add_comment(
        anchor, "Reviewer", "R", "Confirm the North America figure before publishing."
    )
    link_paragraph = _append_line(document, "Full figures: ")
    document.insert_hyperlink(
        link_paragraph,
        1,
        "investor relations page",
        HyperlinkTarget.url("https://example.com/investors"),
    )
    return document.save()


# ---------------------------------------------------------------------------------------------
# V-XLSX-01 … V-XLSX-06
# ---------------------------------------------------------------------------------------------


def v_xlsx_01() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Summary")
    cells = quarterly_cells()
    cells.append(CellWrite.shared_text("A6", "Audited"))
    cells.append(CellWrite.boolean("B6", False))
    cells.append(CellWrite.shared_text("A7", "Deliberately in error"))
    cells.append(CellWrite.error("B7", "#N/A"))
    cells.append(
        CellWrite.inline_text(
            "A8", "Inline, not shared: this string is written into the cell"
        )
    )
    cells.append(CellWrite.blank("B8"))
    workbook.write_cells(0, cells)
    notes = workbook.add_sheet("Notes")
    workbook.write_cells(
        notes,
        [CellWrite.inline_text("A1", "Figures are unaudited and subject to revision.")],
    )
    return workbook.save()


def v_xlsx_02() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Formats")
    workbook.write_cells(0, quarterly_cells())

    style_font = workbook.append_font(
        FontProperties(font_name="Times New Roman", italic=True, size_in_points=13.0)
    )
    style_fill = workbook.append_pattern_fill(PatternFillSpec.solid("445566"))
    style_border = workbook.append_border(
        BorderSpec(
            left=BorderEdgeSpec(),
            right=BorderEdgeSpec(style=BorderStyle.Thick, color=Color.from_opaque_rgb("000000")),
            top=BorderEdgeSpec(),
            bottom=BorderEdgeSpec(),
            diagonal=BorderEdgeSpec(),
        )
    )
    beneath = workbook.append_cell_format(
        CellFormatTarget.CellStyleFormats,
        CellFormatSpec.skeleton_cell_style_format().with_resources(
            font_index=style_font, fill_index=style_fill, border_index=style_border
        ),
    )

    direct_font = workbook.append_font(
        FontProperties(font_name="Arial", bold=True, size_in_points=12.0)
    )
    direct_fill = workbook.append_pattern_fill(PatternFillSpec.solid("112233"))
    direct_border = workbook.append_border(
        BorderSpec(
            left=BorderEdgeSpec(style=BorderStyle.Thin, color=Color.from_opaque_rgb("000000")),
            right=BorderEdgeSpec(),
            top=BorderEdgeSpec(),
            bottom=BorderEdgeSpec(),
            diagonal=BorderEdgeSpec(),
        )
    )
    direct = workbook.append_cell_format(
        CellFormatTarget.CellFormats,
        CellFormatSpec(
            number_format_id=0,
            font_index=0,
            fill_index=0,
            border_index=0,
            cell_style_format_index=beneath,
        ).with_resources(
            font_index=direct_font, fill_index=direct_fill, border_index=direct_border
        ),
    )
    for reference in ("A1", "B1", "C1"):
        workbook.set_cell_style(0, reference, direct)
    workbook.effective_cell_format(0, "A1")
    return workbook.save()


def v_xlsx_03() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Grid")
    workbook.write_cells(0, quarterly_cells())
    workbook.merge_cells(0, "A6:C6")
    workbook.set_row_height(0, 0, 30.0, True)
    workbook.set_row_hidden(0, 4, True)
    workbook.set_row_outline_level(0, 2, 1)
    workbook.set_row_outline_level(0, 3, 1)
    workbook.set_column_width(0, 0, 0, 24.0, True)
    workbook.set_column_width(0, 1, 2, 14.0, True)
    workbook.set_column_hidden(0, 5, 5, True)
    return workbook.save()


def v_xlsx_04() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Charts")
    workbook.write_cells(0, quarterly_cells())
    sheet_name = workbook.sheet(0).name
    anchor = workbook.add_range_chart(
        0,
        ChartKind.Bar,
        f"{sheet_name}!$A$2:$A$4",
        [
            ChartRangeSeries("Revenue", f"{sheet_name}!$B$2:$B$4").named_by_cell(
                f"{sheet_name}!$B$1"
            )
        ],
        4,
        1,
        11,
        16,
        "Revenue by region",
        ResizingBehavior.MoveAndResizeWithAnchorCells,
    )
    workbook.set_chart_title(0, anchor, "Revenue by region")
    workbook.set_chart_legend(0, anchor, LegendPosition.Bottom)
    workbook.set_chart_axis_title(0, anchor, 0, "Region")
    return workbook.save()


def v_xlsx_05() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Drawings")
    workbook.write_cells(0, quarterly_cells())
    workbook.add_two_cell_anchored_picture(
        0,
        DEFAULT_PLACEHOLDER_IMAGE,
        "Two-cell anchored",
        4,
        0,
        1,
        0,
        7,
        0,
        8,
        0,
        ResizingBehavior.MoveAndResizeWithAnchorCells,
    )
    workbook.add_one_cell_anchored_picture(
        0, DEFAULT_PLACEHOLDER_IMAGE, "One-cell anchored", 4, 0, 10, 0, 2 * INCH, INCH
    )
    return workbook.save()


def v_xlsx_06() -> bytes:
    workbook = Workbook.blank()
    workbook.rename_sheet(0, "Comments")
    workbook.write_cells(0, quarterly_cells())
    workbook.add_cell_comment(
        0, "B2", "Reviewer", "Confirm the North America figure before publishing."
    )
    workbook.add_cell_comment(0, "C4", "Reviewer", "Growth restated in March.")
    workbook.set_cell_hyperlink_url(0, "A1", "https://example.com/investors")
    return workbook.save()


#: Every area, keyed by the artefact file name the Rust generator writes. The key is what binds this
#: file to the index: a name that stops matching fails the comparison below rather than drifting.
GENERATORS: dict[str, Callable[[], bytes]] = {
    "v-pptx-01-authored.pptx": v_pptx_01,
    "v-pptx-02-authored.pptx": v_pptx_02,
    "v-pptx-03-authored.pptx": v_pptx_03,
    "v-pptx-04-authored.pptx": v_pptx_04,
    "v-pptx-05-authored.pptx": v_pptx_05,
    "v-pptx-06-authored.pptx": v_pptx_06,
    "v-pptx-07-authored.pptx": v_pptx_07,
    "v-pptx-08-authored.pptx": v_pptx_08,
    "v-docx-01-authored.docx": v_docx_01,
    "v-docx-02-authored.docx": v_docx_02,
    "v-docx-03-authored.docx": v_docx_03,
    "v-docx-04-authored.docx": v_docx_04,
    "v-docx-05-authored.docx": v_docx_05,
    "v-docx-06-authored.docx": v_docx_06,
    "v-xlsx-01-authored.xlsx": v_xlsx_01,
    "v-xlsx-02-authored.xlsx": v_xlsx_02,
    "v-xlsx-03-authored.xlsx": v_xlsx_03,
    "v-xlsx-04-authored.xlsx": v_xlsx_04,
    "v-xlsx-05-authored.xlsx": v_xlsx_05,
    "v-xlsx-06-authored.xlsx": v_xlsx_06,
}


@pytest.fixture(scope="session")
def rust_artefacts(tmp_path_factory: pytest.TempPathFactory) -> pathlib.Path:
    """The Rust generator's output, produced once for the whole session."""
    directory = tmp_path_factory.mktemp("validation-artefacts-rust")
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "xtask",
            "--",
            "validation-artefacts",
            "--out",
            str(directory),
        ],
        cwd=str(REPOSITORY_ROOT),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    return directory


def test_the_generator_set_is_the_whole_catalogue(rust_artefacts: pathlib.Path) -> None:
    """This file writes every artefact the Rust generator does, and no others.

    Without this the comparison below is green precisely when a generator is missing — the failure
    mode this project keeps finding. It is stated over the *filesystem* rather than over a list in
    this file, so an area added in Rust and not here fails.
    """
    produced = {path.name for path in rust_artefacts.iterdir()}
    assert produced, "the Rust generator wrote nothing"
    assert len(produced) >= 20, f"only {len(produced)} artefact(s); the catalogue has shrunk"
    assert produced == set(GENERATORS), (
        "the Python generators and the Rust ones name different artefacts: "
        f"only in Rust {sorted(produced - set(GENERATORS))}, "
        f"only in Python {sorted(set(GENERATORS) - produced)}"
    )


@pytest.mark.parametrize("name", sorted(GENERATORS))
def test_every_artefact_matches_the_rust_one(
    name: str, rust_artefacts: pathlib.Path
) -> None:
    """Part for part, byte for byte, against the artefact the Rust generator wrote."""
    from_python = part_payloads(GENERATORS[name]())
    from_rust = part_payloads((rust_artefacts / name).read_bytes())
    assert sorted(from_python) == sorted(from_rust), (
        f"{name}: the two generators author different parts"
    )
    differing = [part for part in from_python if from_python[part] != from_rust[part]]
    assert not differing, f"{name}: these parts differ between Python and Rust: {differing}"
