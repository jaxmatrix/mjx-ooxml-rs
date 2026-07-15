//! Conversion between the raw preservation tree and typed models: the [`FromXml`] / [`ToXml`] traits.
//!
//! A typed model (e.g. a DrawingML text body) is a *view* over one element of a [`RawElement`] tree.
//! [`FromXml`] parses such a view out of an element; [`ToXml`] rebuilds an element from it. Together
//! they are the seam every format crate implements (by hand at first, and later via the `mjx-derive`
//! proc-macro, which generates exactly these impls).
//!
//! # The interner invariant
//!
//! Every name in the tree ([`RawName`]) and every attribute ([`crate::RawAttribute`]) holds
//! [`Symbol`]s, and **a `Symbol` is only meaningful in the [`Interner`] that produced it** (there is
//! one interner per part, owned by the [`crate::RawDocument`]). A typed model therefore stores the
//! `Symbol`s it read verbatim and reuses them on the way out. Two consequences:
//!
//! - [`FromXml::from_xml`] borrows the part's interner (to resolve names for matching) and copies /
//!   clones the pieces it preserves — it never needs to intern.
//! - [`ToXml::to_xml`] is handed `&mut Interner` so it can intern any *newly introduced* strings, but
//!   the [`RawElement`] it returns is only serializable within a [`crate::RawDocument`] that uses **the
//!   same interner the value was parsed from**. Serializing it against a different interner would
//!   resolve its `Symbol`s to the wrong strings (or panic).
//!
//! [`Symbol`]: crate::Symbol
//! [`Interner`]: crate::Interner
//! [`RawName`]: crate::RawName

use crate::intern::Interner;
use crate::raw::RawElement;

/// Parses a typed value out of a [`RawElement`].
///
/// The caller decides *which* element to hand to which type — `from_xml` does **not** validate the
/// element's own name or namespace (the same complex type can appear under different tags/prefixes;
/// for example a DrawingML `CT_TextBody` is serialized as `p:txBody` inside a slide but `a:txBody`
/// elsewhere). Implementations match and recurse into *children* by `(namespace, local name)`, and
/// preserve everything they do not model so the value can round-trip.
///
/// See the [module docs](self) for the interner invariant governing the `interner` argument.
pub trait FromXml: Sized {
    /// Builds `Self` from `element`, resolving names through `interner`.
    ///
    /// # Errors
    /// Returns [`FromXmlError`] if content the type *does* model is malformed — e.g. text that is not
    /// valid UTF-8, or an entity reference that cannot be decoded.
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError>;
}

/// Rebuilds a [`RawElement`] from a typed value.
///
/// This is infallible: a well-formed typed value always reconstructs a well-formed element (names are
/// reused, text is escaped totally). The returned element must be serialized within a
/// [`crate::RawDocument`] that uses the **same interner** the value was parsed from — see the
/// [module docs](self).
pub trait ToXml {
    /// Serializes `self` into a [`RawElement`], interning any newly introduced strings into `interner`.
    fn to_xml(&self, interner: &mut Interner) -> RawElement;
}

/// An error produced while parsing a typed value with [`FromXml::from_xml`].
///
/// Non-exhaustive: later phases (attribute typing, required-child validation) add variants without a
/// breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FromXmlError {
    /// Text or an attribute value was not valid UTF-8.
    InvalidUtf8,
    /// An entity or character reference in text content could not be decoded. The payload is a
    /// human-readable description of the offending reference.
    InvalidEntity(String),
}

impl core::fmt::Display for FromXmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidUtf8 => f.write_str("text or attribute value was not valid UTF-8"),
            Self::InvalidEntity(detail) => {
                write!(f, "could not decode an entity reference: {detail}")
            }
        }
    }
}

impl std::error::Error for FromXmlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_non_empty_and_is_std_error() {
        for err in [
            FromXmlError::InvalidUtf8,
            FromXmlError::InvalidEntity("&bogus;".to_owned()),
        ] {
            assert!(!err.to_string().is_empty(), "empty Display for {err:?}");
            // Coerces to the std error trait (proves the hand-written impl compiles).
            let _dyn: &dyn std::error::Error = &err;
        }
        assert!(FromXmlError::InvalidEntity("&bogus;".to_owned())
            .to_string()
            .contains("&bogus;"));
    }
}
