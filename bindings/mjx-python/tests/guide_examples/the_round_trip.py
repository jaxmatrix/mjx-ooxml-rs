"""The guide's **The round trip** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against `crates/mjx-ooxml/examples/guide_the_round_trip.rs` part
by part.

Unlike `saving_validates`, **this example authors nothing.** Every part of the package it saves was
written by whoever produced `tests/fixtures/sample.xlsx`, so the assertions below are the round-trip
contract — copy-on-write and verbatim re-emission — rather than a check that this library agrees
with itself. Reading the file is above the sentinel because it is the caller's job: the library is
bytes in and bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
original = (_FIXTURES / "sample.xlsx").read_bytes()

# guide-example:start
from mjx_ooxml import Workbook

# `original` is the file's bytes. Reading them is the caller's job in every one of the three
# languages: this library is bytes in and bytes out and never touches a filesystem.
saved = Workbook.open(original).save()

# Nothing was edited, so every part comes back byte for byte. That is the contract, and it is
# `mjx_opc`'s copy-on-write part graph that keeps it rather than anything this facade does.
before = Workbook.open(original)
after = Workbook.open(saved)
assert before.part_names() == after.part_names()
for part in before.part_names():
    was = before.part_bytes(part)
    now = after.part_bytes(part)
    assert was == now, f"{part} changed"
# guide-example:end
