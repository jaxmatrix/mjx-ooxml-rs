# Laziness and copy-on-write

`CLAUDE.md` states the mechanism as *"parts stay raw bytes until first mutation; untouched parts
re-emit verbatim; on first edit, serialize from the model and drop raw bytes."* Every clause of that
is true, and **the word most readers supply for themselves — that opening a file is cheap because
little of it is read — is not.**

## What "lazy" means, and what it does not

**The laziness is in *parsing*, not in *decompression*.**
[`Package::open`](crate::Package::open) walks every ZIP entry and inflates all of it into RAM, right
then, with `read_to_end`. There is no streaming, no memory map and no on-demand entry. What is
deferred is the XML parse: a part arrives as [`PartBody::Raw`](crate::PartBody::Raw) — a `Vec<u8>` and
nothing else — and becomes a tree only when [`part_tree`](crate::Package::part_tree) or
[`part_tree_mut`](crate::Package::part_tree_mut) asks for one.

So the cost model of an open package is:

* **Decompressed size of the whole container, always.** Opening a 40 MB `.pptx` whose media inflates
  to 300 MB costs 300 MB whether you touch a slide or not.
* **Plus a tree for each part you actually read**, and nothing for the rest.

That distinction has a security consequence, and it is why it is stated here rather than left to be
inferred: **a small archive can expand without bound.** A ZIP entry's declared uncompressed size is
attacker-controlled and is only checked *after* the data has been read, so a container declaring four
gigabytes for four bytes of payload is trivially written — the fuzz campaign's first hostile container
is one, in 757 bytes. This crate no longer *reserves* from that figure
(`MAXIMUM_SPECULATIVE_PART_CAPACITY` caps the speculative allocation at one mebibyte and the buffer
grows as bytes actually arrive, which is what `tests/fixtures/declared_size_lie.zip` pins), so the
header can no longer be believed before the data backs it up. But there is still **no budget on the
total inflated size**, and a genuinely large compressed payload is genuinely inflated. That gap is
**MJXOFF-154**, and it is open.

## The three states of a part body

[`PartBody`](crate::PartBody) is the copy-on-write state machine, and it has three states rather than
two:

| State | Holds | What [`save`](crate::Package::save) writes |
|---|---|---|
| [`Raw`](crate::PartBody::Raw) | the decompressed bytes | those bytes, verbatim |
| [`Parsed`](crate::PartBody::Parsed) | those bytes **and** a cached tree | those bytes, verbatim |
| [`Edited`](crate::PartBody::Edited) | a tree only | the tree, re-serialised |

The middle state is the one that matters and the one a two-state model would miss. **Reading a part
never dirties it.** [`part_tree`](crate::Package::part_tree) parses, caches the tree for later reads,
and *keeps the original bytes* — shared with the tree rather than copied, so a part read as a tree
costs one buffer and not two — and [`save`](crate::Package::save) still re-emits them verbatim.

```
use mjx_opc::{Package, PartName};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let original = mjx_fixtures::fixture("sample.pptx");
let presentation = PartName::new("/ppt/presentation.xml")?;

let mut package = Package::open(&original)?;
let before = package.part_bytes(&presentation).expect("present").to_vec();

// Reading parses and caches a tree. It must not change what is written.
let _tree = package.part_tree(&presentation)?;

let reopened = Package::open(&package.save()?)?;
assert_eq!(
    reopened.part_bytes(&presentation).expect("present"),
    before.as_slice(),
    "reading a part changed its saved bytes",
);
# Ok(())
# }
```

`crates/mjx-opc/tests/edit_surface.rs`'s `reading_a_part_does_not_change_its_saved_bytes` is that
claim as a gate. It matters more than it looks: every typed model in this workspace is built by
*reading* parts, and if reading dirtied them, "an edit changes one part" would be false of every call
in the library.

## Three states, three questions

The third state has **no stored bytes**, and that is what makes
[`part_bytes`](crate::Package::part_bytes) a trap rather than a convenience: it answers `None` for a
part that is dirty exactly as it does for a part that is not in the package. Those are different
facts, and code that read them as one was wrong about a part it had itself just edited — `MJXOFF-222`
is two `from_package` constructors reporting a main part missing the moment a caller edited it.

So there is one call per question, and picking the wrong one is now a naming mistake rather than a
silent defect:

| The question | The call | `Raw` | `Parsed` | `Edited` | absent |
|---|---|---|---|---|---|
| is this part in the package? | [`contains_part`](crate::Package::contains_part) | `true` | `true` | `true` | `false` |
| what does this part contain? | [`part_payload`](crate::Package::part_payload) | the bytes | the bytes | the tree, serialised | `None` |
| does it still carry the bytes it arrived with? | [`part_bytes`](crate::Package::part_bytes) | `Some` | `Some` | `None` | `None` |

`part_payload` borrows when it can, so the ordinary case — every part of a file nobody has edited —
still costs no copy; only the dirty case allocates, and what it allocates is byte for byte what
[`save`](crate::Package::save) would write for that part.

**The third row is a fidelity question, not a content one.** A round-trip suite asking "did this part
survive untouched" wants it. Anything else almost certainly wants one of the first two.

```
use mjx_opc::{Package, PartName};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let presentation = PartName::new("/ppt/presentation.xml")?;
let absent = PartName::new("/ppt/slides/slide99.xml")?;
let mut package = Package::open(&mjx_fixtures::fixture("sample.pptx"))?;

package.part_tree_mut(&presentation)?;

// `part_bytes` cannot tell the edited part from the one that is not there.
assert!(package.part_bytes(&presentation).is_none());
assert!(package.part_bytes(&absent).is_none());

// The other two can.
assert!(package.contains_part(&presentation) && !package.contains_part(&absent));
assert!(package.part_payload(&presentation).is_some());
assert!(package.part_payload(&absent).is_none());
# Ok(())
# }
```

## "Drop raw bytes" is true at the part, and not at the subtree

[`part_tree_mut`](crate::Package::part_tree_mut) moves the body to
[`Edited`](crate::PartBody::Edited) and drops the whole-part `original` buffer — the bytes are now
stale, so keeping them would be keeping a lie. [`save`](crate::Package::save) re-serialises from the
tree.

**But the tree keeps a source buffer of its own**, and that is deliberate rather than an oversight.
`mjx_ooxml_core::RawDocument` retains the bytes it was parsed from, and every element records the byte
range it occupied, so any subtree the caller did not touch is still written by copying those bytes
rather than by being rebuilt. Copy-on-write does not stop at the part; it goes down to the subtree.
[The preservation tree](the_preservation_tree) is how.

The consequence for memory is that editing one attribute of a part does not free that part's bytes —
it moves the ownership from [`PartBody`](crate::PartBody) into the tree.
[`release_unused_part_sources`](crate::Package::release_unused_part_sources) is what reclaims it, and
it is honest about when it can: only for a part where **every** element has been rewritten, because
until then the buffer is still what most of the part will be written from.

```
use mjx_opc::{Package, PartName};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let drawing = PartName::new("/ppt/drawings/vmlDrawing1.vml")?;
let mut package = Package::open(&mjx_fixtures::fixture("vml.pptx"))?;

// One edit at the root. Every child can still be copied from the buffer, so nothing is reclaimable.
package.part_tree_mut(&drawing)?.root.attributes.clear();
assert_eq!(package.release_unused_part_sources(), 0);

// Replacing the root outright leaves nothing pointing into the buffer.
let tree = package.part_tree_mut(&drawing)?;
tree.root = tree.root.clone();
assert_eq!(package.release_unused_part_sources(), 1);
assert_eq!(package.release_unused_part_sources(), 0, "nothing left to release");

// And the part still saves, reconstructed from the model alone.
assert!(Package::open(&package.save()?)?.part_bytes(&drawing).is_some());
# Ok(())
# }
```

It walks every materialised tree, so it is a reclaim call for a long-lived editing session — not
something to do after each edit.

## Two more things a caller should expect

**[`replace_part_bytes`](crate::Package::replace_part_bytes) is the coarse door, and it is coarse on
purpose.** It stores new bytes [`Raw`](crate::PartBody::Raw) and drops any tree the part had. That is
the write path for a payload this library does not model — an embedded workbook, an image — where
re-serialising from a tree is not an option. For an XML part that *is* modelled, editing the tree
through [`part_tree_mut`](crate::Package::part_tree_mut) preserves far more, because everything the
edit did not reach is still copied from the source buffer.

**Provenance is a property of the bytes, not of your history with them.**
[`PartProvenance`](crate::PartProvenance) answers *where will the bytes being written come from* —
[`FromContainer`](crate::PartProvenance::FromContainer) for a part still holding what it was opened
with, [`Authored`](crate::PartProvenance::Authored) for one this library produced. Reading a part
never changes it; editing one does. That is what
[`validate`](crate::Package::validate) scopes its markup checks by, and it is the reason a save never
tokenises markup it was not going to re-serialise.
