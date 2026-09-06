//! Embedded objects and form controls: `x:oleObjects` (rank **34** of `CT_Worksheet`) and
//! `x:controls` (rank **35**) — the two slots that hang an object off a sheet without putting it in
//! the drawing part.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_OleObjects` | 3046 | `x:oleObjects` (rank 34 of `CT_Worksheet`) |
//! | `CT_OleObject` | 3051 | `x:oleObjects/oleObject` |
//! | `CT_Controls` | 3109 | `x:controls` (rank 35) |
//! | `CT_Control` | 3114 | `x:controls/control` |
//! | `CT_ControlPr` | 3122 | `x:controls/control/controlPr` |
//!
//! # Why these are here rather than in the drawing part
//!
//! An OLE object and a form control are drawn on the sheet like a picture, but they are not anchored
//! through `xdr:wsDr`: each carries its **own** [`ObjectAnchor`](crate::ObjectAnchor), and each
//! reaches its appearance through a `shapeId` that names a shape in the sheet's *legacy VML*
//! drawing rather than in the DrawingML one. So they sit in `CT_Worksheet` beside the `drawing`
//! slot, not inside it — which is why MJXOFF-107 owns all three ranks and models them together.
//!
//! **The anchor vocabulary is not re-modelled here.** MJXOFF-127 (D16) put
//! [`ObjectAnchor`](crate::ObjectAnchor) and [`ObjectProperties`](crate::ObjectProperties) in
//! [`crate::features::objects`] precisely so that this file, and MJXOFF-114's comments, consume one
//! model instead of three; `CT_ObjectPr` is already there and is used unchanged.
//! [`FormControlProperties`] is the one type that had to be added, because `CT_ControlPr` is
//! `CT_ObjectPr` plus `@recalcAlways`, `@linkedCell`, `@listFillRange` and `@cf`, and is a different
//! complex type with its own generated table.
//!
//! # The `true`-defaulting flag family, for the second time
//!
//! MJXOFF-127 recorded it for `CT_ObjectPr`: **six of its nine boolean attributes default to
//! `true`** — `@locked`, `@defaultSize`, `@print`, `@autoFill`, `@autoLine`, `@autoPict` — which is
//! the opposite of every other flag family in `sml.xsd`. `CT_ControlPr` has exactly the same six
//! plus `@recalcAlways` (which defaults to `false`), and this file declares each default beside its
//! wire name rather than leaving it to a call site to remember.
//!
//! # Nothing here loads, runs or draws anything
//!
//! `@progId` names a COM class, `@link` is a formula naming the cell an OLE link updates from,
//! `@macro` names a function, and `@r:id` reaches an embedded object part whose bytes are somebody
//! else's format. Every one of them is reported exactly as written and **none is resolved,
//! evaluated, loaded or executed** — the rule [`crate::features`] states for filters, sorts and
//! validations, at a fifth and sixth door.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::CONTROL_PROPERTIES;
use mjx_ooxml_types::spreadsheetml::{DataViewAspect, OleUpdateType};
use mjx_ooxml_types::support::OnOff;

use crate::features::objects::{ObjectAnchor, ObjectProperties};
use crate::leaf::relationship_reference;
use crate::worksheet::rebuild_element;

/// Declares one `CT_*` list wrapper — an element whose whole content model is a single repeating
/// child — and the accessors every such wrapper in this crate has.
///
/// `CT_OleObjects` and `CT_Controls` are the same shape as
/// [`Hyperlinks`](crate::Hyperlinks) and [`TableParts`](crate::TableParts): one child slot, declared
/// `minOccurs="1"`, so appending *is* placing and no ordering table is needed.
macro_rules! entry_list {
    (
        $(#[$meta:meta])*
        $ty:ident, $content:ident, $entry:ty, $local:literal, $entry_local:literal, $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
        #[xml(namespace = SML)]
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            empty: bool,
            #[xml(children, child(local = $entry_local, variant = $variant, ty = $entry))]
            content: Vec<$content>,
        }

        #[doc = concat!("One child of [`", stringify!($ty), "`].")]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $content {
            #[doc = concat!("`x:", $entry_local, "` — one entry.")]
            $variant($entry),
            /// Anything else — preserved verbatim, in position.
            Raw(RawNode),
        }

        impl $ty {
            #[doc = concat!("Builds an empty `x:", $local, "`, bound to `prefix` or to the default \
                namespace.\n\nThe schema declares `", $entry_local, "` `minOccurs=\"1\"`, so an \
                element with no entry is invalid markup; it is still constructible, because a \
                caller builds one and then fills it.")]
            #[must_use]
            pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
                Self {
                    name: crate::leaf::sml_name(interner, prefix, $local),
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
            pub fn content(&self) -> &[$content] {
                &self.content
            }

            #[doc = concat!("Every `x:", $entry_local, "`, in document order.")]
            pub fn entries(&self) -> impl Iterator<Item = &$entry> + '_ {
                self.content.iter().filter_map(|item| match item {
                    $content::$variant(entry) => Some(entry),
                    $content::Raw(_) => None,
                })
            }

            /// How many entries the element lists.
            #[must_use]
            pub fn len(&self) -> usize {
                self.entries().count()
            }

            /// Whether the element lists no entry at all, which the schema forbids.
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// The `index`-th entry, mutably.
            pub fn entry_mut(&mut self, index: usize) -> Option<&mut $entry> {
                self.content
                    .iter_mut()
                    .filter_map(|item| match item {
                        $content::$variant(entry) => Some(entry),
                        $content::Raw(_) => None,
                    })
                    .nth(index)
            }

            /// Appends an entry after the ones already present — which is also its rank, because
            /// this type declares exactly one child slot.
            pub fn push(&mut self, entry: $entry) {
                self.content.push($content::$variant(entry));
                self.empty = false;
            }

            /// Removes the `index`-th entry and returns it, or `None` when the element holds fewer.
            ///
            /// Markup between the entries is left where it is: only the entry element itself is
            /// taken out. **The relationship the entry named is not touched** — this crate cannot
            /// see one.
            pub fn remove(&mut self, index: usize) -> Option<$entry> {
                let at = self
                    .content
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| matches!(item, $content::$variant(_)))
                    .map(|(at, _)| at)
                    .nth(index)?;
                match self.content.remove(at) {
                    $content::$variant(entry) => Some(entry),
                    $content::Raw(_) => unreachable!("the position was filtered on the entry"),
                }
            }

            /// This element rebuilt as a [`RawElement`], without an interner.
            #[must_use]
            pub fn as_raw_element(&self) -> RawElement {
                let children = self
                    .content
                    .iter()
                    .map(|item| match item {
                        $content::$variant(entry) => RawNode::Element(entry.as_raw_element()),
                        $content::Raw(node) => node.clone(),
                    })
                    .collect();
                rebuild_element(self.name, &self.attributes, children, self.empty)
            }
        }

        impl ToXml for $ty {
            fn to_xml(&self, _interner: &mut Interner) -> RawElement {
                self.as_raw_element()
            }
        }
    };
}

// -------------------------------------------------------------------------------------------
// `x:oleObjects` — rank 34
// -------------------------------------------------------------------------------------------

/// `x:oleObject` (`CT_OleObject`, `sml.xsd:3051`) — one embedded or linked object on the sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_OleObject`. Wire element: `oleObject`, the only child of
/// `CT_OleObjects`.
///
/// `@shapeId` is `use="required"` and is the **legacy VML** shape this object is drawn as, not a
/// DrawingML one: an OLE object's appearance lives in the `vmlDrawing` part MJXOFF-114 (E5) owns,
/// and this crate resolves neither. `@r:id` names the embedded object part — an `.xlsx`, a `.docx`,
/// a `.bin`, whatever the producing application wrote — and is held as the string the file wrote,
/// exactly as a [`TablePart`](crate::TablePart)'s is.
///
/// `@link` is a **formula**, not a URI: an OLE link updates from a cell reference. It is carried as
/// text and never parsed, for the reason [`CellFormula`](crate::CellFormula) states.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "progId", codec = Text, accessor = program_id))]
#[xml(attribute(local = "dvAspect", codec = Enumeration<DataViewAspect>, accessor = view_aspect, default = DataViewAspect::Content))]
#[xml(attribute(local = "link", codec = Text, accessor = link_formula))]
#[xml(attribute(local = "oleUpdate", codec = Enumeration<OleUpdateType>, accessor = update_mode))]
#[xml(attribute(local = "autoLoad", codec = OnOff, accessor = loads_automatically, default = false))]
#[xml(attribute(local = "shapeId", codec = Number<u32>, accessor = shape_id, required))]
pub struct EmbeddedObject {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "objectPr", variant = Properties, ty = ObjectProperties))]
    content: Vec<EmbeddedObjectContent>,
}

relationship_reference!(EmbeddedObject);

/// One child of [`EmbeddedObject`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedObjectContent {
    /// `x:objectPr` (rank 0) — how the object behaves, and where it is anchored.
    Properties(ObjectProperties),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl EmbeddedObject {
    /// Builds an `x:oleObject` naming `shape_id`, bound to `prefix` or to the default namespace.
    ///
    /// `@shapeId` is the one attribute the schema declares `use="required"`, so it is the one
    /// argument: an element without it is not an entry this crate would be able to read back.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, shape_id: u32) -> Self {
        let mut value = Self {
            name: crate::leaf::sml_name(interner, prefix, "oleObject"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_shape_id(interner, shape_id);
        value
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order.
    #[must_use]
    pub fn content(&self) -> &[EmbeddedObjectContent] {
        &self.content
    }

    /// `x:objectPr` — how the object behaves on the sheet. `None` when it writes none, which the
    /// schema permits (`minOccurs="0"`).
    #[must_use]
    pub fn properties(&self) -> Option<&ObjectProperties> {
        self.content.iter().find_map(|item| match item {
            EmbeddedObjectContent::Properties(properties) => Some(properties),
            EmbeddedObjectContent::Raw(_) => None,
        })
    }

    /// `x:objectPr`, mutably.
    pub fn properties_mut(&mut self) -> Option<&mut ObjectProperties> {
        self.content.iter_mut().find_map(|item| match item {
            EmbeddedObjectContent::Properties(properties) => Some(properties),
            EmbeddedObjectContent::Raw(_) => None,
        })
    }

    /// Where the object is pinned, from its `x:objectPr`.
    #[must_use]
    pub fn anchor(&self) -> Option<&ObjectAnchor> {
        self.properties()?.anchor()
    }

    /// Sets `x:objectPr`, replacing the existing one where it is or inserting it at rank 0 — the
    /// only rank `CT_OleObject` has.
    pub fn set_properties(&mut self, properties: ObjectProperties) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, EmbeddedObjectContent::Properties(_)))
        {
            *item = EmbeddedObjectContent::Properties(properties);
            return;
        }
        self.content
            .insert(0, EmbeddedObjectContent::Properties(properties));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                EmbeddedObjectContent::Properties(properties) => {
                    RawNode::Element(properties.as_raw_element())
                }
                EmbeddedObjectContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for EmbeddedObject {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

entry_list! {
    /// `x:oleObjects` (`CT_OleObjects`, `sml.xsd:3046`) — every embedded object on the sheet, in
    /// document order.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_OleObjects`. Wire element: `oleObjects`, rank **34** of
    /// `CT_Worksheet` — between `picture` (33) and `controls` (35).
    EmbeddedObjects, EmbeddedObjectsContent, EmbeddedObject, "oleObjects", "oleObject", Object
}

// -------------------------------------------------------------------------------------------
// `x:controls` — rank 35
// -------------------------------------------------------------------------------------------

/// `x:controlPr` (`CT_ControlPr`, `sml.xsd:3122`) — how a form control behaves on the sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_ControlPr`. Wire element: `controlPr`, the only child of
/// `CT_Control`.
///
/// `CT_ObjectPr` plus four: `@recalcAlways` (`false`), and the three that are what makes a *control*
/// rather than an object — `@linkedCell` (the cell the control writes its value into),
/// `@listFillRange` (the range a list control draws its items from) and `@cf` (the clipboard format
/// its appearance is stored in, defaulting to `pict`).
///
/// **Six of its ten boolean attributes default to `true`** — the family MJXOFF-127 recorded for
/// `CT_ObjectPr`, repeated here exactly. `@linkedCell` and `@listFillRange` are `ST_Formula`: they
/// are carried as text, never parsed into a range, and nothing here reads or writes the cell either
/// one names.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "locked", codec = OnOff, accessor = is_locked, default = true))]
#[xml(attribute(local = "defaultSize", codec = OnOff, accessor = has_default_size, default = true))]
#[xml(attribute(local = "print", codec = OnOff, accessor = is_printed, default = true))]
#[xml(attribute(local = "disabled", codec = OnOff, accessor = is_disabled, default = false))]
#[xml(attribute(local = "recalcAlways", codec = OnOff, accessor = recalculates_always, default = false))]
#[xml(attribute(local = "uiObject", codec = OnOff, accessor = is_user_interface_object, default = false))]
#[xml(attribute(local = "autoFill", codec = OnOff, accessor = fills_automatically, default = true))]
#[xml(attribute(local = "autoLine", codec = OnOff, accessor = outlines_automatically, default = true))]
#[xml(attribute(local = "autoPict", codec = OnOff, accessor = scales_picture_automatically, default = true))]
#[xml(attribute(local = "macro", codec = Text, accessor = macro_formula))]
#[xml(attribute(local = "altText", codec = Text, accessor = alternative_text))]
#[xml(attribute(local = "linkedCell", codec = Text, accessor = linked_cell_formula))]
#[xml(attribute(local = "listFillRange", codec = Text, accessor = list_fill_range_formula))]
#[xml(attribute(local = "cf", codec = Text, accessor = clipboard_format))]
pub struct FormControlProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "anchor", variant = Anchor, ty = ObjectAnchor))]
    content: Vec<FormControlPropertiesContent>,
}

relationship_reference!(FormControlProperties);

/// One child of [`FormControlProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormControlPropertiesContent {
    /// `x:anchor` (rank 0) — where the control is pinned. `minOccurs="1"`.
    Anchor(ObjectAnchor),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl FormControlProperties {
    /// Builds an empty `x:controlPr`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "controlPr"),
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

    /// Every child, in document order.
    #[must_use]
    pub fn content(&self) -> &[FormControlPropertiesContent] {
        &self.content
    }

    /// `x:anchor` — `None` for markup that writes none, which the schema forbids.
    #[must_use]
    pub fn anchor(&self) -> Option<&ObjectAnchor> {
        self.content.iter().find_map(|item| match item {
            FormControlPropertiesContent::Anchor(anchor) => Some(anchor),
            FormControlPropertiesContent::Raw(_) => None,
        })
    }

    /// `x:anchor`, mutably.
    pub fn anchor_mut(&mut self) -> Option<&mut ObjectAnchor> {
        self.content.iter_mut().find_map(|item| match item {
            FormControlPropertiesContent::Anchor(anchor) => Some(anchor),
            FormControlPropertiesContent::Raw(_) => None,
        })
    }

    /// Sets `x:anchor`, replacing the existing one where it is or inserting at its rank in
    /// `CT_ControlPr`'s sequence — which the generated [`CONTROL_PROPERTIES`] table gives, exactly
    /// as [`ObjectProperties::set_anchor`](crate::ObjectProperties::set_anchor) takes it from
    /// `OBJECT_PROPERTIES`.
    pub fn set_anchor(&mut self, anchor: ObjectAnchor) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, FormControlPropertiesContent::Anchor(_)))
        {
            *item = FormControlPropertiesContent::Anchor(anchor);
            return;
        }
        let at = CONTROL_PROPERTIES.insert_index_of_names(
            self.content.iter().map(FormControlPropertiesContent::rank),
            "anchor",
        );
        self.content
            .insert(at, FormControlPropertiesContent::Anchor(anchor));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                FormControlPropertiesContent::Anchor(anchor) => {
                    RawNode::Element(anchor.as_raw_element())
                }
                FormControlPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl FormControlPropertiesContent {
    /// This child's rank in `CT_ControlPr`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Anchor(_) => CONTROL_PROPERTIES.rank_of(None, "anchor"),
            Self::Raw(_) => None,
        }
    }
}

impl ToXml for FormControlProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:control` (`CT_Control`, `sml.xsd:3114`) — one form control on the sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_Control`. Wire element: `control`, the only child of `CT_Controls`.
///
/// Two attributes are `use="required"` and both name something outside this element: `@shapeId` is
/// the legacy VML shape the control is drawn as, and `@r:id` reaches the ActiveX control part
/// holding its persisted state. Neither is resolved here.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "shapeId", codec = Number<u32>, accessor = shape_id, required))]
#[xml(attribute(local = "name", codec = Text, accessor = control_name))]
pub struct FormControl {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "controlPr", variant = Properties, ty = FormControlProperties))]
    content: Vec<FormControlContent>,
}

relationship_reference!(FormControl);

/// One child of [`FormControl`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormControlContent {
    /// `x:controlPr` (rank 0) — how the control behaves, and where it is anchored.
    Properties(FormControlProperties),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl FormControl {
    /// Builds an `x:control` naming `shape_id`, bound to `prefix` or to the default namespace.
    ///
    /// `@r:id` is `use="required"` too but is not an argument, for the reason
    /// [`TablePart::new`](crate::TablePart) leaves its own out: a relationship identifier is
    /// allocated by whoever holds the package, and this crate does not.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>, shape_id: u32) -> Self {
        let mut value = Self {
            name: crate::leaf::sml_name(interner, prefix, "control"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        };
        value.set_shape_id(interner, shape_id);
        value
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order.
    #[must_use]
    pub fn content(&self) -> &[FormControlContent] {
        &self.content
    }

    /// `x:controlPr` — `None` when the control writes none, which the schema permits.
    #[must_use]
    pub fn properties(&self) -> Option<&FormControlProperties> {
        self.content.iter().find_map(|item| match item {
            FormControlContent::Properties(properties) => Some(properties),
            FormControlContent::Raw(_) => None,
        })
    }

    /// `x:controlPr`, mutably.
    pub fn properties_mut(&mut self) -> Option<&mut FormControlProperties> {
        self.content.iter_mut().find_map(|item| match item {
            FormControlContent::Properties(properties) => Some(properties),
            FormControlContent::Raw(_) => None,
        })
    }

    /// Where the control is pinned, from its `x:controlPr`.
    #[must_use]
    pub fn anchor(&self) -> Option<&ObjectAnchor> {
        self.properties()?.anchor()
    }

    /// Sets `x:controlPr`, replacing the existing one where it is or inserting it at rank 0 — the
    /// only rank `CT_Control` has.
    pub fn set_properties(&mut self, properties: FormControlProperties) {
        if let Some(item) = self
            .content
            .iter_mut()
            .find(|item| matches!(item, FormControlContent::Properties(_)))
        {
            *item = FormControlContent::Properties(properties);
            return;
        }
        self.content
            .insert(0, FormControlContent::Properties(properties));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                FormControlContent::Properties(properties) => {
                    RawNode::Element(properties.as_raw_element())
                }
                FormControlContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for FormControl {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

entry_list! {
    /// `x:controls` (`CT_Controls`, `sml.xsd:3109`) — every form control on the sheet, in document
    /// order.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Controls`. Wire element: `controls`, rank **35** of
    /// `CT_Worksheet` — the last of the three slots MJXOFF-107 owns.
    FormControls, FormControlsContent, FormControl, "controls", "control", Control
}
