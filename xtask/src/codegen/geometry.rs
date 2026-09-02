//! Extracts per-shape **adjustment metadata** from the ECMA-376 `presetShapeDefinitions.xml` and
//! renders the committed `adjustments_of`, `adjustable_shapes` and `adjustment_bound_guides_of`
//! tables for `mjx-ooxml-types::drawingml`.
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
//! geometry, not all of it — 335 of the file's 3923 guides — because resolving an adjustment's domain
//! is what needs them, and the drawing paths are a rendering concern. Every other name those formulas
//! reach is either a user-facing adjustment (seeded by the caller from the shape's current values) or
//! a built-in variable.
//!
//! This is pure mechanical extraction — no naming.

// This module emits source code, so explicit trailing newlines in `write!` are intentional.
#![allow(clippy::write_with_newline)]

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use anyhow::{Context, Result};
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
/// `gdLst` guides its adjustment bounds are computed from.
struct ShapeAdjustments {
    token: String,
    adjustments: Vec<Adjustment>,
    bound_guides: Vec<(String, String)>,
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
                            });
                        }
                    }
                } else if name.local == "avLst" {
                    shape.in_avlst = false;
                } else if name.local == "gdLst" {
                    shape.in_gdlst = false;
                } else if name.local == "ahLst" {
                    shape.in_ahlst = false;
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
    /// `avLst` `val N` seeds, in declaration order.
    seeds: Vec<(String, i32)>,
    /// `gdLst` guides, in declaration order.
    guides: Vec<(String, String)>,
    /// Adjustment name → (axis, min, max) from the first handle that references it.
    handles: HashMap<String, (&'static str, Bound, Bound)>,
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0"?>
        <presetShapeDefinitons>
          <roundRect>
            <avLst xmlns="urn:a"><gd name="adj" fmla="val 16667"/></avLst>
            <gdLst xmlns="urn:a"><gd name="x1" fmla="*/ ss a 100000"/></gdLst>
            <ahLst xmlns="urn:a"><ahXY gdRefX="adj" minX="0" maxX="50000"><pos x="x1" y="t"/></ahXY></ahLst>
            <pathLst xmlns="urn:a"><path><close/></path></pathLst>
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
