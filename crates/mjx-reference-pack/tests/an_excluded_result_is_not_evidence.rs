//! **The exclusion: attached to the provider, named in the output, lifting by itself — and never a
//! pass.**
//!
//! # The two traps, and this file holds both ends
//!
//! MJXOFF-165 warns about a *provisional baseline read as parity*: approve baselines against
//! LibreOffice now, keep them silently when PowerPoint arrives, and LibreOffice's quirks are
//! ratified into the ledger while looking like fidelity.
//!
//! MJXOFF-207 warns about the mirror image: *a reference defect read as our defect*. LibreOffice's
//! export is not reliable for shades and gradients. An agent sees the pixel difference, concludes
//! the renderer is wrong, and spends a child editing correct code until it matches a bad export.
//! **Nothing in a pixel diff distinguishes the two directions**, which is why the distinction is
//! made before the diff rather than after it.
//!
//! # The four properties, each asserted
//!
//! 1. **It is a property of the provider**, so it lifts automatically when the Office artefacts
//!    arrive: [`a_gradient_is_excluded_under_libreoffice_and_compared_under_office`].
//! 2. **It is named**, not silent: [`every_exclusion_gives_its_reason_in_a_sentence`].
//! 3. **It is neither a pass nor a failure**: [`an_excluded_verdict_is_not_a_pass_and_not_a_failure`].
//! 4. **It does not cover everything.** An exclusion that covered the whole sheet would prove
//!    nothing, and one carried onto a sheet that did not need it would quietly remove that sheet
//!    from the comparison. The preset decks are authored with solid fills and solid strokes for
//!    exactly this reason, and [`the_geometry_decks_are_excluded_from_nothing`] asserts the
//!    emptiness rather than leaving it to be believed.
//!
//! # And the one that is easy to forget
//!
//! [`a_libreoffice_run_can_never_produce_parity`]. Every row a LibreOffice pass can produce carries
//! [`ReferenceAuthority::Provisional`], and `parity_count` over any such report is **zero by
//! construction** — not by tolerance, not by luck, and not by anybody remembering.

use mjx_reference_pack::authority::{
    parity_count, provisional_baselines, Baseline, ReferenceProvider, RenderedContent, Verdict,
};
use mjx_text::ReferenceAuthority;

/// A verdict that would be a pass if nothing excluded it.
fn an_agreement() -> Verdict {
    Verdict::Agreed {
        differing_fraction: 0.0,
        allowed: 0.02,
    }
}

/// A verdict that would be a failure if nothing excluded it.
fn a_disagreement() -> Verdict {
    Verdict::Disagreed {
        differing_fraction: 0.9,
        allowed: 0.02,
        worst: "somewhere".to_owned(),
    }
}

#[test]
fn a_gradient_is_excluded_under_libreoffice_and_compared_under_office() {
    let excluded = Baseline::new(
        "a gradient-filled shape",
        "a fixture",
        ReferenceProvider::LibreOffice,
        RenderedContent::GradientFill,
        an_agreement(),
    );
    assert!(
        matches!(excluded.verdict, Verdict::NotEvidence { .. }),
        "a gradient compared against LibreOffice produced {excluded:?}"
    );

    // The same fixture, the same verdict, the authoritative provider: a real comparison.
    let compared = Baseline::new(
        "a gradient-filled shape",
        "a fixture",
        ReferenceProvider::OfficeExport,
        RenderedContent::GradientFill,
        an_agreement(),
    );
    assert!(
        compared.verdict.is_pass(),
        "the exclusion did not lift for an authoritative provider: {compared:?}"
    );
    assert!(compared.may_be_called_parity());
    println!("excluded: {excluded}\ncompared: {compared}");
}

/// The exclusion cannot be routed around by handing it a *failure* either — which matters more than
/// the agreement case, because a failure is what an agent would go and "fix".
#[test]
fn an_excluded_disagreement_is_still_not_a_failure() {
    let row = Baseline::new(
        "a shaded shape",
        "a fixture",
        ReferenceProvider::LibreOffice,
        RenderedContent::ShadedFill,
        a_disagreement(),
    );
    assert!(
        !row.verdict.is_failure() && !row.verdict.is_pass(),
        "a shaded fill compared against LibreOffice produced {row:?}; an agent would have gone and \
         changed the renderer to match a bad export"
    );
}

#[test]
fn every_exclusion_gives_its_reason_in_a_sentence() {
    let mut excluded = 0usize;
    for provider in ReferenceProvider::ALL {
        for content in RenderedContent::ALL {
            let Some(reason) = provider.excludes(content) else {
                continue;
            };
            excluded += 1;
            assert!(
                reason.len() > 40,
                "`{}` excludes `{}` with {reason:?}, which tells a reader nothing",
                provider.label(),
                content.label()
            );
            // The reason has to say which side is at fault, or it does not answer the question it
            // exists to answer.
            let row = Baseline::new("x", "y", provider, content, an_agreement());
            let Verdict::NotEvidence { reason: printed } = &row.verdict else {
                panic!(
                    "`{}`/`{}` excluded and then compared",
                    provider.label(),
                    content.label()
                );
            };
            assert_eq!(
                printed, reason,
                "the printed reason is not the declared one"
            );
        }
    }
    // Six kinds and three providers is eighteen pairs; a table that excluded none of them would
    // pass every assertion above.
    assert!(
        excluded >= 9,
        "only {excluded} of the eighteen (provider, content) pairs are excluded, which is not the \
         table this crate documents"
    );
}

#[test]
fn an_excluded_verdict_is_not_a_pass_and_not_a_failure() {
    let verdict = Verdict::NotEvidence {
        reason: "because".to_owned(),
    };
    assert!(!verdict.is_pass());
    assert!(!verdict.is_failure());
    assert!(!verdict.is_evidence());
    assert_eq!(verdict.word(), "excluded");
    // And the two that are evidence really are, so the predicate is not simply always false.
    assert!(an_agreement().is_evidence() && a_disagreement().is_evidence());
    assert!(an_agreement().is_pass() && a_disagreement().is_failure());
}

/// **The emptiness, said out loud.** An exclusion that covered everything would prove nothing.
#[test]
fn the_geometry_decks_are_excluded_from_nothing() {
    // What the preset decks actually contain: an outline, a solid fill, and — on the caption strip,
    // outside every comparison window — text.
    for content in [
        RenderedContent::Outline,
        RenderedContent::SolidFill,
        RenderedContent::TextLayout,
    ] {
        for provider in [
            ReferenceProvider::LibreOffice,
            ReferenceProvider::OfficeExport,
        ] {
            assert!(
                provider.excludes(content).is_none(),
                "`{}` excludes `{}`, so the preset sheets would be removed from the comparison the \
                 exclusion exists to protect",
                provider.label(),
                content.label()
            );
        }
    }
    // And the layout tier is never excluded by anybody, which is the half MJXOFF-165 insists on:
    // a word box says nothing about fill.
    for provider in ReferenceProvider::ALL {
        if provider == ReferenceProvider::None {
            continue;
        }
        assert!(
            provider.excludes(RenderedContent::TextLayout).is_none(),
            "`{}` excludes the layout tier",
            provider.label()
        );
    }
}

/// The one that keeps the whole crate honest.
#[test]
fn a_libreoffice_run_can_never_produce_parity() {
    let report: Vec<Baseline> = RenderedContent::ALL
        .into_iter()
        .map(|content| {
            Baseline::new(
                content.label(),
                "a LibreOffice run",
                ReferenceProvider::LibreOffice,
                content,
                an_agreement(),
            )
        })
        .collect();

    assert_eq!(
        parity_count(&report),
        0,
        "a LibreOffice run produced a row that may be called parity"
    );
    assert_eq!(
        provisional_baselines(&report).len(),
        report.len(),
        "every row of a LibreOffice run must be listable as provisional, so it can be \
         re-adjudicated rather than ageing into an unexamined pass"
    );
    for row in &report {
        assert_eq!(row.authority, ReferenceAuthority::Provisional);
        assert!(row.is_provisional());
        assert!(!row.authority.is_evidence());
    }
}

/// The three providers map onto the three authorities, one each — so `Provisional` is not merely
/// declared, it is the answer for a provider this crate actually uses.
#[test]
fn the_three_providers_are_three_distinct_authorities() {
    let authorities: Vec<ReferenceAuthority> = ReferenceProvider::ALL
        .into_iter()
        .map(ReferenceProvider::authority)
        .collect();
    assert_eq!(
        authorities,
        vec![
            ReferenceAuthority::Unverified,
            ReferenceAuthority::Provisional,
            ReferenceAuthority::Published,
        ]
    );
    assert_eq!(
        ReferenceProvider::ALL
            .into_iter()
            .filter(|provider| provider.is_authoritative())
            .count(),
        1,
        "exactly one provider is authoritative, and a person has to run it"
    );
}
