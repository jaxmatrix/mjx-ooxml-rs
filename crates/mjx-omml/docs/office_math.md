# Office MathML

The sixth page of the upper shared markup's guide, whose other pages live in `mjx-chart` and whose
index is `crates/mjx-chart/docs/guide/README.md`. It is hosted here rather than there because
`mjx-chart` and `mjx-omml` are the **same layering rank**: neither may depend on the other, so a page
in that directory could not resolve a single intra-doc link into this crate. That is the layering rule
showing through the documentation rather than a gap in it — `mjx_mce::guide` sits beside
`mjx_opc::guide` for exactly the same reason.

## What OMML is

`shared-math.xsd` (ECMA-376 Part 1 §22.1, target namespace
`http://schemas.openxmlformats.org/officeDocument/2006/math`) is **Word's equation markup**. An
equation is an `m:oMath` inline inside a paragraph, or an `m:oMathPara` holding several of them as a
display block. It is mathematical *typesetting*: a fraction, a radical, an n-ary operator, a matrix,
the four script forms and their siblings, each with an operand slot that holds more of the same.

```text
m:oMath                       →  Math
  m:f       (fraction)        →  Fraction
    m:num   (numerator)       →  Argument   ← the recursive core: an argument holds MathElements
    m:den   (denominator)     →  Argument
  m:r       (a run)           →  Run
    m:t     (its characters)  →  Text
```

[`Argument`](crate::Argument) (`CT_OMathArg`) is every object's own operand slot and is why the model
recurses: an argument holds [`MathElement`](crate::MathElement)s, a 20-way choice
(`EG_OMathMathElements`) that includes every object above.

```rust
use mjx_ooxml_core::FromXml;
use mjx_omml::{Math, MathElement};

let document = mjx_xml::fidelity::parse(
    br##"<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <m:f>
    <m:num><m:r><m:t>1</m:t></m:r></m:num>
    <m:den><m:r><m:t>2</m:t></m:r></m:den>
  </m:f>
</m:oMath>"##,
)
.expect("well-formed");

let math = Math::from_xml(&document.root, &document.interner).expect("m:oMath");
let elements = math.elements(&document.interner);
let MathElement::Fraction(fraction) = &elements[0] else { panic!("a fraction") };
let numerator = fraction.numerator(&document.interner).expect("m:num");
assert_eq!(numerator.elements(&document.interner).len(), 1);
```

## What OMML is *not*

**It is not MathML.** W3C MathML is a different vocabulary in a different namespace; nothing here
reads or writes it, and nothing converts between the two.

**It is not LaTeX, and there is no equation parser.** This crate reads and writes the markup. Building
an equation means building the objects — `mjx_omml::Fraction::new`, `mjx_omml::Argument::new` and
their siblings — which is what `mjx_docx::Document::append_math` takes a closure for.

**It is not a part.** There is no `math1.xml`. An equation is always a subtree of the part that
carries it, which is why it is validated with that part rather than on its own, and why the child
order it is held to comes from `shared-math`'s table inside the same generated module as `wml`'s.

**It does not render.** Nothing here measures a glyph, chooses a break or resolves a font.

## All 72 complex types, but 47 Rust types

The schema declares 72 complex types and every one is modelled. It is 47 Rust types, and the
difference is deliberate reuse rather than a gap:

* **Twenty leaf `CT_*` value types** — `CT_OnOff`, `CT_Shp`, `CT_Integer255`, `CT_Char` and the rest —
  are all "one element with one `m:val` attribute". They share one mechanism (`crate::leaf`) instead
  of twenty near-identical types.
* **Six `*Pr` types collapse into [`ControlOnlyProperties`](crate::ControlOnlyProperties)**:
  `CT_FuncPr`, `CT_LimLowPr`, `CT_LimUppPr`, `CT_SPrePr`, `CT_SSubPr` and `CT_SSupPr` are, byte for
  byte, *one optional `m:ctrlPr` child and nothing else*. Which of the six a value models is its wire
  name, not its Rust type — the same reuse `mjx-docx`'s `StyleString` already establishes for
  `CT_String`.

The schema's 14 simple types are all generated:
`mjx_ooxml_types::officemath` supplies `ST_Shp` as
[`DelimiterShape`](mjx_ooxml_types::officemath::DelimiterShape), `ST_LimLoc` as
[`LimitLocation`](mjx_ooxml_types::officemath::LimitLocation), and so on. **This crate hand-writes
none of them.**

## Attributes are namespace-qualified, and that is unusual

`shared-math.xsd` is `attributeFormDefault="qualified"` — the only modelled schema besides `wml.xsd`
with that shape, confirmed directly off both the Strict and Transitional editions. So a real producer
writes `<m:chr m:val="…"/>`, never a bare `val`, exactly as WordprocessingML writes `w:type` and
`w:font`. `crate::support`'s `read_val` and `val_element` read and write that `m:`-prefixed spelling,
and getting it wrong would produce markup Word ignores.

## The layering tension, and how it is resolved

`CT_CtrlPr` (`m:ctrlPr`) is the trailing control-properties pass-through every object's own `*Pr` type
may carry, and the schema declares its content as `EG_RPrMath`: a `w:rPr` (`CT_RPr`), or a
`w:ins`/`w:del` (`CT_MathCtrlIns`/`CT_MathCtrlDel`, which extend `CT_TrackChange` and themselves carry
a `w:rPr`). **Every one of those is a WordprocessingML type**, and `shared-math.xsd` imports `wml.xsd`
for exactly this reason: the standard puts a `wml` type inside a `shared-math` type.

`mjx-omml` sits *below* `mjx-docx`, so it cannot name `mjx-docx`'s `RunProperties`, and mirroring the
schema's own direction would be the upward edge `CLAUDE.md` forbids.

The resolution is that [`ControlProperties`](crate::ControlProperties) preserves its children
**wholesale and raw** — the same mechanism `mjx-dml`'s `WordprocessingGroup` and
`WordprocessingCanvas` already use for their own WordprocessingML-typed member shapes, for the
identical reason. Every byte round-trips whether or not this crate understands what is inside, and
`mjx-docx` adds typed accessors **over** those raw children where it needs them:
`crates/mjx-docx/src/document/revisions.rs` holds its own `CT_MathCtrlIns`/`CT_MathCtrlDel` types and
reads them off [`ControlProperties::raw_children`](crate::ControlProperties::raw_children).

This is not a narrower feature than *"model `CT_CtrlPr`'s children"*. Every object here that carries a
`ctrlPr` exposes it as a typed `ControlProperties` whose own raw children are exactly as reachable —
by whoever can type them — as a fully decomposed field would be, and **fidelity does not depend on
which side of a crate boundary the decomposition happens on**.

## Fidelity

Every modelled type is a fidelity wrapper: the element's name with its source prefix, its attributes
in source order, its self-closing flag and every child it does not itself model, preserved verbatim,
with typed **read** accessors layered over that preserved state rather than a decomposed set of typed
fields to lose track of unknown content.

One mechanism backs all of it — `crate::support`'s `fidelity_element_impls!`, invoked directly on
eight types and through `crate::objects`' `fidelity_struct!` on thirty-eight more, forty-six in all.
`xtask/tests/upper_markup_ledger.rs` holds that one body to writing back every field its own reader
captured, and holds this crate to having no second mechanism.

An equation parsed and re-emitted without an edit is byte-identical. Editing one leaf — a
[`Text`](crate::Text)'s characters, several nesting levels deep — leaves every sibling at every level
untouched, through the standard copy-on-write machinery this project relies on everywhere else.
`crates/mjx-omml/tests/deep_nesting.rs` is where that is asserted rather than claimed.
