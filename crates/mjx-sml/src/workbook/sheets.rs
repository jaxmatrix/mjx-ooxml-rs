//! `x:sheets` / `x:sheet` (`CT_Sheets` at `sml.xsd:4211`, `CT_Sheet` at `4216`) — the list that
//! names every sheet in the workbook.
//!
//! # The relationship names the part. Not `@sheetId`, and not the position in the list.
//!
//! `CT_Sheet` carries four attributes and three of them look like they might identify the part:
//!
//! | attribute | what it actually is |
//! |---|---|
//! | `@name` | the text on the tab. User-visible, editable, and not an identifier of anything. |
//! | `@sheetId` | an identifier **internal to the workbook part**: what `definedName/@localSheetId`-adjacent markup, `pivotCache` and revision records refer to. It says nothing about which part holds the sheet's cells. |
//! | `r:id` | an OPC relationship id. Resolving it against `xl/_rels/workbook.xml.rels` is the **only** thing that names the part. |
//! | position | the tab order a consumer shows. Nothing more. |
//!
//! ECMA-376 Part 1 §12.3.24 is explicit: *"the `id` attribute on the `sheet` element shall reference
//! the desired worksheet part"*. A workbook whose list order, `@sheetId` order and relationship
//! order all disagree is perfectly legal, and Excel writes them — deleting the second of three
//! sheets and adding a new one leaves `sheetId="1"`, `sheetId="3"`, `sheetId="4"` pointing at
//! `rId1`, `rId3`, `rId4` in whatever order the user then dragged the tabs into.
//!
//! `tests/fixtures/workbook_sheet_order.xlsx` is authored so that all three orders disagree, because
//! a fixture where they agree cannot tell a correct resolver from one that indexed the relationship
//! list, parsed the digits out of `rId3`, or trusted `@sheetId`.
//!
//! # This module holds the *markup*. Resolution is `mjx-xlsx`'s.
//!
//! Nothing here has ever heard of a package. [`SheetEntry::relationship_id`] hands back the raw
//! identifier string; `crates/mjx-xlsx/src/workbook/sheets.rs` is what turns it into a
//! [`mjx_opc::PartName`](https://docs.rs/mjx-opc). That seam is the reason the two crates exist: an
//! embedded workbook inside a `.pptx` needs this model and has no `mjx-xlsx` above it.

use mjx_ooxml_core::{Enumeration, Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::spreadsheetml::SheetState;

use super::leaf::{attribute_bag, relationship_reference};

attribute_bag! {
    /// `x:sheet` (`CT_Sheet`) — one tab: its name, its workbook-internal id, whether it is shown,
    /// and the relationship that names its part.
    ///
    /// All four attributes are `use="required"` in the schema, and none of them is required here:
    /// a file that omits one is read and reported as it stands, because refusing to open a workbook
    /// over a missing `@sheetId` would trade a readable file for an unreadable one. `@name` and
    /// `@sheetId` are declared optional for exactly that reason; `state` carries the schema's own
    /// default; and [`relationship_id`](Self::relationship_id) answers `None` for an entry that
    /// names none.
    #[xml(attribute(local = "name", codec = Text, accessor = name))]
    #[xml(attribute(local = "sheetId", codec = Number<u32>, accessor = sheet_id))]
    #[xml(attribute(local = "state", codec = Enumeration<SheetState>, accessor = visibility, default = SheetState::Visible))]
    SheetEntry, "sheet"
}

relationship_reference!(SheetEntry);

/// `x:sheets` (`CT_Sheets`) — the sheet list, in tab order.
///
/// The schema declares `sheet` `minOccurs="1"`, so a workbook with no tabs is invalid; it is still
/// read, because this crate reports what a file says rather than refusing it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct SheetList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "sheet", variant = Sheet, ty = SheetEntry))]
    content: Vec<SheetListContent>,
}

/// One child of [`SheetList`]: a sheet entry, or markup this type does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetListContent {
    /// `x:sheet`.
    Sheet(SheetEntry),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl SheetList {
    /// Builds an empty `x:sheets`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "sheets"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:sheet`, in the order the file lists them — which is tab order.
    ///
    /// Lazy and allocation-free: a filter over the content list, so asking for the first tab of a
    /// hundred does not walk the other ninety-nine. [`len`](Self::len) is the count for a caller
    /// that needs one.
    pub fn entries(&self) -> impl Iterator<Item = &SheetEntry> + '_ {
        self.content.iter().filter_map(|item| match item {
            SheetListContent::Sheet(entry) => Some(entry),
            SheetListContent::Raw(_) => None,
        })
    }

    /// The `index`-th `x:sheet` in tab order, mutably.
    ///
    /// Indexes the **sheet entries**, skipping anything unmodelled between them, so a comment
    /// between two `sheet` elements does not shift the numbering a caller sees.
    pub fn entry_mut(&mut self, index: usize) -> Option<&mut SheetEntry> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                SheetListContent::Sheet(entry) => Some(entry),
                SheetListContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// How many tabs the list names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.content
            .iter()
            .filter(|item| matches!(item, SheetListContent::Sheet(_)))
            .count()
    }

    /// Whether the list names no tab at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `entry` after the last sheet already in the list.
    ///
    /// `CT_Sheets`'s content model is a single repeatable slot, so there is no rank to consult: a
    /// new entry can only go among the others, and the end is where a new tab goes.
    pub fn push(&mut self, entry: SheetEntry) {
        self.content.push(SheetListContent::Sheet(entry));
    }
}
