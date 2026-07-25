//! `mjx-vml` — Legacy VML (Transitional-only), feature-gated, preserve-first.
//!
//! VML (Vector Markup Language) is the legacy drawing markup carried in the *Transitional*
//! flavour of OOXML (ECMA-376 Part 4) and dropped from *Strict*. Producers still emit it for
//! constructs with no DrawingML equivalent — OLE-object fallbacks, comment authoring shapes, ink,
//! and legacy form controls — as standalone `vmlDrawingN.vml` parts referenced by relationship id,
//! typically wrapped in `mc:AlternateContent`/`mc:Fallback`.
//!
//! This crate is deliberately **preserve-first**: VML parts are *recognized* so callers can find and
//! read them, but their XML is **not modeled**. Untouched parts already round-trip byte-identically
//! through the generic part-level copy-on-write in `mjx-opc`; this crate adds only the vocabulary
//! (content type, relationship type, extension) and a recognition predicate that format crates use
//! to surface those parts. Rich modeling and shape-level references (OLE / ActiveX / ink) are a later
//! phase.
//!
//! The VML namespaces themselves (`urn:schemas-microsoft-com:vml`, …) live as constants in
//! `mjx-ooxml-types` (`namespaces::VML_MAIN` and friends).

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
///
/// Preserve-first: this only *recognizes* VML by its content type; it never parses or models the
/// part's markup.
#[must_use]
pub fn is_vml_content_type(content_type: &str) -> bool {
    content_type == CONTENT_TYPE_VML
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
}
