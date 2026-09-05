//! The **object-anchor vocabulary**: how anything that floats over a sheet rather than sitting in a
//! cell says where it is.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_ObjectAnchor` | 238 | `x:anchor` — inside `objectPr`, `controlPr` **and** `commentPr` |
//! | `CT_ObjectPr` | 3063 | `x:oleObject/objectPr` |
//!
//! # Why this is in `mjx-sml`, and why no Phase E child should model it again
//!
//! `CT_ObjectAnchor` is not one feature's type. `sml.xsd` reaches it from **three** places, and each
//! belongs to a different queued work item:
//!
//! | Owner of the containing type | Containing type | `sml.xsd` |
//! |---|---|---|
//! | MJXOFF-114 (E5) — cell comments and their legacy VML boxes | `CT_CommentPr` | 300 |
//! | MJXOFF-107 (E3) — OLE objects on a sheet | `CT_ObjectPr` | 3063 |
//! | MJXOFF-107 (E3) / MJXOFF-114 (E5) — form controls | `CT_ControlPr` | 3122 |
//!
//! Three children, one complex type. Modelled once here — in the shared-markup tier, which all three
//! sit above — so that none of them invents a second copy and the three end up disagreeing about
//! what `@moveWithCells` means. [`ObjectAnchor`] and [`ObjectProperties`] are exported from the crate
//! root for exactly that purpose. **MJXOFF-127 (D16) built them; E3 and E5 consume them.**
//!
//! # The markers are `xdr:`, and decoding them is MJXOFF-107's
//!
//! `CT_ObjectAnchor`'s two children are `xdr:from` and `xdr:to` — elements of
//! **`dml-spreadsheetDrawing.xsd`**, not of `sml.xsd`, both of type `xdr:CT_Marker` (a column, a
//! column offset, a row and a row offset). MJXOFF-107 (E3) owns that schema: it is the child that
//! adds `"dml-spreadsheetDrawing"` to `xtask`'s `CHILD_ORDER_SCHEMAS`, models the three anchor modes,
//! and resolves an anchor to absolute EMU against the sheet's column widths and row heights.
//!
//! So this type holds the two markers as **raw elements** — [`from_marker`](ObjectAnchor::from_marker)
//! and [`to_marker`](ObjectAnchor::to_marker) hand back the element the file wrote, verbatim and with
//! its prefix intact — and decodes neither. Writing a second `CT_Marker` model here to save E3 the
//! trouble would be the duplication this crate split exists to prevent, and it would be the wrong
//! crate's markup besides.
//!
//! What *is* here, and what E3 would otherwise have had to re-derive, is the **placement**:
//! [`set_from_marker`](ObjectAnchor::set_from_marker) and [`set_to_marker`](ObjectAnchor::set_to_marker)
//! go through the generated [`OBJECT_ANCHOR`] table, which knows that `xdr:from` precedes `xdr:to`
//! and knows it across a namespace boundary that no `sml` local name reveals.
//!
//! # Nothing here moves, resizes or prints anything
//!
//! `@moveWithCells` and `@sizeWithCells` say what a *consumer* should do to the object when the
//! cells under it move or resize, and `CT_ObjectPr`'s nine flags say what a consumer should let a
//! user do to it. This crate reports every one of them and acts on none — the rule
//! [`crate::features`] states for filters, sorts and validations, at a fourth door.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode, Text, ToXml};
use mjx_ooxml_types::child_order::{OBJECT_ANCHOR, OBJECT_PROPERTIES};
use mjx_ooxml_types::support::OnOff;

use crate::leaf::{attribute_bag, relationship_reference};
use crate::worksheet::rebuild_element;

attribute_bag! {
    /// `x:anchor` (`CT_ObjectAnchor`, `sml.xsd:238`) — where an object that floats over the grid is
    /// pinned to it.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_ObjectAnchor`. Wire element: `anchor`, inside `objectPr`,
    /// `controlPr` and `commentPr` alike.
    ///
    /// Two attributes, both `xsd:boolean` with a schema default of `false`, and two children in the
    /// SpreadsheetDrawingML namespace this crate deliberately does not decode — see the
    /// [module documentation](self).
    ///
    /// The schema declares both `xdr:from` and `xdr:to` `minOccurs="1"`, so an anchor missing either
    /// is invalid markup. It is still readable and still constructible: refusing to *open* a file
    /// over it would be refusing to read a workbook because of a defect in one comment box.
    #[xml(attribute(local = "moveWithCells", codec = OnOff, accessor = moves_with_cells, default = false))]
    #[xml(attribute(local = "sizeWithCells", codec = OnOff, accessor = sizes_with_cells, default = false))]
    ObjectAnchor, "anchor"
}

impl ObjectAnchor {
    /// `xdr:from` — the marker for the anchor's top-left corner, exactly as the file wrote it.
    ///
    /// A [`RawElement`] rather than a decoded `(column, offset, row, offset)`, because `xdr:CT_Marker`
    /// is `dml-spreadsheetDrawing.xsd`'s and MJXOFF-107 (E3) models it. See the
    /// [module documentation](self).
    #[must_use]
    pub fn from_marker(&self, interner: &Interner) -> Option<&RawElement> {
        self.marker(interner, "from")
    }

    /// `xdr:to` — the marker for the anchor's bottom-right corner, exactly as the file wrote it.
    #[must_use]
    pub fn to_marker(&self, interner: &Interner) -> Option<&RawElement> {
        self.marker(interner, "to")
    }

    /// Sets `xdr:from`, replacing the existing marker **where it is** or inserting one at rank 0 of
    /// `CT_ObjectAnchor`'s sequence.
    ///
    /// `marker` is used verbatim: its name, its prefix and its children are whatever the caller
    /// built, and nothing here checks that it is an `xdr:from` at all. That is deliberate — the
    /// caller that has a marker to place is E3, which owns the type, and second-guessing it here
    /// would mean this crate having an opinion about a schema it does not model.
    pub fn set_from_marker(&mut self, interner: &Interner, marker: RawElement) {
        self.place_marker(interner, marker, "from");
    }

    /// Sets `xdr:to`, replacing the existing marker where it is or inserting one at rank 1.
    pub fn set_to_marker(&mut self, interner: &Interner, marker: RawElement) {
        self.place_marker(interner, marker, "to");
    }

    /// The first child element this type's sequence names `local`, in either conformance world's
    /// SpreadsheetDrawingML namespace.
    fn marker<'a>(&'a self, interner: &Interner, local: &str) -> Option<&'a RawElement> {
        self.extra.iter().find_map(|node| {
            let RawNode::Element(element) = node else {
                return None;
            };
            let namespace = element
                .name
                .namespace
                .map(|symbol| interner.resolve(symbol));
            let candidate = interner.resolve(element.name.local);
            (candidate == local && OBJECT_ANCHOR.slot(namespace, candidate).is_some())
                .then_some(element)
        })
    }

    /// Places `marker` at the rank the generated table gives `local`, replacing the marker already
    /// in that slot.
    fn place_marker(&mut self, interner: &Interner, marker: RawElement, local: &str) {
        OBJECT_ANCHOR.replace_or_insert(&mut self.extra, interner, marker, |candidate| {
            candidate == local
        });
        self.empty = false;
    }
}

/// `x:objectPr` (`CT_ObjectPr`, `sml.xsd:3063`) — how an embedded OLE object behaves on the sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_ObjectPr`. Wire element: `objectPr`, the only child of
/// `CT_OleObject` (`sml.xsd:3051`), which is rank **34** of `CT_Worksheet` and **MJXOFF-107 (E3)'s**.
/// It is modelled here rather than there because it is the type that carries [`ObjectAnchor`]; see
/// the [module documentation](self).
///
/// Nine of its twelve attributes are `xsd:boolean`, and **six of them default to `true`** —
/// exactly `@locked`, `@defaultSize`, `@print`, `@autoFill`, `@autoLine` and `@autoPict`. That is
/// unusual enough in `sml.xsd` to be worth stating: a reader that assumed the `false` default every
/// other flag family here has would report six of them backwards for an element that writes none.
/// The declared defaults are the schema's and each getter returns its own.
///
/// `@r:id` is reached through [`relationship_id`](Self::relationship_id) rather than through the
/// attribute grammar, for the same reason [`TablePart`](crate::TablePart)'s is: its *prefix* is the
/// file's choice rather than the schema's. It names the **embedded object part**, and this crate
/// resolves it no more than it resolves a `tablePart@r:id`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "locked", codec = OnOff, accessor = is_locked, default = true))]
#[xml(attribute(local = "defaultSize", codec = OnOff, accessor = has_default_size, default = true))]
#[xml(attribute(local = "print", codec = OnOff, accessor = is_printed, default = true))]
#[xml(attribute(local = "disabled", codec = OnOff, accessor = is_disabled, default = false))]
#[xml(attribute(local = "uiObject", codec = OnOff, accessor = is_user_interface_object, default = false))]
#[xml(attribute(local = "autoFill", codec = OnOff, accessor = fills_automatically, default = true))]
#[xml(attribute(local = "autoLine", codec = OnOff, accessor = outlines_automatically, default = true))]
#[xml(attribute(local = "autoPict", codec = OnOff, accessor = scales_picture_automatically, default = true))]
#[xml(attribute(local = "macro", codec = Text, accessor = macro_formula))]
#[xml(attribute(local = "altText", codec = Text, accessor = alternative_text))]
#[xml(attribute(local = "dde", codec = OnOff, accessor = is_dynamic_data_exchange, default = false))]
pub struct ObjectProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "anchor", variant = Anchor, ty = ObjectAnchor))]
    content: Vec<ObjectPropertiesContent>,
}

relationship_reference!(ObjectProperties);

/// One child of [`ObjectProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectPropertiesContent {
    /// `x:anchor` (rank 0) — where the object is pinned. `minOccurs="1"`.
    Anchor(ObjectAnchor),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ObjectProperties {
    /// Builds an empty `x:objectPr`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `anchor` `minOccurs="1"`, so properties with no anchor are invalid
    /// markup; this is still constructible, because a caller builds one and then sets the anchor.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "objectPr"),
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
    pub fn content(&self) -> &[ObjectPropertiesContent] {
        &self.content
    }

    /// `x:anchor` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn anchor(&self) -> Option<&ObjectAnchor> {
        self.content.iter().find_map(|item| match item {
            ObjectPropertiesContent::Anchor(anchor) => Some(anchor),
            ObjectPropertiesContent::Raw(_) => None,
        })
    }

    /// `x:anchor`, mutably.
    pub fn anchor_mut(&mut self) -> Option<&mut ObjectAnchor> {
        self.content.iter_mut().find_map(|item| match item {
            ObjectPropertiesContent::Anchor(anchor) => Some(anchor),
            ObjectPropertiesContent::Raw(_) => None,
        })
    }

    /// Sets `x:anchor`, replacing the existing one where it is or inserting at rank 0.
    pub fn set_anchor(&mut self, anchor: ObjectAnchor) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, ObjectPropertiesContent::Anchor(_)))
        {
            *item = ObjectPropertiesContent::Anchor(anchor);
            return;
        }
        let at = OBJECT_PROPERTIES.insert_index_of_names(
            self.content.iter().map(ObjectPropertiesContent::rank),
            "anchor",
        );
        self.content
            .insert(at, ObjectPropertiesContent::Anchor(anchor));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ObjectPropertiesContent::Anchor(anchor) => {
                    RawNode::Element(anchor.as_raw_element())
                }
                ObjectPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ObjectPropertiesContent {
    /// This child's rank in `CT_ObjectPr`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Anchor(_) => OBJECT_PROPERTIES.rank_of(None, "anchor"),
            Self::Raw(_) => None,
        }
    }
}

impl ToXml for ObjectProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
