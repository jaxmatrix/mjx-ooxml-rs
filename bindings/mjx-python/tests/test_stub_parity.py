"""The committed stubs and the compiled module say the same thing.

`python/mjx_ooxml/__init__.pyi` is a committed artefact, exactly as `mjx-ooxml-types` is: it is
derived from the Rust and checked in, so `mypy --strict` and every editor see the surface without
building anything. A committed artefact needs a check that it has not drifted — this is it.

Both directions matter, and for different reasons:

* a name at run time that the stub does not declare is a method a typed caller cannot call, and
* a name in the stub that does not exist at run time is a promise that fails at the call site.
"""

from __future__ import annotations

import ast
import inspect
import pathlib

import pytest

import mjx_ooxml

STUB = pathlib.Path(mjx_ooxml.__file__).with_name("__init__.pyi")

# Members the stub states for every class because Python states them for every class.
UNIVERSAL = {"__init__", "__new__", "__init_subclass__", "__subclasshook__", "__class_getitem__"}


@pytest.fixture(scope="module")
def stub() -> ast.Module:
    """The committed stub, parsed."""
    assert STUB.is_file(), f"the committed stub is missing at {STUB}"
    return ast.parse(STUB.read_text(), filename=str(STUB))


def stub_classes(stub: ast.Module) -> dict[str, ast.ClassDef]:
    """Every class the stub declares, by name."""
    return {node.name: node for node in stub.body if isinstance(node, ast.ClassDef)}


def stub_members(node: ast.ClassDef) -> set[str]:
    """Every method, attribute and enumeration member one stub class declares."""
    names: set[str] = set()
    for item in node.body:
        if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
            names.add(item.name)
        elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
            names.add(item.target.id)
    # Dunders are compared nowhere: `dir()` reports the ones every object inherits, so the two
    # sides could never agree on them and nothing about them is part of this binding's contract.
    return {name for name in names if not name.startswith("_")}


def runtime_members(cls: type) -> set[str]:
    """Every public member a compiled class actually has."""
    return {
        name
        for name in dir(cls)
        if not name.startswith("_") and name not in UNIVERSAL
    }


def test_every_exported_name_is_declared_by_the_stub(stub: ast.Module) -> None:
    """`__all__` is derived from what was registered, so this compares the stub to the module."""
    declared = set(stub_classes(stub))
    for node in stub.body:
        if isinstance(node, ast.FunctionDef):
            declared.add(node.name)
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            declared.add(node.target.id)

    missing = sorted(name for name in mjx_ooxml.__all__ if name not in declared)
    assert not missing, f"the compiled module exports names the stub does not declare: {missing}"


def test_the_stub_declares_nothing_that_does_not_exist(stub: ast.Module) -> None:
    """A stub that over-promises is worse than no stub: it type-checks code that then fails."""
    extra = sorted(
        name
        for name in stub_classes(stub)
        if not hasattr(mjx_ooxml, name)
    )
    assert not extra, f"the stub declares classes the module does not have: {extra}"


def test_every_class_agrees_member_for_member(stub: ast.Module) -> None:
    """The members of each class match, in both directions, for every class in the module."""
    problems: list[str] = []
    for name, node in stub_classes(stub).items():
        cls = getattr(mjx_ooxml, name, None)
        if cls is None or not inspect.isclass(cls):
            continue
        if issubclass(cls, BaseException):
            # The exception classes are declared by hand: their attributes are set on instances,
            # not on the class, so `dir()` cannot see them and there is nothing to compare.
            continue
        declared = stub_members(node) - UNIVERSAL
        actual = runtime_members(cls)
        for missing in sorted(actual - declared):
            problems.append(f"{name}.{missing} exists but is not in the stub")
        for absent in sorted(declared - actual):
            problems.append(f"{name}.{absent} is in the stub but does not exist")
    assert not problems, "the stub has drifted from the module:\n  " + "\n  ".join(problems)


def test_the_deck_declares_every_bound_method(stub: ast.Module) -> None:
    """The one class the whole binding is about, checked explicitly and counted."""
    node = stub_classes(stub)["Deck"]
    declared = stub_members(node) - UNIVERSAL
    actual = runtime_members(mjx_ooxml.Deck)
    assert declared == actual
    # Six lifecycle methods plus the delegated surface. The count is stated so that a method
    # silently dropped from the generator is a failure rather than a smaller number nobody reads.
    assert len(actual) == 253, (
        f"expected 253 methods on Deck without the `vml` feature, found {len(actual)}"
    )


def test_the_module_docstring_and_version_are_present() -> None:
    """Two things every packaged module owes its user."""
    assert mjx_ooxml.__doc__ and "PowerPoint" in mjx_ooxml.__doc__
    assert mjx_ooxml.__version__.count(".") == 2


def test_every_public_class_carries_a_docstring() -> None:
    """A binding whose classes have no `help()` is a binding nobody can explore."""
    undocumented = [
        name
        for name in mjx_ooxml.__all__
        if inspect.isclass(getattr(mjx_ooxml, name))
        and not getattr(mjx_ooxml, name).__doc__
    ]
    assert not undocumented, f"these classes have no docstring: {undocumented}"


def test_every_deck_method_carries_a_docstring() -> None:
    """The docstrings are `mjx-ooxml`'s own summaries, so they cannot drift from the Rust."""
    undocumented = [
        name
        for name in runtime_members(mjx_ooxml.Deck)
        if not getattr(getattr(mjx_ooxml.Deck, name), "__doc__", None)
    ]
    assert not undocumented, f"these Deck methods have no docstring: {undocumented}"
