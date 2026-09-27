//! The three layered assertions, and the one thing they are for: **a failure that says which stage
//! broke.**
//!
//! # Why three tiers rather than one image
//!
//! A golden-image suite with a single tier answers every regression the same way: *the picture
//! changed*. A wrapped line, a mis-tessellated corner, a colour resolved from the wrong theme slot
//! and a painter that lost its clip stack all look identical — a grey smear in a diff — and every one
//! of them costs the same afternoon to localise by hand.
//!
//! So the same specimen is asserted three times, at three stages of one pipeline:
//!
//! | Tier | Subject | Catches |
//! |---|---|---|
//! | 1 | [`FragmentTree`](mjx_layout::FragmentTree) snapshot | layout: where things are, how they nest, what handles they name |
//! | 2 | [`DisplayList`](mjx_scene::DisplayList) snapshot | scene building: which commands, in what order, with what paints |
//! | 3 | rendered pixels | painting: tessellation, blending, effects, the rasteriser itself |
//!
//! **Tiers one and two need no painter at all**, which is what makes them cheap enough to run on
//! every specimen on every machine, and what makes a layout regression surface as a named line
//! rather than an image.
//!
//! # The localisation is the gate, not the tiers' existence
//!
//! Three tiers that always agree are one tier written three times. What is asserted in
//! `tests/the_tiers_fail_independently.rs` is the *pattern*: moving a fragment reddens all three and
//! localises to [`Localisation::Layout`]; recolouring a paint leaves tier one green and localises to
//! [`Localisation::SceneBuilding`]; corrupting the stored image alone localises to
//! [`Localisation::Painting`]. Any one of those failing to hold means the tiers are not independent,
//! whatever their names say.
//!
//! # [`Localisation::Inconsistent`] is not a defensive arm
//!
//! It is the answer when tier one differs and the tiers below it do not — which cannot happen for a
//! change that reaches the page, and *can* happen for a change that does not: a fragment's source
//! path, its part number, a handle nothing draws. It says the harness is looking at something the
//! renderer never saw, which is a finding about the specimen rather than about the renderer, and
//! reporting it as *"layout broke"* would send a reader to the wrong crate.

use crate::authority::{ReferenceProvider, RenderedContent, Verdict};
use crate::perceptual::{Difference, Tolerance};
use crate::snapshot::Snapshot;

/// What one tier concluded.
#[derive(Clone, PartialEq, Debug)]
pub enum TierOutcome {
    /// The two agree.
    Matched,
    /// They differ, and this is where.
    Differed {
        /// The first divergence, named. Never *"they differ"*.
        first_difference: String,
    },
    /// The tier did not run, or ran and cannot be believed. **Neither a pass nor a failure** — the
    /// same third state [`Verdict::NotEvidence`] carries, for the same reason.
    NotEvidence {
        /// Why, in a sentence a reader can act on.
        reason: String,
    },
}

impl TierOutcome {
    /// Whether this tier saw a difference. `false` for a tier that produced no evidence, which is
    /// what keeps an excluded tier out of a failure count.
    #[must_use]
    pub const fn is_difference(&self) -> bool {
        matches!(self, Self::Differed { .. })
    }

    /// Whether the tier agreed.
    #[must_use]
    pub const fn is_match(&self) -> bool {
        matches!(self, Self::Matched)
    }

    /// Whether the tier produced evidence either way.
    #[must_use]
    pub const fn is_evidence(&self) -> bool {
        !matches!(self, Self::NotEvidence { .. })
    }

    /// A one-word column for a table.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Differed { .. } => "differs",
            Self::NotEvidence { .. } => "excluded",
        }
    }

    /// Two snapshots as an outcome.
    #[must_use]
    pub fn from_snapshots(taken: &Snapshot, baseline: &Snapshot) -> Self {
        match taken.first_difference(baseline) {
            None => Self::Matched,
            Some(first_difference) => Self::Differed { first_difference },
        }
    }

    /// A pixel difference as an outcome, at `tolerance`.
    #[must_use]
    pub fn from_difference(difference: &Difference, tolerance: Tolerance) -> Self {
        match difference.verdict(tolerance, true) {
            Verdict::Agreed { .. } => Self::Matched,
            Verdict::Disagreed { worst, .. } => Self::Differed {
                first_difference: worst,
            },
            Verdict::NotEvidence { reason } => Self::NotEvidence { reason },
        }
    }
}

impl core::fmt::Display for TierOutcome {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Matched => formatter.write_str("matched"),
            Self::Differed { first_difference } => {
                write!(formatter, "differs — {first_difference}")
            }
            Self::NotEvidence { reason } => write!(formatter, "not evidence — {reason}"),
        }
    }
}

/// Which stage of the pipeline a difference is attributable to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Localisation {
    /// Every tier that produced evidence agreed.
    Everything,
    /// Tier one differs. Whatever else differs is downstream of it, so the finding is the box model
    /// or the specimen's own tree.
    Layout,
    /// Tier one agreed and tier two did not: the same fragments became different commands. The
    /// scene builder, or what the resolver answered it with.
    SceneBuilding,
    /// Tiers one and two agreed and the pixels did not: the same commands became a different image.
    /// The tessellator, the painter, or the rasteriser.
    Painting,
    /// Tier one differs and nothing below it does — so the difference never reached the page. A
    /// finding about the harness or the specimen, not about the renderer.
    Inconsistent,
    /// No tier produced evidence at all.
    NoEvidence,
}

impl Localisation {
    /// Every value, so a sweep cannot miss one.
    pub const ALL: [Self; 6] = [
        Self::Everything,
        Self::Layout,
        Self::SceneBuilding,
        Self::Painting,
        Self::Inconsistent,
        Self::NoEvidence,
    ];

    /// What a reader should do about it.
    #[must_use]
    pub const fn advice(self) -> &'static str {
        match self {
            Self::Everything => "nothing changed",
            Self::Layout => "the fragment tree moved: look at the box model, or at the specimen",
            Self::SceneBuilding => {
                "the same fragments became different commands: look at `mjx-scene`'s builder or at \
                 what the resolver answered"
            }
            Self::Painting => {
                "the same commands became a different image: look at the tessellator, the painter \
                 or the rasteriser"
            }
            Self::Inconsistent => {
                "the fragment tree differs and the page does not, so the change never reached the \
                 renderer: look at the specimen, not at the renderer"
            }
            Self::NoEvidence => "no tier produced evidence; nothing was compared",
        }
    }
}

/// One specimen, compared at all three tiers.
#[derive(Clone, PartialEq, Debug)]
pub struct LayeredComparison {
    /// Which specimen.
    pub specimen: String,
    /// Where the reference came from. `None` for the tiers, which compare against this crate's own
    /// committed baseline rather than against another producer.
    pub provider: ReferenceProvider,
    /// What kind of drawing the specimen is.
    pub content: RenderedContent,
    /// Tier one: the fragment tree.
    pub fragments: TierOutcome,
    /// Tier two: the display list.
    pub commands: TierOutcome,
    /// Tier three: the pixels.
    pub pixels: TierOutcome,
}

impl LayeredComparison {
    /// Which stage the difference is attributable to.
    ///
    /// The order of the arms is the order of the pipeline, and that is the whole algorithm: the
    /// **first** tier to differ is the one that caused it, because every later tier is downstream.
    #[must_use]
    pub fn localise(&self) -> Localisation {
        let tiers = [&self.fragments, &self.commands, &self.pixels];
        if !tiers.iter().any(|tier| tier.is_evidence()) {
            return Localisation::NoEvidence;
        }
        if self.fragments.is_difference() {
            // A tree difference that reached neither the list nor the pixels never reached the
            // page. Note the asymmetry: a tier that produced *no evidence* is not a tier that
            // agreed, so an excluded pixel tier cannot make a real difference look inconsistent.
            let reached_below = self.commands.is_difference() || self.pixels.is_difference();
            let below_had_evidence = self.commands.is_evidence() || self.pixels.is_evidence();
            return if reached_below || !below_had_evidence {
                Localisation::Layout
            } else {
                Localisation::Inconsistent
            };
        }
        if self.commands.is_difference() {
            return Localisation::SceneBuilding;
        }
        if self.pixels.is_difference() {
            return Localisation::Painting;
        }
        Localisation::Everything
    }

    /// Whether every tier that produced evidence agreed.
    #[must_use]
    pub fn agreed(&self) -> bool {
        self.localise() == Localisation::Everything
    }

    /// **Whether this may be written down as *"this matches PowerPoint"*.**
    ///
    /// Always `false` today, and by construction rather than by policy: the tiers compare against
    /// this crate's own committed baselines, whose provider is [`ReferenceProvider::None`], and
    /// [`ReferenceProvider::is_authoritative`] is `true` for an Office export alone. A tier
    /// comparison is a **regression** gate — it says the renderer still does what it did when a
    /// person approved it — and a regression gate is not a fidelity claim.
    #[must_use]
    pub fn may_be_called_parity(&self) -> bool {
        self.provider.is_authoritative() && self.agreed()
    }

    /// A table row: the specimen, the three tiers and what they localise to.
    #[must_use]
    pub fn row(&self) -> String {
        format!(
            "{:<18} {:<9} {:<9} {:<9} {:?}",
            self.specimen,
            self.fragments.word(),
            self.commands.word(),
            self.pixels.word(),
            self.localise()
        )
    }
}

impl core::fmt::Display for LayeredComparison {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            formatter,
            "{} ({}, against {})",
            self.specimen,
            self.content.label(),
            self.provider.label()
        )?;
        writeln!(formatter, "  tier 1 fragments: {}", self.fragments)?;
        writeln!(formatter, "  tier 2 commands:  {}", self.commands)?;
        writeln!(formatter, "  tier 3 pixels:    {}", self.pixels)?;
        write!(
            formatter,
            "  => {:?}: {}",
            self.localise(),
            self.localise().advice()
        )
    }
}
