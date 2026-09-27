//! A painter must be able to tell a stand-in shape from the document's own, and until MJXOFF-163 it
//! could not.
//!
//! MJX-STAND-IN: `OutlineProvenance` is what this suite is about, and the stand-in is the only
//! producer of `Placeholder` there is. This crate is rank 1.7 and `mjx-geometry` is 2.5, so the
//! real provider is an upward edge the layering test refuses by name.
//!
//! # What was wrong, and why it was not cosmetic
//!
//! [`ResolvedOutline`](mjx_scene::ResolvedOutline) has carried `provenance` and `label` since
//! MJXOFF-162, and `mjx-scene`'s own module documentation said in as many words that they were there
//! *"so that a stand-in can never be mistaken for the real thing — by a reviewer looking at a render,
//! by a golden-image gate, or by a painter that wants to draw placeholders in a warning colour"*.
//!
//! None of those three could actually do it. `tessellate.rs`'s `outline_of` — the single point at
//! which this crate consumes a provider for rendering — destructured `commands` and `fill_rule` and
//! **dropped both other fields**, and `SceneMesh`, which is a painter's whole input from
//! [`tessellate_scene`], had three fields and none of them was this one. `provenance` was written
//! once and read zero times anywhere in `crates/mjx-scene/src`.
//!
//! That is the mechanism R10's fidelity rule was told to depend on: *golden images must not be taken
//! against placeholder geometry and called fidelity*. **Every preset shape in this programme
//! resolved to a placeholder when this file was written**, so without it the guard did not exist at
//! all: a full page of framed, crossed rounded rectangles would have compared clean against a golden
//! image of itself and been recorded as parity.
//!
//! MJXOFF-206 wired `mjx-geometry`'s `PresetGeometryProvider` in, so a preset now resolves to the
//! document's own geometry and this field is normally `Document`. **That makes it worth more, not
//! less.** A flag that was `Placeholder` on every shape said nothing about any particular shape; one
//! that is `Placeholder` on a shape today names a handle nobody registered, a preset ECMA-376
//! defines no geometry for, or a shape singular at the adjustments in force.
//!
//! # What is asserted here
//!
//! 1. a scene resolved through [`PlaceholderGeometry`] produces meshes that **say so**, and whose
//!    label names the handle each stands in for;
//! 2. the very same scene through a provider that answers with the document's own geometry produces
//!    meshes that say the opposite — so the field tracks the provider and is not a constant;
//! 3. geometry that never went through a provider at all — an explicit path, a rectangle — is the
//!    document's own, with no label to invent;
//! 4. a **cache hit answers the same as the miss that filled it**, which is the one place this could
//!    have gone quietly wrong: `outline_of` runs before the cache lookup, and a version that moved
//!    it after would report `Document` for every second ask;
//! 5. and the downstream consequence, which is the whole point: a caller can count the placeholders
//!    in a page and refuse to call the render a fidelity render.
//!
//! # Proved by mutation
//!
//! * Hard-coding `SceneMesh::provenance` to `Provenance::document()` in `tessellate_scene` → case
//!   one fails, naming a page of placeholders that claims to be the document's own geometry.
//! * Moving `outline_of` after the cache lookup in `fill_resolved` → case four fails on the second
//!   ask.
//! * Dropping the label from `outline_of` → case one's label assertion fails, which is what stops
//!   the field becoming an unlabelled boolean again.

use mjx_scene::{
    tessellate_scene, Color, Command, DisplayList, FillRule, Geometry, GeometryProvider, MeshRole,
    OutlineProvenance, Paint, PathCommand, PlaceholderGeometry, Provenance, ResolvedOutline,
    SceneBuilder, SceneError, ScenePoint, SceneRect, TessellationOptions, Tessellator,
};
use mjx_text::{DeviceScale, ScaleBucket};

/// The handle the scenes below give their shape.
const SHAPE: u64 = 0x0000_0000_dead_beef;

/// The box it is drawn in.
fn shape_box() -> SceneRect {
    SceneRect::new(10.0, 20.0, 130.0, 100.0)
}

/// A provider that answers with a shape it calls the document's own.
///
/// A triangle rather than another rounded rectangle, so that the two providers below cannot be
/// telling the same story with different labels.
struct DocumentGeometry;

impl GeometryProvider for DocumentGeometry {
    fn outline(&self, _outline: u64, within: SceneRect) -> Result<ResolvedOutline, SceneError> {
        Ok(ResolvedOutline {
            commands: vec![
                PathCommand::MoveTo(ScenePoint::new(within.left, within.bottom)),
                PathCommand::LineTo(ScenePoint::new(within.right, within.bottom)),
                PathCommand::LineTo(ScenePoint::new(
                    (within.left + within.right) / 2.0,
                    within.top,
                )),
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
            label: "isoscelesTriangle".to_owned(),
            provenance: OutlineProvenance::Document,
        })
    }
}

fn ink() -> Color {
    Color {
        red: 0x11,
        green: 0x22,
        blue: 0x33,
        alpha: 0xff,
    }
}

/// A page whose one shape is an unresolved handle.
fn a_page_with_an_unresolved_shape() -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 200.0, 120.0);
    let geometry = builder
        .add_geometry(&Geometry::Unresolved {
            outline: SHAPE,
            bounds: shape_box(),
        })
        .expect("one unresolved geometry");
    let paint = builder.add_paint(Paint::Solid(ink())).expect("one paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.finish().expect("the scene is well formed")
}

/// A page whose one shape is an explicit path — nothing for a provider to resolve.
fn a_page_with_a_resolved_shape() -> DisplayList {
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, 200.0, 120.0);
    let geometry = builder
        .add_geometry(&Geometry::path(
            vec![
                PathCommand::MoveTo(ScenePoint::new(10.0, 10.0)),
                PathCommand::LineTo(ScenePoint::new(90.0, 10.0)),
                PathCommand::LineTo(ScenePoint::new(90.0, 70.0)),
                PathCommand::Close,
            ],
            FillRule::NonZero,
        ))
        .expect("one path");
    let paint = builder.add_paint(Paint::Solid(ink())).expect("one paint");
    builder
        .push(Command::FillPath { geometry, paint })
        .expect("a fill");
    builder.finish().expect("the scene is well formed")
}

#[test]
fn a_placeholder_says_it_is_a_placeholder_and_names_the_handle_it_stands_in_for() {
    let list = a_page_with_an_unresolved_shape();
    let mut tessellator = Tessellator::new();
    let meshes = tessellate_scene(&list, &PlaceholderGeometry::new(), &mut tessellator)
        .expect("the placeholder resolves every handle");

    assert_eq!(meshes.len(), 1);
    let mesh = meshes.first().expect("one mesh");
    assert_eq!(mesh.role, MeshRole::Fill);
    assert!(
        mesh.provenance.is_placeholder(),
        "the only provider that answered was the stand-in, and the mesh says {:?}. A painter that \
         cannot see this cannot draw placeholders in a warning colour, and a golden-image gate that \
         cannot see it will ratify a page of framed rounded rectangles as fidelity.",
        mesh.provenance.origin
    );
    assert_eq!(
        mesh.provenance.label.as_deref(),
        Some(PlaceholderGeometry::label_for(SHAPE).as_str()),
        "the label must name the handle the stand-in is standing in for, or a report of \
         placeholders says only how many and never which"
    );
}

#[test]
fn the_same_scene_through_a_real_provider_says_the_document_drew_it() {
    let list = a_page_with_an_unresolved_shape();
    let mut tessellator = Tessellator::new();
    let meshes = tessellate_scene(&list, &DocumentGeometry, &mut tessellator)
        .expect("the document's own geometry resolves this handle");

    let mesh = meshes.first().expect("one mesh");
    assert!(
        !mesh.provenance.is_placeholder(),
        "this provider answered with the document's own geometry and the mesh claims otherwise"
    );
    assert_eq!(mesh.provenance.origin, OutlineProvenance::Document);
    assert_eq!(
        mesh.provenance.label.as_deref(),
        Some("isoscelesTriangle"),
        "the label is the provider's, carried through unchanged"
    );
}

#[test]
fn geometry_that_never_met_a_provider_is_the_documents_own_and_carries_no_label() {
    let list = a_page_with_a_resolved_shape();
    let mut tessellator = Tessellator::new();
    let meshes = tessellate_scene(&list, &PlaceholderGeometry::new(), &mut tessellator)
        .expect("an explicit path needs no provider");

    let mesh = meshes.first().expect("one mesh");
    assert_eq!(mesh.provenance, Provenance::document());
    assert!(
        mesh.provenance.label.is_none(),
        "an explicit path has no name a provider could have given it, and inventing one would make \
         every mesh on a page claim a provenance nobody supplied"
    );
}

#[test]
fn a_cache_hit_answers_the_same_provenance_as_the_miss_that_filled_it() {
    // The one place this could have gone quietly wrong. `outline_of` runs *before* the cache
    // lookup, so the provider is consulted on a hit too; a version that consulted it only on a miss
    // would report `Document` for every second ask and nothing else in the suite would notice,
    // because the triangles would be identical either way.
    let geometry = Geometry::Unresolved {
        outline: SHAPE,
        bounds: shape_box(),
    };
    let options = TessellationOptions::for_bucket(ScaleBucket::from_steps(8));
    let mut tessellator = Tessellator::new();

    let (first, first_provenance) = tessellator
        .fill_resolved(&geometry, &PlaceholderGeometry::new(), options)
        .expect("a placeholder tessellates");
    let (second, second_provenance) = tessellator
        .fill_resolved(&geometry, &PlaceholderGeometry::new(), options)
        .expect("and again");

    assert_eq!(
        tessellator.cache().hits(),
        1,
        "the second ask must have been a cache hit, or this test is not testing a cache hit"
    );
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "the same path at the same options is the very same triangles"
    );
    assert_eq!(first_provenance, second_provenance);
    assert!(second_provenance.is_placeholder());
}

#[test]
fn a_caller_can_refuse_to_call_a_page_of_placeholders_a_fidelity_render() {
    // The downstream consequence, and the reason the field exists. A getter that answered correctly
    // and that nothing acted on would be exactly the defect this whole file is a fix for.
    let list = a_page_with_an_unresolved_shape();
    let mut tessellator = Tessellator::new();

    let stood_in = tessellate_scene(&list, &PlaceholderGeometry::new(), &mut tessellator)
        .expect("the placeholder resolves every handle");
    let real = tessellate_scene(&list, &DocumentGeometry, &mut tessellator)
        .expect("and so does the document's own geometry");

    let count = |meshes: &[mjx_scene::SceneMesh]| {
        meshes
            .iter()
            .filter(|mesh| mesh.provenance.is_placeholder())
            .count()
    };
    assert_eq!(count(&stood_in), 1);
    assert_eq!(count(&real), 0);

    // And the two really are different pictures, so the count is not distinguishing two identical
    // renders by a label alone.
    let stood_in_bytes = stood_in
        .first()
        .map(|mesh| mesh.mesh.vertex_bytes())
        .unwrap_or_default();
    let real_bytes = real
        .first()
        .map(|mesh| mesh.mesh.vertex_bytes())
        .unwrap_or_default();
    assert_ne!(stood_in_bytes, real_bytes);
}
