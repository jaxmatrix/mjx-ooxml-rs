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
    AxisOrientation,
    CellBorderEdge,
    ChartData,
    ChartKind,
    ChartLabelScope,
    ChartWrap,
    ColorSpec,
    DataLabelSpec,
    Document,
    ErrorBarSpec,
    ErrorBarType,
    ErrorValueType,
    FillSpec,
    HeaderFooterType,
    HyperlinkTarget,
    LegendPosition,
    MergedCellType,
    PageSize,
    SectionLocation,
    LineSpec,
    LineWidth,
    TrendlineKind,
    TrendlineSpec,
    WrapText,
)


def thin_black_outline() -> LineSpec:
    """A one-point black outline — the simplest `LineSpec` these cases need."""
    return LineSpec.solid(LineWidth.from_points(1.0), ColorSpec.srgb("000000"))


def sample_chart() -> ChartData:
    """The two-series chart every chart case below adds."""
    return (
        ChartData(ChartKind.Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("North", [12.5, 18.0, 21.5])
        .series("South", [9.0, 11.5, 14.0])
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
    # `Document.blank` writes one *empty* paragraph — no run — matching the Rust sibling
    # (`document_public_paths.rs`), which appends a run before reading effective run properties
    # for the same reason.
    document.append_run(0, "text")
    run_props = document.effective_run_properties(0, 0)
    assert run_props.bold is None, "a blank document sets no bold"
    paragraph_props = document.effective_paragraph_properties(0)
    assert paragraph_props.outline_level is None

    table = document.append_table(1, 1)
    assert document.effective_cell_fill(table, 0, 0) is None
    assert document.effective_cell_border(table, 0, 0, CellBorderEdge.Top) is None
    # Likewise, a freshly appended table cell starts with one empty paragraph and no run.
    document.set_cell_text(table, 0, 0, "cell text")
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


# ---------------------------------------------------------------------------------------------
# Charts (MJXOFF-103) — the Word half of A10's parity rule
# ---------------------------------------------------------------------------------------------


def test_the_whole_word_chart_family_is_bound_and_reads_back(document: Document) -> None:
    """Every chart method on the Word surface, driven once, with the value read back.

    This is the Python half of the parity clause: `Deck` has had this family since Phase A, and
    a facade method without a Python **and** a TypeScript equivalent is an incomplete task. A
    method missing from the binding is an `AttributeError` here; a method bound to the *wrong*
    delegate is a wrong value, which is why every assertion below reads something back rather
    than merely calling.
    """
    drawing = document.add_chart(0, sample_chart(), 4_572_000, 2_743_200, "Revenue")
    assert document.chart_drawing_ids() == [drawing]
    assert document.chart_rel_id(drawing) is not None
    assert document.chart_part_bytes(drawing) is not None
    assert document.chart_kinds(drawing) == [ChartKind.Bar]

    series = document.chart_series(drawing)
    assert [entry.name for entry in series] == ["North", "South"]
    assert list(series[0].values) == [12.5, 18.0, 21.5]

    workbooks = document.chart_workbooks()
    assert len(workbooks) == 1
    assert workbooks[0].drawing_id == drawing
    assert not workbooks[0].external
    assert document.refresh_chart_workbook(drawing) is True

    document.set_chart_title(drawing, "Regional revenue")
    assert document.chart_title(drawing) == "Regional revenue"

    document.set_chart_legend(drawing, LegendPosition.Right)
    legend = document.chart_legend(drawing)
    assert legend is not None and legend.position == LegendPosition.Right

    document.set_chart_series_values(drawing, 0, [40.0, 41.0, 42.0])
    assert list(document.chart_series(drawing)[0].values) == [40.0, 41.0, 42.0]

    document.set_chart_series_categories(drawing, 1, ["A", "B", "C"])
    assert list(document.chart_series(drawing)[1].categories) == ["A", "B", "C"]

    document.set_chart_axis_scale(drawing, 1, 0.0, 50.0)
    document.set_chart_axis_title(drawing, 1, "Millions")
    document.set_chart_axis_gridlines(drawing, 1, True, False)
    document.set_chart_axis_orientation(drawing, 1, AxisOrientation.MaximumToMinimum)
    axis = document.chart_axes(drawing)[1]
    assert axis.minimum == 0.0
    assert axis.maximum == 50.0
    assert axis.title == "Millions"
    assert axis.major_gridlines is True
    assert axis.orientation == AxisOrientation.MaximumToMinimum

    blue = FillSpec.solid(ColorSpec.srgb("1F77B4"))
    document.set_chart_series_fill(drawing, 0, blue)
    assert document.chart_series_fill(drawing, 0) == blue
    document.set_chart_series_line(drawing, 0, thin_black_outline())

    document.set_chart_data_labels(
        drawing, ChartLabelScope.series(0), DataLabelSpec().value(True)
    )
    assert document.chart_data_labels(drawing, 0).shows_value is True
    tier = document.chart_data_label_tier(drawing, ChartLabelScope.series(0))
    assert tier is not None and tier.shows_value is True
    assert document.chart_point_label_text(drawing, 0, 0) is None

    document.set_chart_point_fill(drawing, 0, 1, blue)
    document.set_chart_point_explosion(drawing, 0, 1, 25)
    formats = document.chart_point_formats(drawing, 0)
    assert [entry.index for entry in formats] == [1]
    assert formats[0].explosion == 25
    document.set_chart_point_line(drawing, 0, 1, thin_black_outline())

    document.add_chart_trendline(drawing, 0, TrendlineSpec(TrendlineKind.Linear))
    assert len(document.chart_trendlines(drawing, 0)) == 1
    document.set_chart_trendline(drawing, 0, 0, TrendlineSpec(TrendlineKind.Logarithmic))
    assert document.chart_trendlines(drawing, 0)[0].kind == TrendlineKind.Logarithmic
    assert document.remove_chart_trendlines(drawing, 0) == 1

    document.set_chart_error_bars(
        drawing, 0, ErrorBarSpec.fixed(ErrorBarType.Both, ErrorValueType.FixedValue, 1.5)
    )
    assert len(document.chart_error_bars(drawing, 0)) == 1
    assert document.remove_chart_error_bars(drawing, 0) == 1

    assert document.chart_dangling_decoration(drawing, 0) == []
    assert document.drop_chart_dangling_decoration(drawing, 0) == 0
    assert document.remove_chart_point_format(drawing, 0, 1) is True
    document.suppress_chart_data_labels(drawing, ChartLabelScope.series(1))
    assert document.remove_chart_data_labels(drawing, ChartLabelScope.series(0)) is True

    assert document.chart_style_id(drawing) is None
    document.detach_chart_workbook(drawing)
    assert document.chart_workbooks() == []


def test_a_floating_word_chart_takes_each_of_the_three_wraps(document: Document) -> None:
    for wrap in (ChartWrap.none(), ChartWrap.square(WrapText.BothSides), ChartWrap.top_and_bottom()):
        fresh = Document.blank(PageSize.a4())
        drawing = fresh.add_floating_chart(
            0, sample_chart(), 228_600, 114_300, 4_572_000, 2_743_200, wrap, "Floating"
        )
        assert fresh.chart_drawing_ids() == [drawing]
        assert fresh.chart_kinds(drawing) == [ChartKind.Bar]
    assert ChartWrap.square(WrapText.Left).kind == "square"
    assert ChartWrap.square(WrapText.Left).wrap_text == WrapText.Left
    assert ChartWrap.none().wrap_text is None


def test_a_word_chart_refusal_carries_the_same_code_the_deck_surface_uses(
    document: Document,
) -> None:
    drawing = document.add_chart(0, sample_chart(), 914_400, 914_400, "Revenue")
    with pytest.raises(Exception) as failure:
        document.chart_series_fill(drawing, 7)
    assert failure.value.code == "IndexOutOfRange"  # type: ignore[attr-defined]

    with pytest.raises(Exception) as failure:
        document.chart_series(9_999)
    assert failure.value.code == "WrongKind"  # type: ignore[attr-defined]


def test_removing_a_word_chart_binding_is_caught_by_this_suite(document: Document) -> None:
    """The parity clause's own guard: **remove one binding and this case goes red.**

    A clause nothing checks quietly stops being true, so this names the binding explicitly
    rather than trusting that some other case would have called it. Delete
    `Document::chart_drawing_ids` from `bindings/mjx-python/src/document.rs` and this fails with
    `AttributeError` before any of the chart cases above even runs.
    """
    for method in (
        "chart_drawing_ids",
        "add_chart",
        "add_floating_chart",
        "chart_series",
        "chart_kinds",
        "chart_axes",
        "chart_title",
        "chart_legend",
        "chart_workbooks",
        "refresh_chart_workbook",
        "detach_chart_workbook",
        "set_chart_series_values",
        "set_chart_data_labels",
        "add_chart_trendline",
        "set_chart_error_bars",
        "drop_chart_dangling_decoration",
    ):
        assert callable(getattr(document, method)), f"Document.{method} is not bound"
