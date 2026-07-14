//! Minimal XSD reader: extracts named `xsd:simpleType` definitions using our own `mjx-xml`.
//!
//! The OOXML schemas are plain XML with no inline documentation, so a full XSD toolchain is
//! unnecessary — we only need element nesting + attributes, which `mjx-xml` provides.

use anyhow::{Context, Result};
use mjx_xml::{Event, Reader};

/// A named `xsd:simpleType` and its classified content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleType {
    /// The XSD type name, e.g. `ST_OnOff`.
    pub name: String,
    /// The classified restriction/union/list content.
    pub kind: SimpleKind,
}

/// The classified content of a simple type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleKind {
    /// `xsd:restriction` carrying `xsd:enumeration` values (ordered).
    Enumeration {
        /// The restriction base (e.g. `xsd:string`).
        base: String,
        /// The enumeration values, in document order.
        values: Vec<String>,
    },
    /// `xsd:restriction` with no enumerations (a base type + optional facets).
    Restriction {
        /// The restriction base (e.g. `xsd:string`, `xsd:unsignedLong`, `xsd:hexBinary`).
        base: String,
        /// A `xsd:pattern` value, if present.
        pattern: Option<String>,
    },
    /// `xsd:union memberTypes="..."`.
    Union {
        /// The member type names.
        members: Vec<String>,
    },
    /// `xsd:list itemType="..."`.
    List {
        /// The item type name.
        item: String,
    },
}

#[derive(Default)]
struct Builder {
    name: String,
    base: Option<String>,
    values: Vec<String>,
    pattern: Option<String>,
    union_members: Option<Vec<String>>,
    list_item: Option<String>,
}

impl Builder {
    fn finish(self) -> SimpleType {
        let kind = if let Some(members) = self.union_members {
            SimpleKind::Union { members }
        } else if let Some(item) = self.list_item {
            SimpleKind::List { item }
        } else if !self.values.is_empty() {
            SimpleKind::Enumeration {
                base: self.base.unwrap_or_default(),
                values: self.values,
            }
        } else {
            SimpleKind::Restriction {
                base: self.base.unwrap_or_default(),
                pattern: self.pattern,
            }
        };
        SimpleType {
            name: self.name,
            kind,
        }
    }
}

/// Parses all top-level named `xsd:simpleType` definitions from an XSD document, in document order.
///
/// The OOXML shared/markup schemas do not nest named simple types, so a flat state machine keyed on
/// the enclosing `simpleType` is sufficient and unambiguous.
pub fn parse_simple_types(xsd: &[u8]) -> Result<Vec<SimpleType>> {
    let mut reader = Reader::new(xsd);
    let mut out = Vec::new();
    let mut current: Option<Builder> = None;

    loop {
        match reader.read().context("reading XSD")? {
            Event::Start(e) | Event::Empty(e) => match e.local() {
                "simpleType" => {
                    if let Some(name) = e.attr("name") {
                        current = Some(Builder {
                            name: name.to_owned(),
                            ..Builder::default()
                        });
                    }
                }
                "restriction" => {
                    if let Some(b) = current.as_mut() {
                        b.base = e.attr("base").map(str::to_owned);
                    }
                }
                "enumeration" => {
                    if let (Some(b), Some(value)) = (current.as_mut(), e.attr("value")) {
                        b.values.push(value.to_owned());
                    }
                }
                "union" => {
                    if let Some(b) = current.as_mut() {
                        b.union_members = Some(
                            e.attr("memberTypes")
                                .unwrap_or_default()
                                .split_whitespace()
                                .map(str::to_owned)
                                .collect(),
                        );
                    }
                }
                "list" => {
                    if let Some(b) = current.as_mut() {
                        b.list_item = e.attr("itemType").map(str::to_owned);
                    }
                }
                "pattern" => {
                    if let Some(b) = current.as_mut() {
                        b.pattern = e.attr("value").map(str::to_owned);
                    }
                }
                _ => {}
            },
            Event::End(name) => {
                if name.local == "simpleType" {
                    if let Some(b) = current.take() {
                        out.push(b.finish());
                    }
                }
            }
            Event::Text(_) => {}
            Event::Eof => break,
        }
    }

    Ok(out)
}

/// Reads the `targetNamespace` declared on the root `xsd:schema` element.
pub fn target_namespace(xsd: &[u8]) -> Result<String> {
    let mut reader = Reader::new(xsd);
    loop {
        match reader.read().context("reading XSD root")? {
            Event::Start(e) | Event::Empty(e) if e.local() == "schema" => {
                return e
                    .attr("targetNamespace")
                    .map(str::to_owned)
                    .context("schema has no targetNamespace");
            }
            Event::Eof => anyhow::bail!("no xsd:schema root element"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_enum_union_and_restriction() {
        let xsd = br#"<?xml version="1.0"?>
            <xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema"
                targetNamespace="urn:test">
              <xsd:simpleType name="ST_Color">
                <xsd:restriction base="xsd:string">
                  <xsd:enumeration value="red"/>
                  <xsd:enumeration value="green"/>
                </xsd:restriction>
              </xsd:simpleType>
              <xsd:simpleType name="ST_OnOff">
                <xsd:union memberTypes="xsd:boolean ST_OnOff1"/>
              </xsd:simpleType>
              <xsd:simpleType name="ST_Guid">
                <xsd:restriction base="xsd:token">
                  <xsd:pattern value="\{.*\}"/>
                </xsd:restriction>
              </xsd:simpleType>
            </xsd:schema>"#;
        let types = parse_simple_types(xsd).unwrap();
        assert_eq!(types.len(), 3);
        assert_eq!(types[0].name, "ST_Color");
        assert_eq!(
            types[0].kind,
            SimpleKind::Enumeration {
                base: "xsd:string".to_owned(),
                values: vec!["red".to_owned(), "green".to_owned()],
            }
        );
        assert_eq!(
            types[1].kind,
            SimpleKind::Union {
                members: vec!["xsd:boolean".to_owned(), "ST_OnOff1".to_owned()],
            }
        );
        assert_eq!(
            types[2].kind,
            SimpleKind::Restriction {
                base: "xsd:token".to_owned(),
                pattern: Some("\\{.*\\}".to_owned()),
            }
        );
    }

    #[test]
    fn reads_target_namespace() {
        let xsd = br#"<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:x"/>"#;
        assert_eq!(target_namespace(xsd).unwrap(), "urn:x");
    }
}
