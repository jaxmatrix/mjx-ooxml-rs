//! The identification surface over the parts this library preserves and does not model.
//!
//! # The question this answers
//!
//! *"Does this workbook have pivot tables, and where are they?"* — with the sheet, the range, the
//! cache, and the part name, and **without** the ninety-seven complex types `sml.xsd:560–1712`
//! declares. The same for external links (which books, which sheets), for connections (which
//! sources), for query tables, for XML maps, and for revisions (present or absent, and by whom).
//!
//! It is A6's preserve-first-with-typed-surface pattern
//! (`crates/mjx-pptx/src/presentation/legacy_content.rs`) restated for Excel: `mjx-sml`'s
//! [`preserved`](mjx_sml::preserved) module reads a handful of attributes off each part's root, this
//! file resolves the relationships those attributes name into [`PartName`]s, and neither half can
//! write a byte.
//!
//! # The seam, once more
//!
//! `mjx-sml` reads the markup and **resolves nothing** — a `pivotCacheDefinition@r:id` is the string
//! the file wrote. This file is where that string becomes a part name, exactly as
//! [`crate::worksheet::tables`] is for a `tablePart@r:id`.
//!
//! # Reading never dirties, and reading is not free
//!
//! Every accessor here parses the parts it needs from their **bytes**, into a document that lives
//! for the length of the call, and drops it. Nothing is cached in the package, nothing is marked
//! dirty, and [`Workbook::save`](crate::Workbook::save) still re-emits every one of these parts
//! verbatim — which `crates/mjx-xlsx/tests/preserved_parts.rs` asserts against the bytes the fixture
//! went in with, *after an edit to a cell on the very sheet the pivot table sits on*.
//!
//! The cost is a parse per part per call. That is the right trade for a surface a caller asks once
//! at open time, and it is why [`Workbook::preserved_parts`] — the inventory — parses **nothing at
//! all**: it walks relationships only, so "what preserved parts are in here" costs no markup.
//!
//! # A malformed file is reported, never repaired and never fatal
//!
//! A pivot table whose `location/@ref` is not a range still appears in
//! [`Workbook::pivot_tables`]; [`mjx_sml::PivotTableIdentity::range`] answers `None` and
//! [`location`](mjx_sml::PivotTableIdentity::location) still holds what the file wrote. A relationship
//! that resolves to nothing yields a `None` part rather than an error, because `mjx-opc` already
//! reports a dangling reference and restating it here would be a second, drifting implementation of
//! one rule. What *is* an error is a `use="required"` attribute that is absent — see
//! [`mjx_sml::preserved`] for why substituting a default would assert something the file does not
//! say.

use mjx_ooxml_core::{Interner, RawElement};
use mjx_opc::{PartName, TargetMode};
use mjx_sml::{
    ConnectionIdentity, ExternalLinkIdentity, PivotCacheIdentity, PivotTableIdentity,
    QueryTableIdentity, RevisionHeadersIdentity, SharedWorkbookUsersIdentity, SmlError,
    WorkbookPart, XmlMapsIdentity,
};

use crate::error::XlsxError;
use crate::parts::{
    PartKind, PivotCacheParts, PivotTableParts, RevisionHeadersParts, WorksheetParts,
    REL_EXTERNAL_LINK,
};
use crate::workbook::Workbook;

/// Every part of a workbook that belongs to one of the clusters this library preserves and does not
/// model, resolved to part names.
///
/// Built by walking relationships only — **no markup is parsed** — so a caller can ask "is there
/// anything in here I should be careful with" for the cost of a graph walk. The typed accessors
/// beside it ([`Workbook::pivot_tables`] and its siblings) are what read the parts themselves.
///
/// Every field is a part [`Workbook::save`](crate::Workbook::save) re-emits byte for byte.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreservedParts {
    /// Every pivot table part, in sheet order and then in each sheet's relationship order.
    pub pivot_tables: Vec<PartName>,
    /// Every pivot table cache definition, in the workbook's relationship order.
    pub pivot_cache_definitions: Vec<PartName>,
    /// Every pivot table cache records part, in the order of the definitions that reach them.
    pub pivot_cache_records: Vec<PartName>,
    /// Every external workbook references part, in the workbook's relationship order.
    pub external_links: Vec<PartName>,
    /// `xl/connections.xml`, if the workbook relates to one.
    pub connections: Option<PartName>,
    /// Every query table part, in sheet order and then in each sheet's relationship order.
    pub query_tables: Vec<PartName>,
    /// The cell metadata part, if the workbook relates to one.
    pub metadata: Option<PartName>,
    /// The volatile dependencies part, if the workbook relates to one.
    pub volatile_dependencies: Option<PartName>,
    /// The Custom XML Mappings part (`xl/xmlMaps.xml`), if the workbook relates to one.
    pub custom_xml_mappings: Option<PartName>,
    /// Every Single Cell Table Definitions part, in sheet order — at most one per worksheet.
    pub single_cell_table_definitions: Vec<PartName>,
    /// The shared-workbook revision headers part, if the workbook relates to one. **Its presence is
    /// what says the workbook is in shared mode**; see [`Workbook::revision_state`].
    pub revision_headers: Option<PartName>,
    /// Every revision log part, in the headers part's relationship order.
    pub revision_logs: Vec<PartName>,
    /// The shared-workbook user data part, if the workbook relates to one.
    pub shared_workbook_user_data: Option<PartName>,
    /// Every Custom Property part, in sheet order — opaque bytes of a content type §12.3.5 leaves
    /// entirely to the producer.
    pub custom_properties: Vec<PartName>,
}

impl PreservedParts {
    /// Every part named above, paired with its kind, so a caller can sweep them all.
    ///
    /// The order is the field order, which is the order the clusters are documented in. What this
    /// exists for is the assertion that matters: *every one of these parts comes back byte for
    /// byte*, which is a statement about the whole list rather than about any one field.
    #[must_use]
    pub fn all(&self) -> Vec<(PartKind, PartName)> {
        let mut all = Vec::new();
        let mut push_many = |kind: PartKind, parts: &[PartName]| {
            all.extend(parts.iter().map(|part| (kind, part.clone())));
        };
        push_many(PartKind::PivotTable, &self.pivot_tables);
        push_many(
            PartKind::PivotCacheDefinition,
            &self.pivot_cache_definitions,
        );
        push_many(PartKind::PivotCacheRecords, &self.pivot_cache_records);
        push_many(PartKind::ExternalLink, &self.external_links);
        push_many(PartKind::QueryTable, &self.query_tables);
        push_many(
            PartKind::SingleCellTableDefinitions,
            &self.single_cell_table_definitions,
        );
        push_many(PartKind::RevisionLog, &self.revision_logs);
        push_many(PartKind::CustomProperty, &self.custom_properties);
        for (kind, part) in [
            (PartKind::Connections, &self.connections),
            (PartKind::Metadata, &self.metadata),
            (PartKind::VolatileDependencies, &self.volatile_dependencies),
            (PartKind::CustomXmlMappings, &self.custom_xml_mappings),
            (PartKind::RevisionHeaders, &self.revision_headers),
            (
                PartKind::SharedWorkbookUserData,
                &self.shared_workbook_user_data,
            ),
        ] {
            if let Some(part) = part {
                all.push((kind, part.clone()));
            }
        }
        all
    }

    /// Whether the workbook carries no preserved-cluster part at all — the common case, and the one
    /// `tests/fixtures/sample.xlsx` is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.all().is_empty()
    }
}

/// One pivot table, resolved to its parts and identified.
///
/// Owned, borrowing nothing — the shape [`crate::SheetTable`] and [`crate::DefinedNameEntry`] take,
/// and the shape a binding can project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPivotTable {
    /// The pivot table part — `/xl/pivotTables/pivotTable1.xml` in everything a real producer
    /// writes, though nothing requires that spelling.
    pub part: PartName,
    /// The index of the tab it sits on, into [`Workbook::sheets`](crate::Workbook::sheets).
    pub sheet_index: usize,
    /// That tab's name, as `x:sheets` gives it.
    pub sheet_name: String,
    /// The worksheet part it hangs off.
    pub sheet_part: PartName,
    /// The `.rels` identifier the sheet reached it through.
    pub relationship_id: String,
    /// What the part's own markup says: its name, its cache id, and its location.
    pub identity: PivotTableIdentity,
    /// The cache definition part the table's own `.rels` reaches, when it has one.
    ///
    /// §12.3.11 requires the relationship; a table without it is a defect in the file, reported as
    /// `None` rather than as an error, because `mjx-opc` already faults a dangling reference and
    /// this library does not repair a missing one.
    pub cache_definition_part: Option<PartName>,
    /// The cache records part that cache definition reaches, when there is one. Absent for a cache
    /// saved with `saveData="0"`.
    pub cache_records_part: Option<PartName>,
    /// What the cache definition says about where its data came from, when the cache is resolvable.
    pub cache: Option<PivotCacheIdentity>,
}

/// One external workbook reference, resolved to its part and identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookExternalLink {
    /// The external-link part.
    pub part: PartName,
    /// The workbook's `externalReference@r:id` that names it, when the markup names it at all.
    ///
    /// `None` for a link part the workbook relates to but `x:externalReferences` does not list —
    /// which is a file that has quietly lost the index its `[1]Sheet1!A1` formulas resolve through.
    pub relationship_id: Option<String>,
    /// The **one-based** index a formula's `[n]` refers to this link by, when
    /// `x:externalReferences` lists it. That list's document order is the numbering, per §18.14.
    pub reference_index: Option<usize>,
    /// What the part's own markup says: a workbook, a DDE conversation, an OLE link, or nothing.
    pub identity: ExternalLinkIdentity,
    /// The `Target` of the **external** relationship the link's own `.rels` declares — the URI of
    /// the other workbook, as the file wrote it.
    ///
    /// An untrusted string: never resolved, never opened, and not required to exist or to be a
    /// well-formed URI. `None` when the link carries no such relationship (a DDE link need not).
    pub target: Option<String>,
}

/// One data connection, from the workbook's connections part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookConnection {
    /// `xl/connections.xml` — the part every connection in the workbook lives in.
    pub part: PartName,
    /// What the `x:connection` element says.
    pub identity: ConnectionIdentity,
}

/// One query table, resolved to its part and identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetQueryTable {
    /// The query table part.
    pub part: PartName,
    /// The index of the tab it sits on, into [`Workbook::sheets`](crate::Workbook::sheets).
    pub sheet_index: usize,
    /// That tab's name.
    pub sheet_name: String,
    /// The worksheet part it hangs off.
    pub sheet_part: PartName,
    /// The `.rels` identifier the sheet reached it through.
    pub relationship_id: String,
    /// What the part's own markup says.
    pub identity: QueryTableIdentity,
}

/// The Custom XML Mappings part, resolved and identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookXmlMaps {
    /// `xl/xmlMaps.xml`.
    pub part: PartName,
    /// What the part's own markup says: the selection namespaces, the schema identifiers, and each
    /// map's root element.
    pub identity: XmlMapsIdentity,
}

/// Whether this workbook is a shared workbook, and what its revision history says.
///
/// **This library never writes a revision.** Editing a cell appends nothing to a revision log, so a
/// workbook saved after an edit has a history that no longer describes it; the logs' bytes survive
/// untouched and their meaning is the file's. See [`mjx_sml::preserved`] and the guide's gap table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevisionState {
    /// Whether the workbook relates to a revision headers part at all — §12.3.16's own test for
    /// shared mode, and the one question worth asking before editing one.
    pub is_shared: bool,
    /// The revision headers part, if there is one.
    pub headers_part: Option<PartName>,
    /// What that part says: the shared-workbook guid, the flags, and one entry per editing session.
    pub headers: Option<RevisionHeadersIdentity>,
    /// Every revision log part the headers part relates to, in relationship order.
    pub log_parts: Vec<PartName>,
    /// The shared-workbook user data part, if there is one.
    pub user_data_part: Option<PartName>,
    /// What that part says: who is sharing the workbook.
    pub users: Option<SharedWorkbookUsersIdentity>,
}

impl Workbook {
    /// Every part of this workbook belonging to a cluster this library preserves and does not model.
    ///
    /// Walks relationships only and parses no markup — see [`PreservedParts`].
    ///
    /// # Errors
    /// [`XlsxError::ExternalTarget`] if one of these relationships is `TargetMode="External"` (none
    /// of SpreadsheetML's own parts ever is; the external-link part's edge to *another workbook* is,
    /// and lives in that part's own `.rels` rather than here), or
    /// [`XlsxError::TargetResolution`] if a target does not resolve to a valid part name.
    pub fn preserved_parts(&self) -> Result<PreservedParts, XlsxError> {
        let workbook = self.parts();
        let mut preserved = PreservedParts {
            pivot_cache_definitions: workbook.pivot_cache_definitions.clone(),
            external_links: workbook.external_links.clone(),
            connections: workbook.connections.clone(),
            metadata: workbook.metadata.clone(),
            volatile_dependencies: workbook.volatile_dependencies.clone(),
            custom_xml_mappings: workbook.custom_xml_mappings.clone(),
            revision_headers: workbook.revision_headers.clone(),
            shared_workbook_user_data: workbook.shared_workbook_user_data.clone(),
            ..PreservedParts::default()
        };

        for definition in &preserved.pivot_cache_definitions {
            if let Some(records) = PivotCacheParts::resolve(self.package(), definition)?.records {
                preserved.pivot_cache_records.push(records);
            }
        }
        if let Some(headers) = &preserved.revision_headers {
            preserved.revision_logs = RevisionHeadersParts::resolve(self.package(), headers)?.logs;
        }
        for sheet in self.sheets() {
            let Some(sheet_part) = sheet.part.as_ref() else {
                continue;
            };
            let sheet_parts = WorksheetParts::resolve(self.package(), sheet_part)?;
            preserved.pivot_tables.extend(sheet_parts.pivot_tables);
            preserved.query_tables.extend(sheet_parts.query_tables);
            preserved
                .custom_properties
                .extend(sheet_parts.custom_properties);
            if let Some(single) = sheet_parts.single_cell_table_definitions {
                preserved.single_cell_table_definitions.push(single);
            }
        }
        Ok(preserved)
    }

    /// Every pivot table in the workbook: its sheet, its range, the cache it reads, and the parts
    /// all three live in.
    ///
    /// In sheet order, and within a sheet in the order that sheet's relationships list them. Reading
    /// does not dirty the package.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if a pivot table part is missing a `use="required"` attribute or its
    /// `x:location`, [`XlsxError::Xml`] if one is not well-formed, or [`XlsxError`] if a
    /// relationship cannot be resolved. A part whose root is not an `x:pivotTableDefinition` is
    /// **skipped**, not faulted: what it is instead is `mjx-opc`'s content-type question.
    pub fn pivot_tables(&self) -> Result<Vec<SheetPivotTable>, XlsxError> {
        let mut tables = Vec::new();
        for (sheet_index, sheet) in self.sheets().iter().enumerate() {
            let Some(sheet_part) = sheet.part.clone() else {
                continue;
            };
            let sheet_name = sheet.name.clone();
            let Some(rels) = self.package().relationships_for(Some(&sheet_part)) else {
                continue;
            };
            let edges: Vec<(String, PartName)> = rels
                .by_type(PartKind::PivotTable.relationship_type())
                .map(|rel| {
                    crate::nav::resolve_target(&sheet_part, &rel.target)
                        .map(|part| (rel.id.clone(), part))
                })
                .collect::<Result<_, _>>()?;
            for (relationship_id, part) in edges {
                let Some(identity) = self.read_part(&part, |root, interner, _| {
                    PivotTableIdentity::read_root(root, interner)
                })?
                else {
                    continue;
                };
                let cache_definition_part =
                    PivotTableParts::resolve(self.package(), &part)?.cache_definition;
                let (cache_records_part, cache) = match &cache_definition_part {
                    Some(definition) => (
                        PivotCacheParts::resolve(self.package(), definition)?.records,
                        self.read_part(definition, |root, interner, prefix| {
                            PivotCacheIdentity::read_root(root, interner, prefix)
                        })?,
                    ),
                    None => (None, None),
                };
                tables.push(SheetPivotTable {
                    part,
                    sheet_index,
                    sheet_name: sheet_name.clone(),
                    sheet_part: sheet_part.clone(),
                    relationship_id,
                    identity,
                    cache_definition_part,
                    cache_records_part,
                    cache,
                });
            }
        }
        Ok(tables)
    }

    /// Every external workbook reference: the part, the book or conversation it names, and the
    /// one-based index a `[n]Sheet1!A1` formula reaches it by.
    ///
    /// The index comes from `xl/workbook.xml`'s `x:externalReferences` list, which §18.14 makes the
    /// numbering, **not** from relationship order. A link part the workbook relates to but that list
    /// never names still appears, with [`reference_index`](WorkbookExternalLink::reference_index)
    /// `None` — a file that has lost the index its formulas resolve through is a file this reports
    /// rather than renumbers.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if a link part's markup is missing a `use="required"` attribute of the arm
    /// it carries, [`XlsxError::Xml`] if one is not well-formed, or [`XlsxError`] if the workbook
    /// part cannot be read.
    pub fn external_links(&self) -> Result<Vec<WorkbookExternalLink>, XlsxError> {
        let listed = self.external_reference_ids()?;
        let mut links = Vec::new();
        for part in self.parts().external_links.clone() {
            let Some(identity) = self.read_part(&part, |root, interner, prefix| {
                ExternalLinkIdentity::read_root(root, interner, prefix)
            })?
            else {
                continue;
            };
            let relationship_id = self
                .package()
                .relationships_for(Some(self.workbook_part()))
                .and_then(|rels| {
                    rels.by_type(REL_EXTERNAL_LINK).find(|rel| {
                        crate::nav::resolve_target(self.workbook_part(), &rel.target)
                            .is_ok_and(|resolved| resolved == part)
                    })
                })
                .map(|rel| rel.id.clone());
            let reference_index = relationship_id.as_ref().and_then(|id| {
                listed
                    .iter()
                    .position(|listed| listed == id)
                    .map(|index| index + 1)
            });
            let target = self
                .package()
                .relationships_for(Some(&part))
                .and_then(|rels| {
                    rels.iter()
                        .find(|rel| rel.mode == TargetMode::External)
                        .map(|rel| rel.target.clone())
                });
            links.push(WorkbookExternalLink {
                part,
                relationship_id,
                reference_index,
                identity,
                target,
            });
        }
        Ok(links)
    }

    /// Every data connection the workbook's connections part declares, in document order.
    ///
    /// An empty vector for a workbook with no connections part — a workbook that has none and one
    /// whose part lists none are the same answer to the same question.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if a `x:connection` is missing its `use="required"` `@id`, or
    /// [`XlsxError::Xml`] if the part is not well-formed.
    pub fn connections(&self) -> Result<Vec<WorkbookConnection>, XlsxError> {
        let Some(part) = self.parts().connections.clone() else {
            return Ok(Vec::new());
        };
        let Some(identities) = self.read_part(&part, |root, interner, _| {
            ConnectionIdentity::read_root(root, interner)
        })?
        else {
            return Ok(Vec::new());
        };
        Ok(identities
            .into_iter()
            .map(|identity| WorkbookConnection {
                part: part.clone(),
                identity,
            })
            .collect())
    }

    /// Every query table in the workbook, in sheet order and then in each sheet's relationship
    /// order.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables), for query table parts.
    pub fn query_tables(&self) -> Result<Vec<SheetQueryTable>, XlsxError> {
        let mut query_tables = Vec::new();
        for (sheet_index, sheet) in self.sheets().iter().enumerate() {
            let Some(sheet_part) = sheet.part.clone() else {
                continue;
            };
            let Some(rels) = self.package().relationships_for(Some(&sheet_part)) else {
                continue;
            };
            let edges: Vec<(String, PartName)> = rels
                .by_type(PartKind::QueryTable.relationship_type())
                .map(|rel| {
                    crate::nav::resolve_target(&sheet_part, &rel.target)
                        .map(|part| (rel.id.clone(), part))
                })
                .collect::<Result<_, _>>()?;
            for (relationship_id, part) in edges {
                let Some(identity) = self.read_part(&part, |root, interner, _| {
                    QueryTableIdentity::read_root(root, interner)
                })?
                else {
                    continue;
                };
                query_tables.push(SheetQueryTable {
                    part,
                    sheet_index,
                    sheet_name: sheet.name.clone(),
                    sheet_part: sheet_part.clone(),
                    relationship_id,
                    identity,
                });
            }
        }
        Ok(query_tables)
    }

    /// The Custom XML Mappings part and what it says, or `None` when the workbook relates to none.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the part is missing a `use="required"` attribute, or
    /// [`XlsxError::Xml`] if it is not well-formed.
    pub fn xml_maps(&self) -> Result<Option<WorkbookXmlMaps>, XlsxError> {
        let Some(part) = self.parts().custom_xml_mappings.clone() else {
            return Ok(None);
        };
        let Some(identity) = self.read_part(&part, |root, interner, _| {
            XmlMapsIdentity::read_root(root, interner)
        })?
        else {
            return Ok(None);
        };
        Ok(Some(WorkbookXmlMaps { part, identity }))
    }

    /// Whether this workbook is shared, and what its revision history says.
    ///
    /// # Errors
    /// [`XlsxError::Sml`] if the headers or user data part is missing a `use="required"` attribute,
    /// [`XlsxError::Xml`] if either is not well-formed, or [`XlsxError`] if a revision log
    /// relationship cannot be resolved.
    pub fn revision_state(&self) -> Result<RevisionState, XlsxError> {
        let headers_part = self.parts().revision_headers.clone();
        let user_data_part = self.parts().shared_workbook_user_data.clone();
        let (headers, log_parts) = match &headers_part {
            Some(part) => (
                self.read_part(part, |root, interner, prefix| {
                    RevisionHeadersIdentity::read_root(root, interner, prefix)
                })?,
                RevisionHeadersParts::resolve(self.package(), part)?.logs,
            ),
            None => (None, Vec::new()),
        };
        let users = match &user_data_part {
            Some(part) => self.read_part(part, |root, interner, _| {
                SharedWorkbookUsersIdentity::read_root(root, interner)
            })?,
            None => None,
        };
        Ok(RevisionState {
            is_shared: headers_part.is_some(),
            headers_part,
            headers,
            log_parts,
            user_data_part,
            users,
        })
    }

    /// Parses `part` from its **bytes**, hands the root element, its interner and the prefix the
    /// part binds to the relationship-reference namespace to `read`, and drops the document.
    ///
    /// `Ok(None)` when the package holds no such part, or when `read` says the root is not the
    /// element it was looking for. Reading does not dirty the package: the bytes are parsed into a
    /// document that lives only for this call, so
    /// [`Workbook::save`](crate::Workbook::save) still re-emits the part verbatim.
    fn read_part<T>(
        &self,
        part: &PartName,
        read: impl FnOnce(&RawElement, &Interner, Option<&str>) -> Result<Option<T>, SmlError>,
    ) -> Result<Option<T>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let prefix = mjx_sml::relationship_prefix(&document.root, &document.interner);
        Ok(read(&document.root, &document.interner, prefix)?)
    }

    /// Every `externalReference@r:id` `xl/workbook.xml` lists, in document order — the numbering a
    /// formula's `[n]` uses.
    fn external_reference_ids(&self) -> Result<Vec<String>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(self.workbook_part()) else {
            return Ok(Vec::new());
        };
        let document = mjx_xml::fidelity::parse(bytes)?;
        let Some(model) = WorkbookPart::read_root(&document.root, &document.interner)? else {
            return Ok(Vec::new());
        };
        let prefix = model.relationship_prefix(&document.interner);
        let Some(references) = model.external_references() else {
            return Ok(Vec::new());
        };
        let mut ids = Vec::new();
        for reference in references.references() {
            // `CT_ExternalReference` declares `r:id` required, so an entry without one is a defect
            // in the file — and one `mjx-opc` reports over the parts it will write. The entry keeps
            // its *position* here, as the empty string, because the `[n]` numbering is positional
            // and dropping a row would renumber every link after it.
            ids.push(
                reference
                    .relationship_id(&document.interner, prefix)
                    .map_err(mjx_ooxml_core::FromXmlError::from)?
                    .unwrap_or_default(),
            );
        }
        Ok(ids)
    }
}
