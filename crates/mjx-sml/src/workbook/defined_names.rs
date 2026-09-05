//! `x:definedNames` / `x:definedName` (`CT_DefinedNames` at `sml.xsd:4318`, `CT_DefinedName` at
//! `4323`) — a name a formula may use in place of a range, and the eight names SpreadsheetML
//! reserves for itself.
//!
//! # A defined name's value is a formula, and a formula is text
//!
//! `CT_DefinedName` is an `xsd:simpleContent` extension of `ST_Formula`, which is `xsd:string`. The
//! definition is therefore **text**, and this crate carries it as text:
//! `Summary!$B$1`, `SUM(Sheet1!A:A)`, `#REF!` and `'Hidden Data'!$A$1:$C$9` are all the same kind of
//! value here. Nothing parses it, nothing rewrites sheet names inside it, and nothing evaluates it —
//! there is no expression parser in this workspace and MJXOFF-115 (D11) states the same position for
//! `f` elements in a worksheet.
//!
//! A caller that knows a particular name holds a plain reference can hand the text to
//! [`CellRange::parse`](crate::CellRange::parse) or
//! [`SheetQualifiedReference`](crate::SheetQualifiedReference), which is D03's vocabulary and not a
//! second one.
//!
//! # `@localSheetId` is reported, never checked and never renumbered
//!
//! An absent `@localSheetId` scopes the name to the whole workbook; a present one scopes it to the
//! sheet at that **index in the `x:sheets` list** (not to that `@sheetId` — see
//! [`super::sheets`] for why the two are different spaces). The schema does not constrain it against
//! the number of sheets, and a file whose `localSheetId` points past the end is a file this crate
//! reads and reports: renumbering it would silently rescope somebody's name, and dropping it would
//! silently promote a sheet-scoped name to a global one.
//!
//! # The eight built-in names
//!
//! [`BuiltInName`] is the exact list ECMA-376 Part 1 §18.2.6 (`name (Defined Name)`) gives, matched
//! on the exact token and nothing else. The prose groups them as Print, Filter & Advanced Filter,
//! and Miscellaneous; every one begins `_xlnm.`, which the standard reserves — *"End users shall not
//! use this string for custom names in the user interface"*.

use mjx_ooxml_core::{Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::support::OnOff;

/// One of the eight names ECMA-376 Part 1 §18.2.6 reserves, recognised by its exact token.
///
/// A name that is not one of these — including one that merely *starts* with `_xlnm.` — is not a
/// built-in, and [`from_wire`](Self::from_wire) says so rather than guessing at a ninth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInName {
    /// `_xlnm.Print_Area` — the workbook's print area.
    PrintArea,
    /// `_xlnm.Print_Titles` — the rows or columns repeated at the top of each printed page.
    PrintTitles,
    /// `_xlnm.Criteria` — the range holding an advanced filter's criteria values.
    Criteria,
    /// `_xlnm._FilterDatabase` — the unfiltered source range an advanced filter or an AutoFilter
    /// was applied to.
    FilterDatabase,
    /// `_xlnm.Extract` — the range holding an advanced filter's output values.
    Extract,
    /// `_xlnm.Consolidate_Area` — a consolidation area.
    ConsolidationArea,
    /// `_xlnm.Database` — a range whose data comes from a database data source.
    Database,
    /// `_xlnm.Sheet_Title` — a sheet title.
    SheetTitle,
}

impl BuiltInName {
    /// The prefix every reserved name begins with, which end users may not use.
    pub const RESERVED_PREFIX: &'static str = "_xlnm.";

    /// Recognises a reserved name from its exact `@name` token, or `None` for anything else.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        Some(match name {
            "_xlnm.Print_Area" => Self::PrintArea,
            "_xlnm.Print_Titles" => Self::PrintTitles,
            "_xlnm.Criteria" => Self::Criteria,
            "_xlnm._FilterDatabase" => Self::FilterDatabase,
            "_xlnm.Extract" => Self::Extract,
            "_xlnm.Consolidate_Area" => Self::ConsolidationArea,
            "_xlnm.Database" => Self::Database,
            "_xlnm.Sheet_Title" => Self::SheetTitle,
            _ => return None,
        })
    }

    /// The exact `@name` token for this reserved name.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::PrintArea => "_xlnm.Print_Area",
            Self::PrintTitles => "_xlnm.Print_Titles",
            Self::Criteria => "_xlnm.Criteria",
            Self::FilterDatabase => "_xlnm._FilterDatabase",
            Self::Extract => "_xlnm.Extract",
            Self::ConsolidationArea => "_xlnm.Consolidate_Area",
            Self::Database => "_xlnm.Database",
            Self::SheetTitle => "_xlnm.Sheet_Title",
        }
    }

    /// Every reserved name, in the order the specification lists them.
    pub const ALL: &'static [Self] = &[
        Self::PrintArea,
        Self::PrintTitles,
        Self::Criteria,
        Self::FilterDatabase,
        Self::Extract,
        Self::ConsolidationArea,
        Self::Database,
        Self::SheetTitle,
    ];
}

impl core::fmt::Display for BuiltInName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.to_wire())
    }
}

/// `x:definedName` (`CT_DefinedName`) — one name, its scope, and the formula text it stands for.
///
/// The element's character data is the definition. It is held **decoded** (entity references
/// resolved) and re-escaped minimally on write, which is what every text leaf in this workspace
/// does; the element's own name, its attribute vector and its self-closing flag are preserved
/// exactly.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(attribute(local = "name", codec = Text, accessor = name, required))]
#[xml(attribute(local = "comment", codec = Text, accessor = comment))]
#[xml(attribute(local = "customMenu", codec = Text, accessor = custom_menu_text))]
#[xml(attribute(local = "description", codec = Text, accessor = description))]
#[xml(attribute(local = "help", codec = Text, accessor = help_text))]
#[xml(attribute(local = "statusBar", codec = Text, accessor = status_bar_text))]
#[xml(attribute(local = "localSheetId", codec = Number<u32>, accessor = local_sheet_index))]
#[xml(attribute(local = "hidden", codec = OnOff, accessor = hidden, default = false))]
#[xml(attribute(local = "function", codec = OnOff, accessor = is_function, default = false))]
#[xml(attribute(local = "vbProcedure", codec = OnOff, accessor = is_visual_basic_procedure, default = false))]
#[xml(attribute(local = "xlm", codec = OnOff, accessor = is_macro_sheet_function, default = false))]
#[xml(attribute(local = "functionGroupId", codec = Number<u32>, accessor = function_group_id))]
#[xml(attribute(local = "shortcutKey", codec = Text, accessor = shortcut_key))]
#[xml(attribute(local = "publishToServer", codec = OnOff, accessor = publish_to_server, default = false))]
#[xml(attribute(local = "workbookParameter", codec = OnOff, accessor = is_workbook_parameter, default = false))]
pub struct DefinedName {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
    definition: String,
}

impl DefinedName {
    /// The formula text this name stands for, exactly as the file wrote it (entity references
    /// decoded).
    #[must_use]
    pub fn definition(&self) -> &str {
        &self.definition
    }

    /// Replaces the formula text.
    ///
    /// Nothing validates it: a definition is a formula, formulas are text here, and a caller that
    /// writes `#REF!` has written what Excel itself writes when a name's target is deleted.
    pub fn set_definition(&mut self, definition: impl Into<String>) {
        self.definition = definition.into();
        self.empty = false;
    }

    /// Which reserved name this is, or `None` for an author's own name.
    ///
    /// # Errors
    /// [`AttributeError`](mjx_ooxml_core::AttributeError) if `@name` is absent or will not decode —
    /// the same failure [`name`](Self::name) reports, surfaced rather than swallowed into "not
    /// built-in".
    pub fn built_in(
        &self,
        interner: &mjx_ooxml_core::Interner,
    ) -> Result<Option<BuiltInName>, mjx_ooxml_core::AttributeError> {
        Ok(BuiltInName::from_wire(&self.name(interner)?))
    }

    /// Whether the name is scoped to one sheet (`@localSheetId` present) rather than to the
    /// workbook.
    ///
    /// # Errors
    /// [`AttributeError`](mjx_ooxml_core::AttributeError) if the attribute is present but is not an
    /// `xsd:unsignedInt`.
    pub fn is_sheet_scoped(
        &self,
        interner: &mjx_ooxml_core::Interner,
    ) -> Result<bool, mjx_ooxml_core::AttributeError> {
        Ok(self.local_sheet_index(interner)?.is_some())
    }
}

/// `x:definedNames` (`CT_DefinedNames`) — every defined name, in document order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct DefinedNames {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "definedName", variant = Name, ty = DefinedName))]
    content: Vec<DefinedNamesContent>,
}

/// One child of [`DefinedNames`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinedNamesContent {
    /// `x:definedName`.
    Name(DefinedName),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl DefinedNames {
    /// Builds an empty `x:definedNames`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "definedNames"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:definedName`, in document order.
    ///
    /// Document order is preserved and never sorted: `CT_DefinedNames` declares one repeatable slot
    /// and imposes no ordering, and reordering somebody's names would change bytes for nothing.
    pub fn names(&self) -> impl Iterator<Item = &DefinedName> + '_ {
        self.content.iter().filter_map(|item| match item {
            DefinedNamesContent::Name(name) => Some(name),
            DefinedNamesContent::Raw(_) => None,
        })
    }

    /// The names, mutably, in document order.
    pub fn names_mut(&mut self) -> impl Iterator<Item = &mut DefinedName> + '_ {
        self.content.iter_mut().filter_map(|item| match item {
            DefinedNamesContent::Name(name) => Some(name),
            DefinedNamesContent::Raw(_) => None,
        })
    }

    /// How many names are declared.
    #[must_use]
    pub fn len(&self) -> usize {
        self.content
            .iter()
            .filter(|item| matches!(item, DefinedNamesContent::Name(_)))
            .count()
    }

    /// Whether no name is declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a name after the ones already present.
    pub fn push(&mut self, name: DefinedName) {
        self.content.push(DefinedNamesContent::Name(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reserved token round-trips, and the list is exactly the eight the specification gives.
    #[test]
    fn the_eight_reserved_names_are_matched_on_their_exact_token() {
        assert_eq!(BuiltInName::ALL.len(), 8);
        for built_in in BuiltInName::ALL {
            assert_eq!(BuiltInName::from_wire(built_in.to_wire()), Some(*built_in));
            assert!(built_in.to_wire().starts_with(BuiltInName::RESERVED_PREFIX));
        }
    }

    /// A name that merely looks reserved is not one — the match is on the whole token.
    #[test]
    fn a_name_that_only_starts_with_the_reserved_prefix_is_not_a_built_in() {
        assert_eq!(BuiltInName::from_wire("_xlnm.Print_Areas"), None);
        assert_eq!(BuiltInName::from_wire("_xlnm.NotAThing"), None);
        assert_eq!(BuiltInName::from_wire("Print_Area"), None);
        assert_eq!(BuiltInName::from_wire("TaxRate"), None);
        // Case is part of the token: `ST_Xstring` is case-sensitive and so is this.
        assert_eq!(BuiltInName::from_wire("_xlnm.print_area"), None);
    }
}
