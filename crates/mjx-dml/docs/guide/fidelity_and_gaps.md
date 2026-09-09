# Fidelity and the known gaps

This crate models more elements than any other in the workspace, and *modelling* is the risky word:
every type here is a partial view of an element it does not own, and the question a fidelity library
has to answer is what happens to the part it did not look at.

## The four mechanisms, and which of them are guaranteed once

At 0.0.138 the numbers are printed by `crates/mjx-dml/tests/serialization_ledger.rs` on every run, so
they cannot go stale silently.

| Mechanism | Types | Backed by |
|---|---|---|
| `#[derive(FromXml, ToXml)]` | 41 | **codegen** — `crates/mjx-derive/tests/derive.rs`, one failure reaching every derived type at once |
| `fidelity_element_impls!` | 57 | **one macro body** in `crates/mjx-dml/src/build.rs`, read once |
| A hand-written impl | 9 (13 impls) | **a ledger**, `crates/mjx-dml/tests/serialization_ledger.rs` |
| No `ToXml` at all | 3 of those 9 | nothing to lose |

The first two are the ones to want. `mjx-derive`'s `#[xml(children, child(…))]` arm emits the `Raw`
fallthrough **unconditionally** — there is no way to invoke it without one — and
`#[derive(XmlAttributes)]` never rebuilds the attribute vector, which is what keeps an unknown
attribute's position, prefix and quote character intact. `fidelity_element_impls!` is one four-line
body storing `name`, `attributes`, `children` and `empty` verbatim.

A hand-written pair is inside neither, and **MJXOFF-216 is what that costs**. Of the eight pairs this
crate held at 0.0.137, four destroyed content and two more re-emitted a self-closing element as an
open/close pair. All six are fixed at 0.0.138 — the four moved onto the derive, the two took the
macro's own self-closing formula — and each is pinned by a case in
`crates/mjx-dml/tests/in_context_roundtrip.rs` written against markup this project's writer would
never emit, so each fails against 0.0.137.

What stops a seventh is the ledger: every hand-written impl must be on it with an idiom and a reason,
**and the idiom is checked against the impl's own body** — a row claiming to preserve everything
while handing `RawElement::rebuilt` a fresh `Vec::new()` fails, which is exactly the shape
`Picture::to_xml` had.

**Each of the other two crates that writes serialization by hand now has a gate of its own**, and
neither is a copy of this one, because the idioms are the finding and they differ:
`crates/mjx-sml/tests/serialization_ledger.rs` (MJXOFF-220) follows a one-line delegation into the
`as_raw_element` behind it, and `crates/mjx-docx/tests/serialization_ledger.rs` (MJXOFF-218) compares
146 copies of one body against a canonical text character for character. The three files share a
source scanner and nothing else; the `mjx-docx` file records why that is three files rather than one
shared crate, and what the duplication costs.

## What is actually asserted

| Suite | What it proves |
|---|---|
| `crates/mjx-dml/tests/in_context_roundtrip.rs` | a real element out of a real part, through `from_xml`/`to_xml` and back — **whole part byte-identical**, plus a disagreeing corpus of literals in forms this writer never emits |
| `crates/mjx-dml/tests/serialization_ledger.rs` | no hand-written impl exists that nobody has justified |
| `crates/mjx-dml/tests/schema_order.rs` | a child this library inserts lands at its `xsd:sequence` rank — with fixtures that leave a **gap**, so neither an appending nor a prepending writer could pass |
| `crates/mjx-dml/tests/resolve_model.rs`, `color_model.rs` | the colour resolver's arithmetic, value-pinned |
| `crates/mjx-dml/tests/guide_formula.rs` | all seventeen operators of the guide-formula language |

The round-trip suite deserves its own note, because it is where the trap in this kind of testing
lives. Every committed fixture was authored here or by LibreOffice, so every `ST_OnOff` in the corpus
is already spelled the way this project writes one and every value is already double-quoted: **a
byte-identity assertion over that corpus cannot see a normalizing bug**, because the writer agrees
with the corpus and we wrote both. So the fixtures are joined by hand-written literals carrying
`rotWithShape='on'`, `sx="105%"`, single quotes, an unknown attribute *between* two known ones, a
namespaced attribute and a character reference in a value a model actually reads — and the same
assertion is made over those.

## The gaps, each with why it is still there

### A `*Spec` is not a `*` — by design, and worth stating once more

Every `spec()` in this crate drops what the description does not describe: a gradient's shade path, a
blip's effect chain, tile and fill rectangles. `value.spec(i).to_fill(i)` is therefore **not** the
identity, and a caller who reads through a spec and writes back through one has rewritten the element
from its key values. Read through the spec, edit through the wrapper.

**A colour's transforms used to be on that list, and are not any more.** Until MJXOFF-219
[`mjx_dml::ColorSpec`](crate::ColorSpec) had three variants and none carried a transform child, so no
authoring path — here or through any facade above — could produce a `a:comp`, `a:gray`, `a:gamma` or
`a:invGamma`, and `Color::spec()` silently dropped a producer's. Both halves are closed:
[`ColorSpec::with_transform`](crate::ColorSpec::with_transform) and its six named conveniences author
every member of `EG_ColorTransform`, and [`mjx_dml::Color::spec`](crate::Color::spec) carries them
into the description in order, so `spec()` → [`from_spec`](crate::Color::from_spec) keeps them.
`crates/mjx-dml/docs/guide/filling_outlining_and_colour.md` is the how-to; the *rest* of the sentence
above still holds, and a colour is not the counter-example it once was.

### Two preset shapes stay mechanical

`teardrop` and `sun` read as [`ShapeGeometry::Unmodeled`](crate::ShapeGeometry) and always have —
a Phase A deferral, recorded as spec-ambiguous, and
`crates/mjx-dml/tests/shape_model.rs`'s `unported_shape_is_unmodeled_and_unknown_prst_is_none` is
what keeps the claim honest. Both still round-trip byte for byte, and both still expose their
adjustments by wire name through
[`mjx_dml::PresetGeometry::adjustment`](crate::PresetGeometry::adjustment). Naming a control
parameter the ECMA prose does not pin would be worse than leaving it unnamed.

Separately: `ST_ShapeType` has 187 tokens but `presetShapeDefinitions.xml` holds **186 distinct
shapes** — `upDownArrow` is defined twice and `upArrow` has no geometry block at all. See
`docs/DRAWINGML_PRESET_SHAPES.md`.

### `a:t` does not preserve an entity spelling through an edit

The derive's text arm writes a single minimally-escaped text node, so `&#38;` inside an `a:t` that
something edited comes back as `&amp;`. It is a write-path property only — an untouched part
round-trips byte for byte with no model involved. [Text bodies](text_bodies) has the argument.

### `mjx_dml::Theme` has no writer

Deliberate: it is a projection that drops `a:bgFillStyleLst`, the `@name` attributes and every unknown
child, so serializing it would emit a theme with holes. Authoring goes through
[`mjx_dml::default_theme_xml`](crate::default_theme_xml), which is a whole document rather than a
serialization of a view. [The theme](the_theme) has the rest, including the rule that a package which
already has a theme keeps it.

### One payload kind is typed; the rest are raw

[`mjx_dml::GraphicData`](crate::GraphicData) types `pic:pic` and nothing else. A chart, a diagram, a
table, an OLE object and a Word shape all stay
[`GraphicDataContent::Raw`](crate::GraphicDataContent::Raw) — **preserved verbatim in their original
positions**, and parsed by the crate above if it wants them. That is the layering rule, not an
omission: typing a `c:chart` here would mean reaching up past rank 2.0.

### `mjx-pptx` navigates `p:spPr` by hand

[`mjx_dml::ShapeProperties`](crate::ShapeProperties) arrived after PowerPoint's own reader was written
and tested, and migrating it is a refactor with no defect behind it. Both readers insert a new child
through the same generated [`SHAPE_PROPERTIES`](mjx_ooxml_types::child_order::SHAPE_PROPERTIES) rank
table, which is the one thing that would actually corrupt a file if they drifted. New callers use the
type. The reasoning is at the head of `crates/mjx-dml/src/shape_properties.rs`.

## What is *not* promised

* **Nothing here is validated against the schema.** `mjx-schema-gate` validates whole parts written by
  the three format crates; a type in this crate will happily hold markup that does not validate,
  because refusing to read an invalid file is not fidelity.
* **Nothing here is a renderer.** The resolvers and the formula evaluator answer *what value is this*,
  and `comp`/`gray`/`gamma`/`invGamma` in particular follow a documented interpretation that is **not**
  guaranteed pixel-identical to Microsoft Office.
* **Byte identity is per part, not per ZIP.** That contract belongs to `mjx-opc` and is stated in
  `crates/mjx-opc/docs/guide/the_round_trip_contract.md`, which is also where this crate's exceptions
  are listed alongside the rest of the workspace's.
