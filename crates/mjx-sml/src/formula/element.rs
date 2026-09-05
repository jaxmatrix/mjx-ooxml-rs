//! `ST_Formula` **as an element** — the shape a formula takes everywhere in `sml.xsd` except inside
//! a cell.
//!
//! # One type, three elements, and why it is not three types
//!
//! `sml.xsd` declares `ST_Formula` as `<xsd:restriction base="s:ST_Xstring"/>` and then hangs three
//! elements off it whose only difference is their name:
//!
//! | Element | Owner | `sml.xsd` |
//! |---|---|---|
//! | `x:cfRule/formula` | [`ConditionalFormattingRule`](crate::ConditionalFormattingRule) | 2717 |
//! | `x:dataValidation/formula1` | [`DataValidation`](crate::DataValidation) | 2581 |
//! | `x:dataValidation/formula2` | [`DataValidation`](crate::DataValidation) | 2581 |
//!
//! The content model, the escaping rules and the no-evaluation contract are identical for all three,
//! so [`FormulaElement`] carries its own local name and there is exactly one implementation. It was
//! `ConditionalFormattingFormula` in `features/conditional_rules.rs` until MJXOFF-123 needed the
//! same element under two more names; the answer to a second consumer is one implementation both can
//! reach, not a copy with a different doc comment.
//!
//! A cell's own `<f>` is **not** here: `CT_CellFormula` carries twelve attributes and lives inside
//! MJXOFF-95's packed store as a byte range, which is why [`CellFormula`](crate::CellFormula) is a
//! *view* over those bytes rather than a struct. See [`crate::formula`]'s own module documentation.
//!
//! # Why this is not `#[derive(FromXml, ToXml)]`
//!
//! The same reason [`DefinedName`](crate::DefinedName) is not: `mjx-derive`'s `#[xml(text)]` grammar
//! re-escapes character data **minimally** on write, so a producer that spelled a comparison
//! `&quot;OK&quot;` would get `"OK"` back — the same string and different bytes. That is invisible
//! while the part is copied verbatim and becomes visible the moment anything *else* in the part
//! changes, because a rebuilt text node denies its element, and every ancestor of it, the verbatim
//! source range it would otherwise keep.
//!
//! So the pair is written by hand: [`from_xml`](FromXml::from_xml) keeps the original children as
//! they stood, and the rebuild replays them until [`set_text`](FormulaElement::set_text) states
//! otherwise.
//!
//! # It is never evaluated, and never rewritten
//!
//! MJXOFF-115's contract, restated for every place `sml.xsd` puts a formula that is not a cell's.
//! Nothing here parses the expression, translates it between `A1` and `R1C1`, offsets its references
//! when a rule is copied to another range, or — the trap MJXOFF-123 names — **resolves a list
//! validation's `Sheet2!$A$1:$A$9` into the values that range holds**. A `formula1` is text going in
//! and the same text coming out.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

/// An element whose content is an `ST_Formula`: `x:cfRule/formula`, `x:dataValidation/formula1` and
/// `x:dataValidation/formula2`.
///
/// **`ST_`/`CT_` symbol:** `ST_Formula`, used as an element type. Wire elements: `formula`,
/// `formula1`, `formula2`.
///
/// The element keeps the qualified name it was read with, so a file that bound SpreadsheetML to a
/// prefix gets that prefix back, and its character data is replayed byte for byte — entity spellings
/// and CDATA sections included — until [`set_text`](Self::set_text) replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    /// The character data, decoded — what [`text`](Self::text) answers with.
    text: String,
    /// The element's children exactly as the file wrote them, or `None` once the text has been
    /// replaced and there is nothing left to preserve.
    verbatim: Option<Vec<RawNode>>,
}

impl FormulaElement {
    /// Builds an element named `local` holding `text`, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `local` is the wire name of the slot being filled — `"formula"`, `"formula1"` or
    /// `"formula2"`. It is a parameter rather than a constant because one element type serves three
    /// slots; see this module's own documentation.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        prefix: Option<&str>,
        local: &str,
        text: impl Into<String>,
    ) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, local),
            attributes: Vec::new(),
            empty: false,
            text: text.into(),
            verbatim: None,
        }
    }

    /// The formula's text, exactly as the file wrote it (entity references decoded).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text.
    ///
    /// Nothing validates it: a condition is a formula, formulas are text here, and a caller that
    /// writes something Excel will refuse has written what a producer is free to write.
    ///
    /// This is the point at which the preserved character data is given up.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.verbatim = None;
        self.empty = false;
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = replayed_children(self.verbatim.as_ref(), &self.text);
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl FromXml for FormulaElement {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text: decoded_text(element)?,
            verbatim: Some(element.children.clone()),
        })
    }
}

/// The character data of an `ST_Formula`-typed element, decoded — text nodes unescaped and CDATA
/// sections taken literally, concatenated in document order.
///
/// `pub(crate)` because `CT_TableFormula` ([`TableFormula`](crate::TableFormula)) is a
/// `simpleContent` **extension** of `ST_Formula` rather than `ST_Formula` itself: it carries an
/// `@array` the other three slots do not declare, so it is its own complex type — but its content is
/// the same content, and the decoding of it is written once, here.
pub(crate) fn decoded_text(element: &RawElement) -> Result<String, FromXmlError> {
    let mut text = String::new();
    for child in &element.children {
        match child {
            RawNode::Text(bytes) => {
                let raw = core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                let decoded = mjx_xml::text::unescape_text(raw)
                    .map_err(|error| FromXmlError::InvalidEntity(error.to_string()))?;
                text.push_str(&decoded);
            }
            RawNode::CData(bytes) => {
                text.push_str(core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?);
            }
            _ => {}
        }
    }
    Ok(text)
}

/// The children an `ST_Formula`-typed element writes: the ones the file held while they are still
/// claimed, and one freshly escaped text node once [`set_text`](FormulaElement::set_text) has
/// replaced them.
///
/// The counterpart of [`decoded_text`], `pub(crate)` for the same reason.
#[must_use]
pub(crate) fn replayed_children(verbatim: Option<&Vec<RawNode>>, text: &str) -> Vec<RawNode> {
    match verbatim {
        // Untouched: replay exactly what the file held — entity spellings and CDATA included.
        Some(children) => children.clone(),
        None if text.is_empty() => Vec::new(),
        None => vec![RawNode::Text(
            mjx_xml::text::escape_text(text).as_bytes().into(),
        )],
    }
}

impl ToXml for FormulaElement {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
