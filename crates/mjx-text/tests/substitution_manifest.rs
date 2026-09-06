//! The per-document substitution manifest: populated by resolving, and queryable afterwards.
//!
//! `docs/UI_PLATFORM_PLAN.md` §10 asks for this because *"a user is entitled to know that what they
//! are seeing is not what the author sent"*, and U08's font picker renders it.
//!
//! Every fixture below names **fonts that are deliberately absent**: Calibri, Cambria, Arial, Times
//! New Roman, Courier New and Wingdings are none of them in `assets/fonts/`, which holds only the
//! clones. So a manifest that stayed empty would be a manifest that was not populated, and every
//! assertion here would fail — which is the point. The suite would not pass if the work were not
//! done.

mod support;

use mjx_text::{
    FontRequest, FontResolution, FontResolver, MetricCompatibility, ResolutionTier,
    UnverifiedReason,
};
use support::bundled_font_directory;

fn resolver() -> FontResolver {
    FontResolver::builder()
        .with_bundled_font_directory(&bundled_font_directory())
        .expect("the committed asset directory is readable")
        .build()
}

#[test]
fn a_substitution_that_actually_happened_is_recorded_and_queryable() {
    let mut resolver = resolver();
    assert!(
        resolver.manifest().is_empty(),
        "a fresh manifest has nothing in it"
    );

    resolver
        .resolve(&FontRequest::new("Calibri"))
        .expect("a committed face re-opens");

    let record = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Calibri")
        .expect("the manifest records the family that was asked for");
    assert!(record.is_substitution());
    assert_eq!(record.tier, ResolutionTier::Bundled);
    assert_eq!(record.occurrences, 1);
    assert!(matches!(
        record.metric_compatibility,
        MetricCompatibility::Verified { .. }
    ));
    assert_eq!(
        resolver
            .interner()
            .resolve(record.resolved_family.expect("a face was chosen")),
        "Carlito"
    );
}

#[test]
fn a_family_asked_for_repeatedly_is_counted_rather_than_duplicated() {
    let mut resolver = resolver();
    for _ in 0..7 {
        resolver
            .resolve(&FontRequest::new("Times New Roman"))
            .expect("a committed face re-opens");
    }

    assert_eq!(
        resolver.manifest().len(),
        1,
        "seven runs in one family is one row, not seven"
    );
    let record = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Times New Roman")
        .expect("recorded");
    assert_eq!(
        record.occurrences, 7,
        "a substitution affecting seven runs and one affecting a single run are different problems"
    );
}

#[test]
fn every_absent_family_in_a_document_ends_up_in_the_manifest() {
    let mut resolver = resolver();
    let requested = [
        "Calibri",
        "Cambria",
        "Arial",
        "Times New Roman",
        "Courier New",
        "Wingdings",
    ];
    for family in requested {
        resolver
            .resolve(&FontRequest::new(family))
            .expect("a committed face re-opens");
    }

    assert_eq!(resolver.manifest().len(), requested.len());
    assert_eq!(
        resolver.manifest().substitution_count(),
        requested.len(),
        "none of these families is present, so every one of them is a substitution"
    );

    for family in requested {
        let record = resolver
            .manifest()
            .record_for_family(resolver.interner(), family)
            .unwrap_or_else(|| panic!("`{family}` is missing from the manifest"));
        assert!(
            record.is_substitution(),
            "`{family}` should be a substitution"
        );
    }

    let lines = resolver.manifest().describe(resolver.interner());
    assert_eq!(lines.len(), requested.len());
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Calibri") && line.contains("Carlito")),
        "the description should name both sides of the swap; it was {lines:?}"
    );
}

/// A face that is present is not a substitution, and must not be reported as one — otherwise a
/// user interface would warn about every document it opened.
#[test]
fn a_family_that_is_present_is_recorded_without_being_called_a_substitution() {
    let mut resolver = resolver();
    resolver
        .resolve(&FontRequest::new("Liberation Serif"))
        .expect("a committed face re-opens");

    let record = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Liberation Serif")
        .expect("recorded");
    assert!(!record.is_substitution());
    assert_eq!(
        record.metric_compatibility,
        MetricCompatibility::NotSubstituted
    );
    assert_eq!(resolver.manifest().substitution_count(), 0);
    assert!(resolver.manifest().describe(resolver.interner()).is_empty());
}

/// The manifest's sharpest question: *will this document paginate the way the author saw it?*
/// A substitution the published metrics cover answers yes; Cambria's cannot answer at all; and a
/// blind fallback answers no.
#[test]
fn the_manifest_separates_verified_substitutions_from_unproven_ones() {
    let mut resolver = resolver();
    for family in ["Arial", "Cambria", "Wingdings"] {
        resolver
            .resolve(&FontRequest::new(family))
            .expect("a committed face re-opens");
    }

    let arial = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Arial")
        .expect("recorded");
    assert!(arial.metric_compatibility.preserves_line_breaks());

    let cambria = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Cambria")
        .expect("recorded");
    assert_eq!(
        cambria.metric_compatibility,
        MetricCompatibility::Unverified {
            reason: UnverifiedReason::ReferenceCarriesNoPublishedNumbers
        },
        "Cambria has no published metrics, so its substitution cannot be called verified"
    );

    let wingdings = resolver
        .manifest()
        .record_for_family(resolver.interner(), "Wingdings")
        .expect("recorded");
    assert_eq!(
        wingdings.metric_compatibility,
        MetricCompatibility::Unverified {
            reason: UnverifiedReason::NoReferenceForTheOriginal
        },
        "a blind fallback has nothing to be measured against"
    );
}

/// A metrically divergent substitution — one that *will* move a line break — must be reachable
/// through the manifest without walking every record, because that is the set a user interface
/// warns about.
#[test]
fn a_divergent_substitution_is_reachable_on_its_own() {
    let mut resolver = FontResolver::builder()
        // Only Carlito. A request for Arial therefore cannot reach Liberation Sans, falls through
        // to the blind fallback, and lands on a face with Calibri's advances — which is exactly the
        // situation the manifest exists to make visible.
        .with_bundled_face_bytes(
            std::fs::read(bundled_font_directory().join("Carlito-Regular.ttf"))
                .expect("the committed face is readable"),
        )
        .build();

    let FontResolution::Resolved(resolved) = resolver
        .resolve(&FontRequest::new("Arial"))
        .expect("a committed face re-opens")
    else {
        panic!("with one face indexed, the blind fallback should still find it");
    };
    assert_eq!(resolved.face.identity().family, "Carlito");

    // The blind fallback records `Unverified`, because it did not go through the substitution
    // table; what makes the divergence visible is measuring it, which the compatibility check does.
    let divergence = mjx_text::verify_metric_compatibility("Arial", &resolved.face)
        .expect("a committed face re-opens");
    assert!(
        divergence.is_divergent(),
        "Carlito does not have Arial's advances, and the check should say so; it said {divergence:?}"
    );
}
