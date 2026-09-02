//! The error a VML read can produce.

use mjx_ooxml_core::FromXmlError;
use mjx_xml::XmlError;

/// What can go wrong reading a VML drawing part.
///
/// VML parts are attacker-controlled input like any other part of a package, so nothing here panics:
/// a malformed part is a typed error, and a *well-formed* part carrying markup this crate does not
/// model is not an error at all — it rides through the `Raw` bucket.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VmlError {
    /// The part is not well-formed XML.
    #[error("VML drawing is not well-formed XML: {0}")]
    Xml(#[from] XmlError),

    /// Content this crate models is malformed — text that is not UTF-8, or an entity it cannot
    /// decode.
    #[error("VML drawing could not be modeled: {0}")]
    Model(#[from] FromXmlError),
}
