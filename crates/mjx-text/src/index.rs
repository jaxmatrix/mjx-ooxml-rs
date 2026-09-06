//! An indexed set of faces, and the tiers built out of it.
//!
//! All three local tiers — the document's own embedded faces, the platform's, and the bundled
//! metric-compatible set — are the same thing with different contents: a set of faces indexed by
//! family, weight, width and slant, queried by the CSS font-matching rules. So there is one
//! implementation, [`FaceIndex`], and a tier is a [`FaceIndex`] with a label.
//!
//! # Why tier 1 may be empty, and why that is not an error
//!
//! **iOS does not expose system font files to a sandboxed process.** Reaching them needs CoreText,
//! which is a C API, which this crate may not link — it sits below the platform boundary that
//! `docs/UI_PLATFORM_PLAN.md` §3 draws at `mjx-paint`. So on that platform
//! [`FaceIndex::empty`] *is* the system tier, permanently, and every request resolves through tiers
//! 2 and 3. Nothing here treats that as degraded: an empty index answers `None` to a query, the
//! resolver moves down, and the substitution manifest records what happened. The only way this
//! could become a special case discovered late is if some path assumed tier 1 answers, which is
//! what `tests/empty_system_tier.rs` exists to prevent.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::error::FontError;
use crate::face::{FontFace, FontSlant, FontWeight, FontWidth};

/// Which tier answered a request.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ResolutionTier {
    /// The document carried the face itself. Always preferred: it is what the author saw.
    Embedded,
    /// The platform's own font files, enumerated at startup. Absent on iOS.
    System,
    /// The metric-compatible faces the application bundles — `docs/UI_PLATFORM_PLAN.md` §10's
    /// second tier, and the first one iOS can reach.
    Bundled,
    /// A subset that would have to be fetched. **This loop ships no transport**, so a resolution
    /// that reaches here is a plan, not a face.
    LazilyFetched,
}

impl ResolutionTier {
    /// How the tier is named in a message or a user interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Embedded => "embedded in the document",
            Self::System => "installed on this device",
            Self::Bundled => "bundled with the application",
            Self::LazilyFetched => "available to download",
        }
    }
}

/// What a document, or a token stack, is asking for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FontRequest<'a> {
    /// The family name, exactly as the document wrote it.
    pub family: &'a str,
    /// The weight the run is set in.
    pub weight: FontWeight,
    /// The width class the run is set in.
    pub width: FontWidth,
    /// Whether the run is upright, italic or oblique.
    pub slant: FontSlant,
    /// Characters the chosen face must have glyphs for, or empty when the caller does not care.
    ///
    /// This is what makes tier 3 answerable: a face can match the family and still have nothing to
    /// draw the run with — asking for `Calibri` and getting Carlito is no use to a line of Han —
    /// and the subset that *would* cover it is chosen from the characters that failed.
    ///
    /// It is a coverage requirement on **one** face, not itemisation: splitting a run across
    /// several faces is R03's job, and this crate only ever answers "which face".
    pub required_characters: &'a [char],
}

impl<'a> FontRequest<'a> {
    /// A request for `family` in the regular face.
    #[must_use]
    pub fn new(family: &'a str) -> Self {
        Self {
            family,
            weight: FontWeight::REGULAR,
            width: FontWidth::Normal,
            slant: FontSlant::Upright,
            required_characters: &[],
        }
    }

    /// The same request, requiring the chosen face to cover `characters`.
    #[must_use]
    pub fn requiring(mut self, characters: &'a [char]) -> Self {
        self.required_characters = characters;
        self
    }

    /// The same request, at `weight`.
    #[must_use]
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// The same request, at `width`.
    #[must_use]
    pub fn with_width(mut self, width: FontWidth) -> Self {
        self.width = width;
        self
    }

    /// The same request, at `slant`.
    #[must_use]
    pub fn with_slant(mut self, slant: FontSlant) -> Self {
        self.slant = slant;
        self
    }

    /// The same request against a different family — how a substitution is retried.
    #[must_use]
    pub fn for_family<'b>(&self, family: &'b str) -> FontRequest<'b>
    where
        'a: 'b,
    {
        FontRequest {
            family,
            weight: self.weight,
            width: self.width,
            slant: self.slant,
            required_characters: self.required_characters,
        }
    }
}

/// A set of faces, indexed by family and queried by the CSS font-matching rules.
///
/// The index is `fontdb`'s, which is what implements those rules; the cache above it is ours,
/// because parsing a face is cheap but not free and a document asks for the same family thousands
/// of times.
#[derive(Debug)]
pub struct FaceIndex {
    database: fontdb::Database,
    loaded: HashMap<fontdb::ID, Arc<FontFace>>,
}

impl FaceIndex {
    /// An index with nothing in it.
    ///
    /// This is the supported shape of the system tier on iOS, not a placeholder. See the module
    /// documentation.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            database: fontdb::Database::new(),
            loaded: HashMap::new(),
        }
    }

    /// An index of every face the platform exposes.
    ///
    /// On a platform that exposes none — iOS — this returns the same thing [`FaceIndex::empty`]
    /// does, and nothing downstream can tell the difference or needs to.
    #[must_use]
    pub fn from_platform() -> Self {
        let mut index = Self::empty();
        index.database.load_system_fonts();
        index
    }

    /// Add every face in `directory` and its subdirectories.
    ///
    /// Files that are not faces are skipped rather than reported: a font directory routinely holds
    /// `fonts.dir`, `.uuid` and licence text, and refusing to index a directory because it contains
    /// a `README` would make the bundled tier fragile for no gain. A directory that cannot be read
    /// *at all* is a different matter and is returned.
    ///
    /// # Errors
    ///
    /// [`FontError::UnreadableDirectory`] when the directory itself cannot be listed.
    pub fn add_directory(&mut self, directory: &Path) -> Result<(), FontError> {
        // `load_fonts_dir` swallows a missing directory, which would make a mis-pointed bundle path
        // look like an empty bundle. Checking first is what turns that into a message.
        let listing =
            std::fs::read_dir(directory).map_err(|source| FontError::UnreadableDirectory {
                path: directory.to_owned(),
                source,
            })?;
        drop(listing);
        self.database.load_fonts_dir(directory);
        Ok(())
    }

    /// Add a single face file.
    ///
    /// # Errors
    ///
    /// [`FontError::UnreadableFile`] when the file cannot be read or holds no face.
    pub fn add_file(&mut self, path: &Path) -> Result<(), FontError> {
        self.database
            .load_font_file(path)
            .map_err(|source| FontError::UnreadableFile {
                path: path.to_owned(),
                source,
            })
    }

    /// Add a face from bytes already in memory — an embedded font, or a fetched subset.
    pub fn add_bytes(&mut self, data: Vec<u8>) {
        self.database.load_font_data(data);
    }

    /// How many faces the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.database.len()
    }

    /// Whether the index holds no faces at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.database.is_empty()
    }

    /// Every family in the index, each named once, sorted.
    #[must_use]
    pub fn families(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .database
            .faces()
            .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Whether the index holds any face in `family`.
    #[must_use]
    pub fn has_family(&self, family: &str) -> bool {
        self.database.faces().any(|face| {
            face.families
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(family))
        })
    }

    /// The best face in the index for `request`, by the CSS font-matching rules — which is what
    /// makes a request for a bold face that the index does not hold resolve to the regular one
    /// rather than to nothing.
    ///
    /// # Errors
    ///
    /// Whatever [`FontFace::parse`] returns, if the matched face's bytes will not parse. A face
    /// `fontdb` indexed can still fail here: `fontdb` reads the `name` and `OS/2` tables, and a
    /// truncated `glyf` only shows up on a full parse.
    pub fn best_match(
        &mut self,
        request: &FontRequest<'_>,
    ) -> Result<Option<Arc<FontFace>>, FontError> {
        let families = [fontdb::Family::Name(request.family)];
        let query = fontdb::Query {
            families: &families,
            weight: fontdb::Weight(request.weight.0),
            stretch: stretch_of(request.width),
            style: style_of(request.slant),
        };
        let Some(id) = self.database.query(&query) else {
            return Ok(None);
        };
        self.load(id).map(Some)
    }

    /// Load, parse and cache the face `id` names.
    fn load(&mut self, id: fontdb::ID) -> Result<Arc<FontFace>, FontError> {
        if let Some(face) = self.loaded.get(&id) {
            return Ok(Arc::clone(face));
        }
        // `with_face_data` reads the file (or borrows the bytes) and hands them over for the
        // lifetime of the closure, which is why the parse happens inside it.
        let parsed = self
            .database
            .with_face_data(id, |data, index| {
                FontFace::parse(Arc::from(data), index).map(Arc::new)
            })
            .transpose()?;
        let Some(face) = parsed else {
            // `query` returned this id from this database an instant ago, so the only way here is a
            // file that vanished between the two. That is a real, reportable condition rather than
            // an invariant to assert.
            let path = self
                .database
                .face(id)
                .map(|info| info.post_script_name.clone())
                .unwrap_or_default();
            return Err(FontError::UnreadableFile {
                path: path.into(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "the indexed face is no longer readable",
                ),
            });
        };
        self.loaded.insert(id, Arc::clone(&face));
        Ok(face)
    }
}

impl Default for FaceIndex {
    fn default() -> Self {
        Self::empty()
    }
}

fn stretch_of(width: FontWidth) -> fontdb::Stretch {
    match width {
        FontWidth::UltraCondensed => fontdb::Stretch::UltraCondensed,
        FontWidth::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        FontWidth::Condensed => fontdb::Stretch::Condensed,
        FontWidth::SemiCondensed => fontdb::Stretch::SemiCondensed,
        FontWidth::Normal => fontdb::Stretch::Normal,
        FontWidth::SemiExpanded => fontdb::Stretch::SemiExpanded,
        FontWidth::Expanded => fontdb::Stretch::Expanded,
        FontWidth::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        FontWidth::UltraExpanded => fontdb::Stretch::UltraExpanded,
    }
}

fn style_of(slant: FontSlant) -> fontdb::Style {
    match slant {
        FontSlant::Upright => fontdb::Style::Normal,
        FontSlant::Italic => fontdb::Style::Italic,
        FontSlant::Oblique => fontdb::Style::Oblique,
    }
}
