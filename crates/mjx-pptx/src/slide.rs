//! Navigation of a slide's shape tree (`p:sld > p:cSld > p:spTree > p:sp > p:txBody`).

use mjx_dml::{ColorMap, ColorSchemeSlot, Fill, StyleMatrixReference};
use mjx_ooxml_core::{FromXml, Interner, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};

use crate::error::PptxError;
use crate::nav;

/// The `p:spPr` child elements that a fill must precede, per `CT_ShapeProperties`'s content order
/// (line, effects, 3-D, extensions). A new fill is inserted before the first of these.
const AFTER_FILL_LOCALS: [&str; 6] = ["ln", "effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

/// The `p:spPr` child elements that the outline (`a:ln`) must precede, per `CT_ShapeProperties`'s
/// content order (effects, 3-D, extensions). This is [`AFTER_FILL_LOCALS`] without the leading `ln`,
/// so a new outline lands after any geometry and fill (neither is in the set) and before effects.
const AFTER_LINE_LOCALS: [&str; 5] = ["effectLst", "effectDag", "scene3d", "sp3d", "extLst"];

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

/// A shape's fill style reference (`p:sp > p:style > a:fillRef`), if it has one — the theme
/// fill-style index and the color that substitutes the style's `phClr`.
pub(crate) fn shape_fill_ref(
    shape: &RawElement,
    interner: &Interner,
) -> Option<StyleMatrixReference> {
    let style = nav::child(shape, interner, PML, "style")?;
    let fill_ref = nav::child(style, interner, DML_MAIN, "fillRef")?;
    StyleMatrixReference::from_xml(fill_ref, interner).ok()
}

/// A shape's placeholder identity (`p:sp > p:nvSpPr > p:nvPr > p:ph`): the `@type` (as the
/// title-family flag) and `@idx`. The slot a placeholder occupies, used to match it against the
/// same-slot placeholder on the slide layout / master when its own fill is inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placeholder {
    /// Whether the placeholder is a title (`type` = `title` or `ctrTitle`) — these share one slot.
    pub title_family: bool,
    /// The placeholder index (`@idx`, default `0`).
    pub idx: u32,
}

impl Placeholder {
    /// Whether `self` and `other` name the same inheritance slot: title-family placeholders match each
    /// other regardless of index; any other placeholder matches by index. (A documented heuristic —
    /// PowerPoint's exact matching has more nuance around body/object placeholders.)
    pub(crate) fn matches(self, other: Placeholder) -> bool {
        if self.title_family || other.title_family {
            self.title_family && other.title_family
        } else {
            self.idx == other.idx
        }
    }
}

/// The placeholder identity of `shape` (`p:nvSpPr > p:nvPr > p:ph`), or `None` if it is not a
/// placeholder shape.
pub(crate) fn shape_placeholder(shape: &RawElement, interner: &Interner) -> Option<Placeholder> {
    let nv_sp_pr = nav::child(shape, interner, PML, "nvSpPr")?;
    let nv_pr = nav::child(nv_sp_pr, interner, PML, "nvPr")?;
    let ph = nav::child(nv_pr, interner, PML, "ph")?;
    let ph_type = nav::attr_value(ph, interner, "type").unwrap_or("obj");
    let idx = nav::attr_value(ph, interner, "idx")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    Some(Placeholder {
        title_family: matches!(ph_type, "title" | "ctrTitle"),
        idx,
    })
}

/// The first `p:sp` in `sp_tree` whose placeholder matches `target` (see [`Placeholder::matches`]).
pub(crate) fn find_placeholder<'a>(
    sp_tree: &'a RawElement,
    target: Placeholder,
    interner: &'a Interner,
) -> Option<&'a RawElement> {
    shapes(sp_tree, interner)
        .find(|shape| shape_placeholder(shape, interner).is_some_and(|ph| ph.matches(target)))
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

/// A shape's explicit outline element (`p:spPr > a:ln`), if present. Returns `None` when the shape has
/// no `p:spPr` or no `a:ln` (its outline is then inherited).
pub(crate) fn shape_line<'a>(shape: &'a RawElement, interner: &Interner) -> Option<&'a RawElement> {
    let sp_pr = nav::child(shape, interner, PML, "spPr")?;
    nav::child(sp_pr, interner, DML_MAIN, "ln")
}

/// The index of `sp_pr`'s existing outline child (`a:ln`), if any.
pub(crate) fn line_child_index(sp_pr: &RawElement, interner: &Interner) -> Option<usize> {
    sp_pr
        .children
        .iter()
        .position(|node| dml_element_local(node, interner) == Some("ln"))
}

/// Where a new outline child (`a:ln`) should be inserted in `sp_pr`: before the first
/// effect/3-D/extension child (so it lands after any geometry and fill), or at the end when none is
/// present.
pub(crate) fn line_insert_index(sp_pr: &RawElement, interner: &Interner) -> usize {
    sp_pr
        .children
        .iter()
        .position(|node| {
            dml_element_local(node, interner)
                .is_some_and(|local| AFTER_LINE_LOCALS.contains(&local))
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
    fn finds_existing_line() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/><a:ln/>"#);
        assert_eq!(line_child_index(&el, &interner), Some(3));
    }

    #[test]
    fn no_line_child_when_absent() {
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/>"#);
        assert_eq!(line_child_index(&el, &interner), None);
    }

    #[test]
    fn line_insert_index_lands_after_fill_before_effects() {
        // The outline inserts after geometry+fill and before the effect list.
        let (el, interner) =
            sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/><a:effectLst/>"#);
        assert_eq!(line_insert_index(&el, &interner), 3);
    }

    #[test]
    fn line_insert_index_appends_when_no_trailing_children() {
        // No effect/3-D/ext child: the outline appends after geometry and fill.
        let (el, interner) = sp_pr(r#"<a:xfrm/><a:prstGeom prst="rect"/><a:solidFill/>"#);
        assert_eq!(line_insert_index(&el, &interner), 3);
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

    fn sp(inner: &str) -> (RawElement, mjx_ooxml_core::Interner) {
        let fragment = format!(r#"<p:sp xmlns:p="{P}" xmlns:a="{A}">{inner}</p:sp>"#);
        let doc = fidelity::parse(fragment.as_bytes()).expect("parse");
        (doc.root, doc.interner)
    }

    #[test]
    fn shape_placeholder_reads_type_and_idx_with_defaults() {
        let (shape, interner) = sp(
            r#"<p:nvSpPr><p:cNvPr id="2" name="T"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr/>"#,
        );
        let ph = shape_placeholder(&shape, &interner).expect("placeholder");
        assert!(ph.title_family);
        assert_eq!(ph.idx, 0);

        // A body placeholder with an explicit idx.
        let (shape, interner) = sp(
            r#"<p:nvSpPr><p:cNvPr id="3" name="B"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>"#,
        );
        let ph = shape_placeholder(&shape, &interner).expect("placeholder");
        assert!(!ph.title_family);
        assert_eq!(ph.idx, 1);

        // No p:ph -> not a placeholder.
        let (shape, interner) =
            sp(r#"<p:nvSpPr><p:cNvPr id="4" name="X"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>"#);
        assert!(shape_placeholder(&shape, &interner).is_none());
    }

    #[test]
    fn placeholder_matching_rules() {
        let title = Placeholder {
            title_family: true,
            idx: 0,
        };
        let ctr_title = Placeholder {
            title_family: true,
            idx: 5,
        };
        let body0 = Placeholder {
            title_family: false,
            idx: 0,
        };
        let body1 = Placeholder {
            title_family: false,
            idx: 1,
        };
        // Title-family match regardless of idx.
        assert!(title.matches(ctr_title));
        // Body matches by idx.
        assert!(body0.matches(Placeholder {
            title_family: false,
            idx: 0
        }));
        assert!(!body0.matches(body1));
        // Title never matches body.
        assert!(!title.matches(body0));
    }

    #[test]
    fn find_placeholder_picks_the_matching_shape() {
        let (sp_tree, interner) = element(format!(
            concat!(
                r#"<p:spTree xmlns:p="{P}" xmlns:a="{A}">"#,
                r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="B"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr></p:sp>"#,
                r#"<p:sp><p:nvSpPr><p:cNvPr id="3" name="T"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr></p:sp>"#,
                r#"</p:spTree>"#
            ),
            P = P,
            A = A
        ));
        let title_target = Placeholder {
            title_family: true,
            idx: 0,
        };
        let found = find_placeholder(&sp_tree, title_target, &interner).expect("title match");
        assert!(shape_placeholder(found, &interner).unwrap().title_family);

        // No matching body idx.
        assert!(find_placeholder(
            &sp_tree,
            Placeholder {
                title_family: false,
                idx: 9
            },
            &interner
        )
        .is_none());
    }
}
