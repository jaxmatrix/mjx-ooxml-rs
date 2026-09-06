//! The iOS path: the whole resolver, with system enumeration switched off.
//!
//! iOS does not expose system font files to a sandboxed process. Reaching them would need CoreText,
//! which is a C API, and `mjx-text` sits below the platform boundary `docs/UI_PLATFORM_PLAN.md` §3
//! draws at `mjx-paint`, so it may not link one. **Tier 1 is therefore permanently empty on that
//! platform**, and this suite exists so that fact is a supported configuration held by a test rather
//! than a special case discovered by the first person to run the thing on a phone.
//!
//! Everything below runs against `FontResolver::builder()` with no system tier at all: only the
//! bundled faces and the tier-3 policy.

mod support;

use mjx_text::{
    FontRequest, FontResolution, FontResolver, MetricCompatibility, ResolutionTier,
    UnverifiedReason,
};
use mjx_tokens::Tokens;
use support::bundled_font_directory;

/// A resolver in the shape iOS gets: nothing enumerated, only what the application bundles.
fn sandboxed_resolver() -> FontResolver {
    let resolver = FontResolver::builder()
        .with_bundled_font_directory(&bundled_font_directory())
        .expect("the committed asset directory is readable")
        .build();
    assert!(
        resolver.system_tier().is_empty(),
        "this suite is meaningless unless tier 1 really is empty"
    );
    assert!(
        !resolver.bundled_tier().is_empty(),
        "tier 2 must have the committed faces in it"
    );
    resolver
}

#[test]
fn every_microsoft_family_the_plan_names_resolves_with_no_system_fonts_at_all() {
    let mut resolver = sandboxed_resolver();

    for (requested, expected_substitute) in [
        ("Calibri", "Carlito"),
        ("Cambria", "Caladea"),
        ("Arial", "Liberation Sans"),
        ("Times New Roman", "Liberation Serif"),
        ("Courier New", "Liberation Mono"),
    ] {
        let resolution = resolver
            .resolve(&FontRequest::new(requested))
            .expect("a committed face re-opens");
        let FontResolution::Resolved(resolved) = resolution else {
            panic!("`{requested}` did not resolve through tier 2 with tier 1 empty");
        };
        assert_eq!(
            resolved.tier,
            ResolutionTier::Bundled,
            "`{requested}` should have been answered by the bundled tier"
        );
        assert_eq!(
            resolved.face.identity().family,
            expected_substitute,
            "`{requested}` should have resolved to `{expected_substitute}`"
        );
        assert!(resolved.is_substitution());
    }
}

/// Four of the five pairs carry published metrics, so their substitutions come back *verified*
/// rather than merely made — which is the difference between "we drew something" and "the lines
/// break where the author's did".
#[test]
fn the_substitutions_that_can_be_verified_are_verified_at_resolution_time() {
    let mut resolver = sandboxed_resolver();

    for requested in ["Calibri", "Arial", "Times New Roman", "Courier New"] {
        let FontResolution::Resolved(resolved) = resolver
            .resolve(&FontRequest::new(requested))
            .expect("a committed face re-opens")
        else {
            panic!("`{requested}` did not resolve");
        };
        assert!(
            matches!(
                resolved.metric_compatibility,
                MetricCompatibility::Verified { .. }
            ),
            "substituting for `{requested}` should be verified against published metrics, and the \
             verdict was {:?}",
            resolved.metric_compatibility
        );
        assert!(resolved.metric_compatibility.preserves_line_breaks());
    }

    // And the fifth is honest about being unproven rather than claiming a verification it has no
    // source for. See `tests/metric_compatibility.rs` for why Cambria has none.
    let FontResolution::Resolved(cambria) = resolver
        .resolve(&FontRequest::new("Cambria"))
        .expect("a committed face re-opens")
    else {
        panic!("`Cambria` did not resolve");
    };
    assert_eq!(
        cambria.metric_compatibility,
        MetricCompatibility::Unverified {
            reason: UnverifiedReason::ReferenceCarriesNoPublishedNumbers
        }
    );
}

/// A CSS generic is where every font stack ends, so it has to resolve on a device with nothing
/// installed — otherwise the chrome itself would have no type on iOS.
#[test]
fn the_generic_families_resolve_from_the_bundle_alone() {
    let mut resolver = sandboxed_resolver();

    for (generic, expected) in [
        ("sans-serif", "Liberation Sans"),
        ("serif", "Liberation Serif"),
        ("monospace", "Liberation Mono"),
        ("system-ui", "Liberation Sans"),
        ("ui-monospace", "Liberation Mono"),
    ] {
        let FontResolution::Resolved(resolved) = resolver
            .resolve(&FontRequest::new(generic))
            .expect("a committed face re-opens")
        else {
            panic!("the generic `{generic}` did not resolve");
        };
        assert_eq!(resolved.face.identity().family, expected);
        assert_eq!(resolved.tier, ResolutionTier::Bundled);
    }
}

/// The design tokens R01 generated name the chrome's type as CSS font stacks. Resolving one is the
/// whole reason this crate depends on `mjx-tokens`, and it has to work on the sandboxed device too.
#[test]
fn a_design_token_font_stack_resolves_from_the_bundle_alone() {
    let mut resolver = sandboxed_resolver();
    let tokens = Tokens::DEFAULTS;

    for (stack, expected) in [
        (&tokens.font.sans, "Liberation Sans"),
        (&tokens.font.serif, "Liberation Serif"),
        (&tokens.font.mono, "Liberation Mono"),
    ] {
        let template = FontRequest::new("");
        let FontResolution::Resolved(resolved) = resolver
            .resolve_stack(stack, &template)
            .expect("a committed face re-opens")
        else {
            panic!("the token stack `{}` did not resolve", stack.css());
        };
        assert_eq!(
            resolved.face.identity().family,
            expected,
            "`{}` should have fallen through to `{expected}`",
            stack.css()
        );
    }
}

/// Tier 3, the policy half. Nothing bundled can draw Han, and §10's answer is to name the subset
/// that would rather than to draw a row of missing-glyph boxes. There is no transport in this loop,
/// so the answer is a plan.
#[test]
fn a_script_no_local_face_covers_produces_a_tier_three_fetch_plan() {
    let mut resolver = sandboxed_resolver();

    for (characters, expected_family) in [
        (['漢', '字'], "Noto Sans SC"),
        (['ひ', 'ら'], "Noto Sans JP"),
        (['한', '글'], "Noto Sans KR"),
        (['ا', 'ب'], "Noto Naskh Arabic"),
        (['ש', 'ל'], "Noto Sans Hebrew"),
        (['क', 'ख'], "Noto Sans Devanagari"),
        (['ก', 'ข'], "Noto Sans Thai"),
    ] {
        let request = FontRequest::new("Calibri").requiring(&characters);
        let resolution = resolver
            .resolve(&request)
            .expect("a committed face re-opens");
        let FontResolution::FetchRequired(plan) = resolution else {
            panic!("{characters:?} should have produced a fetch plan, and produced {resolution:?}");
        };
        assert_eq!(plan.subset.family, expected_family);
        assert_eq!(plan.first_uncovered_character, characters[0]);
        assert_eq!(plan.requested_family, "Calibri");
        assert!(
            plan.subset.approximate_bytes > 0,
            "a fetch plan a user is asked to approve must be able to say how large the download is"
        );
    }
}

/// The coverage requirement must not fire when the bundle *can* draw the run: a Latin request with
/// Latin characters resolves locally, and never reaches tier 3.
#[test]
fn a_covered_run_never_reaches_tier_three() {
    let mut resolver = sandboxed_resolver();
    let characters: Vec<char> = "The quick brown fox, 0123456789.".chars().collect();
    let request = FontRequest::new("Calibri").requiring(&characters);

    let FontResolution::Resolved(resolved) = resolver
        .resolve(&request)
        .expect("a committed face re-opens")
    else {
        panic!("a Latin run should resolve from the bundle");
    };
    assert_eq!(resolved.face.identity().family, "Carlito");
    assert_eq!(resolved.tier, ResolutionTier::Bundled);
}

/// The complement of the suite: with tier 1 present, the same requests still resolve. The tier that
/// answers depends on what this machine has installed, so that is not asserted — what is asserted is
/// that enumerating the platform does not *break* anything the sandboxed path could do.
#[test]
fn enumerating_the_platform_resolves_the_same_families() {
    let mut resolver = FontResolver::builder()
        .with_platform_fonts()
        .with_bundled_font_directory(&bundled_font_directory())
        .expect("the committed asset directory is readable")
        .build();

    for requested in [
        "Calibri",
        "Cambria",
        "Arial",
        "Times New Roman",
        "Courier New",
        "sans-serif",
    ] {
        let resolution = resolver
            .resolve(&FontRequest::new(requested))
            .expect("an indexed face re-opens");
        assert!(
            resolution.face().is_some(),
            "`{requested}` did not resolve with the platform's fonts indexed"
        );
    }
}
