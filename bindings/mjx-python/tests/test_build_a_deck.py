"""The building-a-deck guide, written through the Python binding.

This is `crates/mjx-ooxml/examples/build_a_deck.rs` call for call — the same fixture, the same
layouts, the same shapes, the same table, the same chart, the same notes, the same assertions. That
is the point: if the curated subset were missing anything, this file could not be written, and if
the identity mapping were not identity, it would not read the same.

`tests/node/build_a_deck.mjs` is the third copy. All three write to `target/examples/`, and
`test_the_three_walkthroughs_agree` checks that this one and the Rust one produce the same deck.
"""

from __future__ import annotations

import dataclasses
import io
import pathlib
import subprocess
import zipfile

import pytest

import mjx_ooxml
from mjx_ooxml import (
    CellFormat,
    Cells,
    CharacterPropertiesSpec,
    ChartData,
    ChartKind,
    ColorSpec,
    Deck,
    FillSpec,
    Format,
    LineSpec,
    LineWidth,
    OoxmlError,
    PlaceholderType,
    PresetShapeType,
    ShapeBounds,
)

OUTPUT_NAME = "python_build_a_deck.pptx"


@dataclasses.dataclass
class Walkthrough:
    """What the walkthrough noticed on its way through, so the assertions can name it."""

    saved: bytes
    template_counts: tuple[int, int, int]
    layouts: list[tuple[int, str | None]]
    placeholders: list[tuple[int, PlaceholderType]]
    title_size_points: float | None
    external_links: int
    swept_parts: int
    slide: int
    badge: int
    caption: int
    table_slide: int
    table: int
    chart_slide: int
    chart_frame: int
    slide_count: int


def build_the_guides_deck(template: bytes) -> Walkthrough:
    """The walkthrough, run once. Everything it learns comes back in a `Walkthrough`."""

    # ---- What is this file? ------------------------------------------------------------------
    # Detection reads the package, not the name: a `.pptm` or a `.potx` answers correctly, and a
    # `.docx` renamed to `.pptx` is still reported as a Word document.
    detected = mjx_ooxml.detect_format(template)
    assert detected == Format.Presentation
    assert detected.conventional_extension == "pptx"
    assert detected.is_editable

    # ---- Open --------------------------------------------------------------------------------
    deck = Deck.open(template)

    # ---- Look before editing -------------------------------------------------------------------
    template_counts = (deck.slide_count(), deck.layout_count(), deck.master_count())
    layouts = [(layout.index, layout.name) for layout in deck.layouts()]

    # ---- A slide from a layout, and its placeholders --------------------------------------------
    # Indices are plain ints here, and a surface or a shape address takes one directly.
    slide = deck.add_slide_from_layout(1)
    placeholders = [
        (shape.index, shape.placeholder.kind)
        for shape in deck.shapes(slide)
        if shape.placeholder is not None
    ]
    deck.set_shape_text_content(slide, 0, "Quarterly results")
    deck.set_shape_text_content(slide, 1, "Revenue up 14% year on year")

    # Nothing above set a font or a size: the title renders at the master's title size, in the
    # theme's major typeface, because that is what the layout and master say.
    title = deck.effective_run_properties(slide, 0, 0, 0)
    title_size_points = title.size_points

    # ---- Shapes of our own -----------------------------------------------------------------------
    badge = deck.add_shape(
        slide, PresetShapeType.Ellipse, ShapeBounds.from_inches(8.0, 0.4, 1.2, 1.2)
    )
    deck.set_shape_fill(slide, badge, FillSpec.solid(ColorSpec.srgb("1F3864")))
    deck.set_shape_outline(
        slide,
        badge,
        LineSpec.solid(LineWidth.from_points(1.5), ColorSpec.srgb("FFFFFF")),
    )

    caption = deck.add_text_box(
        slide, "Source: internal", ShapeBounds.from_inches(0.5, 6.5, 4.0, 0.4)
    )
    deck.set_shape_run_properties(
        slide,
        caption,
        CharacterPropertiesSpec().with_size_points(10.0).with_italic(True),
    )

    # ---- A picture ---------------------------------------------------------------------------------
    deck.add_picture(
        slide,
        mjx_ooxml.DEFAULT_PLACEHOLDER_IMAGE,
        ShapeBounds.from_inches(7.5, 5.5, 1.5, 1.5),
    )

    # ---- A table -----------------------------------------------------------------------------------
    table_slide = deck.add_slide_from_layout(1)
    deck.set_shape_text_content(table_slide, 0, "By region")
    table = deck.add_table(table_slide, 3, 2, ShapeBounds.from_inches(1.0, 2.0, 6.0, 2.0))
    for offset, (region, revenue) in enumerate([("North", "4.2"), ("South", "3.1")]):
        row = offset + 1
        deck.set_cell_text(table_slide, table, row, 0, 0, region)
        deck.set_cell_text(table_slide, table, row, 1, 0, revenue)
    deck.set_cell_text(table_slide, table, 0, 0, 0, "Region")
    deck.set_cell_text(table_slide, table, 0, 1, 0, "Revenue")

    # One call for the whole header row — `Cells` names the selection, `CellFormat` the change.
    deck.format_cells(
        table_slide,
        table,
        Cells.row(0),
        CellFormat().with_fill(FillSpec.solid(ColorSpec.srgb("1F3864"))),
    )
    deck.format_cell_text(
        table_slide,
        table,
        Cells.row(0),
        CharacterPropertiesSpec().with_bold(True).with_color(ColorSpec.srgb("FFFFFF")),
    )

    # ---- A chart -----------------------------------------------------------------------------------
    chart_slide = deck.add_slide_from_layout(1)
    deck.set_shape_text_content(chart_slide, 0, "Trend")
    chart = (
        ChartData(ChartKind.Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25])
    )
    chart_frame = deck.add_chart(chart_slide, chart, ShapeBounds.from_inches(1.0, 2.0, 8.0, 4.0))

    # ---- Speaker notes -----------------------------------------------------------------------------
    deck.set_notes_text(slide, "Lead with the revenue number, then the regional split.")

    # ---- Package hygiene ---------------------------------------------------------------------------
    # The three delegates that reach the package without handing out the part graph.
    external_links = len(deck.external_links())
    swept_parts = len(deck.remove_unused_parts())

    # ---- Save --------------------------------------------------------------------------------------
    # `save` validates first, exactly as the Rust `Deck.save` does; the binding does not route
    # around that check. `validate` is the same pass without writing.
    deck.validate()

    return Walkthrough(
        saved=deck.save(),
        template_counts=template_counts,
        layouts=layouts,
        placeholders=placeholders,
        title_size_points=title_size_points,
        external_links=external_links,
        swept_parts=swept_parts,
        slide=slide,
        badge=badge,
        caption=caption,
        table_slide=table_slide,
        table=table,
        chart_slide=chart_slide,
        chart_frame=chart_frame,
        slide_count=deck.slide_count(),
    )


def test_the_guide_builds_and_reopens(template: bytes, output_directory: pathlib.Path) -> None:
    """The walkthrough runs, and the deck it writes says what it was told to say."""
    run = build_the_guides_deck(template)
    (output_directory / OUTPUT_NAME).write_bytes(run.saved)

    assert run.template_counts == (2, 3, 1), "the fixture the guide starts from"
    assert run.placeholders, "layout 1 places a title and a body"
    assert run.title_size_points is not None, (
        "the title inherits a size from the master even though this deck never set one"
    )
    assert run.slide_count == 5, "two from the fixture, three the guide adds"
    assert run.layouts, "the template names its layouts"
    assert run.external_links == 0 and run.swept_parts >= 0

    # ---- Reopen what we wrote ------------------------------------------------------------------
    # A walkthrough that never checks its own output is a claim, not a demonstration.
    reopened = Deck.open(run.saved)
    assert reopened.slide_count() == run.slide_count
    assert reopened.format() == Format.Presentation
    assert reopened.shape_text(run.slide, 0) == "Quarterly results"
    assert reopened.shape_text(run.slide, run.caption) == "Source: internal"
    assert reopened.cell_text(run.table_slide, run.table, 0, 0) == "Region"
    assert reopened.cell_text(run.table_slide, run.table, 1, 1) == "4.2"
    assert reopened.notes_text(run.slide) is not None

    # The badge really is an ellipse with the fill and outline the guide gave it.
    fill = reopened.shape_fill(run.slide, run.badge)
    assert fill is not None and fill.kind == "solid"
    assert fill.color is not None and fill.color.srgb_value == "1F3864"
    outline = reopened.shape_outline(run.slide, run.badge)
    assert outline is not None and outline.width is not None
    assert outline.width.points == pytest.approx(1.5)

    # And the chart is a chart, with the series the guide gave it.
    series = reopened.chart_series(run.chart_slide, run.chart_frame)
    assert [entry.name for entry in series] == ["2026"]
    assert series[0].values == [12.0, 15.5, 14.0, 19.25]


def test_a_word_document_is_detected_and_refused(word_document: bytes) -> None:
    """Detection works before editing does, so the refusal names the format."""
    assert mjx_ooxml.detect_format(word_document) == Format.Document
    with pytest.raises(mjx_ooxml.UnsupportedFormatError) as refusal:
        Deck.open(word_document)
    assert refusal.value.code == "UnsupportedFormat"
    assert isinstance(refusal.value, OoxmlError)


def _part_payloads(archive: bytes) -> dict[str, bytes]:
    """Every part of a package, by name, decompressed."""
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        return {entry.filename: package.read(entry.filename) for entry in package.infolist()}


@pytest.mark.skipif(
    subprocess.run(["cargo", "--version"], capture_output=True).returncode != 0,
    reason="cargo is not on PATH, so the Rust walkthrough cannot be run to compare against",
)
def test_the_three_walkthroughs_agree(
    template: bytes, output_directory: pathlib.Path
) -> None:
    """This walkthrough and the Rust one produce the *same deck*, part for part.

    Not "both produce a file", and not "both produce a file of about the right size": the same part
    names, and byte-identical payloads for every one of them. That is the only assertion that can
    tell a faithful binding from a plausible one — a method wired to the wrong `Deck` method, or an
    argument converted with the wrong units, changes a payload here and nothing else would notice.

    The Node walkthrough is compared against the same reference by
    `bindings/mjx-wasm/tests/node/build_a_deck.mjs`, which writes its deck beside these two.
    """
    from_python = build_the_guides_deck(template).saved

    rust_output = output_directory / "facade_build_a_deck.pptx"
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mjx-ooxml",
            "--example",
            "build_a_deck",
            "--",
            str(rust_output),
        ],
        cwd=str(pathlib.Path(__file__).resolve().parents[3]),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

    from_rust = rust_output.read_bytes()
    python_parts = _part_payloads(from_python)
    rust_parts = _part_payloads(from_rust)

    assert sorted(python_parts) == sorted(rust_parts), (
        "the two walkthroughs must author the same set of parts"
    )
    differing = [name for name in python_parts if python_parts[name] != rust_parts[name]]
    assert not differing, f"these parts differ between the Python and Rust walkthroughs: {differing}"
