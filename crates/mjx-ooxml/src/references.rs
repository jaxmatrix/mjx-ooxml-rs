//! The three read structures that name a part, restated with `String` part names.
//!
//! `mjx-opc`'s [`PartName`](mjx_opc::PartName) is a validated, normalized package path. It is the
//! right type *inside* the library and the wrong one at its edge: a binding cannot hold an opaque
//! Rust handle across the FFI boundary and hand it back, so the facade speaks part names as strings
//! and validates them on the way in (see [`crate::index::part_name`]).
//!
//! Everything else `mjx-pptx` returns is already string- or number-shaped and is re-exported
//! unchanged; these three are the only structures that carried a `PartName` field.

use crate::index::count;

/// A relationship whose target lies **outside** the package — a linked image, a chart's external
/// workbook, a linked OLE object or media file — and the part whose `.rels` holds it.
///
/// The discovery half of package hygiene: [`Deck::external_links`](crate::Deck::external_links)
/// lists them, [`Deck::retarget_external_link`](crate::Deck::retarget_external_link) redirects one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalLink {
    /// The part whose `.rels` holds this relationship, or `None` for the package root.
    pub source: Option<String>,
    /// The relationship id, unique within its source's `.rels`.
    pub id: String,
    /// The relationship type URI, which identifies what kind of external source it binds.
    pub rel_type: String,
    /// The external target, exactly as recorded (an absolute path or a URI).
    pub target: String,
}

impl From<mjx_pptx::ExternalRelationship> for ExternalLink {
    fn from(link: mjx_pptx::ExternalRelationship) -> Self {
        Self {
            source: link.source.map(|part| part.as_str().to_owned()),
            id: link.id,
            rel_type: link.rel_type,
            target: link.target,
        }
    }
}

/// One ink (InkML) reference on a surface: the `p:contentPart` that names it, the relationship id,
/// and the part it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InkReference {
    /// The top-level shape index of the `p:contentPart`, or `None` if the relationship is declared
    /// but no shape references it.
    pub shape_index: Option<u32>,
    /// The relationship id the content part carries (`r:id`).
    pub rel_id: String,
    /// The ink part the relationship resolves to, or `None` if it resolves to nothing.
    pub part: Option<String>,
}

impl From<mjx_pptx::InkReference> for InkReference {
    fn from(reference: mjx_pptx::InkReference) -> Self {
        Self {
            shape_index: reference.shape_index.map(count),
            rel_id: reference.rel_id,
            part: reference.part.map(|part| part.as_str().to_owned()),
        }
    }
}

/// The five parts a SmartArt diagram frame names: its data, layout, style and colour parts, plus the
/// optional cached drawing. Each is `None` when the frame declares no relationship of that kind or
/// the relationship resolves to nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramParts {
    /// The data model part (`dgm:relIds@dm`).
    pub data: Option<String>,
    /// The layout definition part (`dgm:relIds@lo`).
    pub layout: Option<String>,
    /// The style definition part (`dgm:relIds@qs`).
    pub style: Option<String>,
    /// The colour transform part (`dgm:relIds@cs`).
    pub colors: Option<String>,
    /// The optional cached drawing part, which renderers that do not lay out SmartArt fall back on.
    pub drawing: Option<String>,
}

impl From<mjx_pptx::DiagramParts> for DiagramParts {
    fn from(parts: mjx_pptx::DiagramParts) -> Self {
        let name = |part: Option<mjx_pptx::PartName>| part.map(|p| p.as_str().to_owned());
        Self {
            data: name(parts.data),
            layout: name(parts.layout),
            style: name(parts.style),
            colors: name(parts.colors),
            drawing: name(parts.drawing),
        }
    }
}
