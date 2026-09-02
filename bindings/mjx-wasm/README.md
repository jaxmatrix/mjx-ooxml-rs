# @mjx/ooxml

Read, edit and write PowerPoint files in the browser, in Node, in Bun, in a worker. A
[pure-Rust OOXML library](https://github.com/jaxmatrix/mjx-ooxml-rs) compiled to WebAssembly, with
real TypeScript types.

```ts
import { Deck, ShapeBounds, CharacterPropertiesSpec } from "@mjx/ooxml";

const deck = Deck.open(new Uint8Array(await file.arrayBuffer()));
try {
  const slide = deck.addSlideFromLayout(0);
  const title = deck.addTextBox(slide, "Quarterly results",
                                ShapeBounds.fromInches(0.5, 0.4, 9.0, 1.2));
  deck.setShapeRunProperties(
    slide, title,
    new CharacterPropertiesSpec().withSizePoints(40).withBold(true));

  const blob = new Blob([deck.save()], {
    type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  });
  // …hand `blob` to a download link, an upload, an object URL…
} finally {
  deck.free();          // ← not optional
}
```

## `free()` is mandatory

A `Deck` is memory on the WebAssembly heap. JavaScript's garbage collector does not know it exists
and **will never reclaim it**: a deck you do not free is leaked for the lifetime of the module, and
a multi-megabyte deck leaked in a loop will exhaust the heap.

So wrap every deck in `try { … } finally { deck.free() }`. Where explicit resource management is
available, `using deck = Deck.open(bytes)` does the same thing — the package emits
`[Symbol.dispose]`.

The same is true of every other class here (`FillSpec`, `ShapeBounds`, `Surface`, …), though those
are small enough that leaking one matters far less. The numeric spellings of an address —
`deck.shapeFill(0, 2)` rather than `deck.shapeFill(Surface.slide(0), ShapePath.top(2))` — allocate
nothing at all, which is why the examples use them.

Calling a method on a freed object throws `Error: null pointer passed to rust`.

## Installing

```sh
npm install @mjx/ooxml
```

One package, two builds, chosen by conditional exports:

```ts
import { Deck } from "@mjx/ooxml";        // bundler build — Vite, webpack, Rollup, Node, Bun
import { Deck } from "@mjx/ooxml/web";    // ESM build for a browser with no bundler
```

The `/web` build needs its `init()` called before anything else:

```js
import init, { Deck } from "@mjx/ooxml/web";
await init();
```

The bundler build needs no initialisation. In Node it relies on WebAssembly ES-module imports, which
Node supports and currently prints an experimental warning for.

## What you get

* **The whole PowerPoint surface** — 257 methods on `Deck`: slides, shapes, groups, text, runs,
  paragraphs, hyperlinks, fills, outlines, effects, 3-D, geometry, tables, cells, charts, chart
  decoration, pictures, media, notes, SmartArt, OLE objects, ActiveX controls, ink, and package
  hygiene.
* **Fidelity.** Open any deck, change one thing, write it back: every part you did not touch is
  re-emitted byte for byte.
* **Types.** A generated `.d.ts` with every method, every enumeration and every doc comment.
* **Real errors.** Every failure is an `Error` with `name === "OoxmlError"`, a stable `code`, and a
  `detail` object carrying the coordinates:

  ```ts
  try {
    deck.shapeText(0, 99);
  } catch (failure) {
    failure instanceof Error;        // true
    failure.code;                    // "IndexOutOfRange"
    failure.detail.shape.indices;    // [99]
  }
  ```

## Naming

Methods are camelCase — `deck.setShapeRunProperties(…)`. Classes and enumeration members keep their
PascalCase spelling. Two shapes differ from the Rust and from the Python binding, both because the
platform forces it:

* a range argument becomes two numbers (`…Start`, `…End`), because JavaScript has no half-open
  range; and
* a `Format`'s five accessors are free functions (`formatContentType(format)`) rather than
  properties, because a WebAssembly enumeration is a number in JavaScript and cannot carry a getter.

## Addressing

A **surface** is a number (the slide index) or a `Surface`; a **shape** is a number (a top-level
shape), an array of numbers (a descent through nested groups), or a `ShapePath`:

```ts
deck.shapeKind(0, 2);                        // the third top-level shape of slide 0
deck.shapeKind(0, [2, 1]);                   // member 1 of the group at index 2
deck.shapeKind(Surface.layout(1), 0);        // the first shape of layout 1
```

## No file system

The library is bytes in and bytes out: no filesystem, no clock, no threads, no random number
generator, no network. `File`, `Blob` and `fetch` stay on your side of the boundary, which is exactly
why the same calls work in a browser and in Node.

## Status

Pre-release (`v0.0.x`). PowerPoint is implemented and tested; Word and Excel are detected but not yet
editable.

## Licence

MIT or Apache-2.0, at your option.
