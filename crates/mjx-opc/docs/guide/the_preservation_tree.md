# The preservation tree

Below the package sit two crates and one data structure. `mjx_ooxml_core::RawDocument` is a **lossless
DOM**: everything a part's bytes say, including the things a normal DOM throws away. `mjx-xml` is the
only place in the workspace that uses `quick-xml`, and it turns bytes into that tree and back.

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Nothing here is canonical. Single quotes, an odd attribute order, an entity spelled the long way,
// a comment, and an explicitly-empty element beside a self-closing one.
let xml = br#"<p:sld xmlns:p='urn:p' xmlns:a="urn:a"><!-- keep --><a:t>1 &#38; 2</a:t><a:x/><a:y></a:y></p:sld>"#;

let document = mjx_xml::fidelity::parse(xml)?;
assert_eq!(mjx_xml::fidelity::serialize_to_vec(&document), xml, "byte for byte");
# Ok(())
# }
```

## What the tree records that a DOM does not

`mjx_ooxml_core::RawName` keeps both the **literal prefix as written** and the resolved namespace URI
— the redundancy is the point, because a file may bind `w:` to the Strict namespace and re-emitting
the URI would rewrite markup nobody edited. `mjx_ooxml_core::RawAttribute` keeps its position in the
vector and its `mjx_ooxml_core::QuoteStyle`. Values and text are stored as **raw escaped bytes**:
nothing is unescaped on the way in, so `&#38;`, `&amp;` and a literal `&` in a CDATA section stay
distinguishable. `mjx_ooxml_core::RawNode` has a variant for each of text, CDATA, comment, processing
instruction, declaration and doctype, so none of them is a special case that got dropped. And
`mjx_ooxml_core::RawDocument` carries the prologue and epilogue — the bytes before and after the root,
including a trailing newline — plus whether the source began with a byte-order mark.

`crates/mjx-xml/src/fidelity/mod.rs`'s own unit tests pin each of those individually
(`attribute_order_quotes_and_entities_preserved`, `cdata_comment_and_pi`,
`namespace_prefixes_and_xmlns_order`, `trailing_newline_epilogue`, `byte_order_mark_preserved`), and
`crates/mjx-opc/tests/tree_roundtrip.rs` runs the whole of it over every XML part of every committed
fixture with no exception list.

## Subtree copy-on-write: the bytes, not the model

There is one class of property a decomposed tree **cannot** record — the whitespace *between* two
attributes, the whitespace before a `/>`. Office wraps a VML start tag across four lines, and a tree
that stores each attribute's name, value and quote separately has nowhere to put the line breaks.

So the tree does not try. The reader records, for every element, **the byte range it occupied in the
input**, and hands the input buffer to the document. An element still in the state the reader left it
in is not reconstructed at all: `mjx_xml::fidelity::serialize` copies its range and does not descend
into it. A lightly-edited part becomes mostly `memcpy`.

Two rules keep that honest, and both are worth knowing because they are what makes it safe rather
than merely fast:

* **The range is untrusted.** It is sliced fallibly and then checked against the element it claims to
  describe — the bytes must open with `<` and this element's qualified name, and close the way this
  element says it closes. Anything that does not check out is reconstructed instead. The failure mode
  is a reflow, never wrong bytes.
* **A rewritten element re-emits its namespace declarations, always.** A verbatim subtree carries
  prefixes but not the `xmlns:` that binds them, so a rewritten ancestor that pruned its declarations
  would silently unbind every descendant beneath it. That cannot happen, and structurally rather than
  by a special case: the reader keeps `xmlns` declarations as ordinary attributes in document order,
  and the writer emits every attribute an element holds without inspecting any of them. There is no
  code path that could decide a declaration is unused.

`crates/mjx-xml/tests/subtree_cow.rs` is the suite, and its first test is the namespace one — because
that is the one way subtree copy-on-write could corrupt a document that part-level copy-on-write
could not.

## Typed models are views, and `write_back` is why they stay cheap

`mjx_ooxml_core::FromXml` and `mjx_ooxml_core::ToXml` are the seam between the raw tree and a typed
model. The awkward fact about that seam is that a `to_xml` pass **rebuilds every element it looked
at** — and nearly all of them come back byte-for-byte the same, yet a rebuilt element is born with no
source range, so assigning it over the original (`*slot = value.to_xml(interner)`) throws away every
range underneath and the part reflows.

`mjx_ooxml_core::ToXml::write_back` is the fix, and
`mjx_ooxml_core::RawElement::replace_preserving_verbatim_source` is the mechanism: it copies the
original's range onto the rebuilt node **wherever the two compare equal** — same name, same
self-closing style, same attributes in the same order with the same quoting, and, all the way down,
the same children. That set of properties is exactly what an element's markup determines, so *equal*
really does mean *these bytes spell this element*. Where anything differs, the node keeps the `None` a
rebuild is born with, and so does every ancestor of it.

The practical rule for anyone writing a model: **edit through `write_back`, not by assignment.**
Assignment is correct and slow and reflows the file; `write_back` is correct and keeps the bytes.

## Two readers, and only one of them preserves

`mjx-xml` exposes two, and choosing the wrong one is silent:

* `mjx_xml::fidelity` — the byte-preserving parse/serialize above. **This is the one for document
  parts**, and the one the whole model is built on.
* `mjx_xml::Reader` — a small namespace-resolving pull reader that unescapes values and hands back
  owned events. It is used for the tiny OPC control parts (`[Content_Types].xml`, `_rels/*.rels`) and
  by the schema code generator. It is **not** byte-preserving.

`mjx_xml::text` is the bridge for a typed model that wants a decoded `String` — and it states its own
limit plainly, because the limit is real: `mjx_xml::text::escape_text` escapes **minimally**, only
`<` and `&`. Decode-then-re-encode is byte-identical when the source used the canonical spellings and
is *not* otherwise. A `&#65;` comes back as `A`. See
[The round-trip contract](the_round_trip_contract) for which types that reaches and which decline it
because of it.

## One resource limit, and where it is

`mjx_xml::fidelity::MAXIMUM_DEPTH` refuses a document nested more than 256 elements deep, with
`mjx_xml::XmlError::DepthLimit`. The reader itself is iterative and would build any depth; the cost is
paid by everyone who later *walks* the tree, and those walks are recursive because the data is —
`Drop` and `Clone` for `mjx_ooxml_core::RawNode` are compiler-generated, the serializer descends into
a dirty element, and `mjx_mce::resolve` descends the whole document. None of them takes an attacker's
bytes directly; all of them receive a tree this one function built out of them. One bound here bounds
every one of them.

256 is measured from both ends rather than guessed: the deepest part in the committed fixture corpus
is depth 13, and the shallowest measured stack overflow is depth 1,024 on a debug build on a 2 MiB
thread. The constant's own documentation carries both numbers, and raising it means redoing the
measurement.
