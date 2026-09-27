//! The state matrix: the four axes every in-canvas element is audited along, and the one canonical
//! point a plate is taken at.
//!
//! # ⚠ Why the axes are a type rather than four booleans passed around
//!
//! `docs/client-platform/CANVAS_UI_INVENTORY.md` §3 asks for *"a state panel per scene — toggles for
//! every axis of the state matrix, so a state is reached by clicking rather than by contriving an
//! interaction"*. Sixty of those points exist per element (5 × 2 × 3 × 2) and 3 660 across the
//! inventory, which is far more than any plate set should hold — so the plates are taken at
//! [`State::CANONICAL`] and every other point is reachable **live**, in the harness, by clicking.
//!
//! # The instrument this module is shaped by
//!
//! *A parameter reached constantly but only ever at its no-op value is invisible to any reachability
//! check.* An axis that every scene ignores would be a state panel with four toggles that do
//! nothing, and *"61 elements exercised"* would be a sentence about registration rather than about
//! drawing. So each axis is a value a scene reads, each [`crate::inventory::Entry`] **declares**
//! which axes move its pixels, and `tests/the_axes_are_not_identities.rs` measures the declaration
//! against the renders in both directions: an entry that claims to respond to hover and does not
//! fails, and so does one that responds without saying so.

use std::fmt;

pub use mjx_tokens::ColorScheme;

/// One axis of the state matrix — what a toggle in the harness's state panel changes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Axis {
    /// Default / hover / active / focused / disabled.
    Interaction,
    /// Light or dark.
    Scheme,
    /// 1×, 2× or 3× device pixels to the point.
    Density,
    /// A mouse pointer or a finger.
    Input,
}

impl Axis {
    /// Every axis, so a sweep cannot miss one.
    pub const ALL: [Self; 4] = [Self::Interaction, Self::Scheme, Self::Density, Self::Input];

    /// How the axis is named in the harness and in a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Interaction => "interaction",
            Self::Scheme => "scheme",
            Self::Density => "density",
            Self::Input => "input",
        }
    }
}

impl fmt::Display for Axis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// What the pointer or the selection is doing to the element.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum Interaction {
    /// At rest.
    #[default]
    Default,
    /// The pointer is over the element's grab region.
    Hover,
    /// The element is being dragged, pressed or resized.
    Active,
    /// The element holds keyboard focus.
    Focused,
    /// The element cannot be operated — a locked object, a protected sheet, a field the reader may
    /// not edit.
    Disabled,
}

impl Interaction {
    /// Every value, in panel order.
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Hover,
        Self::Active,
        Self::Focused,
        Self::Disabled,
    ];

    /// The value's stable, URL- and filename-safe spelling.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover => "hover",
            Self::Active => "active",
            Self::Focused => "focused",
            Self::Disabled => "disabled",
        }
    }

    /// The value that spelling names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.slug() == text)
    }

    /// Whether the element is being operated right now — hover and active both, because an
    /// affordance that grows under the pointer grows further while it is dragged.
    #[must_use]
    pub const fn is_engaged(self) -> bool {
        matches!(self, Self::Hover | Self::Active)
    }
}

/// How many device pixels there are to a typographic point, as a reader's display and zoom give it.
///
/// Three buckets rather than a float, because the question the audit asks is *"does this hairline
/// survive?"* and the answer is different at each of the three densities a real display has. A
/// continuous zoom is loop 2's problem.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum Density {
    /// An ordinary display.
    #[default]
    One,
    /// A Retina or 200 % display.
    Two,
    /// A 300 % display — the one where a half-pixel hairline disappears or doubles.
    Three,
}

impl Density {
    /// Every value, in panel order.
    pub const ALL: [Self; 3] = [Self::One, Self::Two, Self::Three];

    /// The value's stable spelling.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::One => "1x",
            Self::Two => "2x",
            Self::Three => "3x",
        }
    }

    /// The value that spelling names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.slug() == text)
    }

    /// The multiplier over the unzoomed 96/72.
    #[must_use]
    pub const fn factor(self) -> f32 {
        match self {
            Self::One => 1.0,
            Self::Two => 2.0,
            Self::Three => 3.0,
        }
    }

    /// The scale a display list is built at.
    #[must_use]
    pub fn device_scale(self) -> mjx_text::DeviceScale {
        mjx_text::DeviceScale::UNZOOMED.zoomed_by(self.factor())
    }
}

/// What is pointing at the canvas.
///
/// **The axis eleven inventory entries are entirely about.** A grab region sized for a mouse is a
/// grab region a thumb misses, so the two are different numbers rather than the same number with a
/// tolerance, and the hit-test visualiser draws whichever is in force.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum Input {
    /// A mouse or a trackpad: a precise position, and hover exists.
    #[default]
    Pointer,
    /// A finger: an imprecise position under an opaque fingertip, and no hover at all.
    Touch,
}

impl Input {
    /// Every value, in panel order.
    pub const ALL: [Self; 2] = [Self::Pointer, Self::Touch];

    /// The value's stable spelling.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Touch => "touch",
        }
    }

    /// The value that spelling names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.slug() == text)
    }

    /// How wide a drawn affordance is, in points.
    ///
    /// Seven points for a pointer — what a desktop resize handle has always been — and eleven for a
    /// finger. The numbers differ because the *drawn* thing differs, not only the region around it:
    /// a handle a thumb can hit but cannot see is as useless as one it can see and cannot hit.
    #[must_use]
    pub const fn affordance_size(self) -> f64 {
        match self {
            Self::Pointer => 7.0,
            Self::Touch => 11.0,
        }
    }

    /// How far past a drawn affordance its grab region reaches, in points.
    ///
    /// This is the number the hit-test visualiser exists to make judgeable. Apple asks for 44 CSS
    /// pixels of touch target and Microsoft for 40; a 24-point square around an 11-point handle is
    /// 32 device pixels at 1× and the honest answer is that **it is for the user to say whether it
    /// is enough**, on a real finger, which is why the harness draws it rather than asserting it.
    #[must_use]
    pub const fn grab_padding(self) -> f64 {
        match self {
            Self::Pointer => 3.0,
            Self::Touch => 8.0,
        }
    }
}

/// One point of the state matrix.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct State {
    /// What the pointer is doing.
    pub interaction: Interaction,
    /// Which colour scheme.
    pub scheme: ColorScheme,
    /// How dense the display is.
    pub density: Density,
    /// What is pointing at it.
    pub input: Input,
}

impl Default for State {
    /// [`State::CANONICAL`]. `mjx_tokens::ColorScheme` derives neither `Default` nor `Ord`, so this
    /// is written out rather than derived — and writing it out is what makes the default *the point
    /// a plate is taken at* rather than whatever the first variant of each axis happens to be.
    fn default() -> Self {
        Self::CANONICAL
    }
}

impl State {
    /// **The point every plate is taken at**: at rest, light, 1×, pointer.
    ///
    /// Named rather than spelled out at each call site, because a baseline taken at a different
    /// point than it is checked at is green for the wrong reason and the two spellings would drift.
    pub const CANONICAL: Self = Self {
        interaction: Interaction::Default,
        scheme: ColorScheme::Light,
        density: Density::One,
        input: Input::Pointer,
    };

    /// The same state with one axis moved off [`State::CANONICAL`], or `None` when the axis has no
    /// other value to move to — which never happens, since every axis has at least two.
    ///
    /// This is what `tests/the_axes_are_not_identities.rs` renders: the canonical image, and one
    /// image per axis with only that axis changed, so a difference is attributable to one toggle.
    #[must_use]
    pub fn moved(self, axis: Axis) -> Self {
        let mut moved = self;
        match axis {
            Axis::Interaction => moved.interaction = Interaction::Hover,
            Axis::Scheme => moved.scheme = ColorScheme::Dark,
            Axis::Density => moved.density = Density::Two,
            Axis::Input => moved.input = Input::Touch,
        }
        moved
    }

    /// Every value of `axis`, with the other three left where they are.
    #[must_use]
    pub fn along(self, axis: Axis) -> Vec<Self> {
        match axis {
            Axis::Interaction => Interaction::ALL
                .into_iter()
                .map(|value| Self {
                    interaction: value,
                    ..self
                })
                .collect(),
            Axis::Scheme => [ColorScheme::Light, ColorScheme::Dark]
                .into_iter()
                .map(|value| Self {
                    scheme: value,
                    ..self
                })
                .collect(),
            Axis::Density => Density::ALL
                .into_iter()
                .map(|value| Self {
                    density: value,
                    ..self
                })
                .collect(),
            Axis::Input => Input::ALL
                .into_iter()
                .map(|value| Self {
                    input: value,
                    ..self
                })
                .collect(),
        }
    }

    /// How many points the whole matrix has — 5 × 2 × 3 × 2.
    pub const POINTS: usize = Interaction::ALL.len() * 2 * Density::ALL.len() * Input::ALL.len();

    /// The state's stable spelling, as the harness's URLs and the plate names use it.
    #[must_use]
    pub fn slug(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.interaction.slug(),
            scheme_slug(self.scheme),
            self.density.slug(),
            self.input.slug()
        )
    }
}

/// A colour scheme's stable spelling. `mjx_tokens::ColorScheme` has no `slug` of its own and this
/// crate may not give it one, so the mapping is stated once here rather than at each call site.
#[must_use]
pub fn scheme_slug(scheme: ColorScheme) -> &'static str {
    match scheme {
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    }
}

/// The colour scheme that spelling names.
#[must_use]
pub fn parse_scheme(text: &str) -> Option<ColorScheme> {
    match text {
        "light" => Some(ColorScheme::Light),
        "dark" => Some(ColorScheme::Dark),
        _ => None,
    }
}
