# The package

An OOXML file is a ZIP container of **parts** described by two kinds of control stream:
`[Content_Types].xml`, which says what each part *is*, and `_rels/*.rels`, which say what points at
what. [`Package`](crate::Package) models that and nothing more. It has never heard of a slide, a
paragraph or a cell.

```
use mjx_opc::{Package, PartName};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let package = Package::open(&mjx_fixtures::fixture("sample.pptx"))?;

// Every addressable part resolves to a content type, by an `Override` or by a `Default`.
let presentation = PartName::new("/ppt/presentation.xml")?;
assert_eq!(
    package.content_type_of(&presentation),
    Some("application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"),
);

// The package root's own relationships are where every walk of the file starts.
let root_rels = package.relationships_for(None).expect("every package has _rels/.rels");
assert!(root_rels.iter().any(|rel| rel.target.contains("presentation.xml")));
# Ok(())
# }
```

## Part names are not ZIP entry names

[`PartName`](crate::PartName) is an OPC part name: absolute, `/`-rooted, `/ppt/presentation.xml`. The
ZIP entry is the same string without the leading slash. The two are one method apart
([`zip_name`](crate::PartName::zip_name) and [`from_zip_name`](crate::PartName::from_zip_name)), and
mixing them up is the commonest way to look up a part that is definitely there.

A relationship `Target`, meanwhile, is neither: it is a **relative IRI** resolved against the
directory of the part whose `.rels` declared it, or against the package root for `_rels/.rels`. That
is [`PartName::resolve`](crate::PartName::resolve) and
[`PartName::resolve_from_root`](crate::PartName::resolve_from_root), and
[`relative_target`](crate::PartName::relative_target) goes back the other way when you are writing
one.

```
use mjx_opc::PartName;

# fn main() -> Result<(), mjx_opc::OpcError> {
let slide = PartName::new("/ppt/slides/slide1.xml")?;

// Resolved against the *directory* of the declaring part, not against the root.
assert_eq!(slide.resolve("../media/image1.png")?.as_str(), "/ppt/media/image1.png");

// And back again, which is the form a `.rels` file actually holds.
let image = PartName::new("/ppt/media/image1.png")?;
assert_eq!(slide.relative_target(&image), "../media/image1.png");

// The ZIP entry has no leading slash. Looking a part up by the wrong one finds nothing.
assert_eq!(slide.zip_name(), "ppt/slides/slide1.xml");
# Ok(())
# }
```

Because a target is an IRI, it may be **percent-encoded** — Office writes `image%20one.png` for a
file called `image one.png`. Resolution decodes; nothing ever writes a decoded value back. See
[Removing a part](removing_a_part), where the consequence of missing that once is on the record.

## Content types: two rules, one lookup

`[Content_Types].xml` holds `Default` rules keyed by extension and `Override` rules keyed by part
name, and an `Override` wins. [`content_type_of`](crate::Package::content_type_of) is that lookup;
[`insert_part`](crate::Package::insert_part) registers an `Override` **only if** the part does not
already resolve to the type you asked for through an existing `Default` — so inserting a second PNG
into a package that already declares `Default Extension="png"` adds no rule at all.

## Relationships

[`relationships_for`](crate::Package::relationships_for) takes `Option<&PartName>`, and the `None` is
the package root rather than an absence — `_rels/.rels` is the source of every walk. A
[`Relationship`](crate::Relationship) is an id, a type URI, a target and a
[`TargetMode`](crate::TargetMode); an `External` target names no part in the container at all, which
is why every graph walk in this crate filters on `Internal` first.

[`add_relationship`](crate::Package::add_relationship) edits the source's `.rels` tree *and* the
parsed navigation view in lock-step, synthesising a `.rels` part when the source has none.
[`external_relationships`](crate::Package::external_relationships) and
[`retarget_relationship`](crate::Package::retarget_relationship) are the pair a caller reaches for
when a document links out to the network and they would rather it did not.

## Saving is checked by default, and that is not negotiable

[`save`](crate::Package::save) runs [`validate`](crate::Package::validate) before it writes a byte,
and refuses a package that violates a packaging invariant:

* a part no content-type rule covers;
* a relationship whose `Internal` target names a part that is not in the container;
* two relationships from one source sharing an id;
* markup naming a relationship id its own `.rels` never declares.

None of those is visible to a per-part schema check, because none of them is a property of a part —
they are properties of the graph, and they are exactly what makes a consumer announce that it "found
a problem with the content". Making the check something a caller must remember is how such a fault
ships, so it is not optional.

The last of the four is scoped differently from the other three, and deliberately: markup checks run
only over [`authored_xml_parts`](crate::Package::authored_xml_parts) — the parts whose bytes *this
library produced*. A part still holding its container bytes re-emits verbatim, so faulting it would
mean refusing to write back a file we were given, which is the opposite of the promise. It also means
**reading a part can never change whether a package saves**.

[`save_unchecked`](crate::Package::save_unchecked) skips the pass. Reach for it only when writing an
inconsistent package is the point: re-saving a file that was already broken when it was opened
(refusing would lose it), or a deliberately intermediate state.

```
use mjx_opc::{Package, PartName};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut package = Package::open(&mjx_fixtures::fixture("sample.pptx"))?;

// Break the graph: the slide's relationship now points at a part that is not there.
package.remove_part(&PartName::new("/ppt/slides/slide1.xml")?)?;

let defect = package.save().expect_err("a dangling target must not be written");
assert!(matches!(defect, mjx_opc::OpcError::Invalid(_)), "{defect}");

// The escape hatch writes it anyway, and says so in its name.
assert!(!package.save_unchecked()?.is_empty());
# Ok(())
# }
```

## What is *not* in this crate

`mjx-opc` models the container. It does not know that `/ppt/presentation.xml` is a presentation, and
it will not stop you from building a package that is valid OPC and meaningless PresentationML. The
format crates own that: `mjx_pptx::Presentation`, `mjx_docx::Document`, `mjx_xlsx::Workbook`. Two
conveniences do lean toward OOXML without knowing which flavour —
[`Package::empty`](crate::Package::empty), which builds a container with the two `Default` rules every
package needs and an empty root `.rels`, and [`doc_props`](crate::doc_props), which reads and writes
the `docProps/core.xml` and `docProps/app.xml` every Office file carries.

[`ImageFormat::sniff`](crate::ImageFormat::sniff) is the other: it identifies an image from its magic
bytes rather than from a caller's claim about a file extension, so a media part gets the content type
its bytes deserve.
