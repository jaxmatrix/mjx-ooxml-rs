//! DrawingML fills: the `EG_FillProperties` choice — `a:noFill`, `a:solidFill`, `a:gradFill`,
//! `a:blipFill`, `a:pattFill`, `a:grpFill`.
//!
//! Each fill kind is a **fidelity wrapper** over its element (name, attributes, children, and the
//! self-closing flag preserved verbatim); the key values are exposed by typed accessors, while
//! rare/deep internals (blip effects, gradient shade paths, source/tile/fill rects) stay opaque so the
//! fill round-trips byte-for-byte. [`Fill`] is the exhaustive choice over all six, dispatching on the
//! element's local name.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::support::on_off;

use crate::build::{
    attr_by_local, attr_str, dml_attr, dml_child, dml_element, dml_name, fidelity_element_impls,
    first_color_child, prefixed_attr,
};
use crate::color::{Color, ColorKind, ColorSpec};
use crate::geometry::{Angle, Fraction};

pub use mjx_ooxml_types::drawingml::PatternType;

// ---------------------------------------------------------------------------------------------
// solidFill (existing)
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`SolidFill`]: the typed fill [`Color`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolidFillContent {
    /// The fill color (any `EG_ColorChoice` element).
    Color(Color),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:solidFill` (`CT_SolidColorFillProperties`) — a solid color fill: at most one color child.
///
/// The child is any `EG_ColorChoice` element (`a:srgbClr`, `a:schemeClr`, …), typed as [`Color`];
/// anything else is kept opaque so the fill round-trips. The color is optional (an empty
/// `<a:solidFill/>` is schema-legal).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct SolidFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "srgbClr", variant = Color, ty = Color),
        child(local = "schemeClr", variant = Color, ty = Color),
        child(local = "sysClr", variant = Color, ty = Color),
        child(local = "scrgbClr", variant = Color, ty = Color),
        child(local = "hslClr", variant = Color, ty = Color),
        child(local = "prstClr", variant = Color, ty = Color)
    )]
    content: Vec<SolidFillContent>,
}

impl SolidFill {
    /// Builds an `a:solidFill` around `color` (a self-closing `<a:solidFill/>` when `None`).
    #[must_use]
    pub fn new(interner: &mut Interner, color: Option<Color>) -> Self {
        let empty = color.is_none();
        Self {
            name: dml_name(interner, "solidFill"),
            attributes: Vec::new(),
            empty,
            content: color.into_iter().map(SolidFillContent::Color).collect(),
        }
    }

    /// The fill color, if present.
    #[must_use]
    pub fn color(&self) -> Option<&Color> {
        self.content.iter().find_map(|item| match item {
            SolidFillContent::Color(color) => Some(color),
            SolidFillContent::Raw(_) => None,
        })
    }

    /// The fill's ordered content (the typed color interleaved with any opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[SolidFillContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// noFill / grpFill (empty markers)
// ---------------------------------------------------------------------------------------------

/// `a:noFill` (`CT_NoFillProperties`) — an explicit "no fill". An empty element: no children, no
/// attributes; preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl NoFill {
    /// Builds a self-closing `<a:noFill/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "noFill"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }
}

fidelity_element_impls!(NoFill);

/// `a:grpFill` (`CT_GroupFillProperties`) — "inherit the group's fill". An empty element: no
/// children, no attributes; preserved verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl GroupFill {
    /// Builds a self-closing `<a:grpFill/>`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "grpFill"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }
}

fidelity_element_impls!(GroupFill);

// ---------------------------------------------------------------------------------------------
// gradFill
// ---------------------------------------------------------------------------------------------

/// One parsed gradient stop: a [`Fraction`] position and its [`Color`]. A read-only **view** over an
/// `a:gs`, not a fidelity type — build a gradient with [`GradientFill::linear`].
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    /// The stop's position along the gradient (`0.0`..=`1.0`; the `@pos` percentage).
    pub position: Fraction,
    /// The stop's color.
    pub color: Color,
}

/// `a:gradFill` (`CT_GradientFillProperties`) — a gradient fill: an ordered stop list (`gsLst`), an
/// optional shade (`a:lin` linear or `a:path`), and an optional `tileRect`; attributes `@flip` /
/// `@rotWithShape`.
///
/// The stop list and linear angle are exposed typed; the shade path, tile rect, and any other
/// internals are preserved opaque so the fill round-trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl GradientFill {
    /// Builds a linear `a:gradFill` from `stops` (position + color) at `angle`, emitting
    /// `<a:gradFill><a:gsLst>…</a:gsLst><a:lin ang="…"/></a:gradFill>`.
    #[must_use]
    pub fn linear(interner: &mut Interner, stops: &[(Fraction, Color)], angle: Angle) -> Self {
        build_gradient(interner, stops, Some(angle))
    }

    /// The gradient stops (`gsLst > gs`), in order — each stop's `@pos` as a [`Fraction`] and its
    /// color child. Stops missing a position or color are skipped.
    #[must_use]
    pub fn stops(&self, interner: &Interner) -> Vec<GradientStop> {
        let Some(gs_lst) = dml_child(&self.children, interner, "gsLst") else {
            return Vec::new();
        };
        gs_lst
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(gs)
                    if crate::build::is_dml(&gs.name, interner)
                        && interner.resolve(gs.name.local) == "gs" =>
                {
                    let position =
                        attr_str(&gs.attributes, interner, "pos").and_then(parse_percentage)?;
                    let color = first_color_child(gs, interner)?;
                    Some(GradientStop { position, color })
                }
                _ => None,
            })
            .collect()
    }

    /// The linear-shade angle (`a:lin@ang`), or `None` if this gradient has no linear shade.
    #[must_use]
    pub fn linear_angle(&self, interner: &Interner) -> Option<Angle> {
        let lin = dml_child(&self.children, interner, "lin")?;
        attr_str(&lin.attributes, interner, "ang").and_then(parse_angle)
    }

    /// The tile flip mode (`@flip`: `none`/`x`/`y`/`xy`), verbatim, or `None` if unset.
    #[must_use]
    pub fn flip(&self, interner: &Interner) -> Option<&str> {
        attr_str(&self.attributes, interner, "flip")
    }

    /// Whether the gradient rotates with the shape (`@rotWithShape`), or `None` if unset.
    #[must_use]
    pub fn rot_with_shape(&self, interner: &Interner) -> Option<bool> {
        attr_str(&self.attributes, interner, "rotWithShape").and_then(on_off::from_wire)
    }
}

fidelity_element_impls!(GradientFill);

// ---------------------------------------------------------------------------------------------
// blipFill
// ---------------------------------------------------------------------------------------------

/// How a [`BlipFill`] maps its image onto the shape (`EG_FillModeProperties`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlipFillMode {
    /// `a:tile` — the image repeats.
    Tile,
    /// `a:stretch` — the image is stretched to fill.
    Stretch,
    /// Neither `a:tile` nor `a:stretch` is present.
    None,
}

/// `a:blipFill` (`CT_BlipFillProperties`) — an image fill: an `a:blip` image reference, an optional
/// `srcRect`, and an optional fill mode (`a:tile` / `a:stretch`); attributes `@dpi` / `@rotWithShape`.
///
/// The image relationship id and the fill mode are exposed typed; the blip's compression effects,
/// source rect, and tile/fill rects are preserved opaque so the fill round-trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlipFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl BlipFill {
    /// Builds an `a:blipFill` referencing the image relationship `rel_id` with the given `mode`,
    /// emitting `<a:blipFill><a:blip r:embed="{rel_id}"/>[<a:tile/>|<a:stretch/>]</a:blipFill>`.
    ///
    /// The `r` prefix binds the relationships namespace on the containing part's root element (the
    /// caller's responsibility); this builder emits the attribute prefixed, unresolved, as the reader
    /// stores it. The relationship itself (and the image part) must be added to the package separately.
    #[must_use]
    pub fn new(interner: &mut Interner, rel_id: &str, mode: BlipFillMode) -> Self {
        let embed = prefixed_attr(interner, "r", "embed", rel_id);
        let blip = RawNode::Element(dml_element(interner, "blip", vec![embed], Vec::new()));
        let mut children = vec![blip];
        match mode {
            BlipFillMode::Tile => {
                children.push(RawNode::Element(dml_element(
                    interner,
                    "tile",
                    Vec::new(),
                    Vec::new(),
                )));
            }
            BlipFillMode::Stretch => {
                children.push(RawNode::Element(dml_element(
                    interner,
                    "stretch",
                    Vec::new(),
                    Vec::new(),
                )));
            }
            BlipFillMode::None => {}
        }
        Self {
            name: dml_name(interner, "blipFill"),
            attributes: Vec::new(),
            children,
            empty: false,
        }
    }

    /// The embedded image relationship id (`a:blip@r:embed`), or `None` if absent. Resolve it against
    /// the source part's `.rels` to reach the image part.
    #[must_use]
    pub fn image_rel_id(&self, interner: &Interner) -> Option<&str> {
        let blip = dml_child(&self.children, interner, "blip")?;
        attr_by_local(&blip.attributes, interner, "embed")
    }

    /// The linked (external) image relationship id (`a:blip@r:link`), or `None` if absent.
    #[must_use]
    pub fn image_link_id(&self, interner: &Interner) -> Option<&str> {
        let blip = dml_child(&self.children, interner, "blip")?;
        attr_by_local(&blip.attributes, interner, "link")
    }

    /// The fill mode: [`Tile`](BlipFillMode::Tile) if an `a:tile` child is present,
    /// [`Stretch`](BlipFillMode::Stretch) if `a:stretch`, else [`None`](BlipFillMode::None).
    #[must_use]
    pub fn mode(&self, interner: &Interner) -> BlipFillMode {
        if dml_child(&self.children, interner, "tile").is_some() {
            BlipFillMode::Tile
        } else if dml_child(&self.children, interner, "stretch").is_some() {
            BlipFillMode::Stretch
        } else {
            BlipFillMode::None
        }
    }
}

fidelity_element_impls!(BlipFill);

// ---------------------------------------------------------------------------------------------
// pattFill
// ---------------------------------------------------------------------------------------------

/// `a:pattFill` (`CT_PatternFillProperties`) — a two-color preset pattern fill: attribute `@prst`
/// (the [`PatternType`]) with an `a:fgClr` foreground and an `a:bgClr` background (each wrapping one
/// color).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternFill {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl PatternFill {
    /// Builds `<a:pattFill prst="{preset}"><a:fgClr>{fg}</a:fgClr><a:bgClr>{bg}</a:bgClr></a:pattFill>`.
    #[must_use]
    pub fn new(interner: &mut Interner, preset: PatternType, fg: Color, bg: Color) -> Self {
        Self::from_parts(interner, Some(preset), Some(fg), Some(bg))
    }

    /// Builds an `a:pattFill` from optional parts — each of `@prst`, `a:fgClr`, `a:bgClr` is emitted
    /// only when present (all three are schema-optional). Behind [`FillSpec::to_fill`].
    #[must_use]
    pub fn from_parts(
        interner: &mut Interner,
        preset: Option<PatternType>,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> Self {
        let attributes = preset
            .map(|preset| vec![dml_attr(interner, "prst", preset.to_wire())])
            .unwrap_or_default();
        let mut children = Vec::new();
        if let Some(fg) = fg {
            let fg_node = RawNode::Element(fg.to_xml(interner));
            children.push(RawNode::Element(dml_element(
                interner,
                "fgClr",
                Vec::new(),
                vec![fg_node],
            )));
        }
        if let Some(bg) = bg {
            let bg_node = RawNode::Element(bg.to_xml(interner));
            children.push(RawNode::Element(dml_element(
                interner,
                "bgClr",
                Vec::new(),
                vec![bg_node],
            )));
        }
        let empty = children.is_empty();
        Self {
            name: dml_name(interner, "pattFill"),
            attributes,
            children,
            empty,
        }
    }

    /// The preset pattern (`@prst`), or `None` if unset or its token is unrecognized.
    #[must_use]
    pub fn preset(&self, interner: &Interner) -> Option<PatternType> {
        attr_str(&self.attributes, interner, "prst").and_then(PatternType::from_wire)
    }

    /// The foreground color (`a:fgClr`'s color child), or `None` if absent.
    #[must_use]
    pub fn foreground(&self, interner: &Interner) -> Option<Color> {
        let fg_clr = dml_child(&self.children, interner, "fgClr")?;
        first_color_child(fg_clr, interner)
    }

    /// The background color (`a:bgClr`'s color child), or `None` if absent.
    #[must_use]
    pub fn background(&self, interner: &Interner) -> Option<Color> {
        let bg_clr = dml_child(&self.children, interner, "bgClr")?;
        first_color_child(bg_clr, interner)
    }
}

fidelity_element_impls!(PatternFill);

// ---------------------------------------------------------------------------------------------
// Fill (the exhaustive choice)
// ---------------------------------------------------------------------------------------------

/// The `EG_FillProperties` choice — exactly one of the six DrawingML fill kinds. Each variant is a
/// fidelity wrapper, so a parsed fill re-serializes byte-for-byte. Dispatched on the element's local
/// name via [`FromXml`]/[`ToXml`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fill {
    /// `a:noFill`.
    None(NoFill),
    /// `a:solidFill`.
    Solid(SolidFill),
    /// `a:gradFill`.
    Gradient(GradientFill),
    /// `a:blipFill`.
    Blip(BlipFill),
    /// `a:pattFill`.
    Pattern(PatternFill),
    /// `a:grpFill`.
    Group(GroupFill),
}

impl Fill {
    /// Whether `local` names one of the six `EG_FillProperties` elements.
    #[must_use]
    pub fn is_fill_local(local: &str) -> bool {
        matches!(
            local,
            "noFill" | "solidFill" | "gradFill" | "blipFill" | "pattFill" | "grpFill"
        )
    }
}

impl FromXml for Fill {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(match interner.resolve(element.name.local) {
            "noFill" => Fill::None(NoFill::from_xml(element, interner)?),
            "solidFill" => Fill::Solid(SolidFill::from_xml(element, interner)?),
            "gradFill" => Fill::Gradient(GradientFill::from_xml(element, interner)?),
            "blipFill" => Fill::Blip(BlipFill::from_xml(element, interner)?),
            "pattFill" => Fill::Pattern(PatternFill::from_xml(element, interner)?),
            // Any other local name (a malformed or foreign fill) is preserved as a group-fill-shaped
            // fidelity wrapper so nothing is lost; callers dispatch on the modeled variants.
            _ => Fill::Group(GroupFill::from_xml(element, interner)?),
        })
    }
}

impl ToXml for Fill {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        match self {
            Fill::None(fill) => fill.to_xml(interner),
            Fill::Solid(fill) => fill.to_xml(interner),
            Fill::Gradient(fill) => fill.to_xml(interner),
            Fill::Blip(fill) => fill.to_xml(interner),
            Fill::Pattern(fill) => fill.to_xml(interner),
            Fill::Group(fill) => fill.to_xml(interner),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// FillSpec — the interner-free description
// ---------------------------------------------------------------------------------------------

/// One stop of a [`FillSpec::Gradient`]: an interner-free position + color.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStopSpec {
    /// The stop's position along the gradient (`0.0`..=`1.0`).
    pub position: Fraction,
    /// The stop's color.
    pub color: ColorSpec,
}

/// An interner-free description of a shape [`Fill`] — the friendly value an interner-less caller reads
/// and writes (`mjx-pptx`'s `shape_fill` / `set_shape_fill`). Convert with [`Fill::spec`] /
/// [`FillSpec::to_fill`]. A spec is a value description, not a fidelity view: converting a `Fill` to a
/// spec and back rebuilds the element from its key values and drops any opaque internals (blip
/// effects, gradient shade paths, tile/fill rects).
#[derive(Debug, Clone, PartialEq)]
pub enum FillSpec {
    /// `a:noFill` — an explicit "no fill".
    None,
    /// `a:solidFill` — a solid color (an absent color reads as [`ColorSpec::Other`] with
    /// [`Unknown`](ColorKind::Unknown)).
    Solid(ColorSpec),
    /// `a:gradFill` — a gradient: ordered stops and an optional linear angle (`None` when the source
    /// gradient has no `a:lin` linear shade).
    Gradient {
        /// The gradient stops, in order.
        stops: Vec<GradientStopSpec>,
        /// The linear-shade angle, if any.
        angle: Option<Angle>,
    },
    /// `a:blipFill` — an image fill referencing an image relationship id.
    Blip {
        /// The image relationship id (`a:blip@r:embed`, or `@r:link`).
        rel_id: String,
        /// How the image maps onto the shape.
        mode: BlipFillMode,
    },
    /// `a:pattFill` — a two-color preset pattern. Each part is schema-optional.
    Pattern {
        /// The preset pattern (`@prst`).
        preset: Option<PatternType>,
        /// The foreground color (`a:fgClr`).
        foreground: Option<ColorSpec>,
        /// The background color (`a:bgClr`).
        background: Option<ColorSpec>,
    },
    /// `a:grpFill` — inherit the group's fill.
    Group,
}

impl FillSpec {
    /// A solid fill of `color`.
    #[must_use]
    pub fn solid(color: ColorSpec) -> Self {
        FillSpec::Solid(color)
    }

    /// A linear gradient from `stops` at `angle`.
    #[must_use]
    pub fn linear_gradient(stops: Vec<GradientStopSpec>, angle: Angle) -> Self {
        FillSpec::Gradient {
            stops,
            angle: Some(angle),
        }
    }

    /// A preset pattern fill with a foreground and background color.
    #[must_use]
    pub fn pattern(preset: PatternType, foreground: ColorSpec, background: ColorSpec) -> Self {
        FillSpec::Pattern {
            preset: Some(preset),
            foreground: Some(foreground),
            background: Some(background),
        }
    }

    /// Builds the fidelity [`Fill`] for this description, interning against `interner`.
    #[must_use]
    pub fn to_fill(&self, interner: &mut Interner) -> Fill {
        match self {
            FillSpec::None => Fill::None(NoFill::new(interner)),
            FillSpec::Group => Fill::Group(GroupFill::new(interner)),
            FillSpec::Solid(color) => {
                let color = Color::from_spec(interner, color);
                Fill::Solid(SolidFill::new(interner, color))
            }
            FillSpec::Gradient { stops, angle } => {
                let pairs: Vec<(Fraction, Color)> = stops
                    .iter()
                    .filter_map(|stop| {
                        Color::from_spec(interner, &stop.color).map(|color| (stop.position, color))
                    })
                    .collect();
                Fill::Gradient(build_gradient(interner, &pairs, *angle))
            }
            FillSpec::Blip { rel_id, mode } => Fill::Blip(BlipFill::new(interner, rel_id, *mode)),
            FillSpec::Pattern {
                preset,
                foreground,
                background,
            } => {
                let fg = foreground
                    .as_ref()
                    .and_then(|spec| Color::from_spec(interner, spec));
                let bg = background
                    .as_ref()
                    .and_then(|spec| Color::from_spec(interner, spec));
                Fill::Pattern(PatternFill::from_parts(interner, *preset, fg, bg))
            }
        }
    }
}

impl Fill {
    /// This fill as an interner-free [`FillSpec`] — resolving the key values of the current variant
    /// and dropping opaque internals. Reading does not need a mutable interner.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> FillSpec {
        match self {
            Fill::None(_) => FillSpec::None,
            Fill::Group(_) => FillSpec::Group,
            Fill::Solid(solid) => FillSpec::Solid(solid.color().map_or(
                ColorSpec::Other {
                    kind: ColorKind::Unknown,
                    value: None,
                },
                |color| color.spec(interner),
            )),
            Fill::Gradient(gradient) => {
                let stops = gradient
                    .stops(interner)
                    .into_iter()
                    .map(|stop| GradientStopSpec {
                        position: stop.position,
                        color: stop.color.spec(interner),
                    })
                    .collect();
                FillSpec::Gradient {
                    stops,
                    angle: gradient.linear_angle(interner),
                }
            }
            Fill::Blip(blip) => FillSpec::Blip {
                rel_id: blip
                    .image_rel_id(interner)
                    .or_else(|| blip.image_link_id(interner))
                    .unwrap_or_default()
                    .to_owned(),
                mode: blip.mode(interner),
            },
            Fill::Pattern(pattern) => FillSpec::Pattern {
                preset: pattern.preset(interner),
                foreground: pattern
                    .foreground(interner)
                    .map(|color| color.spec(interner)),
                background: pattern
                    .background(interner)
                    .map(|color| color.spec(interner)),
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------------

/// Parses a DrawingML percentage to a [`Fraction`]: the integer form (`50000` = 50%, native/100000)
/// or an explicit-percent form (`50%`).
fn parse_percentage(s: &str) -> Option<Fraction> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix('%') {
        stripped
            .trim()
            .parse::<f64>()
            .ok()
            .map(|value| Fraction::from_ratio(value / 100.0))
    } else {
        s.parse::<f64>()
            .ok()
            .map(|value| Fraction::from_ratio(value / 100_000.0))
    }
}

/// Parses a DrawingML angle attribute (`@ang`, 60000ths of a degree) to an [`Angle`].
fn parse_angle(s: &str) -> Option<Angle> {
    s.trim()
        .parse::<f64>()
        .ok()
        .map(|value| Angle::from_degrees(value / 60_000.0))
}

/// Builds an `a:gradFill` with a `gsLst` of `stops` and, when `angle` is `Some`, an `a:lin` linear
/// shade. Behind [`GradientFill::linear`] and [`FillSpec::to_fill`].
fn build_gradient(
    interner: &mut Interner,
    stops: &[(Fraction, Color)],
    angle: Option<Angle>,
) -> GradientFill {
    let gs_nodes: Vec<RawNode> = stops
        .iter()
        .map(|(position, color)| {
            let pos = (position.ratio() * 100_000.0).round() as i64;
            let attributes = vec![dml_attr(interner, "pos", &pos.to_string())];
            let color_node = RawNode::Element(color.to_xml(interner));
            RawNode::Element(dml_element(interner, "gs", attributes, vec![color_node]))
        })
        .collect();
    let mut children = vec![RawNode::Element(dml_element(
        interner,
        "gsLst",
        Vec::new(),
        gs_nodes,
    ))];
    if let Some(angle) = angle {
        let ang = (angle.degrees() * 60_000.0).round() as i64;
        let lin_attributes = vec![dml_attr(interner, "ang", &ang.to_string())];
        children.push(RawNode::Element(dml_element(
            interner,
            "lin",
            lin_attributes,
            Vec::new(),
        )));
    }
    GradientFill {
        name: dml_name(interner, "gradFill"),
        attributes: Vec::new(),
        children,
        empty: false,
    }
}
