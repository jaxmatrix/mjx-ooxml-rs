//! Where a reference came from, how much it is worth, and the third answer that is neither pass nor
//! fail.
//!
//! # The two traps, and they point in opposite directions
//!
//! MJXOFF-165 states the first: **a provisional baseline read as parity.** If baselines are approved
//! against LibreOffice now and quietly kept when PowerPoint arrives, LibreOffice's quirks are
//! ratified into the ledger — strictly worse than having no reference, because it *looks* like
//! parity.
//!
//! MJXOFF-207 states the second, and it is the mirror image: **a reference defect read as our
//! defect.** LibreOffice's export is not reliable for shades and gradients. A pixel difference there
//! is the reference's fault, and nothing in a pixel diff says which way round it is — so an agent
//! sees the difference, concludes the renderer is wrong, and spends a child "fixing" correct code
//! until it matches a bad export.
//!
//! Both are answered by the same three things:
//!
//! 1. [`ReferenceProvider`] says where a reference came from, and [`ReferenceProvider::authority`]
//!    maps that to `mjx-text`'s existing [`ReferenceAuthority`] — the **same** three-valued flag the
//!    font table uses, because a workspace with two authority enumerations in it has one too many.
//!    LibreOffice is [`ReferenceAuthority::Provisional`]. Until this crate, that variant was
//!    constructed nowhere in the workspace.
//! 2. The exclusion is a property of the **provider**, not of the fixture:
//!    [`ReferenceProvider::excludes`]. So it lifts automatically the day an Office export arrives,
//!    rather than being a list somebody must remember to revisit.
//! 3. [`Verdict`] has **three** states. An excluded comparison is
//!    [`Verdict::NotEvidence`] — not a pass, not a failure — and it carries the reason as text, so a
//!    reader of the report sees *why* rather than seeing a gap.
//!
//! # What is excluded, and what deliberately is not
//!
//! Two exclusions, and they have different reasons. Naming them separately matters: a single
//! "LibreOffice is unreliable" flag would make them indistinguishable, and one of them lifts for a
//! reason the other does not.
//!
//! * **Gradients and shades** — the user's own constraint. LibreOffice's PDF export does not
//!   reproduce them, so a pixel difference on that content says nothing about our renderer.
//! * **Preset hatches** — a different argument, and this crate's own. The fifty-four hatch bitmaps
//!   are the *thing being measured*: ECMA-376 gives none, `mjx_paint::PATTERN_MASKS` derives
//!   forty-four from their names and draws ten by hand, and the sitting exists to replace those
//!   ten with measured ones. Comparing them against LibreOffice's hatches would ratify
//!   **LibreOffice's** drawing, which is the first trap wearing the second trap's clothes.
//!
//! **Solid fill, stroke and geometry are not excluded from anything**, and that is the load-bearing
//! half. The preset decks are authored with solid fills and solid strokes precisely so their
//! exclusion list is *empty* — an exclusion that covered everything would prove nothing, and one
//! carried onto a sheet that did not need it would quietly remove the sheet from the comparison.
//! Two suites assert that emptiness rather than leaving it to be believed, one in each of this
//! module's two consumers: `mjx-reference-pack`'s `the_geometry_decks_are_excluded_from_nothing`
//! over the 187-plate decks, and this crate's own
//! `the_content_the_specimens_are_made_of_is_excluded_from_nothing` over the specimen corpus.
//!
//! The **layout tier is never excluded**: a word's bounding box says nothing about how the shape
//! behind it is filled, so `pdftotext -bbox-layout` comparisons stay fully meaningful on exactly the
//! files whose pixel tier is compromised.
//!
//! # Where this module lives, and why it moved
//!
//! MJXOFF-207 wrote it inside `mjx-reference-pack`. MJXOFF-165 needed exactly the same three things
//! — a provider, a three-state verdict and provider-attached exclusions — and **nothing may depend
//! on the reference pack**, so a second copy would have been the alternative. A workspace with two
//! answers to *"how much is this reference worth"* has one too many, which is the same argument that
//! put [`ReferenceAuthority`] in `mjx-text` rather than in every crate that needed it. So it moved
//! **down** into `mjx-render-oracle`, and the pack re-exports it as `mjx_reference_pack::authority`:
//! every path that named it still names it, and there is still one of it.

use mjx_text::ReferenceAuthority;

/// Who produced a reference rendering.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReferenceProvider {
    /// No reference at all. Our own output can still be checked against itself for *plumbing* —
    /// that the pipeline runs end to end — and against nothing else.
    ///
    /// This is not the absence of a provider, it is a provider that answers "nothing". The
    /// difference matters: a comparison run with no reference must produce a *named* non-answer
    /// rather than an empty report.
    None,
    /// LibreOffice, headless, via `soffice --convert-to pdf`. Available on every machine and in CI,
    /// and **not authoritative**: it is a change detector — *"this used to render and now does
    /// not"* — and never parity.
    LibreOffice,
    /// Microsoft Office on Windows, exporting to PDF, run by a person. The authoritative reference,
    /// and the one an agent cannot produce.
    OfficeExport,
}

impl ReferenceProvider {
    /// Every provider, so a sweep cannot miss one.
    pub const ALL: [Self; 3] = [Self::None, Self::LibreOffice, Self::OfficeExport];

    /// How much a comparison against this provider is worth, in the workspace's existing vocabulary.
    ///
    /// [`ReferenceAuthority::is_evidence`] is `true` for `Published` alone, which is exactly the
    /// property this needs: only an Office export makes a comparison evidence.
    #[must_use]
    pub fn authority(self) -> ReferenceAuthority {
        match self {
            // No numbers at all, and the gap is visible rather than silently absent — which is what
            // `Unverified` is documented to mean.
            Self::None => ReferenceAuthority::Unverified,
            // "Transcribed from a source that describes a *related* thing rather than the original,
            // kept so the shape of the data is honest, but never treated as proof." LibreOffice
            // renders the same document with a different engine; that is the same relationship a
            // metric clone has to the face it stands in for.
            Self::LibreOffice => ReferenceAuthority::Provisional,
            Self::OfficeExport => ReferenceAuthority::Published,
        }
    }

    /// Whether a comparison against this provider may be recorded as parity.
    #[must_use]
    pub fn is_authoritative(self) -> bool {
        self.authority().is_evidence()
    }

    /// How the provider is named in a report.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "no reference",
            Self::LibreOffice => "LibreOffice (preliminary)",
            Self::OfficeExport => "Microsoft Office (authoritative)",
        }
    }

    /// Why this provider cannot speak about `content`, or `None` when it can.
    ///
    /// The reason is a sentence rather than a flag, because it is printed: a reader who sees
    /// *"excluded"* learns nothing, and a reader who sees *"LibreOffice's PDF export does not
    /// reproduce gradients"* learns which side to suspect.
    #[must_use]
    pub fn excludes(self, content: RenderedContent) -> Option<&'static str> {
        match (self, content) {
            // An authoritative provider excludes nothing. The exclusions live here, on the
            // provider, precisely so that this arm lifts them all at once.
            (Self::OfficeExport, _) => None,
            (Self::None, _) => Some(
                "there is no reference to compare against; this run proves the pipeline and \
                 nothing about fidelity",
            ),
            (Self::LibreOffice, RenderedContent::GradientFill | RenderedContent::ShadedFill) => {
                Some(
                    "LibreOffice's PDF export does not reproduce gradients and shades, so a pixel \
                     difference here is the reference's and not ours",
                )
            }
            (Self::LibreOffice, RenderedContent::PatternFill) => Some(
                "the fifty-four hatch bitmaps are what this artefact exists to measure, and \
                 LibreOffice draws its own; comparing against them would ratify LibreOffice's \
                 hatches as ECMA-376's",
            ),
            (
                Self::LibreOffice,
                RenderedContent::Outline | RenderedContent::SolidFill | RenderedContent::TextLayout,
            ) => None,
        }
    }
}

/// What kind of drawing a comparison is looking at.
///
/// Coarse on purpose: it is not a description of the page, it is the axis the exclusions are
/// declared along. A plate is one of these, and a plate that were two would be a plate to split.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RenderedContent {
    /// A shape's outline — its path, stroked. What the preset decks are.
    Outline,
    /// A solid `a:solidFill`. Reproduced faithfully by every provider here.
    SolidFill,
    /// An `a:gradFill`. The user's stated exclusion.
    GradientFill,
    /// A shade — a fill whose colour varies across the shape without being a stated gradient
    /// (`a:path` shades, theme shade/tint transforms). Excluded for the same reason and named
    /// separately, because a reader chasing a difference needs to know which of the two they hit.
    ShadedFill,
    /// An `a:pattFill` — one of the fifty-four preset hatches.
    PatternFill,
    /// Where words landed, as `pdftotext -bbox-layout` reports them. **Never excluded**: a word box
    /// says nothing about fill.
    TextLayout,
}

impl RenderedContent {
    /// Every kind, so a sweep cannot miss one.
    pub const ALL: [Self; 6] = [
        Self::Outline,
        Self::SolidFill,
        Self::GradientFill,
        Self::ShadedFill,
        Self::PatternFill,
        Self::TextLayout,
    ];

    /// How the kind is named in a report.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Outline => "outline",
            Self::SolidFill => "solid fill",
            Self::GradientFill => "gradient fill",
            Self::ShadedFill => "shaded fill",
            Self::PatternFill => "pattern fill",
            Self::TextLayout => "text layout",
        }
    }
}

/// What one comparison concluded — and *whether it concluded anything*.
///
/// Three states, not two. The third is the whole point: an excluded comparison must not be
/// recordable as a pass, because a pass is what a later reader counts.
#[derive(Clone, PartialEq, Debug)]
pub enum Verdict {
    /// The two renderings agree within the stated tolerance.
    Agreed {
        /// What fraction of the compared pixels (or word boxes) differ.
        differing_fraction: f64,
        /// The fraction that was allowed.
        allowed: f64,
    },
    /// They differ by more than the tolerance.
    Disagreed {
        /// What fraction differ.
        differing_fraction: f64,
        /// The fraction that was allowed.
        allowed: f64,
        /// Where the worst difference is and what each side put there, so the finding names a place.
        worst: String,
    },
    /// **Neither.** The comparison did not happen, or happened and cannot be believed. Never a pass
    /// and never a failure.
    NotEvidence {
        /// Why, in a sentence a reader can act on.
        reason: String,
    },
}

impl Verdict {
    /// Whether this verdict may be counted as a pass.
    ///
    /// `false` for [`NotEvidence`](Self::NotEvidence), which is the assertion that keeps an excluded
    /// case out of a pass count.
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Agreed { .. })
    }

    /// Whether this verdict may be counted as a failure.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Disagreed { .. })
    }

    /// Whether the comparison produced no evidence either way.
    #[must_use]
    pub fn is_evidence(&self) -> bool {
        !matches!(self, Self::NotEvidence { .. })
    }

    /// A one-word column for a table.
    #[must_use]
    pub fn word(&self) -> &'static str {
        match self {
            Self::Agreed { .. } => "agreed",
            Self::Disagreed { .. } => "differs",
            Self::NotEvidence { .. } => "excluded",
        }
    }
}

impl core::fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Agreed {
                differing_fraction,
                allowed,
            } => write!(
                formatter,
                "agreed ({:.4}% differ, {:.4}% allowed)",
                differing_fraction * 100.0,
                allowed * 100.0
            ),
            Self::Disagreed {
                differing_fraction,
                allowed,
                worst,
            } => write!(
                formatter,
                "differs ({:.4}% differ, {:.4}% allowed) — {worst}",
                differing_fraction * 100.0,
                allowed * 100.0
            ),
            Self::NotEvidence { reason } => write!(formatter, "not evidence — {reason}"),
        }
    }
}

/// One recorded observation about one plate, carrying **where its reference came from** beside what
/// it concluded.
///
/// A verdict on its own is not recordable: *"agreed"* is a statement about two files, and which two
/// is the whole question. So this is the unit the report is made of, and
/// [`Baseline::may_be_called_parity`] is the one predicate that decides whether it may ever be
/// written down as fidelity.
#[derive(Clone, PartialEq, Debug)]
pub struct Baseline {
    /// What was compared — a preset's wire token, a probe character, a hatch's token.
    pub subject: String,
    /// Which artefact and which page it was on, so it can be found again.
    pub provenance: String,
    /// Where the reference came from.
    pub provider: ReferenceProvider,
    /// How much that provider's answer is worth. Derived from `provider`; carried rather than
    /// recomputed so that a stored record is readable without re-deriving the mapping.
    pub authority: ReferenceAuthority,
    /// What kind of drawing this is, which is what the exclusions are declared along.
    pub content: RenderedContent,
    /// What the comparison concluded.
    pub verdict: Verdict,
}

impl Baseline {
    /// A baseline from `provider` about `content`, with the exclusion already applied.
    ///
    /// **The exclusion is applied here rather than at the call site**, so a caller cannot forget it:
    /// pass a real verdict for excluded content and it is replaced by
    /// [`Verdict::NotEvidence`] carrying the provider's own reason. That is the difference between
    /// an exclusion that is enforced and one that is documented.
    #[must_use]
    pub fn new(
        subject: impl Into<String>,
        provenance: impl Into<String>,
        provider: ReferenceProvider,
        content: RenderedContent,
        verdict: Verdict,
    ) -> Self {
        let verdict = match provider.excludes(content) {
            Some(reason) => Verdict::NotEvidence {
                reason: reason.to_owned(),
            },
            None => verdict,
        };
        Self {
            subject: subject.into(),
            provenance: provenance.into(),
            provider,
            authority: provider.authority(),
            content,
            verdict,
        }
    }

    /// Whether this observation may be written down as *"this matches PowerPoint"*.
    ///
    /// Both halves are required and neither is sufficient: the provider must be authoritative
    /// **and** the verdict must be an agreement. A green LibreOffice run is not parity, and an
    /// Office run that disagreed is not parity either.
    #[must_use]
    pub fn may_be_called_parity(&self) -> bool {
        self.provider.is_authoritative() && self.verdict.is_pass()
    }

    /// Whether this observation must be re-adjudicated when an authoritative reference arrives.
    #[must_use]
    pub fn is_provisional(&self) -> bool {
        self.authority == ReferenceAuthority::Provisional
    }
}

impl core::fmt::Display for Baseline {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "{:<28} {:<12} {:<26} {}",
            self.subject,
            self.content.label(),
            self.provider.label(),
            self.verdict
        )
    }
}

/// Every baseline in `report` that must be re-adjudicated when Office's artefacts arrive.
///
/// The suite has to be *able to list them*: MJXOFF-165's requirement, and the thing that stops a
/// provisional record ageing into an unexamined one.
#[must_use]
pub fn provisional_baselines(report: &[Baseline]) -> Vec<&Baseline> {
    report
        .iter()
        .filter(|baseline| baseline.is_provisional())
        .collect()
}

/// How many of `report` may be called parity — which is zero until a person has run Office.
#[must_use]
pub fn parity_count(report: &[Baseline]) -> usize {
    report
        .iter()
        .filter(|baseline| baseline.may_be_called_parity())
        .count()
}
