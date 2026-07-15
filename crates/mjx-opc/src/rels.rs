//! Relationship parts (`_rels/*.rels`): the typed links between parts (and to external targets).
//!
//! Each relationship source has an associated `.rels` part. The package root's relationships live
//! in `/_rels/.rels`; a part `/dir/file.ext`'s relationships live in `/dir/_rels/file.ext.rels`.

use mjx_xml::{Event, Reader};

use crate::error::OpcError;
use crate::name::PartName;

/// Whether a relationship target is inside the package or an external URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    /// A target part inside the package.
    Internal,
    /// An external URI.
    External,
}

/// A single `<Relationship>` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The relationship id (`rId1`, …), unique within its `.rels` part.
    pub id: String,
    /// The relationship type URI.
    pub rel_type: String,
    /// The target, resolved relative to the source part (or an absolute URI when external).
    pub target: String,
    /// Whether the target is internal or external.
    pub mode: TargetMode,
}

/// The ordered set of relationships parsed from one `.rels` part.
#[derive(Debug, Clone, Default)]
pub struct Relationships {
    rels: Vec<Relationship>,
}

impl Relationships {
    /// Parses a `.rels` part from its raw bytes.
    ///
    /// # Errors
    /// Returns [`OpcError`] on malformed XML or a `Relationship` missing a required attribute.
    pub fn parse(xml: &[u8]) -> Result<Self, OpcError> {
        let mut reader = Reader::new(xml);
        let mut rels = Vec::new();
        loop {
            let event = reader.read()?;
            let element = match event {
                Event::Start(e) | Event::Empty(e) => e,
                Event::Eof => break,
                _ => continue,
            };
            if element.local() != "Relationship" {
                continue;
            }
            let id = element
                .attr("Id")
                .ok_or_else(|| OpcError::malformed("Relationship missing Id"))?
                .to_owned();
            let rel_type = element
                .attr("Type")
                .ok_or_else(|| OpcError::malformed("Relationship missing Type"))?
                .to_owned();
            let target = element
                .attr("Target")
                .ok_or_else(|| OpcError::malformed("Relationship missing Target"))?
                .to_owned();
            let mode = match element.attr("TargetMode") {
                Some("External") => TargetMode::External,
                // Absent or "Internal" both mean internal.
                _ => TargetMode::Internal,
            };
            rels.push(Relationship {
                id,
                rel_type,
                target,
                mode,
            });
        }
        Ok(Self { rels })
    }

    /// The relationships, in document order.
    pub fn iter(&self) -> std::slice::Iter<'_, Relationship> {
        self.rels.iter()
    }

    /// The number of relationships.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rels.len()
    }

    /// Whether there are no relationships.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rels.is_empty()
    }

    /// Looks up a relationship by its id.
    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&Relationship> {
        self.rels.iter().find(|r| r.id == id)
    }

    /// All relationships with the given type URI, in document order.
    pub fn by_type<'a>(&'a self, rel_type: &'a str) -> impl Iterator<Item = &'a Relationship> + 'a {
        self.rels.iter().filter(move |r| r.rel_type == rel_type)
    }

    /// Builds a set containing a single relationship (used when synthesizing a fresh `.rels` part).
    pub(crate) fn with_one(rel: Relationship) -> Self {
        Self { rels: vec![rel] }
    }

    /// Appends a relationship (view-side; the caller edits the `.rels` tree in tandem).
    pub(crate) fn push(&mut self, rel: Relationship) {
        self.rels.push(rel);
    }

    /// Removes the relationship with the given id, if any. Returns whether one was removed.
    pub(crate) fn remove_by_id(&mut self, id: &str) -> bool {
        let before = self.rels.len();
        self.rels.retain(|r| r.id != id);
        self.rels.len() != before
    }
}

/// The OPC relationships namespace, used when synthesizing a fresh `.rels` part.
pub(crate) const RELATIONSHIPS_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";

/// A `.rels` part together with the part it belongs to (`None` = the package root).
#[derive(Debug, Clone)]
pub struct RelationshipsPart {
    /// The source part these relationships belong to, or `None` for the package root.
    pub source: Option<PartName>,
    /// The parsed relationships.
    pub relationships: Relationships,
}

/// Given a ZIP entry name, returns its relationship source if the entry is a `.rels` part.
///
/// - `Some(None)` — the package-root relationships (`_rels/.rels`).
/// - `Some(Some(part))` — the relationships for `part`.
/// - `None` — the entry is not a relationship part.
pub(crate) fn rels_source(zip_name: &str) -> Result<Option<Option<PartName>>, OpcError> {
    if !zip_name.ends_with(".rels") {
        return Ok(None);
    }
    let (dir, file) = match zip_name.rfind('/') {
        Some(idx) => (&zip_name[..idx], &zip_name[idx + 1..]),
        None => ("", zip_name),
    };
    // The directory must be (or end with) `_rels`.
    let Some(parent) = dir.strip_suffix("_rels") else {
        return Ok(None);
    };
    let parent = parent.strip_suffix('/').unwrap_or(parent);
    let Some(base) = file.strip_suffix(".rels") else {
        return Ok(None);
    };
    if base.is_empty() {
        // `_rels/.rels` — the package root.
        return Ok(Some(None));
    }
    let source = if parent.is_empty() {
        format!("/{base}")
    } else {
        format!("/{parent}/{base}")
    };
    Ok(Some(Some(PartName::new(&source)?)))
}

/// The ZIP entry name of the `.rels` part for a given source (`None` = the package root).
///
/// Inverse of [`rels_source`]: `None` → `_rels/.rels`; `Some(/dir/file.ext)` →
/// `dir/_rels/file.ext.rels`.
pub(crate) fn rels_zip_name_for(source: Option<&PartName>) -> String {
    match source {
        None => "_rels/.rels".to_owned(),
        Some(part) => {
            let zip = part.zip_name();
            match zip.rfind('/') {
                Some(idx) => format!("{}/_rels/{}.rels", &zip[..idx], &zip[idx + 1..]),
                None => format!("_rels/{zip}.rels"),
            }
        }
    }
}

/// Serializes a fresh `.rels` part containing `rels`, in canonical form. Used when a source acquires
/// its first relationship (no prior bytes exist to preserve).
pub(crate) fn build_rels_bytes(rels: &[Relationship]) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(r#"<Relationships xmlns=""#);
    out.push_str(RELATIONSHIPS_NS);
    out.push_str(r#"">"#);
    for rel in rels {
        out.push_str(r#"<Relationship Id=""#);
        crate::package::escape_attribute_into(&rel.id, &mut out);
        out.push_str(r#"" Type=""#);
        crate::package::escape_attribute_into(&rel.rel_type, &mut out);
        out.push_str(r#"" Target=""#);
        crate::package::escape_attribute_into(&rel.target, &mut out);
        out.push('"');
        if rel.mode == TargetMode::External {
            out.push_str(r#" TargetMode="External""#);
        }
        out.push_str("/>");
    }
    out.push_str("</Relationships>");
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relationships_in_order() {
        let xml = br#"<?xml version="1.0"?>
            <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
              <Relationship Id="rId1" Type="urn:a" Target="ppt/presentation.xml"/>
              <Relationship Id="rId2" Type="urn:b" Target="http://x/" TargetMode="External"/>
            </Relationships>"#;
        let rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.len(), 2);
        assert_eq!(rels.by_id("rId1").unwrap().target, "ppt/presentation.xml");
        assert_eq!(rels.by_id("rId1").unwrap().mode, TargetMode::Internal);
        assert_eq!(rels.by_id("rId2").unwrap().mode, TargetMode::External);
        let types: Vec<_> = rels.iter().map(|r| r.rel_type.as_str()).collect();
        assert_eq!(types, ["urn:a", "urn:b"]);
    }

    #[test]
    fn computes_rels_source() {
        assert_eq!(rels_source("_rels/.rels").unwrap(), Some(None));
        assert_eq!(
            rels_source("ppt/_rels/presentation.xml.rels").unwrap(),
            Some(Some(PartName::new("/ppt/presentation.xml").unwrap()))
        );
        assert_eq!(
            rels_source("ppt/slideMasters/_rels/slideMaster1.xml.rels").unwrap(),
            Some(Some(
                PartName::new("/ppt/slideMasters/slideMaster1.xml").unwrap()
            ))
        );
        assert_eq!(rels_source("ppt/presentation.xml").unwrap(), None);
        assert_eq!(rels_source("notes.rels").unwrap(), None); // not in a _rels dir
    }
}
