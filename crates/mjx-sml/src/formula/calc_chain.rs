//! `calcChain.xml` (`CT_CalcChain` at `sml.xsd:257`, `CT_CalcCell` at `263`) — the calculation-order
//! cache, modelled and left alone.
//!
//! # What it is
//!
//! §18.6: *"The calculation chain specifies the order in which the cells in a workbook were last
//! calculated."* It holds one `<c>` per formula cell, most recently calculated first, and it records
//! **order** rather than dependency — the standard is explicit that *"it does not track or express
//! dependencies amongst the formulas"*.
//!
//! It is also, in the standard's own words, optional: *"The calculation chain described in this
//! section is not required by the spreadsheet application, but can be used if the spreadsheet
//! application finds it useful… the spreadsheet application is free to perform calculations in a
//! different order at run time."*
//!
//! # The policy: leave it, exactly as found
//!
//! The chain is derived data Excel owns, and an edit through this library can make it stale — adding
//! a formula puts a cell in the sheet that the chain does not mention. There are three things a
//! library can do about that, and only two of them are honest:
//!
//! 1. **Maintain it.** Rejected, and not close. Maintaining the chain means computing a dependency
//!    order, which means parsing formula expressions and resolving references across sheets — a
//!    calculation engine in all but name, which `PLAN.md` settles as out of scope. A *partially*
//!    maintained chain is worse than a stale one, because it looks current.
//! 2. **Drop the part.** Honest, and Excel rebuilds. Rejected anyway: deleting a part is an edit to
//!    the package that the caller did not ask for, it happens on *every* save rather than only when a
//!    formula changed, and it loses the one thing the part carries that nothing else does — a record
//!    of the order a producer last calculated in, which a caller may be reading the file precisely to
//!    inspect. A library whose `save` silently removes a part is not one you can round-trip with.
//! 3. **Leave it.** Chosen. The part keeps its container bytes, `save` re-emits them verbatim, and a
//!    consumer that finds the chain inconsistent with the sheets rebuilds it — which is the same
//!    thing it does when it finds the chain absent, and exactly what the standard says it is free to
//!    do.
//!
//! The cost is written down rather than hidden: a workbook whose formulas were edited here carries a
//! chain that names cells in an order that is no longer current. `docs/fidelity_and_gaps.md` and
//! `crates/mjx-xlsx/docs/guide/formulas_and_cached_values.md` both say so, in prose, naming it
//! deliberate.
//!
//! # An attribute the standard declares and never describes
//!
//! `CT_CalcCell` declares **seven** attributes — `r`, `ref`, `i`, `s`, `l`, `t`, `a`. ECMA-376
//! Part 1 §18.6.1's attribute table documents **six** of them; `ref` appears in the schema and
//! nowhere in the prose, so there is no statement anywhere of what it means.
//!
//! It is read here as a [`CellRange`] rather than as a [`CellReference`], and that is a decision
//! about untrusted input rather than about the schema. `ST_CellRef` and `ST_Ref` are **both**
//! declared `<xsd:restriction base="xsd:string"/>` with no facets at all (`sml.xsd:204` and `207`),
//! so the schema distinguishes them in name only and constrains neither: a validator accepts
//! `F5:G7` in a `ST_CellRef` exactly as it accepts `F5`. A reader that refused the first would be
//! refusing something the file is allowed to say about an attribute nobody has documented. The wider
//! type accepts both and reports what the file wrote.

use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml, Interner, Number, RawAttribute, RawDocument, RawElement,
    RawName, RawNode,
};
use mjx_ooxml_types::namespaces::SML;
use mjx_ooxml_types::support::OnOff;

use crate::address::{CellRange, CellReference};
use crate::error::SmlError;
use crate::leaf::attribute_bag;

attribute_bag! {
    /// `x:c` (`CT_CalcCell`, `sml.xsd:263`) — one cell in the calculation order.
    ///
    /// The names below come from ECMA-376 Part 1 §18.6.1's own attribute table, which is the only
    /// place the single-letter tokens are explained: `l` is *New Dependency Level* and `s` is *Child
    /// Chain*, and nothing about either letter says so.
    ///
    /// | Wire | Prose name (§18.6.1) | Accessor |
    /// |---|---|---|
    /// | `r` | Cell Reference | [`reference`](Self::reference) |
    /// | `i` | Sheet Id | [`sheet_id`](Self::sheet_id) |
    /// | `s` | Child Chain | [`is_on_child_chain`](Self::is_on_child_chain) |
    /// | `l` | New Dependency Level | [`starts_new_dependency_level`](Self::starts_new_dependency_level) |
    /// | `t` | New Thread | [`starts_new_thread`](Self::starts_new_thread) |
    /// | `a` | Array | [`is_array_formula`](Self::is_array_formula) |
    /// | `ref` | *— not in the prose —* | [`range`](Self::range) |
    ///
    /// Two of them are **carried forward from the previous entry when absent**, which is unusual and
    /// is the standard's own rule: `i` (*"If this is omitted, it is assumed to be the same as the `i`
    /// value of the previous cell"*) and `s` (likewise). The accessors here answer for *this element*
    /// and do not walk backwards — resolving the carry-forward is
    /// [`CalculationChain::resolved`](CalculationChain::resolved)'s job, so that the two questions
    /// stay separable.
    #[xml(attribute(local = "r", codec = Enumeration<CellReference>, accessor = reference))]
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range))]
    #[xml(attribute(local = "i", codec = Number<i32>, accessor = sheet_id))]
    #[xml(attribute(local = "s", codec = OnOff, accessor = is_on_child_chain, default = false))]
    #[xml(attribute(local = "l", codec = OnOff, accessor = starts_new_dependency_level, default = false))]
    #[xml(attribute(local = "t", codec = OnOff, accessor = starts_new_thread, default = false))]
    #[xml(attribute(local = "a", codec = OnOff, accessor = is_array_formula, default = false))]
    CalculationChainCell, "c"
}

/// `x:calcChain` (`CT_CalcChain`, `sml.xsd:257`) — the whole `xl/calcChain.xml` part.
///
/// A read-only model in practice: it has no mutator, because **nothing in this workspace maintains a
/// calculation chain**. See the [module documentation](self) for the policy and for the two
/// alternatives that were rejected.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct CalculationChain {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "c", variant = Cell, ty = CalculationChainCell))]
    content: Vec<CalculationChainContent>,
}

/// One child of [`CalculationChain`]: an entry, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalculationChainContent {
    /// `x:c` — one cell in the calculation order.
    Cell(CalculationChainCell),
    /// The one slot this frame does not model — `extLst` — plus any foreign element, any
    /// `mc:AlternateContent`, and the text, comments and processing instructions between siblings.
    Raw(RawNode),
}

impl CalculationChain {
    /// Reads a whole `xl/calcChain.xml` part.
    ///
    /// `Ok(None)` when the document's root is not an `x:calcChain` — the caller handed over a
    /// different part, which is a question rather than an error.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match the shape its complex type declares.
    pub fn read_part(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        Self::read_root(&document.root, &document.interner)
    }

    /// [`read_part`](Self::read_part) for a caller holding the root element and the interner.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        let namespace = root.name.namespace.map(|symbol| interner.resolve(symbol));
        let in_spreadsheetml =
            namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict);
        if !in_spreadsheetml || interner.resolve(root.name.local) != "calcChain" {
            return Ok(None);
        }
        Ok(Some(Self::from_xml(root, interner)?))
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CalculationChainContent] {
        &self.content
    }

    /// Every entry, in the order the part wrote them — which **is** the calculation order.
    pub fn cells(&self) -> impl Iterator<Item = &CalculationChainCell> + '_ {
        self.content.iter().filter_map(|item| match item {
            CalculationChainContent::Cell(cell) => Some(cell),
            CalculationChainContent::Raw(_) => None,
        })
    }

    /// How many entries the chain holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells().count()
    }

    /// Whether the chain holds no entry at all.
    ///
    /// `CT_CalcChain` declares `c` with `minOccurs="1"`, so an empty chain is schema-invalid — which
    /// is a reason to *report* one, not a reason to refuse to read it.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every entry with §18.6.1's two carry-forward rules applied: `@i` and `@s`, when absent, take
    /// the value of the previous entry.
    ///
    /// The entries themselves are untouched — this is a *reading* of the chain, not a rewriting of
    /// it, and nothing here is on the write path.
    ///
    /// # Errors
    /// [`AttributeError`] if an `@i` or `@s` will not decode.
    pub fn resolved<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> Result<Vec<ResolvedCalculationChainCell<'a>>, AttributeError> {
        let mut sheet_id = None;
        let mut is_on_child_chain = false;
        let mut resolved = Vec::new();
        for cell in self.cells() {
            if let Some(written) = cell.sheet_id(interner)? {
                sheet_id = Some(written);
            }
            // `@s` has `default="false"`, and §18.6.1's carry-forward rule is about the attribute
            // being *absent*, so the raw presence is what decides — not the decoded value, which is
            // `false` either way.
            if cell.has_written_child_chain(interner) {
                is_on_child_chain = cell.is_on_child_chain(interner)?;
            }
            resolved.push(ResolvedCalculationChainCell {
                cell,
                sheet_id,
                is_on_child_chain,
            });
        }
        Ok(resolved)
    }
}

/// One entry of a [`CalculationChain`] with §18.6.1's two carry-forward rules applied.
///
/// `@i` and `@s` are the only attributes in this schema that mean *"the same as the previous
/// element's"* when absent, and resolving them is a separate question from what one element says.
/// [`CalculationChain::resolved`] answers the first; the accessors on [`CalculationChainCell`]
/// answer the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedCalculationChainCell<'a> {
    /// The entry itself, exactly as the file wrote it.
    pub cell: &'a CalculationChainCell,
    /// `@i` — this entry's own, or the nearest earlier entry's, or `None` if no entry so far wrote
    /// one.
    pub sheet_id: Option<i32>,
    /// `@s` — this entry's own, or the nearest earlier entry's, or `false`.
    pub is_on_child_chain: bool,
}

impl CalculationChainCell {
    /// Whether `@s` was written at all, which is what §18.6.1's carry-forward rule turns on — the
    /// attribute has `default="false"`, so an absent one and a written `s="0"` decode alike and mean
    /// different things in a chain.
    #[must_use]
    pub fn has_written_child_chain(&self, interner: &Interner) -> bool {
        mjx_xml::attribute::find(&self.attributes, interner, None, "s").is_some()
    }
}
