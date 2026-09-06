//! `xl/xmlMaps.xml` — the Custom XML Mappings part (`x:MapInfo`, `CT_MapInfo`, `sml.xsd:336–376`).
//!
//! # The part §9 of the programme epic recorded as unowned
//!
//! Four complex types, and until MJXOFF-133 nothing in this workspace named the part at all — not
//! its relationship type, not its content type, not a classification. It was preserved, because
//! everything is, but no test said so and nothing could report it. This file and
//! `mjx_xlsx::PartKind::CustomXmlMappings` close that.
//!
//! # Why the mapping is identified and not modelled
//!
//! `CT_Schema` is `mixed="true"` around an `xsd:any`: an entry's content is **an XML Schema
//! document in somebody else's namespace**, inlined. Modelling it means modelling XSD, which is not
//! SpreadsheetML and not this project's. So the schema entries are reported by their identifiers
//! and their bodies stay exactly as the file wrote them — the unknown-bucket rule, applied to a slot
//! whose contents are a whole other language.
//!
//! A map's purpose is to say which XPath of that schema lands in which cell; the cell end of it is a
//! `x:tableColumn@xpath` (D15's [`TableColumn`](crate::TableColumn)) or a Single Cell Table
//! Definitions part. Neither is followed here.
//!
//! # `MapInfo`, `Schema`, `Map` — the capitals are the schema's
//!
//! Every other element in `sml.xsd` is `lowerCamelCase`. These three are not, and `@ID`, `@Name`,
//! `@RootElement`, `@SchemaID` are not either. The wire tokens are quoted exactly, as they are
//! everywhere in this project; the Rust names beside them are ordinary.

use mjx_ooxml_core::{Interner, RawElement};

use crate::error::SmlError;

use super::{is_sml, optional_text, required_text, required_unsigned, sml_children};

/// One `x:Map` of the mappings part (`CT_Map`), identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlMapIdentity {
    /// `@ID` (`use="required"`) — what a `tableColumn@mapId` and a `singleXmlCell@mapId` name.
    pub id: u32,
    /// `@Name` (`use="required"`) — what a consumer shows in its XML Source pane.
    pub name: String,
    /// `@RootElement` (`use="required"`) — the element of the mapped schema this map is rooted at.
    pub root_element: String,
    /// `@SchemaID` (`use="required"`) — the `Schema@ID` of the schema entry this map uses.
    pub schema_id: String,
    /// `DataBinding/@DataBindingName`, when the map carries a data binding at all. The binding's own
    /// body is an `xsd:any` and is preserved, not read.
    pub data_binding_name: Option<String>,
}

/// The Custom XML Mappings part (`x:MapInfo`, `CT_MapInfo`), identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlMapsIdentity {
    /// `@SelectionNamespaces` (`use="required"`) — the `xmlns:` declarations the maps' XPaths are
    /// evaluated against, as one whitespace-separated string, exactly as the file wrote it. Never
    /// split into bindings here: nothing evaluates an XPath.
    pub selection_namespaces: String,
    /// Every `x:Schema@ID`, in document order. The schema **bodies** are `xsd:any` content in
    /// another language and stay in the preserved bytes.
    pub schema_ids: Vec<String>,
    /// Every `x:Map`, in document order.
    pub maps: Vec<XmlMapIdentity>,
}

impl XmlMapsIdentity {
    /// Reads a mappings part's root element, or `Ok(None)` when `root` is not an `x:MapInfo`.
    ///
    /// # Errors
    /// [`SmlError::Model`] if a `use="required"` attribute of the part, of a schema entry or of a
    /// map is absent or will not decode.
    pub fn read_root(root: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        if !is_sml(root, interner, "MapInfo") {
            return Ok(None);
        }
        let mut schema_ids = Vec::new();
        for schema in sml_children(root, interner, "Schema") {
            schema_ids.push(required_text(&schema.attributes, interner, "ID")?);
        }
        let mut maps = Vec::new();
        for map in sml_children(root, interner, "Map") {
            let data_binding_name = match super::sml_child(map, interner, "DataBinding") {
                Some(binding) => optional_text(&binding.attributes, interner, "DataBindingName")?,
                None => None,
            };
            maps.push(XmlMapIdentity {
                id: required_unsigned(&map.attributes, interner, "ID")?,
                name: required_text(&map.attributes, interner, "Name")?,
                root_element: required_text(&map.attributes, interner, "RootElement")?,
                schema_id: required_text(&map.attributes, interner, "SchemaID")?,
                data_binding_name,
            });
        }
        Ok(Some(Self {
            selection_namespaces: required_text(&root.attributes, interner, "SelectionNamespaces")?,
            schema_ids,
            maps,
        }))
    }
}
