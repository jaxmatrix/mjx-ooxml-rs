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

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use crate::build::{dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child};
use crate::codec::{EmuLineWidth, Percentage};
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
// The attribute faces of the outline's child elements
// ---------------------------------------------------------------------------------------------
//
// A dash preset, a miter limit and a line end are read out of the outline's own children and
// projected into the interner-free values above; none of the three is a modeled type. Each declares
// its attributes through the `#[xml(attribute(..))]` grammar over the vector it is handed — borrowed
// to read, a fresh one to write — so one declaration serves both directions.

/// `a:prstDash` (`CT_PresetLineDashProperties`) — the attribute face of a named dash pattern.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", codec = Enumeration<PresetLineDash>, accessor = preset, required))]
struct PresetDashAttributes<A> {
    attributes: A,
}

/// `a:miter` (`CT_LineJoinMiterProperties`) — the attribute face of a mitered join.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lim", codec = Percentage, accessor = limit))]
struct MiterJoinAttributes<A> {
    attributes: A,
}

/// `a:headEnd` / `a:tailEnd` (`CT_LineEndProperties`) — the attribute face of an end decoration.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", codec = Enumeration<LineEndType>, accessor = kind))]
#[xml(attribute(local = "w", codec = Enumeration<LineEndWidth>, accessor = width))]
#[xml(attribute(local = "len", codec = Enumeration<LineEndLength>, accessor = length))]
struct LineEndAttributes<A> {
    attributes: A,
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
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "w", codec = EmuLineWidth, accessor = width))]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = cap))]
#[xml(attribute(local = "cmpd", codec = Enumeration<CompoundLine>, accessor = compound))]
#[xml(attribute(local = "algn", codec = Enumeration<PenAlignment>, accessor = pen_alignment))]
pub struct LineProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl LineProperties {
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
            return PresetDashAttributes {
                attributes: &prst.attributes,
            }
            .preset(interner)
            .ok()
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
            let limit = MiterJoinAttributes {
                attributes: &miter.attributes,
            }
            .limit(interner)
            .ok()
            .flatten();
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
            width: self.width(interner).ok().flatten(),
            cap: self.cap(interner).ok().flatten(),
            compound: self.compound(interner).ok().flatten(),
            pen_alignment: self.pen_alignment(interner).ok().flatten(),
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
    let end = LineEndAttributes {
        attributes: &element.attributes,
    };
    LineEnd {
        kind: end.kind(interner).ok().flatten(),
        width: end.width(interner).ok().flatten(),
        length: end.length(interner).ok().flatten(),
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

    /// Builds the fidelity [`LineProperties`] for this description as an `a:ln` element. Use
    /// [`to_line_named`](Self::to_line_named) to emit the same `CT_LineProperties` body under another
    /// tag, such as the underline line group `a:uLn`.
    #[must_use]
    pub fn to_line(&self, interner: &mut Interner) -> LineProperties {
        self.to_line_named(interner, "ln")
    }

    /// Builds the fidelity [`LineProperties`] for this description under `local`, interning against
    /// `interner`. `local` is the element's local name (`ln` for an outline, `uLn` for the underline
    /// line group) — the body is the same `CT_LineProperties` either way. The element is assembled in
    /// schema order: attributes `w`/`cap`/`cmpd`/`algn`, then children fill → dash → join → `headEnd`
    /// → `tailEnd`.
    #[must_use]
    pub fn to_line_named(&self, interner: &mut Interner, local: &str) -> LineProperties {
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

        let mut line = LineProperties {
            name: dml_name(interner, local),
            attributes: Vec::new(),
            empty: children.is_empty(),
            children,
        };
        // Schema order: `w`, `cap`, `cmpd`, `algn`. A setter appends to an empty vector in call
        // order, so the order of these four calls is the order they are written in.
        line.set_width(interner, self.width);
        line.set_cap(interner, self.cap);
        line.set_compound(interner, self.compound);
        line.set_pen_alignment(interner, self.pen_alignment);
        line
    }
}

/// Builds an `a:prstDash`/`a:custDash` element for a [`LineDash`].
fn build_dash(interner: &mut Interner, dash: LineDash) -> RawElement {
    match dash {
        LineDash::Preset(preset) => {
            let mut dash = PresetDashAttributes {
                attributes: Vec::new(),
            };
            dash.set_preset(interner, preset);
            dml_element(interner, "prstDash", dash.attributes, Vec::new())
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
            let mut miter = MiterJoinAttributes {
                attributes: Vec::new(),
            };
            miter.set_limit(interner, limit);
            dml_element(interner, "miter", miter.attributes, Vec::new())
        }
    }
}

/// Builds an `a:headEnd`/`a:tailEnd` element from a [`LineEnd`].
fn build_line_end(interner: &mut Interner, local: &str, end: &LineEnd) -> RawElement {
    let mut end_attributes = LineEndAttributes {
        attributes: Vec::new(),
    };
    end_attributes.set_kind(interner, end.kind);
    end_attributes.set_width(interner, end.width);
    end_attributes.set_length(interner, end.length);
    dml_element(interner, local, end_attributes.attributes, Vec::new())
}
