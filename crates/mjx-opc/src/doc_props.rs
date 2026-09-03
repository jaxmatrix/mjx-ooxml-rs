//! Document properties (`docProps/core.xml`, `docProps/app.xml`) — the OPC Core Properties part
//! (ECMA-376 Part 2 §8.3) and the extended-properties part (`shared-documentPropertiesExtended.xsd`).
//!
//! # Why this lives here, and not in a format crate
//!
//! `docProps/*` is not PresentationML, WordprocessingML or SpreadsheetML — it is packaging-layer
//! markup every one of the three formats carries the same way (`sample.docx` and `sample.xlsx` each
//! ship exactly these two parts; PowerPoint's `charts.pptx` fixture, written by python-pptx, ships
//! them too). Modelling it once here, rather than once per format crate, is what
//! [`mjx_pptx::Presentation::blank`](../../mjx_pptx/struct.Presentation.html#method.blank) and the
//! `Document::blank` / `Workbook::blank` constructors Phase C and D still owe both call into.
//!
//! # What is modelled, and what is not
//!
//! Only the fields a caller actually sets: [`CoreProperties`] carries `title`, `creator`, `created`
//! and `modified` (`opc-coreProperties.xsd`'s `CT_CoreProperties` has fifteen; ten more — `category`,
//! `contentStatus`, `description`, `identifier`, `keywords`, `language`, `lastModifiedBy`,
//! `lastPrinted`, `revision`, `subject`, `version` — are omitted because nothing in this workspace
//! sets them yet, and every one is `minOccurs="0"`, so leaving them out is schema-valid, not a
//! shortcut). [`ExtendedProperties`] carries `application` alone, of `CT_Properties`'s twenty-five.
//!
//! `docProps/custom.xml` (`shared-documentPropertiesCustom.xsd` +
//! `shared-documentPropertiesVariantTypes.xsd`) is **not** modelled here and never will be by this
//! module: no committed fixture carries one, `Presentation::blank` (nor the Word/Excel constructors
//! that consume this module) has any use for open-ended caller-defined properties, and
//! `crates/mjx-schema-gate/src/categories.rs` deliberately leaves its namespace off both the
//! modelled and the preserved-foreign lists — an entry nothing exercises fails that gate's own
//! dead-allowlist check.
//!
//! # Order
//!
//! `CT_CoreProperties` and `CT_Properties` are both declared with `xs:all`, not `xsd:sequence` — the
//! one XSD group where ECMA-376 places **no** constraint on child order at all. [`core_xml`] and
//! [`extended_xml`] pick one fixed order (declaration order in [`CoreProperties`] /
//! [`ExtendedProperties`]) purely so two calls with the same fields produce the same bytes; any other
//! order a reader might see (`tests/fixtures/charts.pptx`'s `docProps/core.xml`, written by
//! python-pptx, orders `title`, `subject`, `creator` before `description`) is equally schema-valid.
//! There is accordingly no `xtask` child-order table for either namespace: an insertion-order table
//! has nothing to govern when the schema does not care about order, and both parts are written whole
//! by this module on every call, never edited in place — the same reason
//! `crates/mjx-schema-gate/src/categories.rs`'s `OPC_RELATIONSHIPS_NS` / `OPC_CONTENT_TYPES_NS`
//! entries carry no ordering table either.
//!
//! # Dates
//!
//! `dcterms:created` / `dcterms:modified` must carry `xsi:type="dcterms:W3CDTF"` (ECMA-376 Part 2
//! §8.3.4.3) — the W3C Date and Time Formats profile of ISO 8601. This module always writes and only
//! ever accepts the complete "date plus hours, minutes, seconds, UTC" granularity
//! (`YYYY-MM-DDThh:mm:ssZ`), the one every committed fixture with a `dcterms:created` uses
//! (`tests/fixtures/charts.pptx`: `2013-01-27T09:14:16Z`); W3C-DTF's coarser granularities
//! (date-only, year-month, …) are part of the profile but not part of what this project emits or
//! needs to accept, so [`DocumentTimestamp`] does not represent them. There is **no `now()`
//! constructor**: a value can only be built from caller-supplied fields, so a package built from
//! nothing has no wall-clock dependency and two calls with the same fields are byte-identical.

use std::fmt::Write as _;

use crate::error::OpcError;

/// The Core Properties part's fixed location (ECMA-376 Part 2 §8.2: "a package shall contain at most
/// one Core Properties part").
pub const CORE_PROPERTIES_PART: &str = "/docProps/core.xml";
/// The extended-properties part's conventional location, matching every committed fixture that
/// carries one.
pub const EXTENDED_PROPERTIES_PART: &str = "/docProps/app.xml";

/// `docProps/core.xml`'s content type (`[Content_Types].xml` `Override`), confirmed against
/// `tests/fixtures/sample.docx`.
pub const CORE_PROPERTIES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-package.core-properties+xml";
/// `docProps/app.xml`'s content type, confirmed against `tests/fixtures/sample.docx`.
pub const EXTENDED_PROPERTIES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.extended-properties+xml";

/// The relationship type from the package root to the Core Properties part, confirmed against
/// `tests/fixtures/sample.docx`'s `_rels/.rels`.
pub const CORE_PROPERTIES_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
/// The relationship type from the package root to the extended-properties part, confirmed against
/// `tests/fixtures/sample.docx`'s `_rels/.rels`.
pub const EXTENDED_PROPERTIES_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties";

/// The Core Properties namespace (ECMA-376 Part 2, `opc-coreProperties.xsd`) — Dublin Core plus a
/// handful of OPC-defined elements, not an ECMA-376 Part 4 namespace, which is why the Part 4
/// namespace table (`mjx_ooxml_types::namespaces`) does not carry it.
pub const CORE_PROPERTIES_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
/// `shared-documentPropertiesExtended.xsd`'s target namespace. A `const` here duplicates the literal
/// `mjx_ooxml_types::namespaces::SHARED_DOCUMENT_PROPERTIES_EXTENDED.transitional` already carries —
/// `mjx-opc` sits beside `mjx-ooxml-types` in the packaging/compat tier and must not depend sideways
/// on it (`CLAUDE.md`'s layering rule), so the string is restated here, exactly as `mjx-pptx`
/// restates OPC-level relationship-type strings in its own `constants.rs` rather than reaching
/// upward past its own tier.
pub const EXTENDED_PROPERTIES_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
/// `dc:` — Dublin Core Simple, ISO 15836-1. `opc-coreProperties.xsd` references `dc:creator` and
/// `dc:title` by `ref`, not by declaring them itself.
const DUBLIN_CORE_NAMESPACE: &str = "http://purl.org/dc/elements/1.1/";
/// `dcterms:` — DCMI Metadata Terms, ISO 15836-2. `opc-coreProperties.xsd` references
/// `dcterms:created` and `dcterms:modified` the same way.
const DUBLIN_CORE_TERMS_NAMESPACE: &str = "http://purl.org/dc/terms/";
/// `xsi:` — required on `dcterms:created` / `dcterms:modified` for the `xsi:type="dcterms:W3CDTF"`
/// attribute ECMA-376 Part 2 §8.3.4.3 mandates on both.
const XML_SCHEMA_INSTANCE_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// A point in time formatted for `dcterms:created` / `dcterms:modified`'s `xsi:type="dcterms:W3CDTF"`
/// content (ECMA-376 Part 2 §8.3.4.3): the W3C-DTF "complete date plus hours, minutes, seconds, UTC"
/// granularity, `YYYY-MM-DDThh:mm:ssZ`.
///
/// Built only from explicit fields — there is no `now()` — so a value is deterministic by
/// construction; see the [module docs](self#dates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentTimestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl DocumentTimestamp {
    /// Builds a timestamp from its calendar and clock fields (UTC).
    ///
    /// This checks each field's *range* (a four-digit year, `1..=12`, `1..=31`, and the usual clock
    /// bounds), not full calendar validity — `2024-02-30` passes this constructor and would need a
    /// calendar library to reject, which this packaging-layer module deliberately does not depend on.
    /// A caller that needs that guarantee gets it from whatever validated the fields before they
    /// arrived here.
    ///
    /// # Errors
    /// Returns [`OpcError::InvalidDocumentTimestamp`] if any field is outside its range.
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, OpcError> {
        let fields: &[(&str, u32, u32, u32)] = &[
            ("year", u32::from(year), 1, 9999),
            ("month", u32::from(month), 1, 12),
            ("day", u32::from(day), 1, 31),
            ("hour", u32::from(hour), 0, 23),
            ("minute", u32::from(minute), 0, 59),
            ("second", u32::from(second), 0, 59),
        ];
        for (field, value, min, max) in fields {
            if *value < *min || *value > *max {
                return Err(OpcError::InvalidDocumentTimestamp {
                    field,
                    value: *value,
                    min: *min,
                    max: *max,
                });
            }
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// The four-digit year.
    #[must_use]
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The month, `1..=12`.
    #[must_use]
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of month, `1..=31`.
    #[must_use]
    pub fn day(&self) -> u8 {
        self.day
    }

    /// The hour, `0..=23`.
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, `0..=59`.
    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, `0..=59`.
    #[must_use]
    pub fn second(&self) -> u8 {
        self.second
    }

    /// Renders the canonical `YYYY-MM-DDThh:mm:ssZ` form `xsi:type="dcterms:W3CDTF"` content takes.
    fn to_w3cdtf(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// The Core Properties fields this project sets. See the [module docs](self) for which of
/// `CT_CoreProperties`'s fifteen elements this covers and why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoreProperties {
    /// `dc:title`.
    pub title: Option<String>,
    /// `dc:creator`.
    pub creator: Option<String>,
    /// `dcterms:created`.
    pub created: Option<DocumentTimestamp>,
    /// `dcterms:modified`.
    pub modified: Option<DocumentTimestamp>,
}

/// The extended-properties fields this project sets. See the [module docs](self) for which of
/// `CT_Properties`'s twenty-five elements this covers and why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtendedProperties {
    /// `Application` — the generating application's name.
    pub application: Option<String>,
}

/// Writes one element with escaped text content, or nothing if `value` is absent.
fn write_element(out: &mut String, prefix: &str, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        let _ = write!(
            out,
            "<{prefix}{name}>{}</{prefix}{name}>",
            mjx_xml::text::escape_text(value)
        );
    }
}

/// Builds `docProps/core.xml`'s bytes.
///
/// Every field is optional and `CT_CoreProperties` is an `xs:all` group, so an all-`None` value
/// produces a schema-valid, childless `<cp:coreProperties/>` rather than an error — see the
/// [module docs](self#order) for why the element order below is one arbitrary schema-valid choice
/// among many.
#[must_use]
pub fn core_xml(props: &CoreProperties) -> Vec<u8> {
    let mut body = String::new();
    write_element(&mut body, "dc:", "title", props.title.as_deref());
    write_element(&mut body, "dc:", "creator", props.creator.as_deref());
    if let Some(created) = props.created {
        let _ = write!(
            body,
            r#"<dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>"#,
            created.to_w3cdtf()
        );
    }
    if let Some(modified) = props.modified {
        let _ = write!(
            body,
            r#"<dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>"#,
            modified.to_w3cdtf()
        );
    }
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            "\n",
            r#"<cp:coreProperties xmlns:cp="{cp}" xmlns:dc="{dc}" xmlns:dcterms="{dcterms}" xmlns:xsi="{xsi}">{body}</cp:coreProperties>"#,
        ),
        cp = CORE_PROPERTIES_NAMESPACE,
        dc = DUBLIN_CORE_NAMESPACE,
        dcterms = DUBLIN_CORE_TERMS_NAMESPACE,
        xsi = XML_SCHEMA_INSTANCE_NAMESPACE,
        body = body,
    )
    .into_bytes()
}

/// Builds `docProps/app.xml`'s bytes. See [`core_xml`] — the same "all fields optional, `xs:all`,
/// one arbitrary order" reasoning applies.
#[must_use]
pub fn extended_xml(props: &ExtendedProperties) -> Vec<u8> {
    let mut body = String::new();
    write_element(&mut body, "", "Application", props.application.as_deref());
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            "\n",
            r#"<Properties xmlns="{ns}">{body}</Properties>"#,
        ),
        ns = EXTENDED_PROPERTIES_NAMESPACE,
        body = body,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_core_properties_writes_a_childless_element() {
        let bytes = core_xml(&CoreProperties::default());
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains("<cp:coreProperties"));
        assert!(xml.contains("</cp:coreProperties>"));
        // No property element at all: xs:all with every child minOccurs="0" allows this.
        assert!(!xml.contains("<dc:"));
        assert!(!xml.contains("<dcterms:"));
    }

    #[test]
    fn a_default_extended_properties_writes_a_childless_element() {
        let bytes = extended_xml(&ExtendedProperties::default());
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains("<Properties xmlns="));
        assert!(!xml.contains("<Application>"));
    }

    #[test]
    fn populated_core_properties_carries_every_field_in_declaration_order() {
        let props = CoreProperties {
            title: Some("Report".to_owned()),
            creator: Some("A & B".to_owned()),
            created: Some(DocumentTimestamp::new(2013, 1, 27, 9, 14, 16).unwrap()),
            modified: Some(DocumentTimestamp::new(2013, 1, 27, 9, 15, 58).unwrap()),
        };
        let bytes = core_xml(&props);
        let xml = String::from_utf8(bytes).unwrap();
        assert_eq!(
            xml,
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
                r#"<dc:title>Report</dc:title>"#,
                r#"<dc:creator>A &amp; B</dc:creator>"#,
                r#"<dcterms:created xsi:type="dcterms:W3CDTF">2013-01-27T09:14:16Z</dcterms:created>"#,
                r#"<dcterms:modified xsi:type="dcterms:W3CDTF">2013-01-27T09:15:58Z</dcterms:modified>"#,
                r#"</cp:coreProperties>"#,
            )
        );
    }

    #[test]
    fn populated_extended_properties_escapes_and_writes_application() {
        let props = ExtendedProperties {
            application: Some("<mjx-ooxml-rs>".to_owned()),
        };
        let bytes = extended_xml(&props);
        let xml = String::from_utf8(bytes).unwrap();
        assert_eq!(
            xml,
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                "\n",
                r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">"#,
                // escape_text is minimal: only `<` and `&`, so the trailing `>` stays literal.
                r#"<Application>&lt;mjx-ooxml-rs></Application>"#,
                r#"</Properties>"#,
            )
        );
    }

    #[test]
    fn two_calls_with_the_same_fields_are_byte_identical() {
        let props = CoreProperties {
            title: Some("Same".to_owned()),
            ..CoreProperties::default()
        };
        assert_eq!(core_xml(&props), core_xml(&props));
    }

    #[test]
    fn a_timestamp_field_outside_its_range_is_a_typed_error() {
        let err = DocumentTimestamp::new(2024, 13, 1, 0, 0, 0).unwrap_err();
        match err {
            OpcError::InvalidDocumentTimestamp {
                field,
                value,
                min,
                max,
            } => {
                assert_eq!(field, "month");
                assert_eq!(value, 13);
                assert_eq!((min, max), (1, 12));
            }
            other => panic!("expected InvalidDocumentTimestamp, got {other:?}"),
        }
    }

    #[test]
    fn a_valid_timestamp_round_trips_through_its_accessors() {
        let ts = DocumentTimestamp::new(2013, 1, 27, 9, 14, 16).unwrap();
        assert_eq!(
            (
                ts.year(),
                ts.month(),
                ts.day(),
                ts.hour(),
                ts.minute(),
                ts.second()
            ),
            (2013, 1, 27, 9, 14, 16)
        );
        assert_eq!(ts.to_w3cdtf(), "2013-01-27T09:14:16Z");
    }
}
