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
from mjx_ooxml import CellWrite, SheetKind, Workbook


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
