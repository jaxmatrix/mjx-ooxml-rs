"""Reading an OPC package's parts, once, for every suite in this directory that compares two.

The round-trip contract this repository states is **per-part decompressed-payload identity plus
structural container identity** — never identical archive bytes. So every comparison here is over
the payloads this module returns, and never over the two archives directly: two encoders that
agree about every part still disagree about compression levels and about the order of the central
directory, and neither difference means anything.

`bindings/mjx-wasm/tests/node/zip.mjs` is the Node half of this, and it has to parse the archive by
hand because Node ships no archive reader. Python ships `zipfile`, so this half is four lines — but
it is *one* four lines. It used to be three copies, one per suite, which is the shape MJXOFF-239
found the Word walkthrough hiding in: a helper copied per file is a comparison that can be omitted
from a file without anything noticing.
"""

from __future__ import annotations

import io
import zipfile


def part_payloads(archive: bytes) -> dict[str, bytes]:
    """Every part of a package, by name, decompressed."""
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        return {entry.filename: package.read(entry.filename) for entry in package.infolist()}
