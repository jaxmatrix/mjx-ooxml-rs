//! `m:mathPr` (`CT_MathPr`, §22.1.2.64 "Math Properties") — the document-level math settings
//! (`word/settings.xml`'s own `m:mathPr`, ECMA-376 Part 1 §17.11.16): default font, binary-operator
//! break rule, display defaults, margins/spacing, default justification, and n-ary/integral limit
//! placement. Wiring this into `mjx-docx`'s `settings.rs` is outside this child's own scope (its
//! Word-side integration is `m:oMath`/`m:oMathPara` placement and tracked-change control properties,
//! not document settings) — the type is modelled here, complete, for whichever child does that wiring.

use mjx_ooxml_core::{Enumeration, Interner, RawAttribute, RawName, RawNode};
use mjx_ooxml_types::officemath::{
    BreakBinaryOperator, BreakBinarySubtraction, Justification, LimitLocation,
};

use crate::leaf::{read_onoff_child, MathFontNameCodec, TwipsCodec};
use crate::support::{fidelity_element_impls, read_val_child};

/// `m:mathPr` (`CT_MathPr`) — see this module's own doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(MathProperties);

impl MathProperties {
    /// Builds an empty `<m:mathPr/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: crate::support::m_name(interner, "mathPr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// `m:mathFont` — the default math font family.
    #[must_use]
    pub fn math_font(&self, interner: &Interner) -> Option<mjx_ooxml_types::shared::XmlString> {
        read_val_child::<MathFontNameCodec>(&self.children, interner, "mathFont")
    }

    /// `m:brkBin` — where a binary operator breaks across a manual line break.
    #[must_use]
    pub fn break_binary_operator(&self, interner: &Interner) -> Option<BreakBinaryOperator> {
        read_val_child::<Enumeration<BreakBinaryOperator>>(&self.children, interner, "brkBin")
    }

    /// `m:brkBinSub` — where a binary subtraction operator breaks.
    #[must_use]
    pub fn break_binary_subtraction(&self, interner: &Interner) -> Option<BreakBinarySubtraction> {
        read_val_child::<Enumeration<BreakBinarySubtraction>>(&self.children, interner, "brkBinSub")
    }

    /// `m:smallFrac` — whether fractions default to the small (inline) form.
    #[must_use]
    pub fn small_fraction(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "smallFrac")
    }

    /// `m:dispDef` — whether equations default to display (block) style.
    #[must_use]
    pub fn display_default(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "dispDef")
    }

    /// `m:lMargin` — the left margin of a display equation.
    #[must_use]
    pub fn left_margin(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "lMargin")
    }

    /// `m:rMargin` — the right margin of a display equation.
    #[must_use]
    pub fn right_margin(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "rMargin")
    }

    /// `m:defJc` — the default justification for display equations.
    #[must_use]
    pub fn default_justification(&self, interner: &Interner) -> Option<Justification> {
        read_val_child::<Enumeration<Justification>>(&self.children, interner, "defJc")
    }

    /// `m:preSp` — spacing before a display equation.
    #[must_use]
    pub fn space_before(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "preSp")
    }

    /// `m:postSp` — spacing after a display equation.
    #[must_use]
    pub fn space_after(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "postSp")
    }

    /// `m:interSp` — spacing between equations in a math paragraph.
    #[must_use]
    pub fn inter_equation_spacing(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "interSp")
    }

    /// `m:intraSp` — spacing between rows within one equation.
    #[must_use]
    pub fn intra_equation_spacing(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "intraSp")
    }

    /// `m:wrapIndent` — the indent a wrapped display equation continues at, the first half of the
    /// `EG_MathPr`-internal `wrapIndent`/`wrapRight` choice.
    #[must_use]
    pub fn wrap_indent(
        &self,
        interner: &Interner,
    ) -> Option<mjx_ooxml_types::shared::TwipsMeasure> {
        read_val_child::<TwipsCodec>(&self.children, interner, "wrapIndent")
    }

    /// `m:wrapRight` — whether a wrapped display equation continues on the right, the other half of
    /// the choice [`MathProperties::wrap_indent`] documents.
    #[must_use]
    pub fn wrap_right(&self, interner: &Interner) -> Option<bool> {
        read_onoff_child(&self.children, interner, "wrapRight")
    }

    /// `m:intLim` — where an integral's own limits are placed.
    #[must_use]
    pub fn integral_limit_location(&self, interner: &Interner) -> Option<LimitLocation> {
        read_val_child::<Enumeration<LimitLocation>>(&self.children, interner, "intLim")
    }

    /// `m:naryLim` — where a (non-integral) n-ary operator's own limits are placed.
    #[must_use]
    pub fn nary_limit_location(&self, interner: &Interner) -> Option<LimitLocation> {
        read_val_child::<Enumeration<LimitLocation>>(&self.children, interner, "naryLim")
    }
}
