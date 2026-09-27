//! **The two traps, and they point in opposite directions.**
//!
//! MJXOFF-165 states the first: *a provisional baseline read as parity*. Approve against LibreOffice
//! now, keep the record quietly when PowerPoint arrives, and LibreOffice's quirks are ratified into
//! the ledger — worse than having no reference, because it *looks* like parity.
//!
//! The user's own constraint states the second, and it is the mirror image: *a reference defect read
//! as our defect*. LibreOffice's PDF export does not reproduce gradients or shades, so a pixel
//! difference there is the reference's fault — and **nothing in a pixel diff says which way round it
//! is**. An agent sees the difference, concludes the renderer is wrong, and spends a child "fixing"
//! correct code until it matches a bad export.
//!
//! Four properties answer both, and every one is asserted here rather than described:
//!
//! 1. **The exclusion is a property of the provider, not of the fixture.** So it lifts by itself for
//!    an authoritative one, and lifting it does not require re-litigating it.
//! 2. **An excluded result is *not evidence*** — neither a pass nor a failure — and it carries the
//!    reason as a sentence, so a reader sees *why* rather than seeing a gap.
//! 3. **The exclusion is named, never silent.** A quietly-skipped case is indistinguishable from a
//!    check that never existed.
//! 4. **Solid fill, stroke, geometry and text layout are excluded from nothing.** An exclusion that
//!    covered everything would prove nothing, and one carried onto content that did not need it
//!    would quietly remove that content from the comparison.

use mjx_render_oracle::authority::{
    parity_count, provisional_baselines, Baseline, ReferenceProvider, RenderedContent, Verdict,
};
use mjx_render_oracle::specimen::SPECIMENS;
use mjx_render_oracle::{plate, Baselines};

/// A verdict that would be a pass if the provider were allowed to speak.
fn an_agreement() -> Verdict {
    Verdict::Agreed {
        differing_fraction: 0.0,
        allowed: 0.02,
    }
}

#[test]
fn the_same_content_is_excluded_under_libreoffice_and_compared_under_office() {
    // **The proof MJXOFF-165 asks for in as many words**: the same fixture yields a real comparison
    // under `OfficeExport` and an excluded result under `LibreOffice`.
    for content in [RenderedContent::GradientFill, RenderedContent::ShadedFill] {
        let provisional = Baseline::new(
            "gradient-panel",
            "the plate gallery, plate 4",
            ReferenceProvider::LibreOffice,
            content,
            an_agreement(),
        );
        assert!(
            !provisional.verdict.is_evidence(),
            "a {} compared against LibreOffice produced evidence",
            content.label()
        );
        assert!(!provisional.verdict.is_pass());
        assert!(!provisional.verdict.is_failure());

        let authoritative = Baseline::new(
            "gradient-panel",
            "the plate gallery, plate 4",
            ReferenceProvider::OfficeExport,
            content,
            an_agreement(),
        );
        assert_eq!(
            authoritative.verdict,
            an_agreement(),
            "the exclusion did not lift for an authoritative provider, so it is a property of the \
             fixture after all"
        );
        assert!(authoritative.may_be_called_parity());
    }
}

#[test]
fn an_excluded_result_says_why_rather_than_leaving_a_gap() {
    let excluded = Baseline::new(
        "gradient-panel",
        "the plate gallery",
        ReferenceProvider::LibreOffice,
        RenderedContent::GradientFill,
        an_agreement(),
    );
    let Verdict::NotEvidence { reason } = &excluded.verdict else {
        panic!("a gradient against LibreOffice is not excluded")
    };
    assert!(
        reason.contains("does not reproduce gradients"),
        "the reason does not say what is wrong with the reference: {reason}"
    );
    assert!(
        reason.contains("the reference's and not ours"),
        "**the reason does not say which side to suspect**, which is the whole of the second trap: \
         {reason}"
    );
    assert_eq!(excluded.verdict.word(), "excluded");
    assert!(excluded.to_string().contains("not evidence"));

    // The hatch exclusion is a *different* argument and says so — a single "LibreOffice is
    // unreliable" flag would make the two indistinguishable, and one of them lifts for a reason the
    // other does not.
    let hatch = ReferenceProvider::LibreOffice
        .excludes(RenderedContent::PatternFill)
        .expect("hatches are excluded");
    assert!(
        hatch.contains("would ratify"),
        "the hatch exclusion does not give its own reason: {hatch}"
    );
    assert_ne!(
        hatch,
        ReferenceProvider::LibreOffice
            .excludes(RenderedContent::GradientFill)
            .expect("gradients are excluded")
    );
}

#[test]
fn the_content_the_specimens_are_made_of_is_excluded_from_nothing() {
    // The load-bearing half. Three of the five specimens are solid fill and one is an outline; if
    // those were excluded too, the whole corpus would be outside every comparison and the suite
    // would be green because it checked nothing.
    for content in [
        RenderedContent::Outline,
        RenderedContent::SolidFill,
        RenderedContent::TextLayout,
    ] {
        assert_eq!(
            ReferenceProvider::LibreOffice.excludes(content),
            None,
            "LibreOffice excludes {}, which is most of the corpus",
            content.label()
        );
    }
    let unexcluded = SPECIMENS
        .iter()
        .filter(|specimen| {
            ReferenceProvider::LibreOffice
                .excludes(specimen.content)
                .is_none()
        })
        .count();
    assert!(
        unexcluded >= 4,
        "only {unexcluded} of {} specimens survive a LibreOffice comparison; an exclusion that \
         covers the corpus proves nothing",
        SPECIMENS.len()
    );
    let excluded = SPECIMENS.len() - unexcluded;
    assert!(
        excluded >= 1,
        "no specimen is excluded from anything, so the exclusion is unexercised — a rule with no \
         case is a rule nobody would notice going wrong"
    );
}

#[test]
fn a_libreoffice_run_can_never_produce_parity() {
    // By construction rather than by policy: `is_authoritative` is true for an Office export alone,
    // so every observation a preliminary run can make is provisional whatever it concluded.
    let report: Vec<Baseline> = SPECIMENS
        .iter()
        .map(|specimen| {
            Baseline::new(
                specimen.name,
                "a preliminary run",
                ReferenceProvider::LibreOffice,
                specimen.content,
                an_agreement(),
            )
        })
        .collect();
    assert_eq!(
        parity_count(&report),
        0,
        "**a LibreOffice run produced parity.** A green preliminary comparison means \"nothing \
         obviously broke\" and never means parity; if this ever passes, the sentence in \
         `lib.rs` that says so has become false."
    );
    assert_eq!(
        provisional_baselines(&report).len(),
        report.len(),
        "the suite cannot list every baseline that must be re-adjudicated, which is how a \
         provisional record ages into an unexamined one"
    );
    for baseline in &report {
        assert!(baseline.is_provisional());
    }

    // And an Office run *can*, which is what stops the case above passing for a
    // `may_be_called_parity` that answers `false` to everything.
    let authoritative = Baseline::new(
        "solid-panels",
        "an Office export",
        ReferenceProvider::OfficeExport,
        RenderedContent::SolidFill,
        an_agreement(),
    );
    assert!(authoritative.may_be_called_parity());
    assert!(!authoritative.is_provisional());
    assert_eq!(parity_count(&[authoritative]), 1);
}

#[test]
fn the_gallery_says_it_is_not_a_parity_claim_and_names_every_exclusion() {
    let set = plate::generate(ReferenceProvider::LibreOffice, &Baselines::committed())
        .expect("the plates generate");
    let page = mjx_render_oracle::gallery::render(&set);

    assert!(
        page.contains("not a parity claim"),
        "the gallery does not say what it is not"
    );
    assert!(
        page.contains("LibreOffice (preliminary)"),
        "the gallery does not name the provider that produced its reference"
    );
    assert!(
        page.contains("Not evidence."),
        "the gallery does not mark its excluded plate"
    );
    assert!(
        page.contains("does not reproduce gradients"),
        "**the gallery excludes a plate without saying why.** A quietly-skipped case is \
         indistinguishable from a check that never existed."
    );

    let excluded: Vec<&str> = set
        .plates
        .iter()
        .filter(|plate| plate.exclusion.is_some())
        .map(|plate| plate.name.as_str())
        .collect();
    assert_eq!(
        excluded,
        vec!["gradient-panel"],
        "exactly the gradient plate is excluded under LibreOffice, and this run excluded {excluded:?}"
    );
    for plate in &set.plates {
        assert!(
            !plate.parity,
            "`{}` claims parity against a non-authoritative provider",
            plate.name
        );
    }

    // And under no reference at all, *everything* is excluded and the page says so — the honest
    // state of the project rather than an empty gallery that looks like a pass.
    let none = plate::generate(ReferenceProvider::None, &Baselines::committed())
        .expect("the plates generate");
    assert!(none.plates.iter().all(|plate| plate.exclusion.is_some()));
    assert!(mjx_render_oracle::gallery::render(&none).contains("no reference at all"));
}
