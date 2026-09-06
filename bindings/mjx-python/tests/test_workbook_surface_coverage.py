"""The Excel sibling of `test_surface_coverage.py` and `test_document_surface_coverage.py`:
mis-wiring guards for the argument pairs a swapped delegate could plausibly confuse, plus the
corners `test_build_a_workbook.py`'s walkthrough does not reach.

See `crates/mjx-ooxml/tests/workbook_public_paths.rs` (the Rust original these guards mirror) for
why every assertion here is asymmetric on purpose: a test that writes `"Region"` into `A1` and reads
`"Region"` back out of `A1` passes against a surface that ignores both arguments and holds one value.
"""

from __future__ import annotations

import pathlib

import pytest

import mjx_ooxml
from mjx_ooxml import CellWrite, GeometrySource, ResizingBehavior, SheetKind, Workbook


@pytest.fixture
def filled() -> Workbook:
    """A blank workbook with a 3x2 block of mixed cell kinds, written in one call."""
    workbook = Workbook.blank()
    workbook.write_cells(
        0,
        [
            CellWrite.shared_text("A1", "Region"),
            CellWrite.shared_text("B1", "Growth"),
            CellWrite.inline_text("A2", "North America"),
            CellWrite.number("B2", 12.5),
            CellWrite.boolean("A3", True),
            CellWrite.error("B3", "#DIV/0!"),
        ],
    )
    return workbook


def test_block_rows_and_columns_are_not_transposed(filled: Workbook) -> None:
    block = filled.read_range(0, "A1:B3")
    assert (block.row_count, block.column_count) == (3, 2)

    # Six different values at six different addresses, and the two columns hold different *kinds*.
    assert block.value(0, 0).text == "Region"
    assert block.value(0, 1).text == "Growth"
    assert block.value(1, 0).text == "North America"
    assert block.value(1, 1).number == 12.5
    assert block.value(2, 0).boolean is True
    assert block.value(2, 1).error_code == "#DIV/0!"
    assert block.range == "A1:B3"


def test_a_text_cell_and_an_error_cell_are_told_apart_by_kind(filled: Workbook) -> None:
    """`rows()` cannot distinguish them — both arrive as `str` — and `kinds()` is why that is fine."""
    block = filled.read_range(0, "A1:B3")
    assert block.rows()[2] == [True, "#DIV/0!"]
    assert block.kinds()[2] == ["boolean", "error"]


def test_an_offset_outside_the_block_raises_rather_than_wrapping(filled: Workbook) -> None:
    block = filled.read_range(0, "A1:B3")
    with pytest.raises(mjx_ooxml.IndexOutOfRangeError) as failure:
        block.value(3, 0)
    assert failure.value.row == 3
    assert failure.value.column == 0
    assert isinstance(failure.value, IndexError)


def test_a_batch_with_a_bad_address_writes_nothing(filled: Workbook) -> None:
    before = filled.save()
    with pytest.raises(mjx_ooxml.InvalidArgumentError):
        filled.write_cells(
            0, [CellWrite.number("C1", 1.0), CellWrite.number("not a cell", 2.0)]
        )
    assert filled.save() == before


def test_rows_are_one_based_and_columns_are_zero_based(filled: Workbook) -> None:
    """The one place this surface mixes bases, because SpreadsheetML does: `row@r` is one-based and
    `A` is column 0. Different values on purpose, so a surface that confused the two could not pass.
    """
    filled.set_row_height(0, 2, 24.0)
    filled.set_column_width(0, 0, 1, 18.0)
    filled.set_row_hidden(0, 3, True)
    filled.set_column_hidden(0, 2, 2, True)
    filled.validate()


def test_a_merge_is_found_from_a_covered_cell_and_removed_by_its_own_reference(
    filled: Workbook,
) -> None:
    filled.merge_cells(0, "A1:B1")
    assert filled.merged_ranges(0) == ["A1:B1"]
    assert filled.merged_range_containing(0, "B1") == "A1:B1"
    assert filled.merged_range_containing(0, "A3") is None
    assert filled.unmerge_cells(0, "A1:B1") is True
    assert filled.merged_ranges(0) == []


def test_an_external_link_and_an_internal_jump_are_not_each_other(filled: Workbook) -> None:
    filled.set_cell_hyperlink_url(0, "A2", "https://example.org/north-america")
    filled.set_cell_hyperlink_location(0, "A3", "Sheet1!B2")

    external = filled.cell_hyperlink(0, "A2")
    assert external is not None
    assert external.target == "https://example.org/north-america"
    assert external.location is None

    internal = filled.cell_hyperlink(0, "A3")
    assert internal is not None
    assert internal.location == "Sheet1!B2"
    assert internal.relationship_id is None, "an internal jump writes no relationship"

    assert filled.remove_cell_hyperlink(0, "A2") is True
    assert len(filled.sheet_hyperlinks(0)) == 1
    assert filled.remove_cell_hyperlink(0, "A2") is False


def test_a_comment_writes_both_halves_and_a_delete_takes_both_away(
    fixtures: pathlib.Path,
) -> None:
    """The two halves of an Excel comment, through the binding.

    Asymmetric on purpose: the two cells hold *different* text and *different* authors, so a
    delegate that ignored `reference` and held one comment would fail. The fixture is
    LibreOffice's, so the producer's own two comments are there to be counted against.
    """
    workbook = Workbook.open((fixtures / "cell_comments.xlsx").read_bytes())
    assert len(workbook.sheet_comments(0)) == 2

    checked = workbook.cell_comment(0, "A2")
    assert checked is not None
    assert checked.text == "Checked against the ledger.\nSecond line."
    assert checked.author == "Unknown Author"
    assert checked.comment_box is not None
    assert checked.comment_box.is_visible is True
    assert checked.comment_box.row == 1
    assert checked.comment_box.column == 0
    # Read, never inferred: the anchor is the producer's own string.
    assert checked.comment_box.anchor_text == "1, 23, 0, 0, 2, 47, 3, 1"

    spend = workbook.cell_comment(0, "B1")
    assert spend is not None
    assert spend.text == "Spend is in thousands."
    assert spend.comment_box is not None
    assert spend.comment_box.is_visible is False

    shape_id = workbook.add_cell_comment(0, "C3", "Jai Shukla", "A fresh note.")
    assert shape_id == 1025
    fresh = workbook.cell_comment(0, "C3")
    assert fresh is not None
    assert fresh.author == "Jai Shukla"
    assert fresh.shape_id == 1025
    assert fresh.comment_box is not None
    assert fresh.comment_box.identifier == "_x0000_s1025"

    assert workbook.set_cell_comment_text(0, "C3", "Rewritten.") is True
    assert workbook.set_cell_comment_text(0, "Z9", "nobody") is False
    rewritten = workbook.cell_comment(0, "C3")
    assert rewritten is not None
    assert rewritten.text == "Rewritten."

    assert workbook.remove_cell_comment(0, "C3") is True
    assert workbook.remove_cell_comment(0, "C3") is False
    assert workbook.cell_comment(0, "C3") is None
    # Both halves went: saving would refuse if either were left behind.
    workbook.save()


def test_a_form_control_resolves_to_its_legacy_shape_and_an_ole_object_does_not(
    fixtures: pathlib.Path,
) -> None:
    """The `shapeId` hop, and the two lists it reads from told apart.

    LibreOffice's fixture lists a form control and no OLE object, so the two methods must answer
    differently — a delegate wired to the wrong list would answer the same thing twice.
    """
    workbook = Workbook.open((fixtures / "legacy_form_control.xlsx").read_bytes())
    assert workbook.vml_shape_id_for_form_control(0, 0) == "AcceptTerms"
    assert workbook.vml_shape_id_for_form_control(0, 7) is None
    assert workbook.vml_shape_id_for_ole_object(0, 0) is None

    vml = workbook.sheet_vml_part_bytes(0)
    assert vml is not None
    assert b'o:spid="_x0000_s1001"' in vml
    with pytest.raises(mjx_ooxml.IndexOutOfRangeError):
        workbook.sheet_vml_part_bytes(workbook.sheet_count())


def test_a_tab_that_is_not_there_and_a_tab_with_no_cells_are_different_failures(
    fixtures: pathlib.Path,
) -> None:
    workbook = Workbook.open((fixtures / "print_and_sheet_kinds.xlsx").read_bytes())
    others = [
        index
        for index, sheet in enumerate(workbook.sheets())
        if sheet.kind != SheetKind.Worksheet
    ]
    assert others, "print_and_sheet_kinds.xlsx must hold a tab that is not a worksheet"

    with pytest.raises(mjx_ooxml.NothingToReadError) as no_cells:
        workbook.read_range(others[0], "A1")
    assert no_cells.value.index == others[0]

    with pytest.raises(mjx_ooxml.IndexOutOfRangeError):
        workbook.read_range(workbook.sheet_count(), "A1")


def test_the_feature_reports_answer_from_files_that_hold_the_features(
    fixtures: pathlib.Path,
) -> None:
    """Each report is exercised against a file that has something to report, not only against one
    that does not — the shape of gate this project keeps finding it needs."""
    conditional = Workbook.open((fixtures / "conditional_formatting.xlsx").read_bytes())
    assert conditional.conditional_formatting_ranges(0)
    assert (conditional.conditional_formatting_rule_count(0, 0) or 0) > 0

    filters = Workbook.open((fixtures / "validation_and_filters.xlsx").read_bytes())
    assert filters.auto_filter_range(0) is not None
    assert filters.data_validation_ranges(0)

    tables = Workbook.open((fixtures / "worksheet_tables.xlsx").read_bytes())
    sheet_tables = tables.sheet_tables(0)
    assert sheet_tables and sheet_tables[0].columns

    preserved = Workbook.open((fixtures / "preserved_parts.xlsx").read_bytes())
    assert not preserved.preserved_parts().is_empty

    links = Workbook.open((fixtures / "hyperlinks.xlsx").read_bytes())
    assert links.sheet_hyperlinks(0)


def test_the_workbook_metadata_and_part_graph_are_reachable(fixtures: pathlib.Path) -> None:
    workbook = Workbook.open((fixtures / "sample.xlsx").read_bytes())

    workbook.defined_names()
    assert workbook.defined_name("Nothing") is None
    workbook.print_area(0)
    workbook.date_system()
    workbook.calculation_settings()
    workbook.window_views()
    workbook.active_sheet()
    workbook.sheet_printer_settings(0)
    workbook.sheet_background_image(0)
    workbook.grid_anomalies(0)
    workbook.next_table_id()
    workbook.table_style_origin("TableStyleMedium2")
    workbook.pivot_tables()
    workbook.external_links()
    workbook.connections()
    workbook.query_tables()
    workbook.xml_maps()
    workbook.revision_state()

    assert workbook.workbook_part().endswith("workbook.xml")
    assert len(workbook.part_names()) > 1
    assert workbook.content_type_of(workbook.workbook_part()) is not None
    assert workbook.part_bytes(workbook.workbook_part())
    with pytest.raises(mjx_ooxml.NotFoundError):
        workbook.part_bytes("/xl/nothing.xml")

    workbook.validate()


# A 1x1 PNG — the smallest thing the image sniffer calls a PNG.
PNG = bytes(
    [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
        0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
        0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
)


def test_the_three_anchor_modes_are_not_each_other(filled: Workbook) -> None:
    """Every number differs from every other, so a wrapper that crossed a column with a row, or a
    `from` offset with a `to` offset, fails here rather than in markup nobody reads."""
    two = filled.add_two_cell_anchored_picture(
        0,
        PNG,
        "two-cell",
        1,
        190_500,
        2,
        47_625,
        3,
        95_250,
        5,
        19_050,
        ResizingBehavior.MoveWithCellsButDoNotResize,
    )
    one = filled.add_one_cell_anchored_picture(
        0, PNG, "one-cell", 4, 76_200, 1, 38_100, 914_400, 457_200
    )
    absolute = filled.add_absolute_anchored_picture(
        0, PNG, "absolute", 1_905_000, 952_500, 685_800, 342_900
    )
    assert (two, one, absolute) == (0, 1, 2)

    drawing = filled.sheet_drawing(0)
    assert drawing is not None
    assert drawing.part.endswith("drawing1.xml")
    assert [item.anchor for item in drawing.objects] == [
        "twoCellAnchor",
        "oneCellAnchor",
        "absoluteAnchor",
    ]
    assert [item.name for item in drawing.objects] == ["two-cell", "one-cell", "absolute"]
    assert [item.object for item in drawing.objects] == ["pic", "pic", "pic"]
    # The `@editAs` the caller asked for, read back off the file — and the two anchors that carry
    # none answer from what they are, so the three do not agree.
    assert [item.resizing for item in drawing.objects] == [
        ResizingBehavior.MoveWithCellsButDoNotResize,
        ResizingBehavior.MoveWithCellsButDoNotResize,
        ResizingBehavior.DoNotMoveOrResizeWithRowsOrColumns,
    ]
    # One image, stored once, shown by all three.
    assert {item.image for item in drawing.objects} == {"/xl/media/image1.png"}
    assert all(item.prints_with_sheet for item in drawing.objects)

    filled.save()


def test_an_anchor_a_blank_sheet_cannot_place_and_one_it_can_are_different_answers(
    filled: Workbook,
) -> None:
    filled.add_one_cell_anchored_picture(0, PNG, "one-cell", 4, 76_200, 1, 38_100, 914_400, 457_200)
    filled.add_absolute_anchored_picture(0, PNG, "absolute", 1_905_000, 952_500, 685_800, 342_900)

    # `Workbook.blank` writes no `x:sheetFormatPr`, so `@defaultRowHeight` is stated nowhere and a
    # cell-anchored object cannot be placed. The honest answer is `None`, not Excel's own 15 points.
    assert filled.sheet_anchor_bounds(0, 0, 7.0, 96.0) is None

    # The absolute anchor names no cell, so it is placeable on the very same sheet.
    bounds = filled.sheet_anchor_bounds(0, 1, 7.0, 96.0)
    assert bounds is not None
    assert (bounds.x_emu, bounds.y_emu) == (1_905_000, 952_500)
    assert (bounds.width_emu, bounds.height_emu) == (685_800, 342_900)
    assert bounds.row_source == GeometrySource.Stated
    assert bounds.column_source == GeometrySource.Stated
    assert bounds.maximum_digit_width_pixels == 7.0
    assert bounds.pixels_per_inch == 96.0


def test_a_producer_drawing_resolves_and_the_three_modes_shift_differently(
    fixtures: pathlib.Path,
) -> None:
    workbook = Workbook.open((fixtures / "worksheet_drawings.xlsx").read_bytes())

    # The extent Apache POI computed for the same column widths, reached through the binding alone.
    bounds = workbook.sheet_anchor_bounds(0, 0, 7.0, 96.0)
    assert bounds is not None
    assert (bounds.width_emu, bounds.height_emu) == (2_085_975, 885_825)
    assert bounds.row_source == GeometrySource.SheetDefault
    assert bounds.column_source == GeometrySource.Stated

    report = workbook.insert_rows_into_drawing(0, 0, 3)
    assert len(report) == 4
    assert report[0].moved and not report[0].resized
    assert report[2].moved and not report[2].resized
    assert not report[3].moved and not report[3].resized
    assert report[3].promise == ResizingBehavior.DoNotMoveOrResizeWithRowsOrColumns
    assert all(shift.promise_kept for shift in report)

    # A row inserted *inside* the first anchor resizes it against its own `@editAs`, which the report
    # says rather than silently leaving wrong.
    inside = workbook.insert_rows_into_drawing(0, 7, 1)
    assert inside[0].resized and not inside[0].promise_kept
    assert inside[1].promise_kept

    for shifted in (
        workbook.remove_rows_from_drawing(0, 0, 1),
        workbook.insert_columns_into_drawing(0, 0, 1),
        workbook.remove_columns_from_drawing(0, 0, 1),
    ):
        assert len(shifted) == 4

    assert workbook.remove_sheet_drawing_object(0, 3)
    assert not workbook.remove_sheet_drawing_object(0, 9)
    drawing = workbook.sheet_drawing(0)
    assert drawing is not None and len(drawing.objects) == 3
    workbook.save()


def test_bytes_that_are_not_an_image_are_refused(filled: Workbook) -> None:
    with pytest.raises(mjx_ooxml.InvalidArgumentError):
        filled.add_absolute_anchored_picture(0, b"not an image", "nope", 0, 0, 1, 1)
    assert filled.sheet_drawing(0) is None


def _sample_chart() -> mjx_ooxml.ChartData:
    """The two-series chart every chart case below adds."""
    return (
        mjx_ooxml.ChartData(mjx_ooxml.ChartKind.Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("North", [12.5, 18.0, 21.5])
        .series("South", [9.0, 11.5, 14.0])
    )


def test_the_whole_excel_chart_family_is_bound_and_reads_back(filled: Workbook) -> None:
    """Every chart method on the Excel surface, driven once, with the value read back.

    MJXOFF-111 (E4). The Excel counterpart of `test_document_surface_coverage.py`'s Word chart
    case, and half of MJXOFF-80's own gate: the same names, taking the same vocabulary, from a
    third surface. What differs is only the address — `(sheet, anchor)` where a document takes a
    drawing id — and every assertion below reads back the value it just set, so a method wired to
    the wrong `chart_ops` function answers something else rather than succeeding quietly.
    """
    anchor = filled.add_chart(
        0,
        _sample_chart(),
        4,
        1,
        10,
        16,
        "Revenue",
        ResizingBehavior.MoveWithCellsButDoNotResize,
    )
    assert filled.chart_anchor_indices(0) == [anchor]
    assert filled.chart_rel_id(0, anchor) is not None
    assert filled.chart_part_bytes(0, anchor) is not None
    assert filled.chart_kinds(0, anchor) == [mjx_ooxml.ChartKind.Bar]

    series = filled.chart_series(0, anchor)
    assert [entry.name for entry in series] == ["North", "South"]
    assert list(series[1].values) == [9.0, 11.5, 14.0]

    workbooks = filled.chart_workbooks()
    assert len(workbooks) == 1
    assert (workbooks[0].sheet, workbooks[0].anchor) == (0, anchor)
    assert workbooks[0].external is False
    assert filled.refresh_chart_workbook(0, anchor) is True

    filled.set_chart_title(0, anchor, "Regional revenue")
    assert filled.chart_title(0, anchor) == "Regional revenue"

    filled.set_chart_legend(0, anchor, mjx_ooxml.LegendPosition.Right)
    legend = filled.chart_legend(0, anchor)
    assert legend is not None and legend.position == mjx_ooxml.LegendPosition.Right

    filled.set_chart_series_values(0, anchor, 0, [40.0, 41.0, 42.0])
    assert list(filled.chart_series(0, anchor)[0].values) == [40.0, 41.0, 42.0]
    filled.set_chart_series_categories(0, anchor, 1, ["A", "B", "C"])
    assert list(filled.chart_series(0, anchor)[1].categories) == ["A", "B", "C"]

    filled.set_chart_axis_scale(0, anchor, 1, 0.0, 50.0)
    filled.set_chart_axis_title(0, anchor, 1, "Millions")
    filled.set_chart_axis_gridlines(0, anchor, 1, True, False)
    filled.set_chart_axis_orientation(0, anchor, 1, mjx_ooxml.AxisOrientation.MaximumToMinimum)
    axis = filled.chart_axes(0, anchor)[1]
    assert (axis.minimum, axis.maximum) == (0.0, 50.0)
    assert axis.title == "Millions"

    blue = mjx_ooxml.FillSpec.solid(mjx_ooxml.ColorSpec.srgb("1F77B4"))
    filled.set_chart_series_fill(0, anchor, 0, blue)
    assert filled.chart_series_fill(0, anchor, 0) == blue
    filled.set_chart_series_line(0, anchor, 0, _thin_black_outline())

    filled.set_chart_data_labels(
        0,
        anchor,
        mjx_ooxml.ChartLabelScope.series(0),
        mjx_ooxml.DataLabelSpec().value(True),
    )
    assert filled.chart_data_labels(0, anchor, 0).shows_value is True
    assert filled.chart_data_label_tier(0, anchor, mjx_ooxml.ChartLabelScope.series(0)) is not None
    assert filled.chart_point_label_text(0, anchor, 0, 0) is None

    filled.set_chart_point_fill(0, anchor, 0, 1, blue)
    filled.set_chart_point_explosion(0, anchor, 0, 1, 25)
    filled.set_chart_point_line(0, anchor, 0, 1, _thin_black_outline())
    assert len(filled.chart_point_formats(0, anchor, 0)) == 1

    filled.add_chart_trendline(0, anchor, 0, mjx_ooxml.TrendlineSpec(mjx_ooxml.TrendlineKind.Linear))
    assert len(filled.chart_trendlines(0, anchor, 0)) == 1
    filled.set_chart_trendline(
        0, anchor, 0, 0, mjx_ooxml.TrendlineSpec(mjx_ooxml.TrendlineKind.Logarithmic)
    )
    assert filled.chart_trendlines(0, anchor, 0)[0].kind == mjx_ooxml.TrendlineKind.Logarithmic
    assert filled.remove_chart_trendlines(0, anchor, 0) == 1

    filled.set_chart_error_bars(
        0,
        anchor,
        0,
        mjx_ooxml.ErrorBarSpec.fixed(
            mjx_ooxml.ErrorBarType.Both, mjx_ooxml.ErrorValueType.FixedValue, 1.5
        ),
    )
    assert len(filled.chart_error_bars(0, anchor, 0)) == 1
    assert filled.remove_chart_error_bars(0, anchor, 0) == 1

    assert filled.chart_dangling_decoration(0, anchor, 0) == []
    assert filled.drop_chart_dangling_decoration(0, anchor, 0) == 0
    assert filled.remove_chart_point_format(0, anchor, 0, 1) is True
    filled.suppress_chart_data_labels(0, anchor, mjx_ooxml.ChartLabelScope.series(1))
    assert filled.remove_chart_data_labels(0, anchor, mjx_ooxml.ChartLabelScope.series(0)) is True
    assert filled.chart_style_id(0, anchor) is None

    filled.detach_chart_workbook(0, anchor)
    assert filled.refresh_chart_workbook(0, anchor) is False
    filled.save()


def _thin_black_outline() -> mjx_ooxml.LineSpec:
    """A one-point black outline — the same one the Word chart case builds."""
    return mjx_ooxml.LineSpec.solid(
        mjx_ooxml.LineWidth.from_points(1.0), mjx_ooxml.ColorSpec.srgb("000000")
    )


def test_a_chart_over_a_live_range_reads_the_cells_and_reports_a_stale_cache(
    filled: Workbook,
) -> None:
    """The half of the chart family only the Excel surface has, and the trap MJXOFF-111 names.

    The cache and the cells are made to **disagree** before anything is asserted about which one a
    reader answered: a chart whose cached values equalled its cell values would let a reader wired
    to either source pass.
    """
    filled.write_cells(0, [CellWrite.number("B4", 7.25), CellWrite.number("B5", 9.5)])
    anchor = filled.add_range_chart(
        0,
        mjx_ooxml.ChartKind.Line,
        "Sheet1!$A$1:$A$3",
        [mjx_ooxml.ChartRangeSeries("Growth", "Sheet1!$B$4:$B$5").named_by_cell("Sheet1!$B$1")],
        4,
        1,
        10,
        16,
        "Live",
        ResizingBehavior.MoveWithCellsButDoNotResize,
    )

    # No embedded workbook, and asking for one does not make one.
    assert filled.refresh_chart_workbook(0, anchor) is False
    assert filled.chart_workbooks() == []

    # The series took its name from the header cell rather than from the literal fallback, and its
    # caches were seeded from the cells.
    series = filled.chart_series(0, anchor)
    assert series[0].name == "Growth"
    assert list(series[0].values) == [7.25, 9.5]
    references = filled.chart_series_references(0, anchor)
    assert references[0].values == "Sheet1!$B$4:$B$5"
    assert references[0].name == "Sheet1!$B$1"

    # The resolver, reached directly. A blank cell is absent rather than zero.
    resolved = filled.resolve_range_reference(0, "Sheet1!$B$2:$B$5")
    assert resolved.fully_resolved is True
    assert resolved.problem is None
    assert resolved.addressed_cells == 4
    assert [cell.reference for cell in resolved.cells] == ["B2", "B3", "B4", "B5"]
    assert resolved.cells[0].number == 12.5
    assert resolved.cells[1].number is None and resolved.cells[1].label == "#DIV/0!"

    missing = filled.resolve_range_reference(0, "Ghost!$A$1")
    assert missing.fully_resolved is False
    assert "Ghost" in (missing.problem or "")

    # Writing a cell leaves the cache alone — and the freshness report is how a caller finds out.
    assert filled.chart_series_freshness(0, anchor)[0].values_agree is True
    filled.write_cells(0, [CellWrite.number("B4", 99.0)])
    assert list(filled.chart_series(0, anchor)[0].values) == [7.25, 9.5]
    freshness = filled.chart_series_freshness(0, anchor)
    assert freshness[0].values_agree is False
    assert list(freshness[0].cached.values) == [7.25, 9.5]
    assert list(freshness[0].from_cells.values) == [99.0, 9.5]
    assert list(filled.chart_series_from_cells(0, anchor)[0].values) == [99.0, 9.5]

    # …and the opt-in repair brings the two back into step.
    assert filled.refresh_chart_cache_from_cells(0, anchor) == 1
    assert list(filled.chart_series(0, anchor)[0].values) == [99.0, 9.5]
    filled.save()


def test_removing_an_excel_chart_binding_is_caught_by_this_suite() -> None:
    """The parity clause's own guard: **remove one binding and this case goes red.**

    `bindings/mjx-wasm/tests/node/workbook_surface.mjs` has carried this guard since MJXOFF-111;
    the Python half of the same pair had none, which MJXOFF-118 found while checking that all
    three pairs enforce the clause rather than one of them. Delete
    `Workbook::chart_anchor_indices` from `bindings/mjx-python/src/workbook.rs` and this fails
    with `AttributeError` before any chart case above runs.
    """
    workbook = mjx_ooxml.Workbook.blank()
    for method in (
        "chart_anchor_indices",
        "chart_rel_id",
        "chart_part_bytes",
        "add_chart",
        "add_range_chart",
        "chart_series",
        "chart_kinds",
        "chart_axes",
        "chart_title",
        "chart_legend",
        "chart_workbooks",
        "refresh_chart_workbook",
        "detach_chart_workbook",
        "chart_series_references",
        "chart_series_from_cells",
        "chart_series_freshness",
        "refresh_chart_cache_from_cells",
        "resolve_range_reference",
        "set_chart_series_values",
        "set_chart_data_labels",
        "add_chart_trendline",
        "set_chart_error_bars",
        "drop_chart_dangling_decoration",
    ):
        assert callable(getattr(workbook, method)), f"Workbook.{method} is not bound"
