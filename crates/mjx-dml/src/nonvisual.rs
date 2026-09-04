//! The non-visual property blocks every DrawingML drawing object carries: identity/name/hidden
//! (`a:CT_NonVisualDrawingProps`, `cNvPr`/`docPr` — shared verbatim across every host schema that
//! places a picture, shape, group, graphic frame or content part), and the six per-kind "locking"
//! siblings (`cNvSpPr`, `cNvPicPr`, `cNvGrpSpPr`, `cNvCnPr`, `cNvGraphicFramePr`,
//! `cNvContentPartPr`) that each add nothing beyond an optional lock list, `extLst`, and — for three
//! of the six — one boolean attribute.
//!
//! `CT_NonVisualDrawingProps` is the one member of this family worth a full accessor set: its `id`
//! and `name` are read constantly (every consumer of a shape tree wants to know what it is looking
//! at), so it gets typed attributes and stays a fidelity wrapper for `hlinkClick`/`hlinkHover`/
//! `extLst`, which nothing in this workspace queries yet. The six lock-list siblings carry
//! essentially no query value of their own — a `CT_ShapeLocking`/`CT_PictureLocking`/… is a bag of
//! "cannot be ungrouped" style toggles nothing in this codebase reads — so each is a minimal fidelity
//! wrapper: the one attribute the schema gives it typed (`txBox`, `preferRelativeResize`,
//! `isComment`), everything else opaque.

use mjx_ooxml_core::{Interner, Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::support::OnOff;

use crate::build::{dml_name, fidelity_element_impls};

/// `a:CT_NonVisualDrawingProps` (`cNvPr` in a shape/picture/connector's non-visual block, `docPr` on
/// `wp:inline`/`wp:anchor`) — the identity every drawing object in DrawingML carries: a required
/// numeric id and name, an optional description, hidden flag and title.
///
/// A fidelity wrapper: `id`/`name`/`descr`/`hidden`/`title` are typed; `hlinkClick`, `hlinkHover` and
/// `extLst` are preserved opaque so the element round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", codec = Number<u32>, accessor = id, required))]
#[xml(attribute(local = "name", codec = Text, accessor = raw_name, required))]
#[xml(attribute(local = "descr", codec = Text, accessor = raw_description))]
#[xml(attribute(local = "hidden", codec = OnOff, accessor = hidden))]
#[xml(attribute(local = "title", codec = Text, accessor = raw_title))]
pub struct NonVisualDrawingProps {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl NonVisualDrawingProps {
    /// Builds `<{local} id="{id}" name="{drawing_name}"/>` — `local` is `"cNvPr"` or `"docPr"`,
    /// whichever the caller's own schema position calls it.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, id: u32, drawing_name: &str) -> Self {
        let mut value = Self {
            name: dml_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        value.set_id(interner, id);
        value.set_raw_name(interner, drawing_name);
        value
    }

    /// The drawing object's own name (`@name`), or `None` if malformed.
    #[must_use]
    pub fn drawing_name(&self, interner: &Interner) -> Option<String> {
        self.raw_name(interner)
            .ok()
            .map(std::borrow::Cow::into_owned)
    }

    /// The alt-text description (`@descr`), or `None` if absent/malformed.
    #[must_use]
    pub fn description(&self, interner: &Interner) -> Option<String> {
        self.raw_description(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }

    /// Whether this object is hidden (`@hidden`; wire default `false`).
    #[must_use]
    pub fn is_hidden(&self, interner: &Interner) -> bool {
        self.hidden(interner).ok().flatten().unwrap_or(false)
    }

    /// The title (`@title`, a tooltip in Word's UI), or `None` if absent/malformed.
    #[must_use]
    pub fn title(&self, interner: &Interner) -> Option<String> {
        self.raw_title(interner)
            .ok()
            .flatten()
            .map(std::borrow::Cow::into_owned)
    }
}

fidelity_element_impls!(NonVisualDrawingProps);

/// Declares a minimal non-visual "locking" fidelity wrapper: an element with no typed children (an
/// optional `CT_*Locking` child and `extLst` both stay opaque) and, when `$attr` is given, one typed
/// boolean attribute.
macro_rules! locking_wrapper {
    ($(#[$meta:meta])* $ty:ident, $local:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            children: Vec<RawNode>,
            empty: bool,
        }

        impl $ty {
            #[doc = concat!("Builds a self-closing `<a:", $local, "/>` with no lock list.")]
            #[must_use]
            pub fn new(interner: &mut Interner) -> Self {
                Self {
                    name: dml_name(interner, $local),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    empty: true,
                }
            }
        }

        fidelity_element_impls!($ty);
    };
    (
        $(#[$meta:meta])* $ty:ident, $local:literal,
        $attr_local:literal, $accessor:ident, $default:literal
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
        #[xml(attribute(local = $attr_local, codec = OnOff, accessor = $accessor))]
        pub struct $ty {
            name: RawName,
            attributes: Vec<RawAttribute>,
            children: Vec<RawNode>,
            empty: bool,
        }

        impl $ty {
            #[doc = concat!("Builds a self-closing `<a:", $local, "/>` with no lock list and `@", $attr_local, "` unstated (wire default `", $default, "`).")]
            #[must_use]
            pub fn new(interner: &mut Interner) -> Self {
                Self {
                    name: dml_name(interner, $local),
                    attributes: Vec::new(),
                    children: Vec::new(),
                    empty: true,
                }
            }
        }

        fidelity_element_impls!($ty);
    };
}

locking_wrapper!(
    /// `a:CT_NonVisualDrawingShapeProps` (`cNvSpPr`) — a shape's own lock list plus whether it is a
    /// text box (`@txBox`; wire default `false`).
    NonVisualDrawingShapeProperties, "cNvSpPr", "txBox", is_text_box, "false"
);

locking_wrapper!(
    /// `a:CT_NonVisualPictureProperties` (`cNvPicPr`) — a picture's own lock list plus whether it
    /// prefers relative resizing (`@preferRelativeResize`; wire default `true`).
    NonVisualPictureProperties, "cNvPicPr", "preferRelativeResize", prefers_relative_resize, "true"
);

locking_wrapper!(
    /// `a:CT_NonVisualGraphicFrameProperties` (`cNvGraphicFramePr`/`cNvFrPr`) — a graphic frame's
    /// own lock list. No attributes of its own.
    NonVisualGraphicFrameProperties, "cNvGraphicFramePr"
);

locking_wrapper!(
    /// `a:CT_NonVisualGroupDrawingShapeProps` (`cNvGrpSpPr`) — a group's own lock list. No
    /// attributes of its own.
    NonVisualGroupDrawingShapeProperties, "cNvGrpSpPr"
);

locking_wrapper!(
    /// `a:CT_NonVisualConnectorProperties` (`cNvCnPr`) — a connector's own lock list plus its
    /// start/end connection sites (`stCxn`/`endCxn`, both kept opaque — nothing in this workspace
    /// reads a connector's own endpoints yet).
    NonVisualConnectorProperties, "cNvCnPr"
);

locking_wrapper!(
    /// `a:CT_NonVisualContentPartProperties` (`cNvContentPartPr`) — an ink content part's own lock
    /// list plus whether it represents a comment/annotation (`@isComment`; wire default `true`).
    NonVisualContentPartProperties, "cNvContentPartPr", "isComment", is_comment, "true"
);
