# Installing

Both packages are self-contained. Neither needs LibreOffice, PowerPoint, COM, a system XML library
or any other C dependency: the whole of `mjx-ooxml` is pure Rust, and what ships is one compiled
artefact per package.

## Python

```sh
pip install mjx-ooxml
```

The wheels are **`abi3-py39`**: one per platform, working unchanged on CPython 3.9 and every later
version, because PyO3's `abi3` build links against the stable ABI rather than one interpreter's. The
import name is `mjx_ooxml` and the distribution name is `mjx-ooxml`; `pyproject.toml` calls that a
mixed layout, with `bindings/mjx-python/python/mjx_ooxml/__init__.py` re-exporting the compiled
`_mjx_ooxml` beside the committed stub and the `py.typed` marker.

```python
import mjx_ooxml

deck = mjx_ooxml.Deck.open(open("in.pptx", "rb").read())
print(deck.slide_count(), mjx_ooxml.detect_format(open("in.pptx", "rb").read()))
```

**Bytes in, bytes out.** No call takes a path: `Deck.open` takes `bytes` and `Deck.save` returns
`bytes`. Reading and writing files is the caller's, so the same code works against an object store, a
request body or a test fixture.

### Building it from source

```sh
cd bindings/mjx-python
maturin develop        # into the active virtualenv
pytest                 # the suite
mypy --strict          # the stub, against the tests that use it
```

`maturin` turns on the `extension-module` feature, which drops the `libpython` link so the wheel
resolves its symbols from whichever interpreter loads it. A plain `cargo build` and
`cargo test --workspace` leave that feature off and link normally, which is why
`bindings/mjx-python/Cargo.toml` declares `crate-type = ["cdylib", "rlib"]` — the wheel ships only
the `cdylib`, and the `rlib` is what lets this crate's own tests build inside the workspace run.

## JavaScript and TypeScript

```sh
npm install @mjx/ooxml
```

```js
import { Deck, SlideSize, detectFormat } from "@mjx/ooxml";

const deck = Deck.blank(SlideSize.widescreen());
const slide = deck.addSlide();
deck.setShapeText(slide, deck.addTextBox(slide, "Hello", bounds), "Hello");
const bytes = deck.save();
```

`bindings/mjx-wasm/npm/package.json` declares **conditional exports**: the bare specifier resolves to
the `bundler` build, and `@mjx/ooxml/web` to the `web` build for a browser that loads the `.wasm`
itself. Both carry their own `mjx_ooxml.d.ts`, so the types come from the package rather than from a
separate `@types` publication.

### Building it from source

```sh
bindings/mjx-wasm/build-npm.sh                       # both targets into npm/dist/
node --test "bindings/mjx-wasm/tests/node/*.mjs"     # the published shape
wasm-pack test --node bindings/mjx-wasm              # the Rust side, in a wasm runtime
```

`build-npm.sh` needs `wasm-pack` and the `wasm32-unknown-unknown` target. It **refuses to run when
`bindings/mjx-wasm/npm/package.json`'s version and the workspace's disagree**, which is why that file is part of every
release commit rather than something updated afterwards.

## Freeing what you allocate

The one thing a JavaScript caller must do that a Python caller need not. A `@mjx/ooxml` value is a
handle into linear memory, and JavaScript's collector does not know what it costs, so every class
carries `free()` and `[Symbol.dispose]()`:

```js
const spec = new CellFormatSpec();
try {
  workbook.formatCells(0, "A1:B2", spec.withFontIndex(2));
} finally {
  spec.free();
}
```

`bindings/mjx-wasm/tests/node/workbook_surface.mjs` wraps that in a `scope` helper that frees
everything it was handed, whatever the test did; it is worth copying. In Python the same objects are
ordinary reference-counted values and there is nothing to release.

## What a wheel and a package do *not* carry

The `vml` feature. `bindings/mjx-python/Cargo.toml` and `bindings/mjx-wasm/Cargo.toml` both declare
it, and it re-exposes four more `Deck` methods — `add_vml_drawing`, `vml_drawing_part`,
`vml_part_bytes` and `vml_part_names` — for an OLE object's or a control's legacy fallback. It is
**off** in a default build, exactly as it is off in `mjx-ooxml` itself, so the committed stub
describes 255 `Deck` methods rather than 259. A build that turns it on has four more, and the stub
would have to grow with them.
