//! The VML shape vocabulary — `v:shape`, `v:shapetype`, `v:group` and the children a shape hangs off
//! itself.
//!
//! Element names follow the ECMA-376 Part 4 §19.1.2 prose, not the wire token: `v:shape` is
//! §19.1.2.19 *shape (Shape Definition)*, `v:shapetype` is §19.1.2.20 *shapetype (Shape Template)*,
//! `v:group` is §19.1.2.7 *group (Shape Group)*, `v:imagedata` is §19.1.2.11 *imagedata (Image
//! Data)*, `v:textbox` is §19.1.2.22 *textbox (Text Box)*, `v:fill` is §19.1.2.5 *fill (Shape Fill
//! Properties)*, `v:stroke` is §19.1.2.21 *stroke (Line Stroke Settings)* and `v:path` is §19.1.2.14
//! *path (Shape Path)*.
//!
//! Every type keeps the element's name, attributes, self-closing flag and the children it does not
//! itself model, so a shape this crate reads and re-emits is byte-identical.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, VML_MAIN, VML_OFFICE_DRAWING};

use crate::build::{self, fidelity_leaf};
use crate::control::AttachedObjectData;
use crate::office::{Ink, ShapeProtections};

/// One ordered child of a [`Shape`] or a [`ShapeTemplate`]: a typed child, or an opaque node kept
/// verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeContent {
    /// The image the shape draws (`v:imagedata`).
    ImageData(ImageData),
    /// The shape's text box (`v:textbox`).
    TextBox(TextBox),
    /// The shape's fill properties (`v:fill`).
    Fill(Fill),
    /// The shape's stroke settings (`v:stroke`).
    Stroke(Stroke),
    /// The shape's path (`v:path`).
    Path(ShapePath),
    /// Ink carried by the shape (`o:ink`).
    Ink(Ink),
    /// The shape's protections (`o:lock`).
    Protections(ShapeProtections),
    /// The data attached to a legacy form control or comment (`x:ClientData`).
    AttachedObjectData(AttachedObjectData),
    /// A reference to the text of a VML diagram node (`p:textdata`).
    DiagramText(DiagramText),
    /// Any other child — `v:formulas`, `v:handles`, `v:shadow`, `v:textpath`, `w10:wrap`, an
    /// unknown extension — kept verbatim.
    Raw(RawNode),
}

/// `v:shape` (`CT_Shape`) — ECMA-376 Part 4 §19.1.2.19 *shape (Shape Definition)*.
///
/// The unit a legacy drawing is made of, and the thing an OLE object frame or an ActiveX control
/// points at: `p:oleObj@spid` / `p:control@spid` in PresentationML, and `o:OLEObject@ShapeID` in
/// WordprocessingML, both name a shape's [`identifier`](Self::identifier).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = VML_MAIN)]
pub struct Shape {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "imagedata", variant = ImageData, ty = ImageData),
        child(local = "textbox", variant = TextBox, ty = TextBox),
        child(local = "fill", variant = Fill, ty = Fill),
        child(local = "stroke", variant = Stroke, ty = Stroke),
        child(local = "path", variant = Path, ty = ShapePath),
        child(ns = VML_OFFICE_DRAWING, local = "ink", variant = Ink, ty = Ink),
        child(ns = VML_OFFICE_DRAWING, local = "lock", variant = Protections, ty = ShapeProtections),
        child(
            ns = VML_SPREADSHEET_DRAWING,
            local = "ClientData",
            variant = AttachedObjectData,
            ty = AttachedObjectData
        ),
        child(
            ns = VML_PRESENTATION_DRAWING,
            local = "textdata",
            variant = DiagramText,
            ty = DiagramText
        )
    )]
    content: Vec<ShapeContent>,
}

/// `v:shapetype` (`CT_Shapetype`) — ECMA-376 Part 4 §19.1.2.20 *shapetype (Shape Template)*.
///
/// A reusable geometry a [`Shape`] adopts by naming it in its `type` attribute with a leading `#`
/// (`type="#_x0000_t202"` adopts the template whose `id` is `_x0000_t202`) — see
/// [`Shape::template_identifier`].
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = VML_MAIN)]
pub struct ShapeTemplate {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "imagedata", variant = ImageData, ty = ImageData),
        child(local = "textbox", variant = TextBox, ty = TextBox),
        child(local = "fill", variant = Fill, ty = Fill),
        child(local = "stroke", variant = Stroke, ty = Stroke),
        child(local = "path", variant = Path, ty = ShapePath),
        child(ns = VML_OFFICE_DRAWING, local = "ink", variant = Ink, ty = Ink),
        child(ns = VML_OFFICE_DRAWING, local = "lock", variant = Protections, ty = ShapeProtections),
        child(
            ns = VML_SPREADSHEET_DRAWING,
            local = "ClientData",
            variant = AttachedObjectData,
            ty = AttachedObjectData
        ),
        child(
            ns = VML_PRESENTATION_DRAWING,
            local = "textdata",
            variant = DiagramText,
            ty = DiagramText
        )
    )]
    content: Vec<ShapeContent>,
}

/// One ordered child of a [`ShapeGroup`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeGroupContent {
    /// A shape in the group (`v:shape`).
    Shape(Shape),
    /// A shape template declared inside the group (`v:shapetype`).
    ShapeTemplate(ShapeTemplate),
    /// A nested group (`v:group`).
    Group(ShapeGroup),
    /// Any other child — the primitive shapes (`v:rect`, `v:oval`, `v:line`, …), `o:diagram`, an
    /// unknown extension — kept verbatim.
    Raw(RawNode),
}

/// `v:group` (`CT_Group`) — ECMA-376 Part 4 §19.1.2.7 *group (Shape Group)*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = VML_MAIN)]
pub struct ShapeGroup {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "shape", variant = Shape, ty = Shape),
        child(local = "shapetype", variant = ShapeTemplate, ty = ShapeTemplate),
        child(local = "group", variant = Group, ty = ShapeGroup)
    )]
    content: Vec<ShapeGroupContent>,
}

/// `v:imagedata` (`CT_ImageData`) — ECMA-376 Part 4 §19.1.2.11 *imagedata (Image Data)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    element: RawElement,
}
fidelity_leaf!(ImageData);

/// `v:textbox` (`CT_Textbox`) — ECMA-376 Part 4 §19.1.2.22 *textbox (Text Box)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBox {
    element: RawElement,
}
fidelity_leaf!(TextBox);

/// `v:fill` (`CT_Fill`) — ECMA-376 Part 4 §19.1.2.5 *fill (Shape Fill Properties)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    element: RawElement,
}
fidelity_leaf!(Fill);

/// `v:stroke` (`CT_Stroke`) — ECMA-376 Part 4 §19.1.2.21 *stroke (Line Stroke Settings)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stroke {
    element: RawElement,
}
fidelity_leaf!(Stroke);

/// `v:path` (`CT_Path`) — ECMA-376 Part 4 §19.1.2.14 *path (Shape Path)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapePath {
    element: RawElement,
}
fidelity_leaf!(ShapePath);

/// `p:textdata` (`CT_Rel`) — ECMA-376 Part 4 §19.5.2.2 *textdata (VML Diagram Text)*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramText {
    element: RawElement,
}
fidelity_leaf!(DiagramText);

impl ImageData {
    /// The relationship id of the image the shape draws (`v:imagedata@r:id`), or `None`.
    #[must_use]
    pub fn relationship_id(&self, interner: &Interner) -> Option<String> {
        build::namespaced_attribute(
            &self.element.attributes,
            interner,
            SHARED_RELATIONSHIP_REFERENCE,
            "id",
        )
        .map(std::borrow::Cow::into_owned)
    }

    /// The relationship id of the image under the Office extension attribute (`v:imagedata@o:relid`),
    /// which Word writes alongside `r:id`, or `None`.
    #[must_use]
    pub fn office_relationship_id(&self, interner: &Interner) -> Option<String> {
        build::namespaced_attribute(
            &self.element.attributes,
            interner,
            VML_OFFICE_DRAWING,
            "relid",
        )
        .map(std::borrow::Cow::into_owned)
    }

    /// The image's title (`v:imagedata@o:title`), or `None`.
    #[must_use]
    pub fn title(&self, interner: &Interner) -> Option<String> {
        build::namespaced_attribute(
            &self.element.attributes,
            interner,
            VML_OFFICE_DRAWING,
            "title",
        )
        .map(std::borrow::Cow::into_owned)
    }
}

impl DiagramText {
    /// The relationship id of the part holding the diagram node's text (`p:textdata@id` — an
    /// unprefixed attribute holding a relationship id, as `CT_Rel` in the PowerPoint drawing schema
    /// declares it), or `None`.
    #[must_use]
    pub fn relationship_id(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.element.attributes, interner, "id").map(std::borrow::Cow::into_owned)
    }
}

/// Generates the shared accessor surface of [`Shape`] and [`ShapeTemplate`], which carry the same
/// core and shape attribute groups (`AG_AllCoreAttributes`, `AG_AllShapeAttributes`) and the same
/// children.
macro_rules! shape_accessors {
    ($ty:ty, $element:literal) => {
        impl $ty {
            /// The element's name as the part spells it, prefix included.
            #[must_use]
            pub fn name(&self) -> &RawName {
                &self.name
            }

            /// Every attribute the element carries, in source order — the escape hatch for one this
            /// crate does not name.
            #[must_use]
            pub fn attributes(&self) -> &[RawAttribute] {
                &self.attributes
            }

            /// The element's children, typed where this crate models them and verbatim where it does
            /// not.
            #[must_use]
            pub fn content(&self) -> &[ShapeContent] {
                &self.content
            }

            /// The element's children, mutably.
            pub fn content_mut(&mut self) -> &mut Vec<ShapeContent> {
                &mut self.content
            }

            /// Appends `child`.
            pub fn push(&mut self, child: ShapeContent) {
                self.content.push(child);
                self.empty = false;
            }

            /// The unique identifier other markup references this by (`@id`) — ECMA-376 Part 4
            /// §19.1.2.19 *id (Unique Identifier)*. This is what `p:oleObj@spid`, `p:control@spid`
            /// and `o:OLEObject@ShapeID` name.
            #[must_use]
            pub fn identifier(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "id")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Sets the unique identifier (`@id`).
            pub fn set_identifier(&mut self, interner: &mut Interner, value: &str) {
                build::set_attribute(&mut self.attributes, interner, "id", value);
            }

            /// The optional string an application uses to identify the shape (`@o:spid`) — ECMA-376
            /// Part 4 §19.1.2.19 *spid (Optional String)*. Distinct from
            /// [`identifier`](Self::identifier), which is what other markup references.
            #[must_use]
            pub fn application_shape_identifier(&self, interner: &Interner) -> Option<String> {
                build::namespaced_attribute(&self.attributes, interner, VML_OFFICE_DRAWING, "spid")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// The CSS2 styling that positions and sizes the shape (`@style`) — ECMA-376 Part 4
            /// §19.1.2.19 *style (Shape Styling Properties)*, e.g.
            /// `position:absolute;margin-left:10pt;width:100pt`.
            #[must_use]
            pub fn style(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "style")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Sets the CSS2 styling (`@style`).
            pub fn set_style(&mut self, interner: &mut Interner, value: &str) {
                build::set_attribute(&mut self.attributes, interner, "style", value);
            }

            /// The identifier of the [`ShapeTemplate`] this adopts (`@type`, with its leading `#`
            /// stripped), or `None` when it declares its own geometry.
            #[must_use]
            pub fn template_identifier(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "type")
                    .map(|value| value.trim_start_matches('#').to_owned())
            }

            /// Points the shape at the [`ShapeTemplate`] with `identifier` (`@type="#identifier"`).
            pub fn set_template_identifier(&mut self, interner: &mut Interner, identifier: &str) {
                build::set_attribute(
                    &mut self.attributes,
                    interner,
                    "type",
                    &format!("#{identifier}"),
                );
            }

            /// The alternative text describing the shape (`@alt`) — ECMA-376 Part 4 §19.1.2.19
            /// *alt (Alternate Text)*.
            #[must_use]
            pub fn alternate_text(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "alt")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Sets the alternative text (`@alt`).
            pub fn set_alternate_text(&mut self, interner: &mut Interner, value: &str) {
                build::set_attribute(&mut self.attributes, interner, "alt", value);
            }

            /// The text shown when the pointer rests on the shape (`@title`) — ECMA-376 Part 4
            /// §19.1.2.19 *title (Shape Title)*.
            #[must_use]
            pub fn title(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "title")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Whether the shape's closed path is filled (`@filled`), or `None` when unstated (the
            /// default is filled). Every `ST_TrueFalse` spelling ECMA-376 Part 4 §19.7.3 admits —
            /// `t`/`true`/`f`/`false`, and the `x`/`0`/`1` forms producers also write — is accepted.
            #[must_use]
            pub fn is_filled(&self, interner: &Interner) -> Option<bool> {
                build::attribute(&self.attributes, interner, "filled")
                    .and_then(|value| crate::true_false(&value))
            }

            /// The colour the shape is filled with (`@fillcolor`), as the part spells it.
            #[must_use]
            pub fn fill_color(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "fillcolor")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Sets the fill colour (`@fillcolor`).
            pub fn set_fill_color(&mut self, interner: &mut Interner, value: &str) {
                build::set_attribute(&mut self.attributes, interner, "fillcolor", value);
            }

            /// Whether the shape's path is stroked (`@stroked`), or `None` when unstated (the default
            /// is stroked).
            #[must_use]
            pub fn is_stroked(&self, interner: &Interner) -> Option<bool> {
                build::attribute(&self.attributes, interner, "stroked")
                    .and_then(|value| crate::true_false(&value))
            }

            /// The hyperlink the shape targets (`@href`), or `None`.
            #[must_use]
            pub fn hyperlink(&self, interner: &Interner) -> Option<String> {
                build::attribute(&self.attributes, interner, "href")
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Whether the shape is an embedded object (`@o:ole`) — ECMA-376 Part 4 §19.1.2.19
            /// *ole (Embedded Object Toggle)*. Word writes it valueless (`o:ole=""`) on the shape
            /// that displays an OLE object, which reads as `Some(true)`.
            #[must_use]
            pub fn is_embedded_object(&self, interner: &Interner) -> Option<bool> {
                let value = build::namespaced_attribute(
                    &self.attributes,
                    interner,
                    VML_OFFICE_DRAWING,
                    "ole",
                )?;
                if value.is_empty() {
                    return Some(true);
                }
                crate::true_false(&value)
            }

            /// The value of the unprefixed attribute `local`, for one this crate does not name.
            #[must_use]
            pub fn attribute(&self, interner: &Interner, local: &str) -> Option<String> {
                build::attribute(&self.attributes, interner, local)
                    .map(::std::borrow::Cow::into_owned)
            }

            /// Sets the unprefixed attribute `local` to `value`, for one this crate does not name.
            pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
                build::set_attribute(&mut self.attributes, interner, local, value);
            }

            /// Removes the unprefixed attribute `local`, if present.
            pub fn remove_attribute(&mut self, interner: &Interner, local: &str) {
                build::remove_attribute(&mut self.attributes, interner, local);
            }

            /// The image the shape draws (`v:imagedata`), or `None`.
            #[must_use]
            pub fn image_data(&self) -> Option<&ImageData> {
                self.content.iter().find_map(|child| match child {
                    ShapeContent::ImageData(value) => Some(value),
                    _ => None,
                })
            }

            /// The shape's text box (`v:textbox`), or `None`.
            #[must_use]
            pub fn text_box(&self) -> Option<&TextBox> {
                self.content.iter().find_map(|child| match child {
                    ShapeContent::TextBox(value) => Some(value),
                    _ => None,
                })
            }

            /// The ink the shape carries (`o:ink`), or `None`.
            #[must_use]
            pub fn ink(&self) -> Option<&Ink> {
                self.content.iter().find_map(|child| match child {
                    ShapeContent::Ink(value) => Some(value),
                    _ => None,
                })
            }

            /// The data attached to the legacy form control or comment this shape draws
            /// (`x:ClientData`), or `None`. This is how a legacy control resolves from the shape that
            /// points at it.
            #[must_use]
            pub fn attached_object_data(&self) -> Option<&AttachedObjectData> {
                self.content.iter().find_map(|child| match child {
                    ShapeContent::AttachedObjectData(value) => Some(value),
                    _ => None,
                })
            }

            /// The reference to the text of the VML diagram node this shape draws (`p:textdata`), or
            /// `None`.
            #[must_use]
            pub fn diagram_text(&self) -> Option<&DiagramText> {
                self.content.iter().find_map(|child| match child {
                    ShapeContent::DiagramText(value) => Some(value),
                    _ => None,
                })
            }

            /// A fresh element with the given unique identifier and CSS2 style, and no children.
            #[must_use]
            pub fn new(interner: &mut Interner, identifier: &str, style: &str) -> Self {
                let name = build::qname(interner, build::VML_PREFIX, VML_MAIN, $element);
                let mut attributes = Vec::with_capacity(2);
                build::set_attribute(&mut attributes, interner, "id", identifier);
                build::set_attribute(&mut attributes, interner, "style", style);
                Self {
                    name,
                    attributes,
                    empty: true,
                    content: Vec::new(),
                }
            }
        }
    };
}

shape_accessors!(Shape, "shape");
shape_accessors!(ShapeTemplate, "shapetype");

impl ShapeGroup {
    /// The element's name as the part spells it, prefix included.
    #[must_use]
    pub fn name(&self) -> &RawName {
        &self.name
    }

    /// Every attribute the group carries, in source order.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// The group's unique identifier (`@id`).
    #[must_use]
    pub fn identifier(&self, interner: &Interner) -> Option<String> {
        build::attribute(&self.attributes, interner, "id").map(std::borrow::Cow::into_owned)
    }

    /// The group's members, typed where this crate models them and verbatim where it does not.
    #[must_use]
    pub fn content(&self) -> &[ShapeGroupContent] {
        &self.content
    }

    /// The group's members, mutably.
    pub fn content_mut(&mut self) -> &mut Vec<ShapeGroupContent> {
        &mut self.content
    }

    /// Appends `child` to the group.
    pub fn push(&mut self, child: ShapeGroupContent) {
        self.content.push(child);
        self.empty = false;
    }
}
