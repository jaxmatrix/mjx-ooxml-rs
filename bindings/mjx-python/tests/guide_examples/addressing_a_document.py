"""The guide's **`Document`: a block and a run** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/addressing.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. `saved` is the package it produced,
which is what the harness compares against
`crates/mjx-ooxml/examples/guide_addressing_a_document.rs` part by part — so a run appended to the
wrong paragraph changes `word/document.xml` and fails there.
"""

# guide-example:start
from mjx_ooxml import Document, PageSize

document = Document.blank(PageSize.a4())
document.append_paragraph()
document.append_run(0, "Quarterly ")
document.append_run(0, "results")
assert document.run_count(0) == 2
assert document.run_text(0, 1) == "results"
assert document.paragraph_text(0) == "Quarterly results"

saved = document.save()
# guide-example:end
