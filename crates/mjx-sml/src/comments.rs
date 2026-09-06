//! Cell comments: the `xl/commentsN.xml` part, and the worksheet's edge to the legacy VML drawing
//! that draws their boxes.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | [`Comments`] | 273 | `x:comments` — the part root |
//! | [`CommentAuthors`] | 280 | `x:comments/authors` |
//! | [`CommentAuthor`] | — | `x:authors/author`, `s:ST_Xstring` |
//! | [`CommentList`] | 285 | `x:comments/commentList` |
//! | [`Comment`] | 290 | `x:commentList/comment` |
//! | [`CommentText`] | 1845 | `x:comment/text`, a `CT_Rst` |
//! | [`CommentProperties`] | 300 | `x:comment/commentPr` |
//! | [`LegacyDrawing`] | 2507 | `x:legacyDrawing`, rank **30** of `CT_Worksheet` |
//!
//! # A comment is two parts, and neither half means anything alone
//!
//! In the Transitional flavour an Excel comment is split across two parts that only make sense
//! together:
//!
//! * `xl/commentsN.xml` — this file's [`Comments`] — carries the **author list** and the **text**,
//!   keyed by cell reference;
//! * a `legacyDrawing` relationship names a **VML drawing part**, whose `v:shape` draws the box: its
//!   position, its size and whether it is visible.
//!
//! A comment part with no box, or a box with no text, is the defect the pairing invites, and it is
//! a *package* invariant rather than a markup one — so it is checked in `mjx_xlsx::Workbook`'s
//! validator, where both parts are in scope, and not here. What this file owns is the markup of the
//! first half plus [`LegacyDrawing`], the one element that names the second.
//!
//! **Nothing here opens the VML.** `mjx-vml` models a `v:shape`, and `mjx-sml` is rank 2.1 while
//! `mjx-vml` is rank 2.2 — the edge would point *upward*, which
//! `xtask/tests/layering.rs` refuses. The two halves are joined one tier up, in `mjx-xlsx`.
//!
//! # `commentPr@anchor` is `CT_ObjectAnchor`, and it is not modelled twice
//!
//! `CT_CommentPr` declares `anchor` of type `CT_ObjectAnchor`, `minOccurs="1"` — the *same* complex
//! type `CT_ObjectPr` (OLE objects) and `CT_ControlPr` (form controls) carry. MJXOFF-127 (D16)
//! modelled it once in [`crate::features::objects`] precisely so that the three consumers do not
//! invent three copies that then disagree about what `@moveWithCells` means, and
//! [`CommentProperties`] consumes it unchanged.
//!
//! Its two children are `xdr:from` and `xdr:to`, elements of **`dml-spreadsheetDrawing.xsd`**, held
//! raw and decoded by MJXOFF-107 (E3). See that module's own documentation.
//!
//! # The `true`-defaulting flag family, for the third time
//!
//! MJXOFF-127 recorded it for `CT_ObjectPr` and MJXOFF-107 for `CT_ControlPr`: a majority of the
//! boolean attributes on these three types default to **`true`**, which is the opposite of every
//! other flag family in `sml.xsd`. `CT_CommentPr` has six of them — `@locked`, `@defaultSize`,
//! `@print`, `@autoFill`, `@autoLine` and `@lockText` — against three that default to `false`
//! (`@disabled`, `@justLastX`, `@autoScale`). A reader that assumed the family-wide `false` would
//! report six of them backwards for an element that writes none, so each default is declared beside
//! its wire name and each getter returns its own.
//!
//! # Two anchors that are not the same anchor
//!
//! Worth stating because the names collide almost exactly and the two disagree in real files.
//!
//! * `x:commentPr/anchor` — [`CommentProperties::anchor`] — is `CT_ObjectAnchor`: two `xdr:` cell
//!   markers, each a column, a column offset, a row and a row offset.
//! * `x:ClientData/x:Anchor` **inside the VML shape** is a comma-separated string of eight numbers
//!   in the `urn:schemas-microsoft-com:office:excel` namespace, and it is the one Excel honours for
//!   the pop-up box. It belongs to `mjx-vml`, is reached through
//!   [`mjx_vml::AttachedObjectData`](https://docs.rs/mjx-vml), and **is read, never inferred**.
//!
//! `tests/fixtures/cell_comments.xlsx` is a file in which the first is absent altogether and only
//! the second exists: LibreOffice writes no `commentPr` at all. A model that computed a box's
//! position from `commentPr` would report nothing for every workbook that producer writes.
//!
//! # Nothing here draws, positions or authors a box
//!
//! `@moveWithCells` says what a *consumer* should do when the cells under a comment move;
//! `@autoScale` says whether it should rescale the text with the zoom. This crate reports every one
//! of them and acts on none — the rule [`crate::features`] states for filters, sorts and
//! validations, at a seventh door.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::{COMMENT, COMMENTS, COMMENT_PROPERTIES};
use mjx_ooxml_types::spreadsheetml::{
    CommentTextHorizontalAlignment, CommentTextVerticalAlignment,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellReference;
use crate::features::objects::ObjectAnchor;
use crate::leaf::{bag_without_declared_attributes, character_data_body, relationship_reference};
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// The worksheet's edge to the VML part
// -----------------------------------------------------------------------------------------------

bag_without_declared_attributes! {
    /// `x:legacyDrawing` (`CT_LegacyDrawing`, `sml.xsd:2507`) — the relationship to the legacy VML
    /// drawing part that draws a sheet's comment boxes, form controls and OLE fallbacks.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_LegacyDrawing`. Wire element: `legacyDrawing`, rank **30** of
    /// `CT_Worksheet`, rank 31 (as `legacyDrawingHF`) and the same slot on a chartsheet, a
    /// dialogsheet and a macrosheet.
    ///
    /// One attribute, `xsd:attribute ref="r:id" use="required"`. **The part it names is not this
    /// crate's:** `xl/drawings/vmlDrawingN.vml` is Transitional VML, whose model is `mjx-vml`'s —
    /// rank 2.2, one tier *above* this crate, so the resolution happens in `mjx-xlsx`. This type is
    /// the element, and holds the identifier as the string the file wrote.
    ///
    /// Structurally identical to [`SheetDrawing`](crate::SheetDrawing), and deliberately a distinct
    /// type: the two name different parts in different vocabularies, and a caller that confused them
    /// would hand a `.vml` to a DrawingML reader.
    LegacyDrawing, "legacyDrawing"
}

relationship_reference!(LegacyDrawing);

// -----------------------------------------------------------------------------------------------
// The author list
// -----------------------------------------------------------------------------------------------

character_data_body! {
    /// `x:author` — one entry of the comment part's author list, of type `s:ST_Xstring`.
    ///
    /// **`ST_`/`CT_` symbol:** none; `CT_Authors` declares the child inline as a simple type. Wire
    /// element: `author`.
    ///
    /// A [`Comment`]'s `@authorId` is an **index into this list**, so the list is ordered and an
    /// entry's position is a public address: removing one renumbers every later entry and silently
    /// reattributes every comment that held those numbers. [`CommentAuthors`] is therefore
    /// append-only, exactly as [`SharedStringTable`](crate::SharedStringTable) is and for the same
    /// reason.
    CommentAuthor, "author"
}

/// `x:authors` (`CT_Authors`, `sml.xsd:280`) — every author who has written a comment in this part.
///
/// **`ST_`/`CT_` symbol:** `CT_Authors`. Wire element: `authors`, rank **0** of `CT_Comments` and
/// `minOccurs="1"`, so a comments part without one is invalid markup.
///
/// One repeating child, so appending *is* placing and no ordering table is needed — the rule
/// [`crate::features::embedded`] states for `CT_OleObjects` and `CT_Controls`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CommentAuthors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "author", variant = Author, ty = CommentAuthor))]
    content: Vec<CommentAuthorsContent>,
}

/// One child of [`CommentAuthors`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentAuthorsContent {
    /// `x:author` — one name, `maxOccurs="unbounded"`.
    Author(CommentAuthor),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CommentAuthors {
    /// Builds an empty `x:authors`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "authors"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CommentAuthorsContent] {
        &self.content
    }

    /// Every author, in the order a `@authorId` indexes them.
    pub fn authors(&self) -> impl ExactSizeIterator<Item = &CommentAuthor> + '_ {
        self.content
            .iter()
            .filter_map(|item| match item {
                CommentAuthorsContent::Author(author) => Some(author),
                CommentAuthorsContent::Raw(_) => None,
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// The author `index` names, or `None` when the list is shorter than that.
    #[must_use]
    pub fn author(&self, index: u32) -> Option<&CommentAuthor> {
        self.authors().nth(usize::try_from(index).ok()?)
    }

    /// The index of the first entry whose text is exactly `name`, or `None`.
    ///
    /// An exact match, never a case-folded or trimmed one: an author list is a list of strings the
    /// producing application wrote, and two entries that differ only in case are two authors as far
    /// as this file is concerned.
    #[must_use]
    pub fn index_of(&self, name: &str) -> Option<u32> {
        self.authors()
            .position(|author| author.text() == name)
            .and_then(|position| u32::try_from(position).ok())
    }

    /// Appends `name` and answers the index a comment must carry to claim it.
    ///
    /// **Append-only**, for the reason [`CommentAuthor`] states: an entry's position is the address
    /// every `@authorId` in the part holds.
    pub fn push_author(&mut self, interner: &mut Interner, name: &str) -> u32 {
        let index = u32::try_from(self.authors().len()).unwrap_or(u32::MAX);
        let prefix = self
            .name
            .prefix
            .map(|symbol| interner.resolve(symbol).to_owned());
        let author = CommentAuthor::new(interner, prefix.as_deref(), name);
        self.content.push(CommentAuthorsContent::Author(author));
        self.empty = false;
        index
    }

    /// The index of `name`, appending it first if the list does not already carry it.
    pub fn index_for(&mut self, interner: &mut Interner, name: &str) -> u32 {
        match self.index_of(name) {
            Some(index) => index,
            None => self.push_author(interner, name),
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CommentAuthorsContent::Author(author) => RawNode::Element(author.as_raw_element()),
                CommentAuthorsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

// -----------------------------------------------------------------------------------------------
// One comment
// -----------------------------------------------------------------------------------------------

/// `x:text` — a comment's rich text.
///
/// **`ST_`/`CT_` symbol:** `CT_Rst` (`sml.xsd:1845`), the same complex type a shared-string entry
/// and an inline string are — `t?`, `r*`, `rPh*`, `phoneticPr?`.
///
/// # Why this is not [`StringItem`](crate::StringItem), and why it is not a plain text leaf either
///
/// [`StringItem`] is a *view into a packed store*: MJXOFF-97 built it so that a workbook with a
/// million shared strings costs 48 bytes an entry rather than a `RawElement` tree, and the store is
/// addressed by index and owns the part's whole buffer. A comment part carries a handful of entries,
/// is reached one comment at a time by cell reference, and has to sit inside a `RawElement`-backed
/// [`Comment`] whose siblings are typed models — so the store's shape buys nothing here and its
/// addressing is wrong.
///
/// It is not a [`character_data_body!`](crate::leaf::character_data_body) leaf either, and the
/// difference is the one a naive reader gets wrong: **a `CT_Rst` carries no character data of its
/// own.** `<text>hello</text>` is invalid markup; the text lives in a `t` child, or — when the
/// author formatted part of it — in the `t` of each `r` run. A decoder that read the element's own
/// character data answers the empty string for every comment any producer has ever written.
///
/// # What [`text`](Self::text) answers, and what it gives up
///
/// The **displayed string**: the plain `t` if there is one, then every run's `t`, concatenated in
/// document order. Phonetic runs (`rPh`) are *readings printed above* the base text rather than part
/// of it, so they are excluded — the same distinction [`crate::strings`] draws.
///
/// What that gives up is the per-run font. Nothing in this workspace renders a comment's runs, and a
/// second `CT_RElt` decoder would be a model with no reader; the markup is reached verbatim through
/// [`runs_markup`](Self::runs_markup) instead. Every byte the file wrote is preserved either way —
/// runs, phonetic markup, `xml:space` and all — until [`set_text`](Self::set_text) replaces the
/// content, which is exactly what replacing a comment's text means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentText {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    /// The displayed string, decoded — what [`text`](Self::text) answers with.
    text: String,
    /// Every child the file wrote, or the single authored `t` element that replaced them.
    children: Vec<RawNode>,
    /// Whether [`children`](Self::children) are still the file's own.
    preserved: bool,
}

impl CommentText {
    /// The wire local name this type is written under: `text`.
    pub const WIRE_LOCAL: &'static str = "text";

    /// Builds an `x:text` holding `text` as a single `t` child, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `xml:space="preserve"` is written exactly when dropping it would change the string — the same
    /// rule [`crate::strings`] authors a `t` by, because it is the same element.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        let mut value = Self {
            name: crate::leaf::sml_name(interner, prefix, Self::WIRE_LOCAL),
            attributes: Vec::new(),
            empty: false,
            text: String::new(),
            children: Vec::new(),
            preserved: false,
        };
        value.set_text(interner, text);
        value
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// The displayed string: the plain `t`, then each run's `t`, concatenated — with entity
    /// references decoded and nothing else changed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the whole text with a single `t` child, giving up every run and every byte the file
    /// wrote for this element's content.
    ///
    /// The element's own name, its attributes, their order and their quoting are untouched.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        let prefix = self
            .name
            .prefix
            .map(|symbol| interner.resolve(symbol).to_owned());
        let mut markup = Vec::new();
        crate::strings::write_text_element(&mut markup, prefix.as_deref(), text);
        let element = crate::leaf::parse_into(&markup, interner)
            .expect("a `t` element this crate serialized is well-formed");
        self.text = text.to_owned();
        self.children = vec![RawNode::Element(element)];
        self.preserved = false;
        self.empty = false;
    }

    /// The element's children exactly as the file wrote them — its `t`, its `r` runs, its phonetic
    /// markup — or `None` once [`set_text`](Self::set_text) has replaced them.
    ///
    /// The escape hatch for a caller that needs the run structure this type does not decode. Reading
    /// it changes nothing.
    #[must_use]
    pub fn runs_markup(&self) -> Option<&[RawNode]> {
        self.preserved.then_some(self.children.as_slice())
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl FromXml for CommentText {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text: crate::leaf::decoded_rich_text(element, interner)?,
            children: element.children.clone(),
            preserved: true,
        })
    }
}

impl ToXml for CommentText {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:commentPr` (`CT_CommentPr`, `sml.xsd:300`) — how the box that draws a comment behaves.
///
/// **`ST_`/`CT_` symbol:** `CT_CommentPr`. Wire element: `commentPr`, rank **1** of `CT_Comment`
/// and `minOccurs="0"` — LibreOffice writes none at all, so a reader that required one would report
/// nothing for every workbook that producer saves.
///
/// Twelve attributes. **Six of the nine booleans default to `true`** — `@locked`, `@defaultSize`,
/// `@print`, `@autoFill`, `@autoLine` and `@lockText` — which is the opposite of every other flag
/// family in `sml.xsd`; see the [module documentation](self). The declared defaults are the
/// schema's and each getter returns its own.
///
/// Its one child is the `CT_ObjectAnchor` MJXOFF-127 modelled once, in
/// [`crate::features::objects`]. It is **not** the anchor Excel honours for the pop-up box — that
/// one is inside the VML shape; again, see the module documentation.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "locked", codec = OnOff, accessor = is_locked, default = true))]
#[xml(attribute(local = "defaultSize", codec = OnOff, accessor = has_default_size, default = true))]
#[xml(attribute(local = "print", codec = OnOff, accessor = is_printed, default = true))]
#[xml(attribute(local = "disabled", codec = OnOff, accessor = is_disabled, default = false))]
#[xml(attribute(local = "autoFill", codec = OnOff, accessor = fills_automatically, default = true))]
#[xml(attribute(local = "autoLine", codec = OnOff, accessor = outlines_automatically, default = true))]
#[xml(attribute(local = "altText", codec = Text, accessor = alternative_text))]
#[xml(attribute(local = "textHAlign", codec = Enumeration<CommentTextHorizontalAlignment>, accessor = horizontal_alignment, default = CommentTextHorizontalAlignment::Left))]
#[xml(attribute(local = "textVAlign", codec = Enumeration<CommentTextVerticalAlignment>, accessor = vertical_alignment, default = CommentTextVerticalAlignment::Top))]
#[xml(attribute(local = "lockText", codec = OnOff, accessor = locks_text, default = true))]
#[xml(attribute(local = "justLastX", codec = OnOff, accessor = justifies_last_line, default = false))]
#[xml(attribute(local = "autoScale", codec = OnOff, accessor = scales_automatically, default = false))]
pub struct CommentProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "anchor", variant = Anchor, ty = ObjectAnchor))]
    content: Vec<CommentPropertiesContent>,
}

/// One child of [`CommentProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentPropertiesContent {
    /// `x:anchor` (rank 0) — where the box is pinned. `minOccurs="1"`.
    Anchor(ObjectAnchor),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CommentProperties {
    /// Builds an empty `x:commentPr`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `anchor` `minOccurs="1"`, so properties with no anchor are invalid
    /// markup; this is still constructible, because a caller builds one and then sets the anchor.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "commentPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CommentPropertiesContent] {
        &self.content
    }

    /// `x:anchor` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn anchor(&self) -> Option<&ObjectAnchor> {
        self.content.iter().find_map(|item| match item {
            CommentPropertiesContent::Anchor(anchor) => Some(anchor),
            CommentPropertiesContent::Raw(_) => None,
        })
    }

    /// `x:anchor`, mutably.
    pub fn anchor_mut(&mut self) -> Option<&mut ObjectAnchor> {
        self.content.iter_mut().find_map(|item| match item {
            CommentPropertiesContent::Anchor(anchor) => Some(anchor),
            CommentPropertiesContent::Raw(_) => None,
        })
    }

    /// Sets `x:anchor`, replacing the existing one where it is or inserting at rank 0.
    pub fn set_anchor(&mut self, anchor: ObjectAnchor) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, CommentPropertiesContent::Anchor(_)))
        {
            *item = CommentPropertiesContent::Anchor(anchor);
            return;
        }
        let at = COMMENT_PROPERTIES.insert_index_of_names(
            self.content.iter().map(CommentPropertiesContent::rank),
            "anchor",
        );
        self.content
            .insert(at, CommentPropertiesContent::Anchor(anchor));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CommentPropertiesContent::Anchor(anchor) => {
                    RawNode::Element(anchor.as_raw_element())
                }
                CommentPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl CommentPropertiesContent {
    /// This child's rank in `CT_CommentPr`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Anchor(_) => COMMENT_PROPERTIES.rank_of(None, "anchor"),
            Self::Raw(_) => None,
        }
    }
}

/// `x:comment` (`CT_Comment`, `sml.xsd:290`) — one comment: the cell it is attached to, who wrote
/// it, what it says, and how its box behaves.
///
/// **`ST_`/`CT_` symbol:** `CT_Comment`. Wire element: `comment`, the only child of
/// `CT_CommentList`.
///
/// `@ref` is `ST_Ref` and `use="required"`: the cell — or, the schema permits, the range — the
/// comment is attached to. `@authorId` is `use="required"` and is an **index into the part's
/// [`CommentAuthors`]**, not a name.
///
/// `@shapeId` is `use="optional"` and names the `v:shape` in the sheet's legacy VML drawing that
/// draws this comment's box — the Excel spelling of the hop `p:oleObj@spid` makes in PresentationML.
/// It is a number here and a string there: Excel writes `shapeId="1025"` beside a shape whose `id`
/// is `_x0000_s1025`. `mjx_xlsx::Workbook` walks it; this crate resolves nothing.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "ref", codec = Enumeration<CellReference>, accessor = cell, required))]
#[xml(attribute(local = "authorId", codec = Number<u32>, accessor = author_index, required))]
#[xml(attribute(local = "guid", codec = Text, accessor = guid))]
#[xml(attribute(local = "shapeId", codec = Number<u32>, accessor = shape_id))]
pub struct Comment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "text", variant = Text, ty = CommentText),
        child(local = "commentPr", variant = Properties, ty = CommentProperties)
    )]
    content: Vec<CommentContent>,
}

/// One child of [`Comment`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentContent {
    /// `x:text` (rank 0) — the comment's rich text. `minOccurs="1"`.
    Text(CommentText),
    /// `x:commentPr` (rank 1) — how the box behaves. `minOccurs="0"`.
    Properties(CommentProperties),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl Comment {
    /// Builds a comment on `cell` by author `author_index`, holding `text`.
    ///
    /// The `text` child is placed at rank 0 through the generated table, as
    /// [`set_properties`](Self::set_properties) places `commentPr` at rank 1.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        prefix: Option<&str>,
        cell: CellReference,
        author_index: u32,
        text: &str,
    ) -> Self {
        let body = CommentText::new(interner, prefix, text);
        let mut comment = Self {
            name: crate::leaf::sml_name(interner, prefix, "comment"),
            attributes: Vec::new(),
            empty: false,
            content: vec![CommentContent::Text(body)],
        };
        comment.set_cell(interner, cell);
        comment.set_author_index(interner, author_index);
        comment
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CommentContent] {
        &self.content
    }

    /// `x:text` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn text(&self) -> Option<&CommentText> {
        self.content.iter().find_map(|item| match item {
            CommentContent::Text(text) => Some(text),
            _ => None,
        })
    }

    /// `x:text`, mutably.
    pub fn text_mut(&mut self) -> Option<&mut CommentText> {
        self.content.iter_mut().find_map(|item| match item {
            CommentContent::Text(text) => Some(text),
            _ => None,
        })
    }

    /// Replaces the comment's text, adding the `x:text` child if there is none.
    pub fn set_text(&mut self, interner: &mut Interner, text: &str) {
        if let Some(existing) = self.text_mut() {
            existing.set_text(interner, text);
            return;
        }
        let prefix = self
            .name
            .prefix
            .map(|symbol| interner.resolve(symbol).to_owned());
        let body = CommentText::new(interner, prefix.as_deref(), text);
        let at =
            COMMENT.insert_index_of_names(self.content.iter().map(CommentContent::rank), "text");
        self.content.insert(at, CommentContent::Text(body));
        self.empty = false;
    }

    /// `x:commentPr` — `None` for a comment that writes none, which is most of them.
    #[must_use]
    pub fn properties(&self) -> Option<&CommentProperties> {
        self.content.iter().find_map(|item| match item {
            CommentContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// `x:commentPr`, mutably.
    pub fn properties_mut(&mut self) -> Option<&mut CommentProperties> {
        self.content.iter_mut().find_map(|item| match item {
            CommentContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// Sets `x:commentPr`, replacing the existing one where it is or inserting it at rank 1.
    pub fn set_properties(&mut self, properties: CommentProperties) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, CommentContent::Properties(_)))
        {
            *item = CommentContent::Properties(properties);
            return;
        }
        let at = COMMENT
            .insert_index_of_names(self.content.iter().map(CommentContent::rank), "commentPr");
        self.content
            .insert(at, CommentContent::Properties(properties));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CommentContent::Text(text) => RawNode::Element(text.as_raw_element()),
                CommentContent::Properties(properties) => {
                    RawNode::Element(properties.as_raw_element())
                }
                CommentContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl CommentContent {
    /// This child's rank in `CT_Comment`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Text(_) => COMMENT.rank_of(None, "text"),
            Self::Properties(_) => COMMENT.rank_of(None, "commentPr"),
            Self::Raw(_) => None,
        }
    }
}

// -----------------------------------------------------------------------------------------------
// The list, and the part
// -----------------------------------------------------------------------------------------------

/// `x:commentList` (`CT_CommentList`, `sml.xsd:285`) — every comment in this part, in the order the
/// file lists them.
///
/// **`ST_`/`CT_` symbol:** `CT_CommentList`. Wire element: `commentList`, rank **1** of
/// `CT_Comments` and `minOccurs="1"`.
///
/// One repeating child, so appending *is* placing and no ordering table is needed.
///
/// **The order is the file's, not a sort.** Excel writes comments in row-major cell order and
/// LibreOffice does not always; nothing here reorders them, because the order is not addressable —
/// a comment is found by its `@ref`, and rewriting the list into a canonical order would change
/// every byte of a part nobody asked to change.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CommentList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "comment", variant = Comment, ty = Comment))]
    content: Vec<CommentListContent>,
}

/// One child of [`CommentList`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentListContent {
    /// `x:comment` — one comment, `maxOccurs="unbounded"`.
    Comment(Comment),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CommentList {
    /// Builds an empty `x:commentList`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "commentList"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CommentListContent] {
        &self.content
    }

    /// Every comment, in the order the file lists them.
    pub fn comments(&self) -> impl Iterator<Item = &Comment> + '_ {
        self.content.iter().filter_map(|item| match item {
            CommentListContent::Comment(comment) => Some(comment),
            CommentListContent::Raw(_) => None,
        })
    }

    /// Every comment, mutably.
    pub fn comments_mut(&mut self) -> impl Iterator<Item = &mut Comment> + '_ {
        self.content.iter_mut().filter_map(|item| match item {
            CommentListContent::Comment(comment) => Some(comment),
            CommentListContent::Raw(_) => None,
        })
    }

    /// How many comments the list carries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.comments().count()
    }

    /// Whether the list carries no comment at all — which is what a part left behind by a delete
    /// looks like, and is why `mjx_xlsx::Workbook` removes the part rather than leaving it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.comments().next().is_none()
    }

    /// Appends `comment`.
    pub fn push(&mut self, comment: Comment) {
        self.content.push(CommentListContent::Comment(comment));
        self.empty = false;
    }

    /// Removes and returns the comment at `index` **among the comments**, ignoring anything this
    /// type does not model.
    pub fn remove(&mut self, index: usize) -> Option<Comment> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, CommentListContent::Comment(_)))
            .nth(index)
            .map(|(at, _)| at)?;
        match self.content.remove(at) {
            CommentListContent::Comment(comment) => Some(comment),
            other => {
                self.content.insert(at, other);
                None
            }
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CommentListContent::Comment(comment) => RawNode::Element(comment.as_raw_element()),
                CommentListContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

/// `x:comments` (`CT_Comments`, `sml.xsd:273`) — the root of an `xl/commentsN.xml` part.
///
/// **`ST_`/`CT_` symbol:** `CT_Comments`. Wire element: `comments`, the global element
/// `sml.xsd:272` declares.
///
/// Three children: [`CommentAuthors`] (`minOccurs="1"`), [`CommentList`] (`minOccurs="1"`) and an
/// `extLst` held raw. Both required children are still `Option` on the reading side, because a file
/// that omits one is a file this library must be able to open and re-emit — refusing to read a
/// workbook over a malformed comment part would be the repair-on-read this project does not do.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct Comments {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "authors", variant = Authors, ty = CommentAuthors),
        child(local = "commentList", variant = List, ty = CommentList)
    )]
    content: Vec<CommentsContent>,
}

/// One child of [`Comments`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentsContent {
    /// `x:authors` (rank 0) — the author list. `minOccurs="1"`.
    Authors(CommentAuthors),
    /// `x:commentList` (rank 1) — the comments. `minOccurs="1"`.
    List(CommentList),
    /// Anything else — the `extLst` at rank 2, and any foreign markup — verbatim, in position.
    Raw(RawNode),
}

impl Comments {
    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every attribute the root carries, in source order — the namespace declarations included.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CommentsContent] {
        &self.content
    }

    /// `x:authors` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn authors(&self) -> Option<&CommentAuthors> {
        self.content.iter().find_map(|item| match item {
            CommentsContent::Authors(authors) => Some(authors),
            _ => None,
        })
    }

    /// `x:authors`, mutably.
    pub fn authors_mut(&mut self) -> Option<&mut CommentAuthors> {
        self.content.iter_mut().find_map(|item| match item {
            CommentsContent::Authors(authors) => Some(authors),
            _ => None,
        })
    }

    /// `x:commentList` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn list(&self) -> Option<&CommentList> {
        self.content.iter().find_map(|item| match item {
            CommentsContent::List(list) => Some(list),
            _ => None,
        })
    }

    /// `x:commentList`, mutably.
    pub fn list_mut(&mut self) -> Option<&mut CommentList> {
        self.content.iter_mut().find_map(|item| match item {
            CommentsContent::List(list) => Some(list),
            _ => None,
        })
    }

    /// The comment attached to `cell`, and its index in the list.
    ///
    /// The first match, because `@ref` is `ST_Ref` and nothing forbids a file writing two comments
    /// on one cell; Excel writes one. A `@ref` that is a *range* rather than a single cell does not
    /// match, because [`CellReference`] is a cell.
    #[must_use]
    pub fn comment_at(
        &self,
        interner: &Interner,
        cell: CellReference,
    ) -> Option<(usize, &Comment)> {
        self.list()?
            .comments()
            .enumerate()
            .find(|(_, comment)| comment.cell(interner).ok() == Some(cell))
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CommentsContent::Authors(authors) => RawNode::Element(authors.as_raw_element()),
                CommentsContent::List(list) => RawNode::Element(list.as_raw_element()),
                CommentsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }

    /// The rank a fresh child would be inserted at, from the generated `CT_Comments` table.
    ///
    /// Exposed so `Comments::blank` below and `mjx-xlsx`'s authoring path place through the same
    /// table rather than at a hand-written index.
    fn insert_index(&self, local: &str) -> usize {
        COMMENTS.insert_index_of_names(self.content.iter().map(CommentsContent::rank), local)
    }

    /// Sets `x:authors`, replacing the existing one where it is or inserting it at rank 0.
    pub fn set_authors(&mut self, authors: CommentAuthors) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, CommentsContent::Authors(_)))
        {
            *item = CommentsContent::Authors(authors);
            return;
        }
        let at = self.insert_index("authors");
        self.content.insert(at, CommentsContent::Authors(authors));
        self.empty = false;
    }

    /// Sets `x:commentList`, replacing the existing one where it is or inserting it at rank 1.
    pub fn set_list(&mut self, list: CommentList) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, CommentsContent::List(_)))
        {
            *item = CommentsContent::List(list);
            return;
        }
        let at = self.insert_index("commentList");
        self.content.insert(at, CommentsContent::List(list));
        self.empty = false;
    }
}

impl CommentsContent {
    /// This child's rank in `CT_Comments`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Authors(_) => COMMENTS.rank_of(None, "authors"),
            Self::List(_) => COMMENTS.rank_of(None, "commentList"),
            Self::Raw(_) => None,
        }
    }
}

impl ToXml for CommentAuthors {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

impl ToXml for CommentList {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

impl ToXml for Comments {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

impl ToXml for Comment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

impl ToXml for CommentProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// Reads a comments part from its own bytes.
///
/// # Errors
/// [`SmlError::Xml`](crate::SmlError::Xml) if `markup` is not well-formed, or
/// [`SmlError::Model`](crate::SmlError::Model) if the root will not decode.
pub fn parse_comments(markup: &[u8]) -> Result<(Comments, Interner), crate::SmlError> {
    let document = mjx_xml::fidelity::parse(markup)?;
    let comments = Comments::from_xml(&document.root, &document.interner)?;
    Ok((comments, document.interner))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A comments part in the shape LibreOffice writes it: no `commentPr`, `xml:space` on the text,
    /// two comments, one author.
    const LIBREOFFICE_SHAPED: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors><author>Unknown Author</author></authors><commentList><comment ref="A2" authorId="0"><text><r><rPr><sz val="10"/></rPr><t xml:space="preserve">Checked.&#10;Twice.</t></r></text></comment><comment ref="B1" authorId="0"><text><t>Thousands.</t></text></comment></commentList></comments>"#;

    #[test]
    fn a_producer_comments_part_round_trips_byte_for_byte() {
        let (comments, mut interner) = parse_comments(LIBREOFFICE_SHAPED).expect("parse");
        let mut out = Vec::new();
        mjx_xml::fidelity::serialize_element(
            &comments.to_xml(&mut interner),
            &interner,
            None,
            &mut out,
        );
        let body = &LIBREOFFICE_SHAPED[LIBREOFFICE_SHAPED
            .windows(2)
            .position(|pair| pair == b"?>")
            .expect("an XML declaration")
            + 3..];
        assert_eq!(
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(body),
            "the part did not come back verbatim"
        );
    }

    #[test]
    fn the_author_index_is_a_position_not_a_name() {
        let (comments, interner) = parse_comments(LIBREOFFICE_SHAPED).expect("parse");
        let authors = comments.authors().expect("authors");
        assert_eq!(
            authors.author(0).map(CommentAuthor::text),
            Some("Unknown Author")
        );
        assert_eq!(authors.author(1), None);
        assert_eq!(authors.index_of("Unknown Author"), Some(0));
        assert_eq!(authors.index_of("unknown author"), None);
        let list = comments.list().expect("list");
        assert_eq!(list.len(), 2);
        for comment in list.comments() {
            assert_eq!(comment.author_index(&interner).expect("authorId"), 0);
        }
    }

    #[test]
    fn a_comments_text_concatenates_every_run() {
        let (comments, interner) = parse_comments(LIBREOFFICE_SHAPED).expect("parse");
        let cell = CellReference::parse("A2").expect("A2");
        let (index, comment) = comments.comment_at(&interner, cell).expect("A2 comment");
        assert_eq!(index, 0);
        assert_eq!(
            comment.text().map(CommentText::text),
            Some("Checked.\nTwice.")
        );
        assert!(comment.text().and_then(CommentText::runs_markup).is_some());
        assert!(
            comment.properties().is_none(),
            "LibreOffice writes no commentPr"
        );
    }

    #[test]
    fn commentpr_reports_the_six_true_defaults_for_an_element_that_writes_none() {
        let markup = br#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors><author>A</author></authors><commentList><comment ref="A1" authorId="0"><text><t>x</t></text><commentPr><anchor moveWithCells="1"/></commentPr></comment></commentList></comments>"#;
        let (comments, interner) = parse_comments(markup).expect("parse");
        let cell = CellReference::parse("A1").expect("A1");
        let (_, comment) = comments.comment_at(&interner, cell).expect("A1");
        let properties = comment.properties().expect("commentPr");
        for (name, value) in [
            ("locked", properties.is_locked(&interner)),
            ("defaultSize", properties.has_default_size(&interner)),
            ("print", properties.is_printed(&interner)),
            ("autoFill", properties.fills_automatically(&interner)),
            ("autoLine", properties.outlines_automatically(&interner)),
            ("lockText", properties.locks_text(&interner)),
        ] {
            assert!(value.expect(name), "@{name} defaults to true");
        }
        for (name, value) in [
            ("disabled", properties.is_disabled(&interner)),
            ("justLastX", properties.justifies_last_line(&interner)),
            ("autoScale", properties.scales_automatically(&interner)),
        ] {
            assert!(!value.expect(name), "@{name} defaults to false");
        }
        let anchor = properties.anchor().expect("anchor");
        assert!(anchor.moves_with_cells(&interner).expect("moveWithCells"));
        assert!(!anchor.sizes_with_cells(&interner).expect("sizeWithCells"));
    }

    #[test]
    fn a_comment_authored_here_places_its_children_through_the_generated_table() {
        let (mut comments, mut interner) = parse_comments(LIBREOFFICE_SHAPED).expect("parse");
        let index = comments
            .authors_mut()
            .expect("authors")
            .index_for(&mut interner, "Jai");
        assert_eq!(index, 1, "a new author appends");
        let cell = CellReference::parse("C3").expect("C3");
        let mut comment = Comment::new(&mut interner, None, cell, index, "Fresh");
        let properties = CommentProperties::new(&mut interner, None);
        comment.set_properties(properties);
        let locals: Vec<_> = comment
            .content()
            .iter()
            .map(|item| match item {
                CommentContent::Text(_) => "text",
                CommentContent::Properties(_) => "commentPr",
                CommentContent::Raw(_) => "raw",
            })
            .collect();
        assert_eq!(locals, ["text", "commentPr"], "commentPr follows text");
        comments.list_mut().expect("list").push(comment);
        assert_eq!(comments.list().expect("list").len(), 3);
    }

    #[test]
    fn removing_a_comment_steps_over_markup_the_model_does_not_hold() {
        let markup = br#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors><author>A</author></authors><commentList><!-- a note --><comment ref="A1" authorId="0"><text><t>one</t></text></comment><comment ref="A2" authorId="0"><text><t>two</t></text></comment></commentList></comments>"#;
        let (mut comments, interner) = parse_comments(markup).expect("parse");
        let list = comments.list_mut().expect("list");
        let removed = list.remove(0).expect("the first comment");
        assert_eq!(
            removed.cell(&interner).expect("ref"),
            CellReference::parse("A1").expect("A1")
        );
        assert_eq!(list.len(), 1);
        assert!(
            list.content()
                .iter()
                .any(|item| matches!(item, CommentListContent::Raw(_))),
            "the comment node between the two entries survived"
        );
    }

    #[test]
    fn the_legacy_drawing_edge_holds_the_identifier_the_file_wrote() {
        let document =
            mjx_xml::fidelity::parse(br#"<legacyDrawing xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId7"/>"#)
                .expect("parse");
        let drawing = LegacyDrawing::from_xml(&document.root, &document.interner).expect("model");
        assert_eq!(
            drawing
                .relationship_id(&document.interner, Some("r"))
                .expect("r:id"),
            Some("rId7".to_owned())
        );
    }
}
