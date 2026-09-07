# The round-trip contract

**Open a file, edit one thing, save it: every part you did not touch comes back byte for byte.**

That is the promise this project exists to keep, and this page is where it is stated precisely enough
to be wrong. *"Preserves unknown content"* and *"round-trips faithfully"* are equally true of a
library that silently drops half a file, so every guarantee below names the type, the function or the
test that makes it true — and where nothing does, it says so.

## What is promised

**Per-part decompressed-payload byte identity, plus structural container identity.** Not identical
ZIP bytes: the compression level and the entry encoding are the container's business, not the
document's. What is fixed is the set of entries, their order, and the decompressed bytes of each.

```
use mjx_opc::Package;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let original = mjx_fixtures::fixture("charts.pptx");
let package = Package::open(&original)?;
let reopened = Package::open(&package.save()?)?;

// Structural identity: the same entries, in the same order.
let before: Vec<&str> = package.entries().iter().map(|e| e.name.as_str()).collect();
let after: Vec<&str> = reopened.entries().iter().map(|e| e.name.as_str()).collect();
assert_eq!(before, after);

// Payload identity: every part, byte for byte.
for entry in package.entries() {
    let name = mjx_opc::PartName::from_zip_name(&entry.name)?;
    assert_eq!(reopened.part_bytes(&name), entry.bytes(), "{} changed", entry.name);
}
# Ok(())
# }
```

`crates/mjx-opc/tests/roundtrip.rs`'s `round_trip_preserves_every_part_verbatim` is that over every
committed fixture, and it reads the corpus from `mjx_fixtures::package_fixtures` rather than a list
in the file — six of fifteen fixtures once sat outside the list that stood there.

## `CLAUDE.md`'s four fidelity rules, answered one at a time

The project's architecture document states four things about this tier. They are claims, and each
has an answer.

### 1 · "Parts stay raw bytes until first mutation; untouched parts re-emit verbatim; on first edit, serialize from the model and drop raw bytes"

**True, with one word to correct and one to sharpen.**

The word to correct is *lazy*. The laziness is in **parsing**, not in decompression:
[`Package::open`](crate::Package::open) inflates every entry eagerly. The word to sharpen is *drop*:
the whole-part buffer is dropped on first edit, but the tree keeps a source buffer of its own so that
untouched subtrees are still copied rather than rebuilt. Both are laid out in
[Laziness and copy-on-write](laziness_and_copy_on_write).

**What would fail if it stopped being true:**
`crates/mjx-opc/tests/edit_surface.rs`'s `reading_a_part_does_not_change_its_saved_bytes` (a read that
dirtied a part), `edit_one_part_every_other_byte_identical` (an edit that reached a second part),
`editing_one_attribute_leaves_the_other_subtrees_of_the_same_part_byte_identical` and
`reading_a_vml_part_does_not_reflow_it` (subtree copy-on-write giving up), and
`release_unused_part_sources_reclaims_only_a_fully_rewritten_part` (a buffer released while something
still pointed into it).

### 2 · "Every modeled complex type carries `extra: Vec<RawNode>` for unknown children, and preserves unknown attributes, attribute order, and namespace prefixes"

**The guarantee is substantially real and strongly enforced. The sentence is not accurate, and
"every" has exceptions.**

*The field name is one idiom of three.* A type keeps what it did not model in one of these shapes, and
searching the codebase for `extra` finds only the first:

* **`extra: Vec<RawNode>`** — the children a type declared no accessor for
  (`mjx_docx::SignedTwipsMeasureElement`).
* **`children: Vec<RawNode>` holding *all* children**, with typed accessors reading out of it on
  demand (`mjx_dml::Inline`). This preserves strictly more, because nothing was separated out in the
  first place.
* **A typed content vector with a `Raw(RawNode)` variant** (`mjx_docx::ParagraphContent`), so an
  unmodelled child keeps its *position* among its modelled siblings rather than being collected at the
  end.

*What holds it up is stronger than a convention.* For the third idiom it is **codegen**:
`mjx-derive`'s `#[xml(children, child(…))]` arm generates a `from_xml` that tries each declared
`(namespace, local)` arm and, for every node matching none of them, pushes a `Raw` variant —
unconditionally, with no way to invoke the arm without it. `#[derive(XmlAttributes)]` likewise never
rebuilds the attribute vector; it rewrites one attribute in place, which is what keeps order, quoting
and unknown attributes intact. **A single test failure therefore reaches every type that derives
them**, which is the opposite of per-fixture coverage:
`crates/mjx-derive/tests/derive.rs`'s `unknown_namespaced_child_preserved_as_raw` and
`container_round_trips_typed_child_and_raw`, and
`crates/mjx-derive/tests/attributes.rs`'s `markup_nobody_assigned_to_re_emits_byte_for_byte` and
`setting_every_modeled_attribute_leaves_the_unknown_one_where_it_was`.

*The exceptions, and they are the finding.* Three kinds of type sit outside all of that:

1. **`#[xml(text)]` leaves.** The derive's text arm reads only text and CDATA nodes and drops any
   other child, and writes a single minimally-escaped text node — so an entity spelling, a character
   reference, a CDATA section or an interleaved comment does not survive a rebuild. This is a
   **write-path** property of the derive, not of the reader: `mjx_xml::fidelity` never decodes text at
   all, and an untouched part carrying `&#38;` round-trips byte for byte with no derive involved
   (`crates/mjx-xml/tests/subtree_cow.rs`'s `an_untouched_document_round_trips_byte_for_byte`). Five
   types decline the derive because of it and hand-write the pair instead — `mjx_sml::DefinedName`,
   `mjx_sml::HeaderFooterText`, `mjx_sml::CommentAuthor` (the three `s:ST_Xstring` leaves, which share
   `crates/mjx-sml/src/leaf.rs`'s macro) and `mjx_sml::FormulaElement`, `mjx_sml::TableFormula` (the
   two formula ones). `crates/mjx-sml/tests/workbook_markup.rs`'s
   `an_entity_spelling_in_a_definition_survives_an_edit_elsewhere` is what pins the escape hatch.
   `mjx_dml::Text` — DrawingML's `a:t` — *does* use the derive and accepts the loss.
2. **Read-only projections.** `mjx_dml::Theme`, `mjx_dml::ColorScheme` and
   `mjx_dml::StyleMatrixReference` implement `FromXml` and **no** `ToXml`. They are views over an
   element, not wrappers around one, so nothing is ever written back through them and nothing can be
   lost — but each is a public type standing for a complex type with no bucket at all, so the claim's
   *"every modeled complex type"* is only true if it is read as *"every type that can be written
   back"*.
3. **One type that genuinely drops content.** `mjx_dml::Picture` and `mjx_dml::PictureNonVisual`
   hand-write `FromXml`/`ToXml` with no raw remainder, no `attributes` field, and a synthesised
   element name. `from_xml` reads three children by name and discards everything else; `to_xml`
   rebuilds with an empty attribute vector. An attribute on `<pic:pic>`, a foreign child beside the
   three, and the source's own prefix binding are all destroyed. This is **MJXOFF-216**, raised by
   MJXOFF-215's audit and owned by `mjx-dml`; it is **latent rather than live**, because no shipped write
   path reaches it — `mjx_dml::GraphicData`'s `ToXml` is its only caller, and the only shipped code
   that writes a `mjx_dml::Graphic` builds a fresh one for a chart. It becomes live the day anyone
   writes a picture-editing method on the typed value.

### 3 · "MCE is handled in `mjx-mce`, preserved on write and resolved (non-mutating) on read/render"

**Preservation: true by construction. Non-mutating: true by the type system. "Handled in `mjx-mce`":
true of resolution and not of everything.**

*Preserved on write* needs no code at all, and that is the strongest form the claim could take: the
stored tree already contains every `mc:*` node and attribute verbatim, so serialising it re-emits
them. Preservation **is** the untouched tree. It is covered by exactly the same gate as everything
else — `crates/mjx-opc/tests/tree_roundtrip.rs`.

*Non-mutating* is not a promise anyone has to keep: `mjx_mce::resolve` takes `&RawDocument` and
returns `mjx_mce::ResolvedNode` values that **borrow** from it. There is no `&mut` in the signature, so
a later serialize is byte-identical because it cannot be anything else. `crates/mjx-mce/tests/resolve.rs`
is the behavioural suite.

*Handled in `mjx-mce`* is the part to qualify. Exactly one shipped call site resolves —
`crates/mjx-docx/src/document/headers.rs` — and two format crates instead walk MCE **by hand**:
`crates/mjx-pptx/src/slide.rs` and `crates/mjx-xlsx/src/nav.rs` each declare their own `MCE` namespace
constant so their child-matching helpers can descend into `mc:AlternateContent` / `mc:Choice` /
`mc:Fallback` directly, because an OLE object's `p:oleObj` and a worksheet's `x:controls` arrive
wrapped in it. `mjx_xlsx`'s helper says outright that it does not resolve which branch a consumer would
pick. That is a defensible design — the identifier it wants is the same in every branch, and choosing
between them is a rendering decision — but it means "MCE is handled in `mjx-mce`" describes the
resolution, not the navigation. `crates/mjx-mce/docs/markup_compatibility.md` is the page for it.

### 4 · "Round-trip contract: per-part decompressed-payload byte identity + structural container identity"

**True, and the best-enforced of the four — at three different granularities.**

* **The container.** `crates/mjx-opc/tests/roundtrip.rs` —
  `round_trip_preserves_every_part_verbatim` over every committed fixture, with an anti-vacuity floor
  so a corpus that shrank to nothing cannot pass silently.
* **The tree.** `crates/mjx-opc/tests/tree_roundtrip.rs` —
  `every_xml_part_round_trips_byte_identical` parses and re-serialises every XML part of every fixture
  through `mjx_xml::fidelity`. There are no exceptions and no list of them.
* **The edit.** `crates/mjx-ooxml/tests/preservation/main.rs` — every committed fixture crossed with
  every mutating method of all three facade surfaces, each method declaring which classes of part it
  may add, change or remove. A difference matching no clause fails, and a reader declaring `NOTHING`
  must bring the package back part for part identical.

The third is the one with teeth, and its own vacuity traps are closed in both directions: the method
list is derived from the facade's source and compared with the registry both ways, the corpus is read
from `mjx-fixtures` and compared with what the sweep visited both ways.

## What the contract does *not* promise

**It is about parts, not about types.** Byte identity is asserted for a part that was not edited. Once
a part *is* edited, what survives is whatever subtree copy-on-write could keep — which is nearly
everything, and is not a guarantee about any particular element. A type that rebuilds differently
(finding 2.3 above) loses what it loses, and the container-level and tree-level gates cannot see it,
because they never edit anything.

**The edit-level gate is per fixture, not per type.** `preservation` would catch a type dropping
unknown content only if a committed fixture happens to carry that markup *and* a registered method
happens to edit it. The corpus is canonical Office and LibreOffice output, so a `<pic:pic>` in it
carries no unusual attribute and the diff shows nothing. **The mechanism-level derive tests are what
give per-type coverage, and a hand-written `FromXml`/`ToXml` pair is outside them by definition.**
That is the shape of the gap, and it is why every hand-written pair is worth a second look.

**Nothing is repaired and nothing is evaluated.** A file that arrives broken is written back broken
(that is what [`save_unchecked`](crate::Package::save_unchecked) is for); a formula's cached value is
never recomputed. Both are refusals, not omissions.

**Container bytes are not promised.** Deflate encodings vary between writers and between versions of
one writer. If you need to compare two saves, compare the parts —
[`Package::part_bytes`](crate::Package::part_bytes) — never the files.
