"""Every guide example, run here and compared against the Rust example of the same name.

The examples themselves live in `guide_examples/`, one module each, and the code between their
`guide-example` sentinels is what `crates/mjx-ooxml/docs/guide/` shows as its `python` block —
literally, because `cargo run -p xtask -- guide-examples` copies it there and
`xtask/tests/guide_examples.rs` proves the copy is current.

**Importing a module is running the example.** Their code is top level, so every assertion a reader
sees in the guide executes the moment this file imports it.

*Which* packages a module offers is read from the example's Rust half, which declares them on one
`guide-example:packages` line (MJXOFF-262). That is a statement rather than an inference: this
harness used to decide by looking for a binding named `saved`, so an example whose `saved` binding
had been deleted was indistinguishable from one of the seven that genuinely produce none, and more
than a third of the corpus took a skip path nobody read. There is no skip left here — an example
that declares `none` is *asserted* to bind nothing, and one that declares packages has every one of
them compared (MJXOFF-260).

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

#: The sentinel a Rust half declares its packages with, and the word for "none of them".
PACKAGES_DECLARATION = "guide-example:packages"
NO_PACKAGES_TOKEN = "none"

#: The prefix every package binding's name carries. `saved` alone is the single-package spelling.
PACKAGE_BINDING_PREFIX = "saved"


def declared_packages(name: str) -> list[str]:
    """The packages an example offers, as its Rust half declares them.

    Read out of `crates/mjx-ooxml/examples/guide_<name>.rs` rather than inferred from this module,
    because the point of the declaration is that a half which stopped binding one fails against the
    statement instead of silently agreeing with itself. `xtask/tests/guide_examples.rs` is what
    holds all three halves to the same line.
    """
    source = (
        REPOSITORY_ROOT / "crates" / "mjx-ooxml" / "examples" / f"{RUST_EXAMPLE_PREFIX}{name}.rs"
    ).read_text()
    lines = [line for line in source.splitlines() if PACKAGES_DECLARATION in line]
    assert len(lines) == 1, (
        f"{name}: its Rust half carries {len(lines)} `{PACKAGES_DECLARATION}` lines, and every "
        f"example states its packages exactly once"
    )
    words = lines[0].split(PACKAGES_DECLARATION, 1)[1].split()
    assert words, f"{name}: an empty `{PACKAGES_DECLARATION}` declaration"
    if words == [NO_PACKAGES_TOKEN]:
        return []
    for word in words:
        assert word.startswith(PACKAGE_BINDING_PREFIX), (
            f"{name}: {word!r} is not a package binding"
        )
    return words


def package_output_path(base: pathlib.Path, binding: str) -> pathlib.Path:
    """Where the Rust half wrote one of its packages.

    The first declared binding takes the path itself; a later one has its suffix inserted before
    the extension. The same rule lives in `xtask::guide_examples::package_output_path` and is
    restated inside each two-package Rust half, which cannot depend on `xtask`.
    """
    suffix = binding[len(PACKAGE_BINDING_PREFIX) :].lstrip("_")
    if not suffix:
        return base
    return base.with_name(f"{base.stem}.{suffix}{base.suffix}")

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


@pytest.mark.parametrize("name", guide_example_names())
def test_an_example_that_declares_no_package_produces_none(name: str) -> None:
    """The declared-`none` case is a verdict, not a skip.

    Seven of the examples are about a refusal the library reports and have nothing to save, and
    making them save something to satisfy a harness would be a worse example compared over bytes it
    is not about. So the fact is asserted from the other side: an example that says it produces
    none must produce none, and this half binding a package it never declared is a failure here.
    """
    declared = declared_packages(name)
    module = _run(name)
    bound = sorted(
        attribute
        for attribute in vars(module)
        if attribute == PACKAGE_BINDING_PREFIX
        or attribute.startswith(f"{PACKAGE_BINDING_PREFIX}_")
    )
    assert bound == sorted(declared), (
        f"the {name} example declares {sorted(declared)} and this half binds {bound}"
    )


@pytest.mark.skipif(
    not _CARGO_IS_ON_PATH,
    reason="cargo is not on PATH, so the Rust half cannot be run to compare against",
)
@pytest.mark.parametrize("name", guide_example_names())
def test_the_guide_example_and_the_rust_one_agree(
    name: str, output_directory: pathlib.Path
) -> None:
    """This half and the Rust half produce the *same packages*, part for part.

    Not "both produce a file", and not "both produce a file of about the right size": the same part
    names, and byte-identical payloads for every one of them. That is the only assertion that can
    tell a faithful projection from a plausible one — a method wired to the wrong `Deck` method, or
    an argument converted with the wrong units, changes a payload here and nothing else would
    notice.

    Every package the example declares is compared, not just the first: two examples author two
    packages each, and until MJXOFF-260 the second one's bytes were compared by nothing.

    `bindings/mjx-wasm/tests/node/guide_examples.mjs` compares the JavaScript half against the same
    reference.
    """
    declared = declared_packages(name)
    if not declared:
        pytest.skip(
            f"the {name} example declares it produces no package; "
            f"test_an_example_that_declares_no_package_produces_none is what checks that"
        )
    module = _run(name)

    base = output_directory / f"facade_{RUST_EXAMPLE_PREFIX}{name}{PACKAGE_SUFFIX}"
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
            str(base),
        ],
        cwd=str(REPOSITORY_ROOT),
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr

    for binding in declared:
        saved: object = getattr(module, binding, None)
        assert isinstance(saved, bytes), (
            f"the {name} example declares `{binding}` and this half does not bind it"
        )
        rust_output = package_output_path(base, binding)
        python_parts = part_payloads(saved)
        rust_parts = part_payloads(rust_output.read_bytes())

        assert sorted(python_parts) == sorted(rust_parts), (
            f"`{binding}`: the two halves must author the same set of parts"
        )
        differing = [part for part in python_parts if python_parts[part] != rust_parts[part]]
        assert not differing, (
            f"`{binding}`: these parts differ between the Python and Rust halves: {differing}"
        )
