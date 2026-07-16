//! Extracts per-shape **adjustment metadata** from the ECMA-376 `presetShapeDefinitions.xml` and
//! renders the committed `adjustments_of` table for `mjx-ooxml-types::drawingml`.
//!
//! A preset shape's user-facing adjustments are exactly the `avLst` guides that some `ahLst` handle
//! references via `gdRef{X,Y,Ang,R}`; the referencing attribute discloses the axis (X = horizontal,
//! Y = vertical, Ang = angle, R = radius). The default is the guide's `val N` seed; the domain is the
//! handle's `min*`/`max*` — a literal, or the name of a computed `gdLst` guide (data-dependent,
//! resolved only by the deferred evaluator). `avLst` entries with **no** handle (e.g. `star5.hf/vf`,
//! all of `pentagon`) are constants and are dropped. This is pure mechanical extraction — no naming.

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

/// A shape and its ordered user-facing adjustments (empty for fixed-geometry shapes).
struct ShapeAdjustments {
    token: String,
    adjustments: Vec<Adjustment>,
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
    Ok(s)
}

fn render_bound(bound: &Bound) -> String {
    match bound {
        Bound::Literal(n) => format!("Literal({n})"),
        Bound::Guide(name) => format!("Guide({name:?})"),
    }
}

/// Parses each shape block, joining its `avLst` seeds to its `ahLst` handle references.
fn parse(xml: &[u8]) -> Result<Vec<ShapeAdjustments>> {
    let mut reader = Reader::new(xml);
    let mut out: Vec<ShapeAdjustments> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mut depth = 0usize;
    let mut token: Option<String> = None;
    let mut seeds: Vec<(String, i32)> = Vec::new();
    let mut handles: HashMap<String, (&'static str, Bound, Bound)> = HashMap::new();
    let mut in_avlst = false;
    let mut in_ahlst = false;

    loop {
        match reader
            .read()
            .context("reading presetShapeDefinitions.xml")?
        {
            Event::Start(e) => {
                depth += 1;
                if depth == 2 {
                    token = Some(e.local().to_owned());
                    seeds.clear();
                    handles.clear();
                    in_avlst = false;
                    in_ahlst = false;
                } else if depth >= 3 && token.is_some() {
                    record(&e, &mut in_avlst, &mut in_ahlst, &mut seeds, &mut handles);
                }
            }
            Event::Empty(e) => {
                if depth >= 2 && token.is_some() {
                    record(&e, &mut in_avlst, &mut in_ahlst, &mut seeds, &mut handles);
                }
            }
            Event::End(name) => {
                if depth == 2 {
                    if let Some(tok) = token.take() {
                        // `upDownArrow` is defined twice, byte-identical — keep only the first.
                        if seen.insert(tok.clone()) {
                            out.push(ShapeAdjustments {
                                token: tok,
                                adjustments: join(&seeds, &handles),
                            });
                        }
                    }
                } else if name.local == "avLst" {
                    in_avlst = false;
                } else if name.local == "ahLst" {
                    in_ahlst = false;
                }
                depth = depth.saturating_sub(1);
            }
            Event::Text(_) => {}
            Event::Eof => break,
        }
    }
    Ok(out)
}

/// Handles one element inside a shape: section markers, `avLst` seeds, and `ahLst` handle refs.
fn record(
    e: &mjx_xml::Element,
    in_avlst: &mut bool,
    in_ahlst: &mut bool,
    seeds: &mut Vec<(String, i32)>,
    handles: &mut HashMap<String, (&'static str, Bound, Bound)>,
) {
    match e.local() {
        "avLst" => *in_avlst = true,
        "ahLst" => *in_ahlst = true,
        "gd" if *in_avlst => {
            if let (Some(name), Some(fmla)) = (e.attr("name"), e.attr("fmla")) {
                if let Some(value) = parse_val(fmla) {
                    seeds.push((name.to_owned(), value));
                }
            }
        }
        "ahXY" if *in_ahlst => {
            record_axis(e, "gdRefX", "minX", "maxX", "Horizontal", handles);
            record_axis(e, "gdRefY", "minY", "maxY", "Vertical", handles);
        }
        "ahPolar" if *in_ahlst => {
            record_axis(e, "gdRefAng", "minAng", "maxAng", "Angle", handles);
            record_axis(e, "gdRefR", "minR", "maxR", "Radius", handles);
        }
        _ => {}
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
        </presetShapeDefinitons>"#;

    #[test]
    fn extracts_literal_and_guide_bounds() {
        let shapes = parse(SAMPLE).unwrap();
        assert_eq!(shapes.len(), 3);

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
}
