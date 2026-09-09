"""The guide's **Eleven codes, and what each one means you should do** example, through Python.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/errors.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — nothing
was written, which is its last assertion — so there is nothing for the harness to compare and it
skips that half by construction.

**One exception class per code, and no `CellInput`.** Both differences are the projection's, and
both are stated in the guide above the blocks.
"""

# guide-example:start
from mjx_ooxml import CellWrite, IndexOutOfRangeError, InvalidArgumentError, Workbook

workbook = Workbook.blank()

# An address that does not parse: refused before the worksheet is even opened.
try:
    workbook.write_cells(0, [CellWrite.number("not-a-cell", 1.0)])
    raise AssertionError("`not-a-cell` is not an A1 reference")
except InvalidArgumentError as failure:
    assert failure.code == "InvalidArgument"

# A value SpreadsheetML has no spelling for.
try:
    workbook.write_cells(0, [CellWrite.number("A1", float("nan"))])
    raise AssertionError("SpreadsheetML cannot spell NaN")
except InvalidArgumentError as failure:
    assert failure.code == "InvalidArgument"

# A tab that is not there.
try:
    workbook.read_range(9, "A1")
    raise AssertionError("there is one sheet")
except IndexOutOfRangeError as failure:
    assert failure.index == 9

# And the contract worth knowing: a batch that would fail halfway is refused before the
# package is touched at all, so nothing above wrote anything.
assert workbook.read_sheet(0).is_empty
# guide-example:end
