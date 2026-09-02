//! `x:ClientData` — the data attached to a legacy form control, a comment, or an image placeholder
//! (`urn:schemas-microsoft-com:office:excel`).
//!
//! ECMA-376 Part 4 §19.4.2.12 *ClientData (Attached Object Data)*. Its `ObjectType` attribute says
//! which kind of object the [`Shape`](crate::Shape) carrying it draws, and its children — all simple
//! text leaves — carry that object's settings. This is what makes a legacy form control resolve from
//! the shape that points at it.

use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::VML_SPREADSHEET_DRAWING;

use crate::build::{self, fidelity_leaf};

/// `x:ClientData` (`CT_ClientData`) — ECMA-376 Part 4 §19.4.2.12 *ClientData (Attached Object
/// Data)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedObjectData {
    element: RawElement,
}
fidelity_leaf!(AttachedObjectData);

/// What kind of object an [`AttachedObjectData`] describes — `x:ClientData@ObjectType`
/// (`ST_ObjectType`). The variant names are ECMA-376 Part 4 §19.4.3.2's own, not the wire tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttachedObjectKind {
    /// A pushbutton control. Wire value `Button`.
    PushButton,
    /// A checkbox control. Wire value `Checkbox`.
    Checkbox,
    /// A dialog. Wire value `Dialog`.
    Dialog,
    /// A dropdown (combo box) control. Wire value `Drop`.
    DropdownBox,
    /// An editable text field control. Wire value `Edit`.
    EditableTextField,
    /// A group box control. Wire value `GBox`.
    GroupBox,
    /// A group of objects, such as a group of checkboxes. Wire value `Group`.
    Group,
    /// A label control. Wire value `Label`.
    Label,
    /// A formula auditing arrow. Wire value `LineA`.
    AuditingLine,
    /// A list control. Wire value `List`.
    ListBox,
    /// A movie object in Mac format. Wire value `Movie`.
    Movie,
    /// A comment. Wire value `Note`.
    Comment,
    /// A placeholder image. Wire value `Pict`.
    Image,
    /// A radio button control. Wire value `Radio`.
    RadioButton,
    /// A rectangle shape that is not a control. Wire value `Rect`.
    PlainRectangle,
    /// A formula auditing rectangle. Wire value `RectA`.
    AuditingRectangle,
    /// A scroll bar. Wire value `Scroll`.
    ScrollBar,
    /// A general shape that is not a control. Wire value `Shape`.
    PlainShape,
    /// A spin button (spinner) control. Wire value `Spin`.
    SpinButton,
}

impl AttachedObjectKind {
    /// The kind a wire value names, or `None` for one `ST_ObjectType` does not define.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Some(match value {
            "Button" => Self::PushButton,
            "Checkbox" => Self::Checkbox,
            "Dialog" => Self::Dialog,
            "Drop" => Self::DropdownBox,
            "Edit" => Self::EditableTextField,
            "GBox" => Self::GroupBox,
            "Group" => Self::Group,
            "Label" => Self::Label,
            "LineA" => Self::AuditingLine,
            "List" => Self::ListBox,
            "Movie" => Self::Movie,
            "Note" => Self::Comment,
            "Pict" => Self::Image,
            "Radio" => Self::RadioButton,
            "Rect" => Self::PlainRectangle,
            "RectA" => Self::AuditingRectangle,
            "Scroll" => Self::ScrollBar,
            "Shape" => Self::PlainShape,
            "Spin" => Self::SpinButton,
            _ => return None,
        })
    }

    /// The exact wire value for this kind.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::PushButton => "Button",
            Self::Checkbox => "Checkbox",
            Self::Dialog => "Dialog",
            Self::DropdownBox => "Drop",
            Self::EditableTextField => "Edit",
            Self::GroupBox => "GBox",
            Self::Group => "Group",
            Self::Label => "Label",
            Self::AuditingLine => "LineA",
            Self::ListBox => "List",
            Self::Movie => "Movie",
            Self::Comment => "Note",
            Self::Image => "Pict",
            Self::RadioButton => "Radio",
            Self::PlainRectangle => "Rect",
            Self::AuditingRectangle => "RectA",
            Self::ScrollBar => "Scroll",
            Self::PlainShape => "Shape",
            Self::SpinButton => "Spin",
        }
    }
}

impl AttachedObjectData {
    /// A fresh `x:ClientData` describing an object of `kind`, with no settings yet.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: AttachedObjectKind) -> Self {
        let mut element = build::element(
            interner,
            build::EXCEL_PREFIX,
            VML_SPREADSHEET_DRAWING,
            "ClientData",
            Vec::new(),
            Vec::new(),
        );
        build::set_attribute(
            &mut element.attributes,
            interner,
            "ObjectType",
            kind.to_wire(),
        );
        Self { element }
    }

    /// Which kind of object this describes (`@ObjectType`), or `None` when the attribute is absent
    /// or names a value `ST_ObjectType` does not define.
    #[must_use]
    pub fn kind(&self, interner: &Interner) -> Option<AttachedObjectKind> {
        build::attribute(&self.element.attributes, interner, "ObjectType")
            .and_then(|value| AttachedObjectKind::from_wire(&value))
    }

    /// The `ObjectType` exactly as the part spells it, including a value `ST_ObjectType` does not
    /// define — so an unrecognised kind is still reportable rather than silently `None`.
    #[must_use]
    pub fn kind_wire_value(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "ObjectType")
            .map(std::borrow::Cow::into_owned)
    }

    /// The decoded text of the setting `local` (`x:FmlaLink`, `x:Anchor`, `x:Row`, …), or `None`
    /// when this object states none.
    ///
    /// Every `CT_ClientData` child is a text leaf, so one accessor reads them all. A child present
    /// but empty — `<x:Visible/>`, which ECMA-376 Part 4 §19.7.3 `ST_TrueFalseBlank` reads as *true* —
    /// answers `Some("")`, which is how [`flag`](Self::flag) tells "stated" from "absent".
    #[must_use]
    pub fn setting(&self, interner: &Interner, local: &str) -> Option<String> {
        self.element.children.iter().find_map(|node| {
            let RawNode::Element(child) = node else {
                return None;
            };
            build::name_is(&child.name, interner, VML_SPREADSHEET_DRAWING, local)
                .then(|| build::element_text(&child.children))
        })
    }

    /// The setting `local` read as an `ST_TrueFalseBlank` flag: a value-less element means *true*,
    /// and `t`/`true`/`True`/`1` and `f`/`false`/`False`/`0` read as themselves. `None` when the
    /// object states no such setting.
    #[must_use]
    pub fn flag(&self, interner: &Interner, local: &str) -> Option<bool> {
        let value = self.setting(interner, local)?;
        if value.trim().is_empty() {
            return Some(true);
        }
        crate::true_false(value.trim())
    }

    /// The cell the control's value is bound to (`x:FmlaLink`, ECMA-376 Part 4 §19.4.2.26 *FmlaLink
    /// (Linked Formula)*), or `None`.
    #[must_use]
    pub fn linked_formula(&self, interner: &Interner) -> Option<String> {
        self.setting(interner, "FmlaLink")
    }

    /// The macro the control runs (`x:FmlaMacro`), or `None`.
    #[must_use]
    pub fn macro_formula(&self, interner: &Interner) -> Option<String> {
        self.setting(interner, "FmlaMacro")
    }

    /// The anchor that positions the object against the sheet grid (`x:Anchor`), or `None`.
    #[must_use]
    pub fn anchor(&self, interner: &Interner) -> Option<String> {
        self.setting(interner, "Anchor")
    }

    /// The embedded control the object hosts (`x:MapOCX`, ECMA-376 Part 4 §19.4.2.39 *MapOCX
    /// (Embedded Control)*) — present when the shape draws an ActiveX control rather than a built-in
    /// form control.
    #[must_use]
    pub fn hosts_embedded_control(&self, interner: &Interner) -> bool {
        self.flag(interner, "MapOCX").unwrap_or(false)
    }

    /// Sets the setting `local` to `value`, rewriting the existing child in place or appending one.
    pub fn set_setting(&mut self, interner: &mut Interner, local: &str, value: &str) {
        let escaped = mjx_xml::text::escape_text(value);
        let text = RawNode::Text(escaped.as_bytes().into());
        for node in &mut self.element.children {
            let RawNode::Element(child) = node else {
                continue;
            };
            if build::name_is(&child.name, interner, VML_SPREADSHEET_DRAWING, local) {
                child.children = vec![text];
                child.empty = false;
                return;
            }
        }
        let child = build::element(
            interner,
            build::EXCEL_PREFIX,
            VML_SPREADSHEET_DRAWING,
            local,
            Vec::new(),
            vec![text],
        );
        self.element.children.push(RawNode::Element(child));
        self.element.empty = false;
    }
}
