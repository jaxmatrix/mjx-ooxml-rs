# mjx-ooxml for Python

Read, edit and write PowerPoint files. A [pure-Rust OOXML library](https://github.com/jaxmatrix/mjx-ooxml-rs)
with a PyO3 binding — no LibreOffice, no COM, no PowerPoint, no C libraries.

```python
import mjx_ooxml

deck = mjx_ooxml.Deck.blank(mjx_ooxml.SlideSize.widescreen())
slide = deck.add_slide_from_layout(0)
title = deck.add_text_box(slide, "Quarterly results",
                          mjx_ooxml.ShapeBounds.from_inches(0.5, 0.4, 9.0, 1.2))
deck.set_shape_run_properties(
    slide, title,
    mjx_ooxml.CharacterPropertiesSpec()
        .with_size_points(40)
        .with_color(mjx_ooxml.ColorSpec.srgb("1F3864")))

with open("out.pptx", "wb") as file:
    file.write(deck.save())
```

## Installing

```sh
pip install mjx-ooxml
```

Wheels are `abi3-py39`: one per platform, working on CPython 3.9 and every later version.

## What you get

* **The whole PowerPoint surface** — 257 methods on `Deck`: slides, shapes, groups, text, runs,
  paragraphs, hyperlinks, fills, outlines, effects, 3-D, geometry, tables, cells, charts, chart
  decoration, pictures, media, notes, SmartArt, OLE objects, ActiveX controls, ink, and package
  hygiene.
* **Fidelity.** Open any deck, change one thing, write it back: every part you did not touch is
  re-emitted byte for byte. The library never rewrites markup it did not need to.
* **Types.** Committed stubs and a `py.typed` marker, checked with `mypy --strict`.
* **Real exceptions.** Everything raises a subclass of `OoxmlError`, one per error code, each
  carrying `.code` and the coordinates `.surface`, `.shape`, `.row`, `.column`, `.index`.
  `IndexOutOfRangeError` is also an `IndexError`.

## Addressing

A **surface** is an `int` (the slide index) or a `Surface`:

```python
deck.shape_count(0)                       # slide 0
deck.shape_count(mjx_ooxml.Surface.layout(1))
deck.shape_count(mjx_ooxml.Surface.notes_master())
```

A **shape** is an `int` (a top-level shape), a list of `int` (a descent through nested groups), or
a `ShapePath`:

```python
deck.shape_kind(0, 2)          # the third top-level shape
deck.shape_kind(0, [2, 1])     # member 1 of the group at index 2
```

## Reading a file

The library is bytes in and bytes out — it never touches a filesystem, a clock, a thread or a
random number generator. The file I/O is deliberately yours:

```python
with open("in.pptx", "rb") as file:
    deck = mjx_ooxml.Deck.open(file.read())
```

`mjx_ooxml.detect_format(data)` says what a file is by reading its package, not its name, so a
`.pptm`, a `.potx` and a `.docx` renamed to `.pptx` are all reported correctly.

## Threading

Use one `Deck` from one thread. `Deck.open` and `Deck.save` release the interpreter lock for their
duration, so opening one deck per thread genuinely parallelises; nothing else does enough work to
be worth it.

## Status

Pre-release (`v0.0.x`). PowerPoint is implemented and tested; Word and Excel are detected but not
yet editable.

## Licence

MIT or Apache-2.0, at your option.
