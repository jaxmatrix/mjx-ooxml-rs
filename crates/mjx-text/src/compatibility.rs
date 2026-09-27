//! Measuring a substitute against the published metrics of the font it replaced.
//!
//! This is not only a test's business. The resolver runs [`verify_metric_compatibility`] every time
//! it substitutes, and the verdict goes into the substitution manifest — so a user asking "why does
//! this look different from what the author sent" gets an answer that says whether the *layout* can
//! be trusted, not merely that a swap happened. That is also what keeps
//! [`crate::reference`]'s table honest: a table nothing consults could say anything.

use crate::error::FontError;
use crate::face::FontFace;
use crate::reference::{reference_for_family, ADVANCE_TOLERANCE_PER_MILLE};

/// What comparing a substitute against the original's published metrics established.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MetricCompatibility {
    /// Nothing was substituted, so the question does not arise.
    NotSubstituted,

    /// Every character the published table covers has the same advance in the substitute, within
    /// [`ADVANCE_TOLERANCE_PER_MILLE`].
    Verified {
        /// How many characters were compared. A verdict resting on one character is a weaker
        /// verdict than one resting on ninety, and the caller can see which it has.
        characters_compared: u32,
        /// The largest disagreement found, in thousandths of an em. Always below the tolerance.
        worst_deviation_per_mille: f64,
    },

    /// The substitute's advances do **not** match the original's. Text set in this face will break
    /// into different lines from the ones the author saw.
    Divergent {
        /// How many characters were compared.
        characters_compared: u32,
        /// The largest disagreement found, in thousandths of an em.
        worst_deviation_per_mille: f64,
        /// The character that disagreed most.
        worst_character: char,
    },

    /// A substitution happened and nothing could be proved about it.
    Unverified {
        /// Why not.
        reason: UnverifiedReason,
    },
}

impl MetricCompatibility {
    /// Whether the substitution is known not to move a line break.
    #[must_use]
    pub fn preserves_line_breaks(self) -> bool {
        matches!(self, Self::NotSubstituted | Self::Verified { .. })
    }

    /// Whether the substitution is known to move one.
    #[must_use]
    pub fn is_divergent(self) -> bool {
        matches!(self, Self::Divergent { .. })
    }
}

/// Why a substitution could not be checked.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum UnverifiedReason {
    /// [`crate::reference`] has no entry for the original family at all.
    NoReferenceForTheOriginal,
    /// There is an entry, but it carries no published numbers — see the module documentation there
    /// for which families those are and what would fill them in.
    ReferenceCarriesNoPublishedNumbers,
    /// The substitute has no glyph for any character the published table covers, so there was
    /// nothing to compare. A Latin reference against a face that carries only Han, for instance.
    SubstituteCoversNoReferencedCharacter,
}

/// Compare `substitute` against the published metrics of `original_family`.
///
/// # Errors
///
/// [`FontError`] only if the substitute's own bytes will not re-open, which
/// [`FontFace::parse`] has already ruled out; it is returned rather than unwrapped because this
/// crate does not unwrap on a font.
pub fn verify_metric_compatibility(
    original_family: &str,
    substitute: &FontFace,
) -> Result<MetricCompatibility, FontError> {
    let Some(reference) = reference_for_family(original_family) else {
        return Ok(MetricCompatibility::Unverified {
            reason: UnverifiedReason::NoReferenceForTheOriginal,
        });
    };
    // `reference::tests::authority_and_content_agree` holds the two halves of this in step, so in
    // practice only one of them can be true at a time; both are tested because a table is data and
    // a check that relied on a test having run would be a check that relied on a test having run.
    if !reference.advance_authority.is_evidence() || reference.advances.is_empty() {
        return Ok(MetricCompatibility::Unverified {
            reason: UnverifiedReason::ReferenceCarriesNoPublishedNumbers,
        });
    }

    let reader = substitute.reader()?;
    let mut characters_compared = 0_u32;
    let mut worst_deviation = 0.0_f64;
    let mut worst_character = '\u{0}';

    for (character, _) in reference.advances {
        let (Some(expected), Some(measured)) = (
            reference.advance_for_character(*character),
            reader.advance_for_character(*character),
        ) else {
            continue;
        };
        characters_compared += 1;
        let deviation = measured.deviation_per_mille(expected);
        if deviation > worst_deviation {
            worst_deviation = deviation;
            worst_character = *character;
        }
    }

    if characters_compared == 0 {
        return Ok(MetricCompatibility::Unverified {
            reason: UnverifiedReason::SubstituteCoversNoReferencedCharacter,
        });
    }
    if worst_deviation > ADVANCE_TOLERANCE_PER_MILLE {
        return Ok(MetricCompatibility::Divergent {
            characters_compared,
            worst_deviation_per_mille: worst_deviation,
            worst_character,
        });
    }
    Ok(MetricCompatibility::Verified {
        characters_compared,
        worst_deviation_per_mille: worst_deviation,
    })
}
