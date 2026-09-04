//! `w:hdr` / `w:ftr` (`CT_HdrFtr`) — a header or footer part's own content — and **variant
//! resolution**: given a section and which of the three variants (`default`/`even`/`first`) a page
//! needs, which header or footer part actually applies.
//!
//! # `CT_HdrFtr` is `EG_BlockLevelElts`, minus the trailing `w:sectPr` only `CT_Body` appends
//!
//! `wml.xsd` declares `CT_HdrFtr` as exactly `EG_BlockLevelElts` (`§3391`, `wml.xsd`) —
//! paragraphs, tables and range markup, the same repeatable choice group `CT_Body` opens with
//! before its own trailing `w:sectPr`. [`HdrFtr`] therefore reuses [`super::body::BlockContent`]
//! (MJXOFF-92's own type) and the paragraph-vec addressing `body.rs` generalized for this child
//! (`block_paragraph`, `block_insert_paragraph`, …) rather than declaring a second content enum or a
//! second copy of `Body`'s paragraph methods — see `body.rs`'s own doc comment on that
//! generalization. A `w:sectPr` a non-conformant header part carried would simply fall to
//! `BlockContent::Raw`, exactly as an out-of-place element does anywhere else this crate models.
//!
//! Once addressed as a container ([`HdrFtr::paragraph_mut`], …), a header or footer's paragraphs and
//! runs are [`super::Paragraph`]/[`super::Run`] — MJXOFF-92's own types, unmodified — so MJXOFF-94's
//! run properties, MJXOFF-96's paragraph properties and MJXOFF-106's effective-property ladder all
//! already work inside a header or footer with no further wiring: none of the three reaches into
//! `Body` or `HdrFtr` themselves, only into the [`super::Paragraph`]/[`super::Run`] a caller already
//! holds.
//!
//! # Variant resolution — ECMA-376 Part 1 §17.10.1, §17.10.5/.2, §17.10.6, quoted
//!
//! A section can reference up to three headers (`w:headerReference`) and three footers
//! (`w:footerReference`), one per [`HeaderFooterType`] (`default`/`even`/`first`). *Which* one a
//! renderer shows for a given page is not a lookup on that list — it is governed by two flags and a
//! fallback-then-inherit rule, all stated in ECMA-376 Part 1 (`References/ECMA-376-1_5th_edition_
//! december_2016/…Part 1….pdf`, extracted with `pdftotext`):
//!
//! - **§17.10.6, `titlePg`** (a per-section `w:sectPr` flag): *"If this element is set to false and
//!   a first page header/footer is specified, then it shall be ignored and only the odd page
//!   header/footer shall be displayed."* So a `type="first"` reference existing is not enough — with
//!   `titlePg` off (the schema default when the element is absent), a query for the first page
//!   resolves exactly as a query for the default (odd) page would.
//! - **§17.10.1, `evenAndOddHeaders`** (a document-wide `w:settings` flag, read directly here —
//!   MJXOFF-136 models the part itself): *"If this element is set to false and an even page
//!   header/footer is specified, then it shall be ignored and only the odd page header/footer shall
//!   be displayed."* Same shape as `titlePg`, for the even variant.
//! - **§17.10.5 (header) / §17.10.2 (footer), inheritance — identical prose in both**: *"If no
//!   headerReference for the \[…\] page header is specified \[…\] the \[…\] page header shall be
//!   inherited from the previous section or, if this is the first section in the document, a new
//!   blank header shall be created."* This is stated **per variant**, independently: a section that
//!   references only its `default` header still inherits `first`/`even` from whatever the nearest
//!   preceding section that names them; a section with no reference at all inherits all three. There
//!   is no reference to inherit from before the document's first section — [`resolve_reference`]
//!   answers `None` there, matching "a new blank header shall be created" (this crate does not
//!   fabricate one on a *read*, only [`super::super::Document::create_header`]/`create_footer` on
//!   request).
//!
//! Both rules are evaluated for the section actually being queried (never for whichever section the
//! winning reference happens to live on), and only *then* does the inheritance walk look backward —
//! matching the prose's own two-step shape: "is this type ignored for this section" first, "which
//! section actually states the (possibly downgraded) type" second. [`resolve_reference`] is that
//! walk; [`super::super::Document::resolve_header`]/`resolve_footer` are its callers, after adding
//! the one thing [`resolve_reference`] cannot: turning the winning reference's `r:id` into the part
//! it names.

use mjx_ooxml_core::{FromXml, FromXmlError, Interner, RawAttribute, RawName};
use mjx_ooxml_types::namespaces::WML;

/// `ST_HdrFtr` (`wml.xsd`) — which of the three header/footer variants (`default`/`even`/`first`) a
/// `w:headerReference`/`w:footerReference` names. Re-exported from `mjx_ooxml_types` rather than
/// redeclared, matching `mjx_docx::PageOrientation`'s own precedent (`crate::page`'s own module doc)
/// for a generated `ST_*` enumeration this crate's public API needs to name.
pub use mjx_ooxml_types::wordprocessingml::HeaderFooterType;

use crate::address::BlockPath;
use crate::error::DocxError;

use super::body::{
    block_append_table, block_insert_paragraph, block_paragraph, block_paragraph_mut,
    block_paragraphs, block_remove_paragraph, block_remove_table, block_table, block_table_mut,
    block_tables, wml_name, BlockContent, Paragraph,
};
use super::sections::{HeaderFooterReference, SectionSpan};
use super::tables::Table;

/// `CT_HdrFtr` — a header or footer part's own root content: block-level content only
/// (`EG_BlockLevelElts`) — see this module's own doc comment for why this reuses
/// [`BlockContent`] (MJXOFF-92's own type) rather than declaring a second content enum.
///
/// **`w:sectPr` is schema-invalid inside `CT_HdrFtr`** (only `CT_Body` carries one), but this
/// struct's own `#[xml(children, …)]` list still maps [`BlockContent::SectionProperties`]: the
/// derive macro generates one exhaustive match over `BlockContent`'s full variant set for *every*
/// struct that holds a `Vec<BlockContent>`, so a struct whose list omits a variant fails to compile,
/// not merely fails to parse it — `Body`'s own list is the only one that gets to omit nothing, and
/// this one reusing the same enum inherits that constraint. Mapping it here costs nothing (no public
/// [`HdrFtr`] method ever constructs or inserts one) and is *more* faithful than dropping a
/// non-conformant file's stray `w:sectPr` to the opaque `Raw` bucket would be — this crate never
/// rejects on read.
///
/// Parses `w:hdr` and `w:ftr` identically, since `wml.xsd` gives both the same `CT_HdrFtr` type;
/// which one a given [`HdrFtr`] is is a fact about the *part* (its root element name), not this type.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = WML)]
pub struct HdrFtr {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "customXml", variant = CustomXml, ty = super::body::Unmodeled),
        child(local = "sdt", variant = StructuredDocumentTag, ty = super::body::Unmodeled),
        child(local = "p", variant = Paragraph, ty = Paragraph),
        child(local = "tbl", variant = Table, ty = Table),
        child(local = "proofErr", variant = ProofingError, ty = super::body::ProofingError),
        child(local = "permStart", variant = PermissionRangeStart, ty = super::body::PermissionRangeStart),
        child(local = "permEnd", variant = PermissionRangeEnd, ty = super::body::PermissionRangeEnd),
        child(local = "sectPr", variant = SectionProperties, ty = super::sections::SectionProperties),
        child(local = "tcPr", variant = Properties, ty = super::tables::CellProperties)
    )]
    content: Vec<BlockContent>,
}

impl HdrFtr {
    /// Builds a new, empty `local` root (`"hdr"` or `"ftr"`) holding one empty paragraph — matching
    /// what a genuinely blank header or footer part needs to be non-degenerate, the same reasoning
    /// [`super::super::Document::blank`] applies to the main document body.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str) -> Self {
        Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            empty: false,
            content: vec![BlockContent::Paragraph(Paragraph::new(interner))],
        }
    }

    /// This element's name as written (`w:hdr` or `w:ftr`).
    #[must_use]
    pub fn name(&self) -> &RawName {
        &self.name
    }

    /// How many paragraphs this header or footer holds, in document order.
    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs().count()
    }

    /// Every paragraph in document order.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        block_paragraphs(&self.content)
    }

    /// The paragraph at `path`, or `None` if the address is out of range.
    #[must_use]
    pub fn paragraph(&self, path: impl Into<BlockPath>) -> Option<&Paragraph> {
        block_paragraph(&self.content, &path.into())
    }

    /// The paragraph at `path`, mutably.
    pub fn paragraph_mut(&mut self, path: impl Into<BlockPath>) -> Option<&mut Paragraph> {
        block_paragraph_mut(&mut self.content, &path.into())
    }

    /// Inserts `paragraph` so it becomes the paragraph at `path`, shifting every paragraph at or
    /// after that position one place later. `path` must address an existing paragraph slot or the
    /// one past the last (i.e. `0..=paragraph_count()`); anything else is rejected.
    ///
    /// Returns `false`, leaving `self` untouched, if `path` is out of range.
    #[must_use]
    pub fn insert_paragraph(&mut self, path: impl Into<BlockPath>, paragraph: Paragraph) -> bool {
        let path = path.into();
        let end = self.content.len();
        block_insert_paragraph(&mut self.content, &path, paragraph, || end)
    }

    /// Appends `paragraph` as this header or footer's new last paragraph — unlike
    /// [`super::body::Body::append_paragraph`], always at the very end of `content`: `CT_HdrFtr`
    /// carries no trailing `w:sectPr` to stay ahead of.
    pub fn append_paragraph(&mut self, paragraph: Paragraph) {
        self.content.push(BlockContent::Paragraph(paragraph));
    }

    /// Removes and returns the paragraph at `path`, or `None` if the address is out of range.
    pub fn remove_paragraph(&mut self, path: impl Into<BlockPath>) -> Option<Paragraph> {
        block_remove_paragraph(&mut self.content, &path.into())
    }

    /// How many tables this header or footer holds at its own top level, in document order.
    #[must_use]
    pub fn table_count(&self) -> usize {
        self.tables().count()
    }

    /// Every top-level table in document order.
    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        block_tables(&self.content)
    }

    /// The top-level table at `index`, or `None` if there is no such table.
    #[must_use]
    pub fn table(&self, index: usize) -> Option<&Table> {
        block_table(&self.content, index)
    }

    /// [`HdrFtr::table`], mutably.
    pub fn table_mut(&mut self, index: usize) -> Option<&mut Table> {
        block_table_mut(&mut self.content, index)
    }

    /// Appends `table` as this header or footer's new last top-level table, and returns its new
    /// index — unlike [`super::body::Body::append_table`], always at the very end of `content`:
    /// `CT_HdrFtr` carries no trailing `w:sectPr` to stay ahead of.
    pub fn append_table(&mut self, table: Table) -> usize {
        let at = self.content.len();
        block_append_table(&mut self.content, table, at)
    }

    /// Removes and returns the top-level table at `index`, or `None` if there is no such table.
    pub fn remove_table(&mut self, index: usize) -> Option<Table> {
        block_remove_table(&mut self.content, index)
    }
}

/// The XML declaration every part this module writes begins with, matching `blank.rs`'s own
/// constant of the same name.
const XML_DECLARATION: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n"
);

/// The `w:` namespace `local`'s `xmlns:w` declares — matching `blank.rs`'s own constant.
const WML_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// The bytes of a brand-new header or footer part: `local` (`"hdr"` or `"ftr"`) holding one empty
/// `w:p`, with `xmlns:w` declared on the root — a freshly built [`HdrFtr`] has no ancestor to inherit
/// that declaration from, exactly the reason `Document`'s own `create_style_sheet_part` writes its
/// minimal `word/styles.xml` as a literal template rather than through [`HdrFtr::new`] and
/// [`FromXml`]/[`mjx_ooxml_core::ToXml::to_xml`]. [`super::super::Document::create_header`]/
/// `create_footer` insert these bytes, then immediately re-parse them through the normal
/// `part_tree_mut`/[`FromXml`] path — the same "the typed model only ever mutates a tree it actually
/// read" sequence that private method's own doc comment describes.
pub(crate) fn initial_bytes(local: &str) -> Vec<u8> {
    format!(
        concat!(
            "{declaration}",
            r#"<w:{local} xmlns:w="{ns}"><w:p/></w:{local}>"#,
        ),
        declaration = XML_DECLARATION,
        ns = WML_NAMESPACE,
        local = local,
    )
    .into_bytes()
}

/// Resolves which `w:headerReference` (`is_header = true`) or `w:footerReference`
/// (`is_header = false`) actually applies to `section_index`'s pages of variant `kind`, and returns
/// its `r:id`, owned — `None` if no section from `section_index` back to the document's first states
/// one. See this module's own doc comment for the ECMA-376 Part 1 prose this implements.
///
/// # Errors
/// Returns [`DocxError::SectionOutOfRange`] if `section_index` is not a valid index into `spans`, or
/// [`DocxError::Model`] if a `w:titlePg` or `w:headerReference`/`w:footerReference@type` this walk
/// reads is malformed.
pub(crate) fn resolve_reference(
    spans: &[SectionSpan],
    section_index: usize,
    kind: HeaderFooterType,
    even_and_odd_headers: bool,
    interner: &Interner,
    is_header: bool,
) -> Result<Option<String>, DocxError> {
    let current = spans
        .get(section_index)
        .ok_or(DocxError::SectionOutOfRange {
            index: section_index,
            count: spans.len(),
        })?;

    let title_page = current
        .properties
        .as_ref()
        .map(|properties| properties.title_page(interner))
        .transpose()
        .map_err(FromXmlError::from)?
        .flatten()
        .unwrap_or(false);

    // §17.10.6 / §17.10.1: a first-page or even-page query with its governing flag off is not "no
    // header" — it is exactly the default (odd) page's own query, flag and all.
    let effective_kind = match kind {
        HeaderFooterType::First if !title_page => HeaderFooterType::Default,
        HeaderFooterType::Even if !even_and_odd_headers => HeaderFooterType::Default,
        other => other,
    };

    // §17.10.5 / §17.10.2: inherit from the nearest preceding section (this one included) that
    // states a reference of the effective variant.
    for span in spans[..=section_index].iter().rev() {
        let Some(properties) = &span.properties else {
            continue;
        };
        let references: Box<dyn Iterator<Item = &HeaderFooterReference> + '_> = if is_header {
            Box::new(properties.header_references())
        } else {
            Box::new(properties.footer_references())
        };
        for reference in references {
            let reference_kind = reference.kind(interner).map_err(FromXmlError::from)?;
            if reference_kind == effective_kind {
                let rel_id = reference
                    .relationship_id(interner)
                    .map_err(FromXmlError::from)?;
                return Ok(Some(rel_id.into_owned()));
            }
        }
    }
    Ok(None)
}

/// The VML content of every `w:pict` in a header or footer part's resolved tree — resolving any
/// `mc:AlternateContent` wrapper via `mjx-mce` (non-mutating: the tree this walks is a flattened
/// *view*, never written back) and reading each surviving `w:pict` through `mjx_vml::Drawing`, the
/// same typed model MJXOFF-58 built and `mjx-pptx` already consumes for PresentationML's own legacy
/// surfaces. This is the first consumer of that model outside PowerPoint.
///
/// `w:pict` (`CT_Picture`) is not itself modeled here — MJXOFF-131 owns `w:pict` in the document
/// body, and giving it a typed home in `mjx-docx` at all is that child's call to make, not this
/// one's. This function only ever *reads*: it does not require a `w:pict` to be reachable through
/// [`super::Run`]'s own `RunInnerContent` variant, so it finds a picture regardless of how deeply an
/// `mc:AlternateContent`/`mc:Fallback` pair nests it.
///
/// `w:txbxContent` — what a VML text box's own text wraps its paragraphs in — exists only in
/// ECMA-376 Part 4 Transitional (13 of `wml.xsd`'s 14 global elements are common to Strict and
/// Transitional; `txbxContent` is the 14th, Transitional-only). A watermark or text box a header
/// carries is therefore Transitional-only markup, exactly the reasoning behind `mjx-pptx`'s own `vml`
/// feature flag — restated here because `mjx-docx` does not gate it (see `Cargo.toml`'s own comment).
///
/// # Errors
/// Returns [`DocxError::Mce`] if the part's `mc:AlternateContent` markup is malformed, or
/// [`DocxError::Vml`] if a `w:pict` this walk finds does not parse as VML.
pub(crate) fn vml_drawings_in(
    document: &mjx_ooxml_core::RawDocument,
) -> Result<Vec<mjx_vml::Drawing>, DocxError> {
    let resolved = mjx_mce::resolve(document, &mjx_mce::UnderstoodNamespaces::new())?;
    let mut drawings = Vec::new();
    collect_pict_drawings(&resolved, &document.interner, &mut drawings)?;
    Ok(drawings)
}

/// Walks `node` for `w:pict` elements, appending each one's VML content to `out`. Does not descend
/// into a `w:pict` it has already collected — a VML shape nests other VML shapes (`v:group`), never
/// another `w:pict`.
fn collect_pict_drawings(
    node: &mjx_mce::ResolvedElement<'_>,
    interner: &Interner,
    out: &mut Vec<mjx_vml::Drawing>,
) -> Result<(), DocxError> {
    let is_pict = node.name().namespace.map(|s| interner.resolve(s)) == Some(WML.transitional)
        && interner.resolve(node.name().local) == "pict";
    if is_pict {
        out.push(mjx_vml::Drawing::from_xml(node.source, interner)?);
        return Ok(());
    }
    for child in &node.children {
        if let mjx_mce::ResolvedNode::Element(element) = child {
            collect_pict_drawings(element, interner, out)?;
        }
    }
    Ok(())
}
