# The mapping rules

A caller who knows one of the three languages should be able to read the other two. That is only
true if the translation is a **rule** rather than a set of decisions, so this page states the rules
and then states every place they are broken — five in total, and each one is forced by the target
language rather than chosen.

## Python: the mapping is the identity

`mjx-ooxml`'s names are already `snake_case` and already self-explanatory, so nothing is renamed.
`mjx_ooxml::Deck::set_shape_run_properties` in Rust is `deck.set_shape_run_properties(…)` in Python,
and the walkthrough at `crates/mjx-ooxml/examples/build_a_deck.rs` translates into
`bindings/mjx-python/tests/test_build_a_deck.py` line for line.

What changes is the *shape* a Python caller expects, and none of it is a rename:

| Rust | Python |
|---|---|
| `&[u8]` / `Vec<u8>` | `bytes` |
| `Result<T, mjx_ooxml::Error>` | `T`, or an exception |
| `Option<T>` | `T \| None` |
| `impl Into<mjx_ooxml::Surface>` | `int \| Surface` |
| `impl Into<mjx_ooxml::ShapePath>` | `int \| Sequence[int] \| ShapePath` |
| `Range<u32>` | Python's own `range`, refused unless its step is 1 |
| `usize` | `int` — the facade takes `u32` throughout, so nothing here is platform-width |

## TypeScript: camelCase, derived rather than typed

A `snake_case` API is an immediate smell to a TypeScript consumer, so every exported name reaches
JavaScript in camelCase. `CLAUDE.md` describes this as *"an explicit `js_name` on every one"*, and
that is not quite what the code does: of the **1,743** functions `wasm-bindgen` exports, **1,605**
carry an explicit `js_name` and **138** carry none — every one of the 138 being a single word, where
snake case and camel case are the same string. The surface is camelCase throughout either way.

What matters is not that the attribute is present but that the name it gives is **derived**: a
hand-written `js_name` is a place a typo lands silently, and only the methods a test happens to call
would ever notice. `xtask/tests/binding_projection.rs` is the check —
`every_javascript_name_is_the_camel_case_of_its_rust_name` requires each `js_name` to equal the camel
case of the Rust name it sits on, and requires every name without one to be a single word.

## The five forced differences

### 1 · `None` is `NONE`, in fourteen enumerations

`None` is a Python keyword, so the `None` **member** of fourteen enumerations is spelled `NONE`:
nine from PowerPoint's vocabulary and five added with Excel's. It is the only rename in the whole
Python binding, and `bindings/mjx-python/tests/test_enums.py` proves it twice — `RENAMED_NONE` lists
the fourteen and checks each one, and `test_no_other_member_was_renamed` walks every exported class
looking for the shape of a rename and requires the list to be complete.

TypeScript has no such problem: `None` is an ordinary member name there, so
`bindings/mjx-wasm/src/enums.rs` keeps the Rust spelling and the two bindings read differently on
exactly those fourteen.

### 2 · A range becomes two numbers

JavaScript has no half-open range, and an object literal would type as `any` in the `.d.ts`. So
where Rust takes a `Range<u32>` and Python takes a `range`, TypeScript takes two numbers:
`Cells.rectangle(0, 2, 1, 4)` is rows 0–1 and columns 1–3.

### 3 · A `Format`'s accessors are free functions

A `#[wasm_bindgen]` C-like enumeration is a **number** in JavaScript, not an object, so it can carry
no methods and no getters — `Format.Presentation` is `0`, and `(0).contentType` is `undefined`. The
five things a format knows about itself are therefore free functions taking a format:
`formatContentType(Format.Presentation)`, `formatFamily`, `formatIsMacroEnabled`,
`formatConventionalExtension`, `formatIsEditable`. Python's enumerations are real classes, so it
spells the same five as properties on `Format`.

### 4 · Two classes exist only in TypeScript

`mjx_ooxml::Deck::table_dimensions`, `cell_span` and `merged_cell_anchor` — and their three
counterparts on `mjx_ooxml::Document` — return an anonymous `(u32, u32)`. Python gets a
`tuple[int, int]`, which is what a Python caller expects of a Rust tuple. `wasm-bindgen` cannot
return a tuple at all, and a two-element array would type as `number[]` and lose which half is
which, so `bindings/mjx-wasm/src/deck.rs` declares two classes with named getters:

* **`CellExtent`** — `rows` and `columns`, returned by `tableDimensions` and `cellSpan`;
* **`CellAddress`** — `row` and `column`, returned by `mergedCellAnchor`.

**Two, not one.** They are not interchangeable and it would be wrong if they were: an extent counts
and an address locates. MJXOFF-214 recorded this as the facade's divergence rather than the
binding's, and it is right that it is — had `mjx-ooxml` named the pair, all three languages would
agree — but the *shape* the wasm binding chose is better than the tuple it was given, not worse.

### 5 · Failures: eleven classes against one `Error`

`except` selects on a class in Python and on nothing in JavaScript, so the two projections of
`mjx_ooxml::Error` are opposites, and both are right.

* **Python** raises one of eleven subclasses of `OoxmlError`, one per `mjx_ooxml::ErrorCode`, each
  carrying `.code` and the five coordinates `.surface`, `.shape`, `.row`, `.column`, `.index`. Each
  is `None` when the failure carried no such coordinate, so none of them ever raises
  `AttributeError`. `IndexOutOfRangeError` is *also* an `IndexError`, so code that already guards a
  lookup keeps working when the lookup is a slide index.
* **TypeScript** throws a real `js_sys::Error` with `name === "OoxmlError"` and two own properties:
  `code`, the same eleven strings, and `detail`, a plain object carrying only the coordinates the
  failure had. A `#[wasm_bindgen]` struct thrown as an exception is not an `Error` — `instanceof
  Error` is false and it has no `stack` — and every logger and test runner that special-cases
  `Error` would treat it as an opaque object.

Both carry the same information, and `bindings/mjx-python/tests/test_errors.py` and
`bindings/mjx-wasm/tests/node/surface.mjs` check each side against the same failures.

## Nothing else differs

That is the whole list, and it is checkable rather than asserted. The two bindings export the same
185 value classes and the same 100 enumerations; `Deck`, `Document` and `Workbook` carry 255, 123 and
138 methods **in both languages, method for method**, read off
`bindings/mjx-python/python/mjx_ooxml/__init__.pyi` and the generated `mjx_ooxml.d.ts` rather than
off either binding's source.

Two members are Python's alone and are not on the list above because neither is a projection:
`Deck.__repr__` and `Workbook.__repr__` exist because a Python object without one is unhelpful in a
traceback, and they call nothing in `mjx-ooxml`. Being dunders they sit outside what the committed
stub declares and outside what `bindings/mjx-python/tests/test_stub_parity.py` compares, which is
why the 255 above still matches TypeScript's 255 exactly.
