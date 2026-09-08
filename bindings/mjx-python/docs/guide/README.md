# The bindings

**One surface, three languages.** `mjx-ooxml` is the library; `bindings/mjx-python` projects it onto
Python as the extension module `mjx_ooxml`, and `bindings/mjx-wasm` projects it onto JavaScript and
TypeScript as the npm package `@mjx/ooxml`. Neither adds behaviour: every method calls exactly one
`mjx_ooxml` method, and every class wraps exactly one value.

This is **one guide set over two crates**, hosted here for the same reason the packaging tier's set
is hosted by `mjx-opc` and the upper markup's by `mjx-chart`: the two bindings are siblings, neither
may see the other, and the story is the same story told twice. The one page that is genuinely about
TypeScript alone is hosted by the crate it is about.

| Page | Crate | What it covers |
|---|---|---|
| [Installing](installing) | `mjx-python` | The wheel, the npm package, and what a build of each needs |
| [The mapping rules](the_mapping_rules) | `mjx-python` | Identity in Python, camelCase in TypeScript, and the five places the shape is forced to differ |
| [What is not projected](what_is_not_projected) | `mjx-python` | The nine methods that stay in Rust, the one gap that is a gap, and two classes nothing can produce |
| [How much is exercised](how_much_is_exercised) | `mjx-python` | What the suites actually call, measured, and the gate that keeps the figure honest |
| The TypeScript surface (`bindings/mjx-wasm/docs/guide/the_typescript_surface.md`) | `mjx-wasm` | The npm package's exports, the two classes it had to invent, and errors as real `Error`s |

## The shape of the API, in one page

Three handle classes, and a vocabulary of value classes their arguments and results are made of.

| Handle | Opens | Methods | Where it lives |
|---|---|---|---|
| `Deck` | PresentationML — `.pptx`, `.pptm`, `.potx`, `.potm`, `.ppsx`, `.ppsm` | **255** | `bindings/mjx-python/src/deck.rs`, `bindings/mjx-wasm/src/deck.rs` |
| `Document` | WordprocessingML — `.docx`, `.docm`, `.dotx`, `.dotm` | **123** | `bindings/mjx-python/src/document.rs`, `bindings/mjx-wasm/src/document.rs` |
| `Workbook` | SpreadsheetML — `.xlsx`, `.xlsm`, `.xltx`, `.xltm` | **138** | `bindings/mjx-python/src/workbook.rs`, `bindings/mjx-wasm/src/workbook.rs` |

**Those three counts are the same in both languages, method for method.** They are not an
approximation and not a promise: they are read off the two committed contracts —
`bindings/mjx-python/python/mjx_ooxml/__init__.pyi`, which `bindings/mjx-python/tests/test_stub_parity.py` holds to the
compiled module in both directions, and the `mjx_ooxml.d.ts` that `bindings/mjx-wasm/build-npm.sh`
emits, whose only extra member per class is `wasm-bindgen`'s own `free()`.

Beside the handles sit **181 value classes** — `ShapeBounds`, `ColorSpec`, `FillSpec`,
`CellWrite`, `ChartData`, `CellFormatSpec` and the rest — and **102 enumerations**, and both bindings
export exactly the same ones. Only three names differ between the two exports, and every one is
forced: Python adds the eleven exception subclasses of `OoxmlError` because `except` selects on a
class there, and TypeScript adds `CellExtent` and `CellAddress` because `wasm-bindgen` cannot return
a tuple. [The mapping rules](the_mapping_rules) is where each of those is argued.

## Which page answers what

* *How do I install it, and what does building it from source need?* — [Installing](installing).
* *I know the Rust API. What is this call called here?* — [The mapping rules](the_mapping_rules).
* *Why can I not do X from Python?* — [What is not projected](what_is_not_projected).
* *How much of this is actually tested?* — [How much is exercised](how_much_is_exercised), which
  answers with a number rather than an impression.
* *What does the npm package look like from TypeScript?* — The TypeScript surface, at
  `bindings/mjx-wasm/docs/guide/the_typescript_surface.md`, which the wasm crate hosts because
  neither binding may see the other.

The reference is rustdoc and the two committed contracts, not these pages: every public item already
carries a doc comment, `bindings/mjx-python/python/mjx_ooxml/__init__.pyi` restates all of them in
Python, and the generated `.d.ts` restates them in TypeScript. What is written here is the part
neither can say — why the surface has the shape it has, and where it stops.
