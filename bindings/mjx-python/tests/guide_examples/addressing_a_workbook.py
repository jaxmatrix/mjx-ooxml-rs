"""The guide's **`Workbook`: a tab index and A1 text** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/addressing.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against `crates/mjx-ooxml/examples/guide_addressing_a_workbook.rs`
part by part.

**There is no `CellInput` here.** An enumeration carrying a payload has no projection, so
`CellWrite` has one static constructor per kind instead — the same information with the variant
folded into the function name. The guide says so above the blocks.
"""

# guide-example:start
from mjx_ooxml import CellWrite, Workbook

workbook = Workbook.blank()
workbook.write_cells(
    0,
    [
        CellWrite.shared_text("A1", "Region"),
        CellWrite.number("B1", 12.5),
        # The anchoring is data, not address: `$B$2` and `B2` spell one cell.
        CellWrite.number("$B$2", 18.0),
    ],
)

# A block is row-major over the whole requested rectangle, blanks included, and its two
# arguments are offsets *into the block* rather than sheet coordinates.
block = workbook.read_range(0, "A1:B2")
assert block.first_row == 0, "A1 is row 0, column 0"
assert block.first_column == 0
assert block.value(0, 0).text == "Region"
assert block.value(1, 1).number == 18.0
assert block.range == "A1:B2"
assert workbook.used_range(0) == "A1:B2"

saved = workbook.save()
# guide-example:end
