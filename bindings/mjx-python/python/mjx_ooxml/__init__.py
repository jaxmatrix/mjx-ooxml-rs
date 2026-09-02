"""Read, edit and write PowerPoint files — a pure-Rust OOXML library, bound to Python.

Everything lives in the compiled half, ``mjx_ooxml._mjx_ooxml``; this module re-exports it so the
committed stubs (``__init__.pyi``) and the ``py.typed`` marker sit beside it in one package.

The star import is deliberate and safe: the extension module defines ``__all__`` and nothing else,
so what arrives here is exactly the documented surface. ``__init__.pyi`` states that surface for
type checkers, and ``tests/test_stub_parity.py`` proves the two agree name for name.
"""

from ._mjx_ooxml import *  # noqa: F401,F403
from ._mjx_ooxml import __all__, __doc__, __version__  # noqa: F401
