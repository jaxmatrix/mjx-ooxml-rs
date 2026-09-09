# Legacy VML

The last page of the upper shared markup's guide, whose other pages live in `mjx-chart` and whose
index is `crates/mjx-chart/docs/guide/README.md`. It is hosted here rather than there because
`mjx-chart` and `mjx-vml` are the **same layering rank**: neither may depend on the other, so a page in
that directory could not resolve a single intra-doc link into this crate — the same reason
`mjx_mce::guide` sits beside `mjx_opc::guide` rather than inside it.

## Read this first: VML's guarantee is weaker than every other crate's, and by how much

**A VML part is schema-validated one child at a time, and nothing derives its child order from an
XSD.**

Both halves have a cause, and the first half used to be stronger than it is:

* **A `.vml` part's root is a bare `<xml>` element in no namespace at all**, which none of the five
  VML schemas declares a global element for. `xmllint` pointed at the document reports *Element
  'xml': No matching global declaration available for the validation root* and gets no further, so
  the part as a whole cannot be validated. Its children can: `v:shape`, `v:shapetype`,
  `o:shapelayout`, `x:ClientData` and the rest are global elements, and `vml-main.xsd` imports its
  four siblings, so one driver over it reaches every child kind a producer writes.
* **No `vml-*` schema is in the child-order generator's `CHILD_ORDER_SCHEMAS`**, so nothing checks
  the *sequence* a drawing's children are written in — only that each child is itself well formed
  against the XSD. MJXOFF-264 owns that gap.

`crates/mjx-schema-gate/src/categories.rs` therefore files a VML part as a `WrapperRoot` — the
category whose parts are validated child by child — with that reason written on the entry itself.

Until MJXOFF-245 it was category 2, *markup this project preserves verbatim and never validates*, on
a reason with a third bullet: that `vml-main.xsd` could not compile at all without an `xml.xsd` the
Transitional set does not ship. That stopped being true when MJXOFF-134 gave every schema a
generated driver, and the entry kept saying it for two phases. `COVERAGE.md` still records all five
`vml-*` schemas as **not modelled** for simple types and for child order, and both remain literally
true: this crate never authors an `ST_*` value, and nothing re-sequences a VML part.

### What that means for a caller, concretely

| A `.pptx`, `.docx` or `.xlsx` part | What checks it |
|---|---|
| a slide, a document, a worksheet, a chart | `xmllint` against the ECMA schema, **plus** a generated child-order walk, **plus** the round trip |
| a VML drawing | `xmllint` against `vml-main.xsd`, **one child of the `<xml>` wrapper at a time**, **plus** the round trip — but **no** child-order walk |

So:

* **Reading and re-emitting is as safe here as anywhere.** The fidelity mechanism is the same one the
  rest of the workspace uses, it is exercised by `crates/mjx-vml/tests/drawing.rs`, and
  `xtask/tests/upper_markup_ledger.rs` holds this crate's one serialization mechanism to writing back
  every field it read.
* **Authoring or editing still carries a risk the other crates do not, and it is now a narrower
  one.** An attribute VML does not admit is caught: MJXOFF-245 put every child of an authored
  wrapper through `xmllint`, and `crates/mjx-pptx/tests/schema_validity.rs`'s
  `a_vml_child_that_breaks_the_schema_is_reported_invalid_naming_it` breaks one on purpose to prove
  the check is live. A shape written in the wrong *order* is not caught, because no VML schema has a
  child-order table (MJXOFF-264) — the first thing to notice would be Office. That is why
  `docs/validation/06-the-office-pass.md` exists and why the VML entries in it matter more than
  their size suggests.
* **Nothing about `.vml` is enumerated by a hand-written list any more, and that took a defect to
  arrange.** MJXOFF-114 found `mjx-opc`'s exception list of suffix-less XML content types carrying
  `…vmlDrawing` in Office's own capitalisation while `is_xml_content_type` folded its argument, so the
  entry matched nothing at all: **every authored `.vml` part sat outside `Package::validate` and every
  markup check from the day the list was written.** MJXOFF-221 found the same exact-match in this
  crate's own [`is_vml_content_type`](crate::is_vml_content_type) and in `mjx-schema-gate`'s two copies
  of the rule; all three now fold, and each has a test that fails against the old body.

## What VML is, and why the crate exists at all

VML (Vector Markup Language) is the legacy drawing markup carried in the **Transitional** flavour of
OOXML (ECMA-376 Part 4 §14.1, reference material §19) and dropped from **Strict**. Producers still
emit it, because some constructs have no DrawingML equivalent:

* an **OLE object**'s fallback picture,
* a **comment**'s authoring box,
* **ink**,
* legacy **form controls**.

It arrives two ways: as a standalone `vmlDrawingN.vml` part referenced by relationship id, and inline
inside a WordprocessingML `w:pict` or an `mc:Fallback` branch.

**A legacy construct is only useful if you can get from the modern markup that points at it to the
legacy shape that draws it.** That hop is an identifier match, and making it is what this crate is
for: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a
[`Shape::identifier`](crate::Shape::identifier), and
[`Drawing::shape_by_identifier`](crate::Drawing::shape_by_identifier) resolves it.

```rust
use mjx_vml::DrawingPart;

# fn main() -> Result<(), mjx_vml::VmlError> {
let part = DrawingPart::parse(
    br##"<xml xmlns:v="urn:schemas-microsoft-com:vml"
 xmlns:o="urn:schemas-microsoft-com:office:office">
 <v:shape id="_x0000_s1026" type="#_x0000_t202" style="width:100pt" filled="f"/>
</xml>"##,
)?;

let shape = part
    .drawing()
    .shape_by_identifier(part.interner(), "_x0000_s1026")
    .expect("the shape an OLE frame's spid names");
assert_eq!(shape.is_filled(part.interner()), Some(false));
# Ok(())
# }
```

**SpreadsheetML names a shape by the number alone** — `x:oleObject@shapeId`, `x:control@shapeId` and
`x:comment@shapeId` are all `xsd:unsignedInt` — while the shape itself carries the full
`_x0000_s1026` string. [`shape_identifier_for_number`](crate::shape_identifier_for_number) is the one
place that spelling is written, so the two halves of the hop cannot drift apart, and
[`Drawing::shape_by_numeric_identifier`](crate::Drawing::shape_by_numeric_identifier) is the lookup
that takes the number.

## Names come from the prose, never from the wire token

VML element names are cryptic even by OOXML's standards, so every type here is named from the
ECMA-376 Part 4 §19 prose: `v:shapetype` is a [`ShapeTemplate`](crate::ShapeTemplate) (§19.1.2.20
*shapetype (Shape Template)*), `o:idmap` is a [`ShapeIdMap`](crate::ShapeIdMap) (§19.2.2.14 *idmap
(Shape ID Map)*), `x:ClientData` is [`AttachedObjectData`](crate::AttachedObjectData) (§19.4.2.12
*ClientData (Attached Object Data)*). Each item's own docs name its wire element and its section, and
every enumeration variant records its exact wire token.

## Namespace prefixes matter here more than anywhere else

The fidelity reader resolves an *element's* namespace but leaves an *attribute's* prefix unresolved,
and VML is the one vocabulary where that bites: a `v:shape` carries an unprefixed `id` **and**
children with an `r:id`. So unprefixed attributes are matched exactly; a namespaced one is matched on
whatever prefix the element carrying it declares for that namespace, and otherwise on the conventional
prefix §19 binds it to in every example and that every producer emits — `v`, `o`, `p`, `x`, `w10` and
`r`. **If an element rebinds one of those to a different namespace, the lookup answers `None` rather
than matching the wrong attribute**, which is the safe direction.

## Fidelity, down to the whitespace between attributes

Every modelled type keeps the element's name with its prefix, its attributes in source order, its
self-closing flag, and every child it does not model. A drawing parsed and re-emitted without an edit
is byte-identical, and an edit to one shape leaves its siblings untouched.

That holds down to the **whitespace inside a start tag**, which matters here more than elsewhere:
Office wraps a VML start tag across lines far more often than it wraps a slide's, and a decomposed tree
does not record the whitespace *between* attributes. [`DrawingPart`](crate::DrawingPart) therefore
keeps the document it parsed — its source buffer and its parsed root — and an element a model rebuilt
without changing it is copied out of the original bytes rather than reconstructed. A wrapped start tag
comes back wrapped.

It costs the parsed tree alongside the typed model: a `DrawingPart` holds two copies of the markup.
A caller that already owns the part's `RawDocument` (through `mjx_opc::Package::part_tree_mut`) should
use [`Drawing::from_xml`](mjx_ooxml_core::FromXml::from_xml) and `write_back` directly, leave the
document in place, and pay nothing.

## What is modelled, and what stays raw

Sixteen types model an element: five through `#[derive(FromXml, ToXml)]`
([`Drawing`](crate::Drawing), [`Shape`](crate::Shape), [`ShapeGroup`](crate::ShapeGroup),
[`ShapeTemplate`](crate::ShapeTemplate), [`ShapeLayout`](crate::ShapeLayout)) and eleven through this
crate's own `fidelity_leaf!` — the leaves it addresses by their attributes while re-emitting their
subtrees verbatim ([`Fill`](crate::Fill), [`Stroke`](crate::Stroke), [`ImageData`](crate::ImageData),
[`TextBox`](crate::TextBox), [`ShapePath`](crate::ShapePath), [`Ink`](crate::Ink),
[`ShapeProtections`](crate::ShapeProtections), [`ShapeIdMap`](crate::ShapeIdMap),
[`EmbeddedOleObject`](crate::EmbeddedOleObject), [`DiagramText`](crate::DiagramText),
[`AttachedObjectData`](crate::AttachedObjectData)).

Every one of those eleven exposes [`raw`](crate::Fill::raw) and [`raw_mut`](crate::Fill::raw_mut),
which is how a caller reaches an attribute this crate does not name — deliberately, because VML has
hundreds and modelling all of them would buy nothing that `raw_mut` does not already buy.

`v:formulas`, `v:handles`, `v:shadow`, `v:textpath`, `w10:wrap` and every unknown extension stay in
`ShapeContent::Raw` and come back byte for byte.

## Two things this crate does not do

**It does not decide a colour from a theme.** VML has no theme inheritance at all, which is why
`#ffffe1` on a freshly authored comment box is the right value rather than a hard-coded one — there is
nothing for it to inherit from. That is the one place in this project where writing a literal colour
is correct, and MJXOFF-198 §6 records it as checked and cleared.

**It does not know what a part is.** [`DrawingPart::parse`](crate::DrawingPart::parse) takes bytes;
finding the part, registering its content-type Default and relating it from a slide, a header or a
worksheet is the format crate's job — `mjx_pptx::Presentation::add_vml_drawing` behind the `vml`
feature, and the comment-box path in `mjx-xlsx`.
