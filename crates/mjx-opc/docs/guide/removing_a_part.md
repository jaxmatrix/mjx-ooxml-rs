# Removing a part

**Four methods on [`Package`](crate::Package) remove a part, they have genuinely different blast
radii, and choosing wrongly deletes content the caller never named.** That sentence is the reason
this page is the third of the set rather than the last: it is the one decision in this tier where the
cost of getting it wrong is somebody's file.

This is not hypothetical. **MJXOFF-209** was exactly this mistake. Three `mjx_docx::Document` edits
finished by calling [`remove_unreferenced_parts`](crate::Package::remove_unreferenced_parts) — the
package-wide sweep — to clean up after themselves. The sweep did what it says: it removed *every*
part nothing pointed at, including a part the producer had left orphaned before the file was ever
opened, and (combined with a missing percent-decode) a perfectly live image whose relationship target
it could not resolve. An edit about a header deleted a picture.

## The decision, in one table

| You want to | Call | Reaches |
|---|---|---|
| unwire one part yourself and remove exactly it | [`remove_part`](crate::Package::remove_part) | that part only |
| delete a thing and everything it alone owned | [`remove_part_cascading`](crate::Package::remove_part_cascading) | that part, plus what only it referenced |
| clean up after **your own** edit | [`remove_part_if_unreferenced`](crate::Package::remove_part_if_unreferenced) | the same, but **only if nothing still points at it** |
| garbage-collect the whole package, because a caller asked | [`remove_unreferenced_parts`](crate::Package::remove_unreferenced_parts) | **every orphan in the file, including the producer's** |

Read the fourth row as a warning rather than a feature. It is the only one of the four whose reach is
not bounded by the part you named.

## The rule that settles it

> **An edit removes what that edit stranded. Nothing else.**

This is Phase G's standing design rule — *supply a default only in the absence of the user's own,
never in place of it* — pointed at deletion instead of authoring. A file opened from disk keeps what
it came with, including its rubbish. An orphan the producer left is not yours to tidy: it costs a few
bytes, it is legal OPC, and removing it changes a file about something the caller never asked about.

So:

* **Inside an editing method**, the answer is
  [`remove_part_if_unreferenced`](crate::Package::remove_part_if_unreferenced), always. It is guarded
  — a part something still points at is left alone and the call is *not* an error — and its cascade
  is bounded by what the part you named alone held.
* **[`remove_unreferenced_parts`](crate::Package::remove_unreferenced_parts) belongs to a caller who
  asked for it**, never to an edit acting on its own initiative. `mjx_pptx::Presentation` offers it
  as a method of its own for exactly that reason: it is a thing a user of the library decides to do.

## Watching the difference happen

Two orphans, identical in every way except who made them. The scoped removal takes one; the sweep
takes both.

```
use mjx_opc::{Package, PartName, Relationship, TargetMode};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let slide = PartName::new("/ppt/slides/slide1.xml")?;
let mine = PartName::new("/ppt/media/my-edit.png")?;
let theirs = PartName::new("/ppt/media/producer-left-this.png")?;

let mut package = Package::open(&mjx_fixtures::fixture("sample.pptx"))?;
package.insert_part(&mine, "image/png", b"mine".to_vec())?;
package.insert_part(&theirs, "image/png", b"theirs".to_vec())?;

// Only one of them is wired up — so `theirs` is an orphan from this moment on, exactly as a
// producer's leftover would be in a file we opened.
package.add_relationship(
    Some(&slide),
    Relationship {
        id: "rId99".to_owned(),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
            .to_owned(),
        target: slide.relative_target(&mine),
        mode: TargetMode::Internal,
    },
)?;

// The edit: unwire what it added, then clean up after itself and nothing else.
assert!(package.remove_relationship(Some(&slide), "rId99")?);
let removed = package.remove_part_if_unreferenced(&mine)?;

assert_eq!(removed, vec![mine.clone()], "the edit took exactly what it stranded");
assert!(package.part_bytes(&mine).is_none());
assert!(
    package.part_bytes(&theirs).is_some(),
    "the producer's orphan is not this edit's business",
);

// The sweep is the other decision, and it is a caller's to make.
let swept = package.remove_unreferenced_parts()?;
assert!(swept.contains(&theirs), "the sweep takes the producer's orphan too: {swept:?}");
# Ok(())
# }
```

The assertion in the middle is the whole page. `crates/mjx-opc/tests/edit_surface.rs`'s
`removing_an_orphan_by_name_spares_the_orphan_the_producer_left` is the same claim as a gate, and
`crates/mjx-docx/tests/scoped_cleanup.rs` is the regression suite MJXOFF-209 left behind.

## What each one actually does

### `remove_part` — the primitive, and the only one that can leave the package broken

[`remove_part`](crate::Package::remove_part) removes the entry, its content-type `Override` and its
own `.rels` part. **It does not look at the graph in either direction.** Nothing scans for inbound
references, so a relationship elsewhere can be left pointing at nothing — after which
[`save`](crate::Package::save) refuses the package, because [`validate`](crate::Package::validate)
faults a target that resolves to no part. Nothing follows outbound targets either, so whatever the
part alone referenced is left behind.

That is correct behaviour for a primitive — the other three are built out of it — and it is the right
call exactly when you have already unwired the references yourself. It is the wrong call the moment
you are not sure.

### `remove_part_cascading` — downward, unguarded

[`remove_part_cascading`](crate::Package::remove_part_cascading) removes the part **unconditionally**,
then removes each part that part referenced *and that nothing else still references*, transitively,
returning every name it took in removal order.

Deleting a slide is the shape it exists for: the slide's notes slide holds a relationship *back* to
the slide, so leaving it behind leaves a dangling reference — while the media the rest of the deck
still shows must stay. Only `Internal` targets are followed (an `External` one names no part), an
unresolvable target is skipped rather than failing the removal, and a reference cycle terminates
because each part is considered once.

It is **not** guarded: it removes the part you named whether or not something still points at it. The
inbound reference — a `p:sldId`, the presentation's own relationship — is still yours to remove.

### `remove_part_if_unreferenced` — the same walk, guarded

[`remove_part_if_unreferenced`](crate::Package::remove_part_if_unreferenced) is
[`remove_part_cascading`](crate::Package::remove_part_cascading) behind a check: if anything in the
package still resolves to the part, or the part is not there at all, it removes nothing and returns
an empty vector. Neither case is an error.

Being unreferenced is decided by the same resolver the sweep walks with, so the two agree about what
an edge points at. **This is the one an edit should call**, and it is what the three `mjx_docx`
methods MJXOFF-209 fixed call now.

### `remove_unreferenced_parts` — the package-wide sweep

[`remove_unreferenced_parts`](crate::Package::remove_unreferenced_parts) computes the transitive
closure of parts reachable from the package root (`_rels/.rels`) and removes everything else,
returning the orphans in the order the package listed them.

It is conservative in the ways that matter: OPC-required roots such as the core properties and the
thumbnail survive because the root relationships name them; only `Internal` targets are followed; a
cycle terminates; and **control parts are never removed** — `[Content_Types].xml` is not a part, and
every `.rels` is spared, since a `.rels` describes an owner the walk decides on independently.

What it is *not* is scoped. Reachability is a property of the whole file, so the sweep cannot tell an
orphan your edit made from an orphan that was in the file when you opened it. That is not a defect to
be fixed; it is what a garbage collector is. It just means the decision to run one is the caller's.

## A resolver detail that has bitten once already

Reachability is only as good as the resolution of a relationship `Target` to a
[`PartName`](crate::PartName), and OPC targets are **IRIs**: a space in a file name is written
`image%20one.png` while the ZIP entry is `image one.png`. Before MJXOFF-209 nothing decoded them, so
such a target resolved to nothing, the part looked unreachable, and the sweep deleted a live image.

`crates/mjx-opc/src/percent.rs` is the fix, and its two asymmetries are worth knowing:

* **Decoding is for resolution only.** Nothing writes a decoded value back into a producer's markup —
  decode-then-encode is not the identity (`%2520` and `%20`, `%5F` and `%5f`, `%41` and `A` all
  re-encode differently), so normalising on write would change `.rels` bytes in files nobody asked us
  to touch.
* **A malformed escape is passed through, not rejected.** `100% margin.png` is a real file name; a
  `%` that does not introduce two hex digits is left exactly as found, so the part still resolves. The
  one thing refused is an escape that introduces a `/`, because that would silently turn one path
  segment into two.

`crates/mjx-opc/tests/percent_encoded_targets.rs` holds the whole story against
`tests/fixtures/percent_encoded_targets.docx`.
