//! `m:argPr` / the argument wrapper every math object's operand is: `CT_OMathArgPr` and
//! `CT_OMathArg` — the recursive core every object in `crate::objects` bottoms out at.

use mjx_ooxml_core::{FromXml, Interner, Number, RawAttribute, RawName, RawNode, ToXml};

use crate::leaf::ControlProperties;
use crate::objects::MathElement;
use crate::support::{fidelity_element_impls, m_child, read_val_child};

/// `m:argPr` (`CT_OMathArgPr`, §22.1.2.14) — one argument's own size override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(ArgumentProperties);

impl ArgumentProperties {
    /// `m:argSz` (`ST_Integer2`) — the argument's own size adjustment relative to its container's.
    #[must_use]
    pub fn argument_size(&self, interner: &Interner) -> Option<i64> {
        read_val_child::<Number<i64>>(&self.children, interner, "argSz")
    }
}

/// `m:e` / `m:num` / `m:den` / `m:sub` / `m:sup` / `m:deg` / `m:lim` / `m:fName` — every named slot
/// in `crate::objects` is a `CT_OMathArg` (§22.1.2.13 "Argument"): an optional size override,
/// zero or more [`MathElement`]s (the schema's own `EG_OMathElements`, `maxOccurs="unbounded"` —
/// this is the recursive step that lets a fraction's numerator itself be a radical containing an
/// n-ary with sub/superscripts), and an optional trailing control-properties pass-through.
///
/// Which wire element (`e`, `num`, `den`, …) an `Argument` came from is carried by its own retained
/// element name — the object accessors in `crate::objects` (`Fraction::numerator`,
/// `Fraction::denominator`, …) are what give it its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Argument);

impl Argument {
    /// Builds `<m:{local}>{elements}</m:{local}>` — `local` is the wire role (`"e"`, `"num"`,
    /// `"den"`, `"sub"`, `"sup"`, `"deg"`, `"lim"`, `"fName"`, …), and `elements` this argument's own
    /// math content, in order. No `m:argPr`/`m:ctrlPr`.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, elements: &[MathElement]) -> Self {
        let children: Vec<RawNode> = elements
            .iter()
            .map(|element| RawNode::Element(element.to_xml(interner)))
            .collect();
        let empty = children.is_empty();
        Self {
            name: crate::support::m_name(interner, local),
            attributes: Vec::new(),
            children,
            empty,
        }
    }

    /// Builds an argument holding one plain-text run: `<m:{local}><m:r><m:t>{text}</m:t></m:r></m:{local}>`.
    #[must_use]
    pub fn with_text(interner: &mut Interner, local: &str, text: &str) -> Self {
        let run = crate::objects::Run::new(interner, text);
        Self {
            name: crate::support::m_name(interner, local),
            attributes: Vec::new(),
            children: vec![RawNode::Element(run.to_xml(interner))],
            empty: false,
        }
    }

    /// The wire local name this argument was read from (`"e"`, `"num"`, `"den"`, `"sub"`, `"sup"`,
    /// `"deg"`, `"lim"`, `"fName"`, …) — see this type's own doc comment for why no separate variant
    /// exists per role.
    #[must_use]
    pub fn local_name(&self, interner: &Interner) -> String {
        interner.resolve(self.name.local).to_owned()
    }

    /// `m:argPr` — this argument's own size override, if it declares one.
    #[must_use]
    pub fn properties(&self, interner: &Interner) -> Option<ArgumentProperties> {
        m_child(&self.children, interner, "argPr")
            .and_then(|el| ArgumentProperties::from_xml(el, interner).ok())
    }

    /// This argument's own math content, in document order — the recursive step: each
    /// [`MathElement`] may itself be a [`crate::Fraction`], [`crate::Radical`], … , each of which
    /// holds further `Argument`s of its own.
    #[must_use]
    pub fn elements(&self, interner: &Interner) -> Vec<MathElement> {
        MathElement::from_children(&self.children, interner)
    }

    /// `m:ctrlPr` — this argument's own trailing control-properties pass-through, if it declares one.
    #[must_use]
    pub fn control_properties(&self, interner: &Interner) -> Option<ControlProperties> {
        m_child(&self.children, interner, "ctrlPr")
            .and_then(|el| ControlProperties::from_xml(el, interner).ok())
    }
}
