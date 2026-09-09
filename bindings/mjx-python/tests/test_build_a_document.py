"""The building-a-document walkthrough, written through the Python binding.

This is `crates/mjx-ooxml/examples/build_a_document.rs` call for call — the same paragraphs, the
same numbered list, the same hyperlink, the same table, the same header, the same comment, the same
footnote. `bindings/mjx-wasm/tests/node/build_a_document.mjs` is the third copy, and all three are
compared against the Rust one **part by part, byte for byte** by
`test_the_three_word_walkthroughs_agree` below.

That comparison is the reason this file is shaped the way it is. Until MJXOFF-239 it was not here:
this file transcribed the Rust example and wrote its own `.docx`, and nothing ever ran the Rust one
or looked at the two together. Both files passed, which is exactly the failure that is invisible
from a test report — *a `Document` method wired to the wrong facade method produced a different
document and the suite stayed green, because there was nothing to be different from.*
"""

from __future__ import annotations

import dataclasses
import pathlib
import subprocess

import pytest

import mjx_ooxml
from mjx_ooxml import (
    Document,
    Format,
    HeaderFooterType,
    HyperlinkTarget,
    PageSize,
    SectionLocation,
)

from opc import part_payloads

OUTPUT_NAME = "python_build_a_document.docx"


@dataclasses.dataclass
class Walkthrough:
    """What the walkthrough noticed on its way through, so the assertions can name it."""

    saved: bytes
    paragraph_count: int


def build_the_guides_document() -> Walkthrough:
    """The walkthrough itself, so the comparison below and the assertions above share one source."""
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
    return Walkthrough(
        saved=document.save(),
        paragraph_count=document.paragraph_count(),
    )


def test_the_word_walkthrough_runs_end_to_end_through_the_python_binding(
    output_directory: pathlib.Path,
) -> None:
    run = build_the_guides_document()
    assert len(run.saved) > 0
    (output_directory / OUTPUT_NAME).write_bytes(run.saved)

    # ---- Reopen, to prove the bytes are a real document --------------------------------------------
    reopened = Document.open(run.saved)
    assert reopened.paragraph_count() == run.paragraph_count
    assert reopened.paragraph_text(0) == "Quarterly Review"
    assert reopened.cell_text(0, 0, 0) == "Region"
    assert reopened.header_text(0, HeaderFooterType.Default) == "Quarterly Review — Internal"
    assert len(reopened.comments()) == 1
    assert len(reopened.footnotes()) == 1


def test_document_open_refuses_a_presentation_by_name(fixtures: pathlib.Path) -> None:
    presentation_bytes = (fixtures / "sample.pptx").read_bytes()
    try:
        Document.open(presentation_bytes)
    except mjx_ooxml.UnsupportedFormatError as failure:
        assert failure.code == "UnsupportedFormat"
    else:
        raise AssertionError("Document.open must refuse a PresentationML package")


@pytest.mark.skipif(
    subprocess.run(["cargo", "--version"], capture_output=True).returncode != 0,
    reason="cargo is not on PATH, so the Rust walkthrough cannot be run to compare against",
)
def test_the_three_word_walkthroughs_agree(output_directory: pathlib.Path) -> None:
    """This walkthrough and the Rust one produce the *same document*, part for part.

    Not "both produce a file", and not "both produce a file of about the right size": the same part
    names, and byte-identical payloads for every one of them. That is the only assertion that can
    tell a faithful binding from a plausible one — a method wired to the wrong `Document` method, a
    paragraph index off by one, or an argument converted with the wrong units, changes a payload
    here and nothing else would notice.

    The Node walkthrough is compared against the same reference by
    `bindings/mjx-wasm/tests/node/build_a_document.mjs`, which writes its document beside these two.
    """
    from_python = build_the_guides_document().saved

    rust_output = output_directory / "facade_build_a_document.docx"
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mjx-ooxml",
            "--example",
            "build_a_document",
            "--",
            str(rust_output),
        ],
        cwd=str(pathlib.Path(__file__).resolve().parents[3]),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

    python_parts = part_payloads(from_python)
    rust_parts = part_payloads(rust_output.read_bytes())

    assert sorted(python_parts) == sorted(rust_parts), (
        "the two walkthroughs must author the same set of parts"
    )
    differing = [name for name in python_parts if python_parts[name] != rust_parts[name]]
    assert not differing, f"these parts differ between the Python and Rust walkthroughs: {differing}"
