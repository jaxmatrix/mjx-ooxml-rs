//! The attribute-only leaf: the shape **seventeen** of the workbook cluster's twenty-nine complex
//! types have, and the one macro that declares them.
//!
//! # Why a macro rather than seventeen hand-written triples
//!
//! `sml.xsd`'s workbook cluster is overwhelmingly *attribute bags*: a complex type with an
//! `xsd:sequence` that is empty or holds nothing but `extLst`, and between one and twenty-four
//! attributes. `CT_FileVersion`, `CT_WorkbookPr`, `CT_CalcPr`, `CT_WorkbookProtection`,
//! `CT_FileSharing`, `CT_FileRecoveryPr`, `CT_OleSize`, `CT_SmartTagPr`, `CT_SmartTagType`,
//! `CT_FunctionGroup`, `CT_WebPublishing`, `CT_WebPublishObject`, `CT_ExternalReference`,
//! `CT_PivotCache`, `CT_Sheet`, `CT_BookView` and `CT_CustomWorkbookView` are all that shape.
//!
//! [`mjx_derive::XmlAttributes`] already generates the typed accessors from a struct-level
//! declaration; what it does not generate is the struct itself and its
//! [`FromXml`](mjx_ooxml_core::FromXml)/[`ToXml`](mjx_ooxml_core::ToXml) pair, which for an
//! attribute bag is the *same* thirty lines every time — `mjx-docx`'s `web_settings.rs` writes that
//! pair out **eight** times over, and `settings.rs` many more. Writing them once here means a type is declared
//! by saying what the schema says about it and nothing else, and it means the fidelity discipline
//! (keep the element's own name and prefix, keep every attribute in order, keep the self-closing
//! flag, keep unmodelled children) is in **one** place rather than in seventeen copies that could
//! drift apart one at a time.
//!
//! # What a bag preserves
//!
//! Everything. The element's [`RawName`] is kept as it was read, so the prefix the file bound
//! survives; the attribute vector is never rebuilt, so an attribute this crate has never heard of
//! keeps its position, its prefix and its quote character (`sample.xlsx`'s
//! `workbookPr/@dateCompatibility`, which the Transitional schema does not declare, is exactly
//! that); `empty` records whether the file wrote `<x/>` or `<x></x>`; and children the type does not
//! model — an `extLst`, an `mc:AlternateContent`, a comment — are held verbatim in `extra` and
//! written back in document order.
//!
//! A getter takes `&self` and cannot change the file. Normalization happens only where a setter
//! runs, which is the asymmetry [`mjx_derive`]'s own documentation states.

use mjx_ooxml_core::{AttributeError, Interner, RawAttribute, RawName};
use mjx_ooxml_types::namespaces::{SchemaNamespace, SHARED_RELATIONSHIP_REFERENCE, SML};

/// Builds a SpreadsheetML qualified name, bound to `prefix` — or to the default namespace when
/// `prefix` is `None`, which is how every producer this project has read writes `xl/workbook.xml`.
///
/// The namespace symbol is the **Transitional** URI, which is the conformance world
/// `mjx-schema-gate` validates against; a Strict document's own elements keep the names they were
/// read with, because nothing authored here replaces them.
#[must_use]
pub(crate) fn sml_name(interner: &mut Interner, prefix: Option<&str>, local: &str) -> RawName {
    RawName {
        prefix: prefix.map(|prefix| interner.intern(prefix)),
        local: interner.intern(local),
        namespace: Some(interner.intern(SML.transitional)),
    }
}

/// The prefix `attributes` binds to `namespace` through an `xmlns:PREFIX="uri"` declaration.
///
/// Both conformance worlds' URIs match, because a Strict document binds the Strict one and this
/// crate reads both. `None` means the declaration is not on this element — for `r:id`, which is what
/// this is used for, that means the element can carry no relationship reference at all, since an
/// attribute in no namespace is not `r:id` however it is spelled.
#[must_use]
pub(crate) fn namespace_prefix<'a>(
    attributes: &[RawAttribute],
    interner: &'a Interner,
    namespace: SchemaNamespace,
) -> Option<&'a str> {
    attributes.iter().find_map(|attribute| {
        let prefix = attribute.name.prefix?;
        if interner.resolve(prefix) != "xmlns" {
            return None;
        }
        let uri = core::str::from_utf8(&attribute.value).ok()?;
        (uri == namespace.transitional || Some(uri) == namespace.strict)
            .then(|| interner.resolve(attribute.name.local))
    })
}

/// Reads `r:id` — the `xsd:attribute ref="r:id"` three types in this cluster declare — under
/// whichever prefix the part bound the relationship-reference namespace to.
///
/// **Why the prefix is a parameter rather than the literal `"r"`.** The fidelity reader interns
/// attribute names with no resolved namespace (see `mjx_xml::attribute`'s own documentation), so an
/// attribute's namespace is exactly its prefix, and the prefix a file binds is the producer's
/// choice. Every producer this project has read writes `r`, and none of them is obliged to. The
/// binding lives on the part's root element, so [`WorkbookPart`](super::WorkbookPart) resolves it
/// once and passes it down.
///
/// # Errors
/// [`AttributeError`] if the value is not UTF-8 or carries a reference that will not decode.
pub(crate) fn read_relationship_id(
    attributes: &[RawAttribute],
    interner: &Interner,
    reference_prefix: Option<&str>,
) -> Result<Option<String>, AttributeError> {
    let Some(prefix) = reference_prefix else {
        return Ok(None);
    };
    let Some(attribute) = mjx_xml::attribute::find(attributes, interner, Some(prefix), "id") else {
        return Ok(None);
    };
    Ok(Some(
        mjx_xml::attribute::decoded_value(attribute, "r:id")?.into_owned(),
    ))
}

/// Writes `r:id`, in place if it is already there and appended otherwise.
///
/// `reference_prefix` is the prefix the part binds to the relationship-reference namespace; a caller
/// that has none must declare one before this can name anything, which is why this takes the prefix
/// rather than inventing `r` and leaving it unbound.
pub(crate) fn write_relationship_id(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    reference_prefix: &str,
    relationship_id: &str,
) {
    mjx_xml::attribute::set(
        attributes,
        interner,
        Some(reference_prefix),
        "id",
        relationship_id,
    );
}

/// The relationship-reference namespace, re-exported so the modules beside this one name it once.
pub(crate) const RELATIONSHIP_REFERENCE: SchemaNamespace = SHARED_RELATIONSHIP_REFERENCE;

/// Declares one attribute-only complex type: the struct, its typed accessors, and its
/// `FromXml`/`ToXml` pair.
///
/// ```ignore
/// attribute_bag! {
///     /// `x:oleSize` (`CT_OleSize`) — the range an OLE consumer shows.
///     #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
///     EmbeddedObjectSize, "oleSize"
/// }
/// ```
///
/// The `#[xml(attribute(..))]` lines are [`mjx_derive::XmlAttributes`]'s own grammar, passed
/// through unchanged — this macro adds no vocabulary of its own, so a reader who knows the derive
/// knows this. They are emitted **after** the generated `#[derive(..)]` because a derive helper
/// attribute is only in scope once the derive that registers it has been seen.
///
/// A type whose *every* attribute is hand-written — `CT_ExternalReference`, whose only attribute is
/// `r:id` — uses [`bag_without_declared_attributes!`] instead, because `XmlAttributes` requires at
/// least one declaration and deriving it over none is a compile error rather than a no-op.
macro_rules! attribute_bag {
    (
        $(#[$meta:meta])*
        $name:ident, $local:literal $(,)?
    ) => {
        $crate::workbook::leaf::bag_body! {
            #[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
            $(#[$meta])*
            $name, $local
        }
    };
}

/// [`attribute_bag!`] for a type that declares no attributes through the derive at all.
macro_rules! bag_without_declared_attributes {
    (
        $(#[$meta:meta])*
        $name:ident, $local:literal $(,)?
    ) => {
        $crate::workbook::leaf::bag_body! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            $(#[$meta])*
            $name, $local
        }
    };
}

/// The struct, the constructors and the `FromXml`/`ToXml` pair every attribute bag shares — the
/// part that is identical whether or not the type declares attributes through the derive.
macro_rules! bag_body {
    (
        $(#[$meta:meta])*
        $name:ident, $local:literal
    ) => {
        $(#[$meta])*
        pub struct $name {
            name: ::mjx_ooxml_core::RawName,
            attributes: ::std::vec::Vec<::mjx_ooxml_core::RawAttribute>,
            extra: ::std::vec::Vec<::mjx_ooxml_core::RawNode>,
            empty: bool,
        }

        impl $name {
            #[doc = concat!("The wire local name this type is written under: `", $local, "`.")]
            pub const WIRE_LOCAL: &'static str = $local;

            #[doc = concat!("Builds a new `", $local, "` with every attribute absent, bound to \
                `prefix` — or to the default namespace when `prefix` is `None`.")]
            #[must_use]
            #[allow(dead_code)]
            pub fn new(
                interner: &mut ::mjx_ooxml_core::Interner,
                prefix: ::core::option::Option<&str>,
            ) -> Self {
                Self {
                    name: $crate::workbook::leaf::sml_name(interner, prefix, $local),
                    attributes: ::std::vec::Vec::new(),
                    extra: ::std::vec::Vec::new(),
                    empty: true,
                }
            }

            /// The element's own qualified name, as the file wrote it.
            #[must_use]
            #[allow(dead_code)]
            pub fn element_name(&self) -> ::mjx_ooxml_core::RawName {
                self.name
            }

            /// Children this type does not model — an `extLst`, an `mc:AlternateContent`, a
            /// comment — in document order, exactly as they were read.
            #[must_use]
            #[allow(dead_code)]
            pub fn extra(&self) -> &[::mjx_ooxml_core::RawNode] {
                &self.extra
            }
        }

        impl ::mjx_ooxml_core::FromXml for $name {
            fn from_xml(
                element: &::mjx_ooxml_core::RawElement,
                _interner: &::mjx_ooxml_core::Interner,
            ) -> ::core::result::Result<Self, ::mjx_ooxml_core::FromXmlError> {
                ::core::result::Result::Ok(Self {
                    name: element.name,
                    attributes: element.attributes.clone(),
                    extra: element.children.clone(),
                    empty: element.empty,
                })
            }
        }

        impl ::mjx_ooxml_core::ToXml for $name {
            fn to_xml(
                &self,
                _interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                let children = self.extra.clone();
                let empty = self.empty && children.is_empty();
                ::mjx_ooxml_core::RawElement::rebuilt(
                    self.name,
                    self.attributes.clone(),
                    children,
                    empty,
                )
            }
        }
    };
}

/// Gives a type declared by [`attribute_bag!`] its `r:id` accessors.
///
/// Separate from the bag because `r:id` is the one attribute in this cluster whose *prefix* is the
/// file's choice rather than the schema's — see [`read_relationship_id`] — so it cannot be declared
/// through [`mjx_derive::XmlAttributes`]'s literal-prefix grammar without pinning `r`.
macro_rules! relationship_reference {
    ($name:ident) => {
        impl $name {
            /// The `r:id` this element names, under `reference_prefix` — the prefix the part binds
            /// to the relationship-reference namespace, from
            /// [`WorkbookPart::relationship_prefix`](super::WorkbookPart::relationship_prefix).
            ///
            /// `None` means the attribute is absent, or that the part binds the namespace to no
            /// prefix at all and therefore cannot spell `r:id`. Resolving the id to a part is
            /// `mjx-xlsx`'s: this crate holds the raw identifier and knows nothing about packages.
            ///
            /// # Errors
            /// [`AttributeError`](::mjx_ooxml_core::AttributeError) if the value is not UTF-8 or
            /// carries a reference that will not decode.
            #[allow(dead_code)]
            pub fn relationship_id(
                &self,
                interner: &::mjx_ooxml_core::Interner,
                reference_prefix: ::core::option::Option<&str>,
            ) -> ::core::result::Result<
                ::core::option::Option<::std::string::String>,
                ::mjx_ooxml_core::AttributeError,
            > {
                $crate::workbook::leaf::read_relationship_id(
                    &self.attributes,
                    interner,
                    reference_prefix,
                )
            }

            /// Points this element at `relationship_id`, writing the attribute in place if it is
            /// already there.
            #[allow(dead_code)]
            pub fn set_relationship_id(
                &mut self,
                interner: &mut ::mjx_ooxml_core::Interner,
                reference_prefix: &str,
                relationship_id: &str,
            ) {
                $crate::workbook::leaf::write_relationship_id(
                    &mut self.attributes,
                    interner,
                    reference_prefix,
                    relationship_id,
                );
            }
        }
    };
}

pub(crate) use {attribute_bag, bag_body, bag_without_declared_attributes, relationship_reference};
