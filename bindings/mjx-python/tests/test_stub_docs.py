"""The committed stub's prose is the compiled module's prose (MJXOFF-234).

`test_stub_parity.py` holds the stub to the module over *symbols*, in both directions. Beside each
symbol sits a sentence, and until MJXOFF-234 nothing held those to anything: the stub's docstrings
were a hand-written second copy of the `///` doc comments they restate, and MJXOFF-226 found the two
disagreeing about the same item and differently wrong in each.

**The second copy is gone rather than compared.** `tools/stub_docs.py` writes the stub's docstrings
out of the compiled module — PyO3 carries each `///` comment verbatim into `__doc__` — and this is
the drift check over the committed result, the same shape `xtask/tests/codegen_drift.rs` gives
`mjx-ooxml-types`: regenerate, and require the answer to be the file that is checked in.

Why not a comparison. Two prose surfaces of this size cannot be compared by equality (they would
have to be written identically) nor by similarity (a threshold that is subtly wrong across two
thousand members is worse than no check at all, which is the warning
`xtask/tests/binding_projection.rs` records about signature parsing). Neither instrument is needed,
because the two were never independent: one of them is a copy, and a copy can be made rather than
audited.
"""

from __future__ import annotations

import importlib.util
import pathlib
import sys
from types import ModuleType

import pytest

import mjx_ooxml

BINDING = pathlib.Path(__file__).resolve().parent.parent
STUB = pathlib.Path(mjx_ooxml.__file__).with_name("__init__.pyi")


def _tool() -> ModuleType:
    """`tools/stub_docs.py`, imported by path — it is a generator, not an installed module."""
    path = BINDING / "tools" / "stub_docs.py"
    assert path.is_file(), f"the generator is missing at {path}"
    spec = importlib.util.spec_from_file_location("mjx_stub_docs", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def tool() -> ModuleType:
    return _tool()


def test_the_committed_stub_is_what_the_generator_writes(tool: ModuleType) -> None:
    """The drift check. A hand-edited docstring fails here, and so does a stale one."""
    committed = STUB.read_text(encoding="utf-8")
    generated, governed = tool.rewrite(committed, mjx_ooxml)
    if generated != committed:
        import difflib

        diff = "\n".join(
            difflib.unified_diff(
                committed.split("\n"),
                generated.split("\n"),
                "committed",
                "generated",
                lineterm="",
                n=1,
            )
        )
        pytest.fail(
            f"the committed stub disagrees with the compiled module across its {governed} governed "
            "docstrings; run `python bindings/mjx-python/tools/stub_docs.py` to restate them:\n"
            + diff[:20_000]
        )


def test_the_scanner_is_still_matching(tool: ModuleType) -> None:
    """The anti-vacuity floor, and it is the whole reason this suite is not self-satisfying.

    A gate over two thousand docstrings that silently matches *none* of them rewrites the file to
    itself and passes, which is indistinguishable from a stub with no drift in it. So the count is
    asserted, and it is phrased as *the scanner has stopped matching* rather than as an exact size:
    an exact total fails for the one reason it must never fail, which is that somebody legitimately
    added a method — the lesson `test_stub_parity.py` records beside its own floor.
    """
    committed = STUB.read_text(encoding="utf-8")
    found = tool.positions(committed, mjx_ooxml)
    assert len(found) >= tool.GOVERNED_FLOOR, (
        f"only {len(found)} governed docstring(s) were found; the scanner has stopped matching, and "
        "the comparison above would pass on almost nothing"
    )
    # Both halves of the walk have to be alive, not just their total: the classes are found through
    # the stub's own syntax and the members through the compiled classes' `__dict__`, and either
    # could die alone.
    classes = sum(1 for position in found if position.name == "<class>")
    assert classes >= 250, f"only {classes} class docstring(s) were found; the class walk has died"
    members = len(found) - classes
    assert members >= 1_250, f"only {members} member docstring(s) were found; the member walk has died"
    print(f"\n{len(found)} governed docstrings: {classes} classes, {members} members")


def test_a_hand_edited_docstring_is_caught(tool: ModuleType) -> None:
    """The mutation, run against the real file rather than a fixture.

    Changing one docstring in a copy of the committed text must make the generator's answer differ
    from it — which is exactly what a hand edit to the checked-in stub does.
    """
    committed = STUB.read_text(encoding="utf-8")
    edited = committed.replace(
        '"""An open PowerPoint deck.',
        '"""An open PowerPoint presentation.',
        1,
    )
    assert edited != committed, "the docstring this mutation edits has moved; pick another"
    generated, _ = tool.rewrite(edited, mjx_ooxml)
    assert generated != edited, "a hand-edited docstring passed the drift check"
    assert generated == committed, "restating the edited stub did not recover the committed one"


def test_the_prose_is_never_a_second_copy(tool: ModuleType) -> None:
    """Every governed docstring says exactly what `help()` says, read back out of the file.

    `test_the_committed_stub_is_what_the_generator_writes` proves the file is what the generator
    writes; this proves what the generator writes *parses back* to the module's own prose, and is
    not, say, the output of a renderer that quietly dropped a paragraph or mangled a backslash.
    The literals are read with `inspect.cleandoc`, which is what removes the indentation the stub
    adds and Python itself adds nowhere — so this compares the sentence, not the layout, and the
    layout is what the byte comparison above is for.
    """
    committed = STUB.read_text(encoding="utf-8")
    import ast
    import inspect

    tree = ast.parse(committed)
    stated: dict[tuple[str, str], str] = {}
    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            text = ast.get_docstring(node, clean=False)
            if text is not None:
                stated[(node.name, "<class>")] = text
            pending: str | None = None
            for item in node.body:
                if isinstance(item, ast.FunctionDef):
                    pending = None
                    text = ast.get_docstring(item, clean=False)
                    if text is not None:
                        stated[(node.name, item.name)] = text
                elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
                    pending = item.target.id
                elif (
                    pending is not None
                    and isinstance(item, ast.Expr)
                    and isinstance(item.value, ast.Constant)
                    and isinstance(item.value.value, str)
                ):
                    stated[(node.name, pending)] = item.value.value
                    pending = None
                else:
                    pending = None
        elif isinstance(node, ast.FunctionDef):
            text = ast.get_docstring(node, clean=False)
            if text is not None:
                stated[("<module>", node.name)] = text

    problems: list[str] = []
    checked = 0
    for position in tool.positions(committed, mjx_ooxml):
        have = stated.get((position.owner, position.name))
        checked += 1
        if have is None or inspect.cleandoc(have) != position.text:
            problems.append(f"{position.owner}.{position.name}")
    assert not problems, "these stub docstrings are not the module's own prose: " + ", ".join(
        problems[:20]
    )
    assert checked >= tool.GOVERNED_FLOOR, (
        f"only {checked} docstring(s) were compared; the scanner has stopped matching"
    )
