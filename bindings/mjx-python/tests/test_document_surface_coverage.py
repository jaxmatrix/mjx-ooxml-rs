"""The Word sibling of `test_surface_coverage.py`: mis-wiring guards for the pairs a swapped
delegate could plausibly confuse, plus the corners `test_build_a_document.py`'s walkthrough does
not reach (effective properties, fields, numbering, table merges).

See `document_delegate_wiring.rs` (the Rust original this file's guards mirror) for why every
assertion here is asymmetric on purpose.
"""

from __future__ import annotations

import pathlib

import pytest

from mjx_ooxml import (
    CellBorderEdge,
    Document,
    HeaderFooterType,
    HyperlinkTarget,
    MergedCellType,
    PageSize,
    SectionLocation,
)


@pytest.fixture
def document() -> Document:
    """A blank document with one empty paragraph."""
    return Document.blank(PageSize.a4())


def test_paragraph_and_run_addressing_are_not_transposed(document: Document) -> None:
    document.append_paragraph()
    document.append_run(0, "first paragraph")
    document.append_run(1, "second paragraph")

    assert document.paragraph_text(0) == "first paragraph"
    assert document.paragraph_text(1) == "second paragraph"
    assert document.run_text(0, 0) == "first paragraph"
    assert document.run_text(1, 0) == "second paragraph"


def test_table_row_and_column_are_not_transposed(document: Document) -> None:
    table = document.append_table(3, 2)
    document.set_cell_text(table, 2, 1, "row 2, col 1")

    assert document.cell_text(table, 2, 1) == "row 2, col 1"
    assert document.cell_text(table, 1, 0) == ""
    assert document.table_dimensions(table) == (3, 2)


def test_header_and_footer_are_not_each_other(document: Document) -> None:
    body = SectionLocation.body()
    document.set_header_text(body, HeaderFooterType.Default, "top of page")
    document.set_footer_text(body, HeaderFooterType.Default, "bottom of page")

    assert document.header_text(0, HeaderFooterType.Default) == "top of page"
    assert document.footer_text(0, HeaderFooterType.Default) == "bottom of page"

    document.remove_header(body, HeaderFooterType.Default)
    assert document.footer_text(0, HeaderFooterType.Default) == "bottom of page"


def test_cell_span_and_vertical_merge_are_not_each_other(document: Document) -> None:
    table = document.append_table(2, 2)
    document.set_cell_vertical_merge(table, 0, 0, MergedCellType.Restart)
    document.set_cell_vertical_merge(table, 1, 0, MergedCellType.Continue)

    row_span, column_span = document.cell_span(table, 0, 0)
    assert (row_span, column_span) == (2, 1), "a vertical merge must widen the row span"

    anchor = document.merged_cell_anchor(table, 1, 0)
    assert anchor == (0, 0)


def test_hyperlink_url_and_anchor_are_not_each_other(document: Document) -> None:
    document.append_paragraph()
    document.insert_hyperlink(0, 0, "external", HyperlinkTarget.url("https://example.org/"))
    document.insert_hyperlink(1, 0, "internal", HyperlinkTarget.anchor("bookmark"))

    external = document.hyperlink_target(0, 0)
    assert external is not None and external.is_url and external.url_value == "https://example.org/"
    internal = document.hyperlink_target(1, 0)
    assert internal is not None and not internal.is_url and internal.anchor_value == "bookmark"


def test_field_instruction_and_cached_result_are_not_each_other() -> None:
    fixtures = pathlib.Path(__file__).resolve().parents[3] / "tests" / "fixtures"
    document = Document.open((fixtures / "fields_and_hyperlinks.docx").read_bytes())

    before = document.fields(1)
    assert before[0].instruction == ' HYPERLINK "http://example.com" '
    assert before[0].cached_result == "example.com"

    document.set_field_instruction(1, [0], ' HYPERLINK "http://example.org" ')
    after_instruction = document.fields(1)
    assert after_instruction[0].instruction == ' HYPERLINK "http://example.org" '
    assert after_instruction[0].cached_result == "example.com", (
        "set_field_instruction reached the cached result"
    )

    document.set_field_cached_result_text(1, [0], "example.org")
    after_result = document.fields(1)
    assert after_result[0].instruction == ' HYPERLINK "http://example.org" ', (
        "set_field_cached_result_text reached the instruction"
    )
    assert after_result[0].cached_result == "example.org"


def test_accepted_and_rejected_revision_text_agree_with_nothing_to_diverge_on(
    document: Document,
) -> None:
    document.append_run(0, "steady state")
    assert document.text_with_revisions_accepted() == document.text_with_revisions_rejected()


def test_effective_properties_and_table_border_edges_are_reachable(document: Document) -> None:
    run_props = document.effective_run_properties(0, 0)
    assert run_props.bold is None, "a blank document sets no bold"
    paragraph_props = document.effective_paragraph_properties(0)
    assert paragraph_props.outline_level is None

    table = document.append_table(1, 1)
    assert document.effective_cell_fill(table, 0, 0) is None
    assert document.effective_cell_border(table, 0, 0, CellBorderEdge.Top) is None
    cell_run_props = document.effective_cell_run_properties(table, 0, 0, 0, 0)
    assert cell_run_props.bold is None


def test_numbering_attach_and_detach_do_not_error_with_no_numbering_part(
    document: Document,
) -> None:
    # `w:numPr` is written into the paragraph's own `w:pPr` directly; resolving it against
    # `word/numbering.xml` is a separate step, so this needs no numbering definitions part to
    # exist yet.
    document.attach_paragraph_to_list(0, 1, 0)
    document.detach_paragraph_from_list(0)


def test_a_word_error_names_where_it_happened(document: Document) -> None:
    with pytest.raises(Exception) as failure:
        document.paragraph_text(99)
    assert failure.value.code == "IndexOutOfRange"  # type: ignore[attr-defined]


def test_removing_a_document_binding_is_caught_by_this_suite(document: Document) -> None:
    # The proof this suite would fail if a method were unbound: `paragraph_count` really is a
    # bound method reachable on the class, not a value coincidentally present some other way.
    assert callable(document.paragraph_count)
