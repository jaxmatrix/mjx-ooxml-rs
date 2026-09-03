//! `w:body` (`CT_Body`) — the skeleton MJXOFF-92 extends with the block-level content model.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

/// `CT_Body` — a document's or glossary document's body: block-level content (paragraphs, tables —
/// `EG_BlockLevelElts`), then the last section's properties (`w:sectPr`).
///
/// **Skeleton.** Per the schema, `CT_Body`'s content is `sequence(EG_BlockLevelElts{0,unbounded},
/// sectPr?)` — the group is real modeling work MJXOFF-92 owns, and `CT_SectPrBase`'s own ordering
/// row is already generated (`mjx_ooxml_types::child_order::SECTION_PROPERTIES_BASE`) for whichever
/// later child sections that content. Until then this type is a **fidelity wrapper**: it preserves
/// the element's name, attributes, self-closing flag and every child exactly as `mjx-dml`'s
/// `TextBody` (see that type's own doc comment for the general shape) preserves what it does not yet
/// model — here, everything. [`MainDocument::body`](super::MainDocument::body) hands out a `&Body`
/// today; MJXOFF-92 gives it a real content field and switches these two hand-written impls for
/// `#[derive(FromXml, ToXml)]` plus a `BodyContent` enum — the same refinement `mjx-dml`'s
/// `GeometryGuide` documents for a leaf with no typed children yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Body {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Body {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        // Preserve the self-closing flag, but never contradict "self-closing ⇒ no children".
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// `w:background` (`CT_Background`) — a document's or glossary document's page background, the one
/// child `CT_DocumentBase` contributes (`CT_DocumentBase` itself has no serialized form: it is
/// spliced into `CT_Document`/`CT_GlossaryDocument` by `xsd:complexContent`/`xsd:extension`, never a
/// wire element of its own — see `xtask/src/codegen/complex.rs`).
///
/// **Skeleton**, for the same reason as [`Body`]: `CT_Background`'s own content (a repeating choice
/// of VML/Office-drawing wildcards, then an optional `w:drawing`) and its four color attributes are
/// real modeling work nobody has claimed yet. `tests/fixtures/sample.docx`'s `w:document` carries no
/// `w:background` at all, so this type is exercised by the schema's own permission for it to be
/// absent, not by that fixture's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl FromXml for Background {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            children: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Background {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.children.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}
