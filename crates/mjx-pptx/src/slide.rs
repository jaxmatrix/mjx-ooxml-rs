//! Navigation of a slide's shape tree (`p:sld > p:cSld > p:spTree > p:sp > p:txBody`).

use mjx_dml::{ColorMap, ColorSchemeSlot, Fill};
use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};

use crate::error::PptxError;
use crate::nav;

/// The `p:spPr` child elements that a fill must precede, per `CT_ShapeProperties`'s content order
/// (line, effects, 3-D, extensions). A new fill is inserted before the first of these.
const AFTER_FILL_LOCALS: [&str; 6] = ["ln", "effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// The `p:spTree` of a slide (`slide_root` is the `p:sld`).
pub(crate) fn sp_tree<'a>(
    slide_root: &'a RawElement,
    interner: &Interner,
) -> Result<&'a RawElement, PptxError> {
    let c_sld = nav::child(slide_root, interner, PML, "cSld")
        .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
    nav::child(c_sld, interner, PML, "spTree").ok_or(PptxError::MalformedSlide("missing p:spTree"))
}

/// The `p:spTree` of a slide, mutably.
pub(crate) fn sp_tree_mut<'a>(
    slide_root: &'a mut RawElement,
    interner: &Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let c_sld = nav::child_mut(slide_root, interner, PML, "cSld")
        .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
    nav::child_mut(c_sld, interner, PML, "spTree")
        .ok_or(PptxError::MalformedSlide("missing p:spTree"))
}

/// The `p:sp` shape children of a shape tree, in order (skips `p:nvGrpSpPr` / `p:grpSpPr`).
pub(crate) fn shapes<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
) -> impl Iterator<Item = &'a RawElement> {
    nav::children(sp_tree, interner, PML, "sp")
}

/// A shape's `p:txBody`, if it has one.
pub(crate) fn shape_txbody<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    nav::child(shape, interner, PML, "txBody")
}

/// A shape's preset geometry (`p:spPr > a:prstGeom`), if it has one. A shape with custom geometry
/// (`a:custGeom`) or an inherited placeholder geometry returns `None`.
pub(crate) fn shape_prstgeom<'a>(
    shape: &'a RawElement,
    interner: &Interner,
) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "prstGeom")
}

/// Parses a `p:clrMap` / `a:overrideClrMapping` element into a [`ColorMap`] — the twelve logical
/// color-name attributes, each a `ST_ColorSchemeIndex` token. Returns `None` if any of the twelve is
/// absent or unrecognized (e.g. the schema-loose attribute-less `overrideClrMapping` some files emit).
pub(crate) fn parse_color_map(element: &RawElement, interner: &Interner) -> Option<ColorMap> {
    let slot =
        |local| nav::attr_value(element, interner, local).and_then(ColorSchemeSlot::from_wire);
    Some(ColorMap {
        background1: slot("bg1")?,
        text1: slot("tx1")?,
        background2: slot("bg2")?,
        text2: slot("tx2")?,
        accent1: slot("accent1")?,
        accent2: slot("accent2")?,
        accent3: slot("accent3")?,
        accent4: slot("accent4")?,
        accent5: slot("accent5")?,
        accent6: slot("accent6")?,
        hyperlink: slot("hlink")?,
        followed_hyperlink: slot("folHlink")?,
    })
}

/// The local name of `node` if it is a DrawingML-main element (accepting both URIs), else `None`.
fn dml_element_local<'a>(node: &'a RawNode, interner: &'a Interner) -> Option<&'a str> {
    match node {
        RawNode::Element(element) => {
            let namespace = element
                .name
                .namespace
                .map(|symbol| interner.resolve(symbol));
            if namespace == Some(DML_MAIN.transitional) || namespace == DML_MAIN.strict {
                Some(interner.resolve(element.name.local))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A shape's explicit fill element (`p:spPr`'s `EG_FillProperties` child), if present. Returns `None`
/// when the shape has no `p:spPr` or no explicit fill (its fill is then inherited).
pub(crate) fn shape_fill<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    sp_pr.children.iter().find_map(|node| match node {
        RawNode::Element(element)
            if dml_element_local(node, interner).is_some_and(Fill::is_fill_local) =>
        {
            Some(element)
        }
        _ => None,
    })
}

/// The index of `sp_pr`'s existing fill child (`EG_FillProperties`), if any.
pub(crate) fn fill_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner).is_some_and(Fill::is_fill_local))
}

/// Where a new fill child should be inserted in `sp_pr`: before the first line/effect/3-D/extension
/// child (so it lands after any geometry), or at the end when none is present.
pub(crate) fn fill_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_FILL_LOCALS.contains(&local))
        })
        .unwrap_or(sp_pr.children.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_xml::fidelity;

    const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
    const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

    fn sp_pr(inner: &str) -> (RawElement, mjx_ooxml_core::Interner) {
        let fragment = format!(r#"<p:spPr xmlns:p="{P}" xmlns:a="{A}">{inner}</p:spPr>"#);
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    fn element(fragment: String) -> (RawElement, mjx_ooxml_core::Interner) {
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    #[test]
    fn finds_existing_fill_of_any_kind() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:gradFill/><a:ln/>"#);
        assert_eq!(fill_child_index(&el, &interner), Some(2));
    }

    #[test]
    fn no_fill_child_when_absent() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/>"#);
        assert_eq!(fill_child_index(&el, &interner), None);
    }

    #[test]
    fn insert_index_lands_after_geometry_before_line() {
        // With an a:ln present, the fill inserts right before it (after geometry).
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:ln/>"#);
        assert_eq!(fill_insert_index(&el, &interner), 2);
    }

    #[test]
    fn insert_index_appends_when_no_trailing_children() {
        // No line/effect/3-D/ext child: the fill appends after geometry.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/>"#);
        assert_eq!(fill_insert_index(&el, &interner), 2);
    }

    #[test]
    fn parse_color_map_reads_twelve_slots() {
        let (map_el, interner) = element(format!(
            concat!(
                r#"<p:clrMap xmlns:p="{P}" bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" "#,
                r#"accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" "#,
                r#"accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#
            ),
            P = P
        ));
        let map = parse_color_map(&map_el, &interner).expect("color map");
        assert_eq!(
            map.resolve(mjx_dml::SchemeColor::Background1),
            Some(ColorSchemeSlot::Light1)
        );
        assert_eq!(
            map.resolve(mjx_dml::SchemeColor::Text1),
            Some(ColorSchemeSlot::Dark1)
        );
    }

    #[test]
    fn parse_color_map_rejects_attribute_less_override() {
        // A schema-loose, attribute-less overrideClrMapping yields no map (caller falls back).
        let (map_el, interner) = element(format!(r#"<a:overrideClrMapping xmlns:a="{A}"/>"#));
        assert!(parse_color_map(&map_el, &interner).is_none());
    }
}
