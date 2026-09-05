//! `xl/workbook.xml` — `CT_Workbook`, the part that names every sheet.
//!
//! # What this is, and what it is deliberately not
//!
//! [`WorkbookPart`] is the **markup** of the workbook part: `CT_Workbook`'s nineteen-slot
//! `xsd:sequence` (`sml.xsd:4097`) and the twenty-nine complex types its cluster runs to at
//! `sml.xsd:4439`. It knows nothing about packages. It holds an `r:id` as the string the file wrote
//! and has never heard of a [`PartName`](https://docs.rs/mjx-opc), a relationship or a content type.
//!
//! Resolving a sheet's `r:id` to an actual part, and the `Workbook::sheets()` a caller navigates
//! with, are `mjx-xlsx`'s — `crates/mjx-xlsx/src/workbook/`. **That seam is the reason the two
//! crates exist:** an authored chart inside a `.pptx` embeds a whole workbook at
//! `/ppt/embeddings/*.xlsx`, and `mjx-chart` (rank 2.2) has to reach this model without the format
//! crate (rank 3.0) above it, which would be an upward edge. `xtask/tests/layering.rs` checks that
//! no `mjx-opc` type appears in this crate's public signatures by checking the dependency graph, and
//! `crates/mjx-sml/tests/workbook_markup.rs` checks it the other way round: it reads and re-emits a
//! whole `workbook.xml` without naming `mjx_opc` once.
//!
//! # The nineteen slots
//!
//! In `xsd:sequence` order, which is the order [`WORKBOOK`] gives by rank and the order this type
//! places a newly set child at. Eighteen are modelled; the nineteenth is `extLst`.
//!
//! | rank | element | modelled as |
//! |---|---|---|
//! | 0 | `fileVersion` | [`FileVersion`] |
//! | 1 | `fileSharing` | [`FileSharing`] |
//! | 2 | `workbookPr` | [`WorkbookProperties`] |
//! | 3 | `workbookProtection` | [`WorkbookProtection`] |
//! | 4 | `bookViews` | [`BookViews`] |
//! | 5 | `sheets` | [`SheetList`] |
//! | 6 | `functionGroups` | [`FunctionGroups`] |
//! | 7 | `externalReferences` | [`ExternalReferences`] |
//! | 8 | `definedNames` | [`DefinedNames`] |
//! | 9 | `calcPr` | [`CalculationProperties`] |
//! | 10 | `oleSize` | [`EmbeddedObjectSize`] |
//! | 11 | `customWorkbookViews` | [`CustomWorkbookViews`] |
//! | 12 | `pivotCaches` | [`PivotCaches`] |
//! | 13 | `smartTagPr` | [`SmartTagProperties`] |
//! | 14 | `smartTagTypes` | [`SmartTagTypes`] |
//! | 15 | `webPublishing` | [`WebPublishing`] |
//! | 16 | `fileRecoveryPr` | [`FileRecoveryProperties`] — the one `maxOccurs="unbounded"` slot |
//! | 17 | `webPublishObjects` | [`WebPublishObjects`] |
//! | 18 | `extLst` | **[`WorkbookContent::Raw`]**, on purpose — see below |
//!
//! The ranks are never written down here. Every placement goes through
//! [`mjx_ooxml_types::child_order::WORKBOOK`], which `cargo run -p xtask -- codegen` generates from
//! `sml.xsd` itself; MJXOFF-89 (A7c) deleted fourteen hand-rolled ordering tables and this crate is
//! not going to add a fifteenth.
//!
//! # Why `extLst` is not modelled, and why that is the strongest option
//!
//! `CT_ExtensionList` is a bag of `ext` elements, each identified by a GUID `uri` and holding markup
//! in *somebody else's* namespace. Nothing in this workspace models one — not `mjx-dml`, not
//! `mjx-pptx`, not `mjx-docx` — because a typed model of an extension list would be a typed model of
//! a hole. It falls into [`WorkbookContent::Raw`], which is this type's unknown bucket, and comes
//! back byte-identical with its prefix bindings intact.
//!
//! That is load-bearing for a real file. `tests/fixtures/sample.xlsx` writes
//!
//! ```xml
//! <extLst><ext xmlns:loext="http://schemas.libreoffice.org/"
//!              uri="{7626C862-2A13-11E5-B345-FEFF819CDC9F}">
//!   <loext:extCalcPr stringRefSyntax="CalcA1"/></ext></extLst>
//! ```
//!
//! — a LibreOffice extension that states the workbook's *reference syntax*, the same thing
//! `calcPr/@refMode` states, through a different producer's mechanism. A consumer that read only
//! `refMode` would be ignoring half of what the file says, and one that "normalised" the extension
//! away would destroy a setting it does not understand. Both are wrong, and neither is possible
//! here: the whole `extLst` subtree is preserved as it stood, prefix and all.
//!
//! # Reading is not mutating
//!
//! A getter takes `&self` and never rewrites anything. Every element keeps the [`RawName`] it was
//! read with (so a Strict document's prefixes survive), its whole attribute vector in order (so
//! `sample.xlsx`'s undeclared `workbookPr/@dateCompatibility` survives), and its self-closing flag.
//! A model returned to its own tree through
//! [`ToXml::write_back`](mjx_ooxml_core::ToXml::write_back) gives every unchanged subtree its
//! verbatim source range back, so editing one sheet's name re-flows one start tag and copies the
//! rest.

pub(crate) mod leaf;

mod calculation;
mod defined_names;
mod properties;
mod references;
mod sheets;
mod views;
mod web;

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawDocument, RawElement, RawName, RawNode,
};
use mjx_ooxml_types::child_order::WORKBOOK;
use mjx_ooxml_types::namespaces::SML;
use mjx_ooxml_types::shared::ConformanceClass;

use crate::error::SmlError;

pub use calculation::CalculationProperties;
pub use defined_names::{BuiltInName, DefinedName, DefinedNames, DefinedNamesContent};
pub use properties::{
    EmbeddedObjectSize, FileRecoveryProperties, FileSharing, FileVersion, WorkbookProperties,
    WorkbookProtection,
};
pub use references::{
    ExternalReference, ExternalReferences, ExternalReferencesContent, FunctionGroup,
    FunctionGroups, FunctionGroupsContent, PivotCache, PivotCaches, PivotCachesContent,
};
pub use sheets::{SheetEntry, SheetList, SheetListContent};
pub use views::{
    BookViews, BookViewsContent, CustomWorkbookView, CustomWorkbookViews,
    CustomWorkbookViewsContent, WorkbookView,
};
pub use web::{
    SmartTagProperties, SmartTagType, SmartTagTypes, SmartTagTypesContent, WebPublishObject,
    WebPublishObjects, WebPublishObjectsContent, WebPublishing,
};

/// `x:workbook` (`CT_Workbook`, `sml.xsd:4097`) — the whole workbook part.
///
/// See the [module documentation](self) for the nineteen slots, for why `extLst` is deliberately
/// unmodelled, and for where the boundary with `mjx-xlsx` runs.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = SML)]
#[xml(attribute(local = "conformance", codec = Enumeration<ConformanceClass>, accessor = conformance))]
pub struct WorkbookPart {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "fileVersion", variant = FileVersion, ty = FileVersion),
        child(local = "fileSharing", variant = FileSharing, ty = FileSharing),
        child(local = "workbookPr", variant = Properties, ty = WorkbookProperties),
        child(local = "workbookProtection", variant = Protection, ty = WorkbookProtection),
        child(local = "bookViews", variant = BookViews, ty = BookViews),
        child(local = "sheets", variant = Sheets, ty = SheetList),
        child(local = "functionGroups", variant = FunctionGroups, ty = FunctionGroups),
        child(local = "externalReferences", variant = ExternalReferences, ty = ExternalReferences),
        child(local = "definedNames", variant = DefinedNames, ty = DefinedNames),
        child(local = "calcPr", variant = Calculation, ty = CalculationProperties),
        child(local = "oleSize", variant = EmbeddedObjectSize, ty = EmbeddedObjectSize),
        child(local = "customWorkbookViews", variant = CustomWorkbookViews, ty = CustomWorkbookViews),
        child(local = "pivotCaches", variant = PivotCaches, ty = PivotCaches),
        child(local = "smartTagPr", variant = SmartTagProperties, ty = SmartTagProperties),
        child(local = "smartTagTypes", variant = SmartTagTypes, ty = SmartTagTypes),
        child(local = "webPublishing", variant = WebPublishing, ty = WebPublishing),
        child(local = "fileRecoveryPr", variant = FileRecovery, ty = FileRecoveryProperties),
        child(local = "webPublishObjects", variant = WebPublishObjects, ty = WebPublishObjects)
    )]
    content: Vec<WorkbookContent>,
}

/// One child of [`WorkbookPart`]: eighteen modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbookContent {
    /// `x:fileVersion` (rank 0).
    FileVersion(FileVersion),
    /// `x:fileSharing` (rank 1).
    FileSharing(FileSharing),
    /// `x:workbookPr` (rank 2).
    Properties(WorkbookProperties),
    /// `x:workbookProtection` (rank 3).
    Protection(WorkbookProtection),
    /// `x:bookViews` (rank 4).
    BookViews(BookViews),
    /// `x:sheets` (rank 5).
    Sheets(SheetList),
    /// `x:functionGroups` (rank 6).
    FunctionGroups(FunctionGroups),
    /// `x:externalReferences` (rank 7).
    ExternalReferences(ExternalReferences),
    /// `x:definedNames` (rank 8).
    DefinedNames(DefinedNames),
    /// `x:calcPr` (rank 9).
    Calculation(CalculationProperties),
    /// `x:oleSize` (rank 10).
    EmbeddedObjectSize(EmbeddedObjectSize),
    /// `x:customWorkbookViews` (rank 11).
    CustomWorkbookViews(CustomWorkbookViews),
    /// `x:pivotCaches` (rank 12).
    PivotCaches(PivotCaches),
    /// `x:smartTagPr` (rank 13).
    SmartTagProperties(SmartTagProperties),
    /// `x:smartTagTypes` (rank 14).
    SmartTagTypes(SmartTagTypes),
    /// `x:webPublishing` (rank 15).
    WebPublishing(WebPublishing),
    /// `x:fileRecoveryPr` (rank 16) — the only slot the schema declares `maxOccurs="unbounded"`.
    FileRecovery(FileRecoveryProperties),
    /// `x:webPublishObjects` (rank 17).
    WebPublishObjects(WebPublishObjects),
    /// Everything this type does not model — `x:extLst` above all, plus any foreign element, any
    /// `mc:AlternateContent`, and any comment or processing instruction between siblings.
    ///
    /// Preserved verbatim and in position: placement skips a node it cannot rank, so an unmodelled
    /// child never moves and never moves anything else.
    Raw(RawNode),
}

impl WorkbookContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    #[must_use]
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::FileVersion(_) => "fileVersion",
            Self::FileSharing(_) => "fileSharing",
            Self::Properties(_) => "workbookPr",
            Self::Protection(_) => "workbookProtection",
            Self::BookViews(_) => "bookViews",
            Self::Sheets(_) => "sheets",
            Self::FunctionGroups(_) => "functionGroups",
            Self::ExternalReferences(_) => "externalReferences",
            Self::DefinedNames(_) => "definedNames",
            Self::Calculation(_) => "calcPr",
            Self::EmbeddedObjectSize(_) => "oleSize",
            Self::CustomWorkbookViews(_) => "customWorkbookViews",
            Self::PivotCaches(_) => "pivotCaches",
            Self::SmartTagProperties(_) => "smartTagPr",
            Self::SmartTagTypes(_) => "smartTagTypes",
            Self::WebPublishing(_) => "webPublishing",
            Self::FileRecovery(_) => "fileRecoveryPr",
            Self::WebPublishObjects(_) => "webPublishObjects",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Workbook`'s `xsd:sequence`, from the generated table.
    ///
    /// `None` for a node the table does not name, which is exactly the set of nodes placement must
    /// step over rather than treat as a boundary.
    #[must_use]
    fn rank(&self) -> Option<u16> {
        WORKBOOK.rank_of(None, self.local()?)
    }
}

/// Declares one singleton slot: a borrowing getter, a mutable getter, and a setter that replaces the
/// existing child in place or inserts a new one at its rank in `CT_Workbook`'s sequence.
///
/// The three bodies are identical for all seventeen singleton slots, and writing them out
/// seventeen times would be seventeen chances to reach for the wrong variant. Modelled on
/// `mjx_docx::document::property_macros::value_property!`, which exists for the same reason one
/// layer up.
macro_rules! singleton_slot {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|item| match item {
                WorkbookContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the workbook has none.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            self.content.iter_mut().find_map(|item| match item {
                WorkbookContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some(value)` replaces the \
            existing element **where it is**, or inserts a new one at its rank in `CT_Workbook`'s \
            `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            let is_target = |item: &WorkbookContent| matches!(item, WorkbookContent::$variant(_));
            self.replace_or_insert($local, is_target, value.map(WorkbookContent::$variant));
        }
    };
}

impl WorkbookPart {
    /// Reads a whole `xl/workbook.xml` part.
    ///
    /// `Ok(None)` when the document's root is not an `x:workbook` — the caller handed over a
    /// different part, which is a question rather than an error, exactly as
    /// [`SharedStringTable::read_part`](crate::SharedStringTable::read_part) treats it.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a modelled element does not match its complex type — text or an
    /// attribute value that is not UTF-8, or an entity reference that will not decode. Nothing a
    /// well-formed file can *say* is refused.
    pub fn read_part(document: &RawDocument) -> Result<Option<Self>, SmlError> {
        Self::read_root(&document.root, &document.interner)
    }

    /// [`read_part`](Self::read_part) for a caller that holds the root element and the interner
    /// rather than the whole document.
    ///
    /// That is the shape an *editing* caller is in: reaching a part's tree mutably yields a
    /// `&mut RawDocument`, and the root and the interner have to be borrowed apart from each other
    /// before the model can be parsed from one and written back through both.
    ///
    /// # Errors
    /// As [`read_part`](Self::read_part).
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        let namespace = root.name.namespace.map(|symbol| interner.resolve(symbol));
        let in_spreadsheetml =
            namespace == Some(SML.transitional) || (namespace.is_some() && namespace == SML.strict);
        if !in_spreadsheetml || interner.resolve(root.name.local) != "workbook" {
            return Ok(None);
        }
        Ok(Some(Self::from_xml(root, interner)?))
    }

    /// Builds an empty `x:workbook`, bound to `prefix` or to the default namespace.
    ///
    /// Declares no namespaces of its own: a part written from this has to bind at least the
    /// SpreadsheetML namespace, and — if any sheet is to name a relationship — the
    /// relationship-reference one. MJXOFF-112 (D10) is what writes whole parts from nothing;
    /// this exists so that the model is constructible rather than only readable.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: leaf::sml_name(interner, prefix, "workbook"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The prefix this part binds to the relationship-reference namespace, from its own root-element
    /// `xmlns:` declarations — `r` in every file this project has read, and the producer's choice
    /// rather than the schema's.
    ///
    /// Hand this to [`SheetEntry::relationship_id`], [`ExternalReference::relationship_id`] and
    /// [`PivotCache::relationship_id`]. `None` means the part binds the namespace nowhere, so no
    /// element in it can carry an `r:id` at all.
    #[must_use]
    pub fn relationship_prefix<'a>(&self, interner: &'a Interner) -> Option<&'a str> {
        leaf::namespace_prefix(&self.attributes, interner, leaf::RELATIONSHIP_REFERENCE)
    }

    /// Every child, in document order, including the ones this type does not model.
    pub fn content(&self) -> &[WorkbookContent] {
        &self.content
    }

    singleton_slot!(
        file_version,
        file_version_mut,
        set_file_version,
        FileVersion,
        FileVersion,
        "fileVersion",
        "`x:fileVersion` — which application last wrote the workbook. `None` if it wrote none."
    );
    singleton_slot!(
        file_sharing,
        file_sharing_mut,
        set_file_sharing,
        FileSharing,
        FileSharing,
        "fileSharing",
        "`x:fileSharing` — the write-reservation password and read-only recommendation. `None` if \
         the workbook declares neither."
    );
    singleton_slot!(
        properties,
        properties_mut,
        set_properties,
        Properties,
        WorkbookProperties,
        "workbookPr",
        "`x:workbookPr` — the date system, the object-display setting and a dozen producer \
         preferences. `None` if the workbook writes none."
    );
    singleton_slot!(
        protection,
        protection_mut,
        set_protection,
        Protection,
        WorkbookProtection,
        "workbookProtection",
        "`x:workbookProtection` — whether the sheet structure and window layout are locked. `None` \
         if the element is absent; note that `sample.xlsx` writes an **empty** one, which is \
         present with every attribute defaulted and is not the same thing."
    );
    singleton_slot!(
        book_views,
        book_views_mut,
        set_book_views,
        BookViews,
        BookViews,
        "bookViews",
        "`x:bookViews` — the window views: which tab is active, where the window sat, how wide the \
         tab strip was."
    );
    singleton_slot!(
        sheets,
        sheets_mut,
        set_sheets,
        Sheets,
        SheetList,
        "sheets",
        "`x:sheets` — the sheet list, in tab order. The schema declares it `minOccurs=\"1\"`, so a \
         workbook without one is invalid; it is still read, and reported as `None`."
    );
    singleton_slot!(
        function_groups,
        function_groups_mut,
        set_function_groups,
        FunctionGroups,
        FunctionGroups,
        "functionGroups",
        "`x:functionGroups` — the worksheet-function groups an add-in registered."
    );
    singleton_slot!(
        external_references,
        external_references_mut,
        set_external_references,
        ExternalReferences,
        ExternalReferences,
        "externalReferences",
        "`x:externalReferences` — the relationships to external-link parts. The parts themselves \
         are MJXOFF-133's (D18); only the references are modelled here."
    );
    singleton_slot!(
        defined_names,
        defined_names_mut,
        set_defined_names,
        DefinedNames,
        DefinedNames,
        "definedNames",
        "`x:definedNames` — the names a formula may use in place of a range, including the eight \
         SpreadsheetML reserves."
    );
    singleton_slot!(
        calculation_properties,
        calculation_properties_mut,
        set_calculation_properties,
        Calculation,
        CalculationProperties,
        "calcPr",
        "`x:calcPr` — the calculation settings, reported and never acted on: there is no \
         calculation engine here."
    );
    singleton_slot!(
        embedded_object_size,
        embedded_object_size_mut,
        set_embedded_object_size,
        EmbeddedObjectSize,
        EmbeddedObjectSize,
        "oleSize",
        "`x:oleSize` — the range an OLE container shows when this workbook is embedded in another \
         document."
    );
    singleton_slot!(
        custom_workbook_views,
        custom_workbook_views_mut,
        set_custom_workbook_views,
        CustomWorkbookViews,
        CustomWorkbookViews,
        "customWorkbookViews",
        "`x:customWorkbookViews` — the saved custom views."
    );
    singleton_slot!(
        pivot_caches,
        pivot_caches_mut,
        set_pivot_caches,
        PivotCaches,
        PivotCaches,
        "pivotCaches",
        "`x:pivotCaches` — the relationships to pivot-cache definition parts. The parts themselves \
         are MJXOFF-133's (D18)."
    );
    singleton_slot!(
        smart_tag_properties,
        smart_tag_properties_mut,
        set_smart_tag_properties,
        SmartTagProperties,
        SmartTagProperties,
        "smartTagPr",
        "`x:smartTagPr` — whether smart-tag data is embedded, and how a consumer indicates one."
    );
    singleton_slot!(
        smart_tag_types,
        smart_tag_types_mut,
        set_smart_tag_types,
        SmartTagTypes,
        SmartTagTypes,
        "smartTagTypes",
        "`x:smartTagTypes` — the recognised smart-tag types."
    );
    singleton_slot!(
        web_publishing,
        web_publishing_mut,
        set_web_publishing,
        WebPublishing,
        WebPublishing,
        "webPublishing",
        "`x:webPublishing` — how the workbook was rendered when it was saved as a web page."
    );
    singleton_slot!(
        web_publish_objects,
        web_publish_objects_mut,
        set_web_publish_objects,
        WebPublishObjects,
        WebPublishObjects,
        "webPublishObjects",
        "`x:webPublishObjects` — the ranges and objects published to a web page."
    );

    /// Every `x:fileRecoveryPr`, in document order.
    ///
    /// A list, not an `Option`: this is the **only** slot in `CT_Workbook`'s nineteen that the
    /// schema declares `maxOccurs="unbounded"`, and a model that assumed one would silently drop the
    /// rest of a file that wrote several.
    pub fn file_recovery_properties(&self) -> impl Iterator<Item = &FileRecoveryProperties> + '_ {
        self.content.iter().filter_map(|item| match item {
            WorkbookContent::FileRecovery(value) => Some(value),
            _ => None,
        })
    }

    /// Appends one `x:fileRecoveryPr` at its rank in the sequence, after any already present.
    pub fn push_file_recovery_properties(&mut self, value: FileRecoveryProperties) {
        let at = self.insert_index("fileRecoveryPr");
        self.content
            .insert(at, WorkbookContent::FileRecovery(value));
        self.empty = false;
    }

    /// Where a child named `local` belongs among the current children.
    ///
    /// One call into the generated table: unranked nodes are stepped over rather than treated as a
    /// boundary, so an `mc:AlternateContent` or a comment between two slots neither moves nor
    /// displaces what is inserted next to it.
    fn insert_index(&self, local: &str) -> usize {
        WORKBOOK.insert_index_of_names(self.content.iter().map(WorkbookContent::rank), local)
    }

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&WorkbookContent) -> bool,
        value: Option<WorkbookContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = self.insert_index(local);
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::ToXml;

    /// Parses `markup` as a workbook part.
    fn read(markup: &str) -> (RawDocument, WorkbookPart) {
        let document = mjx_xml::fidelity::parse(markup.as_bytes()).expect("the part parses");
        let part = WorkbookPart::read_part(&document)
            .expect("the part reads")
            .expect("the root is an x:workbook");
        (document, part)
    }

    const SML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    /// Every slot the generated table names is one this type models — or `extLst`, which is
    /// deliberately raw. A slot added to `sml.xsd` and regenerated would fail here rather than be
    /// silently dropped into the unknown bucket.
    #[test]
    fn every_slot_of_the_generated_sequence_is_accounted_for() {
        assert_eq!(WORKBOOK.symbol, "CT_Workbook");
        assert_eq!(
            WORKBOOK.slots.len(),
            19,
            "CT_Workbook is a nineteen-slot sequence"
        );
        let modelled: Vec<&'static str> = (0..18)
            .map(|rank| {
                WORKBOOK
                    .slots
                    .iter()
                    .find(|slot| slot.rank == rank)
                    .expect("every rank is occupied")
                    .local
            })
            .collect();
        for local in &modelled {
            assert!(
                WORKBOOK.rank_of(None, local).is_some(),
                "{local} must be rankable"
            );
        }
        let last = WORKBOOK
            .slots
            .iter()
            .find(|slot| slot.rank == 18)
            .expect("rank 18 is occupied");
        assert_eq!(
            last.local, "extLst",
            "the one unmodelled slot must be the extension list"
        );
    }

    /// A part with nothing but a sheet list reads, and the list is reached without a package.
    #[test]
    fn a_minimal_workbook_reads_its_sheet_list() {
        let (document, part) = read(&format!(
            r#"<workbook xmlns="{SML_NS}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="One" sheetId="4" r:id="rId9"/></sheets></workbook>"#
        ));
        let sheets = part.sheets().expect("a sheet list");
        assert_eq!(sheets.len(), 1);
        let entry = sheets.entries().next().expect("one entry");
        assert_eq!(
            entry.name(&document.interner).expect("a name").as_deref(),
            Some("One")
        );
        assert_eq!(entry.sheet_id(&document.interner).expect("an id"), Some(4));
        let prefix = part.relationship_prefix(&document.interner);
        assert_eq!(prefix, Some("r"));
        assert_eq!(
            entry
                .relationship_id(&document.interner, prefix)
                .expect("an r:id"),
            Some("rId9".to_owned())
        );
    }

    /// A part whose root is something else is a question, not an error.
    #[test]
    fn a_part_that_is_not_a_workbook_reads_as_none() {
        let document = mjx_xml::fidelity::parse(format!(r#"<sst xmlns="{SML_NS}"/>"#).as_bytes())
            .expect("parses");
        assert!(WorkbookPart::read_part(&document)
            .expect("no error")
            .is_none());
        // …and so is an element that is merely *named* workbook in somebody else's namespace.
        let foreign = mjx_xml::fidelity::parse(br#"<workbook xmlns="urn:not-spreadsheetml"/>"#)
            .expect("parses");
        assert!(WorkbookPart::read_part(&foreign)
            .expect("no error")
            .is_none());
    }

    /// A newly set child lands at its rank in the sequence, not at the end.
    #[test]
    fn a_new_child_is_placed_at_its_schema_rank() {
        let (mut document, mut part) = read(&format!(
            r#"<workbook xmlns="{SML_NS}"><sheets/><calcPr/></workbook>"#
        ));
        let properties = WorkbookProperties::new(&mut document.interner, None);
        part.set_properties(Some(properties));
        let locals: Vec<Option<&str>> = part.content().iter().map(WorkbookContent::local).collect();
        assert_eq!(
            locals,
            vec![Some("workbookPr"), Some("sheets"), Some("calcPr")],
            "workbookPr ranks 2, before sheets at 5"
        );
    }

    /// An unmodelled child is stepped over by placement rather than treated as a boundary, and
    /// keeps its position relative to the ranked siblings around it.
    ///
    /// The insertion point is *one past the last sibling that must precede* the new child, and a
    /// node the table cannot rank is neither that sibling nor a stopping point. So `sheets` lands
    /// immediately after `fileVersion`, and the foreign node is still where it was: after
    /// `fileVersion`, before `calcPr`.
    #[test]
    fn an_unmodelled_child_is_stepped_over_rather_than_treated_as_a_boundary() {
        let (mut document, mut part) = read(&format!(
            r#"<workbook xmlns="{SML_NS}"><fileVersion/><q:note xmlns:q="urn:q"/><calcPr/></workbook>"#
        ));
        let sheets = SheetList::new(&mut document.interner, None);
        part.set_sheets(Some(sheets));
        let locals: Vec<Option<&str>> = part.content().iter().map(WorkbookContent::local).collect();
        assert_eq!(
            locals,
            vec![Some("fileVersion"), Some("sheets"), None, Some("calcPr")],
            "sheets ranks 5, so it goes one past fileVersion (0); the unranked node is skipped \
             rather than ending the scan, and calcPr (9) still follows it"
        );
        // The foreign node itself is untouched — same element, same namespace binding.
        let WorkbookContent::Raw(RawNode::Element(foreign)) = &part.content()[2] else {
            panic!("the third child is the foreign element");
        };
        assert_eq!(document.interner.resolve(foreign.name.local), "note");
    }

    /// A read followed by a write reproduces the part exactly, extension list and all.
    #[test]
    fn a_read_and_a_rebuild_reproduce_the_bytes() {
        let markup = format!(
            r#"<workbook xmlns="{SML_NS}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><workbookPr backupFile='false' dateCompatibility="false"/><sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets><extLst><ext xmlns:loext="urn:loext" uri="{{GUID}}"><loext:extCalcPr stringRefSyntax="CalcA1"/></ext></extLst></workbook>"#
        );
        let (mut document, part) = read(&markup);
        part.write_back(&mut document.root, &mut document.interner);
        assert_eq!(
            mjx_xml::fidelity::serialize_to_vec(&document),
            markup.as_bytes(),
            "an undeclared attribute, a single-quoted one and a foreign extension all survive"
        );
    }
}
