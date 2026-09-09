"""The guide's **Detection reads the package, not the filename** example, through the Python binding.

Everything between the two `guide-example` sentinels is what
`crates/mjx-ooxml/docs/guide/opening_and_saving.md` shows as its `python` block — literally, because
`cargo run -p xtask -- guide-examples` copies it there and `xtask/tests/guide_examples.rs` proves the
copy is current.

Importing this module *is* running the example: the code below is top level, so `pytest` collecting
`tests/test_guide_examples.py` executes every assertion in it. This one binds no `saved` — it only
reads a format out of bytes — so there is nothing for the harness to compare and it skips that half
by construction.

**A `Format`'s five accessors are attributes here and methods in Rust.** The guide says so above the
blocks; the difference is the projection's, not this file's.

Reading the file is above the sentinel because it is the caller's job: the library is bytes in and
bytes out and never touches a filesystem.
"""

import pathlib

_FIXTURES = pathlib.Path(__file__).resolve().parents[4] / "tests" / "fixtures"
data = (_FIXTURES / "sample.docx").read_bytes()

# guide-example:start
from mjx_ooxml import Format, FormatFamily, detect_format

# `data` is a Word document. Nothing here looks at a filename: detection opens the container,
# follows the root `officeDocument` relationship and reads the content type it lands on.
detected = detect_format(data)
assert detected == Format.Document
assert detected.family == FormatFamily.WordProcessing
assert detected.conventional_extension == "docx"
assert detected.is_editable
assert not detected.is_macro_enabled
# guide-example:end
