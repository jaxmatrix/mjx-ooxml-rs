"""Every guide example, run here and compared against the Rust example of the same name.

The examples themselves live in `guide_examples/`, one module each, and the code between their
`guide-example` sentinels is what `crates/mjx-ooxml/docs/guide/` shows as its `python` block —
literally, because `cargo run -p xtask -- guide-examples` copies it there and
`xtask/tests/guide_examples.rs` proves the copy is current.

**Importing a module is running the example.** Their code is top level, so every assertion a reader
sees in the guide executes the moment this file imports it. A module that produced a package binds
`saved`, and that is what the comparison below reads.

The population comes from the directory, never from a list here: an example added to
`guide_examples/` joins these tests with no edit to this file, which is the same rule
`xtask/tests/derived_rosters.rs` states for the workspace at large.
"""

from __future__ import annotations

import importlib
import pathlib
import subprocess
import types

import pytest

from opc import part_payloads

HERE = pathlib.Path(__file__).resolve().parent
REPOSITORY_ROOT = HERE.parents[2]
EXAMPLES = HERE / "guide_examples"

#: What the Rust half of an example is called under `crates/mjx-ooxml/examples/`.
RUST_EXAMPLE_PREFIX = "guide_"

#: The extension the Rust half's output is written under. The format is the example's own business
#: — one may author a deck and the next a workbook — and this comparison is over part payloads, so
#: the harness does not pretend to know which.
PACKAGE_SUFFIX = ".pkg"

_CARGO_IS_ON_PATH = subprocess.run(["cargo", "--version"], capture_output=True).returncode == 0


def guide_example_names() -> list[str]:
    """The examples, derived from `guide_examples/` rather than listed."""
    names = sorted(path.stem for path in EXAMPLES.glob("*.py") if not path.name.startswith("_"))
    assert names, f"no guide example under {EXAMPLES} — the walk has stopped matching"
    return names


def _run(name: str) -> types.ModuleType:
    """Import — that is, run — one guide example."""
    return importlib.import_module(f"guide_examples.{name}")


@pytest.mark.parametrize("name", guide_example_names())
def test_the_guide_example_runs(name: str) -> None:
    """The block a reader sees executes here, assertions and all.

    There is nothing to assert beyond the import: the example's own assertions are the test, and an
    import that returns has run all of them.
    """
    assert _run(name).__name__.endswith(name)


@pytest.mark.skipif(
    not _CARGO_IS_ON_PATH,
    reason="cargo is not on PATH, so the Rust half cannot be run to compare against",
)
@pytest.mark.parametrize("name", guide_example_names())
def test_the_guide_example_and_the_rust_one_agree(
    name: str, output_directory: pathlib.Path
) -> None:
    """This half and the Rust half produce the *same package*, part for part.

    Not "both produce a file", and not "both produce a file of about the right size": the same part
    names, and byte-identical payloads for every one of them. That is the only assertion that can
    tell a faithful projection from a plausible one — a method wired to the wrong `Deck` method, or
    an argument converted with the wrong units, changes a payload here and nothing else would
    notice.

    `bindings/mjx-wasm/tests/node/guide_examples.mjs` compares the JavaScript half against the same
    reference.
    """
    saved: object = getattr(_run(name), "saved", None)
    if saved is None:
        pytest.skip(f"the {name} example produces no package, so there is nothing to compare")
    assert isinstance(saved, bytes)

    rust_output = output_directory / f"facade_{RUST_EXAMPLE_PREFIX}{name}{PACKAGE_SUFFIX}"
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mjx-ooxml",
            "--example",
            f"{RUST_EXAMPLE_PREFIX}{name}",
            "--",
            str(rust_output),
        ],
        cwd=str(REPOSITORY_ROOT),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

    python_parts = part_payloads(saved)
    rust_parts = part_payloads(rust_output.read_bytes())

    assert sorted(python_parts) == sorted(rust_parts), (
        "the two halves must author the same set of parts"
    )
    differing = [name for name in python_parts if python_parts[name] != rust_parts[name]]
    assert not differing, f"these parts differ between the Python and Rust halves: {differing}"
