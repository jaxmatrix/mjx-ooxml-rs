//! The error type for the WordprocessingML layer.

use mjx_ooxml_core::FromXmlError;
use mjx_ooxml_types::wordprocessingml::StyleType;
use mjx_opc::OpcError;
use mjx_xml::XmlError;

/// Errors produced while opening, reading, editing, or saving a Word document.
///
/// # Deliberately exhaustive
///
/// This enum is **not** `#[non_exhaustive]`, so a `match` over it must name every variant — the same
/// contract `mjx_pptx::PptxError` documents on itself: [`mjx_ooxml::Error`] collapses every variant
/// here into one of its stable [`ErrorCode`]s through a `match` with no wildcard arm, so **adding a
/// variant to this enum fails to compile until it is classified there**. A wildcard arm would
/// silently file the new failure under whatever code the catch-all named, which is exactly the bug
/// the exhaustive match exists to prevent.
///
/// [`mjx_ooxml::Error`]: https://docs.rs/mjx-ooxml
/// [`ErrorCode`]: https://docs.rs/mjx-ooxml
#[derive(Debug, thiserror::Error)]
pub enum DocxError {
    /// The underlying OPC package could not be read, edited, or written.
    #[error(transparent)]
    Opc(#[from] OpcError),

    /// A part was not well-formed XML.
    #[error(transparent)]
    Xml(#[from] XmlError),

    /// A modeled element (e.g. the document root's `@conformance`) was malformed.
    #[error(transparent)]
    Model(#[from] FromXmlError),

    /// A header or footer part's `mc:AlternateContent` markup could not be resolved (MJXOFF-113's
    /// own VML-exposure path — see `crate::document::headers`'s own doc comment).
    #[error(transparent)]
    Mce(#[from] mjx_mce::ResolveError),

    /// A `w:pict` a header or footer part carries did not parse as legacy VML.
    #[error(transparent)]
    Vml(#[from] mjx_vml::VmlError),

    /// The package root has no `officeDocument` relationship (not an Office document).
    #[error("package has no officeDocument relationship")]
    MissingOfficeDocument,

    /// The main document part named by the `officeDocument` relationship is absent.
    #[error("document part {0} is missing from the package")]
    MissingDocumentPart(String),

    /// `word/document.xml` (or another part this crate resolves) did not have the expected
    /// structure.
    #[error("document is malformed: {0}")]
    MalformedDocument(&'static str),

    /// A relationship target could not be resolved to a part name.
    #[error("relationship target {target} could not be resolved")]
    TargetResolution {
        /// The unresolvable target.
        target: String,
    },

    /// A relationship target points outside the package (not supported here).
    #[error("external relationship target {target} is not supported")]
    ExternalTarget {
        /// The external target.
        target: String,
    },

    /// The document declares no `w:body` (legal per the schema — `CT_Document`'s `body` is
    /// `minOccurs="0"` — but there is then nothing for a paragraph/run address to resolve against).
    #[error("document has no w:body")]
    NoBody,

    /// A [`crate::BlockPath`] or [`crate::RunPath`] did not resolve — out of range, or (for a run
    /// path) landing on a container rather than, in the end, a run.
    #[error("{0}")]
    AddressNotFound(String),

    /// [`crate::Document::blank`] (or [`crate::Document::blank_with_properties`]) was asked for a
    /// page size this crate's fixed margins cannot fit inside — see [`crate::PageSize`]'s own doc
    /// comment for why this, not a `ST_TwipsMeasure` numeric range, is the real constraint.
    #[error("page size {width_twips}x{height_twips} twips is invalid: {reason}")]
    InvalidPageSize {
        /// The page width asked for, in twips.
        width_twips: u32,
        /// The page height asked for, in twips.
        height_twips: u32,
        /// Why the size was refused.
        reason: &'static str,
    },

    /// A style sheet has no style with this `styleId` — returned by [`crate::StyleIndex`] when the
    /// *starting* point of a lookup or a `w:basedOn` walk does not resolve (an *ancestor* the walk
    /// cannot resolve is not an error — see [`crate::StyleIndex::based_on_chain`]'s own doc
    /// comment).
    #[error("no style with styleId {0:?}")]
    UnknownStyleId(String),

    /// A `w:basedOn` chain starting at `style_id` did not terminate within `limit` steps — almost
    /// certainly a cycle (`w:basedOn` pointing back at an ancestor, directly or indirectly) rather
    /// than a legitimately deep hierarchy. See [`crate::MAX_BASED_ON_CHAIN_DEPTH`]'s own doc comment
    /// for why a bounded depth, not a visited-set, is how this is detected.
    #[error(
        "w:basedOn chain starting at styleId {style_id:?} did not terminate within {limit} steps \
         (likely a cycle)"
    )]
    BasedOnChainTooDeep {
        /// The style the chain walk started from.
        style_id: String,
        /// The bound that was exceeded.
        limit: usize,
    },

    /// [`crate::NumberingIndex::resolve`] (or [`crate::Document::resolve_numbering`]): a `numId`
    /// naming no `w:num` in the numbering definitions part — or, when the document relates to no
    /// `word/numbering.xml` at all, any non-zero `numId` a caller asks to resolve. `0` is never this
    /// — it means "no numbering"; see [`crate::NumberingLookup::None`].
    #[error("no w:num with numId {0}")]
    UnknownNumberingId(i64),

    /// A `w:num`'s own `w:abstractNumId` names no `w:abstractNum` in the same numbering definitions
    /// part.
    #[error("no w:abstractNum with abstractNumId {0}")]
    UnknownAbstractNumberingId(i64),

    /// The `w:num` named by `.0` (its own `numId`) carries no `w:abstractNumId` at all — legal only
    /// for a non-conformant file (`CT_Num`'s `abstractNumId` is `minOccurs="1"`); never rejected on
    /// read (fidelity-first: preserve what is there), only here, on resolution.
    #[error("w:num with numId {0} carries no w:abstractNumId")]
    MissingAbstractNumberingReference(i64),

    /// [`crate::Document::resolve_numbering`]: a `w:abstractNum/w:numStyleLink` names a `styleId`
    /// that does not resolve — either the document relates to no `word/styles.xml` at all, or that
    /// part does not define a style with this id.
    #[error("w:numStyleLink names styleId {style_id:?}, which does not resolve")]
    NumberingStyleLinkTargetMissing {
        /// The unresolved `styleId`.
        style_id: String,
    },

    /// A `w:numStyleLink` target resolves, but is not itself a numbering-type style
    /// (`w:style/@type="numbering"`).
    #[error(
        "w:numStyleLink names styleId {style_id:?}, which is not a numbering-type style (found \
         {found:?})"
    )]
    NumberingStyleLinkWrongKind {
        /// The linked style's own id.
        style_id: String,
        /// The linked style's actual kind, if it states one.
        found: Option<StyleType>,
    },

    /// A `w:numStyleLink` target resolves to a numbering-type style, but that style carries no
    /// `w:pPr/w:numPr/w:numId` of its own to redirect through.
    #[error("numbering style {style_id:?} carries no w:pPr/w:numPr/w:numId")]
    NumberingStyleLinkHasNoNumbering {
        /// The linked style's own id.
        style_id: String,
    },

    /// A `w:numStyleLink` redirect chain starting at `numbering_id` did not terminate within
    /// `limit` hops — almost certainly a cycle. See [`crate::MAX_NUM_STYLE_LINK_DEPTH`]'s own doc
    /// comment for why a bounded depth, not a visited-set, is how this is detected.
    #[error(
        "w:numStyleLink chain starting at numId {numbering_id} did not terminate within {limit} \
         hops (likely a cycle)"
    )]
    NumberingStyleLinkTooDeep {
        /// The `numId` the redirect chain started from.
        numbering_id: i64,
        /// The bound that was exceeded.
        limit: usize,
    },

    /// [`crate::Document::resolve_header`]/`resolve_footer`/`create_header`/`create_footer`/
    /// `remove_header`/`remove_footer`: `index` does not name one of the document's `count` sections
    /// (see [`crate::SectionSpan`]).
    #[error("section index {index} is out of range (document has {count} sections)")]
    SectionOutOfRange {
        /// The out-of-range index asked for.
        index: usize,
        /// How many sections the document actually has.
        count: usize,
    },

    /// A table `(row, column)` address named a cell outside the table's own dimensions, **or**
    /// (for [`crate::Document::merged_cell_anchor`]) a `w:vMerge` continuation with no reachable
    /// anchor — the malformed-grid case ECMA-376 Part 1 §17.4.84 itself allows, exposed here rather
    /// than confused with an ordinary out-of-range address. Mirrors `mjx_pptx::PptxError::
    /// TableCellOutOfRange`'s own field names.
    #[error("table cell ({row}, {column}) is out of range ({rows} rows, {columns} columns)")]
    TableCellOutOfRange {
        /// The row asked for.
        row: usize,
        /// The column asked for.
        column: usize,
        /// How many rows the table actually has.
        rows: usize,
        /// How many columns the table's grid actually declares.
        columns: usize,
    },

    /// A structural row/column edit was refused because it would leave a table with no rows or no
    /// columns — mirrors `mjx_pptx::PptxError::InvalidTableSize`.
    #[error("table would have {rows} rows and {columns} columns, which is invalid")]
    InvalidTableSize {
        /// The row count the edit would have left.
        rows: usize,
        /// The column count the edit would have left.
        columns: usize,
    },

    /// A `w:fldChar` `begin`/`separate`/`end` marker sequence within one paragraph does not
    /// balance — an unmatched `separate`, an unmatched `end`, or a `begin` with no matching `end`
    /// before the paragraph's own content is exhausted. Schema-valid markup — `ST_FldCharType`
    /// imposes no ordering or balance constraint of its own (MJXOFF-121) — that real `.docx` files
    /// do contain; reported here rather than silently mispaired or panicked on.
    #[error("unbalanced field marker sequence: {0}")]
    UnbalancedField(String),

    /// [`crate::Document::fields`]/`set_field_instruction`/`set_field_cached_result_text`
    /// (MJXOFF-121): `path` does not address a field within the given paragraph.
    #[error("no field at {0}")]
    FieldNotFound(String),

    /// [`crate::Document::set_field_cached_result_text`] (MJXOFF-121) was asked to edit a complex
    /// field's cached result, but that field carries no `w:fldChar` `separate` marker — there is no
    /// cached-result zone to edit (a field with no `separate` is legal markup, not malformed; see
    /// `crate::Field::cached_result`'s own doc comment).
    #[error("field has no w:fldChar separate marker, so it carries no cached result to edit")]
    FieldHasNoCachedResult,

    /// [`crate::Document::set_field_instruction`]/`set_field_cached_result_text` (MJXOFF-121) was
    /// asked to edit a zone that itself contains a nested field — collapsing it to plain text would
    /// silently destroy the nested field's own markup, so the edit is refused instead.
    #[error("field's {zone} contains a nested field; editing it as plain text would destroy it")]
    FieldHasNestedContent {
        /// Which zone refused the edit — `"instruction"` or `"cached result"`.
        zone: &'static str,
    },

    /// A caller-supplied value exceeds the schema's own length bound for this attribute
    /// (MJXOFF-121: `ST_FFName` maxLength 65, `ST_FFHelpTextVal` 256, `ST_FFStatusTextVal` 140,
    /// `ST_MacroName` 33) — refused here, at the API boundary, rather than written and only failing
    /// the schema gate later. Reading an already-over-long value from an untrusted file is never
    /// rejected; only a caller's own new value is.
    #[error("{field} is {len} characters, which exceeds the schema's {max}-character limit")]
    ValueTooLong {
        /// Which value was refused (`"form field name"`, `"help text"`, `"status text"`, `"macro
        /// name"`).
        field: &'static str,
        /// The schema's own `maxLength` bound.
        max: usize,
        /// The length (in Unicode scalar values) of the value that was refused.
        len: usize,
    },

    /// [`crate::Document::add_bookmark`] (MJXOFF-124): another `w:bookmarkStart` anywhere in the body
    /// already carries this `w:name` — refused here so [`crate::Document::resolve_bookmark`] never has
    /// to guess which of two same-named bookmarks a `w:hyperlink w:anchor` meant. Reading a file that
    /// already has two bookmarks sharing a name is never rejected (fidelity-first); only authoring a
    /// new collision is.
    #[error("bookmark name {0:?} is already in use")]
    BookmarkNameInUse(String),

    /// MJXOFF-126: a caller-supplied `@date` for a revision marker (`w:ins`/`w:del`/`w:moveFrom`/
    /// `w:moveTo`, a `*Change` wrapper, `w:cellMerge`, `w:numberingChange`, `w:moveFromRangeStart`/
    /// `w:moveToRangeStart`) is not a well-formed `ST_DateTime` (`xsd:dateTime`). Refused here, at
    /// the API boundary — matching `ValueTooLong`'s own precedent — rather than written and only
    /// failing the schema gate later. **Reading an already-malformed `@date` from an untrusted file
    /// is never rejected or normalised**; only a caller's own new value is checked (see
    /// `crate::document::revisions`'s own module doc for why normalising someone's revision history
    /// would itself be a corruption).
    #[error("{0:?} is not a well-formed ST_DateTime (xsd:dateTime)")]
    MalformedDateTime(String),

    /// [`crate::Document::resolve_data_binding`] (MJXOFF-138): no Custom XML Data Storage part
    /// related to the main document part carries a Custom XML Data Storage Properties part
    /// (§15.2.6) whose `ds:datastoreItem/@ds:itemID` matches `store_item_id` — the file's own
    /// `w:dataBinding` names a part the package does not carry (or the properties part relationship
    /// is itself missing/malformed). Reported rather than panicked on: a broken binding is exactly
    /// the untrusted-input case this crate's fidelity rules require surfacing, not crashing on.
    #[error("no Custom XML Data Storage part has itemID {store_item_id:?}")]
    DataBindingPartNotFound {
        /// The `w:dataBinding/@storeItemID` that did not resolve.
        store_item_id: String,
    },

    /// [`crate::Document::resolve_data_binding`]: the Custom XML Data Storage part named by
    /// `store_item_id` was found, but `xpath` did not resolve against it — either it uses something
    /// outside the documented subset [`crate::resolve_xpath`]'s own doc comment names, or it is a
    /// well-formed step sequence that simply does not match the part's own tree (an out-of-range
    /// index, a step naming an element the tree does not carry at that position).
    #[error(
        "xpath {xpath:?} does not resolve against Custom XML Data Storage part {store_item_id:?}"
    )]
    DataBindingXPathNotFound {
        /// The `w:dataBinding/@storeItemID` that did resolve.
        store_item_id: String,
        /// The `w:dataBinding/@xpath` that did not.
        xpath: String,
    },

    /// [`crate::Document::alt_chunk_payload`] (MJXOFF-138): a `w:altChunk` names a relationship id
    /// this document part's own `.rels` does not carry (or names one of the wrong type, or an
    /// external target — ECMA-376 Part 1 §17.17.2.1 states a document carrying either "shall be
    /// considered non-conformant"). Reported rather than panicked on.
    #[error(
        "altChunk relationship {relationship_id:?} does not resolve to an internal aFChunk part"
    )]
    AltChunkRelationshipNotFound {
        /// The `w:altChunk/@r:id` that did not resolve.
        relationship_id: String,
    },
}
