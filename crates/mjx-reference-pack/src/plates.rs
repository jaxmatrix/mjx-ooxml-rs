//! What the 187 plates are, at their defaults and at their extremes.
//!
//! # 187, and why one of them is not a comparison
//!
//! `PresetShapeType` declares **187** shapes. `presetShapeDefinitions.xml` gives geometry for 186:
//! `mjx_geometry::PRESETS_WITHOUT_GEOMETRY` is exactly `[UpArrow]`. PowerPoint draws `upArrow`
//! perfectly well — the published file simply omits it — so that one plate is the only place in this
//! whole pack where **an Office export is the sole possible source of truth**. It cannot be checked
//! structurally, differentially, or against LibreOffice, because there is no table to check.
//!
//! It is therefore on the sheet, in its alphabetical place, and marked [`PlateKind::NoTable`]. Our
//! side of the comparison draws **nothing** in its window, and the report says so by name.
//!
//! # The extremes, and the three traps MJXOFF-205 measured
//!
//! *"At its default"* is the case most likely to be right by accident, so the second deck pushes
//! every handle to an end of its own domain. Three things go wrong if that is done naively, and all
//! three are handled here **from the data** rather than from a copied list of shape names:
//!
//! 1. **Twenty-two presets — every callout and every bent or curved connector — carry ECMA-376's
//!    own `i32::MAX` *"this handle has no stop"* sentinel.** Dragged there, the shape reaches
//!    3 435 973.8 device pixels outside its box: finite, correct, and off the slide. A deck that
//!    used the raw maximum would produce twenty-two plates neither PowerPoint nor we draw anywhere
//!    anybody can see. So a sentinel bound is clamped to [`UNBOUNDED_CLAMP`], and the plate records
//!    that it was ([`PlateKind::ClampedExtreme`]).
//! 2. **Three (shape, adjustment) pairs have no value at one of their own stops** —
//!    `circularArrow`/`adj5`, `leftCircularArrow`/`adj5`, `leftRightCircularArrow`/`adj5`. That is
//!    the shape's own formula leaving the reals, not a defect on either side: PowerPoint will draw
//!    *something* there and we correctly answer "there is nothing to draw". Such a plate is
//!    [`PlateKind::Singular`], and it is excluded by name.
//! 3. **Eleven presets leave their box somewhere inside a *bounded* domain**, by measured amounts
//!    from `teardrop`'s 80 px down to `curvedLeftArrow`'s 0.0117. They are correct and they are not
//!    excluded — but the comparison window is the cell rather than the box for exactly this reason,
//!    and ink outside the window is compared by nobody.
//!
//! None of those three lists is written down here. Each is *derived*: a sentinel is recognised by
//! its value, a singularity by [`mjx_geometry::GeometryError::has_no_geometry_to_draw`], and a spill
//! by where the resolved outline actually is. A copied list would be a second table to keep in step
//! with `mjx-geometry`'s, and the copy that is not exercised is the one that is wrong.

use mjx_geometry::{
    adjustment_domains, definition_of, preset_outline, seeded_shapes, AdjustmentOverride,
    GeometryError, PresetShapeType, Size, PRESETS_WITHOUT_GEOMETRY,
};

use crate::layout::{PlateGeometry, EMU_PER_POINT, PLATES_PER_SLIDE, SHAPE_BOX};

/// What a sentinel bound is replaced by, in native spec units.
///
/// **100 000 is full scale** — one whole box extent — and it is the largest displacement that is
/// still expressed in the *shape's own size* rather than in the slide's. A callout tail dragged
/// there points one box-width away from its box, which is visible on the plate, comparable, and
/// still an extreme by any reading. The raw sentinel is 2 147 483 647, twenty-one thousand times
/// further, and lands off every slide anyone will ever make.
pub const UNBOUNDED_CLAMP: i32 = 100_000;

/// How near `i32::MAX` a bound has to be before it is read as the schema's *"no stop"* sentinel
/// rather than as a real limit.
///
/// The file writes the sentinel exactly, so a strict equality would do — but a bound is *evaluated*
/// through the guide arithmetic, and a formula that multiplies the sentinel by one and divides by
/// one comes back a few units short. Half of `i32::MAX` is not a value any real domain reaches: the
/// largest genuine bound in the table is `teardrop`'s 200 000, four orders of magnitude below it.
pub const SENTINEL_THRESHOLD: f64 = (i32::MAX as f64) / 2.0;

/// What kind of plate this is — which decides whether it is a comparison at all.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlateKind {
    /// A shape with geometry, at the values the plate states. A real comparison.
    Comparable,
    /// **`upArrow`.** The published geometry file has no entry for it, so we have no table and no
    /// second opinion. Office's export is the only possible source of truth for this one plate.
    NoTable,
    /// The shape's own formulas have no finite value at these adjustments. PowerPoint draws
    /// something; we correctly draw nothing. Excluded, and named.
    Singular {
        /// The guide that lost its value.
        guide: String,
    },
    /// At least one handle's domain is the schema's unbounded sentinel and was clamped to
    /// [`UNBOUNDED_CLAMP`]. Still a real comparison — the clamp is stated so that a reader knows
    /// the plate is not at the number the file literally writes.
    ClampedExtreme {
        /// How many of the shape's handles were clamped.
        handles: usize,
    },
}

impl PlateKind {
    /// Whether our side of the comparison has an outline to draw here.
    #[must_use]
    pub fn draws(&self) -> bool {
        matches!(self, Self::Comparable | Self::ClampedExtreme { .. })
    }

    /// Why this plate is not evidence, or `None` when it is a real comparison.
    #[must_use]
    pub fn not_evidence(&self) -> Option<String> {
        match self {
            Self::Comparable | Self::ClampedExtreme { .. } => None,
            Self::NoTable => Some(
                "`presetShapeDefinitions.xml` publishes no geometry for this shape, so there is \
                 nothing on our side to compare; an Office export is the only source of truth for \
                 this plate"
                    .to_owned(),
            ),
            Self::Singular { guide } => Some(format!(
                "the shape's guide `{guide}` has no finite value at these adjustments, so there is \
                 no geometry to draw — the formula's, not either renderer's"
            )),
        }
    }
}

/// One plate of one deck.
#[derive(Clone, PartialEq, Debug)]
pub struct Plate {
    /// Its position in the deck, `0..187`.
    pub index: usize,
    /// The shape.
    pub preset: PresetShapeType,
    /// Its `prst` token, which is what the caption says and what a report keys on.
    pub token: &'static str,
    /// The adjustments this plate states, by wire name, in native spec units. Empty for the
    /// defaults deck and for a shape with no handles.
    pub adjustments: Vec<(&'static str, i32)>,
    /// What kind of plate it is.
    pub kind: PlateKind,
}

impl Plate {
    /// Which page of the deck it is on.
    #[must_use]
    pub fn page(&self) -> usize {
        self.index / PLATES_PER_SLIDE
    }

    /// Where it sits on that page.
    #[must_use]
    pub fn geometry(&self) -> PlateGeometry {
        PlateGeometry::at(self.index % PLATES_PER_SLIDE)
    }

    /// Its adjustments as the geometry provider's override type.
    #[must_use]
    pub fn overrides(&self) -> Vec<AdjustmentOverride> {
        self.adjustments
            .iter()
            .map(|(name, value)| AdjustmentOverride::new(*name, f64::from(*value)))
            .collect()
    }
}

/// Which of the two preset decks a plate belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PresetDeck {
    /// Every shape at the adjustments its own `a:avLst` seeds.
    AtTheirDefaults,
    /// Every handle at an end of its own domain, clamped where the domain has no end.
    AtTheirExtremes,
}

impl PresetDeck {
    /// Both, so a sweep cannot miss one.
    pub const ALL: [Self; 2] = [Self::AtTheirDefaults, Self::AtTheirExtremes];

    /// The file this deck is written to.
    #[must_use]
    pub fn file_name(self) -> &'static str {
        match self {
            Self::AtTheirDefaults => "01-presets-at-their-defaults.pptx",
            Self::AtTheirExtremes => "02-presets-at-their-extremes.pptx",
        }
    }

    /// The heading printed across the top of every page.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            Self::AtTheirDefaults => {
                "MJXOFF-207 · every preset shape at its default adjustments · page"
            }
            Self::AtTheirExtremes => {
                "MJXOFF-207 · every preset shape at an end of every handle · page"
            }
        }
    }
}

/// The size a plate's outline is resolved against — the shape's own box, in EMU.
#[must_use]
pub fn plate_extents() -> Size {
    Size::from_emu(SHAPE_BOX.0 * EMU_PER_POINT, SHAPE_BOX.1 * EMU_PER_POINT)
}

/// Every preset there is, in wire-token order.
///
/// Alphabetical rather than table order, so a person holding a printed sheet can find `teardrop`
/// without counting. `upArrow` is in it, in its own place, because the deck has 187 plates and only
/// 186 of them are ours to check.
#[must_use]
pub fn every_preset() -> Vec<PresetShapeType> {
    let mut presets: Vec<PresetShapeType> = seeded_shapes()
        .iter()
        .map(|definition| definition.preset)
        .chain(PRESETS_WITHOUT_GEOMETRY.iter().copied())
        .collect();
    presets.sort_by_key(|preset| preset.to_wire());
    presets.dedup();
    presets
}

/// The plates of one deck.
#[must_use]
pub fn plates(deck: PresetDeck) -> Vec<Plate> {
    every_preset()
        .into_iter()
        .enumerate()
        .map(|(index, preset)| plate(index, preset, deck))
        .collect()
}

/// One plate.
fn plate(index: usize, preset: PresetShapeType, deck: PresetDeck) -> Plate {
    let token = preset.to_wire();
    if definition_of(preset).is_none() {
        return Plate {
            index,
            preset,
            token,
            adjustments: Vec::new(),
            kind: PlateKind::NoTable,
        };
    }
    let (adjustments, clamped) = match deck {
        PresetDeck::AtTheirDefaults => (Vec::new(), 0),
        PresetDeck::AtTheirExtremes => extreme_adjustments(preset),
    };
    let kind = match resolves(preset, &adjustments) {
        Ok(()) if clamped > 0 => PlateKind::ClampedExtreme { handles: clamped },
        Ok(()) => PlateKind::Comparable,
        Err(guide) => PlateKind::Singular { guide },
    };
    Plate {
        index,
        preset,
        token,
        adjustments,
        kind,
    }
}

/// Whether the shape has an outline at these adjustments, or the guide that says it has none.
///
/// A failure that is *not* [`GeometryError::has_no_geometry_to_draw`] is a table defect and is
/// re-raised as a panic, because it means the pack is being generated from a broken table and a
/// plate quietly marked "singular" would hide it. Every such failure is a bug in `mjx-geometry`,
/// which its own suites gate on; none is an input.
fn resolves(preset: PresetShapeType, adjustments: &[(&'static str, i32)]) -> Result<(), String> {
    let overrides: Vec<AdjustmentOverride> = adjustments
        .iter()
        .map(|(name, value)| AdjustmentOverride::new(*name, f64::from(*value)))
        .collect();
    let window = PlateGeometry::at(0).shape_box().to_scene_rect();
    match preset_outline(preset, plate_extents(), &overrides, window) {
        Ok(_) => Ok(()),
        Err(GeometryError::SingularGeometry { guide, .. }) => Err(guide),
        Err(error) if error.has_no_geometry_to_draw() => Err(error.to_string()),
        Err(error) => panic!(
            "`{}` will not resolve at {adjustments:?}, and the failure is not a singularity: \
             {error}. That is a defect in the generated table, not a plate to mark excluded.",
            preset.to_wire()
        ),
    }
}

/// Every handle of `preset` at the end of its own domain furthest from its default, with how many
/// of them had to be clamped.
///
/// **All of them at once**, not one at a time: a shape with five handles has five defaults, and
/// moving one while four stay put is still four fifths of the case most likely to be right by
/// accident.
///
/// The clamp count is answered here rather than recovered from the values, and the difference is
/// not cosmetic: `cube`'s only handle has a real upper bound of **100 000**, which is exactly
/// [`UNBOUNDED_CLAMP`]. A count taken by looking for that number afterwards calls `cube` a clamped
/// shape and puts it in a census of callouts and connectors it has nothing to do with. MJXOFF-207
/// wrote that version first and `the_clamped_plates_are_the_callouts_and_the_connectors` caught it.
#[must_use]
pub fn extreme_adjustments(preset: PresetShapeType) -> (Vec<(&'static str, i32)>, usize) {
    let Ok(domains) = adjustment_domains(preset, plate_extents(), &[]) else {
        // `adjustment_domains` fails where a *domain* has no value at this size, which is a
        // different question from whether the shape draws. There is then no defensible extreme, so
        // the plate stays at its defaults and says nothing it cannot support.
        return (Vec::new(), 0);
    };
    let mut clamped = 0usize;
    let adjustments = domains
        .into_iter()
        .map(|domain| {
            let (minimum, minimum_clamped) = clamp_sentinel(domain.minimum);
            let (maximum, maximum_clamped) = clamp_sentinel(domain.maximum);
            let take_minimum = (minimum - domain.value).abs() >= (maximum - domain.value).abs();
            let (furthest, was_clamped) = if take_minimum {
                (minimum, minimum_clamped)
            } else {
                (maximum, maximum_clamped)
            };
            if was_clamped {
                clamped += 1;
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "clamp_sentinel bounds the value to +/- UNBOUNDED_CLAMP or to a real \
                          domain bound, and a real bound is an i32 in the file"
            )]
            (domain.spec.wire_name, furthest.round() as i32)
        })
        .collect();
    (adjustments, clamped)
}

/// A bound, with the schema's *"no stop"* sentinel replaced by a value that is on the slide, and
/// whether the replacement happened.
fn clamp_sentinel(bound: f64) -> (f64, bool) {
    if !bound.is_finite() {
        return (0.0, true);
    }
    if bound >= SENTINEL_THRESHOLD {
        return (f64::from(UNBOUNDED_CLAMP), true);
    }
    if bound <= -SENTINEL_THRESHOLD {
        return (f64::from(-UNBOUNDED_CLAMP), true);
    }
    (bound, false)
}
