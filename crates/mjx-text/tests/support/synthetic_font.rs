//! Fonts built byte by byte, for the shaping cases the bundled faces cannot reach.
//!
//! # Why not a real Arabic or Devanagari face
//!
//! `assets/fonts/` holds five metric-compatible Latin faces. Nothing there has an Arabic joining
//! form or a Devanagari matra, so the two shaping assertions that a naive implementation most
//! obviously fails — glyph ids that differ by position, and a cluster whose glyphs come out in a
//! different order from its characters — have no committed input.
//!
//! The alternatives were a system font (not present in CI, and a test that silently skips is the
//! vacuous gate this programme keeps finding) or committing two more font binaries with their
//! licences and provenance, which MJXOFF-157 already escalated as a repository-owner decision.
//! Building the font here is neither: it is a few hundred bytes of table, it is deterministic, and
//! the expected glyph ids are **chosen**, so an assertion about them is an assertion about what the
//! shaper did rather than about what some vendor's font happens to contain.
//!
//! These fonts were checked against the real thing before being trusted. `NotoNaskhArabic-Regular`
//! shapes `بب` to the isolated form's *neighbours* — a different glyph per position — and
//! `NotoSansDevanagari-Regular` shapes `कि` with the matra first; the fonts below reproduce both
//! behaviours exactly, which is what makes them a stand-in for a real face rather than a mock of one.
//!
//! # What is built
//!
//! The minimum a shaper reads: `head`, `hhea`, `maxp`, `hmtx`, `cmap` (format 12, so no segment
//! arithmetic), and optionally `GSUB` with single-substitution lookups under one script. No
//! outlines: shaping never reads them, and R04's rasteriser is not what this exercises.

#![allow(dead_code)]

use std::sync::Arc;

use mjx_text::FontFace;

/// The em square every synthetic face uses unless one is asked for explicitly.
pub(crate) const DEFAULT_UNITS_PER_EM: u16 = 1000;

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// One `SingleSubstFormat1` lookup, reached by one feature.
///
/// Every glyph in `coverage` is replaced by itself plus `delta`, which is all the Arabic joining
/// forms need: one lookup per position, each moving the base glyph somewhere different.
pub(crate) struct SingleSubstitution {
    /// The four-character feature tag that reaches this lookup — `isol`, `init`, `medi`, `fina`.
    pub(crate) feature: [u8; 4],
    /// The glyphs it applies to.
    pub(crate) coverage: Vec<u16>,
    /// What is added to each of them.
    pub(crate) delta: i16,
}

/// A face to be built: its character map, its advances, and optionally a `GSUB`.
pub(crate) struct SyntheticFace {
    units_per_em: u16,
    character_map: Vec<(u32, u16)>,
    advances: Vec<u16>,
    script: Option<[u8; 4]>,
    lookups: Vec<SingleSubstitution>,
}

impl SyntheticFace {
    /// A face with `advances.len()` glyphs, whose advances are `advances` in glyph order.
    pub(crate) fn new(advances: Vec<u16>) -> Self {
        Self {
            units_per_em: DEFAULT_UNITS_PER_EM,
            character_map: Vec::new(),
            advances,
            script: None,
            lookups: Vec::new(),
        }
    }

    /// The same face at a different em square. Used to prove that two advances which are equal
    /// *fractions of an em* compare equal across different denominators.
    pub(crate) fn with_units_per_em(mut self, units_per_em: u16) -> Self {
        self.units_per_em = units_per_em;
        self
    }

    /// Map `character` to `glyph`.
    pub(crate) fn mapping(mut self, character: char, glyph: u16) -> Self {
        self.character_map.push((character as u32, glyph));
        self
    }

    /// Give the face a `GSUB` under `script`, with one lookup per entry of `lookups`.
    pub(crate) fn with_substitutions(
        mut self,
        script: [u8; 4],
        lookups: Vec<SingleSubstitution>,
    ) -> Self {
        self.script = Some(script);
        self.lookups = lookups;
        self
    }

    /// The font file's bytes.
    pub(crate) fn build(&self) -> Vec<u8> {
        let glyph_count = u16::try_from(self.advances.len())
            .expect("a synthetic face has fewer than 65536 glyphs");

        let mut head = Vec::new();
        push_u32(&mut head, 0x0001_0000); // version
        push_u32(&mut head, 0x0001_0000); // fontRevision
        push_u32(&mut head, 0); // checkSumAdjustment
        push_u32(&mut head, 0x5F0F_3CF5); // magicNumber
        push_u16(&mut head, 0b11); // flags
        push_u16(&mut head, self.units_per_em);
        push_u32(&mut head, 0); // created, high
        push_u32(&mut head, 0); // created, low
        push_u32(&mut head, 0); // modified, high
        push_u32(&mut head, 0); // modified, low
        push_i16(&mut head, 0); // xMin
        push_i16(&mut head, -200); // yMin
        push_i16(&mut head, 1000); // xMax
        push_i16(&mut head, 800); // yMax
        push_u16(&mut head, 0); // macStyle
        push_u16(&mut head, 8); // lowestRecPPEM
        push_i16(&mut head, 2); // fontDirectionHint
        push_i16(&mut head, 0); // indexToLocFormat
        push_i16(&mut head, 0); // glyphDataFormat

        let ascender = i16::try_from(i32::from(self.units_per_em) * 4 / 5).unwrap_or(800);
        let descender = -i16::try_from(i32::from(self.units_per_em) / 5).unwrap_or(200);
        let mut hhea = Vec::new();
        push_u32(&mut hhea, 0x0001_0000);
        push_i16(&mut hhea, ascender);
        push_i16(&mut hhea, descender);
        push_i16(&mut hhea, 0); // lineGap
        push_u16(&mut hhea, self.advances.iter().copied().max().unwrap_or(0));
        push_i16(&mut hhea, 0); // minLeftSideBearing
        push_i16(&mut hhea, 0); // minRightSideBearing
        push_i16(&mut hhea, 1000); // xMaxExtent
        push_i16(&mut hhea, 1); // caretSlopeRise
        push_i16(&mut hhea, 0); // caretSlopeRun
        push_i16(&mut hhea, 0); // caretOffset
        for _ in 0..4 {
            push_i16(&mut hhea, 0); // reserved
        }
        push_i16(&mut hhea, 0); // metricDataFormat
        push_u16(&mut hhea, glyph_count); // numberOfHMetrics

        let mut maxp = Vec::new();
        push_u32(&mut maxp, 0x0000_5000); // version 0.5 — no `glyf`
        push_u16(&mut maxp, glyph_count);

        let mut hmtx = Vec::new();
        for advance in &self.advances {
            push_u16(&mut hmtx, *advance);
            push_i16(&mut hmtx, 0);
        }

        let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"cmap", self.build_character_map()),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"maxp", maxp),
        ];
        if let Some(script) = self.script {
            tables.push((*b"GSUB", build_glyph_substitution(script, &self.lookups)));
        }
        tables.sort_by_key(|(tag, _)| *tag);

        assemble(&tables)
    }

    /// The bytes, parsed as the crate's own [`FontFace`].
    pub(crate) fn face(&self) -> Arc<FontFace> {
        let bytes = self.build();
        Arc::new(
            FontFace::parse(Arc::from(bytes.as_slice()), 0)
                .expect("the synthetic face this module built parses"),
        )
    }

    /// A `cmap` with one format 12 subtable at platform 3, encoding 10.
    ///
    /// Format 12 rather than format 4 because it is a flat list of groups: no `idDelta`,
    /// `idRangeOffset` or `searchRange` arithmetic to get wrong, and a wrong `cmap` would look
    /// exactly like a shaping failure.
    fn build_character_map(&self) -> Vec<u8> {
        let mut groups: Vec<(u32, u16)> = self.character_map.clone();
        groups.sort_unstable();

        let mut subtable = Vec::new();
        push_u16(&mut subtable, 12); // format
        push_u16(&mut subtable, 0); // reserved
        let length_at = subtable.len();
        push_u32(&mut subtable, 0); // length, filled below
        push_u32(&mut subtable, 0); // language
        push_u32(
            &mut subtable,
            u32::try_from(groups.len()).unwrap_or(u32::MAX),
        );
        for (code_point, glyph) in &groups {
            push_u32(&mut subtable, *code_point);
            push_u32(&mut subtable, *code_point);
            push_u32(&mut subtable, u32::from(*glyph));
        }
        let length = u32::try_from(subtable.len()).unwrap_or(u32::MAX);
        subtable[length_at..length_at + 4].copy_from_slice(&length.to_be_bytes());

        let mut cmap = Vec::new();
        push_u16(&mut cmap, 0); // version
        push_u16(&mut cmap, 1); // numTables
        push_u16(&mut cmap, 3); // platformID: Windows
        push_u16(&mut cmap, 10); // encodingID: Unicode full repertoire
        push_u32(&mut cmap, 12); // offset to the subtable
        cmap.extend_from_slice(&subtable);
        cmap
    }
}

/// A `GSUB` with one `SingleSubstFormat1` lookup per entry, one feature reaching each, and one
/// script whose default language system uses all of them.
fn build_glyph_substitution(script: [u8; 4], lookups: &[SingleSubstitution]) -> Vec<u8> {
    let mut lookup_list = Vec::new();
    push_u16(
        &mut lookup_list,
        u16::try_from(lookups.len()).unwrap_or(u16::MAX),
    );
    let offsets_at = lookup_list.len();
    for _ in lookups {
        push_u16(&mut lookup_list, 0);
    }
    for (index, lookup) in lookups.iter().enumerate() {
        let lookup_at = u16::try_from(lookup_list.len()).unwrap_or(u16::MAX);
        let slot = offsets_at + index * 2;
        lookup_list[slot..slot + 2].copy_from_slice(&lookup_at.to_be_bytes());
        push_u16(&mut lookup_list, 1); // lookupType: single substitution
        push_u16(&mut lookup_list, 0); // lookupFlag
        push_u16(&mut lookup_list, 1); // subTableCount
        push_u16(&mut lookup_list, 8); // subtableOffset, from the lookup's own start
        push_u16(&mut lookup_list, 1); // SingleSubstFormat1
        push_u16(&mut lookup_list, 6); // coverageOffset, from the subtable's own start
        push_i16(&mut lookup_list, lookup.delta);
        push_u16(&mut lookup_list, 1); // CoverageFormat1
        push_u16(
            &mut lookup_list,
            u16::try_from(lookup.coverage.len()).unwrap_or(u16::MAX),
        );
        for glyph in &lookup.coverage {
            push_u16(&mut lookup_list, *glyph);
        }
    }

    // The specification requires FeatureList records to be ordered by tag.
    let mut order: Vec<usize> = (0..lookups.len()).collect();
    order.sort_by_key(|index| lookups[*index].feature);

    let mut feature_list = Vec::new();
    push_u16(
        &mut feature_list,
        u16::try_from(order.len()).unwrap_or(u16::MAX),
    );
    let records_at = feature_list.len();
    for index in &order {
        feature_list.extend_from_slice(&lookups[*index].feature);
        push_u16(&mut feature_list, 0);
    }
    for (slot, index) in order.iter().enumerate() {
        let feature_at = u16::try_from(feature_list.len()).unwrap_or(u16::MAX);
        let patch = records_at + slot * 6 + 4;
        feature_list[patch..patch + 2].copy_from_slice(&feature_at.to_be_bytes());
        push_u16(&mut feature_list, 0); // featureParamsOffset
        push_u16(&mut feature_list, 1); // lookupIndexCount
        push_u16(&mut feature_list, u16::try_from(*index).unwrap_or(0));
    }

    let mut script_list = Vec::new();
    push_u16(&mut script_list, 1); // scriptCount
    script_list.extend_from_slice(&script);
    push_u16(&mut script_list, 8); // offset to the Script table
    push_u16(&mut script_list, 4); // defaultLangSysOffset, from the Script table's start
    push_u16(&mut script_list, 0); // langSysCount
    push_u16(&mut script_list, 0); // lookupOrderOffset
    push_u16(&mut script_list, 0xFFFF); // requiredFeatureIndex
    push_u16(
        &mut script_list,
        u16::try_from(order.len()).unwrap_or(u16::MAX),
    );
    for slot in 0..order.len() {
        push_u16(&mut script_list, u16::try_from(slot).unwrap_or(0));
    }

    const HEADER: usize = 10;
    let mut gsub = Vec::new();
    push_u16(&mut gsub, 1); // majorVersion
    push_u16(&mut gsub, 0); // minorVersion
    push_u16(&mut gsub, u16::try_from(HEADER).unwrap_or(u16::MAX));
    push_u16(
        &mut gsub,
        u16::try_from(HEADER + script_list.len()).unwrap_or(u16::MAX),
    );
    push_u16(
        &mut gsub,
        u16::try_from(HEADER + script_list.len() + feature_list.len()).unwrap_or(u16::MAX),
    );
    gsub.extend_from_slice(&script_list);
    gsub.extend_from_slice(&feature_list);
    gsub.extend_from_slice(&lookup_list);
    gsub
}

/// Wrap the tables in an `sfnt` directory, four-byte aligned.
fn assemble(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(tables.len()).unwrap_or(u16::MAX);
    let entry_selector = (u32::BITS - 1 - u32::from(count).leading_zeros()) as u16;
    let search_range = (1_u16 << entry_selector) * 16;

    let mut out = Vec::new();
    push_u32(&mut out, 0x0001_0000); // sfntVersion: TrueType outlines
    push_u16(&mut out, count);
    push_u16(&mut out, search_range);
    push_u16(&mut out, entry_selector);
    push_u16(&mut out, count * 16 - search_range);
    let directory_at = out.len();
    out.resize(directory_at + tables.len() * 16, 0);

    for (index, (tag, data)) in tables.iter().enumerate() {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        let offset = u32::try_from(out.len()).unwrap_or(u32::MAX);
        let record = directory_at + index * 16;
        out[record..record + 4].copy_from_slice(tag);
        out[record + 4..record + 8].copy_from_slice(&0_u32.to_be_bytes()); // checkSum
        out[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
        out[record + 12..record + 16]
            .copy_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// `U+0628 ARABIC LETTER BEH`, mapped to glyph 10, with the four joining forms at 11, 12, 13, 14.
///
/// The advances differ per form as they do in a real face — a `beh` in initial position is much
/// narrower than an isolated one — so a naive shaper's total width is wrong as well as its glyph
/// ids.
pub(crate) fn arabic_face() -> SyntheticFace {
    SyntheticFace::new(vec![
        0, 250, 0, 0, 0, 0, 0, 0, 0, 0, 500, 510, 520, 530, 540,
    ])
    .mapping(' ', 1)
    .mapping('\u{0628}', 10)
    .with_substitutions(
        *b"arab",
        vec![
            SingleSubstitution {
                feature: *b"isol",
                coverage: vec![10],
                delta: 1,
            },
            SingleSubstitution {
                feature: *b"init",
                coverage: vec![10],
                delta: 2,
            },
            SingleSubstitution {
                feature: *b"medi",
                coverage: vec![10],
                delta: 3,
            },
            SingleSubstitution {
                feature: *b"fina",
                coverage: vec![10],
                delta: 4,
            },
        ],
    )
}

/// `U+0915 DEVANAGARI LETTER KA` at glyph 10 and `U+093F DEVANAGARI VOWEL SIGN I` at glyph 11.
///
/// No `GSUB` at all: the pre-base matra reordering is the Indic shaper's own, decided from the
/// characters' Unicode properties, and `NotoSansDevanagari-Regular` reorders the same pair the same
/// way. A font with no substitutions is therefore the sharpest form of the test — whatever moves the
/// matra, it is not a lookup in this file.
pub(crate) fn devanagari_face() -> SyntheticFace {
    SyntheticFace::new(vec![0, 250, 0, 0, 0, 0, 0, 0, 0, 0, 700, 300])
        .mapping(' ', 1)
        .mapping('\u{0915}', 10)
        .mapping('\u{093F}', 11)
}

/// A Latin face at `units_per_em` whose `A` advances exactly half an em and whose `B` advances a
/// quarter, so two of these at different em squares are exactly comparable.
pub(crate) fn proportional_latin_face(units_per_em: u16) -> SyntheticFace {
    let half = units_per_em / 2;
    let quarter = units_per_em / 4;
    SyntheticFace::new(vec![0, quarter, 0, half, quarter])
        .with_units_per_em(units_per_em)
        .mapping(' ', 1)
        .mapping('A', 3)
        .mapping('B', 4)
}
