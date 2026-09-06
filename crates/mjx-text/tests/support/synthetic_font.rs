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

/// A rectangular outline for one glyph, in font units with y upward from the baseline.
///
/// A rectangle rather than a letterform on purpose. Rasterisation is being tested, not typeface
/// design: a filled box has an exactly predictable coverage, an exactly predictable bounding box and
/// an exactly predictable command count, so an assertion about what came out of the scan converter
/// is an assertion rather than a guess.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphBox {
    pub(crate) left: i16,
    pub(crate) bottom: i16,
    pub(crate) right: i16,
    pub(crate) top: i16,
}

impl GlyphBox {
    /// A box `width` by `height` sitting on the baseline at the origin.
    pub(crate) fn upright(width: i16, height: i16) -> Self {
        Self {
            left: 0,
            bottom: 0,
            right: width,
            top: height,
        }
    }
}

/// One layer of a `COLR` colour glyph: which glyph draws it, and which palette entry colours it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ColourLayer {
    pub(crate) glyph: u16,
    pub(crate) palette_entry: u16,
}

/// A `COLR` base glyph: the glyph a `cmap` maps to, drawn as this stack of layers.
#[derive(Clone, Debug)]
pub(crate) struct ColourGlyph {
    pub(crate) base: u16,
    pub(crate) layers: Vec<ColourLayer>,
}

/// A face to be built: its character map, its advances, and optionally outlines, a `GSUB`, and a
/// `COLR`/`CPAL` pair.
pub(crate) struct SyntheticFace {
    units_per_em: u16,
    character_map: Vec<(u32, u16)>,
    advances: Vec<u16>,
    outlines: Vec<Option<GlyphBox>>,
    script: Option<[u8; 4]>,
    lookups: Vec<SingleSubstitution>,
    palette: Vec<[u8; 4]>,
    colour_glyphs: Vec<ColourGlyph>,
}

impl SyntheticFace {
    /// A face with `advances.len()` glyphs, whose advances are `advances` in glyph order.
    pub(crate) fn new(advances: Vec<u16>) -> Self {
        Self {
            units_per_em: DEFAULT_UNITS_PER_EM,
            character_map: Vec::new(),
            advances,
            outlines: Vec::new(),
            script: None,
            lookups: Vec::new(),
            palette: Vec::new(),
            colour_glyphs: Vec::new(),
        }
    }

    /// Give the face `glyf` and `loca` tables, one entry per glyph in glyph order.
    ///
    /// A `None` is a glyph with no outline — a space — which is a case the rasteriser has to answer
    /// for as much as a drawn one.
    pub(crate) fn with_outlines(mut self, outlines: Vec<Option<GlyphBox>>) -> Self {
        self.outlines = outlines;
        self
    }

    /// Give the face a `COLR`/`CPAL` pair: `palette` in red, green, blue, alpha order, and one
    /// entry per colour glyph.
    pub(crate) fn with_colour(
        mut self,
        palette: Vec<[u8; 4]>,
        colour_glyphs: Vec<ColourGlyph>,
    ) -> Self {
        self.palette = palette;
        self.colour_glyphs = colour_glyphs;
        self
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
                                // 1 is the long form: `loca` holds byte offsets rather than halves of them, so no glyph
                                // description has to be padded to an even length for its offset to be expressible.
        push_i16(&mut head, 1); // indexToLocFormat
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
        if self.outlines.is_empty() {
            push_u32(&mut maxp, 0x0000_5000); // version 0.5 — no `glyf`
            push_u16(&mut maxp, glyph_count);
        } else {
            // Version 1.0 is the form a face with `glyf` must use, and every field after the glyph
            // count is a hint about what a hinting interpreter has to reserve. Nothing here is
            // hinted by the font's own bytecode, so the generous constants below are simply large
            // enough that no reader has cause to reject the face.
            push_u32(&mut maxp, 0x0001_0000);
            push_u16(&mut maxp, glyph_count);
            push_u16(&mut maxp, 16); // maxPoints
            push_u16(&mut maxp, 4); // maxContours
            push_u16(&mut maxp, 16); // maxCompositePoints
            push_u16(&mut maxp, 4); // maxCompositeContours
            push_u16(&mut maxp, 2); // maxZones
            push_u16(&mut maxp, 0); // maxTwilightPoints
            push_u16(&mut maxp, 0); // maxStorage
            push_u16(&mut maxp, 0); // maxFunctionDefs
            push_u16(&mut maxp, 0); // maxInstructionDefs
            push_u16(&mut maxp, 0); // maxStackElements
            push_u16(&mut maxp, 0); // maxSizeOfInstructions
            push_u16(&mut maxp, 0); // maxComponentElements
            push_u16(&mut maxp, 0); // maxComponentDepth
        }

        // The left side bearing is not decoration. A TrueType rasteriser shifts a glyph's outline
        // by `lsb - xMin`, so a face whose `hmtx` says zero for a glyph whose outline begins at 400
        // has that outline moved 400 units to the left — which is exactly what a colour glyph's
        // layers must not have happen to them, since it would stack layers meant to sit side by
        // side on top of one another. So the bearing is written to match the outline.
        let mut hmtx = Vec::new();
        for (index, advance) in self.advances.iter().enumerate() {
            push_u16(&mut hmtx, *advance);
            let bearing = match self.outlines.get(index) {
                Some(Some(outline)) => outline.left,
                _ => 0,
            };
            push_i16(&mut hmtx, bearing);
        }

        let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
            (*b"cmap", self.build_character_map()),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"maxp", maxp),
        ];
        if !self.outlines.is_empty() {
            let (glyf, loca) = build_glyph_outlines(&self.outlines, usize::from(glyph_count));
            tables.push((*b"glyf", glyf));
            tables.push((*b"loca", loca));
        }
        if let Some(script) = self.script {
            tables.push((*b"GSUB", build_glyph_substitution(script, &self.lookups)));
        }
        if !self.colour_glyphs.is_empty() {
            tables.push((*b"COLR", build_colour_layers(&self.colour_glyphs)));
            tables.push((*b"CPAL", build_colour_palette(&self.palette)));
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

/// A `glyf` of one-contour rectangles and the long-format `loca` that indexes it.
///
/// A glyph with no outline is a `loca` entry equal to the one after it, which is how a font says
/// "this glyph draws nothing" — the form a space takes in every real face.
fn build_glyph_outlines(outlines: &[Option<GlyphBox>], glyph_count: usize) -> (Vec<u8>, Vec<u8>) {
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    for index in 0..glyph_count {
        push_u32(&mut loca, u32::try_from(glyf.len()).unwrap_or(u32::MAX));
        let Some(Some(box_)) = outlines.get(index) else {
            continue;
        };
        push_i16(&mut glyf, 1); // numberOfContours
        push_i16(&mut glyf, box_.left); // xMin
        push_i16(&mut glyf, box_.bottom); // yMin
        push_i16(&mut glyf, box_.right); // xMax
        push_i16(&mut glyf, box_.top); // yMax
        push_u16(&mut glyf, 3); // endPtsOfContours[0] — four points, indices 0..=3
        push_u16(&mut glyf, 0); // instructionLength
                                // `0x01` is ON_CURVE_POINT with neither short-vector bit set, so each coordinate below is a
                                // signed 16-bit delta from the point before it. Four flags, one per corner.
        glyf.extend_from_slice(&[0x01; 4]);
        let corners = [
            (box_.left, box_.bottom),
            (box_.right, box_.bottom),
            (box_.right, box_.top),
            (box_.left, box_.top),
        ];
        let mut previous_x = 0_i16;
        for (x, _) in corners {
            push_i16(&mut glyf, x.wrapping_sub(previous_x));
            previous_x = x;
        }
        let mut previous_y = 0_i16;
        for (_, y) in corners {
            push_i16(&mut glyf, y.wrapping_sub(previous_y));
            previous_y = y;
        }
    }
    // `loca` holds one more offset than there are glyphs: the end of the last one.
    push_u32(&mut loca, u32::try_from(glyf.len()).unwrap_or(u32::MAX));
    (glyf, loca)
}

/// A `COLR` version 0 table: base glyphs, each naming a run of layer records.
///
/// Version 0 rather than version 1 deliberately. Version 1 adds gradients and compositing modes,
/// which are a paint graph rather than a layer stack; version 0 is the form Windows' own emoji face
/// ships and the form that proves the *route* — palette lookup, per-layer rasterisation, compositing
/// into one RGBA image — without also testing a paint interpreter.
fn build_colour_layers(glyphs: &[ColourGlyph]) -> Vec<u8> {
    let mut base_records = Vec::new();
    let mut layer_records = Vec::new();
    let mut sorted: Vec<&ColourGlyph> = glyphs.iter().collect();
    // The specification requires the base glyph records to be sorted by glyph id, because a reader
    // binary-searches them.
    sorted.sort_by_key(|glyph| glyph.base);
    for glyph in sorted {
        push_u16(&mut base_records, glyph.base);
        push_u16(
            &mut base_records,
            u16::try_from(layer_records.len() / 4).unwrap_or(u16::MAX),
        );
        push_u16(
            &mut base_records,
            u16::try_from(glyph.layers.len()).unwrap_or(u16::MAX),
        );
        for layer in &glyph.layers {
            push_u16(&mut layer_records, layer.glyph);
            push_u16(&mut layer_records, layer.palette_entry);
        }
    }

    const HEADER: usize = 14;
    let mut colr = Vec::new();
    push_u16(&mut colr, 0); // version
    push_u16(
        &mut colr,
        u16::try_from(base_records.len() / 6).unwrap_or(u16::MAX),
    );
    push_u32(&mut colr, u32::try_from(HEADER).unwrap_or(u32::MAX));
    push_u32(
        &mut colr,
        u32::try_from(HEADER + base_records.len()).unwrap_or(u32::MAX),
    );
    push_u16(
        &mut colr,
        u16::try_from(layer_records.len() / 4).unwrap_or(u16::MAX),
    );
    colr.extend_from_slice(&base_records);
    colr.extend_from_slice(&layer_records);
    colr
}

/// A `CPAL` version 0 table with one palette.
///
/// `entries` are given here in red, green, blue, alpha order, which is the order a caller thinks in;
/// `CPAL` stores blue, green, red, alpha, and the swap happens once, below.
fn build_colour_palette(entries: &[[u8; 4]]) -> Vec<u8> {
    // The version 0 header is twelve bytes, and the one `colorRecordIndices` entry for the single
    // palette below adds two more — so the colour records begin at fourteen.
    const HEADER: usize = 12;
    const INDICES: usize = 2;
    let mut cpal = Vec::new();
    push_u16(&mut cpal, 0); // version
    let count = u16::try_from(entries.len()).unwrap_or(u16::MAX);
    push_u16(&mut cpal, count); // numPaletteEntries
    push_u16(&mut cpal, 1); // numPalettes
    push_u16(&mut cpal, count); // numColorRecords
    push_u32(
        &mut cpal,
        u32::try_from(HEADER + INDICES).unwrap_or(u32::MAX), // colorRecordsArrayOffset
    );
    push_u16(&mut cpal, 0); // colorRecordIndices[0]
    for [red, green, blue, alpha] in entries {
        cpal.push(*blue);
        cpal.push(*green);
        cpal.push(*red);
        cpal.push(*alpha);
    }
    cpal
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

/// The letters `A` to `H` at glyphs 2 to 9, each a filled rectangle of its own size, with real
/// `glyf` outlines so that the face can be rasterised rather than only shaped.
///
/// Eight distinct glyphs rather than one, because the questions R04 asks are about a *working set*:
/// how many distinct images a string produces, which of them survive an eviction, and how few of
/// them a second frame has to upload. One glyph cannot answer any of those.
pub(crate) fn outlined_latin_face() -> SyntheticFace {
    let mut advances = vec![0_u16, 250];
    let mut outlines: Vec<Option<GlyphBox>> = vec![None, None];
    let mut mappings: Vec<(char, u16)> = vec![(' ', 1)];
    for (step, letter) in "ABCDEFGH".chars().enumerate() {
        // Widths and heights that differ per letter, so the shelf packer is handed genuinely varied
        // rectangles rather than a grid it could pack correctly by accident.
        let step = i16::try_from(step).unwrap_or(0);
        let width = 380 + step * 20;
        let height = 480 + step * 30;
        advances.push(u16::try_from(width + 60).unwrap_or(u16::MAX));
        outlines.push(Some(GlyphBox::upright(width, height)));
        mappings.push((letter, u16::try_from(step + 2).unwrap_or(2)));
    }

    let mut face = SyntheticFace::new(advances).with_outlines(outlines);
    for (character, glyph) in mappings {
        face = face.mapping(character, glyph);
    }
    face
}

/// A face whose `U+1F600 GRINNING FACE` is a two-layer `COLR` glyph over a two-entry `CPAL`.
///
/// The two layers do not overlap and the palette gives them opposite colours, so a rasterisation
/// that fell back to the plain outline route — or that composited one layer and dropped the other —
/// produces an image with one colour in it and fails an assertion that counts them. That is the
/// point: "it returned RGBA" is satisfied by a bitmap that is entirely one colour, and would be
/// satisfied by a colour path that never ran.
pub(crate) fn colour_glyph_face() -> SyntheticFace {
    SyntheticFace::new(vec![0, 250, 800, 0, 0])
        .mapping(' ', 1)
        .mapping('\u{1F600}', 2)
        .with_outlines(vec![
            None,
            None,
            // The base glyph has an outline of its own, which is what a colour face offers a reader
            // that cannot read `COLR`. A rasteriser that quietly took this route instead of the
            // colour one would produce a single-colour image, which is exactly what an assertion
            // counting colours catches.
            Some(GlyphBox::upright(700, 700)),
            Some(GlyphBox {
                left: 0,
                bottom: 0,
                right: 300,
                top: 700,
            }),
            Some(GlyphBox {
                left: 400,
                bottom: 0,
                right: 700,
                top: 700,
            }),
        ])
        .with_colour(
            vec![[255, 0, 0, 255], [0, 0, 255, 255]],
            vec![ColourGlyph {
                base: 2,
                layers: vec![
                    ColourLayer {
                        glyph: 3,
                        palette_entry: 0,
                    },
                    ColourLayer {
                        glyph: 4,
                        palette_entry: 1,
                    },
                ],
            }],
        )
}

/// `a` at glyph 2 and `U+0301 COMBINING ACUTE ACCENT` at glyph 3, with outlines and **no `GPOS`**.
///
/// No `GPOS` on purpose. With no mark-attachment table the shaper falls back to positioning the mark
/// from the two glyphs' own extents, which is what every face without one relies on and what
/// produces a non-zero `y_offset`. The accent's own outline sits just above the baseline, so the
/// offset the shaper produces is the whole of the distance it is lifted by — and a placer that
/// dropped that offset would draw the accent through the letter rather than over it.
pub(crate) fn combining_mark_face() -> SyntheticFace {
    SyntheticFace::new(vec![0, 250, 560, 0])
        .mapping(' ', 1)
        .mapping('a', 2)
        .mapping('\u{0301}', 3)
        .with_outlines(vec![
            None,
            None,
            Some(GlyphBox::upright(500, 500)),
            Some(GlyphBox {
                left: 120,
                bottom: 0,
                right: 380,
                top: 140,
            }),
        ])
}
