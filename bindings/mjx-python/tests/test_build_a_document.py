"""The building-a-document walkthrough, written through the Python binding.

This is `crates/mjx-ooxml/examples/build_a_document.rs` call for call — the same paragraphs, the
same numbered list, the same hyperlink, the same table, the same header, the same comment, the same
footnote. `bindings/mjx-wasm/tests/node/build_a_document.mjs` is the third copy.
"""

from __future__ import annotations

import os
import pathlib

import mjx_ooxml
from mjx_ooxml import Document, Format, HeaderFooterType, HyperlinkTarget, PageSize, SectionLocation

OUTPUT_NAME = "python_build_a_document.docx"


def _output_directory() -> pathlib.Path:
    configured = os.environ.get("MJX_OUTPUT_DIR")
    if configured:
        return pathlib.Path(configured)
    return pathlib.Path(__file__).resolve().parents[3] / "target" / "examples"


def test_the_word_walkthrough_runs_end_to_end_through_the_python_binding() -> None:
    document = Document.blank(PageSize.a4())
    assert document.format() == Format.Document
    assert document.paragraph_count() == 1

    # ---- Paragraphs and runs -----------------------------------------------------------------
    document.append_run(0, "Quarterly Review")
    document.append_paragraph()
    document.append_run(1, "Prepared by the mjx-ooxml-rs example suite.")
    document.append_paragraph()
    document.append_run(2, "Highlights")
    document.append_paragraph()
    document.append_run(3, "Revenue grew across every region this quarter.")

    document.append_paragraph()
    document.append_run(4, "North America: +12%")
    document.attach_paragraph_to_list(4, 1, 0)
    document.append_paragraph()
    document.append_run(5, "EMEA: +8%")
    document.attach_paragraph_to_list(5, 1, 0)

    # ---- A hyperlink ---------------------------------------------------------------------------
    document.append_paragraph()
    document.append_run(6, "Full figures: ")
    document.insert_hyperlink(
        6, 1, "investor relations page", HyperlinkTarget.url("https://example.com/investors")
    )

    # ---- A table ---------------------------------------------------------------------------------
    table = document.append_table(2, 2)
    document.set_cell_text(table, 0, 0, "Region")
    document.set_cell_text(table, 0, 1, "Growth")
    document.set_cell_text(table, 1, 0, "North America")
    document.set_cell_text(table, 1, 1, "+12%")
    assert document.table_dimensions(table) == (2, 2)

    # ---- A header and a comment -------------------------------------------------------------------
    body = SectionLocation.body()
    document.set_header_text(body, HeaderFooterType.Default, "Quarterly Review — Internal")
    comment_id = document.add_comment(
        0, "Reviewer", "R", "Confirm the North America figure before publishing."
    )
    assert document.comment_range_text(comment_id) is not None

    # ---- A footnote --------------------------------------------------------------------------------
    document.add_footnote(3, "Figures are unaudited and subject to revision.")

    # ---- Save --------------------------------------------------------------------------------------
    document.validate()
    saved = document.save()
    assert len(saved) > 0
    output_directory = _output_directory()
    output_directory.mkdir(parents=True, exist_ok=True)
    (output_directory / OUTPUT_NAME).write_bytes(saved)

    # ---- Reopen, to prove the bytes are a real document --------------------------------------------
    reopened = Document.open(saved)
    assert reopened.paragraph_count() == document.paragraph_count()
    assert reopened.paragraph_text(0) == "Quarterly Review"
    assert reopened.cell_text(0, 0, 0) == "Region"
    assert reopened.header_text(0, HeaderFooterType.Default) == "Quarterly Review — Internal"
    assert len(reopened.comments()) == 1
    assert len(reopened.footnotes()) == 1


def test_document_open_refuses_a_presentation_by_name() -> None:
    fixtures = pathlib.Path(__file__).resolve().parents[3] / "tests" / "fixtures"
    presentation_bytes = (fixtures / "sample.pptx").read_bytes()
    try:
        Document.open(presentation_bytes)
    except mjx_ooxml.UnsupportedFormatError as failure:
        assert failure.code == "UnsupportedFormat"
    else:
        raise AssertionError("Document.open must refuse a PresentationML package")
