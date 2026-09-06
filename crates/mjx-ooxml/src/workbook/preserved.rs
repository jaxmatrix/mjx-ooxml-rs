//! The **half of `sml.xsd` this project deliberately does not model**, reported rather than typed:
//! pivot tables and their caches, external links, connections, query tables, XML maps, cell
//! metadata, volatile dependencies, single-cell tables and the shared-workbook revision parts — 184
//! of `sml.xsd`'s 367 complex types between them.
//!
//! Every part named here is preserved **verbatim** by the copy-on-write layer and round-trips byte
//! for byte. What this file adds is the ability to *notice* them: a caller that rewrites a workbook
//! without knowing it carries a pivot cache is a caller about to surprise somebody.
//!
//! # Why these types restate `mjx-xlsx`'s
//!
//! Each `mjx_xlsx` report carries [`mjx_opc::PartName`] handles and an `mjx-sml` *identity* record
//! of its own — thirteen further types between them, several with enum payloads. The facade names
//! parts as `&str`, as it does everywhere, and flattens each identity to the fields a caller acts
//! on. The full identity records stay reachable through [`Workbook::workbook_mut`], and
//! [*Fidelity and the part graph*](mjx_xlsx::guide::fidelity_and_the_part_graph) has the
//! cluster-by-cluster table.
//!
//! # These reports resolve from relationships, not from markup
//!
//! [`Workbook::preserved_parts`] walks the part graph and parses **nothing**; the six reports beside
//! it read only each part's root element. Asking whether a workbook has pivot tables is therefore
//! cheap on a workbook of any size — which is the whole reason to ask before opening a sheet.

use mjx_xlsx::PartKind;

use crate::error::Error;
use crate::index::count;

use super::Workbook;

/// Every part this project preserves rather than models, by cluster.
///
/// Part names, in the order the part graph reaches them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreservedPartsSummary {
    /// `x:pivotTableDefinition` parts.
    pub pivot_tables: Vec<String>,
    /// `x:pivotCacheDefinition` parts.
    pub pivot_cache_definitions: Vec<String>,
    /// `x:pivotCacheRecords` parts.
    pub pivot_cache_records: Vec<String>,
    /// `x:externalLink` parts.
    pub external_links: Vec<String>,
    /// The `x:connections` part, if there is one.
    pub connections: Option<String>,
    /// `x:queryTable` parts.
    pub query_tables: Vec<String>,
    /// The `x:metadata` part, if there is one.
    pub metadata: Option<String>,
    /// The `x:volTypes` part, if there is one.
    pub volatile_dependencies: Option<String>,
    /// The `x:MapInfo` part, if there is one.
    pub custom_xml_mappings: Option<String>,
    /// `x:singleXmlCell` table-definition parts.
    pub single_cell_table_definitions: Vec<String>,
    /// The `x:headers` revision-headers part, if there is one.
    pub revision_headers: Option<String>,
    /// `x:revisions` revision-log parts.
    pub revision_logs: Vec<String>,
    /// The shared-workbook user-data part, if there is one.
    pub shared_workbook_user_data: Option<String>,
    /// Custom Property parts — user-defined data hung off a worksheet, of any content the producing
    /// application cared to write.
    pub custom_properties: Vec<String>,
}

impl PreservedPartsSummary {
    /// Every preserved part with the kind it is, flattened.
    #[must_use]
    pub fn all(&self) -> Vec<(PartKind, String)> {
        let mut all = Vec::new();
        let mut many = |kind: PartKind, parts: &[String]| {
            all.extend(parts.iter().map(|part| (kind, part.clone())));
        };
        many(PartKind::PivotTable, &self.pivot_tables);
        many(
            PartKind::PivotCacheDefinition,
            &self.pivot_cache_definitions,
        );
        many(PartKind::PivotCacheRecords, &self.pivot_cache_records);
        many(PartKind::ExternalLink, &self.external_links);
        many(PartKind::QueryTable, &self.query_tables);
        many(
            PartKind::SingleCellTableDefinitions,
            &self.single_cell_table_definitions,
        );
        many(PartKind::RevisionLog, &self.revision_logs);
        many(PartKind::CustomProperty, &self.custom_properties);
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

    /// Whether the workbook carries none of these at all — the common case, and the one a caller
    /// checks before deciding whether any of the rest matters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.all().is_empty()
    }
}

/// One pivot table, resolved to its part and its cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPivotTableInfo {
    /// The `x:pivotTableDefinition` part.
    pub part: String,
    /// The index of the tab the table sits on.
    pub sheet: u32,
    /// That tab's name.
    pub sheet_name: String,
    /// That tab's own part.
    pub sheet_part: String,
    /// The `r:id` the sheet reached the table through.
    pub relationship_id: String,
    /// `@name` — what a consumer shows and what a `GETPIVOTDATA` formula names.
    pub name: String,
    /// `@cacheId` — the workbook-level cache this table reads.
    pub cache_id: u32,
    /// `@dataCaption` — the heading shown over the data area.
    pub data_caption: String,
    /// `x:location/@ref`, **exactly as the file wrote it** — text rather than a range, because a
    /// producer may write one this project's address parser does not accept and that is a fact about
    /// the file.
    pub location: String,
    /// The cache-definition part `@cacheId` resolves to, if the workbook declares one.
    pub cache_definition_part: Option<String>,
    /// The cache-records part that definition names, if there is one.
    pub cache_records_part: Option<String>,
    /// `pivotCacheDefinition@recordCount` — the producer's cached number of records, never derived
    /// or corrected here.
    pub cache_record_count: Option<u32>,
    /// `pivotCacheDefinition@refreshedBy` — who last refreshed the cache.
    pub cache_refreshed_by: Option<String>,
}

/// One external workbook reference, resolved to its part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookExternalLinkInfo {
    /// The `x:externalLink` part.
    pub part: String,
    /// The `r:id` `xl/workbook.xml` reached it through, when it reached it through one.
    pub relationship_id: Option<String>,
    /// The position in `x:externalReferences` this link is, when it is listed there.
    pub reference_index: Option<u32>,
    /// The relationship target of the linked workbook — a path or a URL, **carried verbatim** and
    /// never resolved or fetched. `None` for a link that is not a workbook link.
    pub target: Option<String>,
    /// Which of `CT_ExternalLink`'s shapes this is: `"workbook"`, `"dde"`, `"oleObject"`, or
    /// `"none"` for a part carrying none of the three, which the complex type permits.
    pub kind: &'static str,
    /// The names of the linked workbook's sheets, as this file cached them. Empty for the other two
    /// kinds.
    pub sheet_names: Vec<String>,
}

/// One data connection, resolved to the `x:connections` part it is declared in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookConnectionInfo {
    /// The `x:connections` part.
    pub part: String,
    /// `@id` — what a query table's `@connectionId` names.
    pub id: u32,
    /// `@name`.
    pub name: Option<String>,
    /// `@description`.
    pub description: Option<String>,
    /// `@sourceFile` — carried verbatim, never resolved or opened.
    pub source_file: Option<String>,
    /// `@odcFile`, on the same terms.
    pub odc_file: Option<String>,
}

/// One query table, resolved to its part and the sheet it feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetQueryTableInfo {
    /// The `x:queryTable` part.
    pub part: String,
    /// The index of the tab the query table sits on.
    pub sheet: u32,
    /// That tab's name.
    pub sheet_name: String,
    /// That tab's own part.
    pub sheet_part: String,
    /// The `r:id` the sheet reached it through.
    pub relationship_id: String,
    /// `@name`.
    pub name: String,
    /// `@connectionId` — what a [`WorkbookConnectionInfo::id`] matches.
    pub connection_id: u32,
    /// `@refreshOnLoad`.
    pub refresh_on_load: bool,
}

/// The workbook's XML maps — how a custom XML schema is mapped into cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookXmlMapsInfo {
    /// The `x:MapInfo` part.
    pub part: String,
    /// `@SelectionNamespaces` — the namespace declarations the XPaths below are written against.
    pub selection_namespaces: String,
    /// The schema ids declared in the part.
    pub schema_ids: Vec<String>,
    /// The maps themselves.
    pub maps: Vec<XmlMapInfo>,
}

/// One `x:Map` of an [`WorkbookXmlMapsInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlMapInfo {
    /// `@ID`.
    pub id: u32,
    /// `@Name`.
    pub name: String,
    /// `@RootElement` — the element of the mapped schema this map is rooted at.
    pub root_element: String,
    /// `@SchemaID` — which of [`WorkbookXmlMapsInfo::schema_ids`] this map uses.
    pub schema_id: String,
}

/// What the workbook says about shared-workbook change tracking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkbookRevisionState {
    /// Whether `xl/workbook.xml` declares the workbook shared.
    pub is_shared: bool,
    /// The `x:headers` part, if there is one.
    pub headers_part: Option<String>,
    /// `headers@guid`.
    pub guid: Option<String>,
    /// The revision-log parts, in the order the headers name them.
    pub log_parts: Vec<String>,
    /// The shared-workbook user-data part, if there is one.
    pub user_data_part: Option<String>,
    /// The recorded editing sessions.
    pub sessions: Vec<RevisionSessionInfo>,
    /// The recorded users.
    pub users: Vec<SharedWorkbookUserInfo>,
}

/// One recorded editing session of a shared workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSessionInfo {
    /// `@guid` — the session's own identifier.
    pub guid: String,
    /// `@dateTime`, **exactly as the file wrote it**. Text rather than a timestamp: this library
    /// never reads a clock, and parsing one producer's spelling into another would be a repair.
    pub date_time: String,
    /// `@userName`.
    pub user_name: String,
}

/// One recorded user of a shared workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWorkbookUserInfo {
    /// `@id`.
    pub id: i32,
    /// `@name`.
    pub name: String,
    /// `@dateTime`, on [`RevisionSessionInfo::date_time`]'s own terms.
    pub date_time: String,
}

impl Workbook {
    /// Every part this project preserves rather than models.
    ///
    /// Resolved from the part graph alone — **no markup is parsed** — so this is cheap on a workbook
    /// of any size.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if the package's
    /// relationships cannot be read.
    pub fn preserved_parts(&self) -> Result<PreservedPartsSummary, Error> {
        let parts = self.workbook.preserved_parts()?;
        Ok(PreservedPartsSummary {
            pivot_tables: names(&parts.pivot_tables),
            pivot_cache_definitions: names(&parts.pivot_cache_definitions),
            pivot_cache_records: names(&parts.pivot_cache_records),
            external_links: names(&parts.external_links),
            connections: name(parts.connections.as_ref()),
            query_tables: names(&parts.query_tables),
            metadata: name(parts.metadata.as_ref()),
            volatile_dependencies: name(parts.volatile_dependencies.as_ref()),
            custom_xml_mappings: name(parts.custom_xml_mappings.as_ref()),
            single_cell_table_definitions: names(&parts.single_cell_table_definitions),
            revision_headers: name(parts.revision_headers.as_ref()),
            revision_logs: names(&parts.revision_logs),
            shared_workbook_user_data: name(parts.shared_workbook_user_data.as_ref()),
            custom_properties: names(&parts.custom_properties),
        })
    }

    /// Every pivot table in the workbook, sheet by sheet, with its cache resolved.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a pivot part's root
    /// element cannot be read.
    pub fn pivot_tables(&self) -> Result<Vec<SheetPivotTableInfo>, Error> {
        Ok(self
            .workbook
            .pivot_tables()?
            .into_iter()
            .map(|table| SheetPivotTableInfo {
                part: table.part.as_str().to_owned(),
                sheet: count(table.sheet_index),
                sheet_name: table.sheet_name,
                sheet_part: table.sheet_part.as_str().to_owned(),
                relationship_id: table.relationship_id,
                name: table.identity.name,
                cache_id: table.identity.cache_id,
                data_caption: table.identity.data_caption,
                location: table.identity.location,
                cache_definition_part: name(table.cache_definition_part.as_ref()),
                cache_records_part: name(table.cache_records_part.as_ref()),
                cache_record_count: table.cache.as_ref().and_then(|cache| cache.record_count),
                cache_refreshed_by: table
                    .cache
                    .as_ref()
                    .and_then(|cache| cache.refreshed_by.clone()),
            })
            .collect())
    }

    /// Every external workbook reference, with the target it names carried verbatim.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables).
    pub fn external_links(&self) -> Result<Vec<WorkbookExternalLinkInfo>, Error> {
        use mjx_sml::ExternalLinkTarget;
        Ok(self
            .workbook
            .external_links()?
            .into_iter()
            .map(|link| {
                let (kind, sheet_names) = match &link.identity.target {
                    ExternalLinkTarget::Workbook { sheet_names, .. } => {
                        ("workbook", sheet_names.clone())
                    }
                    ExternalLinkTarget::DynamicDataExchange { .. } => ("dde", Vec::new()),
                    ExternalLinkTarget::OleObject { .. } => ("oleObject", Vec::new()),
                    // `CT_ExternalLink` permits a part carrying none of the three. Reported as
                    // itself rather than folded into one of the others, which would be a repair.
                    ExternalLinkTarget::Nothing => ("none", Vec::new()),
                };
                WorkbookExternalLinkInfo {
                    part: link.part.as_str().to_owned(),
                    relationship_id: link.relationship_id,
                    reference_index: link.reference_index.map(count),
                    target: link.target,
                    kind,
                    sheet_names,
                }
            })
            .collect())
    }

    /// Every data connection the workbook declares.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables).
    pub fn connections(&self) -> Result<Vec<WorkbookConnectionInfo>, Error> {
        Ok(self
            .workbook
            .connections()?
            .into_iter()
            .map(|connection| WorkbookConnectionInfo {
                part: connection.part.as_str().to_owned(),
                id: connection.identity.id,
                name: connection.identity.name,
                description: connection.identity.description,
                source_file: connection.identity.source_file,
                odc_file: connection.identity.odc_file,
            })
            .collect())
    }

    /// Every query table, sheet by sheet.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables).
    pub fn query_tables(&self) -> Result<Vec<SheetQueryTableInfo>, Error> {
        Ok(self
            .workbook
            .query_tables()?
            .into_iter()
            .map(|table| SheetQueryTableInfo {
                part: table.part.as_str().to_owned(),
                sheet: count(table.sheet_index),
                sheet_name: table.sheet_name,
                sheet_part: table.sheet_part.as_str().to_owned(),
                relationship_id: table.relationship_id,
                name: table.identity.name,
                connection_id: table.identity.connection_id,
                refresh_on_load: table.identity.refresh_on_load,
            })
            .collect())
    }

    /// The workbook's XML maps, or `None` when it declares none.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables).
    pub fn xml_maps(&self) -> Result<Option<WorkbookXmlMapsInfo>, Error> {
        Ok(self.workbook.xml_maps()?.map(|maps| WorkbookXmlMapsInfo {
            part: maps.part.as_str().to_owned(),
            selection_namespaces: maps.identity.selection_namespaces,
            schema_ids: maps.identity.schema_ids,
            maps: maps
                .identity
                .maps
                .into_iter()
                .map(|map| XmlMapInfo {
                    id: map.id,
                    name: map.name,
                    root_element: map.root_element,
                    schema_id: map.schema_id,
                })
                .collect(),
        }))
    }

    /// What the workbook says about shared-workbook change tracking.
    ///
    /// # Errors
    /// As [`pivot_tables`](Self::pivot_tables).
    pub fn revision_state(&self) -> Result<WorkbookRevisionState, Error> {
        let state = self.workbook.revision_state()?;
        Ok(WorkbookRevisionState {
            is_shared: state.is_shared,
            headers_part: name(state.headers_part.as_ref()),
            guid: state.headers.as_ref().map(|headers| headers.guid.clone()),
            log_parts: names(&state.log_parts),
            user_data_part: name(state.user_data_part.as_ref()),
            sessions: state
                .headers
                .map(|headers| {
                    headers
                        .sessions
                        .into_iter()
                        .map(|session| RevisionSessionInfo {
                            guid: session.guid,
                            date_time: session.date_time,
                            user_name: session.user_name,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            users: state
                .users
                .map(|users| {
                    users
                        .users
                        .into_iter()
                        .map(|user| SharedWorkbookUserInfo {
                            id: user.id,
                            name: user.name,
                            date_time: user.date_time,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

/// A list of part names as the facade states them.
fn names(parts: &[mjx_xlsx::PartName]) -> Vec<String> {
    parts.iter().map(|part| part.as_str().to_owned()).collect()
}

/// One optional part name as the facade states it.
fn name(part: Option<&mjx_xlsx::PartName>) -> Option<String> {
    part.map(|part| part.as_str().to_owned())
}
