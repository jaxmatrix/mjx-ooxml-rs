//! The in-memory package: the ZIP container plus the parsed content-type and relationship graph.
//!
//! # Fidelity backbone
//!
//! Every ZIP entry is retained in [`Package::entries`], in the container's original order. This is
//! the round-trip source of truth: an untouched part re-emits verbatim (decompressed-byte
//! identical), so any part we do not model is preserved exactly.
//!
//! # Copy-on-write parts
//!
//! A part body ([`PartBody`]) is in one of three states. Freshly opened it is [`Raw`](PartBody::Raw)
//! bytes. Reading it as a fidelity tree ([`Package::part_tree`]) parses it into
//! [`Parsed`](PartBody::Parsed) — the tree is cached for later reads while the **original bytes are
//! retained** so [`Package::save`] still re-emits them verbatim (reading never disturbs a part).
//! Mutating it ([`Package::part_tree_mut`]) moves it to [`Edited`](PartBody::Edited) — the stale
//! original bytes are dropped and `save` re-serializes the tree with the byte-preserving fidelity
//! writer. Content types and relationships are edited only through the dedicated helpers, which keep
//! the control parts' trees and the parsed navigation views in lock-step.

use std::io::{Cursor, Read, Write};
use std::mem;

use mjx_ooxml_core::RawDocument;
use mjx_xml::fidelity;

use crate::content_types::{ContentTypes, CONTENT_TYPES_ZIP_NAME};
use crate::error::OpcError;
use crate::name::PartName;
use crate::rels::{rels_source, Relationships, RelationshipsPart};

/// The body of a part, in one of three copy-on-write states.
#[derive(Debug)]
pub enum PartBody {
    /// Untouched decompressed bytes, fresh from the container; re-emitted verbatim.
    Raw(Vec<u8>),
    /// Read as a fidelity tree but not mutated: the original bytes are retained (and re-emitted
    /// verbatim on save), with the parsed tree cached alongside for repeated reads.
    Parsed {
        /// The original decompressed bytes, re-emitted verbatim by [`Package::save`].
        original: Vec<u8>,
        /// The cached fidelity tree.
        tree: RawDocument,
    },
    /// Mutated: only the tree remains (the now-stale original bytes are dropped). [`Package::save`]
    /// re-serializes it with the byte-preserving fidelity writer.
    Edited(RawDocument),
}

/// A single ZIP entry: its raw (relative) name and its body, in container order.
#[derive(Debug)]
pub struct ZipEntry {
    /// The raw ZIP entry name (relative form, no leading slash).
    pub name: String,
    /// The entry's body.
    pub body: PartBody,
}

impl ZipEntry {
    /// The entry's decompressed bytes, if they are materialized.
    ///
    /// Returns `Some` for a [`Raw`](PartBody::Raw) or [`Parsed`](PartBody::Parsed) body (the original
    /// bytes), and `None` for an [`Edited`](PartBody::Edited) body — a dirty part has no stored
    /// bytes; inspect it through [`Package::part_tree`] or serialize it via [`Package::save`].
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        match &self.body {
            PartBody::Raw(b) => Some(b),
            PartBody::Parsed { original, .. } => Some(original),
            PartBody::Edited(_) => None,
        }
    }
}

/// An OOXML package loaded fully into memory.
#[derive(Debug)]
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
            let bytes = ct.bytes().ok_or_else(|| {
                OpcError::malformed("[Content_Types].xml has no bytes just after open")
            })?;
            ContentTypes::parse(bytes)?
        };

        let mut relationships = Vec::new();
        for entry in &entries {
            if let Some(source) = rels_source(&entry.name)? {
                let bytes = entry.bytes().ok_or_else(|| {
                    OpcError::malformed("a .rels part has no bytes just after open")
                })?;
                relationships.push(RelationshipsPart {
                    source,
                    relationships: Relationships::parse(bytes)?,
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
    /// Clean parts ([`Raw`](PartBody::Raw) / [`Parsed`](PartBody::Parsed)) are written from their
    /// original bytes; dirty parts ([`Edited`](PartBody::Edited)) are re-serialized from their tree
    /// with the byte-preserving fidelity writer. Only the ZIP compression encoding may differ from
    /// the source (which is why the round-trip guarantee is per-part *decompressed*-byte identity,
    /// not identical container bytes).
    ///
    /// # Errors
    /// Returns [`OpcError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, OpcError> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for entry in &self.entries {
            writer.start_file(entry.name.as_str(), options)?;
            match &entry.body {
                PartBody::Raw(bytes) => writer.write_all(bytes)?,
                PartBody::Parsed { original, .. } => writer.write_all(original)?,
                PartBody::Edited(tree) => {
                    let bytes = fidelity::serialize_to_vec(tree);
                    writer.write_all(&bytes)?;
                }
            }
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
    ///
    /// Returns `None` if the part is absent or has been edited (an [`Edited`](PartBody::Edited) body
    /// has no materialized bytes — read it via [`Package::part_tree`] instead).
    #[must_use]
    pub fn part_bytes(&self, part: &PartName) -> Option<&[u8]> {
        let zip_name = part.zip_name();
        self.entries
            .iter()
            .find(|e| e.name == zip_name)
            .and_then(ZipEntry::bytes)
    }

    /// Borrows a part's fidelity tree for **reading**, parsing and caching it on first access.
    ///
    /// This never dirties the part: its original bytes are retained and `save` still re-emits them
    /// verbatim. To mutate, use [`Package::part_tree_mut`].
    ///
    /// # Errors
    /// Returns [`OpcError::UnknownPart`] if the part is absent, [`OpcError::ControlPart`] if it names
    /// `[Content_Types].xml` or a `.rels` part (edit those through the dedicated helpers), or
    /// [`OpcError::Xml`] if the part is not well-formed XML.
    pub fn part_tree(&mut self, part: &PartName) -> Result<&RawDocument, OpcError> {
        let idx = self.locate_editable(part)?;
        self.entry_tree(idx)
    }

    /// Borrows a part's fidelity tree for **mutation**, marking it dirty.
    ///
    /// The part transitions to [`Edited`](PartBody::Edited): its original bytes are dropped and
    /// `save` re-serializes the (possibly mutated) tree.
    ///
    /// # Errors
    /// Same as [`Package::part_tree`].
    pub fn part_tree_mut(&mut self, part: &PartName) -> Result<&mut RawDocument, OpcError> {
        let idx = self.locate_editable(part)?;
        self.entry_tree_mut(idx)
    }

    /// Resolves a part name to its entry index, rejecting control parts and unknown parts.
    fn locate_editable(&self, part: &PartName) -> Result<usize, OpcError> {
        let zip_name = part.zip_name();
        if is_control_part(zip_name) {
            return Err(OpcError::control_part(part.as_str()));
        }
        self.entries
            .iter()
            .position(|e| e.name == zip_name)
            .ok_or_else(|| OpcError::unknown_part(part.as_str()))
    }

    /// Ensures entry `idx` is parsed and returns a shared reference to its tree **without** dirtying
    /// it (a `Raw` body becomes `Parsed`, retaining its original bytes).
    fn entry_tree(&mut self, idx: usize) -> Result<&RawDocument, OpcError> {
        let entry = &mut self.entries[idx];
        if matches!(entry.body, PartBody::Raw(_)) {
            // Take the bytes out so a parse failure can restore them without leaving a placeholder.
            let PartBody::Raw(original) = mem::replace(&mut entry.body, PartBody::Raw(Vec::new()))
            else {
                unreachable!("guarded by matches! above")
            };
            match fidelity::parse(&original) {
                Ok(tree) => entry.body = PartBody::Parsed { original, tree },
                Err(e) => {
                    entry.body = PartBody::Raw(original);
                    return Err(e.into());
                }
            }
        }
        match &entry.body {
            PartBody::Parsed { tree, .. } | PartBody::Edited(tree) => Ok(tree),
            PartBody::Raw(_) => unreachable!("just transitioned out of Raw"),
        }
    }

    /// Ensures entry `idx` is parsed, transitions it to `Edited` (dropping any original bytes), and
    /// returns a mutable reference to its tree.
    fn entry_tree_mut(&mut self, idx: usize) -> Result<&mut RawDocument, OpcError> {
        let entry = &mut self.entries[idx];
        match mem::replace(&mut entry.body, PartBody::Raw(Vec::new())) {
            PartBody::Raw(bytes) => match fidelity::parse(&bytes) {
                Ok(tree) => entry.body = PartBody::Edited(tree),
                Err(e) => {
                    entry.body = PartBody::Raw(bytes);
                    return Err(e.into());
                }
            },
            PartBody::Parsed { tree, .. } | PartBody::Edited(tree) => {
                entry.body = PartBody::Edited(tree);
            }
        }
        match &mut entry.body {
            PartBody::Edited(tree) => Ok(tree),
            _ => unreachable!("transitioned to Edited above"),
        }
    }
}

/// Whether a ZIP entry name is a control part (`[Content_Types].xml` or a `.rels` part), which must
/// be edited through the dedicated content-type / relationship helpers rather than `part_tree*`.
fn is_control_part(zip_name: &str) -> bool {
    zip_name == CONTENT_TYPES_ZIP_NAME || matches!(rels_source(zip_name), Ok(Some(_)))
}
