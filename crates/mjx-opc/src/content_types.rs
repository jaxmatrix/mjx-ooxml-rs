//! The `[Content_Types].xml` stream: maps parts to their MIME content types.
//!
//! Two rules, in precedence order: an `Override` names a specific part; a `Default` maps a file
//! extension. Entries are stored in their original document order to preserve round-trip fidelity.

use mjx_xml::{Event, Reader};

use crate::error::OpcError;
use crate::name::PartName;

/// The special container item that holds the content-type map. It is not itself a part.
pub const CONTENT_TYPES_ZIP_NAME: &str = "[Content_Types].xml";

/// A `<Default Extension=".." ContentType=".."/>` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Default {
    /// The (lowercased) file extension this rule matches.
    pub extension: String,
    /// The content type applied to parts with that extension.
    pub content_type: String,
}

/// An `<Override PartName=".." ContentType=".."/>` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override {
    /// The specific part this rule applies to.
    pub part_name: PartName,
    /// The content type applied to that part.
    pub content_type: String,
}

/// The parsed content-type map, preserving original entry order.
#[derive(Debug, Clone, Default)]
pub struct ContentTypes {
    defaults: Vec<Default>,
    overrides: Vec<Override>,
}

impl ContentTypes {
    /// Parses `[Content_Types].xml` from its raw bytes.
    ///
    /// # Errors
    /// Returns [`OpcError`] on malformed XML or a rule missing a required attribute.
    pub fn parse(xml: &[u8]) -> Result<Self, OpcError> {
        let mut reader = Reader::new(xml);
        let mut ct = ContentTypes::default();
        loop {
            let event = reader.read()?;
            let element = match event {
                Event::Start(e) | Event::Empty(e) => e,
                Event::Eof => break,
                _ => continue,
            };
            match element.local() {
                "Default" => {
                    let extension = element
                        .attr("Extension")
                        .ok_or_else(|| OpcError::malformed("Default missing Extension"))?
                        .to_ascii_lowercase();
                    let content_type = element
                        .attr("ContentType")
                        .ok_or_else(|| OpcError::malformed("Default missing ContentType"))?
                        .to_owned();
                    ct.defaults.push(Default {
                        extension,
                        content_type,
                    });
                }
                "Override" => {
                    let part_name = element
                        .attr("PartName")
                        .ok_or_else(|| OpcError::malformed("Override missing PartName"))?;
                    let content_type = element
                        .attr("ContentType")
                        .ok_or_else(|| OpcError::malformed("Override missing ContentType"))?
                        .to_owned();
                    ct.overrides.push(Override {
                        part_name: PartName::new(part_name)?,
                        content_type,
                    });
                }
                _ => {} // `Types` root and anything else.
            }
        }
        Ok(ct)
    }

    /// The `Default` rules, in document order.
    #[must_use]
    pub fn defaults(&self) -> &[Default] {
        &self.defaults
    }

    /// The `Override` rules, in document order.
    #[must_use]
    pub fn overrides(&self) -> &[Override] {
        &self.overrides
    }

    /// Resolves the content type for a part: an `Override` wins, otherwise the `Default` for the
    /// part's extension (matched case-insensitively). Returns `None` if neither applies.
    #[must_use]
    pub fn content_type_of(&self, part: &PartName) -> Option<&str> {
        if let Some(o) = self.overrides.iter().find(|o| &o.part_name == part) {
            return Some(&o.content_type);
        }
        let ext = part.extension()?;
        self.defaults
            .iter()
            .find(|d| d.extension == ext)
            .map(|d| d.content_type.as_str())
    }

    /// Inserts or updates the `Override` for `part_name`, setting its content type. The caller edits
    /// the `[Content_Types].xml` tree in tandem so this view never drifts from the raw part.
    pub(crate) fn upsert_override(&mut self, part_name: PartName, content_type: String) {
        if let Some(existing) = self.overrides.iter_mut().find(|o| o.part_name == part_name) {
            existing.content_type = content_type;
        } else {
            self.overrides.push(Override {
                part_name,
                content_type,
            });
        }
    }

    /// Removes the `Override` for `part`, if any. Returns whether one was removed.
    pub(crate) fn remove_override(&mut self, part: &PartName) -> bool {
        let before = self.overrides.len();
        self.overrides.retain(|o| &o.part_name != part);
        self.overrides.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &[u8] = br#"<?xml version="1.0"?>
        <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
          <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
          <Default Extension="xml" ContentType="application/xml"/>
          <Override PartName="/ppt/presentation.xml" ContentType="app/pml"/>
        </Types>"#;

    #[test]
    fn parses_and_resolves() {
        let ct = ContentTypes::parse(XML).unwrap();
        assert_eq!(ct.defaults().len(), 2);
        assert_eq!(ct.overrides().len(), 1);
        // order preserved
        assert_eq!(ct.defaults()[0].extension, "rels");
        assert_eq!(ct.defaults()[1].extension, "xml");

        let pres = PartName::new("/ppt/presentation.xml").unwrap();
        assert_eq!(ct.content_type_of(&pres), Some("app/pml")); // override wins

        let rels = PartName::new("/ppt/_rels/presentation.xml.rels").unwrap();
        assert_eq!(
            ct.content_type_of(&rels),
            Some("application/vnd.openxmlformats-package.relationships+xml")
        ); // default by extension
    }
}
