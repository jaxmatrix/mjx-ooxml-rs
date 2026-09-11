"""The guide's **Opening detects first, then parses once** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — it
refuses to open something — so there is nothing for the harness to compare and it skips that half by
construction.

**The eleven `ErrorCode` values are eleven exception classes here**, all of them subclasses of
`OoxmlError`, and each still carries the code's stable spelling on `.code`. The guide says so above
the blocks.

Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
workbook_bytes = (_FIXTURES / "sample.xlsx").read_bytes()

# guide-example:start
from mjx_ooxml import Deck, UnsupportedFormatError

# `workbook_bytes` is a spreadsheet, and `Deck.open` detects that before it parses anything.
try:
    Deck.open(workbook_bytes)
    raise AssertionError("a workbook is not a deck")
except UnsupportedFormatError as failure:
    assert failure.code == "UnsupportedFormat"

    # The message names the constructor that would have worked, rather than complaining about a
    # `presentation.xml` that was never there.
    assert "Workbook" in str(failure), failure
# guide-example:end
