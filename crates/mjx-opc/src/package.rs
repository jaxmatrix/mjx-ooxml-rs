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

use mjx_ooxml_core::{
    Interner, QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode, Symbol,
};
use mjx_xml::fidelity;

use crate::content_types::{ContentTypes, CONTENT_TYPES_ZIP_NAME};
use crate::error::OpcError;
use crate::name::PartName;
use crate::rels::{
    build_rels_bytes, rels_source, rels_zip_name_for, Relationship, Relationships,
    RelationshipsPart, TargetMode,
};

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

    /// The `[Content_Types].xml` tree, transitioned to `Edited` (parsed on demand). Located by the
    /// container constant since the content-types item is not a [`PartName`].
    fn content_types_tree_mut(&mut self) -> Result<&mut RawDocument, OpcError> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.name == CONTENT_TYPES_ZIP_NAME)
            .ok_or_else(|| OpcError::malformed("missing [Content_Types].xml"))?;
        self.entry_tree_mut(idx)
    }

    /// Sets the content-type `Override` for a part, editing `[Content_Types].xml` and the parsed
    /// content-type view in tandem.
    ///
    /// No-op (the control part stays clean) if the part already resolves to `content_type` via an
    /// existing `Override`.
    ///
    /// # Errors
    /// Returns [`OpcError`] if `[Content_Types].xml` is absent or not well-formed XML.
    pub fn set_content_type_override(
        &mut self,
        part: &PartName,
        content_type: &str,
    ) -> Result<(), OpcError> {
        if self
            .content_types
            .overrides()
            .iter()
            .any(|o| &o.part_name == part && o.content_type == content_type)
        {
            return Ok(());
        }
        {
            let tree = self.content_types_tree_mut()?;
            upsert_override_element(tree, part, content_type);
        }
        self.content_types
            .upsert_override(part.clone(), content_type.to_owned());
        Ok(())
    }

    /// Registers a content-type `Default` mapping every part with `extension` to `content_type`,
    /// editing `[Content_Types].xml` and the parsed content-type view in tandem.
    ///
    /// This is how binary media parts get their content type in an Office-written package (one
    /// `<Default Extension="png" .../>` rather than an `Override` per image). Registering the
    /// `Default` before [`insert_part`](Package::insert_part) therefore leaves that call with no
    /// `Override` to add. The new rule is placed after the last existing `Default`, ahead of the
    /// `Override`s, matching Office's ordering.
    ///
    /// No-op (the control part stays clean) if the extension already maps to `content_type`.
    ///
    /// # Errors
    /// Returns [`OpcError::Malformed`] if a `Default` already maps `extension` to a *different*
    /// content type — rewriting it would silently retype every part with that extension — or if
    /// `[Content_Types].xml` is absent or not well-formed XML.
    pub fn set_content_type_default(
        &mut self,
        extension: &str,
        content_type: &str,
    ) -> Result<(), OpcError> {
        let extension = extension.to_ascii_lowercase();
        if let Some(existing) = self
            .content_types
            .defaults()
            .iter()
            .find(|d| d.extension == extension)
        {
            if existing.content_type == content_type {
                return Ok(());
            }
            return Err(OpcError::malformed(format!(
                "content-type Default for extension {extension} is already {}, not {content_type}",
                existing.content_type
            )));
        }
        {
            let tree = self.content_types_tree_mut()?;
            insert_default_element(tree, &extension, content_type);
        }
        self.content_types
            .push_default(extension, content_type.to_owned());
        Ok(())
    }

    /// Removes the content-type `Override` for a part, if present, editing `[Content_Types].xml` and
    /// the parsed view in tandem. Shared `Default` rules are untouched.
    ///
    /// No-op (the control part stays clean) if the part has no `Override`.
    ///
    /// # Errors
    /// Returns [`OpcError`] if `[Content_Types].xml` is not well-formed XML.
    pub fn remove_content_type_override(&mut self, part: &PartName) -> Result<(), OpcError> {
        if !self
            .content_types
            .overrides()
            .iter()
            .any(|o| &o.part_name == part)
        {
            return Ok(());
        }
        {
            let tree = self.content_types_tree_mut()?;
            remove_override_element(tree, part);
        }
        self.content_types.remove_override(part);
        Ok(())
    }

    /// Adds a relationship from `source` (`None` = the package root), editing (or synthesizing) the
    /// source's `.rels` part and updating the navigation view in tandem.
    ///
    /// When the source has no `.rels` part yet, a fresh one is synthesized and stored as raw bytes.
    ///
    /// # Errors
    /// Returns [`OpcError`] if an existing `.rels` part is not well-formed XML.
    pub fn add_relationship(
        &mut self,
        source: Option<&PartName>,
        rel: Relationship,
    ) -> Result<(), OpcError> {
        let rels_name = rels_zip_name_for(source);
        if let Some(idx) = self.entries.iter().position(|e| e.name == rels_name) {
            {
                let tree = self.entry_tree_mut(idx)?;
                append_relationship_element(tree, &rel);
            }
            match self
                .relationships
                .iter_mut()
                .find(|r| r.source.as_ref() == source)
            {
                Some(part) => part.relationships.push(rel),
                None => self.relationships.push(RelationshipsPart {
                    source: source.cloned(),
                    relationships: Relationships::with_one(rel),
                }),
            }
        } else {
            self.entries.push(ZipEntry {
                name: rels_name,
                body: PartBody::Raw(build_rels_bytes(std::slice::from_ref(&rel))),
            });
            self.relationships.push(RelationshipsPart {
                source: source.cloned(),
                relationships: Relationships::with_one(rel),
            });
        }
        Ok(())
    }

    /// Removes the relationship with id `id` from `source`'s `.rels` part (`None` = the package
    /// root), editing the `.rels` tree and the navigation view in tandem. Returns whether one was
    /// removed; a missing `.rels` part or unknown id is a no-op returning `false`.
    ///
    /// # Errors
    /// Returns [`OpcError`] if the `.rels` part is not well-formed XML.
    pub fn remove_relationship(
        &mut self,
        source: Option<&PartName>,
        id: &str,
    ) -> Result<bool, OpcError> {
        let rels_name = rels_zip_name_for(source);
        let Some(idx) = self.entries.iter().position(|e| e.name == rels_name) else {
            return Ok(false);
        };
        let exists = self
            .relationships
            .iter()
            .find(|r| r.source.as_ref() == source)
            .is_some_and(|r| r.relationships.by_id(id).is_some());
        if !exists {
            return Ok(false);
        }
        {
            let tree = self.entry_tree_mut(idx)?;
            remove_relationship_element(tree, id);
        }
        if let Some(part) = self
            .relationships
            .iter_mut()
            .find(|r| r.source.as_ref() == source)
        {
            part.relationships.remove_by_id(id);
        }
        Ok(true)
    }

    /// Inserts a new part with the given content bytes and content type.
    ///
    /// The bytes are stored [`Raw`](PartBody::Raw) (re-emitted verbatim). A content-type `Override`
    /// is registered only if the part does not already resolve to `content_type` via an existing
    /// `Default` or `Override`. Does not create relationships — wire those with
    /// [`Package::add_relationship`]; an inserted part is unreferenced until then.
    ///
    /// # Errors
    /// Returns [`OpcError::Malformed`] if a part with this name already exists, or an error from
    /// registering the content type.
    pub fn insert_part(
        &mut self,
        part: &PartName,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), OpcError> {
        let zip_name = part.zip_name();
        if self.entries.iter().any(|e| e.name == zip_name) {
            return Err(OpcError::malformed(format!(
                "part already exists: {}",
                part.as_str()
            )));
        }
        self.entries.push(ZipEntry {
            name: zip_name.to_owned(),
            body: PartBody::Raw(bytes),
        });
        if self.content_types.content_type_of(part) != Some(content_type) {
            self.set_content_type_override(part, content_type)?;
        }
        Ok(())
    }

    /// Removes a part, its content-type `Override` (if any), and its own outgoing `.rels` part.
    ///
    /// Shared `Default` content-type rules are left untouched. Inbound relationships *from other
    /// parts* are not scanned (a graph operation left to a later phase); no bytes are corrupted.
    ///
    /// # Errors
    /// Returns [`OpcError::UnknownPart`] if the part is absent, or an error while removing its
    /// content type.
    pub fn remove_part(&mut self, part: &PartName) -> Result<(), OpcError> {
        let zip_name = part.zip_name();
        let idx = self
            .entries
            .iter()
            .position(|e| e.name == zip_name)
            .ok_or_else(|| OpcError::unknown_part(part.as_str()))?;
        self.entries.remove(idx);
        self.remove_content_type_override(part)?;
        let rels_name = rels_zip_name_for(Some(part));
        if let Some(ridx) = self.entries.iter().position(|e| e.name == rels_name) {
            self.entries.remove(ridx);
        }
        self.relationships
            .retain(|r| r.source.as_ref() != Some(part));
        Ok(())
    }
}

/// Whether a ZIP entry name is a control part (`[Content_Types].xml` or a `.rels` part), which must
/// be edited through the dedicated content-type / relationship helpers rather than `part_tree*`.
fn is_control_part(zip_name: &str) -> bool {
    zip_name == CONTENT_TYPES_ZIP_NAME || matches!(rels_source(zip_name), Ok(Some(_)))
}

/// Appends `value` to `out`, escaping the characters significant inside a double-quoted XML
/// attribute value (`&`, `<`, `"`). The fidelity writer emits attribute bytes verbatim, so any value
/// injected from a Rust string must be escaped here to stay well-formed.
pub(crate) fn escape_attribute_into(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

/// Escapes `value` as double-quoted-attribute bytes.
fn escape_attribute_bytes(value: &str) -> Box<[u8]> {
    let mut out = String::with_capacity(value.len());
    escape_attribute_into(value, &mut out);
    out.into_bytes().into_boxed_slice()
}

/// Builds an unprefixed attribute `name="value"` (value escaped), interning the name.
fn make_attribute(interner: &mut Interner, name: &str, value: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern(name),
            namespace: None,
        },
        value: escape_attribute_bytes(value),
        quote: QuoteStyle::Double,
    }
}

/// Builds a self-closing, unprefixed element `<local attrs/>` in `namespace`.
fn make_empty_element(
    interner: &mut Interner,
    namespace: Option<Symbol>,
    local: &str,
    attributes: Vec<RawAttribute>,
) -> RawElement {
    RawElement {
        name: RawName {
            prefix: None,
            local: interner.intern(local),
            namespace,
        },
        attributes,
        children: Vec::new(),
        empty: true,
    }
}

/// Removes any `<Override>` child of the content-types root whose `PartName` equals `part`.
fn remove_override_element(tree: &mut RawDocument, part: &PartName) {
    let target = escape_attribute_bytes(part.as_str());
    let RawDocument { interner, root, .. } = tree;
    root.children.retain(|child| {
        let RawNode::Element(el) = child else {
            return true;
        };
        let is_override = interner.resolve(el.name.local) == "Override";
        let matches_part = el.attributes.iter().any(|a| {
            interner.resolve(a.name.local) == "PartName" && a.value.as_ref() == target.as_ref()
        });
        !(is_override && matches_part)
    });
}

/// Inserts (replacing any existing) the `<Override>` for `part`, setting its content type.
fn upsert_override_element(tree: &mut RawDocument, part: &PartName, content_type: &str) {
    remove_override_element(tree, part);
    let namespace = tree.root.name.namespace;
    let RawDocument { interner, root, .. } = tree;
    let attributes = vec![
        make_attribute(interner, "PartName", part.as_str()),
        make_attribute(interner, "ContentType", content_type),
    ];
    let element = make_empty_element(interner, namespace, "Override", attributes);
    root.empty = false;
    root.children.push(RawNode::Element(element));
}

/// Inserts a `<Default Extension=".." ContentType=".."/>` after the last existing `<Default>` child
/// of the content-types root (or first, when there is none), so `Default`s stay ahead of `Override`s
/// as Office writes them. The caller has checked that no rule for `extension` exists.
fn insert_default_element(tree: &mut RawDocument, extension: &str, content_type: &str) {
    let namespace = tree.root.name.namespace;
    let RawDocument { interner, root, .. } = tree;
    let at = root
        .children
        .iter()
        .rposition(|child| match child {
            RawNode::Element(el) => interner.resolve(el.name.local) == "Default",
            _ => false,
        })
        .map_or(0, |idx| idx + 1);
    let attributes = vec![
        make_attribute(interner, "Extension", extension),
        make_attribute(interner, "ContentType", content_type),
    ];
    let element = make_empty_element(interner, namespace, "Default", attributes);
    root.empty = false;
    root.children.insert(at, RawNode::Element(element));
}

/// Appends a `<Relationship>` child to the relationships root.
fn append_relationship_element(tree: &mut RawDocument, rel: &Relationship) {
    let namespace = tree.root.name.namespace;
    let RawDocument { interner, root, .. } = tree;
    let mut attributes = vec![
        make_attribute(interner, "Id", &rel.id),
        make_attribute(interner, "Type", &rel.rel_type),
        make_attribute(interner, "Target", &rel.target),
    ];
    if rel.mode == TargetMode::External {
        attributes.push(make_attribute(interner, "TargetMode", "External"));
    }
    let element = make_empty_element(interner, namespace, "Relationship", attributes);
    root.empty = false;
    root.children.push(RawNode::Element(element));
}

/// Removes any `<Relationship>` child of the relationships root whose `Id` equals `id`.
fn remove_relationship_element(tree: &mut RawDocument, id: &str) {
    let target = escape_attribute_bytes(id);
    let RawDocument { interner, root, .. } = tree;
    root.children.retain(|child| {
        let RawNode::Element(el) = child else {
            return true;
        };
        let is_rel = interner.resolve(el.name.local) == "Relationship";
        let matches_id = el
            .attributes
            .iter()
            .any(|a| interner.resolve(a.name.local) == "Id" && a.value.as_ref() == target.as_ref());
        !(is_rel && matches_id)
    });
}
