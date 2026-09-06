//! The pivot cluster's identity: what a pivot table is called, where it sits, and which cache it
//! draws on — **without** the ninety-seven complex types behind it.
//!
//! `sml.xsd:560–1712` is a quarter of the schema and this file is a hundred lines. That ratio is the
//! whole design: see [the module documentation](super) for why the cluster is not modelled and what
//! would have to exist first.
//!
//! # What a caller can ask, and what they cannot
//!
//! **Can:** does this workbook have pivot tables, what are they called, which sheet and range does
//! each occupy, which cache does each read, where does that cache's data come from, and which part
//! holds each of those. That is [`PivotTableIdentity`] and [`PivotCacheIdentity`], and it is enough
//! to render a placeholder, to warn a user before an edit, or to decide not to touch a range.
//!
//! **Cannot:** what any cell of the pivot table contains. That is an aggregation over cached
//! records, and computing it is the calculation model this library does not have. Nothing here
//! refreshes a cache either — that would mean re-reading the source range and rewriting the records
//! part, which is precisely the byte-for-byte preservation this whole child exists to guarantee.

use mjx_ooxml_core::{AttributeError, FromXmlError, Interner, RawElement};
use mjx_ooxml_types::spreadsheetml::PivotCacheSourceType;

use crate::address::CellRange;
use crate::error::SmlError;

use super::{
    is_sml, optional_text, optional_unsigned, relationship_id, required_text, required_unsigned,
    sml_child,
};

/// One pivot table part (`x:pivotTableDefinition`, `CT_pivotTableDefinition`), identified.
///
/// Four attributes out of forty-odd, and one child element out of nineteen. Everything else the part
/// holds — every field, every item, every format — stays in the bytes `mjx-opc` carries, and this
/// value can neither read nor write it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PivotTableIdentity {
    /// `@name` (`use="required"`) — what a consumer shows in its field list and what a
    /// `GETPIVOTDATA` formula names.
    pub name: String,
    /// `@cacheId` (`use="required"`) — the workbook-level cache this table reads, matched against
    /// `x:pivotCaches/pivotCache@cacheId` in `xl/workbook.xml`.
    pub cache_id: u32,
    /// `@dataCaption` (`use="required"`) — the heading shown over the data area.
    pub data_caption: String,
    /// `x:location/@ref` (`use="required"` on `CT_Location`) — **exactly as the file wrote it**.
    ///
    /// Text rather than a range because a producer may write one this crate's address parser does
    /// not accept, and a value it cannot parse is a fact about the file rather than an error in
    /// reading it. [`range`](Self::range) is the parsed form when there is one.
    pub location: String,
}

impl PivotTableIdentity {
    /// Reads a pivot table part's root element, or `Ok(None)` when `root` is not an
    /// `x:pivotTableDefinition`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `use="required"` attribute is absent or malformed, or if the part
    /// carries no `x:location` child — all three are statements about an untrusted file, reported
    /// and never panicked on.
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "pivotTableDefinition") {
            return Ok(None);
        }
        let location = sml_child(root, interner, "location").ok_or_else(|| {
            FromXmlError::from(AttributeError::Missing {
                attribute: "location",
            })
        })?;
        Ok(Some(Self {
            name: required_text(&root.attributes, interner, "name")?,
            cache_id: required_unsigned(&root.attributes, interner, "cacheId")?,
            data_caption: required_text(&root.attributes, interner, "dataCaption")?,
            location: required_text(&location.attributes, interner, "ref")?,
        }))
    }

    /// [`location`](Self::location) parsed as a range, or `None` when the file wrote one this crate
    /// cannot read.
    ///
    /// `None` is a **report about the file**, not a repair of it: a pivot table saying
    /// `ref="not a range"` keeps saying exactly that, its bytes are untouched, and nothing here
    /// substitutes a plausible rectangle for it.
    #[must_use]
    pub fn range(&self) -> Option<CellRange> {
        CellRange::parse(&self.location).ok()
    }
}

/// Where a pivot cache's data came from — `x:cacheSource/@type` and the one child it may carry.
///
/// The four variants are [`PivotCacheSourceType`]'s four wire tokens, and the payload is only what
/// `CT_WorksheetSource` states. **Nothing here opens any of them:** an external source names a
/// relationship this crate does not follow, and a worksheet source names a range this crate does not
/// read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PivotCacheSource {
    /// `type="worksheet"` — a range of this workbook or of the one `relationship_id` names.
    Worksheet {
        /// `worksheetSource/@sheet` — the tab the range is on, when the source states one.
        sheet: Option<String>,
        /// `worksheetSource/@ref` — the range, as the file wrote it. Never parsed here for the
        /// reason [`PivotTableIdentity::location`] gives.
        range: Option<String>,
        /// `worksheetSource/@name` — a defined name or table naming the range instead of a `@ref`.
        name: Option<String>,
        /// `worksheetSource/@r:id` — set when the source range is in *another* workbook, reached
        /// through an external-link relationship this crate resolves but never opens.
        relationship_id: Option<String>,
    },
    /// `type="external"` — an external data source, described by the connection
    /// `x:cacheSource/@connectionId` names.
    External,
    /// `type="consolidation"` — several ranges consolidated. `CT_Consolidation`'s page fields and
    /// range sets are preserved and not read.
    Consolidation,
    /// `type="scenario"` — the workbook's scenario manager.
    Scenario,
}

/// One pivot cache definition part (`x:pivotCacheDefinition`, `CT_PivotCacheDefinition`),
/// identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PivotCacheIdentity {
    /// `@r:id` — the relationship to this cache's records part, when it has one.
    ///
    /// Absent for a cache written with `saveData="0"`, which is a real shape a producer writes and
    /// not a defect: the cache describes its fields and stores none of the rows.
    pub records_relationship_id: Option<String>,
    /// `@recordCount` — how many rows the file *says* the records part holds. A count the file
    /// states, reported without being checked against anything, exactly as `sst/@count` is.
    pub record_count: Option<u32>,
    /// `@refreshedBy` — the user name a producer stamped on the last refresh.
    pub refreshed_by: Option<String>,
    /// `x:cacheSource/@connectionId`, or the schema default 0.
    pub connection_id: u32,
    /// Where the data came from.
    pub source: PivotCacheSource,
}

impl PivotCacheIdentity {
    /// Reads a pivot cache definition part's root element, or `Ok(None)` when `root` is not an
    /// `x:pivotCacheDefinition`.
    ///
    /// `reference_prefix` is the prefix the part binds to the relationship-reference namespace —
    /// the producer's choice, resolved from the root's own declarations rather than assumed to be
    /// `r`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if the part carries no `x:cacheSource`, if `cacheSource/@type` is absent
    /// or is not one of `ST_SourceType`'s four tokens, or if an attribute will not decode.
    pub fn read_root(
        root: &RawElement,
        interner: &Interner,
        reference_prefix: Option<&str>,
    ) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "pivotCacheDefinition") {
            return Ok(None);
        }
        let cache_source = sml_child(root, interner, "cacheSource").ok_or_else(|| {
            FromXmlError::from(AttributeError::Missing {
                attribute: "cacheSource",
            })
        })?;
        let type_token = required_text(&cache_source.attributes, interner, "type")?;
        let source_type = PivotCacheSourceType::from_wire(&type_token).ok_or_else(|| {
            FromXmlError::from(AttributeError::InvalidValue {
                attribute: "type",
                detail: format!("{type_token:?} is not one of ST_SourceType's four wire tokens"),
            })
        })?;
        let source = match source_type {
            PivotCacheSourceType::Worksheet => {
                let worksheet = sml_child(cache_source, interner, "worksheetSource");
                match worksheet {
                    Some(worksheet) => PivotCacheSource::Worksheet {
                        sheet: optional_text(&worksheet.attributes, interner, "sheet")?,
                        range: optional_text(&worksheet.attributes, interner, "ref")?,
                        name: optional_text(&worksheet.attributes, interner, "name")?,
                        relationship_id: relationship_id(worksheet, interner, reference_prefix)?,
                    },
                    // `CT_CacheSource`'s choice is `minOccurs="0"`, so a `type="worksheet"` with no
                    // `worksheetSource` is schema-valid and says nothing about where the data is.
                    None => PivotCacheSource::Worksheet {
                        sheet: None,
                        range: None,
                        name: None,
                        relationship_id: None,
                    },
                }
            }
            PivotCacheSourceType::External => PivotCacheSource::External,
            PivotCacheSourceType::Consolidation => PivotCacheSource::Consolidation,
            PivotCacheSourceType::Scenario => PivotCacheSource::Scenario,
        };
        Ok(Some(Self {
            records_relationship_id: relationship_id(root, interner, reference_prefix)?,
            record_count: optional_unsigned(&root.attributes, interner, "recordCount")?,
            refreshed_by: optional_text(&root.attributes, interner, "refreshedBy")?,
            connection_id: optional_unsigned(&cache_source.attributes, interner, "connectionId")?
                .unwrap_or(0),
            source,
        }))
    }
}
