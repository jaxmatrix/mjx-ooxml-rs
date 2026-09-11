"""The guide's **Bytes in, bytes out** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/README.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against
`crates/mjx-ooxml/examples/guide_open_edit_and_save_any_format.rs` part by part.

**A `FormatFamily` is a member of a projected enumeration here, reached through the `family`
attribute** rather than through a method — and the dispatch is an `if` chain rather than a `match`.
The guide says so above the blocks.

Reading and writing the file are the caller's job, so both are outside the block: the library is
bytes in and bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
data = (_FIXTURES / "sample.pptx").read_bytes()

# guide-example:start
from mjx_ooxml import Deck, Document, FormatFamily, Workbook, detect_format

# `data` is whatever the caller read. Which of the three surfaces opens it is the package's
# answer, not the filename's.
family = detect_format(data).family
if family == FormatFamily.Presentation:
    saved = Deck.open(data).save()
elif family == FormatFamily.WordProcessing:
    saved = Document.open(data).save()
elif family == FormatFamily.Spreadsheet:
    saved = Workbook.open(data).save()
else:
    # A fourth family would be another branch here, not a broken program.
    raise ValueError(f"unhandled family {family}")
assert saved
# guide-example:end
