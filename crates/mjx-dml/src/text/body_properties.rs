//! `a:bodyPr` (`CT_TextBodyProperties`) — how a text body sits inside the shape that holds it.
//!
//! # Why this is here and was not before
//!
//! [`TextBody`](super::TextBody) has always preserved `a:bodyPr` verbatim, as an opaque
//! [`RawNode`](mjx_ooxml_core::RawNode), which is everything *fidelity* needs: an untouched body
//! re-emits byte for byte. It is not enough for *layout*. The four insets, the anchor, the wrap
//! flag, the column count and the autofit choice are the whole of the geometry a text body has, and
//! a box model that could not read them would place every paragraph at the shape's top-left corner
//! with a 0.1-inch guess for the margin.
//!
//! So this is the fifth typed piece of `a:txBody`, and it follows the same two-type shape as the
//! four that came before: [`TextBodyProperties`] is the fidelity wrapper — everything it does not
//! model survives verbatim — and [`TextBodyPropertiesSpec`] is the interner-free value description
//! the format-level API speaks, with the `with_`-prefixed setters and the
//! [`merge_under`](TextBodyPropertiesSpec::merge_under) that every other spec in this crate has.
//!
//! # The defaults are constants, not silent behaviour
//!
//! ECMA-376 Part 1 §21.1.2.1.1 gives every one of these attributes a default, and a reader that
//! applies them internally has made a decision a caller can no longer see. `None` here means *the
//! file does not state it*, and each default is a named constant on
//! [`TextBodyPropertiesSpec`] — so `left_inset().unwrap_or(TextBodyPropertiesSpec::DEFAULT_LEFT_INSET)`
//! is the whole of the resolution, written where the value is used and visible in a diff.
//!
//! # `ST_TextWrappingType` is hand-written, and why
//!
//! Every other enumeration here comes from the generated `mjx-ooxml-types` tables.
//! `ST_TextWrappingType` is not in them — the generator's curated type list does not name it — so
//! [`TextWrapping`] is written out here beside the attribute that uses it, exactly as
//! [`Bullet`](super::Bullet) and [`TextSpacing`](super::TextSpacing) are. Its two wire tokens are
//! preserved exactly, as the naming convention requires of a generated type and a hand-written one
//! alike.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::UnknownWireValue;

use crate::build::{dml_child, dml_element, dml_name, fidelity_element_impls, is_dml};
use crate::codec::{EmuCoordinate, Percentage, SixtyThousandthsOfADegree};
use crate::geometry::{Angle, Emu, Fraction};

// Both are already re-exported from the crate root out of the generated tables, so these are plain
// imports rather than a second export path for one type.
use mjx_ooxml_types::drawingml::{TextAnchoring, TextDirection};

/// `ST_TextWrappingType` — whether text wraps inside its shape or runs on past it.
///
/// Hand-written rather than generated: the generator's curated type list does not name
/// `ST_TextWrappingType`, and a two-valued enumeration written beside the attribute that reads it is
/// better than a dependency on regenerating the whole table. The wire tokens are exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextWrapping {
    /// Wire value `none` — lines are not broken at the shape's edge; a long line runs past it.
    None,
    /// Wire value `square` — lines are broken to the shape's own rectangle. The schema default.
    Square,
}

impl TextWrapping {
    /// Parses this value from its exact OOXML wire token.
    #[must_use]
    pub fn from_wire(token: &str) -> Option<Self> {
        match token {
            "none" => Some(Self::None),
            "square" => Some(Self::Square),
            _ => None,
        }
    }

    /// The exact OOXML wire token for this value.
    #[must_use]
    pub const fn to_wire(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Square => "square",
        }
    }
}

impl core::fmt::Display for TextWrapping {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.to_wire())
    }
}

impl core::str::FromStr for TextWrapping {
    type Err = UnknownWireValue;

    fn from_str(token: &str) -> Result<Self, Self::Err> {
        Self::from_wire(token).ok_or_else(|| UnknownWireValue::new(token))
    }
}

/// `EG_TextAutofit` — what a body does when its text does not fit the shape.
///
/// Three mutually exclusive children of `a:bodyPr`, so one enumeration rather than three booleans:
/// a body states at most one of them, and a body that states none inherits the choice.
///
/// # `Normal`'s two numbers are PowerPoint's own, not ours
///
/// `a:normAutofit@fontScale` and `@lnSpcReduction` are values **PowerPoint computed and wrote into
/// the file** the last time it laid the shape out. Reading them back and applying them reproduces
/// what the author saw. Recomputing them is a different act — it is running PowerPoint's own search
/// — and a consumer that does it should say which of the two it did. This type carries the stored
/// values and takes no position on the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAutofit {
    /// `a:noAutofit` — the text is left to overflow the shape.
    None,
    /// `a:normAutofit` — the text is scaled down until it fits, by the two factors stated.
    Normal {
        /// `@fontScale` — every font size is multiplied by this. `None` when unstated, which is
        /// `100 %`.
        font_scale: Option<Fraction>,
        /// `@lnSpcReduction` — line spacing is reduced by this proportion. `None` when unstated,
        /// which is `0 %`.
        line_space_reduction: Option<Fraction>,
    },
    /// `a:spAutoFit` — the *shape* is resized to fit the text, rather than the text to fit the shape.
    Shape,
}

impl TextAutofit {
    /// Whether `local` is one of the three `EG_TextAutofit` element names.
    #[must_use]
    pub fn is_choice_local(local: &str) -> bool {
        matches!(local, "noAutofit" | "normAutofit" | "spAutoFit")
    }

    /// The element name this choice serialises as.
    #[must_use]
    pub const fn wire_local(self) -> &'static str {
        match self {
            Self::None => "noAutofit",
            Self::Normal { .. } => "normAutofit",
            Self::Shape => "spAutoFit",
        }
    }
}

/// `a:normAutofit` — the attribute face of the scaling autofit.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "fontScale", codec = Percentage, accessor = font_scale))]
#[xml(attribute(local = "lnSpcReduction", codec = Percentage, accessor = line_space_reduction))]
struct NormalAutofitAttributes<A> {
    attributes: A,
}

/// `CT_TextBodyProperties` — the geometry of a text body: its insets, anchor, wrap, columns,
/// writing direction, rotation and autofit.
///
/// A fidelity wrapper. The attributes layout reads are typed; `@fromWordArt`, `@forceAA`,
/// `@horzOverflow`, `@vertOverflow`, the `a:prstTxWarp`, `a:scene3d`, `a:sp3d` and `a:extLst`
/// children and anything unknown are preserved verbatim, so a body round-trips whether or not this
/// type understands it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lIns", codec = EmuCoordinate, accessor = left_inset))]
#[xml(attribute(local = "tIns", codec = EmuCoordinate, accessor = top_inset))]
#[xml(attribute(local = "rIns", codec = EmuCoordinate, accessor = right_inset))]
#[xml(attribute(local = "bIns", codec = EmuCoordinate, accessor = bottom_inset))]
#[xml(attribute(local = "anchor", codec = Enumeration<TextAnchoring>, accessor = anchor))]
#[xml(attribute(local = "anchorCtr", codec = OnOff, accessor = is_anchor_centered))]
#[xml(attribute(local = "wrap", codec = Enumeration<TextWrapping>, accessor = wrap))]
// `ST_TextColumnCount` is bounded `1..=16` by the schema; the bound is documented rather than
// enforced, as everywhere else in this crate, because a file may carry an out-of-range value and
// reading one must not fail. The clamp belongs where the division by the count is.
#[xml(attribute(local = "numCol", codec = Number<u16>, accessor = columns))]
#[xml(attribute(local = "spcCol", codec = EmuCoordinate, accessor = column_space))]
#[xml(attribute(local = "rtlCol", codec = OnOff, accessor = has_right_to_left_columns))]
#[xml(attribute(local = "vert", codec = Enumeration<TextDirection>, accessor = vertical))]
#[xml(attribute(local = "rot", codec = SixtyThousandthsOfADegree, accessor = rotation))]
#[xml(attribute(local = "upright", codec = OnOff, accessor = is_upright))]
#[xml(attribute(local = "compatLnSpc", codec = OnOff, accessor = has_compatible_line_spacing))]
#[xml(attribute(local = "spcFirstLastPara", codec = OnOff, accessor = spaces_first_and_last_paragraph))]
pub struct TextBodyProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TextBodyProperties);

impl TextBodyProperties {
    /// The body's autofit choice (`EG_TextAutofit`), or `None` when it states none — in which case
    /// the choice is inherited.
    #[must_use]
    pub fn autofit(&self, interner: &Interner) -> Option<TextAutofit> {
        self.children.iter().find_map(|node| match node {
            RawNode::Element(child) if is_dml(&child.name, interner) => {
                match interner.resolve(child.name.local) {
                    "noAutofit" => Some(TextAutofit::None),
                    "spAutoFit" => Some(TextAutofit::Shape),
                    "normAutofit" => {
                        let read = NormalAutofitAttributes {
                            attributes: &child.attributes,
                        };
                        Some(TextAutofit::Normal {
                            font_scale: read.font_scale(interner).ok().flatten(),
                            line_space_reduction: read
                                .line_space_reduction(interner)
                                .ok()
                                .flatten(),
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }

    /// Replaces the body's autofit choice, or gives it one if it stated none.
    ///
    /// `CT_TextBodyProperties`'s sequence puts `EG_TextAutofit` immediately after the optional
    /// `a:prstTxWarp` and before `a:scene3d`, so a new element lands there rather than at the end;
    /// an existing one is replaced where it already sits, so nothing else about the body moves.
    /// Order is validity here, not style.
    pub fn set_autofit(&mut self, interner: &mut Interner, autofit: TextAutofit) {
        let element = build_autofit(interner, autofit);
        if let Some(position) = self.autofit_position(interner) {
            if let Some(slot) = self.children.get_mut(position) {
                *slot = RawNode::Element(element);
                return;
            }
        }
        let at = self.after_warp_position(interner);
        self.children.insert(at, RawNode::Element(element));
        self.empty = false;
    }

    /// Removes the body's autofit choice, returning whether it had one.
    pub fn remove_autofit(&mut self, interner: &Interner) -> bool {
        let before = self.children.len();
        self.children.retain(|node| match node {
            RawNode::Element(child) => {
                !(is_dml(&child.name, interner)
                    && TextAutofit::is_choice_local(interner.resolve(child.name.local)))
            }
            _ => true,
        });
        before != self.children.len()
    }

    fn autofit_position(&self, interner: &Interner) -> Option<usize> {
        self.children.iter().position(|node| match node {
            RawNode::Element(child) => {
                is_dml(&child.name, interner)
                    && TextAutofit::is_choice_local(interner.resolve(child.name.local))
            }
            _ => false,
        })
    }

    /// Where an autofit element belongs: after `a:prstTxWarp` when there is one, otherwise first.
    fn after_warp_position(&self, interner: &Interner) -> usize {
        match dml_child(&self.children, interner, "prstTxWarp") {
            None => 0,
            Some(_) => self
                .children
                .iter()
                .position(|node| match node {
                    RawNode::Element(child) => {
                        is_dml(&child.name, interner)
                            && interner.resolve(child.name.local) == "prstTxWarp"
                    }
                    _ => false,
                })
                .map_or(0, |position| position + 1),
        }
    }

    /// The interner-free description of these properties.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> TextBodyPropertiesSpec {
        // A spec is a value description: an attribute it cannot represent — absent, or malformed —
        // is simply not part of the description, which is what `None` says here.
        TextBodyPropertiesSpec {
            left_inset: self.left_inset(interner).ok().flatten(),
            top_inset: self.top_inset(interner).ok().flatten(),
            right_inset: self.right_inset(interner).ok().flatten(),
            bottom_inset: self.bottom_inset(interner).ok().flatten(),
            anchor: self.anchor(interner).ok().flatten(),
            anchor_centered: self.is_anchor_centered(interner).ok().flatten(),
            wrap: self.wrap(interner).ok().flatten(),
            columns: self.columns(interner).ok().flatten(),
            column_space: self.column_space(interner).ok().flatten(),
            right_to_left_columns: self.has_right_to_left_columns(interner).ok().flatten(),
            vertical: self.vertical(interner).ok().flatten(),
            rotation: self.rotation(interner).ok().flatten(),
            upright: self.is_upright(interner).ok().flatten(),
            compatible_line_spacing: self.has_compatible_line_spacing(interner).ok().flatten(),
            space_first_and_last_paragraph: self
                .spaces_first_and_last_paragraph(interner)
                .ok()
                .flatten(),
            autofit: self.autofit(interner),
        }
    }

    /// Merges `spec` onto these properties **in place**, writing only what the spec names and
    /// leaving everything else — the unmodeled attributes, `a:prstTxWarp`, `a:extLst` — where it was.
    ///
    /// A property the spec leaves unset is *not* cleared: unset means "don't touch".
    pub fn apply(&mut self, spec: &TextBodyPropertiesSpec, interner: &mut Interner) {
        if spec.left_inset.is_some() {
            self.set_left_inset(interner, spec.left_inset);
        }
        if spec.top_inset.is_some() {
            self.set_top_inset(interner, spec.top_inset);
        }
        if spec.right_inset.is_some() {
            self.set_right_inset(interner, spec.right_inset);
        }
        if spec.bottom_inset.is_some() {
            self.set_bottom_inset(interner, spec.bottom_inset);
        }
        if spec.anchor.is_some() {
            self.set_anchor(interner, spec.anchor);
        }
        if spec.anchor_centered.is_some() {
            self.set_is_anchor_centered(interner, spec.anchor_centered);
        }
        if spec.wrap.is_some() {
            self.set_wrap(interner, spec.wrap);
        }
        if spec.columns.is_some() {
            self.set_columns(interner, spec.columns);
        }
        if spec.column_space.is_some() {
            self.set_column_space(interner, spec.column_space);
        }
        if spec.right_to_left_columns.is_some() {
            self.set_has_right_to_left_columns(interner, spec.right_to_left_columns);
        }
        if spec.vertical.is_some() {
            self.set_vertical(interner, spec.vertical);
        }
        if spec.rotation.is_some() {
            self.set_rotation(interner, spec.rotation);
        }
        if spec.upright.is_some() {
            self.set_is_upright(interner, spec.upright);
        }
        if spec.compatible_line_spacing.is_some() {
            self.set_has_compatible_line_spacing(interner, spec.compatible_line_spacing);
        }
        if spec.space_first_and_last_paragraph.is_some() {
            self.set_spaces_first_and_last_paragraph(interner, spec.space_first_and_last_paragraph);
        }
        if let Some(autofit) = spec.autofit {
            self.set_autofit(interner, autofit);
        }
    }
}

/// Builds one `EG_TextAutofit` element.
fn build_autofit(interner: &mut Interner, autofit: TextAutofit) -> RawElement {
    let mut attributes = Vec::new();
    if let TextAutofit::Normal {
        font_scale,
        line_space_reduction,
    } = autofit
    {
        let mut normal = NormalAutofitAttributes {
            attributes: &mut attributes,
        };
        normal.set_font_scale(interner, font_scale);
        normal.set_line_space_reduction(interner, line_space_reduction);
    }
    dml_element(interner, autofit.wire_local(), attributes, Vec::new())
}

/// The interner-free description of a text body's geometry.
///
/// Every field is `Option`, and `None` means *the file does not state it* — never that it is zero.
/// The schema's own defaults are the `DEFAULT_` constants below, applied at the point of use so that
/// a reader can still tell an authored `0` inset from an unstated one.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextBodyPropertiesSpec {
    left_inset: Option<Emu>,
    top_inset: Option<Emu>,
    right_inset: Option<Emu>,
    bottom_inset: Option<Emu>,
    anchor: Option<TextAnchoring>,
    anchor_centered: Option<bool>,
    wrap: Option<TextWrapping>,
    columns: Option<u16>,
    column_space: Option<Emu>,
    right_to_left_columns: Option<bool>,
    vertical: Option<TextDirection>,
    rotation: Option<Angle>,
    upright: Option<bool>,
    compatible_line_spacing: Option<bool>,
    space_first_and_last_paragraph: Option<bool>,
    autofit: Option<TextAutofit>,
}

impl TextBodyPropertiesSpec {
    /// `@lIns`'s schema default — 0.1 inch (ECMA-376 Part 1 §21.1.2.1.1).
    pub const DEFAULT_LEFT_INSET: Emu = Emu::from_emu(91_440);
    /// `@tIns`'s schema default — 0.05 inch.
    pub const DEFAULT_TOP_INSET: Emu = Emu::from_emu(45_720);
    /// `@rIns`'s schema default — 0.1 inch.
    pub const DEFAULT_RIGHT_INSET: Emu = Emu::from_emu(91_440);
    /// `@bIns`'s schema default — 0.05 inch.
    pub const DEFAULT_BOTTOM_INSET: Emu = Emu::from_emu(45_720);
    /// `@anchor`'s schema default.
    pub const DEFAULT_ANCHOR: TextAnchoring = TextAnchoring::Top;
    /// `@wrap`'s schema default.
    pub const DEFAULT_WRAP: TextWrapping = TextWrapping::Square;
    /// `@numCol`'s schema default.
    pub const DEFAULT_COLUMNS: u16 = 1;
    /// `@spcCol`'s schema default.
    pub const DEFAULT_COLUMN_SPACE: Emu = Emu::from_emu(0);
    /// `@vert`'s schema default.
    pub const DEFAULT_VERTICAL: TextDirection = TextDirection::Horizontal;

    /// Properties that name nothing — everything inherits. The same as [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the inset between the shape's left edge and its text (`@lIns`).
    #[must_use]
    pub fn with_left_inset(mut self, inset: Emu) -> Self {
        self.left_inset = Some(inset);
        self
    }

    /// Sets the inset between the shape's top edge and its text (`@tIns`).
    #[must_use]
    pub fn with_top_inset(mut self, inset: Emu) -> Self {
        self.top_inset = Some(inset);
        self
    }

    /// Sets the inset between the shape's right edge and its text (`@rIns`).
    #[must_use]
    pub fn with_right_inset(mut self, inset: Emu) -> Self {
        self.right_inset = Some(inset);
        self
    }

    /// Sets the inset between the shape's bottom edge and its text (`@bIns`).
    #[must_use]
    pub fn with_bottom_inset(mut self, inset: Emu) -> Self {
        self.bottom_inset = Some(inset);
        self
    }

    /// Sets all four insets at once.
    #[must_use]
    pub fn with_insets(self, left: Emu, top: Emu, right: Emu, bottom: Emu) -> Self {
        self.with_left_inset(left)
            .with_top_inset(top)
            .with_right_inset(right)
            .with_bottom_inset(bottom)
    }

    /// Sets where the text sits vertically inside the shape (`@anchor`).
    #[must_use]
    pub fn with_anchor(mut self, anchor: TextAnchoring) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// Sets whether the text block is centred horizontally inside the shape (`@anchorCtr`).
    #[must_use]
    pub fn with_anchor_centered(mut self, centered: bool) -> Self {
        self.anchor_centered = Some(centered);
        self
    }

    /// Sets whether lines break at the shape's edge (`@wrap`).
    #[must_use]
    pub fn with_wrap(mut self, wrap: TextWrapping) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// Sets how many columns the text flows through (`@numCol`).
    #[must_use]
    pub fn with_columns(mut self, columns: u16) -> Self {
        self.columns = Some(columns);
        self
    }

    /// Sets the gap between two columns (`@spcCol`).
    #[must_use]
    pub fn with_column_space(mut self, space: Emu) -> Self {
        self.column_space = Some(space);
        self
    }

    /// Sets whether the columns are ordered right to left (`@rtlCol`).
    #[must_use]
    pub fn with_right_to_left_columns(mut self, right_to_left: bool) -> Self {
        self.right_to_left_columns = Some(right_to_left);
        self
    }

    /// Sets which way the text runs (`@vert`).
    #[must_use]
    pub fn with_vertical(mut self, vertical: TextDirection) -> Self {
        self.vertical = Some(vertical);
        self
    }

    /// Sets the rotation of the text within the shape (`@rot`), independent of the shape's own.
    #[must_use]
    pub fn with_rotation(mut self, rotation: Angle) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// Sets whether the text stays upright when the shape is rotated (`@upright`).
    #[must_use]
    pub fn with_upright(mut self, upright: bool) -> Self {
        self.upright = Some(upright);
        self
    }

    /// Sets whether line spacing is computed the way earlier releases did (`@compatLnSpc`).
    #[must_use]
    pub fn with_compatible_line_spacing(mut self, compatible: bool) -> Self {
        self.compatible_line_spacing = Some(compatible);
        self
    }

    /// Sets whether space-before and space-after apply to the first and last paragraph
    /// (`@spcFirstLastPara`).
    #[must_use]
    pub fn with_space_first_and_last_paragraph(mut self, spaced: bool) -> Self {
        self.space_first_and_last_paragraph = Some(spaced);
        self
    }

    /// Sets what the body does when its text does not fit (`EG_TextAutofit`).
    #[must_use]
    pub fn with_autofit(mut self, autofit: TextAutofit) -> Self {
        self.autofit = Some(autofit);
        self
    }

    /// The left inset (`@lIns`), or `None` if unstated.
    #[must_use]
    pub fn left_inset(&self) -> Option<Emu> {
        self.left_inset
    }

    /// The top inset (`@tIns`), or `None` if unstated.
    #[must_use]
    pub fn top_inset(&self) -> Option<Emu> {
        self.top_inset
    }

    /// The right inset (`@rIns`), or `None` if unstated.
    #[must_use]
    pub fn right_inset(&self) -> Option<Emu> {
        self.right_inset
    }

    /// The bottom inset (`@bIns`), or `None` if unstated.
    #[must_use]
    pub fn bottom_inset(&self) -> Option<Emu> {
        self.bottom_inset
    }

    /// Where the text sits vertically (`@anchor`), or `None` if unstated.
    #[must_use]
    pub fn anchor(&self) -> Option<TextAnchoring> {
        self.anchor
    }

    /// Whether the text block is centred horizontally (`@anchorCtr`), or `None` if unstated.
    #[must_use]
    pub fn is_anchor_centered(&self) -> Option<bool> {
        self.anchor_centered
    }

    /// Whether lines break at the shape's edge (`@wrap`), or `None` if unstated.
    #[must_use]
    pub fn wrap(&self) -> Option<TextWrapping> {
        self.wrap
    }

    /// How many columns (`@numCol`), or `None` if unstated.
    #[must_use]
    pub fn columns(&self) -> Option<u16> {
        self.columns
    }

    /// The gap between two columns (`@spcCol`), or `None` if unstated.
    #[must_use]
    pub fn column_space(&self) -> Option<Emu> {
        self.column_space
    }

    /// Whether the columns run right to left (`@rtlCol`), or `None` if unstated.
    #[must_use]
    pub fn has_right_to_left_columns(&self) -> Option<bool> {
        self.right_to_left_columns
    }

    /// Which way the text runs (`@vert`), or `None` if unstated.
    #[must_use]
    pub fn vertical(&self) -> Option<TextDirection> {
        self.vertical
    }

    /// The text's own rotation (`@rot`), or `None` if unstated.
    #[must_use]
    pub fn rotation(&self) -> Option<Angle> {
        self.rotation
    }

    /// Whether the text stays upright (`@upright`), or `None` if unstated.
    #[must_use]
    pub fn is_upright(&self) -> Option<bool> {
        self.upright
    }

    /// Whether line spacing uses the earlier algorithm (`@compatLnSpc`), or `None` if unstated.
    #[must_use]
    pub fn has_compatible_line_spacing(&self) -> Option<bool> {
        self.compatible_line_spacing
    }

    /// Whether the first and last paragraph take their spacing (`@spcFirstLastPara`), or `None` if
    /// unstated.
    #[must_use]
    pub fn spaces_first_and_last_paragraph(&self) -> Option<bool> {
        self.space_first_and_last_paragraph
    }

    /// What the body does when its text does not fit, or `None` if unstated.
    #[must_use]
    pub fn autofit(&self) -> Option<TextAutofit> {
        self.autofit
    }

    /// Fills every property this spec leaves unset from `lower`, and returns the result.
    ///
    /// The inheritance primitive every effective-property walk is made of: the higher tier wins
    /// field by field, and a tier that says nothing about a field contributes nothing.
    #[must_use]
    pub fn merge_under(mut self, lower: &Self) -> Self {
        self.left_inset = self.left_inset.or(lower.left_inset);
        self.top_inset = self.top_inset.or(lower.top_inset);
        self.right_inset = self.right_inset.or(lower.right_inset);
        self.bottom_inset = self.bottom_inset.or(lower.bottom_inset);
        self.anchor = self.anchor.or(lower.anchor);
        self.anchor_centered = self.anchor_centered.or(lower.anchor_centered);
        self.wrap = self.wrap.or(lower.wrap);
        self.columns = self.columns.or(lower.columns);
        self.column_space = self.column_space.or(lower.column_space);
        self.right_to_left_columns = self.right_to_left_columns.or(lower.right_to_left_columns);
        self.vertical = self.vertical.or(lower.vertical);
        self.rotation = self.rotation.or(lower.rotation);
        self.upright = self.upright.or(lower.upright);
        self.compatible_line_spacing = self
            .compatible_line_spacing
            .or(lower.compatible_line_spacing);
        self.space_first_and_last_paragraph = self
            .space_first_and_last_paragraph
            .or(lower.space_first_and_last_paragraph);
        self.autofit = self.autofit.or(lower.autofit);
        self
    }

    /// A fresh `a:bodyPr` carrying exactly what this spec names.
    #[must_use]
    pub fn to_properties(&self, interner: &mut Interner) -> TextBodyProperties {
        let mut properties = TextBodyProperties {
            name: dml_name(interner, "bodyPr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        properties.apply(self, interner);
        properties
    }
}

impl ToXml for TextBodyPropertiesSpec {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        self.to_properties(interner).to_xml(interner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_core::FromXml;

    /// Parses `xml` as one element, with the interner the parse built.
    fn element(xml: &str) -> (RawElement, Interner) {
        let document = mjx_xml::fidelity::parse(xml.as_bytes()).expect("the fixture parses");
        (document.root, document.interner)
    }

    #[test]
    fn every_modeled_attribute_reads() {
        let (root, interner) = element(
            r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                 lIns="1" tIns="2" rIns="3" bIns="4" anchor="ctr" anchorCtr="1" wrap="none"
                 numCol="3" spcCol="457200" rtlCol="1" vert="vert270" rot="5400000" upright="1"
                 compatLnSpc="1" spcFirstLastPara="1"/>"#,
        );
        let properties = TextBodyProperties::from_xml(&root, &interner).expect("it reads");
        let spec = properties.spec(&interner);
        assert_eq!(spec.left_inset(), Some(Emu::from_emu(1)));
        assert_eq!(spec.top_inset(), Some(Emu::from_emu(2)));
        assert_eq!(spec.right_inset(), Some(Emu::from_emu(3)));
        assert_eq!(spec.bottom_inset(), Some(Emu::from_emu(4)));
        assert_eq!(spec.anchor(), Some(TextAnchoring::Center));
        assert_eq!(spec.is_anchor_centered(), Some(true));
        assert_eq!(spec.wrap(), Some(TextWrapping::None));
        assert_eq!(spec.columns(), Some(3));
        assert_eq!(spec.column_space(), Some(Emu::from_emu(457_200)));
        assert_eq!(spec.has_right_to_left_columns(), Some(true));
        assert_eq!(spec.vertical(), Some(TextDirection::Vertical270));
        assert!((spec.rotation().expect("a rotation").degrees() - 90.0).abs() < 1e-9);
        assert_eq!(spec.is_upright(), Some(true));
        assert_eq!(spec.has_compatible_line_spacing(), Some(true));
        assert_eq!(spec.spaces_first_and_last_paragraph(), Some(true));
    }

    #[test]
    fn an_unstated_attribute_is_none_and_not_its_default() {
        let (root, interner) = element(
            r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#,
        );
        let spec = TextBodyProperties::from_xml(&root, &interner)
            .expect("it reads")
            .spec(&interner);
        assert_eq!(spec.left_inset(), None);
        assert_eq!(spec.anchor(), None);
        assert_eq!(spec.wrap(), None);
        assert_eq!(spec.autofit(), None);
    }

    #[test]
    fn an_authored_zero_inset_is_not_an_unstated_one() {
        let (root, interner) = element(
            r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" lIns="0"/>"#,
        );
        let spec = TextBodyProperties::from_xml(&root, &interner)
            .expect("it reads")
            .spec(&interner);
        assert_eq!(spec.left_inset(), Some(Emu::from_emu(0)));
        assert_ne!(
            spec.left_inset(),
            Some(TextBodyPropertiesSpec::DEFAULT_LEFT_INSET)
        );
    }

    #[test]
    fn each_autofit_choice_reads() {
        for (xml, expected) in [
            ("<a:noAutofit/>", TextAutofit::None),
            ("<a:spAutoFit/>", TextAutofit::Shape),
            (
                r#"<a:normAutofit fontScale="62500" lnSpcReduction="20000"/>"#,
                TextAutofit::Normal {
                    font_scale: Some(Fraction::from_ratio(0.625)),
                    line_space_reduction: Some(Fraction::from_ratio(0.2)),
                },
            ),
            (
                "<a:normAutofit/>",
                TextAutofit::Normal {
                    font_scale: None,
                    line_space_reduction: None,
                },
            ),
        ] {
            let (root, interner) = element(&format!(
                r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">{xml}</a:bodyPr>"#
            ));
            let properties = TextBodyProperties::from_xml(&root, &interner).expect("it reads");
            assert_eq!(properties.autofit(&interner), Some(expected), "for {xml}");
        }
    }

    #[test]
    fn an_autofit_lands_after_the_warp_and_replaces_an_existing_one() {
        let (root, mut interner) = element(
            r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:prstTxWarp prst="textNoShape"/><a:noAutofit/><a:extLst/></a:bodyPr>"#,
        );
        let mut properties = TextBodyProperties::from_xml(&root, &interner).expect("it reads");
        properties.set_autofit(&mut interner, TextAutofit::Shape);
        assert_eq!(properties.autofit(&interner), Some(TextAutofit::Shape));
        assert_eq!(properties.children.len(), 3, "nothing else moved or grew");

        assert!(properties.remove_autofit(&interner));
        assert_eq!(properties.autofit(&interner), None);
        properties.set_autofit(
            &mut interner,
            TextAutofit::Normal {
                font_scale: Some(Fraction::from_ratio(0.5)),
                line_space_reduction: None,
            },
        );
        // Inserted after `a:prstTxWarp`, which is index 0.
        let RawNode::Element(inserted) = &properties.children[1] else {
            panic!("the autofit is an element");
        };
        assert_eq!(interner.resolve(inserted.name.local), "normAutofit");
    }

    #[test]
    fn merging_under_lets_the_higher_tier_win_field_by_field() {
        let higher = TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Bottom);
        let lower = TextBodyPropertiesSpec::new()
            .with_anchor(TextAnchoring::Center)
            .with_columns(2);
        let merged = higher.merge_under(&lower);
        assert_eq!(merged.anchor(), Some(TextAnchoring::Bottom));
        assert_eq!(merged.columns(), Some(2));
    }

    #[test]
    fn a_spec_round_trips_through_an_element() {
        let mut interner = Interner::default();
        let spec = TextBodyPropertiesSpec::new()
            .with_insets(
                Emu::from_emu(10),
                Emu::from_emu(20),
                Emu::from_emu(30),
                Emu::from_emu(40),
            )
            .with_anchor(TextAnchoring::Bottom)
            .with_wrap(TextWrapping::None)
            .with_columns(4)
            .with_autofit(TextAutofit::Normal {
                font_scale: Some(Fraction::from_ratio(0.75)),
                line_space_reduction: Some(Fraction::from_ratio(0.1)),
            });
        let properties = spec.to_properties(&mut interner);
        assert_eq!(properties.spec(&interner), spec);
    }

    #[test]
    fn unmodeled_attributes_and_children_survive_an_edit() {
        let (root, mut interner) = element(
            r#"<a:bodyPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" fromWordArt="1" forceAA="1"><a:extLst/></a:bodyPr>"#,
        );
        let mut properties = TextBodyProperties::from_xml(&root, &interner).expect("it reads");
        properties.apply(
            &TextBodyPropertiesSpec::new().with_anchor(TextAnchoring::Center),
            &mut interner,
        );
        let rebuilt = properties.to_xml(&mut interner);
        let names: Vec<&str> = rebuilt
            .attributes
            .iter()
            .map(|attribute| interner.resolve(attribute.name.local))
            .collect();
        assert!(names.contains(&"fromWordArt"), "{names:?}");
        assert!(names.contains(&"forceAA"), "{names:?}");
        assert_eq!(rebuilt.children.len(), 1, "the extension list survived");
    }

    #[test]
    fn the_wrapping_tokens_are_exact() {
        assert_eq!(
            TextWrapping::from_wire("square"),
            Some(TextWrapping::Square)
        );
        assert_eq!(TextWrapping::from_wire("none"), Some(TextWrapping::None));
        assert_eq!(TextWrapping::from_wire("Square"), None);
        assert_eq!(TextWrapping::Square.to_wire(), "square");
        assert_eq!(TextWrapping::None.to_string(), "none");
        assert!("square".parse::<TextWrapping>().is_ok());
        assert!("wrapped".parse::<TextWrapping>().is_err());
    }
}
