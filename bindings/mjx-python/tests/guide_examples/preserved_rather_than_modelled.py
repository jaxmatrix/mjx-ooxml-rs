"""The guide's **What is preserved rather than modelled** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — it only
reads, which is the point — so there is nothing for the harness to compare and it skips that half by
construction.

Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
original = (_FIXTURES / "preserved_parts.xlsx").read_bytes()

# guide-example:start
from mjx_ooxml import Workbook

workbook = Workbook.open(original)

# Inventoried from the relationships, with no markup parsed at all.
summary = workbook.preserved_parts()
assert summary.pivot_tables
assert summary.pivot_cache_definitions

# And resolved far enough to say which tab each one sits on.
for table in workbook.pivot_tables():
    assert table.sheet_name
# guide-example:end
