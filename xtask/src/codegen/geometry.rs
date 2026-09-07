//! Extracts the ECMA-376 `presetShapeDefinitions.xml` twice over, for two crates in two tiers.
//!
//! # What comes out of it
//!
//! * **Adjustment metadata**, rendered into `mjx-ooxml-types::drawingml` as `adjustments_of`,
//!   `adjustable_shapes` and `adjustment_bound_guides_of` — [`emit_shape_adjustments`].
//! * **The whole geometry**, rendered into `mjx-geometry`'s `generated.rs` as the
//!   `PresetShapeDefinition` table every preset shape is drawn from — [`emit_preset_geometry`].
//!
//! The two are one parse and one shape model. They are two outputs because they are read by two
//! crates four ranks apart: `mjx-dml` needs an adjustment's *domain* and nothing else, while the
//! renderer needs the paths.
//!
//! # The adjustment slice
//!
//! A preset shape's user-facing adjustments are exactly the `avLst` guides that some `ahLst` handle
//! references via `gdRef{X,Y,Ang,R}`; the referencing attribute discloses the axis (X = horizontal,
//! Y = vertical, Ang = angle, R = radius). The default is the guide's `val N` seed; the domain is the
//! handle's `min*`/`max*` — a literal, or the name of a computed `gdLst` guide. `avLst` entries with
//! **no** handle (e.g. `star5.hf/vf`, all of `pentagon`) are constants and are dropped.
//!
//! A guide-named bound is only a number once the shape's size is known, so the third table carries
//! the `gdLst` guides those bounds are computed from: the **transitive closure** of every named bound
//! over the shape's own `gdLst`, in declaration order. It is a deliberate slice of the shape's
//! geometry — 334 of the file's 3923 `a:gd` elements — because that is all resolving a domain needs.
//! (**334, and this line said 335 until MJXOFF-203 counted it**;
//! `crates/mjx-geometry/tests/the_generated_table_is_the_spec_file.rs` now asserts the figure, so it
//! cannot expire again.)
//! Every other name those formulas reach is either a user-facing adjustment (seeded by the caller
//! from the shape's current values) or a built-in variable.
//!
//! # The geometry table
//!
//! [`emit_preset_geometry`] crosses that boundary: it takes each shape's **whole** `gdLst` in
//! declaration order — declaration order is evaluation order (§20.1.9.11) — and its whole
//! `pathLst`, each `a:path` with its `@w`/`@h` coordinate box, its `@fill`/`@stroke`/`@extrusionOk`
//! flags and its ordered drawing steps.
//!
//! **The file defines 186 shapes and `ST_ShapeType` declares 187.** `upArrow` has no geometry block
//! in ECMA-376's own file; every other value has exactly one, except `upDownArrow`, which has two
//! byte-identical blocks. Neither is papered over: the generator emits the missing one into
//! `PRESETS_WITHOUT_GEOMETRY` and **fails** if it meets a shape element that is not an
//! `ST_ShapeType` value, or a drawing step it cannot read.
//!
//! This is pure mechanical extraction — no naming beyond the `ST_ShapeType` variant each shape
//! element already maps to.

// This module emits source code, so explicit trailing newlines in `write!` are intentional.
#![allow(clippy::write_with_newline)]

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use anyhow::{bail, Context, Result};
use mjx_dml::geometry::GuideOperator;
use mjx_xml::{Event, Reader};

use crate::codegen::spec;

/// A domain bound: a literal value, or the name of a computed `gdLst` guide (resolved later).
#[derive(Debug, Clone)]
enum Bound {
    Literal(i32),
    Guide(String),
}

/// One extracted user-facing adjustment.
#[derive(Debug, Clone)]
struct Adjustment {
    wire_name: String,
    /// The Rust `AdjustmentAxis` variant name (`Horizontal` / `Vertical` / `Angle` / `Radius`).
    axis: &'static str,
    default: i32,
    min: Bound,
    max: Bound,
}

/// A shape and its ordered user-facing adjustments (empty for fixed-geometry shapes), plus the
/// `gdLst` guides its adjustment bounds are computed from, its whole `gdLst`, its `pathLst`, and
/// whatever the reader could not make sense of.
struct ShapeAdjustments {
    token: String,
    adjustments: Vec<Adjustment>,
    bound_guides: Vec<(String, String)>,
    /// The shape's whole `avLst`, in declaration order — every adjustable value with its `val N`
    /// seed, including the ones no adjust handle references and [`Adjustment`] therefore drops.
    adjustment_values: Vec<(String, String)>,
    /// The shape's whole `gdLst`, in declaration order.
    guides: Vec<(String, String)>,
    /// The shape's `pathLst`, in order.
    paths: Vec<PathBlock>,
    /// Everything about this shape the reader could not represent, in the order it was met.
    ///
    /// Never empty *and* ignored: [`emit_preset_geometry`] fails on the first shape that has any,
    /// naming the shape and every complaint, so a preset can never be silently half-extracted.
    unreadable: Vec<String>,
}

/// One `a:path`: its optional coordinate box, its three flags, and its ordered steps.
#[derive(Debug, Clone)]
struct PathBlock {
    width: Option<i64>,
    height: Option<i64>,
    /// The `@fill` wire token (`ST_PathFillMode`), defaulted to the schema's `norm`.
    fill: String,
    stroke: bool,
    extrusion_ok: bool,
    steps: Vec<Step>,
}

/// One drawing step, exactly as the file writes it — coordinates and angles left as strings,
/// because a literal and a guide name are the same attribute and the distinction is made on emit.
#[derive(Debug, Clone)]
enum Step {
    Close,
    MoveTo(Pt),
    LineTo(Pt),
    ArcTo {
        width_radius: String,
        height_radius: String,
        start_angle: String,
        swing_angle: String,
    },
    QuadBezierTo(Pt, Pt),
    CubicBezierTo(Pt, Pt, Pt),
}

/// An `a:pt`'s `@x` and `@y`, verbatim.
type Pt = (String, String);

/// A step whose `a:pt` children have not all arrived yet.
#[derive(Debug, Clone)]
struct PendingStep {
    /// The element's local name, which is also how many points it wants.
    element: &'static str,
    points: Vec<Pt>,
}

/// Renders the `adjustments_of` table source (appended after the `PresetShapeType` enum).
pub fn emit_shape_adjustments(xml: &[u8]) -> Result<String> {
    let shapes = parse(xml)?;
    let mut s = String::new();
    s.push_str(
        "/// The user-facing adjustments of a preset shape, in `avLst` declaration order.\n\
         ///\n\
         /// Extracted from `presetShapeDefinitions.xml`: each is an `avLst` guide referenced by an\n\
         /// adjust handle. Fixed-geometry shapes (and any shape not in the spec's geometry file, e.g.\n\
         /// `upArrow`) return an empty slice. Values are in native spec units (fractions in 1000ths of\n\
         /// a percent; angles in 60000ths of a degree).\n\
         #[must_use]\n\
         pub fn adjustments_of(shape: PresetShapeType) -> &'static [crate::drawingml::AdjustmentSpec] {\n\
         \x20   use crate::drawingml::AdjustmentAxis::{Angle, Horizontal, Radius, Vertical};\n\
         \x20   use crate::drawingml::AdjustmentBound::{Guide, Literal};\n\
         \x20   use crate::drawingml::AdjustmentSpec;\n\
         \x20   match shape {\n",
    );
    for shape in &shapes {
        if shape.adjustments.is_empty() {
            continue;
        }
        let variant = spec::ENGINE.variant_name("ST_ShapeType", &shape.token);
        let _ = write!(s, "        PresetShapeType::{variant} => &[\n");
        for adj in &shape.adjustments {
            let _ = write!(
                s,
                "            AdjustmentSpec {{ wire_name: {:?}, axis: {}, default: {}, min: {}, max: {} }},\n",
                adj.wire_name,
                adj.axis,
                adj.default,
                render_bound(&adj.min),
                render_bound(&adj.max),
            );
        }
        s.push_str("        ],\n");
    }
    s.push_str("        _ => &[],\n    }\n}\n");
    emit_adjustable_shapes(&mut s, &shapes);
    emit_bound_guides(&mut s, &shapes);
    Ok(s)
}

/// Renders `adjustable_shapes` — every shape `adjustments_of` answers for, in file order.
fn emit_adjustable_shapes(s: &mut String, shapes: &[ShapeAdjustments]) {
    s.push_str(
        "\n/// Every preset shape that exposes at least one user-facing adjustment, in\n\
         /// `presetShapeDefinitions.xml` order — exactly the shapes [`adjustments_of`] answers with a\n\
         /// non-empty slice.\n\
         #[must_use]\n\
         pub fn adjustable_shapes() -> &'static [PresetShapeType] {\n\
         \x20   &[\n",
    );
    for shape in shapes {
        if shape.adjustments.is_empty() {
            continue;
        }
        let variant = spec::ENGINE.variant_name("ST_ShapeType", &shape.token);
        let _ = write!(s, "        PresetShapeType::{variant},\n");
    }
    s.push_str("    ]\n}\n");
}

/// Renders `adjustment_bound_guides_of` — the `gdLst` closure behind each shape's named bounds.
fn emit_bound_guides(s: &mut String, shapes: &[ShapeAdjustments]) {
    s.push_str(
        "\n/// The `gdLst` guides a preset shape's adjustment **domain bounds** are computed from, in\n\
         /// declaration order.\n\
         ///\n\
         /// An [`AdjustmentSpec`](crate::drawingml::AdjustmentSpec) bound is often not a number but the\n\
         /// name of a computed guide (`maxAdj1`, `maxAng`, …) that depends on the shape's width and\n\
         /// height. These are those guides, and the ones they in turn depend on, extracted from\n\
         /// `presetShapeDefinitions.xml`. Evaluate them in order with the shape's current adjustment\n\
         /// values already bound, and every bound becomes a number.\n\
         ///\n\
         /// Empty for a shape whose bounds are all literals, and for a shape with no adjustments.\n\
         #[must_use]\n\
         pub fn adjustment_bound_guides_of(\n\
         \x20   shape: PresetShapeType,\n\
         ) -> &'static [crate::drawingml::PresetGuide] {\n\
         \x20   use crate::drawingml::PresetGuide;\n\
         \x20   match shape {\n",
    );
    for shape in shapes {
        if shape.bound_guides.is_empty() {
            continue;
        }
        let variant = spec::ENGINE.variant_name("ST_ShapeType", &shape.token);
        let _ = write!(s, "        PresetShapeType::{variant} => &[\n");
        for (name, formula) in &shape.bound_guides {
            let _ = write!(
                s,
                "            PresetGuide {{ wire_name: {name:?}, formula: {formula:?} }},\n"
            );
        }
        s.push_str("        ],\n");
    }
    s.push_str("        _ => &[],\n    }\n}\n");
}

fn render_bound(bound: &Bound) -> String {
    match bound {
        Bound::Literal(n) => format!("Literal({n})"),
        Bound::Guide(name) => format!("Guide({name:?})"),
    }
}

/// Parses each shape block, joining its `avLst` seeds to its `ahLst` handle references and
/// collecting the `gdLst` closure its named bounds depend on.
fn parse(xml: &[u8]) -> Result<Vec<ShapeAdjustments>> {
    let mut reader = Reader::new(xml);
    let mut out: Vec<ShapeAdjustments> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mut depth = 0usize;
    let mut token: Option<String> = None;
    let mut shape = ShapeBlock::default();

    loop {
        match reader
            .read()
            .context("reading presetShapeDefinitions.xml")?
        {
            Event::Start(e) => {
                depth += 1;
                if depth == 2 {
                    token = Some(e.local().to_owned());
                    shape = ShapeBlock::default();
                } else if depth >= 3 && token.is_some() {
                    shape.record(&e);
                }
            }
            Event::Empty(e) => {
                if depth >= 2 && token.is_some() {
                    shape.record(&e);
                }
            }
            Event::End(name) => {
                if depth == 2 {
                    if let Some(tok) = token.take() {
                        // `upDownArrow` is defined twice, byte-identical — keep only the first.
                        if seen.insert(tok.clone()) {
                            let adjustments = join(&shape.seeds, &shape.handles);
                            let bound_guides = shape.bound_guide_closure();
                            out.push(ShapeAdjustments {
                                token: tok,
                                adjustments,
                                bound_guides,
                                adjustment_values: std::mem::take(&mut shape.adjustment_values),
                                guides: std::mem::take(&mut shape.guides),
                                paths: std::mem::take(&mut shape.paths),
                                unreadable: std::mem::take(&mut shape.unreadable),
                            });
                        }
                    }
                } else if depth >= 3 && token.is_some() {
                    shape.close_element(&name.local);
                }
                depth = depth.saturating_sub(1);
            }
            Event::Text(_) => {}
            Event::Eof => break,
        }
    }
    Ok(out)
}

/// What one shape block accumulates while it is being read.
#[derive(Default)]
struct ShapeBlock {
    in_avlst: bool,
    in_gdlst: bool,
    in_ahlst: bool,
    in_pathlst: bool,
    /// `avLst` `val N` seeds, in declaration order.
    seeds: Vec<(String, i32)>,
    /// Every `avLst` guide, verbatim, in declaration order — seed or not.
    adjustment_values: Vec<(String, String)>,
    /// `gdLst` guides, in declaration order.
    guides: Vec<(String, String)>,
    /// Adjustment name → (axis, min, max) from the first handle that references it.
    handles: HashMap<String, (&'static str, Bound, Bound)>,
    /// The `a:path` currently open, if any.
    current_path: Option<PathBlock>,
    /// A multi-point step whose `a:pt` children are still arriving.
    pending: Option<PendingStep>,
    /// The paths closed so far, in order.
    paths: Vec<PathBlock>,
    /// What could not be represented, in the order it was met.
    unreadable: Vec<String>,
}

impl ShapeBlock {
    /// Handles one element inside a shape: section markers, `avLst` seeds, `gdLst` guides, and
    /// `ahLst` handle refs.
    fn record(&mut self, e: &mjx_xml::Element) {
        match e.local() {
            "avLst" => self.in_avlst = true,
            "gdLst" => self.in_gdlst = true,
            "ahLst" => self.in_ahlst = true,
            "gd" if self.in_avlst => {
                if let (Some(name), Some(fmla)) = (e.attr("name"), e.attr("fmla")) {
                    self.adjustment_values
                        .push((name.to_owned(), fmla.to_owned()));
                    if let Some(value) = parse_val(fmla) {
                        self.seeds.push((name.to_owned(), value));
                    }
                }
            }
            "gd" if self.in_gdlst => {
                if let (Some(name), Some(fmla)) = (e.attr("name"), e.attr("fmla")) {
                    self.guides.push((name.to_owned(), fmla.to_owned()));
                }
            }
            "ahXY" if self.in_ahlst => {
                record_axis(e, "gdRefX", "minX", "maxX", "Horizontal", &mut self.handles);
                record_axis(e, "gdRefY", "minY", "maxY", "Vertical", &mut self.handles);
            }
            "pathLst" => self.in_pathlst = true,
            "path" if self.in_pathlst => {
                self.current_path = Some(read_path(e));
            }
            "close" if self.current_path.is_some() => self.push_step(Step::Close),
            "arcTo" if self.current_path.is_some() => match read_arc(e) {
                Ok(step) => self.push_step(step),
                Err(complaint) => self.unreadable.push(complaint),
            },
            "moveTo" | "lnTo" | "quadBezTo" | "cubicBezTo" if self.current_path.is_some() => {
                self.begin_step(e.local());
            }
            "pt" if self.pending.is_some() => {
                let point = (
                    e.attr("x").unwrap_or_default().to_owned(),
                    e.attr("y").unwrap_or_default().to_owned(),
                );
                if let Some(pending) = self.pending.as_mut() {
                    pending.points.push(point);
                }
            }
            "ahPolar" if self.in_ahlst => {
                record_axis(
                    e,
                    "gdRefAng",
                    "minAng",
                    "maxAng",
                    "Angle",
                    &mut self.handles,
                );
                record_axis(e, "gdRefR", "minR", "maxR", "Radius", &mut self.handles);
            }
            _ => {}
        }
    }

    /// Handles the end tag of an element inside a shape: the section markers, and the multi-point
    /// drawing steps whose `a:pt` children have now all arrived.
    fn close_element(&mut self, local: &str) {
        match local {
            "avLst" => self.in_avlst = false,
            "gdLst" => self.in_gdlst = false,
            "ahLst" => self.in_ahlst = false,
            "pathLst" => self.in_pathlst = false,
            "path" => {
                self.finish_pending();
                if let Some(path) = self.current_path.take() {
                    self.paths.push(path);
                }
            }
            "moveTo" | "lnTo" | "quadBezTo" | "cubicBezTo" => self.finish_pending(),
            _ => {}
        }
    }

    /// Opens a multi-point step. A previous one still open is malformed and is finished (and
    /// complained about) rather than silently absorbing this step's points.
    fn begin_step(&mut self, local: &str) {
        self.finish_pending();
        let element = match local {
            "moveTo" => "moveTo",
            "lnTo" => "lnTo",
            "quadBezTo" => "quadBezTo",
            _ => "cubicBezTo",
        };
        self.pending = Some(PendingStep {
            element,
            points: Vec::new(),
        });
    }

    /// Turns the open multi-point step into a [`Step`], or complains that it had the wrong number
    /// of `a:pt` children.
    fn finish_pending(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        let wanted = match pending.element {
            "moveTo" | "lnTo" => 1,
            "quadBezTo" => 2,
            _ => 3,
        };
        if pending.points.len() != wanted {
            self.unreadable.push(format!(
                "a `{}` with {} `pt` children, not {wanted}",
                pending.element,
                pending.points.len()
            ));
            return;
        }
        let mut points = pending.points.into_iter();
        let mut next = || points.next().unwrap_or_default();
        let step = match pending.element {
            "moveTo" => Step::MoveTo(next()),
            "lnTo" => Step::LineTo(next()),
            "quadBezTo" => Step::QuadBezierTo(next(), next()),
            _ => Step::CubicBezierTo(next(), next(), next()),
        };
        self.push_step(step);
    }

    /// Appends a finished step to the open path, or complains that there is none.
    fn push_step(&mut self, step: Step) {
        match self.current_path.as_mut() {
            Some(path) => path.steps.push(step),
            None => self
                .unreadable
                .push(format!("a drawing step outside any `path`: {step:?}")),
        }
    }

    /// The `gdLst` guides the shape's guide-named bounds depend on, transitively, in declaration
    /// order. A name that is not a `gdLst` guide is left alone: it is either a user-facing adjustment
    /// (the caller seeds those) or a built-in variable.
    fn bound_guide_closure(&self) -> Vec<(String, String)> {
        let by_name: HashMap<&str, &str> = self
            .guides
            .iter()
            .map(|(name, formula)| (name.as_str(), formula.as_str()))
            .collect();

        let mut needed: HashSet<&str> = HashSet::new();
        let mut pending: Vec<&str> = Vec::new();
        for (_, min, max) in self.handles.values() {
            for bound in [min, max] {
                if let Bound::Guide(name) = bound {
                    pending.push(name.as_str());
                }
            }
        }
        while let Some(name) = pending.pop() {
            let Some(formula) = by_name.get(name) else {
                continue;
            };
            if !needed.insert(name) {
                continue;
            }
            pending.extend(formula.split_whitespace().skip(1));
        }

        self.guides
            .iter()
            .filter(|(name, _)| needed.contains(name.as_str()))
            .cloned()
            .collect()
    }
}

/// Reads an `a:path`'s attributes, applying `CT_Path2D`'s schema defaults for the unstated ones
/// (`fill` = `norm`, `stroke` = `extrusionOk` = `true`, no coordinate box).
fn read_path(e: &mjx_xml::Element) -> PathBlock {
    // `@w`/`@h` are `ST_PositiveCoordinate` with a schema default of 0, which means *no box* rather
    // than a box of no size, so 0 and absent are the same answer here.
    let box_side = |value: Option<&str>| -> Option<i64> {
        value
            .and_then(|text| text.parse::<i64>().ok())
            .filter(|side| *side > 0)
    };
    PathBlock {
        width: box_side(e.attr("w")),
        height: box_side(e.attr("h")),
        fill: e.attr("fill").unwrap_or("norm").to_owned(),
        stroke: on_off(e.attr("stroke"), true),
        extrusion_ok: on_off(e.attr("extrusionOk"), true),
        steps: Vec::new(),
    }
}

/// Reads an `a:arcTo`'s four schema-required attributes.
fn read_arc(e: &mjx_xml::Element) -> Result<Step, String> {
    let attribute = |name: &str| {
        e.attr(name)
            .map(str::to_owned)
            .ok_or_else(|| format!("an `arcTo` with no `@{name}`"))
    };
    Ok(Step::ArcTo {
        width_radius: attribute("wR")?,
        height_radius: attribute("hR")?,
        start_angle: attribute("stAng")?,
        swing_angle: attribute("swAng")?,
    })
}

/// An `ST_OnOff` attribute, in every spelling the schema permits, or `default` when unstated.
///
/// Normalised here rather than trusted: the geometry file writes `false`, but `0`, `f`, `off`, `1`,
/// `t` and `on` are all legal `ST_OnOff` and a reader that only knew one of them would silently
/// read the others as the default.
fn on_off(value: Option<&str>, default: bool) -> bool {
    match value {
        Some("false" | "0" | "f" | "off") => false,
        Some("true" | "1" | "t" | "on") => true,
        _ => default,
    }
}

fn record_axis(
    e: &mjx_xml::Element,
    gd_ref: &str,
    min_attr: &str,
    max_attr: &str,
    axis: &'static str,
    handles: &mut HashMap<String, (&'static str, Bound, Bound)>,
) {
    if let Some(adj) = e.attr(gd_ref) {
        let min = bound(e.attr(min_attr));
        let max = bound(e.attr(max_attr));
        // First handle referencing an adjustment wins (adjustments are referenced once in practice).
        handles.entry(adj.to_owned()).or_insert((axis, min, max));
    }
}

/// The integer of a `val N` seed formula, or `None` for any other formula.
fn parse_val(fmla: &str) -> Option<i32> {
    let mut parts = fmla.split_whitespace();
    if parts.next()? != "val" {
        return None;
    }
    parts.next()?.parse().ok()
}

fn bound(value: Option<&str>) -> Bound {
    match value {
        Some(s) => s
            .parse::<i32>()
            .map(Bound::Literal)
            .unwrap_or(Bound::Guide(s.to_owned())),
        None => Bound::Literal(0),
    }
}

/// A seed is a user-facing adjustment iff a handle references it; emit in `avLst` order.
fn join(
    seeds: &[(String, i32)],
    handles: &HashMap<String, (&'static str, Bound, Bound)>,
) -> Vec<Adjustment> {
    seeds
        .iter()
        .filter_map(|(name, default)| {
            handles.get(name).map(|(axis, min, max)| Adjustment {
                wire_name: name.clone(),
                axis,
                default: *default,
                min: min.clone(),
                max: max.clone(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The geometry table: `crates/mjx-geometry/src/generated.rs`
// ---------------------------------------------------------------------------------------------

/// The header of the emitted file — everything above the table itself.
const GENERATED_HEADER: &str = "\
// @generated by `cargo run -p xtask -- codegen` — do not edit.
//! Every preset shape ECMA-376 defines geometry for, mechanically extracted from
//! `presetShapeDefinitions.xml`.
//!
//! # What a row is, and what it is not
//!
//! One [`PresetShapeDefinition`] per shape element of the file, in the file's own order: the
//! shape's whole `a:gdLst` in declaration order — which is evaluation order, and therefore the
//! whole of the cycle defence (ECMA-376 Part 1 §20.1.9.11) — and its whole `a:pathLst`, each
//! `a:path` with its coordinate box, its three flags and its ordered steps.
//!
//! Nothing here is named, inferred or simplified. A guide's `name` and `fmla` are the file's own
//! strings; a coordinate is a literal where the file writes digits and a guide reference where it
//! writes a name; a step is the `a:path` child element it came from. The one interpretation applied
//! is `CT_Path2D`'s schema defaults for an unstated `@fill` (`norm`), `@stroke` / `@extrusionOk`
//! (`true`) and `@w` / `@h` (no coordinate box).
//!
//! # 186, and the one that is missing
//!
//! `ST_ShapeType` declares 187 values; this file defines geometry for 186 of them.
//! [`PRESETS_WITHOUT_GEOMETRY`] names the rest — and it is *not* a gap in this extraction, it is a
//! gap in ECMA-376's own geometry file, which has no `upArrow` element at all. `upDownArrow` is the
//! file's other oddity: it appears twice, byte-identically, and the second block is dropped.
//!
//! Regenerate with `cargo run -p xtask -- codegen`, which needs the local `References/` tree.

use mjx_ooxml_types::drawingml::{PathFillMode, PresetGuide, PresetShapeType};

use crate::table::{
    PresetAngle, PresetCoordinate, PresetPath, PresetPathStep, PresetPoint, PresetShapeDefinition,
};
use crate::Derivation;

";

/// One formula ECMA-376's own geometry file writes wrongly, and the reading that is not a guess.
///
/// See [`SPEC_ERRATA`].
struct Erratum {
    /// The shape element the guide is in.
    shape: &'static str,
    /// The guide's `@name`.
    guide: &'static str,
    /// What the file says, byte for byte. Checked, not assumed — see [`apply_errata`].
    written: &'static str,
    /// What is emitted instead.
    corrected: &'static str,
}

/// The formulas `presetShapeDefinitions.xml` gets wrong, and the corrections applied on the way out.
///
/// **Eight formulas in three shapes, and one defect.** `+-` takes three arguments (§20.1.9.11:
/// `"+- x y z" = ((x + y) - z)`); these eight give it four, with a trailing `0` after an expression
/// that is already complete. `mjx-dml`'s evaluator refuses a four-argument `+-`, correctly, so
/// without this table `circularArrow`, `leftCircularArrow` and `leftRightCircularArrow` cannot
/// evaluate a single guide and cannot draw at all.
///
/// **The correction is not an invention, and the file itself is the evidence.** Every one of the
/// eight has a *sibling* written a few guides earlier with three arguments and the same shape:
/// `xG = "+- xH dxG 0"` beside `xB = "+- xH 0 dxB 0"`, and `xK = "+- xI dxK 0"` beside
/// `xJ = "+- xI 0 dxJ 0"`, with the `y` of each pair matching. Dropping the excess fourth token
/// leaves `xH + 0 - dxB`, the mirror of `xG` about `xH` — which is what a pair of points either
/// side of an arrowhead's tip has to be. Nothing else is changed and nothing else is corrected: a
/// defect with no such reading would be left to fail loudly rather than guessed at.
///
/// **A second reader of the same file reached the same reading, independently.**
/// `crates/mjx-dml/tests/guide_formula.rs`'s
/// `every_guide_of_every_preset_shape_definition_evaluates` sweeps the whole addendum and pins
/// *exactly these eight*, by shape, guide and formula, applying the same correction — it drops the
/// last whitespace-separated token. That suite was written by MJXOFF-155's DrawingML work and this
/// table was written without reference to it, so the agreement is a check rather than a copy.
/// (**That sweep has never run in CI**: `MJX_REQUIRE_PRESET_GEOMETRY` is set by no workflow and
/// `mjx-dml` appears in no job that carries `References/`, so it prints a skip notice and passes.
/// Filed as MJXOFF-197, not this child's to fix — but it does mean this table's corroboration came
/// from reading that file rather than from watching it go green.)
///
/// Two gates hold this table honest, and they are the reason it is safe to correct anything at all.
/// [`apply_errata`] **fails** if a `written` text is not in the file any more, so a later edition
/// that fixes these cannot leave a silent rewrite behind. [`check_formula_arity`] then walks
/// *every* formula of *every* shape and fails on any that is still malformed — so this table cannot
/// quietly become incomplete either, which is exactly how the first draft of it missed
/// `leftRightCircularArrow`'s second pair.
const SPEC_ERRATA: &[Erratum] = &[
    Erratum {
        shape: "circularArrow",
        guide: "xB",
        written: "+- xH 0 dxB 0",
        corrected: "+- xH 0 dxB",
    },
    Erratum {
        shape: "circularArrow",
        guide: "yB",
        written: "+- yH 0 dyB 0",
        corrected: "+- yH 0 dyB",
    },
    Erratum {
        shape: "leftCircularArrow",
        guide: "xB",
        written: "+- xH 0 dxB 0",
        corrected: "+- xH 0 dxB",
    },
    Erratum {
        shape: "leftCircularArrow",
        guide: "yB",
        written: "+- yH 0 dyB 0",
        corrected: "+- yH 0 dyB",
    },
    Erratum {
        shape: "leftRightCircularArrow",
        guide: "xB",
        written: "+- xH 0 dxB 0",
        corrected: "+- xH 0 dxB",
    },
    Erratum {
        shape: "leftRightCircularArrow",
        guide: "yB",
        written: "+- yH 0 dyB 0",
        corrected: "+- yH 0 dyB",
    },
    Erratum {
        shape: "leftRightCircularArrow",
        guide: "xJ",
        written: "+- xI 0 dxJ 0",
        corrected: "+- xI 0 dxJ",
    },
    Erratum {
        shape: "leftRightCircularArrow",
        guide: "yJ",
        written: "+- yI 0 dyJ 0",
        corrected: "+- yI 0 dyJ",
    },
];

/// Applies [`SPEC_ERRATA`] to the parsed shapes, in place.
///
/// # Errors
///
/// Fails, naming every one, if a row addresses a guide that is there and says something else. A
/// shape or guide that is *absent* is skipped: a later edition may drop either, and the sample XML
/// the unit tests run on has neither. What stops the table from quietly becoming incomplete is
/// [`check_formula_arity`], which runs afterwards and fails on any malformed formula left standing.
fn apply_errata(shapes: &mut [ShapeAdjustments]) -> Result<()> {
    let mut unmatched: Vec<String> = Vec::new();
    for erratum in SPEC_ERRATA {
        // A shape or a guide that is simply *not there* is not a mismatch: a later edition may
        // have removed either, and the sample XML the unit tests run on has neither. What must not
        // pass is a guide that **is** there and does not say what the erratum claims to correct —
        // that is a row rewriting something it was not written for. Completeness in the other
        // direction is [`check_formula_arity`]'s job, which fails on any malformed formula still
        // standing after this runs.
        let Some(shape) = shapes.iter_mut().find(|shape| shape.token == erratum.shape) else {
            continue;
        };
        let Some(guide) = shape
            .guides
            .iter_mut()
            .find(|(name, _)| name == erratum.guide)
        else {
            continue;
        };
        if guide.1 == erratum.corrected {
            continue;
        }
        if guide.1 != erratum.written {
            unmatched.push(format!(
                "`{}`'s guide `{}` is {:?}, not the {:?} this erratum corrects",
                erratum.shape, erratum.guide, guide.1, erratum.written
            ));
            continue;
        }
        guide.1 = erratum.corrected.to_owned();
    }
    if !unmatched.is_empty() {
        bail!(
            "{} spec erratum/errata no longer match presetShapeDefinitions.xml:\n  {}",
            unmatched.len(),
            unmatched.join("\n  ")
        );
    }
    Ok(())
}

/// Fails on any guide formula whose operator or argument count `mjx-dml`'s evaluator would refuse.
///
/// Run **after** [`apply_errata`], over every `avLst` and `gdLst` formula of every shape. Its point
/// is not to validate ECMA-376 — it is to make [`SPEC_ERRATA`] provably complete: a malformed
/// formula that no erratum covers becomes a shape that resolves to nothing at run time, in a table
/// nobody reads, which is precisely the trap MJXOFF-201 §6 describes. Here it is a build failure
/// naming the shape, the guide and the formula.
///
/// The arity comes from [`GuideOperator::argument_count`] — the *same* function the resolver checks
/// against — rather than from a second table here, so the two cannot disagree.
///
/// # Errors
///
/// Names every offending formula.
fn check_formula_arity(shapes: &[ShapeAdjustments]) -> Result<()> {
    let mut malformed: Vec<String> = Vec::new();
    for shape in shapes {
        let lists = [
            ("avLst", &shape.adjustment_values),
            ("gdLst", &shape.guides),
        ];
        for (list, guides) in lists {
            for (name, formula) in guides {
                let mut tokens = formula.split_whitespace();
                let Some(operator) = tokens.next().and_then(GuideOperator::from_wire) else {
                    malformed.push(format!(
                        "`{}`'s {list} guide `{name}` = {formula:?} names no known operator",
                        shape.token
                    ));
                    continue;
                };
                let found = tokens.count();
                if found != operator.argument_count() {
                    malformed.push(format!(
                        "`{}`'s {list} guide `{name}` = {formula:?} gives `{}` {found} \
                         argument(s), not {}",
                        shape.token,
                        operator.to_wire(),
                        operator.argument_count()
                    ));
                }
            }
        }
    }
    if !malformed.is_empty() {
        bail!(
            "{} guide formula(s) of presetShapeDefinitions.xml are malformed and no `SPEC_ERRATA` \
             row corrects them — a shape carrying one cannot resolve at all:\n  {}",
            malformed.len(),
            malformed.join("\n  ")
        );
    }
    Ok(())
}

/// Renders `crates/mjx-geometry/src/generated.rs`: the whole `gdLst` and `pathLst` of every preset
/// shape `presetShapeDefinitions.xml` defines.
///
/// `shape_tokens` is every `ST_ShapeType` enumeration value, in schema order, as parsed out of
/// `dml-main.xsd` by the simple-type emitter. It is what makes the two absences below detectable:
/// a shape element the enumeration does not declare (which would name a Rust variant that does not
/// exist), and an enumeration value the geometry file does not define (which becomes
/// `PRESETS_WITHOUT_GEOMETRY`).
///
/// # Errors
///
/// Fails, naming every offender, when a shape element is not an `ST_ShapeType` value, when a shape
/// has no `a:pathLst` at all, or when any shape carried something the reader could not represent.
/// **A preset is never silently half-extracted or silently dropped.**
pub fn emit_preset_geometry(xml: &[u8], shape_tokens: &[String]) -> Result<String> {
    let mut shapes = parse(xml)?;
    apply_errata(&mut shapes)?;
    check_formula_arity(&shapes)?;
    let declared: HashSet<&str> = shape_tokens.iter().map(String::as_str).collect();

    let mut complaints: Vec<String> = Vec::new();
    for shape in &shapes {
        if !declared.contains(shape.token.as_str()) {
            complaints.push(format!(
                "`{}` is a shape element of presetShapeDefinitions.xml but not an `ST_ShapeType` \
                 value, so it names no `PresetShapeType` variant",
                shape.token
            ));
        }
        if shape.paths.is_empty() {
            complaints.push(format!("`{}` has no `a:pathLst`", shape.token));
        }
        for complaint in &shape.unreadable {
            complaints.push(format!("`{}`: {complaint}", shape.token));
        }
    }
    if !complaints.is_empty() {
        bail!(
            "the preset geometry extraction could not represent {} thing(s):\n  {}",
            complaints.len(),
            complaints.join("\n  ")
        );
    }

    let mut s = String::with_capacity(1 << 20);
    s.push_str(GENERATED_HEADER);
    let _ = write!(
        s,
        "/// Every preset shape `presetShapeDefinitions.xml` defines geometry for, in that file's\n\
         /// order — {} of the 187 `ST_ShapeType` values.\n\
         pub(crate) const GENERATED_SHAPES: &[PresetShapeDefinition] = &[\n",
        shapes.len()
    );
    for shape in &shapes {
        emit_shape_row(&mut s, shape);
    }
    s.push_str("];\n\n");
    emit_presets_without_geometry(&mut s, &shapes, shape_tokens);
    Ok(s)
}

/// One `PresetShapeDefinition` row.
fn emit_shape_row(s: &mut String, shape: &ShapeAdjustments) {
    let variant = spec::ENGINE.variant_name("ST_ShapeType", &shape.token);
    let _ = write!(
        s,
        "    PresetShapeDefinition {{\n        \
         preset: PresetShapeType::{variant},\n        \
         derivation: Derivation::ExtractedFromTheGeometryFile,\n        \
         source: \"presetShapeDefinitions.xml, shape element `{}` — ECMA-376 Part 1 §20.1.10.56 \
         names the preset and §20.1.9.11 the guide language its formulas are written in. \
         Mechanically extracted, in the file's own order, with no naming and no simplification.\",\n",
        shape.token
    );

    emit_guide_list(s, "adjustment_values", &shape.adjustment_values);
    emit_guide_list(s, "guides", &shape.guides);

    s.push_str("        paths: &[\n");
    for path in &shape.paths {
        emit_path(s, path);
    }
    s.push_str("        ],\n    },\n");
}

/// One `&'static [PresetGuide]` field of a row, in declaration order.
fn emit_guide_list(s: &mut String, field: &str, guides: &[(String, String)]) {
    if guides.is_empty() {
        let _ = write!(s, "        {field}: &[],\n");
        return;
    }
    let _ = write!(s, "        {field}: &[\n");
    for (name, formula) in guides {
        let _ = write!(
            s,
            "            PresetGuide {{ wire_name: {name:?}, formula: {formula:?} }},\n"
        );
    }
    s.push_str("        ],\n");
}

/// One `PresetPath`, with its coordinate box, its three flags and its steps.
fn emit_path(s: &mut String, path: &PathBlock) {
    let side = |value: Option<i64>| match value {
        Some(side) => format!("Some({side})"),
        None => "None".to_owned(),
    };
    let fill = spec::ENGINE.variant_name("ST_PathFillMode", &path.fill);
    let _ = write!(
        s,
        "            PresetPath {{\n                \
         width: {},\n                height: {},\n                \
         fill: PathFillMode::{fill},\n                stroke: {},\n                \
         extrusion_ok: {},\n                steps: &[\n",
        side(path.width),
        side(path.height),
        path.stroke,
        path.extrusion_ok,
    );
    for step in &path.steps {
        let _ = write!(s, "                    {},\n", render_step(step));
    }
    s.push_str("                ],\n            },\n");
}

/// One `PresetPathStep`, as a single expression.
fn render_step(step: &Step) -> String {
    match step {
        Step::Close => "PresetPathStep::Close".to_owned(),
        Step::MoveTo(point) => format!("PresetPathStep::MoveTo({})", render_point(point)),
        Step::LineTo(point) => format!("PresetPathStep::LineTo({})", render_point(point)),
        Step::ArcTo {
            width_radius,
            height_radius,
            start_angle,
            swing_angle,
        } => format!(
            "PresetPathStep::ArcTo {{ width_radius: {}, height_radius: {}, start_angle: {}, \
             swing_angle: {} }}",
            render_coordinate(width_radius),
            render_coordinate(height_radius),
            render_angle(start_angle),
            render_angle(swing_angle),
        ),
        Step::QuadBezierTo(control, end) => format!(
            "PresetPathStep::QuadBezierTo {{ control: {}, end: {} }}",
            render_point(control),
            render_point(end),
        ),
        Step::CubicBezierTo(first, second, end) => format!(
            "PresetPathStep::CubicBezierTo {{ first_control: {}, second_control: {}, end: {} }}",
            render_point(first),
            render_point(second),
            render_point(end),
        ),
    }
}

/// A point, in the shorter `PresetPoint::at` form when both coordinates are guide names — which is
/// about seven points in eight.
fn render_point(point: &Pt) -> String {
    let (x, y) = point;
    if is_literal(x) || is_literal(y) {
        format!(
            "PresetPoint::new({}, {})",
            render_coordinate(x),
            render_coordinate(y)
        )
    } else {
        format!("PresetPoint::at({x:?}, {y:?})")
    }
}

/// A coordinate: a whole number is a literal in the path's own units, anything else is a guide name.
fn render_coordinate(value: &str) -> String {
    if is_literal(value) {
        format!("PresetCoordinate::Emu({value})")
    } else {
        format!("PresetCoordinate::Guide({value:?})")
    }
}

/// An angle: a whole number is a literal in 60000ths of a degree, anything else is a guide name
/// (which includes the circle constants `cd2`, `cd4`, `3cd4`, resolved by `mjx-dml`'s evaluator).
fn render_angle(value: &str) -> String {
    if is_literal(value) {
        format!("PresetAngle::Native({value})")
    } else {
        format!("PresetAngle::Guide({value:?})")
    }
}

/// Whether an `ST_AdjCoordinate` / `ST_AdjAngle` attribute is the numeric arm of its union rather
/// than an `ST_GeomGuideName`.
///
/// A guide name is an `xsd:token` and the file's are all identifiers, so "starts with a digit or a
/// minus sign, and parses" is the whole of the distinction. It is applied rather than assumed: a
/// name that happened to parse as a number would silently become a literal, so the parse is what
/// decides and not the first character alone.
fn is_literal(value: &str) -> bool {
    value.parse::<i64>().is_ok()
}

/// Renders `PRESETS_WITHOUT_GEOMETRY` — the `ST_ShapeType` values the geometry file leaves out.
fn emit_presets_without_geometry(
    s: &mut String,
    shapes: &[ShapeAdjustments],
    shape_tokens: &[String],
) {
    let defined: HashSet<&str> = shapes.iter().map(|shape| shape.token.as_str()).collect();
    let missing: Vec<&String> = shape_tokens
        .iter()
        .filter(|token| !defined.contains(token.as_str()))
        .collect();

    s.push_str(
        "/// The preset shapes `ST_ShapeType` declares and `presetShapeDefinitions.xml` defines no\n\
         /// geometry for.\n\
         ///\n\
         /// **A gap in ECMA-376, not in this workspace.** The file has no element for these, so\n\
         /// there is nothing to extract and inventing a path would be exactly the guess the naming\n\
         /// rule forbids elsewhere. [`definition_of`](crate::definition_of) answers `None` for each,\n\
         /// which is the answer that reaches\n\
         /// [`UnknownShapePolicy`](crate::UnknownShapePolicy) — an error, or a counted stand-in.\n\
         ///\n\
         /// Emitted from the difference between the enumeration and the file rather than written\n\
         /// down, so a later edition of either moves this list without anybody remembering to.\n\
         pub const PRESETS_WITHOUT_GEOMETRY: &[PresetShapeType] = &[\n",
    );
    for token in missing {
        let variant = spec::ENGINE.variant_name("ST_ShapeType", token);
        let _ = write!(s, "    PresetShapeType::{variant},\n");
    }
    s.push_str("];\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0"?>
        <presetShapeDefinitons>
          <roundRect>
            <avLst xmlns="urn:a"><gd name="adj" fmla="val 16667"/></avLst>
            <gdLst xmlns="urn:a"><gd name="x1" fmla="*/ ss a 100000"/></gdLst>
            <ahLst xmlns="urn:a"><ahXY gdRefX="adj" minX="0" maxX="50000"><pos x="x1" y="t"/></ahXY></ahLst>
            <pathLst xmlns="urn:a">
              <path w="21600" h="21600" fill="none" stroke="false" extrusionOk="false">
                <moveTo><pt x="l" y="x1"/></moveTo>
                <arcTo wR="x1" hR="x1" stAng="cd2" swAng="cd4"/>
                <lnTo><pt x="0" y="t"/></lnTo>
                <quadBezTo><pt x="r" y="t"/><pt x="r" y="x1"/></quadBezTo>
                <cubicBezTo><pt x="r" y="b"/><pt x="l" y="b"/><pt x="l" y="x1"/></cubicBezTo>
                <close/>
                <lnTo><pt x="l" y="t"/></lnTo>
              </path>
              <path><close/></path>
            </pathLst>
          </roundRect>
          <chevron>
            <avLst xmlns="urn:a"><gd name="adj" fmla="val 50000"/></avLst>
            <gdLst xmlns="urn:a"><gd name="maxAdj" fmla="*/ 100000 w ss"/></gdLst>
            <ahLst xmlns="urn:a"><ahXY gdRefX="adj" minX="0" maxX="maxAdj"><pos x="x2" y="t"/></ahXY></ahLst>
            <pathLst xmlns="urn:a"><path><close/></path></pathLst>
          </chevron>
          <pentagon>
            <avLst xmlns="urn:a"><gd name="hf" fmla="val 105146"/><gd name="vf" fmla="val 110557"/></avLst>
            <gdLst xmlns="urn:a"><gd name="swd2" fmla="*/ wd2 hf 100000"/></gdLst>
            <pathLst xmlns="urn:a"><path><close/></path></pathLst>
          </pentagon>
          <bentArrow>
            <avLst xmlns="urn:a"><gd name="adj1" fmla="val 25000"/></avLst>
            <gdLst xmlns="urn:a">
              <gd name="a1" fmla="pin 0 adj1 50000"/>
              <gd name="th" fmla="*/ ss a1 100000"/>
              <gd name="maxAdj1" fmla="*/ 100000 th ss"/>
              <gd name="unrelated" fmla="+- w 0 th"/>
            </gdLst>
            <ahLst xmlns="urn:a"><ahXY gdRefX="adj1" minX="0" maxX="maxAdj1"><pos x="th" y="t"/></ahXY></ahLst>
            <pathLst xmlns="urn:a"><path><close/></path></pathLst>
          </bentArrow>
        </presetShapeDefinitons>"#;

    #[test]
    fn a_paths_steps_flags_and_coordinate_box_all_survive_the_read() {
        let shapes = parse(SAMPLE).unwrap();
        let round_rect = &shapes[0];
        assert_eq!(round_rect.paths.len(), 2);

        let first = &round_rect.paths[0];
        assert_eq!((first.width, first.height), (Some(21600), Some(21600)));
        assert_eq!(first.fill, "none");
        assert!(!first.stroke);
        assert!(!first.extrusion_ok);
        assert_eq!(first.steps.len(), 7, "every step kind, in order");
        assert!(
            matches!(&first.steps[0], Step::MoveTo(point) if point == &("l".to_owned(), "x1".to_owned()))
        );
        assert!(matches!(
            &first.steps[1],
            Step::ArcTo { width_radius, height_radius, start_angle, swing_angle }
                if width_radius == "x1" && height_radius == "x1" && start_angle == "cd2" && swing_angle == "cd4"
        ));
        assert!(matches!(&first.steps[2], Step::LineTo(point) if point.0 == "0"));
        assert!(
            matches!(&first.steps[3], Step::QuadBezierTo(control, end) if control.0 == "r" && end.1 == "x1")
        );
        assert!(
            matches!(&first.steps[4], Step::CubicBezierTo(a, b, c) if a.1 == "b" && b.0 == "l" && c.1 == "x1")
        );
        assert!(matches!(first.steps[5], Step::Close));
        // A drawing element straight after a `close`, which ECMA-376 permits and six real shapes use.
        assert!(
            matches!(&first.steps[6], Step::LineTo(point) if point == &("l".to_owned(), "t".to_owned()))
        );

        // The second path states nothing, so it takes `CT_Path2D`'s schema defaults.
        let second = &round_rect.paths[1];
        assert_eq!((second.width, second.height), (None, None));
        assert_eq!(second.fill, "norm");
        assert!(second.stroke);
        assert!(second.extrusion_ok);

        assert!(
            round_rect.unreadable.is_empty(),
            "{:?}",
            round_rect.unreadable
        );
    }

    #[test]
    fn the_whole_avlst_is_carried_and_not_only_the_handled_part() {
        let shapes = parse(SAMPLE).unwrap();
        // `pentagon`'s `hf`/`vf` have no adjust handle, so `adjustments_of` drops them — and its
        // own `gdLst` reads them, so the geometry table must not.
        let pentagon = &shapes[2];
        assert_eq!(pentagon.token, "pentagon");
        assert!(pentagon.adjustments.is_empty());
        assert_eq!(
            pentagon.adjustment_values,
            vec![
                ("hf".to_owned(), "val 105146".to_owned()),
                ("vf".to_owned(), "val 110557".to_owned()),
            ]
        );
    }

    #[test]
    fn every_step_renders_as_the_expression_the_table_holds() {
        assert_eq!(render_step(&Step::Close), "PresetPathStep::Close");
        assert_eq!(
            render_step(&Step::MoveTo(("l".to_owned(), "t".to_owned()))),
            r#"PresetPathStep::MoveTo(PresetPoint::at("l", "t"))"#
        );
        // A literal coordinate takes the other arm of `render_point`, and a mixed pair takes it too.
        assert_eq!(
            render_step(&Step::LineTo(("0".to_owned(), "t".to_owned()))),
            r#"PresetPathStep::LineTo(PresetPoint::new(PresetCoordinate::Emu(0), PresetCoordinate::Guide("t")))"#
        );
        assert_eq!(
            render_angle("cd2"),
            r#"PresetAngle::Guide("cd2")"#,
            "an angular built-in is a name, not a number"
        );
        assert_eq!(render_angle("-5400000"), "PresetAngle::Native(-5400000)");
        assert!(is_literal("-2147483647"));
        assert!(
            !is_literal("3cd4"),
            "a circle constant starts with a digit and is not a number"
        );
    }

    #[test]
    fn an_on_off_attribute_is_read_in_every_spelling_the_schema_permits() {
        for text in ["false", "0", "f", "off"] {
            assert!(!on_off(Some(text), true), "{text}");
        }
        for text in ["true", "1", "t", "on"] {
            assert!(on_off(Some(text), false), "{text}");
        }
        assert!(on_off(None, true), "an absent attribute takes the default");
        assert!(!on_off(None, false));
        assert!(
            on_off(Some("maybe"), true),
            "an unparseable value takes the default rather than inventing one"
        );
    }

    #[test]
    fn a_step_the_reader_cannot_represent_is_named_rather_than_dropped() {
        const A_MOVE_WITH_NO_POINT: &[u8] = br#"<?xml version="1.0"?>
            <presetShapeDefinitons>
              <rect>
                <pathLst xmlns="urn:a"><path><moveTo/><close/></path></pathLst>
              </rect>
            </presetShapeDefinitons>"#;
        let shapes = parse(A_MOVE_WITH_NO_POINT).unwrap();
        assert_eq!(
            shapes[0].unreadable,
            vec!["a `moveTo` with 0 `pt` children, not 1"]
        );

        let failure = emit_preset_geometry(A_MOVE_WITH_NO_POINT, &["rect".to_owned()])
            .expect_err("a shape that could not be read must fail the generator");
        let text = format!("{failure}");
        assert!(text.contains("rect"), "{text}");
        assert!(text.contains("moveTo"), "{text}");
    }

    #[test]
    fn a_shape_element_the_enumeration_does_not_declare_is_named_rather_than_dropped() {
        let failure = emit_preset_geometry(SAMPLE, &["roundRect".to_owned()])
            .expect_err("a shape that names no `PresetShapeType` variant must fail the generator");
        let text = format!("{failure}");
        for token in ["chevron", "pentagon", "bentArrow"] {
            assert!(text.contains(token), "{token} is not named: {text}");
        }
    }

    #[test]
    fn a_preset_the_file_leaves_out_becomes_a_named_row_and_not_a_silence() {
        let tokens: Vec<String> = ["roundRect", "chevron", "pentagon", "bentArrow", "upArrow"]
            .iter()
            .map(|token| (*token).to_owned())
            .collect();
        let source = emit_preset_geometry(SAMPLE, &tokens).unwrap();
        assert!(source.contains(
            "pub const PRESETS_WITHOUT_GEOMETRY: &[PresetShapeType] = &[\n    PresetShapeType::UpArrow,\n];"
        ));
        // And the four the sample does define are rows.
        for variant in ["RoundedRectangle", "Chevron", "Pentagon", "BentArrow"] {
            assert!(
                source.contains(&format!("preset: PresetShapeType::{variant},")),
                "{variant} is not a row"
            );
        }
        assert!(source.contains("fill: PathFillMode::None,"));
        assert!(source.contains("width: Some(21600),"));
        assert!(source.contains("extrusion_ok: false,"));
    }

    #[test]
    fn an_erratum_that_no_longer_matches_the_file_fails_rather_than_rewriting_silently() {
        // The table corrects eight formulas of `presetShapeDefinitions.xml` by name. A row that
        // addresses a guide saying something *else* must fail rather than rewrite it: an erratum is
        // a claim about a specific text, and a claim that has stopped being true is a silent
        // rewrite waiting to happen.
        const A_DIFFERENT_XB: &[u8] = br#"<?xml version="1.0"?>
            <presetShapeDefinitons>
              <circularArrow>
                <gdLst xmlns="urn:a"><gd name="xB" fmla="+- xH 0 dxB dxC"/></gdLst>
                <pathLst xmlns="urn:a"><path><close/></path></pathLst>
              </circularArrow>
            </presetShapeDefinitons>"#;
        let mut shapes = parse(A_DIFFERENT_XB).unwrap();
        let failure = apply_errata(&mut shapes).expect_err("the guide says something else");
        let text = format!("{failure}");
        assert!(text.contains("circularArrow"), "{text}");
        assert!(text.contains("xB"), "{text}");
        assert!(
            text.contains("dxC"),
            "the failure does not quote what it found: {text}"
        );

        // A guide the file has already corrected, and one it does not have at all, are both skipped
        // rather than failed — the arity gate is what proves the table complete, not this.
        const ALREADY_CORRECT: &[u8] = br#"<?xml version="1.0"?>
            <presetShapeDefinitons>
              <circularArrow>
                <gdLst xmlns="urn:a"><gd name="xB" fmla="+- xH 0 dxB"/></gdLst>
                <pathLst xmlns="urn:a"><path><close/></path></pathLst>
              </circularArrow>
            </presetShapeDefinitons>"#;
        let mut shapes = parse(ALREADY_CORRECT).unwrap();
        apply_errata(&mut shapes).expect("an already-corrected guide is not a mismatch");
        let mut none_of_them = parse(SAMPLE).unwrap();
        apply_errata(&mut none_of_them).expect("a file without those shapes is not a mismatch");
    }

    #[test]
    fn a_malformed_formula_no_erratum_covers_fails_the_generator() {
        // The gate that makes `SPEC_ERRATA` provably complete rather than merely present. Its first
        // draft covered six of the file's eight four-argument `+-`s; this is what caught the other
        // two.
        const A_FOUR_ARGUMENT_ADD: &[u8] = br#"<?xml version="1.0"?>
            <presetShapeDefinitons>
              <rect>
                <gdLst xmlns="urn:a"><gd name="x1" fmla="+- w 0 h 0"/></gdLst>
                <pathLst xmlns="urn:a"><path><close/></path></pathLst>
              </rect>
            </presetShapeDefinitons>"#;
        let shapes = parse(A_FOUR_ARGUMENT_ADD).unwrap();
        let failure = check_formula_arity(&shapes).expect_err("a four-argument `+-` is malformed");
        let text = format!("{failure}");
        assert!(text.contains("rect"), "{text}");
        assert!(text.contains("+-"), "{text}");
        assert!(text.contains("`x1`"), "{text}");

        // And a formula naming no known operator, the other arm.
        const AN_UNKNOWN_OPERATOR: &[u8] = br#"<?xml version="1.0"?>
            <presetShapeDefinitons>
              <rect>
                <gdLst xmlns="urn:a"><gd name="x1" fmla="frobnicate w"/></gdLst>
                <pathLst xmlns="urn:a"><path><close/></path></pathLst>
              </rect>
            </presetShapeDefinitons>"#;
        let shapes = parse(AN_UNKNOWN_OPERATOR).unwrap();
        let text = format!(
            "{}",
            check_formula_arity(&shapes).expect_err("`frobnicate` is not an operator")
        );
        assert!(text.contains("names no known operator"), "{text}");
    }

    // ---------------------------------------------------------------------------------------
    // Against the real file, when it is here
    // ---------------------------------------------------------------------------------------

    /// The geometry file, or `None` when `References/` is not in this checkout.
    ///
    /// Absence is a skip, the way the ECMA-376 schema gate treats it — the tree is licensed
    /// material, git-ignored, and supplied locally. `MJX_REQUIRE_SCHEMA=1` turns absence into a
    /// failure, which is what CI sets.
    fn the_geometry_file() -> Option<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .join(crate::codegen::GEOMETRIES_XML);
        match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                assert!(
                    std::env::var_os("MJX_REQUIRE_SCHEMA").is_none(),
                    "MJX_REQUIRE_SCHEMA is set and {} could not be read: {error}",
                    path.display()
                );
                eprintln!("skipping: {} is not in this checkout", path.display());
                None
            }
        }
    }

    /// Every `ST_ShapeType` value, read out of the schema the same way `run` reads it.
    fn shape_type_values() -> Option<Vec<String>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .join(crate::codegen::TRANSITIONAL_DIR)
            .join("dml-main.xsd");
        let xsd = std::fs::read(&path).ok()?;
        let types = crate::codegen::xsd::parse_simple_types(&xsd).ok()?;
        let shape_type = types
            .iter()
            .find(|candidate| candidate.name == "ST_ShapeType")?;
        match &shape_type.kind {
            crate::codegen::xsd::SimpleKind::Enumeration { values, .. } => Some(values.clone()),
            _ => None,
        }
    }

    #[test]
    fn the_committed_geometry_table_is_exactly_what_the_file_produces() {
        // Derivation *and* idempotence, without writing anything: the generator is a pure function
        // of the file, so running it twice must give the same bytes, and those bytes must be what
        // is committed. **Nothing here writes into the repository** — see `xtask/tests/tokens.rs`
        // for the race that rule exists to prevent.
        let Some(xml) = the_geometry_file() else {
            return;
        };
        let Some(tokens) = shape_type_values() else {
            eprintln!("skipping: dml-main.xsd is not in this checkout");
            return;
        };
        assert_eq!(
            tokens.len(),
            187,
            "`ST_ShapeType` declares {} values, and every count in this workspace says 187",
            tokens.len()
        );

        let once = emit_preset_geometry(&xml, &tokens).expect("the geometry table");
        let twice = emit_preset_geometry(&xml, &tokens).expect("the geometry table, again");
        assert_eq!(once, twice, "the generator is not deterministic");

        let formatted = crate::codegen::rustfmt(&once).expect("rustfmt");
        let committed = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask has a parent directory")
                .join(crate::codegen::PRESET_GEOMETRY_RS),
        )
        .expect("the committed table");
        if formatted != committed {
            let line = formatted
                .lines()
                .zip(committed.lines())
                .position(|(fresh, old)| fresh != old)
                .unwrap_or_else(|| committed.lines().count().min(formatted.lines().count()));
            panic!(
                "`{}` is not what the generator produces; the first difference is at line {}:\n                   generated: {:?}\n  committed: {:?}\nRun `cargo run -p xtask -- codegen`.",
                crate::codegen::PRESET_GEOMETRY_RS,
                line + 1,
                formatted.lines().nth(line),
                committed.lines().nth(line),
            );
        }
    }

    #[test]
    fn the_extraction_reconciles_with_the_file_s_own_element_counts() {
        // A second reading of the same bytes: the elements are counted by scanning for their start
        // tags, which is not how the extractor reads them, and the two must agree. `upDownArrow` is
        // written twice byte-identically and only the first block is kept, so the difference
        // between the two counts is exactly that block — which is itself asserted rather than
        // assumed.
        let Some(xml) = the_geometry_file() else {
            return;
        };
        let text = String::from_utf8(xml.clone()).expect("the file is UTF-8");
        let occurrences = |tag: &str| text.matches(tag).count();

        let shapes = parse(&xml).expect("the geometry file");
        assert_eq!(shapes.len(), 186, "the file defines 186 distinct shapes");

        let duplicate: Vec<&ShapeAdjustments> = shapes
            .iter()
            .filter(|shape| shape.token == "upDownArrow")
            .collect();
        assert_eq!(
            duplicate.len(),
            1,
            "the duplicate block was not de-duplicated"
        );
        let repeated = duplicate[0];
        assert_eq!(occurrences("<upDownArrow>"), 2, "the file writes it twice");

        let guides: usize = shapes.iter().map(|shape| shape.guides.len()).sum();
        let values: usize = shapes
            .iter()
            .map(|shape| shape.adjustment_values.len())
            .sum();
        assert_eq!(
            guides + values + repeated.guides.len() + repeated.adjustment_values.len(),
            occurrences("<gd "),
            "the guides read plus the duplicated block's are not the file's `a:gd` elements"
        );
        assert_eq!(
            occurrences("<gd "),
            3_923,
            "the file's `a:gd` total has moved"
        );

        let paths: usize = shapes.iter().map(|shape| shape.paths.len()).sum();
        // `<path` also matches `<pathLst`, one per shape element including the duplicated block.
        let path_elements = occurrences("<path") - occurrences("<pathLst");
        assert_eq!(paths + repeated.paths.len(), path_elements);
        assert_eq!(
            occurrences("<pathLst"),
            187,
            "one `a:pathLst` per shape element"
        );

        for (tag, kind) in [
            ("<moveTo", 0usize),
            ("<lnTo", 1),
            ("<arcTo", 2),
            ("<quadBezTo", 3),
            ("<cubicBezTo", 4),
            ("<close", 5),
        ] {
            let count = |shape: &ShapeAdjustments| {
                shape
                    .paths
                    .iter()
                    .flat_map(|path| &path.steps)
                    .filter(|step| {
                        matches!(
                            (kind, step),
                            (0, Step::MoveTo(_))
                                | (1, Step::LineTo(_))
                                | (2, Step::ArcTo { .. })
                                | (3, Step::QuadBezierTo(..))
                                | (4, Step::CubicBezierTo(..))
                                | (5, Step::Close)
                        )
                    })
                    .count()
            };
            let read: usize = shapes.iter().map(count).sum::<usize>() + count(repeated);
            assert_eq!(
                read,
                occurrences(tag),
                "the `{tag}` elements do not reconcile"
            );
        }
    }

    #[test]
    fn a_coordinate_box_never_holds_a_length_guide() {
        // The claim `PresetPath::width` makes, checked against the file rather than assumed: a path
        // with its own `@w`/`@h` writes its coordinates as proportions of that box, so a *length*
        // guide — computed in the shape's EMU — would be meaningless there. Angular ones are not:
        // an angle means the same in either space, and thirteen of them do appear.
        let Some(xml) = the_geometry_file() else {
            return;
        };
        let shapes = parse(&xml).expect("the geometry file");
        let mut angular = 0usize;
        for shape in &shapes {
            for path in shape.paths.iter().filter(|path| path.width.is_some()) {
                let mut names: Vec<&str> = Vec::new();
                for step in &path.steps {
                    match step {
                        Step::MoveTo(point) | Step::LineTo(point) => {
                            names.extend([point.0.as_str(), point.1.as_str()]);
                        }
                        Step::QuadBezierTo(a, b) => {
                            names.extend([a.0.as_str(), a.1.as_str(), b.0.as_str(), b.1.as_str()]);
                        }
                        Step::CubicBezierTo(a, b, c) => names.extend([
                            a.0.as_str(),
                            a.1.as_str(),
                            b.0.as_str(),
                            b.1.as_str(),
                            c.0.as_str(),
                            c.1.as_str(),
                        ]),
                        Step::ArcTo {
                            width_radius,
                            height_radius,
                            start_angle,
                            swing_angle,
                        } => {
                            names.extend([width_radius.as_str(), height_radius.as_str()]);
                            for angle in [start_angle, swing_angle] {
                                if !is_literal(angle) {
                                    assert!(
                                        angle.contains("cd"),
                                        "`{}` writes the angle `{angle}` inside a coordinate box",
                                        shape.token
                                    );
                                    angular += 1;
                                }
                            }
                        }
                        Step::Close => {}
                    }
                }
                for name in names {
                    assert!(
                        is_literal(name),
                        "`{}` writes the length guide `{name}` inside a coordinate box of {:?}",
                        shape.token,
                        path.width
                    );
                }
            }
        }
        assert!(
            angular >= 13,
            "only {angular} angular built-ins were found inside a coordinate box, so the case the \
             assertion above allows is not actually exercised"
        );
    }

    #[test]
    fn extracts_literal_and_guide_bounds() {
        let shapes = parse(SAMPLE).unwrap();
        assert_eq!(shapes.len(), 4);

        let rr = &shapes[0];
        assert_eq!(rr.token, "roundRect");
        assert_eq!(rr.adjustments.len(), 1);
        assert_eq!(rr.adjustments[0].wire_name, "adj");
        assert_eq!(rr.adjustments[0].axis, "Horizontal");
        assert_eq!(rr.adjustments[0].default, 16667);
        assert!(matches!(rr.adjustments[0].min, Bound::Literal(0)));
        assert!(matches!(rr.adjustments[0].max, Bound::Literal(50000)));

        // chevron's max is a computed guide, not a literal.
        assert!(matches!(&shapes[1].adjustments[0].max, Bound::Guide(g) if g == "maxAdj"));

        // pentagon: avLst present but no handle → zero user-facing adjustments.
        assert_eq!(shapes[2].token, "pentagon");
        assert!(shapes[2].adjustments.is_empty());
    }

    #[test]
    fn bound_guides_are_the_transitive_closure_in_declaration_order() {
        let shapes = parse(SAMPLE).unwrap();

        // roundRect's only bound is a literal, so nothing is needed to resolve it.
        assert!(shapes[0].bound_guides.is_empty());

        // chevron's `maxAdj` depends only on built-ins — one guide, itself.
        assert_eq!(
            shapes[1].bound_guides,
            vec![("maxAdj".to_owned(), "*/ 100000 w ss".to_owned())]
        );

        // bentArrow's `maxAdj1` walks back through `th` to `a1`, and the closure is emitted in
        // declaration order (a1, th, maxAdj1) so each guide sees the ones it needs. `unrelated`
        // is a gdLst guide no bound depends on, and is left out.
        let bent = &shapes[3];
        assert_eq!(bent.token, "bentArrow");
        let names: Vec<&str> = bent
            .bound_guides
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(names, ["a1", "th", "maxAdj1"]);
        assert_eq!(bent.bound_guides[0].1, "pin 0 adj1 50000");
    }

    #[test]
    fn emits_match_arms_only_for_shapes_with_adjustments() {
        let src = emit_shape_adjustments(SAMPLE).unwrap();
        assert!(src.contains("PresetShapeType::RoundedRectangle => &["));
        assert!(src.contains(r#"wire_name: "adj", axis: Horizontal, default: 16667"#));
        assert!(src.contains("max: Literal(50000)"));
        assert!(src.contains(r#"max: Guide("maxAdj")"#));
        // pentagon has no adjustments → no arm; caught by the `_ => &[]` wildcard.
        assert!(!src.contains("Pentagon"));
        assert!(src.contains("_ => &[],"));
    }

    #[test]
    fn emits_the_shape_list_and_the_bound_guide_table() {
        let src = emit_shape_adjustments(SAMPLE).unwrap();

        assert!(src.contains("pub fn adjustable_shapes() -> &'static [PresetShapeType] {"));
        assert!(src.contains("        PresetShapeType::RoundedRectangle,\n"));
        assert!(src.contains("        PresetShapeType::BentArrow,\n"));

        assert!(src.contains("pub fn adjustment_bound_guides_of("));
        assert!(src.contains(r#"PresetGuide { wire_name: "maxAdj", formula: "*/ 100000 w ss" }"#));
        assert!(src.contains(r#"PresetGuide { wire_name: "a1", formula: "pin 0 adj1 50000" }"#));
        // A shape whose bounds are all literals gets no arm.
        assert!(!src.contains("PresetShapeType::RoundedRectangle => &[\n            PresetGuide"));
        // A gdLst guide no bound depends on is not carried.
        assert!(!src.contains("unrelated"));
    }
}
