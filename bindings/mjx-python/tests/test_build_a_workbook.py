"""The building-a-workbook walkthrough, written through the Python binding.

This is `crates/mjx-ooxml/examples/build_a_workbook.rs` call for call — the same tabs, the same
cells in the same single batched write, the same style, the same geometry, the same hyperlink.
`bindings/mjx-wasm/tests/node/build_a_workbook.mjs` is the third copy, and all three are compared
against the Rust one **part by part, byte for byte**.

# The one thing to notice

Every cell is written in one `write_cells` call and read back in one `read_range`. That is the only
shape this binding offers: `mjx_xlsx::Workbook` holds no parsed worksheet, so a per-cell call costs a
whole-worksheet parse every time it is made — and a Python caller has no escape hatch to reach the
Rust answer with.
"""

from __future__ import annotations

import io
import pathlib
import subprocess
import zipfile

import pytest

import mjx_ooxml
from mjx_ooxml import (
    BorderEdgeSpec,
    BorderSpec,
    BorderStyle,
    CellFormatSpec,
    CellFormatTarget,
    CellWrite,
    Color,
    ColorSchemeSlot,
    FontProperties,
    Format,
    OoxmlError,
    PatternFillSpec,
    Workbook,
)

OUTPUT_NAME = "python_build_a_workbook.xlsx"


def build_the_guides_workbook() -> bytes:
    """The walkthrough itself, so the comparison below and the assertions above share one source."""
    workbook = Workbook.blank()
    assert workbook.format() == Format.Workbook
    assert workbook.sheet_count() == 1

    # ---- Tabs ----------------------------------------------------------------------------------
    workbook.rename_sheet(0, "Summary")
    notes = workbook.add_sheet("Notes")
    assert notes == 1
    assert workbook.sheet_index("Summary") == 0

    # ---- Cells: one call, every cell -------------------------------------------------------------
    workbook.write_cells(
        0,
        [
            CellWrite.shared_text("A1", "Region"),
            CellWrite.shared_text("B1", "Revenue"),
            CellWrite.shared_text("C1", "Growth"),
            CellWrite.shared_text("A2", "North America"),
            CellWrite.number("B2", 1_250_000.0),
            CellWrite.number("C2", 0.125),
            CellWrite.shared_text("A3", "Europe"),
            CellWrite.number("B3", 980_000.0),
            CellWrite.number("C3", 0.061),
            CellWrite.shared_text("A4", "Asia Pacific"),
            CellWrite.number("B4", 1_410_000.0),
            CellWrite.number("C4", 0.198),
            CellWrite.shared_text("A5", "Audited"),
            CellWrite.boolean("B5", False),
        ],
    )
    workbook.write_cells(
        notes,
        [CellWrite.inline_text("A1", "Figures are unaudited and subject to revision.")],
    )

    # ---- A style, built once and pointed at -------------------------------------------------------
    # The heading's fill is a theme slot, its text a literal — see the Rust walkthrough.
    font = workbook.append_font(
        FontProperties(
            font_name="Calibri",
            bold=True,
            size_in_points=12.0,
            color=Color.from_opaque_rgb("FFFFFF"),
        )
    )
    fill = workbook.append_pattern_fill(
        PatternFillSpec.solid_from_theme(ColorSchemeSlot.Accent1)
    )
    border = workbook.append_border(
        BorderSpec(bottom=BorderEdgeSpec(style=BorderStyle.Medium))
    )
    heading = workbook.append_cell_format(
        CellFormatTarget.CellFormats,
        CellFormatSpec.skeleton_cell_format().with_resources(
            font_index=font, fill_index=fill, border_index=border
        ),
    )
    for heading_cell in ("A1", "B1", "C1"):
        workbook.set_cell_style(0, heading_cell, heading)

    # ---- Geometry -----------------------------------------------------------------------------------
    workbook.set_row_height(0, 1, 22.0)
    workbook.set_column_width(0, 0, 0, 22.0)
    workbook.set_column_width(0, 1, 2, 14.0)

    # ---- A hyperlink --------------------------------------------------------------------------------
    workbook.set_cell_hyperlink_url(0, "A2", "https://example.org/north-america")

    # ---- Save -----------------------------------------------------------------------------------------
    workbook.validate()
    return workbook.save()


def test_the_excel_walkthrough_runs_end_to_end_through_the_python_binding(
    output_directory: pathlib.Path,
) -> None:
    saved = build_the_guides_workbook()
    assert len(saved) > 0
    (output_directory / OUTPUT_NAME).write_bytes(saved)

    reopened = Workbook.open(saved)
    assert reopened.sheet_count() == 2
    assert reopened.sheets()[0].name == "Summary"
    assert reopened.used_range(0) == "A1:C5"

    # One read, every cell — the mirror image of the one write above.
    block = reopened.read_range(0, "A1:C5")
    assert block.row_count == 5
    assert block.column_count == 3
    assert block.value(0, 0).text == "Region"
    assert block.value(1, 0).text == "North America"
    assert block.value(1, 1).number == 1_250_000.0
    assert block.value(3, 2).number == 0.198
    assert block.value(4, 1).boolean is False

    # And the whole block as native Python values, which is the shape to reach for.
    rows = block.rows()
    assert rows[0] == ["Region", "Revenue", "Growth"]
    assert rows[1] == ["North America", 1_250_000.0, 0.125]
    assert block.kinds()[4] == ["text", "boolean", "blank"]

    assert (
        reopened.read_range(1, "A1").value(0, 0).text
        == "Figures are unaudited and subject to revision."
    )
    link = reopened.cell_hyperlink(0, "A2")
    assert link is not None and link.target == "https://example.org/north-america"
    heading = reopened.effective_cell_format(0, "B1")
    assert heading is not None and heading.style_index > 0


def test_workbook_open_refuses_a_presentation_and_a_document_by_name(
    fixtures: pathlib.Path,
) -> None:
    """Detection works before editing does, so each refusal names the format."""
    presentation = (fixtures / "sample.pptx").read_bytes()
    assert mjx_ooxml.detect_format(presentation) == Format.Presentation
    with pytest.raises(mjx_ooxml.UnsupportedFormatError) as refusal:
        Workbook.open(presentation)
    assert refusal.value.code == "UnsupportedFormat"
    assert isinstance(refusal.value, OoxmlError)

    document = (fixtures / "sample.docx").read_bytes()
    with pytest.raises(mjx_ooxml.UnsupportedFormatError):
        Workbook.open(document)


def test_a_workbook_opened_and_saved_untouched_is_byte_identical_part_by_part(
    fixtures: pathlib.Path,
) -> None:
    """The fidelity contract, through the binding.

    A binding that quietly re-serialized a part it never touched would still produce a file Excel
    opens, and every other assertion in this file would still pass. This is the one that would not.
    """
    original = (fixtures / "sample.xlsx").read_bytes()
    workbook = Workbook.open(original)
    saved = workbook.save()

    before = _part_payloads(original)
    after = _part_payloads(saved)
    assert sorted(before) == sorted(after), "the part set must survive a round trip"
    differing = [name for name in before if before[name] != after[name]]
    assert not differing, f"these parts changed on an untouched round trip: {differing}"


def _part_payloads(archive: bytes) -> dict[str, bytes]:
    """Every part of a package, by name, decompressed."""
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        return {entry.filename: package.read(entry.filename) for entry in package.infolist()}


@pytest.mark.skipif(
    subprocess.run(["cargo", "--version"], capture_output=True).returncode != 0,
    reason="cargo is not on PATH, so the Rust walkthrough cannot be run to compare against",
)
def test_the_three_excel_walkthroughs_agree(output_directory: pathlib.Path) -> None:
    """This walkthrough and the Rust one produce the *same workbook*, part for part.

    Not "both produce a file": the same part names, and byte-identical payloads for every one of
    them. A method wired to the wrong `Workbook` method, or a cell written to the wrong address,
    changes a payload here and nothing else would notice.

    The Node walkthrough is compared against the same reference by
    `bindings/mjx-wasm/tests/node/build_a_workbook.mjs`, which writes its workbook beside these two.
    """
    from_python = build_the_guides_workbook()

    rust_output = output_directory / "facade_build_a_workbook.xlsx"
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mjx-ooxml",
            "--example",
            "build_a_workbook",
            "--",
            str(rust_output),
        ],
        cwd=str(pathlib.Path(__file__).resolve().parents[3]),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

    python_parts = _part_payloads(from_python)
    rust_parts = _part_payloads(rust_output.read_bytes())

    assert sorted(python_parts) == sorted(rust_parts), (
        "the two walkthroughs must author the same set of parts"
    )
    differing = [name for name in python_parts if python_parts[name] != rust_parts[name]]
    assert not differing, f"these parts differ between the Python and Rust walkthroughs: {differing}"
