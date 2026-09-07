# Markup compatibility

This is the sixth page of the packaging tier's guide, whose index is
`crates/mjx-opc/docs/guide/README.md`. It lives here rather than there because `mjx-opc` and
`mjx-mce` are the **same layering rank**, so neither may depend on the other and a link between them
would not resolve. The tier is `mjx-ooxml-core`, `mjx-xml`, `mjx-opc` and this crate.

## The problem MCE solves, and the one it makes

ECMA-376 Part 3 is how an OOXML producer ships a feature an older consumer has never heard of without
breaking it. Three mechanisms do the work, and all three arrive in files this library opens:

* **`mc:AlternateContent`** offers a list of `mc:Choice` branches, each declaring the namespace it
  `Requires`, and an optional `mc:Fallback`. A consumer takes the first `Choice` whose requirement it
  understands, or the `Fallback`.
* **`mc:Ignorable`** names namespaces a consumer may skip past if it does not know them — while
  `mc:ProcessContent` says to skip the *element* but keep its children.
* **`mc:MustUnderstand`** is the opposite: refuse the document rather than misread it.

The problem it makes for a library like this one is that **the file has more than one reading**, and
the two things you might want are incompatible. A renderer wants the branch a conforming consumer
would pick. A round-tripping editor wants every branch, untouched, including the one it does not
understand. This crate's answer is to make the second free and the first explicit.

## Preservation is the absence of code

There is nothing to call, and nothing that could go wrong. `mjx_xml::fidelity` parses `mc:` elements
and attributes into the same `mjx_ooxml_core::RawNode` tree as everything else, with no special case;
serialising that tree re-emits them; and an untouched part is copied out of its source buffer without
being walked at all. **Preservation *is* the untouched tree**, and it is covered by the same gate as
every other kind of markup — `crates/mjx-opc/tests/tree_roundtrip.rs`, which round-trips every XML part
of every committed fixture byte for byte.

`tests/fixtures/ole.pptx` and `tests/fixtures/legacy_form_control.xlsx` are real files carrying real
`mc:AlternateContent`, and both are in that corpus.

## Resolution is a view, and cannot mutate

[`resolve`](crate::resolve) takes `&RawDocument` and an
[`UnderstoodNamespaces`](crate::UnderstoodNamespaces), and returns a
[`ResolvedElement`](crate::ResolvedElement) whose children are flattened, whose `mc:*` and
ignored-namespace attributes are filtered out, and whose winning branch has been selected. Everything
it hands back **borrows** from the source tree.

That is the whole of the non-mutating guarantee, and it is worth noticing that it is not a promise
anyone has to keep: there is no `&mut` in the signature, so a serialize after a resolve is
byte-identical because it could not be anything else.

```
use mjx_mce::{resolve, UnderstoodNamespaces};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let xml = br#"<r xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:new="urn:new" xmlns:old="urn:old"><mc:AlternateContent><mc:Choice Requires="new"><new:shape/></mc:Choice><mc:Fallback><old:shape/></mc:Fallback></mc:AlternateContent></r>"#;
let document = mjx_xml::fidelity::parse(xml)?;

// A consumer that understands `urn:new` sees the Choice.
let modern = resolve(&document, &UnderstoodNamespaces::from_uris(["urn:new"]))?;
assert_eq!(modern.children.len(), 1);

// One that understands neither sees the Fallback — a different view of the same bytes.
let legacy = resolve(&document, &UnderstoodNamespaces::new())?;
assert_eq!(legacy.children.len(), 1);

// And the source is untouched, so what would be written is still the file that was read.
assert_eq!(mjx_xml::fidelity::serialize_to_vec(&document), xml);
# Ok(())
# }
```

The last assertion is the point of the whole crate: two readings, one file, and saving it gives back
the bytes that arrived.

## What `resolve` deliberately is not

**It is a content view, not a serialisation.** [`ResolvedNode`](crate::ResolvedNode) has three
variants — element, text and CDATA. Comments and processing instructions are omitted, because the
question this answers is *what content does a consumer see*, and neither is content. Do not build a
part out of a resolved view expecting to write it back; write back the source tree.

**It fails on two things, and only two.** [`ResolveError::MustUnderstand`](crate::ResolveError) when a
namespace the document insists on is not in the understood set, and
[`ResolveError::MalformedAlternateContent`](crate::ResolveError) for a block that is not well-formed
MCE — a `Choice` without a `Requires`, say. Anything else it can make sense of, it does.

**Prefixes are resolved through a scope stack, not through the parse.** MCE control attributes name
namespaces by *prefix* (`Requires="new"`), and a prefix means whatever the bindings in effect at that
element say it means. [`NamespaceScope`](crate::NamespaceScope) is that stack, rebuilt from the
`xmlns` attributes while descending — which is only possible because the tree kept those attributes as
ordinary attributes in document order rather than consuming them at parse time.

## Where MCE is handled by hand instead, and why

`CLAUDE.md` says *"MCE is handled in `mjx-mce`"*. That is true of resolution and is worth stating
precisely, because **two format crates walk MCE themselves rather than calling
[`resolve`](crate::resolve)**:

* `crates/mjx-pptx/src/slide.rs` declares this crate's namespace as a `SchemaNamespace` so its
  child-matching helpers can descend into `mc:AlternateContent` directly — an OLE object's `p:oleObj`
  is wrapped in it in real decks.
* `crates/mjx-xlsx/src/nav.rs` does the same, for the same reason: LibreOffice wraps a worksheet's
  `x:controls` in an `mc:AlternateContent` requiring `x14`, and its helper searches **both** the
  `Choice` and the `Fallback` branches, and walks through LibreOffice's doubled wrapper.

Those are reads that want an identifier which is the same in every branch, so choosing between
branches would be work with no answer to show for it — the helper's own documentation says so.
`crates/mjx-docx/src/document/headers.rs` is the one shipped site that genuinely resolves.

The consequence for a reader of this crate: [`resolve`](crate::resolve) is the *available* answer to
"which branch would a consumer take", not the answer the whole library routes through. If you are
adding a path that must pick a branch, call it; if you are matching a child whose identity is
branch-independent, the two format crates show the cheaper pattern.
