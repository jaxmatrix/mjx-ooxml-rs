"""Writes the committed stub's docstrings out of the compiled module (MJXOFF-234).

`python/mjx_ooxml/__init__.pyi` is a committed artefact whose header has always said its prose
comes from the binding's `#[pymethods]` blocks. Until MJXOFF-234 that was a description of how it
was *written*, not of anything checked: `tests/test_stub_parity.py` compares **names** in both
directions and `test_every_deck_method_carries_a_docstring` asks only that a docstring *exists*, so
the sentence beside each name was unchecked text. Two hand-maintained copies of one fact drift, and
MJXOFF-226 found them doing it — disagreeing about the same item and differently wrong in each.

**The copy is removed rather than compared.** A similarity threshold over two thousand members is
the instrument `xtask/tests/binding_projection.rs` warns about — subtly wrong at that scale is worse
than absent — and it is not needed here, because the two are not independent prose. PyO3 compiles
each `///` doc comment verbatim into the member's `__doc__`, so **the compiled module already
carries the Rust sentence**, and the stub can be written from it with no parser between them.

# What is governed

Every position in the stub where the compiled module carries prose of this project's own:

* a class's docstring, from the class's `__doc__` — including the twelve exception classes, whose
  text is the C string literal in `src/errors.rs`;
* a method's, from the function object in the class's `__dict__` (a `staticmethod` unwrapped to the
  function it holds, because `staticmethod.__doc__` is Python's own boilerplate);
* an attribute's, from the getset descriptor of the same name;
* a module-level function's.

Two things are deliberately **not** governed, and both are excluded by shape rather than by a list:

* **dunders**, whose `__doc__` is PyO3's or Python's, not ours — `__int__` says `int(self)`;
* **enumeration members**, because `SomeEnum.Center` is an *instance* of its class, so its
  `__doc__` is the class's docstring read a second time. Reading them out of `vars(cls)` rather
  than with `getattr` is what tells the two apart.

# The line breaks are the Rust's own

A doc comment's `__doc__` carries the line breaks of the Rust source, and the column budgets
coincide: a `#[pymethods]` comment is written at `    /// ` — four of indent, four of prefix — and
its docstring sits at eight spaces of indent in the stub; a `#[pyclass]` comment is at `/// ` and
its docstring at four. So the text is re-indented and never re-wrapped, which is why this rewrites
1,861 of the docstrings already in the file to exactly what they already say.
"""

from __future__ import annotations

import argparse
import ast
import inspect
import pathlib
import sys
from typing import Iterator

# The stub is beside the compiled module, and the module is what this reads.
STUB_RELATIVE = "python/mjx_ooxml/__init__.pyi"

# The floor beneath the walk. `mjx_ooxml` declares several hundred classes and some two thousand
# documented members, so anything under this means the walk has stopped matching rather than that
# somebody deleted a few — the shape `bindings/mjx-python/tests/test_stub_parity.py` records for
# its own floor, and for the same reason: a scan that matches nothing renders a file identical to
# the one it was given and looks exactly like a file with no drift in it.
GOVERNED_FLOOR = 1_500


def documented(owner: type, name: str) -> str | None:
    """The prose the compiled class carries for `name`, or `None` if it carries none of ours.

    Read out of `vars(owner)` rather than with `getattr`, which is the whole of the discrimination:
    an enumeration member is an *instance* of its own class, so `getattr(TextAlignment, "Center")`
    answers with the class's docstring and would write it onto every member.
    """
    if name.startswith("__"):
        return None
    held = vars(owner).get(name)
    if held is None:
        return None
    if isinstance(held, (staticmethod, classmethod)):
        held = held.__func__
    if not (inspect.isroutine(held) or inspect.isdatadescriptor(held)):
        return None
    text = getattr(held, "__doc__", None)
    return text if isinstance(text, str) and text else None


def render(text: str, indent: int) -> list[str]:
    """One docstring, as the lines of stub source that state it.

    A single line that does not end in a quote is written inline; anything else opens on the first
    line and closes on its own, which is the form the file already uses. A backslash is doubled
    rather than the literal made raw, so that a docstring holding `\\n` — two characters, in prose
    about text that holds a newline — states two characters.
    """
    pad = " " * indent
    body = text.replace("\\", "\\\\")
    lines = body.split("\n")
    if len(lines) == 1 and not lines[0].endswith('"'):
        return [f'{pad}"""{lines[0]}"""']
    rendered = [f'{pad}"""{lines[0]}']
    rendered.extend(f"{pad}{line}" if line else "" for line in lines[1:])
    rendered.append(f'{pad}"""')
    return rendered


class Position:
    """One governed docstring: where it sits in the stub, and what it must say."""

    __slots__ = ("owner", "name", "text", "start", "end", "indent")

    def __init__(
        self, owner: str, name: str, text: str, start: int, end: int, indent: int
    ) -> None:
        self.owner = owner
        self.name = name
        self.text = text
        # `start` and `end` are zero-based line indices, `end` exclusive; `start == end` means the
        # stub states no docstring here and one is inserted.
        self.start = start
        self.end = end
        self.indent = indent

    def __repr__(self) -> str:  # pragma: no cover - diagnostics only
        return f"{self.owner}.{self.name}"


def _docstring_span(body: list[ast.stmt], indent: int) -> tuple[int, int]:
    """The line span of a leading docstring in `body`, or an empty span where one would go."""
    first = body[0]
    if (
        isinstance(first, ast.Expr)
        and isinstance(first.value, ast.Constant)
        and isinstance(first.value.value, str)
    ):
        return first.lineno - 1, first.end_lineno
    return first.lineno - 1, first.lineno - 1


def positions(stub_text: str, module: object) -> list[Position]:
    """Every governed docstring in the stub, in source order."""
    tree = ast.parse(stub_text)
    found: list[Position] = []
    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            cls = getattr(module, node.name, None)
            if not inspect.isclass(cls):
                continue
            text = cls.__doc__
            if isinstance(text, str) and text:
                start, end = _docstring_span(node.body, 4)
                found.append(Position(node.name, "<class>", text, start, end, 4))
            found.extend(_members(node, cls))
        elif isinstance(node, ast.FunctionDef):
            function = getattr(module, node.name, None)
            text = getattr(function, "__doc__", None)
            if isinstance(text, str) and text:
                start, end = _docstring_span(node.body, 4)
                found.append(Position("<module>", node.name, text, start, end, 4))
    found.sort(key=lambda position: position.start)
    return found


def _members(node: ast.ClassDef, cls: type) -> Iterator[Position]:
    """The governed docstrings of one class's methods and attributes."""
    pending: tuple[str, int] | None = None
    for item in node.body:
        if isinstance(item, ast.FunctionDef):
            pending = None
            text = documented(cls, item.name)
            if text is not None:
                start, end = _docstring_span(item.body, 8)
                yield Position(node.name, item.name, text, start, end, 8)
        elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
            # An attribute's docstring is the string expression on the line after it, if any.
            pending = (item.target.id, item.end_lineno or item.lineno)
        elif (
            pending is not None
            and isinstance(item, ast.Expr)
            and isinstance(item.value, ast.Constant)
            and isinstance(item.value.value, str)
        ):
            name, _ = pending
            pending = None
            text = documented(cls, name)
            if text is not None:
                yield Position(
                    node.name, name, text, item.lineno - 1, item.end_lineno or item.lineno, 4
                )
        else:
            if pending is not None:
                name, after = pending
                text = documented(cls, name)
                if text is not None:
                    yield Position(node.name, name, text, after, after, 4)
            pending = None
    if pending is not None:
        name, after = pending
        text = documented(cls, name)
        if text is not None:
            yield Position(node.name, name, text, after, after, 4)


def rewrite(stub_text: str, module: object) -> tuple[str, int]:
    """The stub with every governed docstring restated, and how many were governed.

    # Raises
    `AssertionError` if the walk found implausibly few — a scan that has stopped matching returns
    the file it was given, which is indistinguishable from a file with no drift.
    """
    lines = stub_text.split("\n")
    found = positions(stub_text, module)
    assert len(found) >= GOVERNED_FLOOR, (
        f"only {len(found)} governed docstring(s) were found in the stub; the scanner has stopped "
        f"matching, and a rewrite of nothing is indistinguishable from a stub with no drift"
    )
    out: list[str] = []
    cursor = 0
    for position in found:
        out.extend(lines[cursor : position.start])
        out.extend(render(position.text, position.indent))
        cursor = position.end
    out.extend(lines[cursor:])
    return "\n".join(out), len(found)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="write nothing; report whether the committed stub is current",
    )
    arguments = parser.parse_args(argv)

    import mjx_ooxml

    stub = pathlib.Path(__file__).resolve().parent.parent / STUB_RELATIVE
    current = stub.read_text(encoding="utf-8")
    wanted, governed_count = rewrite(current, mjx_ooxml)
    if wanted == current:
        print(f"the stub's {governed_count} governed docstrings are current")
        return 0
    if arguments.check:
        print(
            f"the stub has drifted from the compiled module across its {governed_count} governed "
            f"docstrings; run `python {pathlib.Path(__file__).name}` to restate them",
            file=sys.stderr,
        )
        return 1
    stub.write_text(wanted, encoding="utf-8")
    print(f"restated {governed_count} governed docstrings in {stub}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
