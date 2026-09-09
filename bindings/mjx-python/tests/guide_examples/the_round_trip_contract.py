"""The guide's **The contract** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against
`crates/mjx-ooxml/examples/guide_the_round_trip_contract.rs` part by part.

**This is the project's central claim, asserted inside the block a reader sees.** Not "three
languages agree with each other" — each half is checked against the input file, so three languages
agreeing on a wrong answer would not pass. The harness comparison on top is a second, different
fact: that all three preserved it the same way.

Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
original = (_FIXTURES / "sample.xlsx").read_bytes()

# guide-example:start
from mjx_ooxml import Workbook

# One edit: the first tab's name, which `xl/workbook.xml` states and no other part does.
workbook = Workbook.open(original)
workbook.rename_sheet(0, "Revised")
saved = workbook.save()

before = Workbook.open(original)
after = Workbook.open(saved)
assert before.part_names() == after.part_names(), "no part appeared"

changed = [
    part for part in before.part_names() if before.part_bytes(part) != after.part_bytes(part)
]
# Everything else came back byte for byte — the whole of the contract.
assert changed == ["/xl/workbook.xml"]
# guide-example:end
