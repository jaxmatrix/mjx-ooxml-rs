//! Which OpenType features a run is shaped with.
//!
//! # Why the set is stated rather than left to the shaper
//!
//! `rustybuzz` already turns on the features the OpenType Layout registry marks *on by default* —
//! `ccmp`, `locl`, `rlig`, `liga`, `clig`, `calt`, `kern`, `mark`, `mkmk` and the script-specific
//! ones — so a caller that passes nothing gets sensible typography. That is not enough here, for two
//! reasons.
//!
//! **A document can switch them off.** Word's `w14:ligatures` says which ligature classes a run
//! uses, and `none` is a value; DrawingML's `a:rPr/@kern` gives the point size below which kerning
//! is *not* applied, so a 9-point run in a document whose `kern` is `1200` is shaped without it.
//! Turning a default-on feature off needs an explicit `tag = 0`, which only an explicit set can
//! carry.
//!
//! **The shaped-run cache is keyed on it.** Two runs of the same text in the same face at the same
//! size shape differently if their features differ, so the features have to be a value the cache can
//! compare and hash — not an absence the shaper fills in later. [`FeatureSet`] is therefore
//! canonical: sorted by tag, one entry per tag, so two sets that mean the same thing *are* the same
//! thing.
//!
//! # This module models features, not document properties
//!
//! Reading `w14:ligatures` or `a:rPr/@kern` out of a document is above this crate — `mjx-text` has
//! never heard of OOXML. What lives here is the vocabulary those properties map onto
//! ([`TypographyOptions`]) and the translation from it to the tags a shaper takes
//! ([`TypographyOptions::to_feature_set`]).

use std::fmt;

/// A four-character OpenType feature tag.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureTag([u8; 4]);

impl FeatureTag {
    /// `kern` — pair kerning. On by default in the registry.
    pub const KERNING: Self = Self(*b"kern");
    /// `liga` — standard ligatures, the ones a font applies unasked: `fi`, `fl`, `ffi`.
    pub const STANDARD_LIGATURES: Self = Self(*b"liga");
    /// `clig` — contextual ligatures, applied only in the contexts the font names.
    pub const CONTEXTUAL_LIGATURES: Self = Self(*b"clig");
    /// `dlig` — discretionary ligatures, off by default: `st`, `ct`.
    pub const DISCRETIONARY_LIGATURES: Self = Self(*b"dlig");
    /// `hlig` — historical ligatures, off by default.
    pub const HISTORICAL_LIGATURES: Self = Self(*b"hlig");
    /// `rlig` — required ligatures. Arabic *lam-alef* is one: the script has no other way to write
    /// the pair, so this is not a preference and a shaper that switched it off would produce text
    /// no reader would accept.
    pub const REQUIRED_LIGATURES: Self = Self(*b"rlig");
    /// `calt` — contextual alternates.
    pub const CONTEXTUAL_ALTERNATES: Self = Self(*b"calt");
    /// `lnum` — lining figures, all the same height as a capital.
    pub const LINING_FIGURES: Self = Self(*b"lnum");
    /// `onum` — old-style figures, with ascenders and descenders.
    pub const OLD_STYLE_FIGURES: Self = Self(*b"onum");
    /// `pnum` — proportional figures, each digit its own width.
    pub const PROPORTIONAL_FIGURES: Self = Self(*b"pnum");
    /// `tnum` — tabular figures, every digit the same width, so columns line up.
    pub const TABULAR_FIGURES: Self = Self(*b"tnum");
    /// `smcp` — lowercase letters drawn as small capitals.
    pub const SMALL_CAPITALS: Self = Self(*b"smcp");
    /// `c2sc` — capitals drawn as small capitals, which is what "all small caps" needs on top of
    /// `smcp`.
    pub const CAPITALS_TO_SMALL_CAPITALS: Self = Self(*b"c2sc");
    /// `subs` — subscript forms.
    pub const SUBSCRIPT: Self = Self(*b"subs");
    /// `sups` — superscript forms.
    pub const SUPERSCRIPT: Self = Self(*b"sups");
    /// `vert` — the vertical forms of a glyph, for East Asian vertical writing. `mjx-dml` already
    /// models the property that asks for it.
    pub const VERTICAL_ALTERNATES: Self = Self(*b"vert");
    /// `vrt2` — vertical rotation, the fuller form of `vert` that also rotates Latin.
    pub const VERTICAL_ROTATION: Self = Self(*b"vrt2");
    /// `isol`, `init`, `medi`, `fina` — the Arabic joining forms. Listed for documentation: the
    /// Arabic shaping engine applies them positionally and a caller never names them.
    pub const ARABIC_ISOLATED_FORM: Self = Self(*b"isol");
    /// See [`FeatureTag::ARABIC_ISOLATED_FORM`].
    pub const ARABIC_INITIAL_FORM: Self = Self(*b"init");
    /// See [`FeatureTag::ARABIC_ISOLATED_FORM`].
    pub const ARABIC_MEDIAL_FORM: Self = Self(*b"medi");
    /// See [`FeatureTag::ARABIC_ISOLATED_FORM`].
    pub const ARABIC_FINAL_FORM: Self = Self(*b"fina");

    /// A tag from its four bytes.
    #[must_use]
    pub const fn new(tag: [u8; 4]) -> Self {
        Self(tag)
    }

    /// The stylistic set `number`, `ss01` through `ss20`, or `None` outside that range.
    ///
    /// The registry defines exactly twenty, which is also how many `w:styleSet` can name.
    #[must_use]
    pub fn stylistic_set(number: u8) -> Option<Self> {
        if !(1..=20).contains(&number) {
            return None;
        }
        let tens = b'0' + number / 10;
        let units = b'0' + number % 10;
        Some(Self([b's', b's', tens, units]))
    }

    /// The tag's four bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }

    /// The tag as text.
    #[must_use]
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("????")
    }
}

impl fmt::Debug for FeatureTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FeatureTag({})", self.name())
    }
}

impl fmt::Display for FeatureTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One feature, and the value it is set to.
///
/// A value of `0` switches the feature off; `1` switches it on; higher values select an alternate
/// where the feature defines a numbered list of them.
///
/// A feature applies to the **whole run**. Runs at this layer are already homogeneous in every
/// property a document can vary — that is what itemisation produced them for — so a sub-range
/// feature would describe a run that should have been two runs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FontFeature {
    /// Which feature.
    pub tag: FeatureTag,
    /// What it is set to.
    pub value: u32,
}

impl FontFeature {
    /// `tag`, switched on.
    #[must_use]
    pub const fn on(tag: FeatureTag) -> Self {
        Self { tag, value: 1 }
    }

    /// `tag`, switched off.
    #[must_use]
    pub const fn off(tag: FeatureTag) -> Self {
        Self { tag, value: 0 }
    }

    /// `tag`, set to `value`.
    #[must_use]
    pub const fn set(tag: FeatureTag, value: u32) -> Self {
        Self { tag, value }
    }

    /// Whether the feature is switched on at all.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.value != 0
    }
}

/// A canonical set of features: sorted by tag, one entry per tag.
///
/// Canonical because the shaped-run cache keys on it. Two callers that build the same features in
/// different orders must produce equal sets, or the cache misses on runs it already holds and the
/// scroll it exists for stops being affordable.
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct FeatureSet {
    features: Vec<FontFeature>,
}

impl FeatureSet {
    /// The empty set — shape with whatever the font and the registry default to.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `feature`, replacing any previous value for the same tag.
    pub fn set(&mut self, feature: FontFeature) {
        match self
            .features
            .binary_search_by(|existing| existing.tag.cmp(&feature.tag))
        {
            Ok(index) => self.features[index] = feature,
            Err(index) => self.features.insert(index, feature),
        }
    }

    /// The same set, with `feature` set — the builder form of [`FeatureSet::set`].
    #[must_use]
    pub fn with(mut self, feature: FontFeature) -> Self {
        self.set(feature);
        self
    }

    /// The value `tag` is set to, if it is in the set.
    #[must_use]
    pub fn value_of(&self, tag: FeatureTag) -> Option<u32> {
        self.features
            .binary_search_by(|existing| existing.tag.cmp(&tag))
            .ok()
            .and_then(|index| self.features.get(index))
            .map(|feature| feature.value)
    }

    /// Whether `tag` is in the set with a non-zero value.
    #[must_use]
    pub fn is_enabled(&self, tag: FeatureTag) -> bool {
        self.value_of(tag).is_some_and(|value| value != 0)
    }

    /// Every feature, in tag order.
    #[must_use]
    pub fn features(&self) -> &[FontFeature] {
        &self.features
    }

    /// How many features the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

impl FromIterator<FontFeature> for FeatureSet {
    fn from_iter<I: IntoIterator<Item = FontFeature>>(iterator: I) -> Self {
        let mut set = Self::new();
        for feature in iterator {
            set.set(feature);
        }
        set
    }
}

/// Which ligature classes a run uses.
///
/// The four fields are the four classes the OpenType registry defines and `w14:ligatures` names.
/// [`LigatureOptions::default`] is the registry's own default state — standard, contextual and
/// required on, discretionary and historical off — which is what a font applies when a document says
/// nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LigatureOptions {
    /// `liga`.
    pub standard: bool,
    /// `clig`.
    pub contextual: bool,
    /// `dlig`.
    pub discretionary: bool,
    /// `hlig`.
    pub historical: bool,
}

impl LigatureOptions {
    /// No ligatures of any class — `w14:ligatures` set to `none`.
    ///
    /// **`rlig` is not switched off by this**, and cannot be from here: an Arabic *lam-alef* is not
    /// a preference, it is the only way the pair is written.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            standard: false,
            contextual: false,
            discretionary: false,
            historical: false,
        }
    }

    /// Every class, including the two the registry leaves off.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            standard: true,
            contextual: true,
            discretionary: true,
            historical: true,
        }
    }
}

impl Default for LigatureOptions {
    fn default() -> Self {
        Self {
            standard: true,
            contextual: true,
            discretionary: false,
            historical: false,
        }
    }
}

/// Which shape the digits take.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum FigureStyle {
    /// Whatever the font draws unasked.
    #[default]
    AsTheFontDraws,
    /// `lnum` — every digit as tall as a capital.
    Lining,
    /// `onum` — digits with ascenders and descenders, sitting in the lowercase.
    OldStyle,
}

/// Whether digits are all one width.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum FigureSpacing {
    /// Whatever the font draws unasked.
    #[default]
    AsTheFontDraws,
    /// `pnum` — each digit its own width.
    Proportional,
    /// `tnum` — every digit the same width, so a column of numbers lines up.
    Tabular,
}

/// Whether lowercase, or everything, is drawn as small capitals.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum SmallCapitals {
    /// Neither.
    #[default]
    None,
    /// `smcp` — lowercase only, which is `w:smallCaps` and `a:rPr/@cap="small"`.
    LowercaseOnly,
    /// `smcp` and `c2sc` — capitals too.
    Everything,
}

/// The twenty stylistic sets, as a bit set.
///
/// `w:styleSet` names one by number and a run may carry several, so a set of booleans is the shape
/// of the property. Twenty fits in a `u32` with room to spare, which keeps [`TypographyOptions`]
/// `Copy` and therefore keeps a cache key cheap to build.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct StylisticSets(u32);

impl StylisticSets {
    /// None of them.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// The same sets, with `number` (1 to 20) added. A number outside that range is ignored, because
    /// the registry defines no feature for it and inventing one would send the shaper a tag no font
    /// has.
    #[must_use]
    pub const fn with(self, number: u8) -> Self {
        if number == 0 || number > 20 {
            return self;
        }
        Self(self.0 | (1 << (number - 1)))
    }

    /// Whether `number` is in the set.
    #[must_use]
    pub const fn contains(self, number: u8) -> bool {
        if number == 0 || number > 20 {
            return false;
        }
        self.0 & (1 << (number - 1)) != 0
    }

    /// Whether no set at all is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Every selected set number, ascending.
    pub fn numbers(self) -> impl Iterator<Item = u8> {
        (1..=20_u8).filter(move |number| self.contains(*number))
    }
}

/// Everything about a run that decides which OpenType features it is shaped with.
///
/// [`TypographyOptions::default`] is **the OpenType Layout registry's own default state** — the
/// features it marks on by default are on, the rest off. It is deliberately not called "Word's
/// defaults": which classes Word turns on varies by template and by the `w14:ligatures` a document
/// carries, and mapping a document's properties onto these fields is the caller's job, above this
/// crate.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct TypographyOptions {
    /// Whether pair kerning applies. DrawingML's `a:rPr/@kern` is a *minimum size*, so the caller
    /// compares it with the run's size and sets this.
    pub kerning: bool,
    /// Which ligature classes apply.
    pub ligatures: LigatureOptions,
    /// Whether contextual alternates apply.
    pub contextual_alternates: bool,
    /// Which shape the digits take.
    pub figure_style: FigureStyle,
    /// Whether the digits are all one width.
    pub figure_spacing: FigureSpacing,
    /// Whether lowercase, or everything, is drawn as small capitals.
    pub small_capitals: SmallCapitals,
    /// Which stylistic sets the run selects.
    pub stylistic_sets: StylisticSets,
    /// Whether the run is set vertically, so the font's vertical forms apply.
    pub vertical_writing: bool,
}

impl TypographyOptions {
    /// The registry's default state: kerning, standard ligatures and contextual ligatures on,
    /// contextual alternates on, everything else as the font draws it.
    #[must_use]
    pub fn registry_defaults() -> Self {
        Self {
            kerning: true,
            ligatures: LigatureOptions::default(),
            contextual_alternates: true,
            figure_style: FigureStyle::AsTheFontDraws,
            figure_spacing: FigureSpacing::AsTheFontDraws,
            small_capitals: SmallCapitals::None,
            stylistic_sets: StylisticSets::none(),
            vertical_writing: false,
        }
    }

    /// The features these options ask for, as the shaper takes them.
    ///
    /// Every feature this type models appears in the set with an explicit value, on or off, so that
    /// two option values that differ produce two sets that differ — which is what makes the
    /// shaped-run cache safe to key on. Features the options say nothing about
    /// ([`FigureStyle::AsTheFontDraws`]) are absent, so the font decides.
    #[must_use]
    pub fn to_feature_set(&self) -> FeatureSet {
        let mut set = FeatureSet::new();
        set.set(FontFeature::set(
            FeatureTag::KERNING,
            u32::from(self.kerning),
        ));
        set.set(FontFeature::set(
            FeatureTag::STANDARD_LIGATURES,
            u32::from(self.ligatures.standard),
        ));
        set.set(FontFeature::set(
            FeatureTag::CONTEXTUAL_LIGATURES,
            u32::from(self.ligatures.contextual),
        ));
        set.set(FontFeature::set(
            FeatureTag::DISCRETIONARY_LIGATURES,
            u32::from(self.ligatures.discretionary),
        ));
        set.set(FontFeature::set(
            FeatureTag::HISTORICAL_LIGATURES,
            u32::from(self.ligatures.historical),
        ));
        set.set(FontFeature::set(
            FeatureTag::CONTEXTUAL_ALTERNATES,
            u32::from(self.contextual_alternates),
        ));

        match self.figure_style {
            FigureStyle::AsTheFontDraws => {}
            FigureStyle::Lining => set.set(FontFeature::on(FeatureTag::LINING_FIGURES)),
            FigureStyle::OldStyle => set.set(FontFeature::on(FeatureTag::OLD_STYLE_FIGURES)),
        }
        match self.figure_spacing {
            FigureSpacing::AsTheFontDraws => {}
            FigureSpacing::Proportional => {
                set.set(FontFeature::on(FeatureTag::PROPORTIONAL_FIGURES));
            }
            FigureSpacing::Tabular => set.set(FontFeature::on(FeatureTag::TABULAR_FIGURES)),
        }
        match self.small_capitals {
            SmallCapitals::None => {}
            SmallCapitals::LowercaseOnly => set.set(FontFeature::on(FeatureTag::SMALL_CAPITALS)),
            SmallCapitals::Everything => {
                set.set(FontFeature::on(FeatureTag::SMALL_CAPITALS));
                set.set(FontFeature::on(FeatureTag::CAPITALS_TO_SMALL_CAPITALS));
            }
        }
        for number in self.stylistic_sets.numbers() {
            if let Some(tag) = FeatureTag::stylistic_set(number) {
                set.set(FontFeature::on(tag));
            }
        }
        if self.vertical_writing {
            set.set(FontFeature::on(FeatureTag::VERTICAL_ALTERNATES));
            set.set(FontFeature::on(FeatureTag::VERTICAL_ROTATION));
        }
        set
    }
}
