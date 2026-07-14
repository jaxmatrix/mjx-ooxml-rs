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
}

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
