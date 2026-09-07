//! A font file is untrusted input, exactly like the document that carried it.
//!
//! An embedded face arrives inside a `.pptx` a stranger sent; a fetched subset arrives over a
//! network; a system font directory is whatever the platform put there. So nothing on the path that
//! reads one may panic — not on truncation, not on a table pointing outside the file, not on an
//! arithmetic overflow, and not on an `unwrap` of a value the format does not guarantee.
//!
//! Two things are checked here. The first is behavioural: several thousand deliberately damaged
//! faces are parsed, and the suite passing at all *is* the assertion, because a panic fails the
//! test. The second is textual:
//! [`the_parse_path_contains_no_unwrap_expect_or_panic`] reads this crate's own sources and refuses
//! them if a panicking construct has appeared outside a test module — because the behavioural check
//! can only see the shapes of corruption it happened to generate, and the grep sees every line.

mod support;

use std::sync::Arc;

use mjx_text::{EmbeddedFont, EmbeddedFontEncoding, FontError, FontFace, ObfuscationKey};
use support::{bundled_face, bundled_font_directory};

/// Read one committed face's bytes.
fn face_bytes(file_name: &str) -> Vec<u8> {
    std::fs::read(bundled_font_directory().join(file_name)).expect("the committed face is readable")
}

/// A deterministic, seedable generator. A test that corrupts fonts at random and cannot be replayed
/// is a test that reports a failure nobody can reproduce.
struct Sequence(u64);

impl Sequence {
    fn next(&mut self) -> u64 {
        // SplitMix64: three lines, no dependency, and the same numbers on every platform.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next() % bound as u64).unwrap_or(0)
    }
}

/// Whatever comes back, read as much of it as the API allows — a parse that succeeds on damaged
/// bytes must not hand out a reader that then panics.
fn exercise(data: Vec<u8>) {
    let shared: Arc<[u8]> = Arc::from(data.as_slice());
    for index in [0_u32, 1, 7, u32::MAX] {
        let Ok(face) = FontFace::parse(Arc::clone(&shared), index) else {
            continue;
        };
        let metrics = *face.metrics();
        assert!(
            metrics.units_per_em >= 16,
            "a face that parsed must be scalable"
        );
        let _ = metrics.line_height();
        let _ = metrics.line_height_at_size(11.0);
        let _ = face.identity().family.len();
        let _ = face.colour_formats().any();
        let _ = face.variation_axes().len();
        let Ok(reader) = face.reader() else { continue };
        for character in ['A', 'a', ' ', '0', '\u{0}', '\u{10FFFF}', '漢', '\u{FFFD}'] {
            let _ = reader.advance_for_character(character);
            if let Some(glyph) = reader.glyph_for_character(character) {
                let _ = reader.bounding_box(glyph);
                let _ = reader.advance(glyph);
            }
        }
        let _ = reader.unshaped_advance("The quick brown fox 漢字 \u{0}");
    }
}

#[test]
fn hand_built_rubbish_is_refused_rather_than_read() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"x".to_vec(),
        b"not a font at all, just some prose".to_vec(),
        // A plausible sfnt header claiming a table count nothing backs.
        vec![0x00, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0],
        // `ttcf`, a collection header, with a face count and no faces.
        b"ttcf\x00\x01\x00\x00\x00\x00\x00\x99".to_vec(),
        // All zeroes, all ones, and a length that lands exactly on a header boundary.
        vec![0_u8; 12],
        vec![0xFF_u8; 4096],
    ];
    for case in cases {
        exercise(case);
    }
}

#[test]
fn a_face_truncated_anywhere_is_refused_rather_than_read_past_its_end() {
    for file_name in ["Caladea-Regular.ttf", "LiberationMono-Regular.ttf"] {
        let bytes = face_bytes(file_name);
        // Every boundary a table directory could plausibly straddle, plus a spread through the
        // body. 1 in 512 of a 320 KB face is several hundred truncations per file.
        let mut length = 0;
        while length < bytes.len() {
            exercise(bytes[..length].to_vec());
            length += 1 + length / 8 + 511;
        }
        // And the three shortest non-empty prefixes, which is where a header read would overrun.
        for length in 1..=64.min(bytes.len()) {
            exercise(bytes[..length].to_vec());
        }
    }
}

#[test]
fn a_face_with_bytes_flipped_anywhere_is_refused_or_read_safely() {
    let bytes = face_bytes("Caladea-Regular.ttf");
    let mut sequence = Sequence(0x4D4A_5854_4558_5400);

    for _ in 0..2_000 {
        let mut damaged = bytes.clone();
        // One to four flips per attempt, weighted towards the head of the file where the table
        // directory, `head`, `maxp` and `cmap` live — the offsets a reader dereferences.
        let flips = 1 + sequence.below(4);
        for _ in 0..flips {
            let position = if sequence.below(2) == 0 {
                sequence.below(4096.min(damaged.len()))
            } else {
                sequence.below(damaged.len())
            };
            #[allow(clippy::cast_possible_truncation)]
            let value = sequence.next() as u8;
            if let Some(byte) = damaged.get_mut(position) {
                *byte = value;
            }
        }
        exercise(damaged);
    }
}

#[test]
fn a_face_that_declares_an_impossible_em_square_is_refused() {
    // `head.unitsPerEm` is a 16-bit field at offset 18 of the `head` table, and the OpenType
    // specification permits 16 to 16384. A face declaring 0 would make every scaled metric a
    // division by zero, so it is refused at the door rather than divided by.
    let mut bytes = face_bytes("Caladea-Regular.ttf");
    let head = sfnt_table_offset(&bytes, b"head").expect("every face has a `head` table");

    for (implausible, label) in [(0_u16, "zero"), (1, "one"), (16_385, "16385")] {
        bytes[head + 18] = (implausible >> 8) as u8;
        bytes[head + 19] = (implausible & 0xFF) as u8;
        match FontFace::parse(Arc::from(bytes.as_slice()), 0) {
            Err(FontError::ImplausibleUnitsPerEm { units_per_em }) => {
                assert_eq!(units_per_em, implausible);
            }
            // `ttf-parser` refuses some of these itself, which is equally correct; what must not
            // happen is a face that parses and then divides by nothing.
            Err(FontError::MalformedFace { .. }) => {}
            other => panic!("an em square of {label} should be refused, and produced {other:?}"),
        }
    }
}

#[test]
fn a_face_index_beyond_the_file_is_refused_with_the_count_it_has() {
    let bytes = face_bytes("Caladea-Regular.ttf");
    assert_eq!(
        FontFace::face_count(&bytes),
        1,
        "a plain font holds one face"
    );
    match FontFace::parse(Arc::from(bytes.as_slice()), 4) {
        Err(FontError::FaceIndexOutOfRange { index, available }) => {
            assert_eq!((index, available), (4, 1));
        }
        other => panic!("face 4 of a single-face file should be refused, and produced {other:?}"),
    }
}

#[test]
fn an_embedded_face_with_a_broken_key_or_broken_bytes_is_refused() {
    let key = ObfuscationKey::parse("{01234567-89AB-CDEF-0123-456789ABCDEF}").expect("a GUID");

    // Obfuscated rubbish: the mask lifts, and what is underneath is still not a face.
    let embedded = EmbeddedFont {
        declared_family: "Whatever".to_owned(),
        weight: mjx_text::FontWeight::REGULAR,
        slant: mjx_text::FontSlant::Upright,
        encoding: EmbeddedFontEncoding::Obfuscated(key),
        data: vec![0x5A; 4096],
    };
    assert!(matches!(
        embedded.decode(),
        Err(FontError::MalformedFace { .. })
    ));

    // Too short to even hold the mask.
    let stub = EmbeddedFont {
        data: vec![0x00; 8],
        ..embedded.clone()
    };
    assert!(matches!(
        stub.decode(),
        Err(FontError::ObfuscatedFontTooShort { length: 8 })
    ));

    // And the round trip a real document goes through: mask a genuine face, then decode it, and get
    // the same face back.
    let genuine = face_bytes("Caladea-Regular.ttf");
    let mut masked = genuine.clone();
    key.apply(&mut masked)
        .expect("a real face is longer than 32 bytes");
    assert_ne!(masked[..32], genuine[..32]);
    let carried = EmbeddedFont {
        data: masked,
        ..embedded
    };
    let decoded = carried
        .decode()
        .expect("the mask lifts and the face parses");
    assert_eq!(decoded.identity().family, "Caladea");
    assert_eq!(
        decoded.metrics().units_per_em,
        bundled_face("Caladea-Regular.ttf").metrics().units_per_em
    );
}

/// The textual half of the rule, over this crate's own sources.
///
/// The behavioural suites above can only see the corruptions they happened to generate. This one
/// sees every line, and it is what stops the next person adding a convenient `.unwrap()` to a path
/// that reads a stranger's font.
///
/// # It proves the absence of a token, not the absence of a panic
///
/// A source grep cannot see a panic reached through a callee, an arithmetic overflow, or a slice
/// index. Where a panic is genuinely possible — and shaping does real arithmetic over tables that
/// came out of an untrusted file — the instrument is execution over hostile input, which is
/// `tests/untrusted_text.rs`. This test is the cheap half, not the whole of the rule.
///
/// # The walk is recursive on purpose (MJXOFF-158)
///
/// It used to read only the top level of `src/`, and the floor below it was `>= 9` against a crate
/// that had ten flat files. A crate laid out as `src/shaping/mod.rs` would therefore have been
/// green over code the walk never opened, and the floor would still have passed on the files it
/// could see. That is a gate that reports success for work it did not check, so the walk descends
/// and the floor is the crate's real file count.
#[test]
fn the_parse_path_contains_no_unwrap_expect_or_panic() {
    const FORBIDDEN: &[&str] = &[
        ".unwrap()",
        ".expect(",
        "panic!(",
        "unreachable!(",
        "todo!(",
        "unimplemented!(",
        "debug_assert",
        "assert!(",
        "assert_eq!(",
    ];

    let source_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    collect_rust_sources(&source_directory, &mut sources);
    sources.sort();

    let mut files_scanned = 0_usize;
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("a source file is readable");
        // Test modules are the last item in each file by convention, and they are allowed to
        // assert — that is what a test is. Everything above the marker is library code.
        let library = match text.find("#[cfg(test)]") {
            Some(position) => &text[..position],
            None => &text[..],
        };
        files_scanned += 1;

        for (number, line) in library.lines().enumerate() {
            // A doc comment may name the construct it forbids.
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for needle in FORBIDDEN {
                assert!(
                    !code.contains(needle),
                    "{}:{} uses `{needle}` outside a test module. A font file is untrusted input, \
                     and this crate returns a `FontError` instead of panicking on one:\n    {code}",
                    path.display(),
                    number + 1,
                );
            }
        }
    }
    // The crate's real file count, raised with the crate. A floor lower than the truth is a floor
    // nobody is standing on: it stays green when the walk stops reaching files, which is exactly
    // the failure this number exists to catch. Nineteen at MJXOFF-158; twenty-two at MJXOFF-159,
    // which added `raster.rs`, `placement.rs` and `atlas.rs`; twenty-three at MJXOFF-164, which
    // added `subset.rs`. Raise it when a module is added, and do not lower it when one is removed
    // without saying why.
    //
    // `subset.rs` is squarely in this gate's scope and not an exception to it: cutting a face down
    // to a glyph set is **table surgery on untrusted bytes** — every offset it reads comes out of a
    // font file — so a slice index or an `unwrap` there is exactly the defect this file refuses. It
    // reads every table through bounds-checked helpers and answers the whole face when it meets one
    // it cannot cut.
    const SOURCE_FILE_COUNT: usize = 23;
    assert_eq!(
        files_scanned, SOURCE_FILE_COUNT,
        "{files_scanned} source files were scanned and this crate has {SOURCE_FILE_COUNT} — either \
         the walk is not reaching them all, or a module was added or removed and this number was \
         not"
    );
}

/// Every `.rs` file under `directory`, at any depth.
///
/// Recursive so that the grep above is not a constraint on how the crate is organised: a module in
/// `src/shaping/mod.rs` must be read like one in `src/shaping.rs`.
fn collect_rust_sources(directory: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
    let listing = std::fs::read_dir(directory).expect("a source directory is readable");
    for entry in listing {
        let path = entry.expect("the directory listing is readable").path();
        if path.is_dir() {
            collect_rust_sources(&path, into);
        } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs") {
            into.push(path);
        }
    }
}

/// Find a table's offset in an sfnt file, so the tests above can damage a specific field.
fn sfnt_table_offset(data: &[u8], tag: &[u8; 4]) -> Option<usize> {
    let table_count = usize::from(u16::from_be_bytes([*data.get(4)?, *data.get(5)?]));
    for index in 0..table_count {
        let record = 12 + index * 16;
        let entry = data.get(record..record + 16)?;
        if &entry[..4] == tag {
            let offset = u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]);
            return usize::try_from(offset).ok();
        }
    }
    None
}
