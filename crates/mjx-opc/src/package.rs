//! The in-memory package: the ZIP container plus the parsed content-type and relationship graph.
//!
//! # Fidelity backbone
//!
//! Every ZIP entry is retained as raw decompressed bytes in [`Package::entries`], in the container's
//! original order. This is the round-trip source of truth: [`Package::save`] re-emits those entries
//! verbatim and in order, so any part we do not model is preserved exactly (decompressed-byte
//! identical). The content-type map and relationship graph are parsed *views* layered on top for
//! navigation; they are not (yet) regenerated on save.
//!
//! This is the copy-on-write foundation: today a part body is always [`PartBody::Raw`]; later phases
//! add a `Parsed` variant that is only serialized back when the part has actually been mutated.

use std::io::{Cursor, Read, Write};

use crate::content_types::{ContentTypes, CONTENT_TYPES_ZIP_NAME};
use crate::error::OpcError;
use crate::name::PartName;
use crate::rels::{rels_source, Relationships, RelationshipsPart};

/// The body of a part. Currently always raw bytes; a `Parsed` variant is added in later phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartBody {
    /// The part's decompressed bytes, retained verbatim for round-trip fidelity.
    Raw(Vec<u8>),
}

/// A single ZIP entry: its raw (relative) name and decompressed bytes, in container order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    /// The raw ZIP entry name (relative form, no leading slash).
    pub name: String,
    /// The entry's decompressed bytes.
    pub body: PartBody,
}

impl ZipEntry {
    /// The entry's decompressed bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match &self.body {
            PartBody::Raw(b) => b,
        }
    }
}

/// An OOXML package loaded fully into memory.
#[derive(Debug, Clone)]
pub struct Package {
    entries: Vec<ZipEntry>,
    content_types: ContentTypes,
    relationships: Vec<RelationshipsPart>,
}

impl Package {
    /// Opens a package from in-memory container bytes.
    ///
    /// Reads every ZIP entry into RAM (order preserved), then parses `[Content_Types].xml` and all
    /// `.rels` parts to build the navigation views.
    ///
    /// # Errors
    /// Returns [`OpcError`] if the ZIP is unreadable, `[Content_Types].xml` is missing, or a control
    /// part is malformed.
    pub fn open(bytes: &[u8]) -> Result<Self, OpcError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_owned();
            let mut data = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
            file.read_to_end(&mut data)?;
            entries.push(ZipEntry {
                name,
                body: PartBody::Raw(data),
            });
        }

        let content_types = {
            let ct = entries
                .iter()
                .find(|e| e.name == CONTENT_TYPES_ZIP_NAME)
                .ok_or_else(|| OpcError::malformed("missing [Content_Types].xml"))?;
            ContentTypes::parse(ct.bytes())?
        };

        let mut relationships = Vec::new();
        for entry in &entries {
            if let Some(source) = rels_source(&entry.name)? {
                relationships.push(RelationshipsPart {
                    source,
                    relationships: Relationships::parse(entry.bytes())?,
                });
            }
        }

        Ok(Self {
            entries,
            content_types,
            relationships,
        })
    }

    /// Serializes the package back to container bytes.
    ///
    /// Entries are written in their original order with their original bytes; only the ZIP
    /// compression encoding may differ from the source (which is why the round-trip guarantee is
    /// per-part *decompressed*-byte identity, not identical container bytes).
    ///
    /// # Errors
    /// Returns [`OpcError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, OpcError> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for entry in &self.entries {
            writer.start_file(entry.name.as_str(), options)?;
            writer.write_all(entry.bytes())?;
        }
        let cursor = writer.finish()?;
        Ok(cursor.into_inner())
    }

    /// All ZIP entries, in container order (the fidelity backbone).
    #[must_use]
    pub fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    /// The parsed content-type map.
    #[must_use]
    pub fn content_types(&self) -> &ContentTypes {
        &self.content_types
    }

    /// All parsed relationship parts (package root + per-part).
    #[must_use]
    pub fn relationships(&self) -> &[RelationshipsPart] {
        &self.relationships
    }

    /// The relationships for a given source part (`None` = the package root), if present.
    #[must_use]
    pub fn relationships_for(&self, source: Option<&PartName>) -> Option<&Relationships> {
        self.relationships
            .iter()
            .find(|r| r.source.as_ref() == source)
            .map(|r| &r.relationships)
    }

    /// Resolves the content type for a part.
    #[must_use]
    pub fn content_type_of(&self, part: &PartName) -> Option<&str> {
        self.content_types.content_type_of(part)
    }

    /// The names of all addressable parts — every ZIP entry except the special
    /// `[Content_Types].xml` item (which is not a part). Invalid names are skipped.
    pub fn part_names(&self) -> impl Iterator<Item = PartName> + '_ {
        self.entries
            .iter()
            .filter(|e| e.name != CONTENT_TYPES_ZIP_NAME)
            .filter_map(|e| PartName::from_zip_name(&e.name).ok())
    }

    /// Looks up a part's decompressed bytes by its part name.
    #[must_use]
    pub fn part_bytes(&self, part: &PartName) -> Option<&[u8]> {
        let zip_name = part.zip_name();
        self.entries
            .iter()
            .find(|e| e.name == zip_name)
            .map(ZipEntry::bytes)
    }
}
