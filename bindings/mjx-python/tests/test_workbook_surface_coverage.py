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
