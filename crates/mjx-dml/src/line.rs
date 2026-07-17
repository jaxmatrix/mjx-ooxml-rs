//! DrawingML line (outline) properties: `a:ln` (`CT_LineProperties`) — the stroke a shape renders
//! around its geometry.
//!
//! [`LineProperties`] is a **fidelity wrapper** over the `a:ln` element (its name, attributes,
//! children, and self-closing flag preserved verbatim); the key values are exposed by typed accessors,
//! while rare/deep internals (custom dash stops, `extLst`) stay opaque so the outline round-trips
//! byte-for-byte. [`LineSpec`] is the interner-free value an interner-less caller (`mjx-pptx`'s
//! `shape_outline` / `set_shape_outline`) reads and writes.
//!
//! The stroke's own fill is an `EG_LineFillProperties` choice — a **subset** of the shape fill
//! (`noFill`/`solidFill`/`gradFill`/`pattFill`; no image or group fill) — so it reuses [`Fill`] /
//! [`FillSpec`] directly.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};

use crate::build::{
    attr_str, dml_attr, dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child,
    parse_percentage,
};
use crate::color::ColorSpec;
use crate::fill::{Fill, FillSpec};
use crate::geometry::{Fraction, LineWidth};

pub use mjx_ooxml_types::drawingml::{
    CompoundLine, LineCap, LineEndLength, LineEndType, LineEndWidth, PenAlignment, PresetLineDash,
};

// ---------------------------------------------------------------------------------------------
// Typed sub-values (interner-free)
// ---------------------------------------------------------------------------------------------

/// A line's dash style (`EG_LineDashProperties`): either a named preset (`a:prstDash@val`) or a custom
/// dash stop list (`a:custDash`) whose stops are kept **opaque** — [`LineProperties`] round-trips them
/// byte-for-byte, but the value tier does not model individual stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDash {
    /// `a:prstDash` — a preset dash pattern.
    Preset(PresetLineDash),
    /// `a:custDash` — a custom dash stop list, preserved verbatim but not modeled here.
    Custom,
}

/// A line's join style at corners (`EG_LineJoinProperties`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineJoin {
    /// `a:round` — rounded corners.
    Round,
    /// `a:bevel` — beveled (flattened) corners.
    Bevel,
    /// `a:miter` — mitered (pointed) corners, with an optional miter limit (`@lim`, a
    /// [`Fraction`] of the line width beyond which the join is beveled).
    Miter {
        /// The miter limit (`@lim`), if specified.
        limit: Option<Fraction>,
    },
}

/// A line end decoration (`CT_LineEndProperties`) — the arrowhead (or other cap) on a stroke's head
/// (`a:headEnd`) or tail (`a:tailEnd`). Each part is schema-optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineEnd {
    /// The end decoration shape (`@type`, default `none`).
    pub kind: Option<LineEndType>,
    /// The end decoration width relative to the line (`@w`).
    pub width: Option<LineEndWidth>,
    /// The end decoration length relative to the line (`@len`).
    pub length: Option<LineEndLength>,
}

// ---------------------------------------------------------------------------------------------
// LineProperties — the fidelity wrapper
// ---------------------------------------------------------------------------------------------

/// `a:ln` (`CT_LineProperties`) — a shape's outline: width/cap/compound/pen-alignment attributes, an
/// optional stroke [`Fill`], a dash, a join, and head/tail line ends.
///
/// A fidelity wrapper: the width, cap, compound, and pen-alignment attributes and the key children are
/// exposed typed, while any custom dash stops, `extLst`, and unknown attributes/children are preserved
/// opaque so the outline round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl LineProperties {
    /// The line width (`@w`, in EMU), or `None` if unset (an inherited/default width).
    #[must_use]
    pub fn width(&self, interner: &Interner) -> Option<LineWidth> {
        attr_str(&self.attributes, interner, "w")
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(LineWidth::from_emu)
    }

    /// The line end cap (`@cap`), or `None` if unset.
    #[must_use]
    pub fn cap(&self, interner: &Interner) -> Option<LineCap> {
        attr_str(&self.attributes, interner, "cap").and_then(LineCap::from_wire)
    }

    /// The compound line type (`@cmpd`), or `None` if unset.
    #[must_use]
    pub fn compound(&self, interner: &Interner) -> Option<CompoundLine> {
        attr_str(&self.attributes, interner, "cmpd").and_then(CompoundLine::from_wire)
    }

    /// The pen alignment (`@algn`), or `None` if unset.
    #[must_use]
    pub fn pen_alignment(&self, interner: &Interner) -> Option<PenAlignment> {
        attr_str(&self.attributes, interner, "algn").and_then(PenAlignment::from_wire)
    }

    /// The stroke fill (`EG_LineFillProperties`: `a:noFill`/`a:solidFill`/`a:gradFill`/`a:pattFill`),
    /// or `None` if the line declares none.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        first_fill_child(&self.children, interner).and_then(|el| Fill::from_xml(el, interner).ok())
    }

    /// The dash style (`a:prstDash` or `a:custDash`), or `None` if the line declares none. A
    /// `prstDash` whose `@val` is absent or unrecognized reads as `None`.
    #[must_use]
    pub fn dash(&self, interner: &Interner) -> Option<LineDash> {
        if let Some(prst) = dml_child(&self.children, interner, "prstDash") {
            return attr_str(&prst.attributes, interner, "val")
                .and_then(PresetLineDash::from_wire)
                .map(LineDash::Preset);
        }
        dml_child(&self.children, interner, "custDash").map(|_| LineDash::Custom)
    }

    /// The join style (`a:round`/`a:bevel`/`a:miter`), or `None` if the line declares none.
    #[must_use]
    pub fn join(&self, interner: &Interner) -> Option<LineJoin> {
        if dml_child(&self.children, interner, "round").is_some() {
            return Some(LineJoin::Round);
        }
        if dml_child(&self.children, interner, "bevel").is_some() {
            return Some(LineJoin::Bevel);
        }
        if let Some(miter) = dml_child(&self.children, interner, "miter") {
            let limit = attr_str(&miter.attributes, interner, "lim").and_then(parse_percentage);
            return Some(LineJoin::Miter { limit });
        }
        None
    }

    /// The head-end decoration (`a:headEnd`), or `None` if absent.
    #[must_use]
    pub fn head_end(&self, interner: &Interner) -> Option<LineEnd> {
        dml_child(&self.children, interner, "headEnd").map(|el| read_line_end(el, interner))
    }

    /// The tail-end decoration (`a:tailEnd`), or `None` if absent.
    #[must_use]
    pub fn tail_end(&self, interner: &Interner) -> Option<LineEnd> {
        dml_child(&self.children, interner, "tailEnd").map(|el| read_line_end(el, interner))
    }

    /// This outline as an interner-free [`LineSpec`] — resolving the key values and dropping opaque
    /// internals (custom dash stops, `extLst`). Reading does not need a mutable interner.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> LineSpec {
        LineSpec {
            width: self.width(interner),
            cap: self.cap(interner),
            compound: self.compound(interner),
            pen_alignment: self.pen_alignment(interner),
            fill: self.fill(interner).map(|fill| fill.spec(interner)),
            dash: self.dash(interner),
            join: self.join(interner),
            head_end: self.head_end(interner),
            tail_end: self.tail_end(interner),
        }
    }
}

fidelity_element_impls!(LineProperties);

/// Reads a `CT_LineEndProperties` element (`a:headEnd`/`a:tailEnd`) into a [`LineEnd`].
fn read_line_end(element: &RawElement, interner: &Interner) -> LineEnd {
    LineEnd {
        kind: attr_str(&element.attributes, interner, "type").and_then(LineEndType::from_wire),
        width: attr_str(&element.attributes, interner, "w").and_then(LineEndWidth::from_wire),
        length: attr_str(&element.attributes, interner, "len").and_then(LineEndLength::from_wire),
    }
}

// ---------------------------------------------------------------------------------------------
// LineSpec — the interner-free description
// ---------------------------------------------------------------------------------------------

/// An interner-free description of a shape outline (`a:ln`) — the friendly value an interner-less
/// caller reads and writes (`mjx-pptx`'s `shape_outline` / `set_shape_outline`). Convert with
/// [`LineProperties::spec`] / [`LineSpec::to_line`]. A spec is a value description, not a fidelity view:
/// converting a `LineProperties` to a spec and back rebuilds the element from its key values and drops
/// any opaque internals (custom dash stops, `extLst`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineSpec {
    /// The line width (`@w`, EMU).
    pub width: Option<LineWidth>,
    /// The end cap (`@cap`).
    pub cap: Option<LineCap>,
    /// The compound line type (`@cmpd`).
    pub compound: Option<CompoundLine>,
    /// The pen alignment (`@algn`).
    pub pen_alignment: Option<PenAlignment>,
    /// The stroke fill (`EG_LineFillProperties`).
    pub fill: Option<FillSpec>,
    /// The dash style. A [`LineDash::Custom`] rebuilds only an empty `<a:custDash/>` (its stops are
    /// not modeled).
    pub dash: Option<LineDash>,
    /// The join style.
    pub join: Option<LineJoin>,
    /// The head-end decoration (`a:headEnd`).
    pub head_end: Option<LineEnd>,
    /// The tail-end decoration (`a:tailEnd`).
    pub tail_end: Option<LineEnd>,
}

impl LineSpec {
    /// An empty outline (all parts unset) — the same as [`LineSpec::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A solid-colored outline of `width` filled with `color`.
    #[must_use]
    pub fn solid(width: LineWidth, color: ColorSpec) -> Self {
        Self {
            width: Some(width),
            fill: Some(FillSpec::Solid(color)),
            ..Self::default()
        }
    }

    /// Builds the fidelity [`LineProperties`] for this description, interning against `interner`. The
    /// element is assembled in `CT_LineProperties` order: attributes `w`/`cap`/`cmpd`/`algn`, then
    /// children fill → dash → join → `headEnd` → `tailEnd`.
    #[must_use]
    pub fn to_line(&self, interner: &mut Interner) -> LineProperties {
        let mut attributes = Vec::new();
        if let Some(width) = self.width {
            attributes.push(dml_attr(interner, "w", &width.emu().to_string()));
        }
        if let Some(cap) = self.cap {
            attributes.push(dml_attr(interner, "cap", cap.to_wire()));
        }
        if let Some(compound) = self.compound {
            attributes.push(dml_attr(interner, "cmpd", compound.to_wire()));
        }
        if let Some(algn) = self.pen_alignment {
            attributes.push(dml_attr(interner, "algn", algn.to_wire()));
        }

        let mut children = Vec::new();
        if let Some(fill) = &self.fill {
            children.push(RawNode::Element(fill.to_fill(interner).to_xml(interner)));
        }
        if let Some(dash) = self.dash {
            children.push(RawNode::Element(build_dash(interner, dash)));
        }
        if let Some(join) = self.join {
            children.push(RawNode::Element(build_join(interner, join)));
        }
        if let Some(head) = &self.head_end {
            children.push(RawNode::Element(build_line_end(interner, "headEnd", head)));
        }
        if let Some(tail) = &self.tail_end {
            children.push(RawNode::Element(build_line_end(interner, "tailEnd", tail)));
        }

        LineProperties {
            name: dml_name(interner, "ln"),
            attributes,
            empty: children.is_empty(),
            children,
        }
    }
}

/// Builds an `a:prstDash`/`a:custDash` element for a [`LineDash`].
fn build_dash(interner: &mut Interner, dash: LineDash) -> RawElement {
    match dash {
        LineDash::Preset(preset) => {
            let attributes = vec![dml_attr(interner, "val", preset.to_wire())];
            dml_element(interner, "prstDash", attributes, Vec::new())
        }
        LineDash::Custom => dml_element(interner, "custDash", Vec::new(), Vec::new()),
    }
}

/// Builds an `a:round`/`a:bevel`/`a:miter` element for a [`LineJoin`].
fn build_join(interner: &mut Interner, join: LineJoin) -> RawElement {
    match join {
        LineJoin::Round => dml_element(interner, "round", Vec::new(), Vec::new()),
        LineJoin::Bevel => dml_element(interner, "bevel", Vec::new(), Vec::new()),
        LineJoin::Miter { limit } => {
            let attributes = limit
                .map(|f| {
                    let native = (f.ratio() * 100_000.0).round() as i64;
                    vec![dml_attr(interner, "lim", &native.to_string())]
                })
                .unwrap_or_default();
            dml_element(interner, "miter", attributes, Vec::new())
        }
    }
}

/// Builds an `a:headEnd`/`a:tailEnd` element from a [`LineEnd`].
fn build_line_end(interner: &mut Interner, local: &str, end: &LineEnd) -> RawElement {
    let mut attributes = Vec::new();
    if let Some(kind) = end.kind {
        attributes.push(dml_attr(interner, "type", kind.to_wire()));
    }
    if let Some(width) = end.width {
        attributes.push(dml_attr(interner, "w", width.to_wire()));
    }
    if let Some(length) = end.length {
        attributes.push(dml_attr(interner, "len", length.to_wire()));
    }
    dml_element(interner, local, attributes, Vec::new())
}
