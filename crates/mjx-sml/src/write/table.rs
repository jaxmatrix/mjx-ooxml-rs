//! `xl/tables/tableN.xml`, authored: a whole table part written from a description.
//!
//! # The rule this file is written to
//!
//! **A part authored on demand writes back a root that was *read*, never one freshly constructed.**
//! `crates/mjx-xlsx/src/blank.rs`'s module documentation states it in full, with the defect that
//! taught it: `mjx-docx`'s `create_footnotes_part` wrote a fresh value over a parsed root, the fresh
//! value had no ancestor to inherit its namespace declaration from, the declaration was dropped, and
//! every footnote vanished on the next open — with a green gate throughout, because the gate
//! asserted on the model rather than on the file that came back.
//!
//! So [`AuthoredTable::from_spec`] writes `<table xmlns="…"/>` as **bytes**, parses them, reads a
//! [`WorksheetTable`] out of the parsed root, fills it in, and returns it through
//! [`ToXml::write_back`] — which keeps the namespace declaration, because it lives in the root's
//! attribute vector and an attribute vector is never rebuilt.
//!
//! This is [`AuthoredStylesheet`](super::stylesheet::AuthoredStylesheet)'s shape exactly, for
//! exactly that reason.
//!
//! # The seed is not schema-valid, and that is fine
//!
//! `<table/>` declares none of `CT_Table`'s three required attributes and none of its one required
//! child. It is never written: [`from_spec`](AuthoredTable::from_spec) fills all four in before
//! anything can ask for bytes, and the constructor is private to that path. The alternative — a seed
//! carrying invented values for `@id`, `@displayName` and `@ref` — would be a table this crate made
//! up, briefly real, and reachable if anything ever serialized between the two steps.

use mjx_ooxml_core::{Interner, RawDocument, ToXml};
use mjx_ooxml_types::namespaces::SML;

use crate::error::SmlError;
use crate::features::table_specs::WorksheetTableSpec;
use crate::features::tables::WorksheetTable;

use super::constants::XML_DECLARATION;

/// A table part under construction: the parsed part, and the interner its names live in.
///
/// Owns a [`RawDocument`] rather than a bare [`Interner`] because the model is a *view* over that
/// document's root — see this module's own documentation for why that indirection is the point and
/// not an accident.
#[derive(Debug)]
pub struct AuthoredTable {
    document: RawDocument,
    part: WorksheetTable,
}

impl AuthoredTable {
    /// The bytes a table part is seeded from: the declaration, and an empty `table` carrying the one
    /// namespace declaration everything in it inherits.
    fn seed_bytes() -> Vec<u8> {
        format!(r#"{XML_DECLARATION}<table xmlns="{}"/>"#, SML.transitional).into_bytes()
    }

    /// Builds the whole part from `spec`.
    ///
    /// # Errors
    /// [`SmlError::TableGeometryDoesNotFit`] or [`SmlError::TableHasNoColumns`] from
    /// [`WorksheetTableSpec::build`]; [`SmlError::Xml`] if the seed does not parse,
    /// [`SmlError::Model`] if it does not match `CT_Table`, or
    /// [`SmlError::AuthoredPartSeedRejected`] if its root is not a `table`. The last three are
    /// unreachable — the seed is a literal in this file — and returned rather than unwrapped because
    /// a library path does not panic on anything.
    pub fn from_spec(spec: &WorksheetTableSpec) -> Result<Self, SmlError> {
        let mut document = mjx_xml::fidelity::parse(&Self::seed_bytes())?;
        let mut part = WorksheetTable::read_root(&document.root, &document.interner)?.ok_or(
            SmlError::AuthoredPartSeedRejected {
                part: "/xl/tables/tableN.xml",
            },
        )?;
        // Written **onto the element read out of the seed**, never over a fresh one: the seed's
        // `xmlns` lives in that element's attribute vector, and an attribute vector is never
        // rebuilt. See this module's own documentation.
        spec.apply_to(&mut part, &mut document.interner, None)?;
        Ok(Self { document, part })
    }

    /// The table markup, for a caller reading back what was authored.
    #[must_use]
    pub fn part(&self) -> &WorksheetTable {
        &self.part
    }

    /// The table markup, mutably — for anything [`WorksheetTableSpec`] does not describe.
    pub fn part_mut(&mut self) -> &mut WorksheetTable {
        &mut self.part
    }

    /// The interner every name in [`part`](Self::part) is interned in.
    #[must_use]
    pub fn interner(&self) -> &Interner {
        &self.document.interner
    }

    /// The interner, mutably — what a caller building a child element of the table needs.
    pub fn interner_mut(&mut self) -> &mut Interner {
        &mut self.document.interner
    }

    /// Writes the model back over the root it was read from, and serializes the whole part into
    /// `out`.
    pub fn write_into(&mut self, out: &mut Vec<u8>) {
        let RawDocument { interner, root, .. } = &mut self.document;
        self.part.write_back(root, interner);
        mjx_xml::fidelity::serialize(&self.document, out);
    }

    /// The whole part as bytes. See [`write_into`](Self::write_into).
    #[must_use]
    pub fn to_part_bytes(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_into(&mut out);
        out
    }
}
