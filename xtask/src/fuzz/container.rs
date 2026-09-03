//! A minimal, deliberately dishonest ZIP builder — the OPC target's structure-aware generator.
//!
//! # Why the campaign needs one
//!
//! This is the answer to the trap the ticket names: *a campaign that ran and found nothing*. Flip
//! bytes in a `.pptx` at random and essentially every mutant fails in `zip`'s central-directory
//! reader, so the campaign spends its whole budget re-deriving that a corrupt ZIP is a corrupt ZIP
//! and never once reaches `[Content_Types].xml`, part-name resolution, or the relationship graph —
//! the code that is actually ours. A generator that emits *well-formed containers with hostile
//! contents* is what puts those paths under test.
//!
//! # Why it is dishonest on purpose
//!
//! Every field a reader is tempted to trust is settable independently of the bytes it describes:
//! the declared uncompressed size, the declared compressed size and the CRC. A ZIP reader that
//! pre-allocates from a declared size, or sizes a buffer from a header rather than from data that
//! has actually arrived, is exactly the shape of defect this target exists to find, and it is not
//! expressible by mutating a real container's compressed bytes.
//!
//! Entries are always *stored* (method 0). Compression is not the subject here, and a hand-written
//! deflate encoder would add a second implementation of something `mjx-opc` already owns. The one
//! thing store cannot express — a genuine compression bomb — the driver builds instead by asking
//! `mjx-opc` itself to write a package whose part is highly compressible, which is both simpler and
//! a more faithful reproduction of what a bomb in the wild looks like.
//!
//! No dependency is added: `zip` is `mjx-opc`'s backend and the workspace keeps it there, so this
//! writes the bytes itself rather than becoming the ZIP backend's second consumer.

/// One entry to write, with every field a reader might trust made independently settable.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The entry name, exactly as it goes into the container (no validation, no normalisation).
    pub name: String,
    /// The bytes actually stored.
    pub data: Vec<u8>,
    /// The uncompressed size to *declare*, if it should differ from `data.len()`.
    pub declared_uncompressed_size: Option<u32>,
    /// The compressed size to *declare*, if it should differ from `data.len()`.
    pub declared_compressed_size: Option<u32>,
    /// The CRC to declare, if it should differ from the true one.
    pub declared_crc: Option<u32>,
    /// The compression method to declare. `0` is store, which is what the bytes actually are.
    pub method: u16,
}

impl Entry {
    /// An honest stored entry: every declared field matches the bytes.
    pub fn stored(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data,
            declared_uncompressed_size: None,
            declared_compressed_size: None,
            declared_crc: None,
            method: 0,
        }
    }
}

/// Serializes `entries` into container bytes.
///
/// The central directory is written in the same order as the local headers, with each entry's local
/// header offset recorded correctly — a reader that gets past the directory then meets whatever
/// inconsistency the entries were built with, which is where the interesting behaviour is.
#[must_use]
pub fn build(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for entry in entries {
        let offset = u32::try_from(out.len()).unwrap_or(u32::MAX);
        let crc = entry.declared_crc.unwrap_or_else(|| crc32(&entry.data));
        let stored = u32::try_from(entry.data.len()).unwrap_or(u32::MAX);
        let compressed = entry.declared_compressed_size.unwrap_or(stored);
        let uncompressed = entry.declared_uncompressed_size.unwrap_or(stored);
        let name = entry.name.as_bytes();
        let name_len = u16::try_from(name.len()).unwrap_or(u16::MAX);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&entry.method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // modification time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // modification date (1980-01-01)
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&compressed.to_le_bytes());
        out.extend_from_slice(&uncompressed.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra field length
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.data);

        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes()); // version made by
        directory.extend_from_slice(&20u16.to_le_bytes()); // version needed
        directory.extend_from_slice(&0u16.to_le_bytes()); // flags
        directory.extend_from_slice(&entry.method.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0x21u16.to_le_bytes());
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&compressed.to_le_bytes());
        directory.extend_from_slice(&uncompressed.to_le_bytes());
        directory.extend_from_slice(&name_len.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes()); // extra
        directory.extend_from_slice(&0u16.to_le_bytes()); // comment
        directory.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        directory.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        directory.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }

    let directory_offset = u32::try_from(out.len()).unwrap_or(u32::MAX);
    let directory_size = u32::try_from(directory.len()).unwrap_or(u32::MAX);
    let count = u16::try_from(entries.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // this disk
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with the directory
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    out
}

/// CRC-32 (IEEE), computed bit by bit.
///
/// A table would be faster; the campaign spends its time inside the code under test rather than
/// here, and a table is 1 KiB of state to get wrong.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::{build, crc32, Entry};

    #[test]
    fn the_crc_matches_the_published_check_value() {
        // The CRC-32 check value from the algorithm's own specification.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn an_honest_container_is_one_mjx_opc_can_open() {
        // If this ever failed, every "structured" OPC mutant would be rejected by the ZIP layer and
        // the target would silently degrade to the useless one this module exists to replace.
        let entries = vec![
            Entry::stored(
                "[Content_Types].xml",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/></Types>"#.to_vec(),
            ),
            Entry::stored(
                "_rels/.rels",
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#.to_vec(),
            ),
        ];
        let package = mjx_opc::Package::open(&build(&entries))
            .expect("the builder must emit a container mjx-opc can open");
        assert_eq!(package.entries().len(), 2);
    }

    #[test]
    fn a_declared_size_is_independent_of_the_bytes() {
        let mut entry = Entry::stored("a.xml", b"<a/>".to_vec());
        entry.declared_uncompressed_size = Some(1_000_000);
        let bytes = build(&[entry]);
        // The container is small; only the header claims otherwise. That gap is the target.
        assert!(bytes.len() < 200);
    }
}
