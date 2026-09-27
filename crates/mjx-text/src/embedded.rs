//! Faces the document carries with it.
//!
//! A `.pptx` and a `.docx` can both embed the faces they were authored in, and a document that does
//! so should be drawn in them — it is the only tier that is guaranteed to be what the author saw.
//! So embedded faces are consulted *before* the system, before the bundle and before any
//! substitution.
//!
//! # Obfuscation
//!
//! Word writes an embedded face in an obfuscated form, and so does XPS: the first 32 bytes are
//! masked with a key derived from a GUID stored beside the part. It is not encryption and is not
//! described as such — it exists so that a font cannot be extracted by renaming the file — but the
//! bytes will not parse until the mask is lifted. PowerPoint writes the face unmasked.
//!
//! This module knows the transform and nothing about the packages it appears in: the key arrives as
//! a string and the bytes arrive as bytes. Which relationship a part came from, and what a
//! `p:embeddedFont` element looks like, belongs to the format crates far above this one.

use std::sync::Arc;

use crate::error::FontError;
use crate::face::{FontFace, FontSlant, FontWeight};

/// The 32-byte prefix an obfuscation key masks, and the 16-byte key that masks it.
const OBFUSCATED_PREFIX: usize = 32;
const KEY_BYTES: usize = 16;

/// A font obfuscation key, as it appears beside an embedded font part.
///
/// Written as a GUID: 32 hexadecimal digits, optionally braced and hyphenated. The mask is the
/// GUID's sixteen bytes **in reverse order**, applied to the font's first thirty-two bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ObfuscationKey {
    mask: [u8; KEY_BYTES],
}

impl ObfuscationKey {
    /// Read a key from its written form.
    ///
    /// # Errors
    ///
    /// [`FontError::MalformedObfuscationKey`] when the text does not hold exactly 32 hexadecimal
    /// digits once braces and hyphens are dropped.
    pub fn parse(written: &str) -> Result<Self, FontError> {
        let digits: Vec<u8> = written
            .bytes()
            .filter(|byte| !matches!(byte, b'{' | b'}' | b'-'))
            .collect();
        if digits.len() != KEY_BYTES * 2 {
            return Err(FontError::MalformedObfuscationKey {
                key: written.to_owned(),
            });
        }

        let mut mask = [0_u8; KEY_BYTES];
        // The length was checked to be exactly `KEY_BYTES * 2` above, so the remainder is empty by
        // construction and is dropped rather than asserted about: an assertion here would be a
        // panic on a path that reads untrusted bytes.
        let (pairs, _remainder) = digits.as_chunks::<2>();
        for (position, pair) in pairs.iter().enumerate() {
            let (Some(high), Some(low)) = (hexadecimal(pair[0]), hexadecimal(pair[1])) else {
                return Err(FontError::MalformedObfuscationKey {
                    key: written.to_owned(),
                });
            };
            // Reversed: the key's last byte masks the font's first.
            mask[KEY_BYTES - 1 - position] = (high << 4) | low;
        }
        Ok(Self { mask })
    }

    /// Lift the mask, in place.
    ///
    /// The transform is its own inverse, so applying it to an unobfuscated face obfuscates it —
    /// which is exactly what writing one back out needs.
    ///
    /// # Errors
    ///
    /// [`FontError::ObfuscatedFontTooShort`] when there are fewer than 32 bytes to unmask.
    pub fn apply(&self, data: &mut [u8]) -> Result<(), FontError> {
        if data.len() < OBFUSCATED_PREFIX {
            return Err(FontError::ObfuscatedFontTooShort { length: data.len() });
        }
        for (position, byte) in data.iter_mut().take(OBFUSCATED_PREFIX).enumerate() {
            *byte ^= self.mask[position % KEY_BYTES];
        }
        Ok(())
    }
}

fn hexadecimal(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// How an embedded face's bytes arrived.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EmbeddedFontEncoding {
    /// Exactly as a font file — what PowerPoint writes.
    Plain,
    /// Masked with a key — what Word and XPS write.
    Obfuscated(ObfuscationKey),
}

/// One face a document carries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EmbeddedFont {
    /// The family the document says these bytes are.
    ///
    /// It is recorded because it is what the document's *runs* name, and a face's own `name` table
    /// occasionally disagrees with the package that embedded it. The resolver indexes the face
    /// under both.
    pub declared_family: String,
    /// The weight the document says these bytes are.
    pub weight: FontWeight,
    /// The slant the document says these bytes are.
    pub slant: FontSlant,
    /// How the bytes are encoded.
    pub encoding: EmbeddedFontEncoding,
    /// The bytes themselves, as they sit in the package.
    pub data: Vec<u8>,
}

impl EmbeddedFont {
    /// Decode the bytes and parse the face.
    ///
    /// # Errors
    ///
    /// [`FontError::ObfuscatedFontTooShort`] when a masked face is shorter than its mask, and
    /// whatever [`FontFace::parse`] returns for bytes that are not a face. An embedded font is
    /// untrusted input like everything else here, so a document that carries rubbish gets an error
    /// and the resolver moves on to the next tier.
    pub fn decode(&self) -> Result<FontFace, FontError> {
        let data: Arc<[u8]> = match &self.encoding {
            EmbeddedFontEncoding::Plain => Arc::from(self.data.as_slice()),
            EmbeddedFontEncoding::Obfuscated(key) => {
                let mut bytes = self.data.clone();
                key.apply(&mut bytes)?;
                Arc::from(bytes.as_slice())
            }
        };
        FontFace::parse(data, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_read_in_reverse_byte_order() {
        let key = ObfuscationKey::parse("{000102030405060708090A0B0C0D0E0F}")
            .expect("that is a well-formed key");
        // The GUID's bytes are 00,01,…,0F; reversed, the first mask byte is 0x0F.
        assert_eq!(key.mask[0], 0x0F);
        assert_eq!(key.mask[15], 0x00);
    }

    #[test]
    fn braces_and_hyphens_are_optional_and_case_does_not_matter() {
        let braced = ObfuscationKey::parse("{01234567-89ab-cdef-0123-456789abcdef}")
            .expect("braced and hyphenated");
        let bare = ObfuscationKey::parse("0123456789ABCDEF0123456789ABCDEF").expect("bare");
        assert_eq!(braced, bare);
    }

    #[test]
    fn a_key_that_is_not_a_guid_is_refused_rather_than_padded() {
        for written in [
            "",
            "{}",
            "not-a-guid",
            "0123456789ABCDEF",
            "zzzz5678-89ab-cdef-0123-456789abcdef",
        ] {
            assert!(
                ObfuscationKey::parse(written).is_err(),
                "`{written}` should not parse as an obfuscation key"
            );
        }
    }

    #[test]
    fn the_mask_is_its_own_inverse() {
        let key = ObfuscationKey::parse("{01234567-89AB-CDEF-0123-456789ABCDEF}").expect("valid");
        let original: Vec<u8> = (0..64_u8).collect();
        let mut bytes = original.clone();
        key.apply(&mut bytes).expect("64 bytes is enough");
        assert_ne!(
            bytes[..32],
            original[..32],
            "the prefix should have changed"
        );
        assert_eq!(
            bytes[32..],
            original[32..],
            "nothing past 32 bytes is touched"
        );
        key.apply(&mut bytes).expect("still 64 bytes");
        assert_eq!(bytes, original);
    }

    #[test]
    fn a_face_shorter_than_the_mask_is_refused_rather_than_read_past() {
        let key = ObfuscationKey::parse("{01234567-89AB-CDEF-0123-456789ABCDEF}").expect("valid");
        let mut bytes = vec![0_u8; 31];
        assert!(matches!(
            key.apply(&mut bytes),
            Err(FontError::ObfuscatedFontTooShort { length: 31 })
        ));
    }
}
