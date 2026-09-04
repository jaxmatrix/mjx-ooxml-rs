//! The three global elements `shared-math.xsd` declares — `m:oMath`, `m:oMathPara`, `m:mathPr` — and
//! `m:oMathPara`'s own paragraph properties. [`Math`] (`m:oMath`) is the crate's own entry point:
//! every fixture and every Word-side integration reads or writes one of these.

use mjx_ooxml_core::{Enumeration, FromXml, Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::officemath::Justification;

use crate::objects::MathElement;
use crate::support::{fidelity_element_impls, m_child, m_children, m_name, read_val_child};

/// `m:oMathParaPr` (`CT_OMathParaPr`, §22.1.2.76) — a math paragraph's own justification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathParagraphProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(MathParagraphProperties);

impl MathParagraphProperties {
    /// `m:jc` (`CT_OMathJc`, `ST_Jc`) — how the paragraph's own equations are justified.
    #[must_use]
    pub fn justification(&self, interner: &Interner) -> Option<Justification> {
        read_val_child::<Enumeration<Justification>>(&self.children, interner, "jc")
    }
}

/// `m:oMath` (`CT_OMath`, §22.1.2.77 "Office Math") — one equation: a sequence of math objects and
/// runs. The crate's own primary entry point — see the crate root doc comment for a worked example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Math {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Math);

impl Math {
    /// Builds an empty `<m:oMath/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: m_name(interner, "oMath"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// Builds `<m:oMath>{elements}</m:oMath>`.
    #[must_use]
    pub fn with_elements(interner: &mut Interner, elements: &[MathElement]) -> Self {
        let children = elements
            .iter()
            .map(|element| RawNode::Element(element.to_xml(interner)))
            .collect();
        Self {
            name: m_name(interner, "oMath"),
            attributes: Vec::new(),
            children,
            empty: elements.is_empty(),
        }
    }

    /// This equation's own content, in document order — the top of the recursive structure
    /// [`crate::arg::Argument::elements`] continues at every nesting level below it.
    #[must_use]
    pub fn elements(&self, interner: &Interner) -> Vec<MathElement> {
        MathElement::from_children(&self.children, interner)
    }
}

/// `m:oMathPara` (`CT_OMathPara`, §22.1.2.78 "Office Math Paragraph") — a paragraph of one or more
/// display equations sharing one justification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathParagraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(MathParagraph);

impl MathParagraph {
    /// Builds an empty `<m:oMathPara/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: m_name(interner, "oMathPara"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// `m:oMathParaPr` — this math paragraph's own properties, if it declares any.
    #[must_use]
    pub fn properties(&self, interner: &Interner) -> Option<MathParagraphProperties> {
        m_child(&self.children, interner, "oMathParaPr")
            .and_then(|el| MathParagraphProperties::from_xml(el, interner).ok())
    }

    /// `m:oMath` — this paragraph's own equations, in order (one or more per the schema).
    #[must_use]
    pub fn equations(&self, interner: &Interner) -> Vec<Math> {
        m_children(&self.children, interner, "oMath")
            .filter_map(|el| Math::from_xml(el, interner).ok())
            .collect()
    }
}
