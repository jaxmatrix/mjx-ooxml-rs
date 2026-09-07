# Guide

Five pages plus this index, in reading order — the same shape `mjx_pptx::guide`, `mjx_docx::guide`,
`mjx_xlsx::guide` and `mjx_ooxml::guide` carry. Those four describe a surface a caller *uses*. This
one describes the layer underneath all of them, **where this project's fidelity promise is actually
implemented**, and it covers four crates rather than one:

| Crate | Rank | What it is |
|---|---|---|
| `mjx-ooxml-core` | 0.0 | The interner and the lossless tree every part is held in |
| `mjx-xml` | 0.1 | The only place `quick-xml` is used: bytes ⇄ that tree |
| `mjx-opc` | 1.0 | The ZIP container, its parts, its content types and its relationship graph |
| `mjx-mce` | 1.0 | Markup Compatibility (`mc:AlternateContent`, `mc:Ignorable`) |

`mjx-opc` and `mjx-mce` are the **same rank**, so neither may depend on the other — `CLAUDE.md`'s
layering rule makes a sideways edge as illegal as an upward one. That is why the sixth page of this
set is hosted by its own crate rather than sitting in this directory: it is
`crates/mjx-mce/docs/markup_compatibility.md`, rendered as `mjx_mce::guide`. A link from here to it
would be an intra-doc link this crate cannot resolve, which is the layering rule showing through the
documentation rather than a gap in it.

| Page | Read it when |
|---|---|
| [The package](the_package) | You have container bytes and want the parts inside them |
| [Laziness and copy-on-write](laziness_and_copy_on_write) | You want to know what an open package costs, and what "lazy" does *not* mean |
| [Removing a part](removing_a_part) | **Before you call any method whose name begins `remove_`** |
| [The preservation tree](the_preservation_tree) | You are reading or editing markup rather than moving parts around |
| [The round-trip contract](the_round_trip_contract) | Before you rely on any of it — what is promised, what is enforced, and what is not |
| `mjx_mce::guide` | A part contains `mc:AlternateContent` and you need to know which branch you are seeing |

Every snippet on every page here is a compiled doctest that `cargo test` runs, and every one asserts
on a value it computed — the same rule the other four guides are held to, and what keeps a page from
drifting away from the API it describes.

## Who should be reading this

**Most callers should not.** An application opens a `.pptx`, `.docx` or `.xlsx` through
`mjx_ooxml::Deck`, `mjx_ooxml::Document` or `mjx_ooxml::Workbook` and never names a crate below the
facade — that is the whole point of `mjx_ooxml::guide::the_curated_surface`. This set is for two
readers:

* someone **working on this library**, for whom these four crates are the machine everything else
  rests on; and
* someone doing something the facade does not offer, who has reached through one of its three escape
  hatches (`mjx_ooxml::Deck::presentation_mut`, `mjx_ooxml::Document::document_mut`,
  `mjx_ooxml::Workbook::workbook_mut`) and on down to a [`Package`](crate::Package).

The second reader is why [Removing a part](removing_a_part) exists. A [`Package`](crate::Package)
will do exactly what you ask it to, including the thing you did not mean — and one of its four
removals reaches parts you never named.

## Four facts explain the tier

**A part is bytes until something needs otherwise.** [`Package::open`](crate::Package::open) reads a
container and produces a [`Package`](crate::Package) whose every part is
[`PartBody::Raw`](crate::PartBody::Raw) — decompressed bytes and nothing else. No XML is parsed until
[`part_tree`](crate::Package::part_tree) asks for it, and a part nobody asks about is written back on
[`save`](crate::Package::save) as the bytes it arrived as. That is the whole of the fidelity mechanism
at the package level. [Laziness and copy-on-write](laziness_and_copy_on_write) is where the two words
in *part-level laziness* are pulled apart, because only one of them is true.

**The tree is lossless, and it remembers where it came from.** `mjx_ooxml_core::RawDocument` keeps
attribute order, quote characters, namespace prefixes, self-closing style, entity spellings, comments
and the bytes before and after the root. Every element also records the byte range it was parsed
from, so an element nothing touched is re-emitted by copying that range rather than by being rebuilt
from the model — which preserves even the whitespace *inside* a start tag, something no decomposed
tree can record. See [The preservation tree](the_preservation_tree).

**Saving checks the graph first.** [`Package::save`](crate::Package::save) runs
[`validate`](crate::Package::validate) and refuses to write a package with a dangling relationship
reference, a target naming a part that is not there, or a part no content-type rule covers — the
faults that make Word offer to repair a file. [`save_unchecked`](crate::Package::save_unchecked) is
the deliberate escape hatch, for writing back a file that arrived broken. See
[The package](the_package).

**Nothing here knows what a slide is.** These four crates model the *container* and the *markup*, and
have no idea which of the three formats they are holding. `mjx-opc` has never heard of
PresentationML; `mjx-xml` has never heard of OOXML. That is what lets one implementation carry a
`.docx` and a `.xlsx` alike, and it is why the [round-trip contract](the_round_trip_contract) is a
statement about bytes rather than about features.
