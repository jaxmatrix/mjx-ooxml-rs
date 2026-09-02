//! `mjx-vml` — Legacy VML (Transitional-only): the vocabulary, the model, and the shape-level
//! references.
//!
//! VML (Vector Markup Language) is the legacy drawing markup carried in the *Transitional* flavour of
//! OOXML (ECMA-376 Part 4 §14.1, reference material §19) and dropped from *Strict*. Producers still
//! emit it for constructs with no DrawingML equivalent — OLE-object fallbacks, comment authoring
//! shapes, ink, and legacy form controls — as standalone `vmlDrawingN.vml` parts referenced by
//! relationship id, and inline inside a WordprocessingML `w:pict` or an `mc:Fallback` branch.
//!
//! # What this crate is for
//!
//! A legacy construct is only useful if you can get from the *modern* markup that points at it to the
//! *legacy* shape that draws it. That hop is an identifier match, and it is what this crate exists to
//! make: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a VML
//! [`Shape::identifier`], and [`Drawing::shape_by_identifier`] resolves it.
//!
//! ```
//! use mjx_vml::DrawingPart;
//!
//! # fn main() -> Result<(), mjx_vml::VmlError> {
//! let part = DrawingPart::parse(br##"<xml xmlns:v="urn:schemas-microsoft-com:vml"
//!  xmlns:o="urn:schemas-microsoft-com:office:office">
//!  <v:shape id="_x0000_s1026" type="#_x0000_t202" style="width:100pt" filled="f"/>
//! </xml>"##)?;
//!
//! let shape = part
//!     .drawing()
//!     .shape_by_identifier(part.interner(), "_x0000_s1026")
//!     .expect("the shape an OLE frame's spid names");
//! assert_eq!(shape.template_identifier(part.interner()).as_deref(), Some("_x0000_t202"));
//! assert_eq!(shape.is_filled(part.interner()), Some(false));
//! # Ok(())
//! # }
//! ```
//!
//! # Naming
//!
//! VML element names are cryptic, so every type here is named from the ECMA-376 Part 4 §19 prose
//! rather than the wire token: `v:shapetype` is a [`ShapeTemplate`] (§19.1.2.20 *shapetype (Shape
//! Template)*), `o:idmap` is a [`ShapeIdMap`] (§19.2.2.14 *idmap (Shape ID Map)*), `x:ClientData` is
//! [`AttachedObjectData`] (§19.4.2.12 *ClientData (Attached Object Data)*). Each item's docs name its
//! wire element and its section, and every enum variant records its exact wire token.
//!
//! # Namespace prefixes
//!
//! The fidelity reader resolves an *element's* namespace but leaves an *attribute's* prefix
//! unresolved, and VML is the one vocabulary where that matters (a `v:shape` carries an unprefixed
//! `id` **and** children with an `r:id`). Unprefixed attributes are therefore matched exactly; a
//! namespaced one is matched on whatever prefix the element carrying it declares for that namespace,
//! and otherwise on the conventional prefix ECMA-376 Part 4 §19 binds it to in every example and that
//! every producer emits — `v`, `o`, `p`, `x`, `w10` and `r`. If an element rebinds one of those to a
//! different namespace, the lookup answers `None` rather than matching the wrong attribute.
//!
//! # Fidelity
//!
//! Every modelled type keeps the element's name (prefix included), its attributes in source order,
//! its self-closing flag, and every child it does not itself model. A drawing parsed and re-emitted
//! without an edit is byte-identical, and an edit to one shape leaves its siblings untouched.

mod build;
mod control;
mod drawing;
mod error;
mod office;
mod shape;

pub use control::{AttachedObjectData, AttachedObjectKind};
pub use drawing::{Drawing, DrawingContent, DrawingPart};
pub use error::VmlError;
pub use office::{
    EmbeddedOleObject, Ink, OleDrawAspect, OleObjectKind, OleUpdateMode, ShapeIdMap, ShapeLayout,
    ShapeLayoutContent, ShapeProtections,
};
pub use shape::{
    DiagramText, Fill, ImageData, Shape, ShapeContent, ShapeGroup, ShapeGroupContent, ShapePath,
    ShapeTemplate, Stroke, TextBox,
};

/// The content type of a legacy VML drawing part (`ppt/drawings/vmlDrawingN.vml`,
/// `word/media/…`, etc.).
///
/// Registered in `[Content_Types].xml` as a Default keyed on the [`VML_DEFAULT_EXTENSION`]
/// extension (VML parts do not share the `.xml` extension, so a Default suffices — no per-part
/// Override is needed).
pub const CONTENT_TYPE_VML: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";

/// The relationship type from a part (a slide, header, footer, …) to a VML drawing part.
pub const REL_VML_DRAWING: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing";

/// The file extension of a VML drawing part, without the leading dot — the key under which
/// [`CONTENT_TYPE_VML`] is registered as a Content-Types Default.
pub const VML_DEFAULT_EXTENSION: &str = "vml";

/// Whether `content_type` names a legacy VML drawing part.
#[must_use]
pub fn is_vml_content_type(content_type: &str) -> bool {
    content_type == CONTENT_TYPE_VML
}

/// Reads an `ST_TrueFalse` / `ST_TrueFalseBlank` value (ECMA-376 Part 4 §19.7.3), or `None` for one
/// neither type defines.
///
/// `ST_TrueFalse` admits `t`/`f`/`true`/`false`; `ST_TrueFalseBlank` adds `True`/`False` and the
/// empty string. Producers also write the `0`/`1` spelling VML's HTML ancestry accepted, so both are
/// read. The empty string is *not* handled here: it means different things in different places (a
/// value-less `x:ClientData` child is true, a blank attribute is unset), so each caller decides.
pub(crate) fn true_false(value: &str) -> Option<bool> {
    match value {
        "t" | "true" | "True" | "1" => Some(true),
        "f" | "false" | "False" | "0" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_the_wire_strings() {
        assert_eq!(
            CONTENT_TYPE_VML,
            "application/vnd.openxmlformats-officedocument.vmlDrawing"
        );
        assert_eq!(
            REL_VML_DRAWING,
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing"
        );
        assert_eq!(VML_DEFAULT_EXTENSION, "vml");
    }

    #[test]
    fn predicate_recognizes_only_the_vml_content_type() {
        assert!(is_vml_content_type(CONTENT_TYPE_VML));
        assert!(!is_vml_content_type(
            "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
        ));
        assert!(!is_vml_content_type(""));
    }

    #[test]
    fn true_false_reads_every_spelling_the_schema_admits() {
        for value in ["t", "true", "True", "1"] {
            assert_eq!(true_false(value), Some(true), "{value}");
        }
        for value in ["f", "false", "False", "0"] {
            assert_eq!(true_false(value), Some(false), "{value}");
        }
        assert_eq!(true_false(""), None);
        assert_eq!(true_false("yes"), None);
    }
}
