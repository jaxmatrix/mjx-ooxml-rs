"""Reading an OPC package's parts, once, for every suite in this directory that compares two.

The round-trip contract this repository states is **per-part decompressed-payload identity plus
structural container identity** — never identical archive bytes. So every comparison here is over
the payloads this module returns, and never over the two archives directly: two encoders that
agree about every part still disagree about compression levels and about the order of the central
directory, and neither difference means anything.

`bindings/mjx-wasm/tests/node/zip.mjs` is the Node half of this, and it has to parse the archive by
hand because Node ships no archive reader. Python ships `zipfile`, so this half is a handful of
lines — but it is *one* handful. It used to be three copies, one per suite, which is the shape
MJXOFF-239 found the Word walkthrough hiding in: a helper copied per file is a comparison that can
be omitted from a file without anything noticing.

`zipfile` is named **here and nowhere else** in this directory, which
`xtask/tests/walkthrough_triples.rs` enforces — so the one suite that needs to *author* a package
rather than read one asks for it here too, and there is still exactly one place that knows what an
archive is.
"""

from __future__ import annotations

import io
import zipfile


def part_payloads(archive: bytes) -> dict[str, bytes]:
    """Every part of a package, by name, decompressed."""
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        return {entry.filename: package.read(entry.filename) for entry in package.infolist()}


def with_part_replaced(archive: bytes, part: str, payload: bytes) -> bytes:
    """The same package, with one part's bytes replaced and every other entry carried over.

    For an input no call in this binding will produce: markup that states a token its own schema
    refuses. `crates/mjx-ooxml/tests/workbook_unreadable_values.rs` builds its copy the same way, one
    tier down, through `mjx_opc`.
    """
    entry = part.removeprefix("/")
    rebuilt = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(archive)) as package, zipfile.ZipFile(rebuilt, "w") as out:
        for item in package.infolist():
            bytes_for = payload if item.filename == entry else package.read(item.filename)
            out.writestr(item.filename, bytes_for)
    return rebuilt.getvalue()
