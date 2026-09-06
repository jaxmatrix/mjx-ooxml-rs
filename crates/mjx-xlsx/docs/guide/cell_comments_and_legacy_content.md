# Cell comments and the legacy content behind them

A cell comment in Excel is **two parts that only make sense together**, and they are not even in the
same vocabulary. The text is SpreadsheetML in `xl/commentsN.xml`; the pop-up box is *Transitional
VML* — a `v:shape` in `xl/drawings/vmlDrawingN.vml`, the drawing language OOXML kept for the
constructs DrawingML has no equivalent for. A comment with no box, or a box with no text, is a file
Excel opens, calls damaged and repairs by throwing one half away.

So this crate writes both halves in one call, removes both in one call, and **refuses to save a
workbook where only one of them is there**. That refusal is the guard; the surface does not merely
promise to get it right.

## Reading one

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

// Written by LibreOffice 25.8.7.3, not by this library.
let workbook = Workbook::open(&mjx_fixtures::fixture("cell_comments.xlsx"))?;

let comment = workbook
    .comment_at(0, CellReference::parse("A2")?)?
    .expect("the producer wrote a comment on A2");

assert_eq!(comment.author.as_deref(), Some("Unknown Author"));
assert_eq!(comment.text, "Checked against the ledger.\nSecond line.");

let drawn = comment.comment_box.as_ref().expect("its box");
assert!(drawn.is_visible);
assert_eq!(drawn.row, Some(1));
assert_eq!(drawn.column, Some(0));
# Ok(())
# }
```

Three things in that snippet are worth stating, because each is a place a plausible model goes
wrong.

**`author` is looked up, not stored.** `x:comment@authorId` is an *index* into the part's
`x:authors` list. A comment whose index points past the end of that list answers `None` — a fact
about the file, not a repair. Because the index is a public address, the author list is append-only:
removing an entry would renumber every later one and silently reattribute every comment that held
those numbers.

**`text` is the concatenation of every run.** A comment's `text` child is a `CT_Rst` — the same
complex type a shared string is — and a `CT_Rst` **carries no character data of its own**. The text
lives in a `t` child, or, when the author formatted part of it, in the `t` of each `r` run. A
decoder that read the element's own characters answers the empty string for every comment any
producer has ever written. Per-run fonts are not decoded; the markup is reached verbatim through
[`mjx_sml::CommentText::runs_markup`].

**The box's anchor is read, never inferred.** There are two anchors here and they are not the same
one. `x:commentPr/anchor` in the comments part is a `CT_ObjectAnchor` — the two `xdr:` cell markers
[`mjx_sml::ObjectAnchor`] models — and LibreOffice writes **no `commentPr` at all**, so for a large
share of real workbooks it does not exist. What Excel actually honours is
`x:ClientData/x:Anchor` *inside the VML shape*: a comma-separated string of eight numbers, with
`x:Row` and `x:Column` beside it naming the cell. `CommentBox::anchor_text` hands that string over
exactly as written rather than decoding it, because it is a different anchor vocabulary from the
sheet drawing's and a second decoder for it would be a model with two descriptions of one thing.

## Writing one

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_sml::CellReference;
use mjx_xlsx::Workbook;

let mut workbook = Workbook::blank()?;
let cell = CellReference::parse("B2")?;

// One call writes seven things: the comments part, its content type, its relationship, the VML
// drawing part, *its* content type, *its* relationship, and the sheet's `x:legacyDrawing` entry.
let shape_id = workbook.add_comment(0, cell, "Jai Shukla", "Check this against the ledger.")?;
assert_eq!(shape_id, 1025, "Excel's first generated shape id, and this crate's");

let comment = workbook.comment_at(0, cell)?.expect("the comment");
assert_eq!(comment.shape_id, Some(1025));
assert_eq!(
    comment.comment_box.as_ref().and_then(|drawn| drawn.identifier.as_deref()),
    Some("_x0000_s1025"),
);

// Changing the text leaves the box alone.
assert!(workbook.set_comment_text(0, cell, "Checked.")?);

// Removing takes both halves, and takes the comments part with the last comment.
assert!(workbook.remove_comment(0, cell)?);
assert!(workbook.comment_at(0, cell)?.is_none());
workbook.save()?;
# Ok(())
# }
```

`remove_comment` deletes the comments part when its last comment goes, because an `x:commentList`
with no `comment` in it is exactly the half-a-comment the validator refuses. It does **not** delete
the VML drawing part: that part also draws form controls and OLE fallbacks, and a sheet can easily
have one of those and no comments at all.

## The identifier hop, in Excel's spelling

A legacy construct is only useful if you can get from the modern markup that points at it to the
legacy shape that draws it, and that hop is an identifier match. PowerPoint's is a string:
`p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a VML shape's `id` directly.

**Excel's is a number.** `x:oleObject@shapeId`, `x:controls/control@shapeId` and
`x:comment@shapeId` are all `xsd:unsignedInt`, and the shape they name carries `_x0000_s` followed
by that number. That spelling — and the three attributes producers actually put it in — is stated
once, in [`mjx_vml::Drawing::shape_by_numeric_identifier`], so both formats reach one resolver
rather than two that could disagree.

Three lookups, in order of confidence, because producers do disagree:

| where the identifier is | who writes it that way |
|---|---|
| `@id` = `_x0000_s<number>` | Excel, and every file written to match it |
| `@o:spid` = `_x0000_s<number>` | LibreOffice, which puts the control's *name* in `@id` |
| `@id` = the bare number | a producer that writes the number and nothing else |

```
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use mjx_xlsx::Workbook;

// LibreOffice again: a form check box, exported as an `x:control` wrapped in an
// `mc:AlternateContent` requiring the `x14` namespace.
let workbook = Workbook::open(&mjx_fixtures::fixture("legacy_form_control.xlsx"))?;

let identifiers = workbook
    .with_vml_shape_for_form_control(0, 0, |shape, interner| {
        (
            shape.identifier(interner),
            shape.application_shape_identifier(interner),
        )
    })?
    .expect("the control's shape");

// `@shapeId` is 1001; the generated identifier is in `@o:spid`, not in `@id`.
assert_eq!(identifiers.0.as_deref(), Some("AcceptTerms"));
assert_eq!(identifiers.1.as_deref(), Some("_x0000_s1001"));
# Ok(())
# }
```

`with_vml_shape_for_ole_object` is deliberately the same name `mjx_pptx::Presentation` gives the
same hop. `with_vml_shape_for_form_control` is deliberately **not** PowerPoint's
`with_vml_shape_for_activex_control`: a `p:control` is an ActiveX control with a persisted COM
state, an `x:control` is one of Excel's own form controls whose properties are an
`x14:formControlPr` part, and naming them alike would claim an equivalence the schemas do not have.

## What this crate does not do with VML

| Not done | Why |
|---|---|
| Evaluate VML geometry | A `v:shape`'s identity, style, references, fill, stroke and children are typed; the `path` command string and `v:formulas` are verbatim. Turning them into a path is a rendering feature — the same non-goal `mjx-docx`'s guide records |
| Decode `x:ClientData/x:Anchor` into EMU | It is a different anchor vocabulary from the sheet drawing's `xdr:` markers, and this crate reports it rather than inventing a second decoder. `Workbook::sheet_anchor_bounds` resolves the *drawing's* anchors |
| Model `x14:formControlPr` | `xl/ctrlPropsN.xml` is rooted in a Microsoft extension namespace ECMA-376 does not define. `mjx-sml` holds the `control@r:id` that names it and nothing opens the part |
| Validate a `.vml` against a schema | A VML part's root is a bare `<xml>` in **no namespace**, which the VML schemas declare no global element for. It is a *named, pinned* skip in `mjx-schema-gate`, not a silent one — and under `xl/` it is admitted by one narrowly-keyed row rather than by relaxing the rule that catches unvalidated parts |
| Recalculate anything a comment says | The same rule this crate applies to filters, sorts, validations and formulas |

## The legacy-content surface, across the three formats

The row a caller most often wants: *does my format have the path the other one has?*

| Content | `mjx-pptx` | `mjx-docx` | `mjx-xlsx` |
|---|---|---|---|
| A typed `mjx_vml::Drawing` over a part | `with_vml_drawing` | `header_footer_vml_drawings` | `vml_drawing_markup` |
| Editing one | `edit_vml_drawing` | through the typed `Drawing` | `edit_vml_drawing_markup` |
| Authoring a VML part | `add_vml_drawing` | — | through `add_comment` |
| The identifier hop from an OLE object | `with_vml_shape_for_ole_object` | — (MJXOFF-131's `w:object`) | `with_vml_shape_for_ole_object` |
| The identifier hop from a control | `with_vml_shape_for_activex_control` (ActiveX) | — (MJXOFF-131's `w:control`) | `with_vml_shape_for_form_control` (a *form* control) |
| Cell comments | — (no such construct) | — (`w:comment` is a different feature) | `sheet_comments`, `add_comment`, `set_comment_text`, `remove_comment` |
| Is `mjx-vml` optional? | **yes**, the `vml` feature | no | no |

The last row is the one to read carefully, because it is two arrangements and not three.
`mjx-pptx` gates VML behind a feature because a PresentationML caller may open a thousand decks and
never meet an OLE fallback. `mjx-docx` does not, because Word headers are the primary place VML
still appears in the wild. `mjx-xlsx` follows `mjx-docx`, and the reason is stronger than either:
**there is no Excel comment without a box**, so a comment surface behind a feature flag would answer
half the truth by default.

[`mjx_sml::CommentText::runs_markup`]: https://docs.rs/mjx-sml/latest/mjx_sml/comments/struct.CommentText.html#method.runs_markup
[`mjx_sml::ObjectAnchor`]: https://docs.rs/mjx-sml/latest/mjx_sml/features/objects/struct.ObjectAnchor.html
[`mjx_vml::Drawing::shape_by_numeric_identifier`]: https://docs.rs/mjx-vml/latest/mjx_vml/struct.Drawing.html#method.shape_by_numeric_identifier
