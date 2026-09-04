//! `pic:pic` (`CT_Picture`, `dml-picture.xsd`) — a picture: its non-visual identity, the image it
//! shows (`a:blipFill`), and its shape properties (`a:spPr`) — the exact `CT_ShapeProperties`
//! [`crate::shape_properties::ShapeProperties`] models, so a picture's crop rectangle, outline and
//! transform read through the same accessors a shape's do.
//!
//! `dml-picture.xsd` is three complex types and eleven lines: `CT_Picture` (`nvPicPr`, `blipFill`,
//! `spPr`), `CT_PictureNonVisual` (`cNvPr` + `cNvPicPr`) and the `pic` root element. Both are modeled
//! here in full — there is no opaque bucket left in the non-visual identity, unlike the "locking"
//! wrappers [`crate::nonvisual`] leaves deliberately shallow.

use mjx_ooxml_core::{
    AttributeError, FromXml, FromXmlError, Interner, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::namespaces::DML_PICTURE;

use crate::fill::PictureFill;
use crate::nonvisual::{NonVisualDrawingProps, NonVisualPictureProperties};
use crate::shape_properties::ShapeProperties;

/// Builds a `pic:local` qualified name in the DrawingML-picture namespace.
fn pic_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("pic")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_PICTURE.transitional)),
    }
}

/// Whether `name` is in the `pic:` namespace, matching both its Strict and Transitional URIs.
fn is_pic(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_PICTURE.transitional) || namespace == DML_PICTURE.strict
}

/// The first `pic:`-namespaced element in `children` named `local` — `nvPicPr`/`blipFill`/`spPr`
/// (and, one level down, `cNvPr`/`cNvPicPr`) are all local element declarations inside
/// `dml-picture.xsd` itself, so they take `pic:`'s own namespace even though `a:CT_ShapeProperties`/
/// `a:CT_BlipFillProperties`/`a:CT_NonVisualDrawingProps` are DrawingML-main *types* — the same
/// distinction `mjx_dml::wordprocessing_drawing`'s own module doc draws for `wp:docPr`.
fn pic_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_pic(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}

/// `pic:nvPicPr` (`CT_PictureNonVisual`) — a picture's non-visual identity: `cNvPr` (its id/name/
/// description/hidden/title) and `cNvPicPr` (its lock list and `preferRelativeResize`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureNonVisual {
    drawing_props: NonVisualDrawingProps,
    picture_props: NonVisualPictureProperties,
}

impl PictureNonVisual {
    /// Builds `<pic:nvPicPr><pic:cNvPr id="{id}" name="{name}"/><pic:cNvPicPr/></pic:nvPicPr>`.
    #[must_use]
    pub fn new(interner: &mut Interner, id: u32, name: &str) -> Self {
        let cnv_pr_name = pic_name(interner, "cNvPr");
        let cnv_pic_pr_name = pic_name(interner, "cNvPicPr");
        Self {
            drawing_props: NonVisualDrawingProps::with_name(interner, cnv_pr_name, id, name),
            picture_props: NonVisualPictureProperties::with_name(interner, cnv_pic_pr_name),
        }
    }

    /// The picture's own identity (`pic:cNvPr`).
    #[must_use]
    pub fn drawing_props(&self) -> &NonVisualDrawingProps {
        &self.drawing_props
    }

    /// The picture's own identity (`pic:cNvPr`), mutably.
    pub fn drawing_props_mut(&mut self) -> &mut NonVisualDrawingProps {
        &mut self.drawing_props
    }

    /// The picture's own lock list and resize preference (`pic:cNvPicPr`).
    #[must_use]
    pub fn picture_props(&self) -> &NonVisualPictureProperties {
        &self.picture_props
    }
}

impl FromXml for PictureNonVisual {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let cnv_pr =
            pic_child(&element.children, interner, "cNvPr").ok_or(AttributeError::Missing {
                attribute: "pic:cNvPr",
            })?;
        let cnv_pic_pr =
            pic_child(&element.children, interner, "cNvPicPr").ok_or(AttributeError::Missing {
                attribute: "pic:cNvPicPr",
            })?;
        Ok(Self {
            drawing_props: NonVisualDrawingProps::from_xml(cnv_pr, interner)?,
            picture_props: NonVisualPictureProperties::from_xml(cnv_pic_pr, interner)?,
        })
    }
}

impl ToXml for PictureNonVisual {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let cnv_pr = RawNode::Element(self.drawing_props.to_xml(interner));
        let cnv_pic_pr = RawNode::Element(self.picture_props.to_xml(interner));
        RawElement::rebuilt(
            pic_name(interner, "nvPicPr"),
            Vec::new(),
            vec![cnv_pr, cnv_pic_pr],
            false,
        )
    }
}

/// `pic:pic` (`CT_Picture`) — a picture: `nvPicPr`, `blipFill` (required, unlike `spPr`'s own
/// `EG_FillProperties` — this is *the* fill, not one alternative), then `spPr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    non_visual: PictureNonVisual,
    fill: PictureFill,
    shape_properties: ShapeProperties,
}

impl Picture {
    /// Builds a picture with the given non-visual identity, image fill and shape properties.
    #[must_use]
    pub fn new(
        non_visual: PictureNonVisual,
        fill: PictureFill,
        shape_properties: ShapeProperties,
    ) -> Self {
        Self {
            non_visual,
            fill,
            shape_properties,
        }
    }

    /// The picture's non-visual identity (`pic:nvPicPr`).
    #[must_use]
    pub fn non_visual(&self) -> &PictureNonVisual {
        &self.non_visual
    }

    /// The picture's non-visual identity (`pic:nvPicPr`), mutably.
    pub fn non_visual_mut(&mut self) -> &mut PictureNonVisual {
        &mut self.non_visual
    }

    /// The picture's image fill (`pic:blipFill`), holding the `a:blip@r:embed`/`@r:link`
    /// relationship id that resolves to the image part.
    #[must_use]
    pub fn fill(&self) -> &PictureFill {
        &self.fill
    }

    /// The picture's image fill (`pic:blipFill`), mutably.
    pub fn fill_mut(&mut self) -> &mut PictureFill {
        &mut self.fill
    }

    /// The picture's shape properties (`pic:spPr`) — transform, crop geometry, outline.
    #[must_use]
    pub fn shape_properties(&self) -> &ShapeProperties {
        &self.shape_properties
    }

    /// The picture's shape properties (`pic:spPr`), mutably.
    pub fn shape_properties_mut(&mut self) -> &mut ShapeProperties {
        &mut self.shape_properties
    }

    /// The embedded image relationship id (`pic:blipFill/a:blip@r:embed`), or `None` if this
    /// picture links an external image instead (`@r:link`) or names neither.
    #[must_use]
    pub fn image_rel_id(&self, interner: &Interner) -> Option<String> {
        self.fill.image_rel_id(interner)
    }
}

impl FromXml for Picture {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let nv_pic_pr =
            pic_child(&element.children, interner, "nvPicPr").ok_or(AttributeError::Missing {
                attribute: "pic:nvPicPr",
            })?;
        let blip_fill =
            pic_child(&element.children, interner, "blipFill").ok_or(AttributeError::Missing {
                attribute: "pic:blipFill",
            })?;
        let sp_pr =
            pic_child(&element.children, interner, "spPr").ok_or(AttributeError::Missing {
                attribute: "pic:spPr",
            })?;
        Ok(Self {
            non_visual: PictureNonVisual::from_xml(nv_pic_pr, interner)?,
            fill: PictureFill::from_xml(blip_fill, interner)?,
            shape_properties: ShapeProperties::from_xml(sp_pr, interner)?,
        })
    }
}

impl ToXml for Picture {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let children = vec![
            RawNode::Element(self.non_visual.to_xml(interner)),
            RawNode::Element(self.fill.to_xml(interner)),
            RawNode::Element(self.shape_properties.to_xml(interner)),
        ];
        RawElement::rebuilt(pic_name(interner, "pic"), Vec::new(), children, false)
    }
}

/// A fresh, minimally-complete picture: `id`/`name` for its identity, `rel_id` naming the image part
/// relationship, and a `1 x 1` EMU transform placeholder a caller is expected to resize immediately
/// (the schema requires `a:off`/`a:ext` to be present once `a:xfrm` is written at all, so an inserted
/// picture cannot leave them unstated).
#[must_use]
pub fn new_picture(interner: &mut Interner, id: u32, name: &str, rel_id: &str) -> Picture {
    let non_visual = PictureNonVisual::new(interner, id, name);
    let blip_fill_name = pic_name(interner, "blipFill");
    let fill = PictureFill::with_name(
        interner,
        blip_fill_name,
        rel_id,
        crate::fill::PictureFillMode::Stretch,
    );
    let sp_pr_name = pic_name(interner, "spPr");
    let mut shape_properties = ShapeProperties::with_name(interner, sp_pr_name);
    shape_properties.set_transform(
        interner,
        crate::geometry::Transform2D {
            position: Some(crate::geometry::Position::from_emu(0, 0)),
            size: Some(crate::geometry::Size::from_emu(1, 1)),
            ..crate::geometry::Transform2D::default()
        },
    );
    let rectangle = crate::geometry::PresetGeometry::new(
        interner,
        mjx_ooxml_types::drawingml::PresetShapeType::Rectangle,
        None,
    );
    shape_properties.set_geometry(
        interner,
        crate::shape_properties::ShapeGeometryChoice::Preset(rectangle),
    );
    Picture::new(non_visual, fill, shape_properties)
}
