//! The attribute-only leaf: the shape most of `sml.xsd`'s complex types have, and the one macro that
//! declares them.
//!
//! # Where this lives, and why it is not under one subject
//!
//! It began as `workbook/leaf.rs` (MJXOFF-100), because the workbook cluster was the first place
//! that needed it — **seventeen** of that cluster's twenty-nine complex types are this shape. It is
//! at the crate root as of MJXOFF-102, which needs the same macro for nine more: `CT_SheetPr`,
//! `CT_SheetDimension`, `CT_OutlinePr`, `CT_PageSetUpPr`, `CT_SheetFormatPr`, `CT_SheetCalcPr`,
//! `CT_Col`, `CT_Pane` and `CT_Selection`. A `use crate::workbook::leaf::…` from the worksheet spine
//! would have said the macro belongs to the workbook, which it never did.
//!
//! # Why a macro rather than a hand-written triple per type
//!
//! `sml.xsd` is overwhelmingly *attribute bags*: a complex type with an
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
/// binding lives on the part's root element, so [`WorkbookPart`](crate::WorkbookPart) resolves it
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

/// Parses `markup` and re-interns the element it holds into `interner`, so that a model built from
/// bytes can be inserted into a document that was interned somewhere else.
///
/// # Why authoring goes through the parser
///
/// [`FontProperties`](crate::FontProperties) writes itself as **bytes** and keeps the markup it does
/// not model as bytes too (see its own documentation for why the packed stores in this crate
/// preserve that way). So there is no node-building path that could reproduce it: a second
/// fifteen-slot writer would still have to parse the unknown bucket, and the two writers would be
/// free to drift apart. Parsing what the one writer produced keeps a single description of the
/// markup, and costs one parse of an element that is a hundred bytes long.
///
/// # Errors
/// [`mjx_xml::XmlError`] if `markup` is not well-formed — reachable only through a hand-authored
/// unknown bucket, since markup this crate serialized never is.
pub(crate) fn parse_into(
    markup: &[u8],
    interner: &mut Interner,
) -> Result<mjx_ooxml_core::RawElement, mjx_xml::XmlError> {
    let document = mjx_xml::fidelity::parse(markup)?;
    Ok(adopt_element(&document.root, &document.interner, interner))
}

/// Copies `element` into an owned one whose every name is interned in `into`.
///
/// The copy carries **no** verbatim source range, which is correct: it describes a different
/// buffer from the one the element was parsed out of.
fn adopt_element(
    element: &mjx_ooxml_core::RawElement,
    source: &Interner,
    into: &mut Interner,
) -> mjx_ooxml_core::RawElement {
    let attributes = element
        .attributes
        .iter()
        .map(|attribute| RawAttribute {
            name: adopt_name(attribute.name, source, into),
            value: attribute.value.clone(),
            quote: attribute.quote,
        })
        .collect();
    let children = element
        .children
        .iter()
        .map(|child| match child {
            mjx_ooxml_core::RawNode::Element(child) => {
                mjx_ooxml_core::RawNode::Element(adopt_element(child, source, into))
            }
            other => other.clone(),
        })
        .collect::<Vec<_>>();
    let empty = element.empty && children.is_empty();
    mjx_ooxml_core::RawElement::rebuilt(
        adopt_name(element.name, source, into),
        attributes,
        children,
        empty,
    )
}

/// Re-interns one qualified name from `source` into `into`.
fn adopt_name(name: RawName, source: &Interner, into: &mut Interner) -> RawName {
    RawName {
        prefix: name
            .prefix
            .map(|symbol| into.intern(source.resolve(symbol))),
        local: into.intern(source.resolve(name.local)),
        namespace: name
            .namespace
            .map(|symbol| into.intern(source.resolve(symbol))),
    }
}

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
        $crate::leaf::bag_body! {
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
        $crate::leaf::bag_body! {
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
                    name: $crate::leaf::sml_name(interner, prefix, $local),
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

            /// This element rebuilt as a [`RawElement`](::mjx_ooxml_core::RawElement), **without an
            /// interner**.
            ///
            /// [`ToXml::to_xml`](::mjx_ooxml_core::ToXml::to_xml) takes `&mut Interner` because a
            /// model that authors a name has to intern it; an attribute bag never authors one — it
            /// keeps the [`RawName`](::mjx_ooxml_core::RawName) it was read with and every attribute the file wrote — so
            /// nothing here needs one. `to_xml` is this method, with the parameter ignored.
            ///
            /// That distinction is load-bearing for `crate::worksheet`, which serializes its
            /// children to **bytes** from a `&self` writer and so has no mutable interner to lend.
            #[must_use]
            #[allow(dead_code)]
            pub fn as_raw_element(&self) -> ::mjx_ooxml_core::RawElement {
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
                self.as_raw_element()
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
            /// [`WorkbookPart::relationship_prefix`](crate::WorkbookPart::relationship_prefix).
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
                $crate::leaf::read_relationship_id(&self.attributes, interner, reference_prefix)
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
                $crate::leaf::write_relationship_id(
                    &mut self.attributes,
                    interner,
                    reference_prefix,
                    relationship_id,
                );
            }
        }
    };
}

/// Declares one `s:ST_Xstring` element: the struct, its text accessors, and its `FromXml`/`ToXml`
/// pair.
///
/// # Why these types are hand-written rather than `#[derive(FromXml, ToXml)]` with `#[xml(text)]`
///
/// `mjx-derive`'s `#[xml(text)]` grammar decodes character data on read and re-escapes it
/// **minimally** on write — only `<` and `&`. That is right for authoring and lossy for
/// preservation: a producer that wrote `&amp;amp;L` gets `&amp;L` back, and one that wrote
/// `&amp;#38;L` gets `&amp;L` too. Same string, different bytes — and a rebuilt text node that
/// differs from the original denies its element, *and every ancestor of it*, the verbatim source
/// range subtree copy-on-write would otherwise give it. `CLAUDE.md` records the gap; fixing it is a
/// foundation change across every text leaf and no work item owns it.
///
/// # Why it is a macro
///
/// Three types in this crate are this exact shape — [`DefinedName`](crate::DefinedName)'s content
/// (MJXOFF-100), [`HeaderFooterText`](crate::HeaderFooterText) (MJXOFF-129) and
/// [`CommentAuthor`](crate::CommentAuthor) (MJXOFF-114) — and the first two were written out
/// longhand before there was a third. The decode loop and the escape-on-authoring rule are the
/// whole of the fidelity contract for an `s:ST_Xstring`, so they are stated once here rather than
/// copied a third time and left free to drift.
///
/// The generated type keeps the element's name and prefix, its attribute vector verbatim (the
/// simple type permits no attribute at all, so an `xml:space` on one is a producer divergence to
/// preserve rather than an accessor to declare), its self-closing flag, and — until
/// `set_text` is called — the children the file wrote, entity spellings, CDATA sections and
/// interleaved comments included.
macro_rules! character_data_body {
    // A type that stands for **several** element names — `HeaderFooterText` is six — so there is no
    // one wire local to declare and no bare `new`. Its callers reach `with_local`.
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $crate::leaf::character_data_shape! { $(#[$meta])* $name }
    };
    (
        $(#[$meta:meta])*
        $name:ident, $local:literal
    ) => {
        $crate::leaf::character_data_shape! { $(#[$meta])* $name }

        impl $name {
            #[doc = concat!("The wire local name this type is written under: `", $local, "`.")]
            pub const WIRE_LOCAL: &'static str = $local;

            #[doc = concat!("Builds a new `", $local, "` holding `text`, bound to `prefix` — or to \
                the default namespace when `prefix` is `None`.")]
            #[must_use]
            #[allow(dead_code)]
            pub fn new(
                interner: &mut ::mjx_ooxml_core::Interner,
                prefix: ::core::option::Option<&str>,
                text: &str,
            ) -> Self {
                Self::with_local(interner, prefix, $local, text)
            }
        }
    };
}

/// The struct, the accessors and the `FromXml`/`ToXml` pair every `s:ST_Xstring` element shares —
/// the part that is identical whether or not the type stands for exactly one element name.
macro_rules! character_data_shape {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            name: ::mjx_ooxml_core::RawName,
            attributes: ::std::vec::Vec<::mjx_ooxml_core::RawAttribute>,
            empty: bool,
            /// The character data, decoded — what `text` answers with.
            text: ::std::string::String,
            /// The element's children exactly as the file wrote them, or `None` once the text has
            /// been replaced and there is nothing left to preserve.
            verbatim: ::core::option::Option<::std::vec::Vec<::mjx_ooxml_core::RawNode>>,
        }

        impl $name {
            /// Builds the element `local` names, bound to `prefix` or to the default namespace,
            /// holding `text`.
            ///
            /// `local` is a parameter because one of these types stands for **six** element names
            /// that differ in nothing but their position in a sequence.
            #[must_use]
            #[allow(dead_code)]
            pub(crate) fn with_local(
                interner: &mut ::mjx_ooxml_core::Interner,
                prefix: ::core::option::Option<&str>,
                local: &str,
                text: &str,
            ) -> Self {
                Self {
                    name: $crate::leaf::sml_name(interner, prefix, local),
                    attributes: ::std::vec::Vec::new(),
                    empty: text.is_empty(),
                    text: text.to_owned(),
                    verbatim: ::core::option::Option::None,
                }
            }

            /// The element's own qualified name, as the file wrote it.
            #[must_use]
            #[allow(dead_code)]
            pub fn element_name(&self) -> ::mjx_ooxml_core::RawName {
                self.name
            }

            /// The character data, with entity references decoded and **nothing else changed**.
            #[must_use]
            #[allow(dead_code)]
            pub fn text(&self) -> &str {
                &self.text
            }

            /// Replaces the whole string.
            ///
            /// This is the point at which the preserved character data is given up; the element's
            /// name, its attributes, their order and their quoting are untouched.
            #[allow(dead_code)]
            pub fn set_text(&mut self, text: impl ::core::convert::Into<::std::string::String>) {
                self.text = text.into();
                self.verbatim = ::core::option::Option::None;
                self.empty = false;
            }

            /// This element rebuilt as a [`RawElement`](::mjx_ooxml_core::RawElement), without an
            /// interner.
            ///
            /// An untouched value replays the children the file held. One that `set_text` has
            /// reached writes a single freshly escaped text node, which is what authoring should
            /// write.
            #[must_use]
            #[allow(dead_code)]
            pub fn as_raw_element(&self) -> ::mjx_ooxml_core::RawElement {
                let children = match &self.verbatim {
                    ::core::option::Option::Some(children) => children.clone(),
                    ::core::option::Option::None if self.text.is_empty() => ::std::vec::Vec::new(),
                    ::core::option::Option::None => ::std::vec![::mjx_ooxml_core::RawNode::Text(
                        ::mjx_xml::text::escape_text(&self.text).as_bytes().into(),
                    )],
                };
                let empty = self.empty && children.is_empty();
                ::mjx_ooxml_core::RawElement::rebuilt(
                    self.name,
                    self.attributes.clone(),
                    children,
                    empty,
                )
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
                    empty: element.empty,
                    text: $crate::leaf::decoded_character_data(element)?,
                    verbatim: ::core::option::Option::Some(element.children.clone()),
                })
            }
        }

        impl ::mjx_ooxml_core::ToXml for $name {
            fn to_xml(
                &self,
                _interner: &mut ::mjx_ooxml_core::Interner,
            ) -> ::mjx_ooxml_core::RawElement {
                self.as_raw_element()
            }
        }
    };
}

/// Every text and CDATA child of `element`, decoded and concatenated.
///
/// The one decode of an `s:ST_Xstring`'s content in this crate. Text nodes are unescaped; a CDATA
/// section's bytes are taken as they stand, because that is what a CDATA section means; every other
/// node — an element, a comment, a processing instruction — contributes nothing, which is what the
/// simple type says it can be.
///
/// # Errors
/// [`FromXmlError`](::mjx_ooxml_core::FromXmlError) if the character data is not UTF-8 or carries a
/// reference that will not decode.
pub(crate) fn decoded_character_data(
    element: &mjx_ooxml_core::RawElement,
) -> Result<String, mjx_ooxml_core::FromXmlError> {
    use mjx_ooxml_core::{FromXmlError, RawNode};

    let mut text = String::new();
    for child in &element.children {
        match child {
            RawNode::Text(bytes) => {
                let raw = core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                let decoded = mjx_xml::text::unescape_text(raw)
                    .map_err(|error| FromXmlError::InvalidEntity(error.to_string()))?;
                text.push_str(&decoded);
            }
            RawNode::CData(bytes) => {
                text.push_str(core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?);
            }
            _ => {}
        }
    }
    Ok(text)
}

/// The **displayed string** of a `CT_Rst` — the plain `t`, then each `r` run's `t`, concatenated.
///
/// A `CT_Rst` carries no character data of its own, so [`decoded_character_data`] answers the empty
/// string for one. This is the walk that answers what a reader means by "the text": the optional
/// plain `t` child, then the `t` inside each `r`, in document order.
///
/// **Phonetic runs are excluded.** An `rPh` is the *reading* printed above a run of kanji — kana the
/// author typed that is not part of the base string — so folding it in would interleave two texts.
/// `phoneticPr`, and any element the type does not declare, contribute nothing.
///
/// Children are matched by **local name**, prefix ignored, exactly as `crate::strings`'s reader
/// matches them: a `CT_Rst` is in the SpreadsheetML namespace wherever it appears, and a fragment
/// lifted out of a part may not re-declare it.
///
/// # Errors
/// [`FromXmlError`](::mjx_ooxml_core::FromXmlError) if the character data is not UTF-8 or carries a
/// reference that will not decode.
pub(crate) fn decoded_rich_text(
    element: &mjx_ooxml_core::RawElement,
    interner: &Interner,
) -> Result<String, mjx_ooxml_core::FromXmlError> {
    use mjx_ooxml_core::RawNode;

    let mut text = String::new();
    for child in &element.children {
        let RawNode::Element(child) = child else {
            continue;
        };
        match interner.resolve(child.name.local) {
            "t" => text.push_str(&decoded_character_data(child)?),
            "r" => {
                for run_child in &child.children {
                    let RawNode::Element(run_child) = run_child else {
                        continue;
                    };
                    if interner.resolve(run_child.name.local) == "t" {
                        text.push_str(&decoded_character_data(run_child)?);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(text)
}

pub(crate) use {
    attribute_bag, bag_body, bag_without_declared_attributes, character_data_body,
    character_data_shape, relationship_reference,
};
