# The TypeScript surface

The page in the bindings' guide set that is genuinely about one binding. The mapping rules, at
`bindings/mjx-python/docs/guide/the_mapping_rules.md`, cover what the two bindings share; this page
covers what only a TypeScript caller meets. It is hosted here rather than beside the other four
for the same reason `mjx-mce` hosts its own page in the packaging tier's set: `mjx-python` and
`mjx-wasm` are siblings and neither may see the other.

## One package, two builds

`bindings/mjx-wasm/npm/package.json` publishes `@mjx/ooxml` with conditional exports:

| Specifier | Build | For |
|---|---|---|
| `@mjx/ooxml` | `bindings/mjx-wasm/npm/dist/bundler/` | webpack, Vite, Rollup, esbuild, Node with a bundler |
| `@mjx/ooxml/web` | `bindings/mjx-wasm/npm/dist/web/` | a browser loading the `.wasm` itself, with no bundler |

Both carry their own `mjx_ooxml.d.ts`, so the types ship with the code. `bindings/mjx-wasm/build-npm.sh`
emits both from one `wasm-pack` run each and prints the payload size; there is one `.wasm` per build
and nothing else to serve.

## Types come from the `.d.ts`, and it is generated

Every name, every argument and every doc comment in `bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml.d.ts` is emitted by
`wasm-bindgen` from the Rust. There is no hand-written declaration file to drift, which is the
difference between this binding and the Python one — there, `bindings/mjx-python/python/mjx_ooxml/__init__.pyi`
is committed and `bindings/mjx-python/tests/test_stub_parity.py` exists precisely to keep it honest.

That has a consequence worth knowing: the `.d.ts` is a **build artefact and is not committed**, so a
question about the TypeScript surface is answered by building it, not by reading the repository.

```sh
bindings/mjx-wasm/build-npm.sh
less bindings/mjx-wasm/npm/dist/bundler/mjx_ooxml.d.ts
```

## Handles, and freeing them

Every class in this package is a handle into WebAssembly linear memory. JavaScript's collector does
not know what one costs and will not reclaim it on its own, so each carries `free()` and
`[Symbol.dispose]()`. The pattern the suite uses — worth copying — is a scope that frees whatever it
was handed, whatever the body did:

```js
function scope(body) {
  const values = [];
  const owned = { keep(value) { values.push(value); return value; } };
  try {
    body(owned);
  } finally {
    for (const value of values) value.free();
  }
}
```

Two things do **not** need freeing, and both are in the suite for a reason. A value **moved into** a
call — the `CellWrite` entries handed to `writeCells`, for instance — is consumed by it, and freeing
it afterwards is a double free. And an enumeration member is a number, not a handle.

## The builder chain, and why the classes are frozen

A value class here is a description, not a handle to something in the document, and every builder
returns a **new** instance rather than mutating the receiver:

```js
const spec = new CellFormatSpec()
  .withFontIndex(2)
  .withFillIndex(3)
  .withAppliesFill(false);
```

Each link in that chain is a fresh handle, which is exactly why the `scope` helper above exists: the
chain leaves three intermediates behind. `bindings/mjx-wasm/tests/node/build_a_workbook.mjs` shows
the shape with its `keep` calls spelled out at every step, which reads badly and frees correctly.

Two of those builders are worth reading together. `withFontIndex` writes `@fontId` **and** the
`@applyFont="1"` that makes a consumer honour it, because an index with no apply flag is the usual
mistake. `withAppliesFont` then states the flag on its own, and takes `boolean | undefined` rather
than `boolean` because §18.8.9 makes the attribute three-valued: absent *participates*, `"0"`
*suppresses*, and those are not the same. Call it **after** the index builder, whose implied `"1"` it
replaces.

## What `wasm-bindgen` could not carry, and what was done instead

* **Tuples.** `tableDimensions`, `cellSpan` and `mergedCellAnchor` return a Rust `(u32, u32)`.
  `wasm-bindgen` cannot project one, and an array would type as `number[]`, so
  `bindings/mjx-wasm/src/deck.rs` declares `CellExtent` (`rows`, `columns`) and `CellAddress`
  (`row`, `column`). Python gets bare tuples for the same six methods.
* **Getters on an enumeration.** A C-like `#[wasm_bindgen]` enumeration is a number, so
  `Format.Presentation` is `0` and can carry nothing. The five things a format knows are free
  functions: `formatFamily`, `formatIsMacroEnabled`, `formatContentType`,
  `formatConventionalExtension`, `formatIsEditable`.
* **Half-open ranges.** `Cells.rectangle(0, 2, 1, 4)` takes four numbers where Rust takes two ranges.
* **Byte constants.** `defaultPlaceholderImage()` is a function rather than a constant, because a
  caller who mutated a shared `Uint8Array` would corrupt every later use of it.

## No serde, deliberately

`serde-wasm-bindgen` would be the obvious way to hand a `FillSpec` across as a plain object. It is
not used, and the reason is a layering one rather than a taste one: it would need `Serialize` and
`Deserialize` derives on `FillSpec` and `ColorSpec` in the **shipped** `mjx-dml`, which contradicts
`CLAUDE.md`'s decision that de/serialization here is hand-written through `mjx-derive` rather than
through serde. So every value crosses as a class with named accessors, and the cost is the `free()`
above.

## Failures

A failure is a real `Error`, not a class of this package's own — `instanceof Error` is true, it has a
`stack`, and every logger that special-cases `Error` handles it:

```js
try {
  deck.shapeText(0, 99);
} catch (failure) {
  failure.name;                 // "OoxmlError"
  failure.code;                 // "IndexOutOfRange"
  failure.detail.shape.indices; // [99]
}
```

`code` is one of the eleven strings `mjx_ooxml::ErrorCode` names, and `detail` carries only the
coordinates the failure actually had — `detail.row ?? null` is the way to read one. See
`bindings/mjx-wasm/src/errors.rs` for why a thrown `#[wasm_bindgen]` struct would have been worse.

## Running the suite

```sh
bindings/mjx-wasm/build-npm.sh
node --test "bindings/mjx-wasm/tests/node/*.mjs"
wasm-pack test --node bindings/mjx-wasm
```

The `node --test` run is the one that matters most: it imports from `bindings/mjx-wasm/npm/dist/bundler/`, so it tests
the **published shape** rather than the crate. `bindings/mjx-wasm/tests/browser.rs` is the same
surface driven from Rust inside a wasm runtime, which catches what a JavaScript test cannot — a value
that crosses the boundary wrongly rather than a name that is spelled wrongly.
