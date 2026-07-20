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

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};
use mjx_ooxml_types::support::on_off;

use crate::build::{
    attr_str, dml_attr, dml_child, dml_element, dml_name, fidelity_element_impls, is_dml,
    parse_percentage, replace_or_insert_child, set_attr,
};
use crate::geometry::{Emu, Fraction, IndentLevel, TextPoint};
use crate::text::character::{CharacterProperties, CharacterPropertiesSpec};

pub use mjx_ooxml_types::drawingml::{FontAlignment, TabAlignment, TextAlignment};

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
/// spacing, tab stops, and the character properties its runs default to.
///
/// A fidelity wrapper: the modeled properties are typed, while the line-breaking attributes
/// (`eaLnBrk`, `latinLnBrk`, `hangingPunct`), `extLst`, the bullet groups and anything unknown are
/// preserved verbatim so a paragraph round-trips byte-for-byte. The element name is preserved too, so
/// the same type reads and writes `a:pPr`, `a:defPPr` and each `a:lvlNpPr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl ParagraphProperties {
    /// The indent level (`@lvl`), or `None` if unset.
    ///
    /// Unset means the paragraph inherits its level, which resolves to [`IndentLevel::TOP`] — but that
    /// substitution belongs to resolution, not to reading, so it is not made here.
    #[must_use]
    pub fn level(&self, interner: &Interner) -> Option<IndentLevel> {
        attr_str(&self.attributes, interner, "lvl")
            .and_then(|s| s.trim().parse::<u8>().ok())
            .and_then(IndentLevel::new)
    }

    /// The horizontal alignment (`@algn`), or `None` if unset.
    #[must_use]
    pub fn alignment(&self, interner: &Interner) -> Option<TextAlignment> {
        attr_str(&self.attributes, interner, "algn").and_then(TextAlignment::from_wire)
    }

    /// The left margin (`@marL`) — the whole paragraph's inset — or `None` if unset.
    #[must_use]
    pub fn left_margin(&self, interner: &Interner) -> Option<Emu> {
        self.emu_attribute(interner, "marL")
    }

    /// The right margin (`@marR`), or `None` if unset.
    #[must_use]
    pub fn right_margin(&self, interner: &Interner) -> Option<Emu> {
        self.emu_attribute(interner, "marR")
    }

    /// The first-line indent (`@indent`), relative to the left margin, or `None` if unset. A
    /// **negative** value hangs the first line out to the left of the rest — how a bullet sits in the
    /// margin of its text.
    #[must_use]
    pub fn indent(&self, interner: &Interner) -> Option<Emu> {
        self.emu_attribute(interner, "indent")
    }

    /// The default gap between tab stops (`@defTabSz`), or `None` if unset.
    #[must_use]
    pub fn default_tab_size(&self, interner: &Interner) -> Option<Emu> {
        self.emu_attribute(interner, "defTabSz")
    }

    /// Whether the paragraph runs right-to-left (`@rtl`), or `None` if unset.
    #[must_use]
    pub fn is_right_to_left(&self, interner: &Interner) -> Option<bool> {
        attr_str(&self.attributes, interner, "rtl").and_then(on_off::from_wire)
    }

    /// Where letters sit between the baselines (`@fontAlgn`), or `None` if unset.
    #[must_use]
    pub fn font_alignment(&self, interner: &Interner) -> Option<FontAlignment> {
        attr_str(&self.attributes, interner, "fontAlgn").and_then(FontAlignment::from_wire)
    }

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
                    Some(TabStop {
                        position: attr_str(&child.attributes, interner, "pos")
                            .and_then(|s| s.trim().parse::<i64>().ok())
                            .map_or(Emu::from_emu(0), Emu::from_emu),
                        alignment: attr_str(&child.attributes, interner, "algn")
                            .and_then(TabAlignment::from_wire),
                    })
                }
                _ => None,
            })
            .collect()
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
        ParagraphPropertiesSpec {
            level: self.level(interner),
            alignment: self.alignment(interner),
            left_margin: self.left_margin(interner),
            right_margin: self.right_margin(interner),
            indent: self.indent(interner),
            default_tab_size: self.default_tab_size(interner),
            right_to_left: self.is_right_to_left(interner),
            font_alignment: self.font_alignment(interner),
            line_spacing: self.line_spacing(interner),
            space_before: self.space_before(interner),
            space_after: self.space_after(interner),
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
        if let Some(level) = spec.level {
            set_attr(
                &mut self.attributes,
                interner,
                "lvl",
                &level.value().to_string(),
            );
        }
        if let Some(alignment) = spec.alignment {
            set_attr(&mut self.attributes, interner, "algn", alignment.to_wire());
        }
        for (name, value) in [
            ("marL", spec.left_margin),
            ("marR", spec.right_margin),
            ("indent", spec.indent),
            ("defTabSz", spec.default_tab_size),
        ] {
            if let Some(value) = value {
                set_attr(
                    &mut self.attributes,
                    interner,
                    name,
                    &value.emu().to_string(),
                );
            }
        }
        if let Some(rtl) = spec.right_to_left {
            set_attr(&mut self.attributes, interner, "rtl", on_off::to_wire(rtl));
        }
        if let Some(font_alignment) = spec.font_alignment {
            set_attr(
                &mut self.attributes,
                interner,
                "fontAlgn",
                font_alignment.to_wire(),
            );
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

    /// The value of an EMU-valued attribute.
    fn emu_attribute(&self, interner: &Interner, local: &str) -> Option<Emu> {
        attr_str(&self.attributes, interner, local)
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(Emu::from_emu)
    }

    /// One of the three `CT_TextSpacing` children, read as a [`TextSpacing`].
    fn spacing_child(&self, interner: &Interner, local: &str) -> Option<TextSpacing> {
        let element = dml_child(&self.children, interner, local)?;
        element.children.iter().find_map(|node| match node {
            RawNode::Element(child) if is_dml(&child.name, interner) => {
                let value = attr_str(&child.attributes, interner, "val")?;
                match interner.resolve(child.name.local) {
                    "spcPct" => parse_percentage(value).map(TextSpacing::Percentage),
                    "spcPts" => value
                        .trim()
                        .parse::<i32>()
                        .ok()
                        .map(|points| TextSpacing::Points(TextPoint::from_wire(points))),
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
        replace_or_insert_child(&mut self.children, interner, element, matches, known_rank);
        self.empty = false;
    }
}

fidelity_element_impls!(ParagraphProperties);

/// Children of `CT_TextParagraphProperties`, in schema order. Anything else sorts last.
///
/// The bullet groups sit between the spacing elements and `a:tabLst`, so they are ranked here even
/// though this model does not yet write them — an existing bullet must not be stepped over.
fn known_rank(local: &str) -> Option<usize> {
    let rank = match local {
        "lnSpc" => 0,
        "spcBef" => 1,
        "spcAft" => 2,
        "buClrTx" | "buClr" => 3,
        "buSzTx" | "buSzPct" | "buSzPts" => 4,
        "buFontTx" | "buFont" => 5,
        "buNone" | "buAutoNum" | "buChar" | "buBlip" => 6,
        "tabLst" => 7,
        "defRPr" => 8,
        "extLst" => 9,
        _ => return None,
    };
    Some(rank)
}

/// Builds one of the `CT_TextSpacing` children (`a:lnSpc`, `a:spcBef`, `a:spcAft`).
fn build_spacing(interner: &mut Interner, local: &str, spacing: TextSpacing) -> RawElement {
    let inner = match spacing {
        TextSpacing::Percentage(fraction) => {
            let native = (fraction.ratio() * 100_000.0).round() as i64;
            let attributes = vec![dml_attr(interner, "val", &native.to_string())];
            dml_element(interner, "spcPct", attributes, Vec::new())
        }
        TextSpacing::Points(points) => {
            let attributes = vec![dml_attr(interner, "val", &points.to_wire().to_string())];
            dml_element(interner, "spcPts", attributes, Vec::new())
        }
    };
    dml_element(interner, local, Vec::new(), vec![RawNode::Element(inner)])
}

/// Builds the `a:tabLst` for a set of tab stops.
fn build_tab_stops(interner: &mut Interner, stops: &[TabStop]) -> RawElement {
    let children = stops
        .iter()
        .map(|stop| {
            let mut attributes = vec![dml_attr(interner, "pos", &stop.position.emu().to_string())];
            if let Some(alignment) = stop.alignment {
                attributes.push(dml_attr(interner, "algn", alignment.to_wire()));
            }
            RawNode::Element(dml_element(interner, "tab", attributes, Vec::new()))
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
