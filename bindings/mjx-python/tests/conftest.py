"""Where the repository's fixtures are, and where these tests write.

The fixtures live at the repository root, not inside this package, because the Rust suites use the
same files — which is the point: the three walkthroughs must start from the same bytes.
"""

from __future__ import annotations

import os
import pathlib

import pytest

REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURES = REPOSITORY_ROOT / "tests" / "fixtures"


@pytest.fixture(scope="session")
def fixtures() -> pathlib.Path:
    """The repository's fixture directory."""
    assert FIXTURES.is_dir(), f"expected the repository fixtures at {FIXTURES}"
    return FIXTURES


@pytest.fixture(scope="session")
def template(fixtures: pathlib.Path) -> bytes:
    """The small multi-layout template the guide starts from."""
    return (fixtures / "layouts.pptx").read_bytes()


@pytest.fixture(scope="session")
def word_document(fixtures: pathlib.Path) -> bytes:
    """A Word document, for the one case detection has to get right and editing must refuse."""
    return (fixtures / "sample.docx").read_bytes()


@pytest.fixture(scope="session")
def output_directory() -> pathlib.Path:
    """Where the walkthrough writes its deck, so the Rust and Node runs can be compared to it."""
    directory = pathlib.Path(
        os.environ.get("MJX_OUTPUT_DIR", REPOSITORY_ROOT / "target" / "examples")
    )
    directory.mkdir(parents=True, exist_ok=True)
    return directory
