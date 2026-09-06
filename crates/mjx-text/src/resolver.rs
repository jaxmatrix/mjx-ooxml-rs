//! The three tiers, resolved in order.
//!
//! `docs/UI_PLATFORM_PLAN.md` §10 lays out the order and this module is it, with the document's own
//! embedded faces ahead of all three because a face the author shipped is by definition the right
//! answer:
//!
//! 1. **Embedded** — faces the package carries. PowerPoint and Word can both embed.
//! 2. **System** — whatever the platform exposes. **Empty on iOS, and that is a supported
//!    configuration**, not a failure; see [`crate::FaceIndex`].
//! 3. **Bundled** — the metric-compatible set the application ships.
//! 4. **The substitution table** — consulted *before* any blind fallback, because a blind fallback
//!    onto a face with different advances re-paginates the document and the table's whole purpose is
//!    that it does not.
//! 5. **The generic families** — where a CSS stack, or a document naming `sans-serif`, ends.
//! 6. **Tier 3** — the fetchable subsets. **This loop ships no transport**, so the answer is a
//!    [`FetchPlan`], and the caller decides.
//! 7. **A blind fallback**, last, and recorded as such.
//!
//! Every step records what it did in the [`SubstitutionManifest`], so the record cannot drift from
//! what was drawn.

use std::sync::Arc;

use mjx_ooxml_core::{Interner, Symbol};
use mjx_tokens::FontStack;

use crate::compatibility::{verify_metric_compatibility, MetricCompatibility, UnverifiedReason};
use crate::error::FontError;
use crate::face::FontFace;
use crate::index::{FaceIndex, FontRequest, ResolutionTier};
use crate::manifest::SubstitutionManifest;
use crate::substitution::{
    fetchable_subset_for_character, substitution_for_family, FetchPlan, GenericFamily,
};

/// A face the resolver settled on.
#[derive(Clone, Debug)]
pub struct ResolvedFont {
    /// The face itself.
    pub face: Arc<FontFace>,
    /// Which tier answered.
    pub tier: ResolutionTier,
    /// The family the document asked for, interned.
    pub requested_family: Symbol,
    /// The family that was used, interned. Equal to `requested_family` when nothing was
    /// substituted.
    pub resolved_family: Symbol,
    /// Whether the swap — if there was one — moves a line break.
    pub metric_compatibility: MetricCompatibility,
}

impl ResolvedFont {
    /// Whether the face used differs from the face asked for.
    #[must_use]
    pub fn is_substitution(&self) -> bool {
        self.requested_family != self.resolved_family
    }
}

/// What resolving a request produced.
#[derive(Clone, Debug)]
pub enum FontResolution {
    /// A face, from one of the three local tiers.
    Resolved(ResolvedFont),
    /// Nothing local can draw the characters, and tier 3 says which subset would.
    ///
    /// There is no transport in this loop; this is the policy answer §10 asks for, and acting on it
    /// belongs to whichever child grows a font-fetching surface.
    FetchRequired(FetchPlan),
    /// Nothing local, and no fetchable subset covers it either — an empty device asked for a script
    /// nothing here knows.
    Unresolvable {
        /// The family the document asked for, interned.
        requested_family: Symbol,
    },
}

impl FontResolution {
    /// The face, when there is one.
    #[must_use]
    pub fn face(&self) -> Option<&Arc<FontFace>> {
        match self {
            Self::Resolved(resolved) => Some(&resolved.face),
            Self::FetchRequired(_) | Self::Unresolvable { .. } => None,
        }
    }
}

/// The three tiers, the substitution table and the manifest, for one document.
///
/// It is a per-document object because the manifest is: two documents open at once have different
/// embedded faces and different substitution histories. The system and bundled indexes could be
/// shared, and a later child that wants to share them can put an [`Arc`] around them without
/// changing this surface.
#[derive(Debug)]
pub struct FontResolver {
    embedded: FaceIndex,
    system: FaceIndex,
    bundled: FaceIndex,
    interner: Interner,
    manifest: SubstitutionManifest,
}

impl FontResolver {
    /// Start building a resolver.
    #[must_use]
    pub fn builder() -> FontResolverBuilder {
        FontResolverBuilder {
            embedded: FaceIndex::empty(),
            system: FaceIndex::empty(),
            bundled: FaceIndex::empty(),
        }
    }

    /// The system tier, for inspection.
    #[must_use]
    pub fn system_tier(&self) -> &FaceIndex {
        &self.system
    }

    /// The bundled tier, for inspection.
    #[must_use]
    pub fn bundled_tier(&self) -> &FaceIndex {
        &self.bundled
    }

    /// The embedded tier, for inspection.
    #[must_use]
    pub fn embedded_tier(&self) -> &FaceIndex {
        &self.embedded
    }

    /// Add a face the document carried. See [`crate::EmbeddedFont::decode`] for the obfuscated
    /// case, which must be decoded before it gets here.
    pub fn add_embedded_face(&mut self, data: Vec<u8>) {
        self.embedded.add_bytes(data);
    }

    /// What has been substituted so far.
    #[must_use]
    pub fn manifest(&self) -> &SubstitutionManifest {
        &self.manifest
    }

    /// The interner the manifest's symbols belong to.
    #[must_use]
    pub fn interner(&self) -> &Interner {
        &self.interner
    }

    /// Resolve one request.
    ///
    /// # Errors
    ///
    /// [`FontError`] only when a face that was indexed will not parse — a truncated file, or one
    /// that vanished between being indexed and being read. A family that is simply absent is not an
    /// error; it is a substitution, and it is in the manifest.
    pub fn resolve(&mut self, request: &FontRequest<'_>) -> Result<FontResolution, FontError> {
        self.resolve_families(request, std::slice::from_ref(&request.family))
    }

    /// Resolve a CSS font list — a `mjx-tokens` [`FontStack`] is one, and so is a document's own
    /// comma-separated list — by trying each family in order and taking the first that answers.
    ///
    /// **The blind fallback happens once, after the whole stack**, not once per entry. That
    /// distinction is the difference between `"Nunito Sans", system-ui, sans-serif` landing on the
    /// sans-serif the stack asked for and it landing on whatever face the bundle happens to list
    /// first, because `Nunito Sans` is absent and a per-entry fallback would answer before
    /// `system-ui` was ever tried.
    ///
    /// The manifest records the substitution under the stack's **first** family, which is the one
    /// the author named.
    ///
    /// # Errors
    ///
    /// As [`FontResolver::resolve`].
    pub fn resolve_stack(
        &mut self,
        stack: &FontStack,
        template: &FontRequest<'_>,
    ) -> Result<FontResolution, FontError> {
        let families: Vec<String> = stack.faces().map(str::to_owned).collect();
        let borrowed: Vec<&str> = families.iter().map(String::as_str).collect();
        self.resolve_families(template, &borrowed)
    }

    /// The one resolution path, over a list of families to try in order.
    fn resolve_families(
        &mut self,
        template: &FontRequest<'_>,
        families: &[&str],
    ) -> Result<FontResolution, FontError> {
        // An empty list is a list with no families in it, which nothing can resolve. It is recorded
        // under the empty name so that even this shows up in the manifest rather than vanishing.
        let Some(first) = families.first() else {
            let requested_family = self.interner.intern("");
            self.manifest.record(
                requested_family,
                None,
                ResolutionTier::LazilyFetched,
                unverified(),
            );
            return Ok(FontResolution::Unresolvable { requested_family });
        };
        let requested_family = self.interner.intern(first);

        // Steps 1–5, for each family in turn: the three local tiers, the substitution table, and
        // the generic families.
        for family in families {
            let request = template.for_family(family);
            if let Some((face, tier, compatibility)) = self.search_named_tiers(&request)? {
                let resolved_family = self.interner.intern(&face.identity().family);
                return Ok(self.record_resolution(
                    requested_family,
                    resolved_family,
                    face,
                    tier,
                    compatibility,
                ));
            }
        }

        // 6. Tier 3: no local face covers what the run needs, so say which subset would.
        let request = template.for_family(first);
        if let Some(plan) = self.plan_a_fetch(&request) {
            self.manifest.record(
                requested_family,
                None,
                ResolutionTier::LazilyFetched,
                unverified(),
            );
            return Ok(FontResolution::FetchRequired(plan));
        }

        // 7. A blind fallback, last, and only once the whole list is exhausted: any face that can
        //    draw what the run needs.
        if let Some((face, tier)) = self.search_last_resort(&request)? {
            let resolved_family = self.interner.intern(&face.identity().family);
            return Ok(self.record_resolution(
                requested_family,
                resolved_family,
                face,
                tier,
                unverified(),
            ));
        }

        self.manifest.record(
            requested_family,
            None,
            ResolutionTier::LazilyFetched,
            unverified(),
        );
        Ok(FontResolution::Unresolvable { requested_family })
    }

    /// Steps 1 to 5 for one family: everything that resolves it *by name*, and nothing blind.
    fn search_named_tiers(
        &mut self,
        request: &FontRequest<'_>,
    ) -> Result<Option<(Arc<FontFace>, ResolutionTier, MetricCompatibility)>, FontError> {
        // 1–3. The three local tiers, under the name that was actually written.
        if let Some((face, tier)) = self.search_local_tiers(request)? {
            return Ok(Some((face, tier, MetricCompatibility::NotSubstituted)));
        }

        // 4. The substitution table, before any blind fallback.
        if let Some(rule) = substitution_for_family(request.family) {
            for substitute in rule.substitutes {
                let Some((face, tier)) =
                    self.search_local_tiers(&request.for_family(substitute))?
                else {
                    continue;
                };
                let compatibility = if rule.is_metric_compatible {
                    verify_metric_compatibility(rule.original, &face)?
                } else {
                    unverified()
                };
                return Ok(Some((face, tier, compatibility)));
            }
        }

        // 5. The generic families — where a CSS stack ends, and where a document naming `serif`
        //    lands.
        if let Some(generic) = GenericFamily::from_css_keyword(request.family) {
            if let Some((face, tier)) = self.search_generic(generic, request)? {
                return Ok(Some((face, tier, unverified())));
            }
        }

        Ok(None)
    }

    fn record_resolution(
        &mut self,
        requested_family: Symbol,
        resolved_family: Symbol,
        face: Arc<FontFace>,
        tier: ResolutionTier,
        metric_compatibility: MetricCompatibility,
    ) -> FontResolution {
        self.manifest.record(
            requested_family,
            Some(resolved_family),
            tier,
            metric_compatibility,
        );
        FontResolution::Resolved(ResolvedFont {
            face,
            tier,
            requested_family,
            resolved_family,
            metric_compatibility,
        })
    }

    /// The three local tiers in order, skipping any face that cannot draw what the run needs.
    fn search_local_tiers(
        &mut self,
        request: &FontRequest<'_>,
    ) -> Result<Option<(Arc<FontFace>, ResolutionTier)>, FontError> {
        for tier in [
            ResolutionTier::Embedded,
            ResolutionTier::System,
            ResolutionTier::Bundled,
        ] {
            let index = match tier {
                ResolutionTier::Embedded => &mut self.embedded,
                ResolutionTier::System => &mut self.system,
                ResolutionTier::Bundled => &mut self.bundled,
                ResolutionTier::LazilyFetched => continue,
            };
            let Some(face) = index.best_match(request)? else {
                continue;
            };
            if covers_everything(&face, request.required_characters)? {
                return Ok(Some((face, tier)));
            }
        }
        Ok(None)
    }

    fn search_generic(
        &mut self,
        generic: GenericFamily,
        request: &FontRequest<'_>,
    ) -> Result<Option<(Arc<FontFace>, ResolutionTier)>, FontError> {
        for candidate in generic.candidate_families() {
            if let Some(found) = self.search_local_tiers(&request.for_family(candidate))? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Any indexed face at all that can draw what the run needs. Deliberately last, and deliberately
    /// recorded as unverified: this is the "blind fallback" §10 says the substitution table exists
    /// to come before.
    fn search_last_resort(
        &mut self,
        request: &FontRequest<'_>,
    ) -> Result<Option<(Arc<FontFace>, ResolutionTier)>, FontError> {
        for tier in [
            ResolutionTier::Embedded,
            ResolutionTier::System,
            ResolutionTier::Bundled,
        ] {
            let families = match tier {
                ResolutionTier::Embedded => self.embedded.families(),
                ResolutionTier::System => self.system.families(),
                ResolutionTier::Bundled => self.bundled.families(),
                ResolutionTier::LazilyFetched => continue,
            };
            for family in families {
                let index = match tier {
                    ResolutionTier::Embedded => &mut self.embedded,
                    ResolutionTier::System => &mut self.system,
                    ResolutionTier::Bundled => &mut self.bundled,
                    ResolutionTier::LazilyFetched => continue,
                };
                let Some(face) = index.best_match(&request.for_family(&family))? else {
                    continue;
                };
                if covers_everything(&face, request.required_characters)? {
                    return Ok(Some((face, tier)));
                }
            }
        }
        Ok(None)
    }

    /// The tier-3 answer: the subset that would cover the first character nothing local can draw.
    fn plan_a_fetch(&self, request: &FontRequest<'_>) -> Option<FetchPlan> {
        for character in request.required_characters {
            if let Some(subset) = fetchable_subset_for_character(*character) {
                return Some(FetchPlan {
                    requested_family: request.family.to_owned(),
                    subset,
                    first_uncovered_character: *character,
                });
            }
        }
        None
    }
}

/// The verdict for a resolution nothing published can speak to: a stylistic stand-in, a generic, or
/// the blind fallback.
fn unverified() -> MetricCompatibility {
    MetricCompatibility::Unverified {
        reason: UnverifiedReason::NoReferenceForTheOriginal,
    }
}

fn covers_everything(face: &FontFace, characters: &[char]) -> Result<bool, FontError> {
    if characters.is_empty() {
        return Ok(true);
    }
    let reader = face.reader()?;
    Ok(characters.iter().all(|character| reader.covers(*character)))
}

/// Assembles a [`FontResolver`] tier by tier.
///
/// The system tier is **not** enumerated unless [`FontResolverBuilder::with_platform_fonts`] is
/// called. That default is deliberate: enumerating a desktop's fonts costs tens of milliseconds and
/// a few megabytes, an iOS build can never do it at all, and a caller who has not said which tiers
/// it wants should get the cheap, portable one rather than a surprise.
#[derive(Debug)]
pub struct FontResolverBuilder {
    embedded: FaceIndex,
    system: FaceIndex,
    bundled: FaceIndex,
}

impl FontResolverBuilder {
    /// Enumerate the platform's fonts into tier 1.
    ///
    /// On iOS this indexes nothing, because the sandbox exposes nothing, and the resolver goes on
    /// working through tiers 2 and 3.
    #[must_use]
    pub fn with_platform_fonts(mut self) -> Self {
        self.system = FaceIndex::from_platform();
        self
    }

    /// Index `directory` as tier 1, instead of the platform's own. Useful for a test, and for a
    /// platform whose fonts live somewhere `fontdb` does not look.
    ///
    /// # Errors
    ///
    /// [`FontError::UnreadableDirectory`] when the directory cannot be listed.
    pub fn with_system_font_directory(
        mut self,
        directory: &std::path::Path,
    ) -> Result<Self, FontError> {
        self.system.add_directory(directory)?;
        Ok(self)
    }

    /// Index `directory` as tier 2 — the metric-compatible faces the application bundles.
    ///
    /// # Errors
    ///
    /// [`FontError::UnreadableDirectory`] when the directory cannot be listed. A mis-pointed bundle
    /// path is reported rather than quietly producing an empty tier, because an empty *bundle* is a
    /// packaging mistake even though an empty *system* tier is not.
    pub fn with_bundled_font_directory(
        mut self,
        directory: &std::path::Path,
    ) -> Result<Self, FontError> {
        self.bundled.add_directory(directory)?;
        Ok(self)
    }

    /// Add one bundled face from bytes.
    #[must_use]
    pub fn with_bundled_face_bytes(mut self, data: Vec<u8>) -> Self {
        self.bundled.add_bytes(data);
        self
    }

    /// Add one embedded face from bytes — already decoded, if it was obfuscated.
    #[must_use]
    pub fn with_embedded_face_bytes(mut self, data: Vec<u8>) -> Self {
        self.embedded.add_bytes(data);
        self
    }

    /// Finish.
    #[must_use]
    pub fn build(self) -> FontResolver {
        FontResolver {
            embedded: self.embedded,
            system: self.system,
            bundled: self.bundled,
            interner: Interner::new(),
            manifest: SubstitutionManifest::new(),
        }
    }
}
