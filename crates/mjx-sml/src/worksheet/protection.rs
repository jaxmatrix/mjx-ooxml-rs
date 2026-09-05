//! Sheet protection: `CT_SheetProtection`, `CT_ProtectedRanges` and `CT_ProtectedRange`.
//!
//! | Type | `sml.xsd` (Transitional) | Element |
//! |---|---|---|
//! | `CT_SheetProtection` | 2887 | `x:sheetProtection` (rank 7) |
//! | `CT_ProtectedRanges` | 2911 | `x:protectedRanges` (rank 8) |
//! | `CT_ProtectedRange` | 2917 | `x:protectedRanges/protectedRange` |
//!
//! # Sheet protection is not security, and this library never says otherwise
//!
//! **Nothing here computes a hash, verifies a hash, or reports whether a password is correct.** The
//! five attributes that carry one — `password`, `algorithmName`, `hashValue`, `saltValue`,
//! `spinCount` — are read as the text the file wrote and written back as the same text, byte for
//! byte, and there is deliberately no call anywhere in this workspace that takes a password.
//!
//! Two separate reasons, and the second is the one that matters:
//!
//! * **A hash this library recomputed would be a claim it cannot make.** Excel derives the hash from
//!   a user-supplied password through an algorithm the file names; a fidelity library editing an
//!   unrelated part of a workbook has no password and no business inventing one.
//! * **Protection is a user-interface convenience, not access control.** ECMA-376 Part 1 says of the
//!   analogous document-protection setting that *"protection is not intended as a security
//!   feature"*, and every flag on [`SheetProtection`] is advisory: it says what a consumer *should*
//!   refuse, and the sheet's bytes are readable by anyone holding the file whatever it says.
//!   Presenting these as security would be the most consequential thing this module could get
//!   wrong, so it is written down here rather than left to be inferred.
//!
//! # Every flag is a **lock**, and its default is usually "locked"
//!
//! This is the trap in the type, and it is not inferable from the wire tokens. ECMA-376 Part 1
//! §18.3.1.85 states each flag in the same form: *"If 1 or true then formatting cells should not be
//! allowed when the sheet is protected."* `formatCells="1"` **forbids** formatting cells. The
//! Transitional type's twenty-one attributes are five of the password family, `@sheet` itself, and
//! **fifteen locks** — of which **eleven default to `true`**, so a bare `<sheetProtection sheet="1"/>`
//! locks nearly everything and states no attribute at all.
//!
//! Every accessor here is therefore named `locks_…`, sourced from that prose rather than from the
//! token, and reading one as an "allow" inverts the file's meaning.
//!
//! `@sheet` is the exception and the gate: *"The value of this attribute dictates whether the other
//! attributes of `sheetProtection` should be applied."* A `sheetProtection` element with
//! `sheet="0"` states fifteen locks that apply to nothing.
//!
//! # `password` exists in Transitional only
//!
//! ECMA-376 Part 1 Annex M records that *"the `password` attribute was removed from
//! `sheetProtection` …, `protectedRange` …"* for Strict, and that `algorithmName`, `hashValue`,
//! `saltValue` and `spinCount` were added. The Strict `CT_SheetProtection` really does declare
//! **twenty** attributes against Transitional's twenty-one, and Strict's `CT_ProtectedRange` drops
//! both `password` and the `securityDescriptor` *attribute*.
//!
//! That is a second reason never to author one: a `password` written into a Strict document is
//! markup the Strict schema rejects. Reading one, and writing back the one a Transitional file
//! wrote, is what fidelity means and is what happens here.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRangeList;
use crate::leaf::attribute_bag;

use super::frame::WorksheetPart;
use super::rebuild_element;

attribute_bag! {
    /// `x:sheetProtection` (`CT_SheetProtection`, `sml.xsd:2887`) — which operations a consumer
    /// should refuse while this sheet is protected, and the password hash that lifts the refusal.
    ///
    /// **Twenty-one attributes** in the Transitional schema this project validates against: the five
    /// of the password family, `@sheet` itself, and fifteen locks — eleven of which default to
    /// locked. See this module's own documentation for why every one of them is a *lock* rather than
    /// a permission, and why nothing here treats any of it as security.
    #[xml(attribute(local = "password", codec = Text, accessor = legacy_password_hash))]
    #[xml(attribute(local = "algorithmName", codec = Text, accessor = hash_algorithm_name))]
    #[xml(attribute(local = "hashValue", codec = Text, accessor = hash_value))]
    #[xml(attribute(local = "saltValue", codec = Text, accessor = salt_value))]
    #[xml(attribute(local = "spinCount", codec = Number<u32>, accessor = hash_iteration_count))]
    #[xml(attribute(local = "sheet", codec = OnOff, accessor = is_protected, default = false))]
    #[xml(attribute(local = "objects", codec = OnOff, accessor = locks_editing_objects, default = false))]
    #[xml(attribute(local = "scenarios", codec = OnOff, accessor = locks_editing_scenarios, default = false))]
    #[xml(attribute(local = "formatCells", codec = OnOff, accessor = locks_formatting_cells, default = true))]
    #[xml(attribute(local = "formatColumns", codec = OnOff, accessor = locks_formatting_columns, default = true))]
    #[xml(attribute(local = "formatRows", codec = OnOff, accessor = locks_formatting_rows, default = true))]
    #[xml(attribute(local = "insertColumns", codec = OnOff, accessor = locks_inserting_columns, default = true))]
    #[xml(attribute(local = "insertRows", codec = OnOff, accessor = locks_inserting_rows, default = true))]
    #[xml(attribute(local = "insertHyperlinks", codec = OnOff, accessor = locks_inserting_hyperlinks, default = true))]
    #[xml(attribute(local = "deleteColumns", codec = OnOff, accessor = locks_deleting_columns, default = true))]
    #[xml(attribute(local = "deleteRows", codec = OnOff, accessor = locks_deleting_rows, default = true))]
    #[xml(attribute(local = "selectLockedCells", codec = OnOff, accessor = locks_selecting_locked_cells, default = false))]
    #[xml(attribute(local = "sort", codec = OnOff, accessor = locks_sorting, default = true))]
    #[xml(attribute(local = "autoFilter", codec = OnOff, accessor = locks_auto_filters, default = true))]
    #[xml(attribute(local = "pivotTables", codec = OnOff, accessor = locks_pivot_tables, default = true))]
    #[xml(attribute(local = "selectUnlockedCells", codec = OnOff, accessor = locks_selecting_unlocked_cells, default = false))]
    SheetProtection, "sheetProtection"
}

attribute_bag! {
    /// `x:protectedRange` (`CT_ProtectedRange`, `sml.xsd:2917`) — one range a named group may edit
    /// even while the sheet is protected.
    ///
    /// `@sqref` and `@name` are the two `use="required"` attributes; `@sqref` is an `ST_Sqref`,
    /// which is MJXOFF-93's [`CellRangeList`]. The password family is the same five as
    /// [`SheetProtection`]'s and is treated the same way — preserved, never computed, never
    /// verified.
    ///
    /// `securityDescriptor` is declared **twice** by this complex type: once as an attribute, once
    /// as an `unbounded` child element. Both carry the same kind of Windows SDDL string, and a file
    /// may use either. The attribute is modelled here; the child element is unmodelled markup and
    /// survives in the type's `extra` bucket, in position. Neither is interpreted.
    #[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = ranges, required))]
    #[xml(attribute(local = "name", codec = Text, accessor = name, required))]
    #[xml(attribute(local = "password", codec = Text, accessor = legacy_password_hash))]
    #[xml(attribute(local = "securityDescriptor", codec = Text, accessor = security_descriptor))]
    #[xml(attribute(local = "algorithmName", codec = Text, accessor = hash_algorithm_name))]
    #[xml(attribute(local = "hashValue", codec = Text, accessor = hash_value))]
    #[xml(attribute(local = "saltValue", codec = Text, accessor = salt_value))]
    #[xml(attribute(local = "spinCount", codec = Number<u32>, accessor = hash_iteration_count))]
    ProtectedRange, "protectedRange"
}

/// `x:protectedRanges` (`CT_ProtectedRanges`, `sml.xsd:2911`) — every protected range of the sheet,
/// in document order.
///
/// No attributes at all: unlike `mergeCells` and `rowBreaks`, this element declares no `count`.
/// The schema declares `protectedRange` `minOccurs="1"`, so an element with none is invalid markup —
/// preserved when read, and never authored by [`ProtectedRanges::remove`], which is why
/// [`WorksheetPart::remove_protected_range`](crate::WorksheetPart::remove_protected_range) takes the
/// whole element away with the last range.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct ProtectedRanges {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "protectedRange", variant = Range, ty = ProtectedRange))]
    content: Vec<ProtectedRangesContent>,
}

/// One child of [`ProtectedRanges`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedRangesContent {
    /// `x:protectedRange`.
    Range(ProtectedRange),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ProtectedRanges {
    /// Builds an empty `x:protectedRanges`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "protectedRanges"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ProtectedRangesContent] {
        &self.content
    }

    /// Every `x:protectedRange`, in document order.
    pub fn ranges(&self) -> impl Iterator<Item = &ProtectedRange> + '_ {
        self.content.iter().filter_map(|item| match item {
            ProtectedRangesContent::Range(range) => Some(range),
            ProtectedRangesContent::Raw(_) => None,
        })
    }

    /// How many `x:protectedRange` children this element holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ranges().count()
    }

    /// Whether the element holds no range at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:protectedRange`, mutably.
    pub fn range_mut(&mut self, index: usize) -> Option<&mut ProtectedRange> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                ProtectedRangesContent::Range(range) => Some(range),
                ProtectedRangesContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a range after the ones already present.
    pub fn push(&mut self, range: ProtectedRange) {
        self.content.push(ProtectedRangesContent::Range(range));
        self.empty = false;
    }

    /// Removes the `index`-th `x:protectedRange`, or `None` when there is no such range.
    pub fn remove(&mut self, index: usize) -> Option<ProtectedRange> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, ProtectedRangesContent::Range(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            ProtectedRangesContent::Range(range) => Some(range),
            ProtectedRangesContent::Raw(_) => unreachable!("the position was filtered on `Range`"),
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                ProtectedRangesContent::Range(range) => RawNode::Element(range.as_raw_element()),
                ProtectedRangesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for ProtectedRanges {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -------------------------------------------------------------------------------------------
// The protected-range surface on the worksheet
// -------------------------------------------------------------------------------------------

impl WorksheetPart {
    /// Every `x:protectedRange` of the sheet, in document order.
    ///
    /// An empty slice for a worksheet with no `x:protectedRanges` element.
    pub fn protected_range_list(&self) -> impl Iterator<Item = &ProtectedRange> + '_ {
        self.protected_ranges()
            .into_iter()
            .flat_map(ProtectedRanges::ranges)
    }

    /// Appends `range` to `x:protectedRanges`, creating the element at its rank in
    /// `CT_Worksheet`'s sequence when the worksheet has none.
    ///
    /// The range is taken exactly as built. Nothing here computes, verifies or inspects a hash; see
    /// this module's own documentation.
    pub fn add_protected_range(&mut self, range: ProtectedRange) {
        if self.protected_ranges().is_none() {
            let prefix = self.own_prefix();
            let block = ProtectedRanges::new(self.interner_mut(), prefix.as_deref());
            self.set_protected_ranges(Some(block));
        }
        self.protected_ranges_mut()
            .expect("the protectedRanges element was just ensured")
            .push(range);
    }

    /// Removes the `index`-th `x:protectedRange`, reporting whether there was one.
    ///
    /// When the last range goes, the whole `x:protectedRanges` element goes with it: the schema
    /// declares `protectedRange` `minOccurs="1"`, so an empty one is markup no validator accepts.
    pub fn remove_protected_range(&mut self, index: usize) -> bool {
        let Some(block) = self.protected_ranges_mut() else {
            return false;
        };
        if block.remove(index).is_none() {
            return false;
        }
        if block.is_empty() {
            self.set_protected_ranges(None);
        }
        true
    }
}
