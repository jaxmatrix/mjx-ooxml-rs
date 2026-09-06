//! The three ways a workbook names data that lives somewhere else: an external workbook reference,
//! a data connection, and a query table.
//!
//! All three are identified and none is modelled, for one reason each — and it is the same reason
//! wearing three hats. **Following any of them is I/O.** Resolving an external link means opening
//! another workbook; refreshing a connection means running a query against a database, a web page or
//! an OLAP cube; refreshing a query table means doing the second on behalf of the first. This
//! library opens the bytes it is handed and nothing else, so what it can honestly report is *what
//! the file says it points at*, which is what these three types carry.

use mjx_ooxml_core::{Interner, RawElement};

use crate::error::SmlError;

use super::{
    is_sml, optional_boolean, optional_text, optional_unsigned, relationship_id, required_text,
    required_unsigned, sml_child, sml_children,
};

/// What one external-link part points at — `CT_ExternalLink`'s `xsd:choice` of three, plus the
/// empty case the schema permits.
///
/// Every arm of the choice is `minOccurs="0"`, so a part carrying none of the three is
/// schema-valid; [`Nothing`](Self::Nothing) is that case reported rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalLinkTarget {
    /// `x:externalBook` — another workbook, reached through `@r:id`.
    Workbook {
        /// `externalBook/@r:id` (`use="required"`) — an **external** relationship in the link
        /// part's own `.rels`, whose `Target` is the other workbook's URI. `mjx-xlsx` resolves it;
        /// nothing opens it.
        relationship_id: Option<String>,
        /// `externalBook/sheetNames/sheetName@val`, in document order — the tabs of the other
        /// workbook, in the order a formula's `[1]!` index runs in. Cached by the producer at the
        /// last refresh, and never checked against the file they name.
        sheet_names: Vec<String>,
        /// How many `externalDefinedName` entries the link caches. The names themselves stay in the
        /// preserved bytes.
        defined_name_count: usize,
    },
    /// `x:ddeLink` — a Dynamic Data Exchange conversation with another application.
    DynamicDataExchange {
        /// `ddeLink/@ddeService` (`use="required"`).
        service: String,
        /// `ddeLink/@ddeTopic` (`use="required"`).
        topic: String,
    },
    /// `x:oleLink` — a link to an OLE object in another application.
    OleObject {
        /// `oleLink/@r:id` (`use="required"`).
        relationship_id: Option<String>,
        /// `oleLink/@progId` (`use="required"`) — the OLE program identifier, e.g. `Word.Document.8`.
        program_id: String,
    },
    /// The part carries none of the three, which `CT_ExternalLink` permits.
    Nothing,
}

/// One external workbook references part (`x:externalLink`, `CT_ExternalLink`), identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalLinkIdentity {
    /// What the link points at.
    pub target: ExternalLinkTarget,
}

impl ExternalLinkIdentity {
    /// Reads an external-link part's root element, or `Ok(None)` when `root` is not an
    /// `x:externalLink`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `use="required"` attribute of the arm the part carries is absent or
    /// will not decode.
    pub fn read_root(
        root: &RawElement,
        interner: &Interner,
        reference_prefix: Option<&str>,
    ) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "externalLink") {
            return Ok(None);
        }
        let target = if let Some(book) = sml_child(root, interner, "externalBook") {
            let sheet_names = match sml_child(book, interner, "sheetNames") {
                Some(list) => sml_children(list, interner, "sheetName")
                    .map(|entry| optional_text(&entry.attributes, interner, "val"))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    // `CT_ExternalSheetName@val` is optional; an entry without one names no sheet
                    // and is reported as the empty string rather than dropped, because dropping it
                    // would renumber every `[1]Sheet2!` index after it.
                    .map(Option::unwrap_or_default)
                    .collect(),
                None => Vec::new(),
            };
            let defined_name_count = match sml_child(book, interner, "definedNames") {
                Some(list) => sml_children(list, interner, "definedName").count(),
                None => 0,
            };
            ExternalLinkTarget::Workbook {
                relationship_id: relationship_id(book, interner, reference_prefix)?,
                sheet_names,
                defined_name_count,
            }
        } else if let Some(dde) = sml_child(root, interner, "ddeLink") {
            ExternalLinkTarget::DynamicDataExchange {
                service: required_text(&dde.attributes, interner, "ddeService")?,
                topic: required_text(&dde.attributes, interner, "ddeTopic")?,
            }
        } else if let Some(ole) = sml_child(root, interner, "oleLink") {
            ExternalLinkTarget::OleObject {
                relationship_id: relationship_id(ole, interner, reference_prefix)?,
                program_id: required_text(&ole.attributes, interner, "progId")?,
            }
        } else {
            ExternalLinkTarget::Nothing
        };
        Ok(Some(Self { target }))
    }
}

/// One `x:connection` of the connections part (`CT_Connection`), identified.
///
/// Six attributes of twenty-two, and none of the five child blocks — `dbPr`, `olapPr`, `webPr`,
/// `textPr` and `parameters` are exactly the markup a refresh would need, and a refresh is I/O.
/// **`@savePassword` and any credential the file carries are deliberately not surfaced**: reporting
/// them would make this library a place secrets are read out of, and it has no use for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionIdentity {
    /// `@id` (`use="required"`) — unique within the connections part, and what
    /// `queryTable@connectionId` and `cacheSource@connectionId` name.
    pub id: u32,
    /// `@name` — what a consumer shows in its connections dialog.
    pub name: Option<String>,
    /// `@description`.
    pub description: Option<String>,
    /// `@sourceFile` — the file the connection reads, as the file states it. An **untrusted path**:
    /// never resolved, never opened, and not required to exist.
    pub source_file: Option<String>,
    /// `@odcFile` — the Office Data Connection file, on the same terms.
    pub odc_file: Option<String>,
    /// `@type` — the source-kind code §18.13.1's table enumerates. Reported as the number the file
    /// wrote: `ST_ConnectionType` is not an enumeration in the schema, so there is no wire token to
    /// map and inventing names for the codes would be guessing.
    pub connection_type: Option<u32>,
}

impl ConnectionIdentity {
    /// Reads every `x:connection` of a connections part's root element, or `Ok(None)` when `root`
    /// is not an `x:connections`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a connection's `use="required"` `@id` is absent or malformed.
    pub fn read_root(
        root: &RawElement,
        interner: &Interner,
    ) -> Result<Option<Vec<Self>>, SmlError> {
        if !is_sml(root, interner, "connections") {
            return Ok(None);
        }
        let mut connections = Vec::new();
        for element in sml_children(root, interner, "connection") {
            connections.push(Self {
                id: required_unsigned(&element.attributes, interner, "id")?,
                name: optional_text(&element.attributes, interner, "name")?,
                description: optional_text(&element.attributes, interner, "description")?,
                source_file: optional_text(&element.attributes, interner, "sourceFile")?,
                odc_file: optional_text(&element.attributes, interner, "odcFile")?,
                connection_type: optional_unsigned(&element.attributes, interner, "type")?,
            });
        }
        Ok(Some(connections))
    }
}

/// One query table part (`x:queryTable`, `CT_QueryTable`), identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTableIdentity {
    /// `@name` (`use="required"`).
    pub name: String,
    /// `@connectionId` (`use="required"`) — the `x:connection@id` in `xl/connections.xml` that says
    /// where the rows come from.
    pub connection_id: u32,
    /// `@refreshOnLoad`, or the schema default `false`.
    ///
    /// **Reported, never obeyed.** A query table asking to be refreshed on load is asking for a
    /// network or file read this library does not perform; saying so is the whole of what happens.
    pub refresh_on_load: bool,
}

impl QueryTableIdentity {
    /// Reads a query table part's root element, or `Ok(None)` when `root` is not an `x:queryTable`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if `@name` or `@connectionId` is absent or malformed.
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "queryTable") {
            return Ok(None);
        }
        Ok(Some(Self {
            name: required_text(&root.attributes, interner, "name")?,
            connection_id: required_unsigned(&root.attributes, interner, "connectionId")?,
            refresh_on_load: optional_boolean(&root.attributes, interner, "refreshOnLoad", false)?,
        }))
    }
}
