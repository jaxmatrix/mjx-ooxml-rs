//! DrawingML paragraph properties: `CT_TextParagraphProperties` — how a paragraph is laid out.
//!
//! [`ParagraphProperties`] is a **fidelity wrapper**, so one type serves every name this complex type
//! appears under: `a:pPr` on a paragraph, `a:defPPr` and `a:lvl1pPr`…`a:lvl9pPr` inside a list style.
//! [`ParagraphPropertiesSpec`] is the interner-free builder the format-level API speaks, following the
//! same conventions as the character properties it sits beside — `with_`-prefixed setters, and
//! [`apply`](ParagraphProperties::apply) merging onto an existing element rather than rebuilding it.
//!
//! # Units
//!
//! Margins, indents and tab stops are **points** on the surface and EMU on the wire, so one
//! typographic unit runs through the whole text API: a 36 pt indent beside an 18 pt font reads as a
//! relationship. [`Emu`] remains available for callers who want the file's own unit.
//!
//! ```
//! use mjx_dml::{IndentLevel, ParagraphPropertiesSpec, TextAlignment};
//!
//! let quotation = ParagraphPropertiesSpec::new()
//!     .with_level(IndentLevel::of(1))
//!     .with_alignment(TextAlignment::Justified)
//!     .with_left_margin_points(36.0);
//!
//! assert_eq!(quotation.left_margin_points(), Some(36.0));
//! ```

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::support::OnOff;

use mjx_ooxml_types::child_order::TEXT_PARAGRAPH_PROPERTIES;

use crate::build::{dml_child, dml_element, dml_name, fidelity_element_impls, is_dml};
use crate::codec::{EmuCoordinate, Percentage, TextIndentLevel, TextPointSize};
use crate::geometry::{Emu, Fraction, IndentLevel, TextPoint};
use crate::text::bullet::{
    build_bullet, build_bullet_color, build_bullet_size, build_bullet_typeface,
    is_bullet_color_local, is_bullet_local, is_bullet_size_local, is_bullet_typeface_local,
    read_bullet, read_bullet_color, read_bullet_size, read_bullet_typeface, Bullet,
    BulletCharacter, BulletColor, BulletSize, BulletTypeface,
};
use crate::text::character::{CharacterProperties, CharacterPropertiesSpec};

pub use mjx_ooxml_types::drawingml::{FontAlignment, TabAlignment, TextAlignment};

/// `a:tab` (`CT_TextTabStop`) — the attribute face of one tab stop.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "pos", codec = EmuCoordinate, accessor = position, required))]
#[xml(attribute(local = "algn", codec = Enumeration<TabAlignment>, accessor = alignment))]
struct TabStopAttributes<A> {
    attributes: A,
}

/// `a:spcPct` (`CT_TextSpacingPercent`) — the attribute face of a percentage spacing.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", codec = Percentage, accessor = value, required))]
struct SpacingPercentAttributes<A> {
    attributes: A,
}

/// `a:spcPts` (`CT_TextSpacingPoint`) — the attribute face of a point-valued spacing.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", codec = TextPointSize, accessor = value, required))]
struct SpacingPointsAttributes<A> {
    attributes: A,
}

/// How much room a paragraph leaves — before it, after it, or between its lines (`CT_TextSpacing`).
///
/// The two arms are genuinely different measurements, not two spellings of one: a percentage scales
/// with the text, points do not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextSpacing {
    /// `a:spcPct` — a proportion of the line height. `1.5` is line-and-a-half spacing.
    Percentage(Fraction),
    /// `a:spcPts` — a fixed distance.
    Points(TextPoint),
}

impl TextSpacing {
    /// Spacing as a proportion of the line height (`1.5` = 150%).
    #[must_use]
    pub fn proportion(proportion: f64) -> Self {
        Self::Percentage(Fraction::from_ratio(proportion))
    }

    /// Spacing as a fixed distance in points.
    #[must_use]
    pub fn points(points: f64) -> Self {
        Self::Points(TextPoint::from_points(points))
    }
}

/// One tab stop (`CT_TextTabStop`) — where a tab character advances to, and how text sits there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabStop {
    /// The distance from the text-box edge (`@pos`).
    pub position: Emu,
    /// How text aligns at this stop (`@algn`), or `None` if unset.
    pub alignment: Option<TabAlignment>,
}

impl TabStop {
    /// A tab stop at `points` from the edge, with the given alignment.
    #[must_use]
    pub fn at_points(points: f64, alignment: TabAlignment) -> Self {
        Self {
            position: Emu::from_points(points),
            alignment: Some(alignment),
        }
    }

    /// The stop's distance from the edge, in points.
    #[must_use]
    pub fn position_points(self) -> f64 {
        self.position.points()
    }
}

// ---------------------------------------------------------------------------------------------
// ParagraphProperties — the fidelity wrapper
// ---------------------------------------------------------------------------------------------

/// `CT_TextParagraphProperties` — a paragraph's layout: its indent level, alignment, margins,
/// spacing, bullet, tab stops, and the character properties its runs default to.
///
/// A fidelity wrapper: the modeled properties are typed, while the line-breaking attributes
/// (`eaLnBrk`, `latinLnBrk`, `hangingPunct`), `extLst` and anything unknown are preserved verbatim so
/// a paragraph round-trips byte-for-byte. The element name is preserved too, so
/// the same type reads and writes `a:pPr`, `a:defPPr` and each `a:lvlNpPr`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lvl", codec = TextIndentLevel, accessor = level))]
#[xml(attribute(local = "algn", codec = Enumeration<TextAlignment>, accessor = alignment))]
#[xml(attribute(local = "marL", codec = EmuCoordinate, accessor = left_margin))]
#[xml(attribute(local = "marR", codec = EmuCoordinate, accessor = right_margin))]
#[xml(attribute(local = "indent", codec = EmuCoordinate, accessor = indent))]
#[xml(attribute(local = "defTabSz", codec = EmuCoordinate, accessor = default_tab_size))]
#[xml(attribute(local = "rtl", codec = OnOff, accessor = is_right_to_left))]
#[xml(attribute(local = "fontAlgn", codec = Enumeration<FontAlignment>, accessor = font_alignment))]
pub struct ParagraphProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl ParagraphProperties {
    /// The spacing between lines within the paragraph (`a:lnSpc`), or `None` if unset.
    #[must_use]
    pub fn line_spacing(&self, interner: &Interner) -> Option<TextSpacing> {
        self.spacing_child(interner, "lnSpc")
    }

    /// The space before the paragraph (`a:spcBef`), or `None` if unset.
    #[must_use]
    pub fn space_before(&self, interner: &Interner) -> Option<TextSpacing> {
        self.spacing_child(interner, "spcBef")
    }

    /// The space after the paragraph (`a:spcAft`), or `None` if unset.
    #[must_use]
    pub fn space_after(&self, interner: &Interner) -> Option<TextSpacing> {
        self.spacing_child(interner, "spcAft")
    }

    /// The paragraph's tab stops (`a:tabLst`), in document order — empty when it declares none.
    #[must_use]
    pub fn tab_stops(&self, interner: &Interner) -> Vec<TabStop> {
        let Some(list) = dml_child(&self.children, interner, "tabLst") else {
            return Vec::new();
        };
        list.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if is_dml(&child.name, interner)
                        && interner.resolve(child.name.local) == "tab" =>
                {
                    let stop = TabStopAttributes {
                        attributes: &child.attributes,
                    };
                    Some(TabStop {
                        // `@pos` is schema-required; a stop that does not state one, or states a
                        // value this model cannot read, is a stop at the left margin.
                        position: stop.position(interner).unwrap_or(Emu::from_emu(0)),
                        alignment: stop.alignment(interner).ok().flatten(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// What marks this paragraph (`EG_TextBullet`), or `None` if it declares no bullet group — in
    /// which case the bullet is inherited. [`Bullet::None`] is the *decision* to have none.
    #[must_use]
    pub fn bullet(&self, interner: &Interner) -> Option<Bullet> {
        read_bullet(&self.children, interner)
    }

    /// The bullet's colour (`EG_TextBulletColor`), or `None` if inherited.
    #[must_use]
    pub fn bullet_color(&self, interner: &Interner) -> Option<BulletColor> {
        read_bullet_color(&self.children, interner)
    }

    /// The bullet's size (`EG_TextBulletSize`), or `None` if inherited.
    #[must_use]
    pub fn bullet_size(&self, interner: &Interner) -> Option<BulletSize> {
        read_bullet_size(&self.children, interner)
    }

    /// The bullet's typeface (`EG_TextBulletTypeface`), or `None` if inherited.
    #[must_use]
    pub fn bullet_typeface(&self, interner: &Interner) -> Option<BulletTypeface> {
        read_bullet_typeface(&self.children, interner)
    }

    /// The character properties this paragraph's runs default to (`a:defRPr`), or `None` if it
    /// declares none. This is the tier a run's own `a:rPr` overrides.
    #[must_use]
    pub fn default_run_properties(&self, interner: &Interner) -> Option<CharacterProperties> {
        dml_child(&self.children, interner, "defRPr")
            .and_then(|el| CharacterProperties::from_xml(el, interner).ok())
    }

    /// The interner-free description of these properties.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> ParagraphPropertiesSpec {
        // A spec is a value description: an attribute it cannot represent — absent, or malformed —
        // is simply not part of the description, which is what `None` says here.
        ParagraphPropertiesSpec {
            level: self.level(interner).ok().flatten(),
            alignment: self.alignment(interner).ok().flatten(),
            left_margin: self.left_margin(interner).ok().flatten(),
            right_margin: self.right_margin(interner).ok().flatten(),
            indent: self.indent(interner).ok().flatten(),
            default_tab_size: self.default_tab_size(interner).ok().flatten(),
            right_to_left: self.is_right_to_left(interner).ok().flatten(),
            font_alignment: self.font_alignment(interner).ok().flatten(),
            line_spacing: self.line_spacing(interner),
            space_before: self.space_before(interner),
            space_after: self.space_after(interner),
            bullet: self.bullet(interner),
            bullet_color: self.bullet_color(interner),
            bullet_size: self.bullet_size(interner),
            bullet_typeface: self.bullet_typeface(interner),
            tab_stops: self.tab_stops(interner),
            default_run_properties: self
                .default_run_properties(interner)
                .map(|properties| properties.spec(interner)),
        }
    }

    /// Merges `spec` onto these properties **in place**, writing only what the spec names and leaving
    /// everything else — the line-breaking attributes, the bullet, unmodeled children — where it was.
    ///
    /// A property the spec leaves unset is *not* cleared: unset means "don't touch". Build a fresh
    /// element with [`ParagraphPropertiesSpec::to_properties`] to drop what an old one carried.
    pub fn apply(&mut self, spec: &ParagraphPropertiesSpec, interner: &mut Interner) {
        // Each attribute is written only when the spec names it: an unset field means "don't
        // touch", which is not what the setters' `None` means (that removes the attribute).
        if spec.level.is_some() {
            self.set_level(interner, spec.level);
        }
        if spec.alignment.is_some() {
            self.set_alignment(interner, spec.alignment);
        }
        if spec.left_margin.is_some() {
            self.set_left_margin(interner, spec.left_margin);
        }
        if spec.right_margin.is_some() {
            self.set_right_margin(interner, spec.right_margin);
        }
        if spec.indent.is_some() {
            self.set_indent(interner, spec.indent);
        }
        if spec.default_tab_size.is_some() {
            self.set_default_tab_size(interner, spec.default_tab_size);
        }
        if spec.right_to_left.is_some() {
            self.set_is_right_to_left(interner, spec.right_to_left);
        }
        if spec.font_alignment.is_some() {
            self.set_font_alignment(interner, spec.font_alignment);
        }

        for (local, spacing) in [
            ("lnSpc", spec.line_spacing),
            ("spcBef", spec.space_before),
            ("spcAft", spec.space_after),
        ] {
            if let Some(spacing) = spacing {
                let element = build_spacing(interner, local, spacing);
                self.replace_child(interner, element, |candidate| candidate == local);
            }
        }
        // The four bullet groups are independent: each replaces its own group and leaves the others
        // exactly as they were, because a level may set one and inherit the rest.
        if let Some(color) = &spec.bullet_color {
            if let Some(element) = build_bullet_color(interner, color) {
                self.replace_child(interner, element, is_bullet_color_local);
            }
        }
        if let Some(size) = spec.bullet_size {
            let element = build_bullet_size(interner, size);
            self.replace_child(interner, element, is_bullet_size_local);
        }
        if let Some(typeface) = &spec.bullet_typeface {
            let element = build_bullet_typeface(interner, typeface);
            self.replace_child(interner, element, is_bullet_typeface_local);
        }
        if let Some(bullet) = &spec.bullet {
            let element = build_bullet(interner, bullet);
            self.replace_child(interner, element, is_bullet_local);
        }
        if !spec.tab_stops.is_empty() {
            let element = build_tab_stops(interner, &spec.tab_stops);
            self.replace_child(interner, element, |local| local == "tabLst");
        }
        if let Some(default_run) = &spec.default_run_properties {
            let element = default_run
                .to_properties(interner, "defRPr")
                .to_xml(interner);
            self.replace_child(interner, element, |local| local == "defRPr");
        }
        self.empty = self.empty && self.children.is_empty();
    }

    /// One of the three `CT_TextSpacing` children, read as a [`TextSpacing`].
    fn spacing_child(&self, interner: &Interner, local: &str) -> Option<TextSpacing> {
        let element = dml_child(&self.children, interner, local)?;
        element.children.iter().find_map(|node| match node {
            RawNode::Element(child) if is_dml(&child.name, interner) => {
                match interner.resolve(child.name.local) {
                    "spcPct" => SpacingPercentAttributes {
                        attributes: &child.attributes,
                    }
                    .value(interner)
                    .ok()
                    .map(TextSpacing::Percentage),
                    "spcPts" => SpacingPointsAttributes {
                        attributes: &child.attributes,
                    }
                    .value(interner)
                    .ok()
                    .map(TextSpacing::Points),
                    _ => None,
                }
            }
            _ => None,
        })
    }

    /// Replaces the first child element whose local name satisfies `matches` with `element`, keeping
    /// its position; inserts it in `CT_TextParagraphProperties` order when there is none.
    fn replace_child(
        &mut self,
        interner: &Interner,
        element: RawElement,
        matches: impl Fn(&str) -> bool,
    ) {
        TEXT_PARAGRAPH_PROPERTIES.replace_or_insert(&mut self.children, interner, element, matches);
        self.empty = false;
    }
}

fidelity_element_impls!(ParagraphProperties);

/// Builds one of the `CT_TextSpacing` children (`a:lnSpc`, `a:spcBef`, `a:spcAft`).
fn build_spacing(interner: &mut Interner, local: &str, spacing: TextSpacing) -> RawElement {
    let inner = match spacing {
        TextSpacing::Percentage(fraction) => {
            let mut attributes = SpacingPercentAttributes {
                attributes: Vec::new(),
            };
            attributes.set_value(interner, fraction);
            dml_element(interner, "spcPct", attributes.attributes, Vec::new())
        }
        TextSpacing::Points(points) => {
            let mut attributes = SpacingPointsAttributes {
                attributes: Vec::new(),
            };
            attributes.set_value(interner, points);
            dml_element(interner, "spcPts", attributes.attributes, Vec::new())
        }
    };
    dml_element(interner, local, Vec::new(), vec![RawNode::Element(inner)])
}

/// Builds the `a:tabLst` for a set of tab stops.
fn build_tab_stops(interner: &mut Interner, stops: &[TabStop]) -> RawElement {
    let children = stops
        .iter()
        .map(|stop| {
            let mut attributes = TabStopAttributes {
                attributes: Vec::new(),
            };
            attributes.set_position(interner, stop.position);
            attributes.set_alignment(interner, stop.alignment);
            RawNode::Element(dml_element(
                interner,
                "tab",
                attributes.attributes,
                Vec::new(),
            ))
        })
        .collect();
    dml_element(interner, "tabLst", Vec::new(), children)
}

// ---------------------------------------------------------------------------------------------
// ParagraphPropertiesSpec — the interner-free builder
// ---------------------------------------------------------------------------------------------

/// An interner-free description of a paragraph's layout — the value the format-level API reads and
/// writes.
///
/// As with character properties, naming a property sets it and leaving it unnamed means **inherit**.
/// Margins and indents are stated in points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParagraphPropertiesSpec {
    level: Option<IndentLevel>,
    alignment: Option<TextAlignment>,
    left_margin: Option<Emu>,
    right_margin: Option<Emu>,
    indent: Option<Emu>,
    default_tab_size: Option<Emu>,
    right_to_left: Option<bool>,
    font_alignment: Option<FontAlignment>,
    line_spacing: Option<TextSpacing>,
    space_before: Option<TextSpacing>,
    space_after: Option<TextSpacing>,
    bullet: Option<Bullet>,
    bullet_color: Option<BulletColor>,
    bullet_size: Option<BulletSize>,
    bullet_typeface: Option<BulletTypeface>,
    tab_stops: Vec<TabStop>,
    default_run_properties: Option<CharacterPropertiesSpec>,
}

impl ParagraphPropertiesSpec {
    /// Properties that name nothing — everything inherits. The same as [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets how deeply the paragraph is nested — the axis its bullet, size and indent are inherited
    /// along.
    #[must_use]
    pub fn with_level(mut self, level: IndentLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Sets the horizontal alignment.
    #[must_use]
    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets the left margin, in points — the inset of the paragraph as a whole.
    #[must_use]
    pub fn with_left_margin_points(mut self, points: f64) -> Self {
        self.left_margin = Some(Emu::from_points(points));
        self
    }

    /// Sets the right margin, in points.
    #[must_use]
    pub fn with_right_margin_points(mut self, points: f64) -> Self {
        self.right_margin = Some(Emu::from_points(points));
        self
    }

    /// Sets the first-line indent, in points, relative to the left margin. **Negative** hangs the
    /// first line out to the left — how a bullet sits in the margin of its text.
    #[must_use]
    pub fn with_indent_points(mut self, points: f64) -> Self {
        self.indent = Some(Emu::from_points(points));
        self
    }

    /// Sets the default gap between tab stops, in points.
    #[must_use]
    pub fn with_default_tab_size_points(mut self, points: f64) -> Self {
        self.default_tab_size = Some(Emu::from_points(points));
        self
    }

    /// Sets the reading direction.
    #[must_use]
    pub fn with_right_to_left(mut self, right_to_left: bool) -> Self {
        self.right_to_left = Some(right_to_left);
        self
    }

    /// Sets where letters sit between the baselines.
    #[must_use]
    pub fn with_font_alignment(mut self, font_alignment: FontAlignment) -> Self {
        self.font_alignment = Some(font_alignment);
        self
    }

    /// Sets the spacing between lines within the paragraph.
    #[must_use]
    pub fn with_line_spacing(mut self, spacing: TextSpacing) -> Self {
        self.line_spacing = Some(spacing);
        self
    }

    /// Sets the space before the paragraph.
    #[must_use]
    pub fn with_space_before(mut self, spacing: TextSpacing) -> Self {
        self.space_before = Some(spacing);
        self
    }

    /// Sets the space after the paragraph.
    #[must_use]
    pub fn with_space_after(mut self, spacing: TextSpacing) -> Self {
        self.space_after = Some(spacing);
        self
    }

    /// Sets what marks the paragraph.
    #[must_use]
    pub fn with_bullet(mut self, bullet: Bullet) -> Self {
        self.bullet = Some(bullet);
        self
    }

    /// Marks the paragraph with a literal character — `with_bullet_character("•")`.
    #[must_use]
    pub fn with_bullet_character(self, character: &str) -> Self {
        self.with_bullet(Bullet::Character(BulletCharacter::new(character)))
    }

    /// Gives the paragraph no bullet at all, **overriding** any it would otherwise inherit. This is
    /// `a:buNone`, a decision — not the same as never naming a bullet.
    #[must_use]
    pub fn without_bullet(self) -> Self {
        self.with_bullet(Bullet::None)
    }

    /// Sets the bullet's colour.
    #[must_use]
    pub fn with_bullet_color(mut self, color: BulletColor) -> Self {
        self.bullet_color = Some(color);
        self
    }

    /// Sets the bullet's size.
    #[must_use]
    pub fn with_bullet_size(mut self, size: BulletSize) -> Self {
        self.bullet_size = Some(size);
        self
    }

    /// Sets the bullet's typeface — a character bullet usually needs one, since the glyph has to
    /// exist in the font.
    #[must_use]
    pub fn with_bullet_typeface(mut self, typeface: BulletTypeface) -> Self {
        self.bullet_typeface = Some(typeface);
        self
    }

    /// Sets the tab stops, replacing any already named.
    #[must_use]
    pub fn with_tab_stops(mut self, stops: Vec<TabStop>) -> Self {
        self.tab_stops = stops;
        self
    }

    /// Sets the character properties this paragraph's runs default to — the tier a run's own
    /// properties override.
    #[must_use]
    pub fn with_default_run_properties(mut self, properties: CharacterPropertiesSpec) -> Self {
        self.default_run_properties = Some(properties);
        self
    }

    /// The indent level, if set.
    #[must_use]
    pub fn level(&self) -> Option<IndentLevel> {
        self.level
    }

    /// The horizontal alignment, if set.
    #[must_use]
    pub fn alignment(&self) -> Option<TextAlignment> {
        self.alignment
    }

    /// The left margin in points, if set.
    #[must_use]
    pub fn left_margin_points(&self) -> Option<f64> {
        self.left_margin.map(Emu::points)
    }

    /// The right margin in points, if set.
    #[must_use]
    pub fn right_margin_points(&self) -> Option<f64> {
        self.right_margin.map(Emu::points)
    }

    /// The first-line indent in points, if set (negative for a hanging indent).
    #[must_use]
    pub fn indent_points(&self) -> Option<f64> {
        self.indent.map(Emu::points)
    }

    /// The default tab gap in points, if set.
    #[must_use]
    pub fn default_tab_size_points(&self) -> Option<f64> {
        self.default_tab_size.map(Emu::points)
    }

    /// The reading direction, if set.
    #[must_use]
    pub fn is_right_to_left(&self) -> Option<bool> {
        self.right_to_left
    }

    /// The font alignment, if set.
    #[must_use]
    pub fn font_alignment(&self) -> Option<FontAlignment> {
        self.font_alignment
    }

    /// The spacing between lines, if set.
    #[must_use]
    pub fn line_spacing(&self) -> Option<TextSpacing> {
        self.line_spacing
    }

    /// The space before the paragraph, if set.
    #[must_use]
    pub fn space_before(&self) -> Option<TextSpacing> {
        self.space_before
    }

    /// The space after the paragraph, if set.
    #[must_use]
    pub fn space_after(&self) -> Option<TextSpacing> {
        self.space_after
    }

    /// What marks the paragraph, if set.
    #[must_use]
    pub fn bullet(&self) -> Option<&Bullet> {
        self.bullet.as_ref()
    }

    /// The bullet's colour, if set.
    #[must_use]
    pub fn bullet_color(&self) -> Option<&BulletColor> {
        self.bullet_color.as_ref()
    }

    /// The bullet's size, if set.
    #[must_use]
    pub fn bullet_size(&self) -> Option<BulletSize> {
        self.bullet_size
    }

    /// The bullet's typeface, if set.
    #[must_use]
    pub fn bullet_typeface(&self) -> Option<&BulletTypeface> {
        self.bullet_typeface.as_ref()
    }

    /// The tab stops, empty if none are named.
    #[must_use]
    pub fn tab_stops(&self) -> &[TabStop] {
        &self.tab_stops
    }

    /// The default run properties, if set.
    #[must_use]
    pub fn default_run_properties(&self) -> Option<&CharacterPropertiesSpec> {
        self.default_run_properties.as_ref()
    }

    /// Merges a **lower** inheritance tier under these properties: `self` wins wherever it names
    /// something, and `lower` supplies only what `self` leaves unset.
    ///
    /// This is one rung of the ladder a paragraph's *effective* layout is resolved along — the
    /// paragraph's own `a:pPr`, then the shape's list style at this level, then the same-slot
    /// placeholder's on the layout and master, then the master's `p:txStyles`, then the presentation
    /// default. Fold from the top: `paragraph.merge_under(&shape).merge_under(&layout)`.
    ///
    /// Three fields are not a plain field-wise fallback:
    ///
    /// - **Each bullet group merges as a unit.** A tier that sets `a:buChar` supplies the whole
    ///   bullet, never a field of one — and `Bullet::None` (`<a:buNone/>`) is a *present* value, so an
    ///   explicit "no bullet" correctly blocks an inherited one. The colour, size and typeface groups
    ///   inherit independently of it, which is why they are separate fields.
    /// - **Tab stops merge as a unit.** `a:tabLst` replaces wholesale in the schema, so an empty list
    ///   means "unset" and takes the lower tier's list entirely rather than concatenating.
    /// - **Default run properties merge recursively**, so a tier setting `a:defRPr@sz` does not shadow
    ///   a lower tier's `a:defRPr@b`.
    #[must_use]
    pub fn merge_under(mut self, lower: &Self) -> Self {
        self.level = self.level.or(lower.level);
        self.alignment = self.alignment.or(lower.alignment);
        self.left_margin = self.left_margin.or(lower.left_margin);
        self.right_margin = self.right_margin.or(lower.right_margin);
        self.indent = self.indent.or(lower.indent);
        self.default_tab_size = self.default_tab_size.or(lower.default_tab_size);
        self.right_to_left = self.right_to_left.or(lower.right_to_left);
        self.font_alignment = self.font_alignment.or(lower.font_alignment);
        self.line_spacing = self.line_spacing.or(lower.line_spacing);
        self.space_before = self.space_before.or(lower.space_before);
        self.space_after = self.space_after.or(lower.space_after);

        self.bullet = self.bullet.or_else(|| lower.bullet.clone());
        self.bullet_color = self.bullet_color.or_else(|| lower.bullet_color.clone());
        self.bullet_size = self.bullet_size.or(lower.bullet_size);
        self.bullet_typeface = self
            .bullet_typeface
            .or_else(|| lower.bullet_typeface.clone());

        if self.tab_stops.is_empty() {
            self.tab_stops = lower.tab_stops.clone();
        }

        self.default_run_properties = match (
            self.default_run_properties.take(),
            lower.default_run_properties.as_ref(),
        ) {
            (Some(higher), Some(lower)) => Some(higher.merge_under(lower)),
            (higher, lower) => higher.or_else(|| lower.cloned()),
        };
        self
    }

    /// Builds a **fresh** element under `local` (`pPr`, `defPPr` or `lvlNpPr`), in
    /// `CT_TextParagraphProperties` order: the attributes, then `a:lnSpc` → `a:spcBef` → `a:spcAft` →
    /// `a:tabLst` → `a:defRPr`.
    ///
    /// Only what the spec names is written. To keep an existing element's other state, merge with
    /// [`ParagraphProperties::apply`] instead.
    #[must_use]
    pub fn to_properties(&self, interner: &mut Interner, local: &str) -> ParagraphProperties {
        let mut properties = ParagraphProperties {
            name: dml_name(interner, local),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        properties.apply(self, interner);
        properties.empty = properties.children.is_empty();
        properties
    }
}
