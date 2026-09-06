//! A scene is built from a **foreign** box model — one with no OOXML anywhere in it — and the
//! result is a valid display list.
//!
//! # Why this is the test that matters
//!
//! `DisplayList` is the upper of the architecture's two seams, and the claim it makes is that
//! **nothing below it has heard of a font, a layout algorithm or a document**. A scene builder
//! validated only against a PowerPoint fragment tree would prove nothing about that: every corner it
//! got wrong would be a corner PowerPoint happens not to use, and the obvious green — *the builder
//! compiles and produces bytes* — is satisfied by a builder that is secretly OOXML-shaped.
//!
//! So the box model here is `mjx-layout`'s own second one: plain text reflowed into a fixed-width
//! column, whose content is a `Vec<String>` and which has never heard of a part name, a
//! relationship, a paragraph property, a theme or a `.docx`. It is **included by path rather than
//! copied** —
//!
//! ```text
//! #[path = "../../mjx-layout/tests/support/mod.rs"]
//! ```
//!
//! — for the reason `xtask/tests/layering.rs` gives about its JSON reader: a workspace with two
//! copies of a contract in it has one copy too many, and the copy that is not exercised is the one
//! that is wrong. If this ever stops compiling because `mjx-layout`'s support module changed, that
//! is the seam telling the truth.
//!
//! # What the resolver returns here
//!
//! Nothing. [`PlainText`] answers `None` to every method of [`ResourceResolver`], because the plain
//! text box model issues no decoration handles, no geometry handles and no image handles at all.
//! That is the strongest form of the test: the scene that comes out is glyph runs and nothing else,
//! and every glyph in it went through `place_run` and the atlas at this crate's own device scale.
//!
//! # Proved by mutation
//!
//! * Making `build_scene` skip `Fragment::GlyphRun` → [`the_plain_text_page_becomes_glyph_runs`]
//!   fails with no `DrawGlyphs` at all.
//! * Making `text_paint` return the resolver's answer without the [`DEFAULT_TEXT_COLOR`] fallback →
//!   the same test fails, because a colourless box model then produces a page drawn in nothing.
//! * Making `relative_transform` return the node's absolute map unconditionally →
//!   [`a_transform_inside_a_transform_is_composed_not_replaced`] fails, naming the composed matrix.

#[path = "../../mjx-layout/tests/support/mod.rs"]
mod support;

use std::sync::Arc;

use mjx_layout::{
    BoxFragment, BoxModel, Constraints, DecorationRef, Fragment, FragmentTreeBuilder, GeometryRef,
    ImageFragment, ImageRef, LayoutPoint, LayoutRect, LayoutSize, PageIndex, PartId, ShapeFragment,
    SourcePath, SourceRef, Transform,
};
use mjx_ooxml_core::measure::{Angle, Emu};
use mjx_scene::{
    build_scene, Color, Command, Decoration, DisplayList, FillStyle, Geometry, Image, Paint,
    ResourceIndex, ResourceResolver, SceneOptions, SceneTransform, SectionKind, StrokeStyle,
    DEFAULT_TEXT_COLOR,
};
use mjx_text::{DeviceScale, FontSize, GlyphAtlas, GlyphRasteriser};

use support::plain_text::{PlainTextColumn, PlainTextDocument};

/// A resolver that knows nothing, because the box model it is paired with issues no handles.
struct PlainText;

impl ResourceResolver for PlainText {
    fn decoration(&self, _reference: DecorationRef) -> Option<Decoration> {
        None
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        None
    }

    fn image(&self, _reference: ImageRef) -> Option<Image> {
        None
    }
}

fn a_page_of_prose() -> LayoutSize {
    LayoutSize::new(Emu::from_points(300.0), Emu::from_points(200.0))
}

// -------------------------------------------------------------------------------------------
// The foreign box model
// -------------------------------------------------------------------------------------------

#[test]
fn the_plain_text_page_becomes_glyph_runs() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let face = support::liberation_sans();
    let mut model = PlainTextColumn::new(
        &mut rasteriser,
        Arc::clone(&face),
        FontSize::from_points(14.0),
    );
    let document = PlainTextDocument::from_paragraphs([
        "The display list is the upper of the architecture's two seams, and this paragraph is long \
         enough to wrap several times inside the column it is being reflowed into.",
        "A second paragraph, so that the tree has more than one box under its root.",
    ]);
    let constraints = Constraints::single_column(a_page_of_prose(), Emu::from_points(18.0));
    let page = model
        .layout_page(&document, PageIndex::FIRST, &constraints, None)
        .expect("the plain-text box model lays out its first page");
    let tree = page.fragments();
    assert!(
        tree.len() > 10,
        "the foreign box model produced only {} fragments, which cannot be a wrapped paragraph",
        tree.len()
    );

    let list = build_scene(
        tree,
        &PlainText,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(a_page_of_prose()),
    )
    .expect("a scene builds from a foreign fragment tree");

    // It is a real, valid display list: it decodes from its own bytes with every check run again.
    let reread = DisplayList::from_bytes(list.as_bytes().to_vec())
        .expect("the list a builder produced decodes as one off a disk would");
    assert_eq!(reread.as_bytes(), list.as_bytes());

    // Count what came out, and compare it against the fragments that went in.
    let glyph_runs_in = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .count();
    assert!(glyph_runs_in > 0, "the box model produced no glyph runs");

    let draws = list
        .commands()
        .filter(|command| matches!(command, Command::DrawGlyphs { .. }))
        .count();
    assert_eq!(
        draws, glyph_runs_in,
        "every glyph-run fragment must become exactly one `DrawGlyphs`"
    );
    assert_eq!(
        list.record_count(SectionKind::GlyphRuns),
        u32::try_from(glyph_runs_in).unwrap_or_default()
    );
    assert!(
        list.record_count(SectionKind::Glyphs) > 100,
        "a wrapped paragraph is more than a hundred glyphs; the run table holds {}",
        list.record_count(SectionKind::Glyphs)
    );

    // A colourless box model still produces a page a reader can see.
    assert_eq!(
        list.record_count(SectionKind::Paints),
        1,
        "one text colour for the whole page, interned once"
    );
    assert_eq!(
        list.paint(ResourceIndex::new(0)),
        Some(Paint::Solid(DEFAULT_TEXT_COLOR))
    );

    // And nothing else: the plain-text model decorates nothing, so there is no fill, no stroke, no
    // clip, no effect and no image anywhere in the list.
    for absent in [
        SectionKind::Clips,
        SectionKind::Gradients,
        SectionKind::Strokes,
        SectionKind::Effects,
        SectionKind::Images,
    ] {
        assert_eq!(
            list.record_count(absent),
            0,
            "the `{absent}` table has entries, but a plain-text page has none of those"
        );
    }
    let (width, height) = list.page_size();
    assert!(
        (width - 400.0).abs() < 1e-3 && (height - 800.0 / 3.0).abs() < 1e-3,
        "300 by 200 points at 96 pixels to the inch is 400 by 266.67 pixels, not {width} by {height}"
    );
}

#[test]
fn every_glyph_the_scene_draws_carries_a_cluster_and_an_image() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let face = support::carlito();
    let mut model = PlainTextColumn::new(
        &mut rasteriser,
        Arc::clone(&face),
        FontSize::from_points(11.0),
    );
    let document = PlainTextDocument::from_paragraphs(["office fidelity, first and finally"]);
    let constraints = Constraints::single_column(a_page_of_prose(), Emu::from_points(18.0));
    let page = model
        .layout_page(&document, PageIndex::FIRST, &constraints, None)
        .expect("a page");
    let list = build_scene(
        page.fragments(),
        &PlainText,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(a_page_of_prose()),
    )
    .expect("a scene");

    let mut seen = 0_u32;
    let mut widest_cluster = 0_u32;
    for index in 0..list.record_count(SectionKind::GlyphRuns) {
        let run = list
            .glyph_run(ResourceIndex::new(index))
            .expect("every run in the table decodes");
        assert!(
            run.residual_scale > 0.0,
            "a run at a zero residual scale draws nothing"
        );
        assert!(
            run.bucket_steps > 0,
            "a run in the empty bucket draws nothing"
        );
        for glyph in &run.glyphs {
            seen += 1;
            widest_cluster = widest_cluster.max(glyph.cluster);
            assert!(
                glyph.subpixel < 4,
                "a subpixel phase outside the four this rasteriser has: {}",
                glyph.subpixel
            );
        }
    }
    assert!(
        seen > 20,
        "only {seen} glyphs were placed for a whole line of text"
    );
    assert!(
        widest_cluster > 0,
        "every glyph claims to belong to byte 0, so the cluster is not being carried through and a \
         caret placed from this list would land on the first character every time"
    );
}

#[test]
fn the_same_page_at_two_zooms_produces_two_lists_that_differ_everywhere_that_matters() {
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let face = support::liberation_sans();
    let mut model = PlainTextColumn::new(
        &mut rasteriser,
        Arc::clone(&face),
        FontSize::from_points(12.0),
    );
    let document = PlainTextDocument::from_paragraphs(["Zoom does not rebuild the fragment tree."]);
    let constraints = Constraints::single_column(a_page_of_prose(), Emu::from_points(18.0));
    let page = model
        .layout_page(&document, PageIndex::FIRST, &constraints, None)
        .expect("a page");

    // One fragment tree, two display lists. This is the whole reason `place_run` is called here and
    // not in layout: the tree survives the zoom and only the scene is rebuilt.
    let unzoomed = build_scene(
        page.fragments(),
        &PlainText,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(a_page_of_prose()),
    )
    .expect("a scene at 1×");
    let zoomed = build_scene(
        page.fragments(),
        &PlainText,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions {
            device_scale: DeviceScale::UNZOOMED.zoomed_by(2.0),
            ..SceneOptions::new(a_page_of_prose())
        },
    )
    .expect("a scene at 2×");

    assert_ne!(unzoomed.page_size(), zoomed.page_size());
    assert!(
        mjx_scene::diff_frames(&unzoomed, &zoomed).header_changed(),
        "two frames at different device scales must report a header change, because every glyph in \
         them was rasterised for a different bucket"
    );
    let first = unzoomed
        .glyph_run(ResourceIndex::new(0))
        .expect("a run at 1×");
    let second = zoomed
        .glyph_run(ResourceIndex::new(0))
        .expect("a run at 2×");
    assert_ne!(
        first.bucket_steps, second.bucket_steps,
        "the same run at twice the scale landed in the same bucket, so the scale is not reaching \
         `place_run`"
    );
}

// -------------------------------------------------------------------------------------------
// The fragment kinds the plain-text model does not produce
// -------------------------------------------------------------------------------------------

/// A resolver that decorates everything, so that the box, shape and image paths are exercised.
struct Decorated;

const SLATE: Color = Color {
    red: 0x2b,
    green: 0x2f,
    blue: 0x36,
    alpha: 0xff,
};

impl ResourceResolver for Decorated {
    fn decoration(&self, reference: DecorationRef) -> Option<Decoration> {
        Some(Decoration {
            fill: FillStyle::Solid(Color {
                red: reference.number() as u8,
                green: 0,
                blue: 0,
                alpha: 0xff,
            }),
            stroke: Some(StrokeStyle::solid(2.0, SLATE)),
            ..Decoration::none()
        })
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        Some(Decoration::filled(FillStyle::Solid(SLATE)))
    }

    fn image(&self, reference: ImageRef) -> Option<Image> {
        Some(Image::stretched(reference.number()))
    }
}

fn an_address() -> SourceRef {
    SourceRef::node(PartId::PRIMARY, SourcePath::new(&[0]))
}

#[test]
fn a_decorated_box_becomes_a_fill_and_a_stroke_over_a_rectangle() {
    let mut builder = FragmentTreeBuilder::new();
    builder.push_simple(
        None,
        an_address(),
        LayoutRect::from_edges(
            Emu::from_points(0.0),
            Emu::from_points(0.0),
            Emu::from_points(72.0),
            Emu::from_points(36.0),
        ),
        Fragment::Box(BoxFragment {
            decoration: Some(DecorationRef::new(7)),
            cell: None,
        }),
    );
    let list = scene_of(builder);

    let commands: Vec<Command> = list.commands().collect();
    assert_eq!(commands.len(), 2, "a fill and a stroke: {commands:?}");
    assert!(matches!(commands.first(), Some(Command::FillPath { .. })));
    assert!(matches!(commands.get(1), Some(Command::StrokePath { .. })));
    assert_eq!(
        list.geometry(ResourceIndex::new(0)),
        Some(Geometry::Rectangle(mjx_scene::SceneRect::new(
            0.0, 0.0, 96.0, 48.0
        ))),
        "72 by 36 points at 96 pixels to the inch"
    );
    assert_eq!(
        list.paint(ResourceIndex::new(0)),
        Some(Paint::Solid(Color {
            red: 7,
            green: 0,
            blue: 0,
            alpha: 0xff
        })),
        "the resolver was asked about handle 7 and its answer reached the paint table"
    );
}

#[test]
fn a_shape_becomes_an_unresolved_geometry_carrying_the_handle_and_the_box() {
    let mut builder = FragmentTreeBuilder::new();
    builder.push_simple(
        None,
        an_address(),
        LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_points(48.0),
            Emu::from_points(24.0),
        ),
        Fragment::Shape(ShapeFragment {
            geometry: GeometryRef::new(0x0000_00ff_0000_0042),
            decoration: Some(DecorationRef::new(1)),
        }),
    );
    let list = scene_of(builder);

    assert_eq!(
        list.geometry(ResourceIndex::new(0)),
        Some(Geometry::Unresolved {
            outline: 0x0000_00ff_0000_0042,
            bounds: mjx_scene::SceneRect::new(0.0, 0.0, 64.0, 32.0),
        }),
        "a shape's outline is R07's `GeometryProvider` to resolve; this crate says which and how \
         large and no more"
    );
}

#[test]
fn an_image_fragments_crop_outranks_the_resolvers() {
    let mut builder = FragmentTreeBuilder::new();
    builder.push_simple(
        None,
        an_address(),
        LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_points(96.0),
            Emu::from_points(96.0),
        ),
        Fragment::Image(ImageFragment {
            image: ImageRef::new(3),
            crop: Some(mjx_layout::UnitRect {
                left: 0.25,
                top: 0.0,
                right: 0.75,
                bottom: 1.0,
            }),
        }),
    );
    let list = scene_of(builder);

    let image = list.image(ResourceIndex::new(0)).expect("one image");
    assert_eq!(image.handle, 3);
    assert_eq!(image.crop, mjx_scene::SceneRect::new(0.25, 0.0, 0.75, 1.0));
    assert!(matches!(
        list.commands().next(),
        Some(Command::DrawImage { .. })
    ));
}

#[test]
fn a_transform_inside_a_transform_is_composed_not_replaced() {
    // A group rotated a quarter turn, with a child that is *also* rotated a quarter turn — so the
    // child's absolute map is a half turn and the map pushed for it must be the quarter turn that
    // composes onto the group's, not the half turn itself.
    let quarter = Transform::rotation(Angle::from_degrees(90.0));
    let half = quarter.then(quarter);

    let mut builder = FragmentTreeBuilder::new();
    let outer_transform = builder.transform(quarter);
    let inner_transform = builder.transform(half);
    let rect = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_points(72.0),
        Emu::from_points(72.0),
    );
    let group = builder
        .push(
            None,
            an_address(),
            rect,
            outer_transform,
            None,
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(1)),
                cell: None,
            }),
        )
        .expect("the group");
    builder
        .push(
            Some(group),
            an_address(),
            rect,
            inner_transform,
            None,
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(2)),
                cell: None,
            }),
        )
        .expect("the child");
    let list = scene_of(builder);

    let pushed: Vec<SceneTransform> = list
        .commands()
        .filter_map(|command| match command {
            Command::PushTransform(index) => list.transform(index),
            _ => None,
        })
        .collect();
    assert_eq!(
        pushed.len(),
        2,
        "one push for the group and one for the child"
    );

    let quarter_in_pixels = SceneTransform::from_layout(quarter, DeviceScale::UNZOOMED);
    for (position, pushed) in pushed.iter().enumerate() {
        assert!(
            close_enough(*pushed, quarter_in_pixels),
            "push {position} installed {pushed:?}, but a display list's `PushTransform` composes \
             onto what is already installed — the child's own map is a half turn, so the map \
             pushed for it must be the quarter turn that takes the group's map to it"
        );
    }
}

#[test]
fn a_clip_is_installed_once_for_a_whole_subtree() {
    let mut builder = FragmentTreeBuilder::new();
    let rect = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_points(72.0),
        Emu::from_points(72.0),
    );
    let clip = builder.clip(rect).expect("a non-empty clip");
    let parent = builder
        .push(
            None,
            an_address(),
            rect,
            mjx_layout::TransformId::IDENTITY,
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(1)),
                cell: None,
            }),
        )
        .expect("the clipped box");
    for step in 0..3_u64 {
        builder
            .push(
                Some(parent),
                an_address(),
                rect,
                mjx_layout::TransformId::IDENTITY,
                Some(clip),
                Fragment::Box(BoxFragment {
                    decoration: Some(DecorationRef::new(2 + step)),
                    cell: None,
                }),
            )
            .expect("a child under the same clip");
    }
    let list = scene_of(builder);

    let pushes = list
        .commands()
        .filter(|command| matches!(command, Command::PushClip(_)))
        .count();
    assert_eq!(
        pushes, 1,
        "four fragments under one clip must install it once, not four times — a display list's \
         clips nest, and re-pushing the same rectangle would narrow it to itself three more times \
         for nothing"
    );
    assert_eq!(list.record_count(SectionKind::Clips), 1);
}

#[test]
fn a_line_and_a_table_draw_nothing_of_their_own() {
    let mut builder = FragmentTreeBuilder::new();
    let rect = LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_points(72.0),
        Emu::from_points(12.0),
    );
    builder.push_simple(
        None,
        an_address(),
        rect,
        Fragment::Table(mjx_layout::TableFragment {
            columns: 3,
            rows: 0..2,
            header_rows: 1,
            continued_from_previous_page: false,
            continues_on_next_page: true,
        }),
    );
    builder.push_simple(
        None,
        an_address(),
        rect,
        Fragment::Line(mjx_layout::LineFragment {
            baseline: Emu::from_points(9.0),
            ascent: Emu::from_points(9.0),
            descent: Emu::from_points(3.0),
            base_direction: mjx_text::TextDirection::LeftToRight,
            hanging_width: Emu::ZERO,
        }),
    );
    let list = scene_of(builder);

    assert_eq!(
        list.commands().count(),
        0,
        "a table's ink is its cells and a line's ink is its glyph runs; both are children, and \
         neither fragment paints anything itself"
    );
}

#[test]
fn a_point_is_a_point_and_the_conversion_from_emu_happens_once() {
    // 72 points is an inch, and an inch at 96 pixels to the inch is 96 pixels. Asserted directly so
    // that a change to the conversion is a red here rather than a page drawn at the wrong size.
    assert_eq!(
        mjx_scene::pixels_from_emu(Emu::from_points(72.0), DeviceScale::UNZOOMED),
        96.0
    );
    assert_eq!(
        mjx_scene::pixels_from_emu(Emu::ZERO, DeviceScale::UNZOOMED),
        0.0
    );
    // And the origin of a display list is the page's corner, not the viewport's.
    assert_eq!(
        mjx_scene::ScenePoint::ORIGIN,
        mjx_scene::ScenePoint::new(0.0, 0.0)
    );
    assert_eq!(LayoutPoint::ORIGIN.x, Emu::ZERO);
}

/// Build a scene from a hand-made tree with the decorating resolver.
fn scene_of(builder: FragmentTreeBuilder) -> DisplayList {
    let tree = builder.finish();
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    build_scene(
        &tree,
        &Decorated,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(a_page_of_prose()),
    )
    .expect("a scene builds")
}

/// Whether two maps agree to within the rounding a `f64` EMU translation takes on its way to a
/// `f32` pixel.
fn close_enough(left: SceneTransform, right: SceneTransform) -> bool {
    const TOLERANCE: f32 = 1e-4;
    (left.scale_x - right.scale_x).abs() < TOLERANCE
        && (left.shear_y - right.shear_y).abs() < TOLERANCE
        && (left.shear_x - right.shear_x).abs() < TOLERANCE
        && (left.scale_y - right.scale_y).abs() < TOLERANCE
        && (left.translate_x - right.translate_x).abs() < TOLERANCE
        && (left.translate_y - right.translate_y).abs() < TOLERANCE
}

// -------------------------------------------------------------------------------------------
// The effect DAG
// -------------------------------------------------------------------------------------------

/// A resolver that puts a shadow of a blur on everything, so that the effect DAG is exercised
/// end to end: two nodes, the second consuming the first, and one `PushEffect` naming the root.
struct Shadowed;

impl ResourceResolver for Shadowed {
    fn decoration(&self, _reference: DecorationRef) -> Option<Decoration> {
        Some(Decoration {
            fill: FillStyle::Solid(SLATE),
            opacity: 0.5,
            effects: vec![
                mjx_scene::EffectStyle {
                    radius: 4.0,
                    ..mjx_scene::EffectStyle::new(mjx_scene::EffectKind::Blur)
                },
                mjx_scene::EffectStyle {
                    input: Some(0),
                    ..mjx_scene::EffectStyle::outer_shadow(SLATE, 3.0, 6.0, 0.75)
                },
            ],
            ..Decoration::none()
        })
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        None
    }

    fn image(&self, _reference: ImageRef) -> Option<Image> {
        None
    }
}

#[test]
fn an_effect_dag_becomes_a_topological_table_and_one_push_of_its_root() {
    let mut builder = FragmentTreeBuilder::new();
    builder.push_simple(
        None,
        an_address(),
        LayoutRect::from_edges(
            Emu::ZERO,
            Emu::ZERO,
            Emu::from_points(72.0),
            Emu::from_points(72.0),
        ),
        Fragment::Box(BoxFragment {
            decoration: Some(DecorationRef::new(1)),
            cell: None,
        }),
    );
    let tree = builder.finish();
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let list = build_scene(
        &tree,
        &Shadowed,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(a_page_of_prose()),
    )
    .expect("a scene with effects builds");

    assert_eq!(
        list.record_count(SectionKind::Effects),
        2,
        "a shadow of a blur is two nodes"
    );
    let blur = list.effect(ResourceIndex::new(0)).expect("the blur");
    let shadow = list.effect(ResourceIndex::new(1)).expect("the shadow");
    assert_eq!(blur.kind, mjx_scene::EffectKind::Blur);
    assert_eq!(blur.input, None, "the blur consumes the subtree itself");
    assert_eq!(shadow.kind, mjx_scene::EffectKind::OuterShadow);
    assert_eq!(
        shadow.input,
        Some(ResourceIndex::new(0)),
        "the shadow consumes the blur, and names it by an index strictly below its own — which is \
         the whole of the acyclicity check"
    );
    assert!(shadow.paint.is_some(), "a shadow is drawn in something");

    let commands: Vec<Command> = list.commands().collect();
    assert_eq!(
        commands.first(),
        Some(&Command::PushOpacity(0.5)),
        "opacity wraps the subtree before the effect does: {commands:?}"
    );
    assert_eq!(
        commands.get(1),
        Some(&Command::PushEffect(ResourceIndex::new(1))),
        "the `PushEffect` names the DAG's **root**, which is the last entry of the decoration's \
         effect list: {commands:?}"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| **command == Command::Pop)
            .count(),
        2,
        "one pop for the opacity and one for the effect: {commands:?}"
    );
}

#[test]
fn an_effect_that_names_an_input_above_itself_is_refused() {
    let mut builder = mjx_scene::SceneBuilder::new(mjx_text::DeviceScale::UNZOOMED, 8.0, 8.0);
    let error = builder
        .add_effect_styles(&[
            mjx_scene::EffectStyle {
                input: Some(1),
                ..mjx_scene::EffectStyle::new(mjx_scene::EffectKind::Blur)
            },
            mjx_scene::EffectStyle::new(mjx_scene::EffectKind::Glow),
        ])
        .expect_err("effect 0 cannot consume effect 1");
    assert!(
        matches!(
            error,
            mjx_scene::SceneError::MalformedEffectChain { index: 0, input: 1 }
        ),
        "expected a malformed-effect-chain error, got {error}"
    );
}

// -------------------------------------------------------------------------------------------
// The clip interpretation — the one place this builder reads the fragment contract twice
// -------------------------------------------------------------------------------------------
//
// `scene.rs`'s module documentation says a fragment's clip is *absolute*, that a display list's
// clips *nest*, and that the builder resolves the difference by installing a clip for a node and
// its whole subtree. Until MJXOFF-161's review that paragraph was the only thing asserting any of
// it: deleting `node.clip().or(enclosing.clip)` outright left the crate at 53 passed, 0 failed, and
// a probe that aborted the process when an ancestor clipped and a descendant did not **never
// fired**. The rule was not weakly covered; it was entirely unexercised.
//
// Read closely, that paragraph is **three** claims, not one, and they fail independently:
//
// 1. *Inheritance is honoured.* A descendant that names no clip is drawn inside its ancestor's.
//    This one is **structural** — it follows from `PushClip … Pop` bracketing the whole subtree —
//    and it is what breaks if the `Pop` is emitted too early.
// 2. *An inherited clip is not re-installed.* `enclosing.clip` is read **only** by the
//    `Some(clip_id) != enclosing.clip` comparison, so `.or(enclosing.clip)` is observable only when
//    a descendant *re-states* the ancestor's clip through an intervening node that states none.
//    This is the claim the review's mutation actually breaks, and the tree it needs is three deep —
//    which is why a two-deep tree could never have caught it.
// 3. *A clip is re-installed when the transform changes*, because the same rectangle in a new space
//    is a different region on the page.
//
// Each is gated below against the **emitted command stream** — the seam is the public output, not
// an internal helper — and each was proved to fail by its own mutation.

/// The commands of a scene built from `builder` with the decorating resolver.
fn commands_of(builder: FragmentTreeBuilder) -> Vec<Command> {
    scene_of(builder).commands().collect()
}

/// A square, 72 points on a side, which every node in these trees uses.
fn a_square() -> LayoutRect {
    LayoutRect::from_edges(
        Emu::ZERO,
        Emu::ZERO,
        Emu::from_points(72.0),
        Emu::from_points(72.0),
    )
}

/// Where `wanted` sits in the stream, for an assertion that talks about ordering.
#[track_caller]
fn position_of(commands: &[Command], wanted: &Command) -> usize {
    commands
        .iter()
        .position(|command| command == wanted)
        .unwrap_or_else(|| panic!("no {wanted:?} in {commands:?}"))
}

/// The paint index whose solid colour has `red` in its red channel.
///
/// [`Decorated`] makes a decoration's red channel its handle, so this turns "handle 2's fill" into
/// the index a command names, without the test having to know how the builder interned it.
#[track_caller]
fn paint_with_red(list: &DisplayList, red: u8) -> ResourceIndex {
    for index in 0..list.record_count(SectionKind::Paints) {
        let candidate = ResourceIndex::new(index);
        if let Some(Paint::Solid(color)) = list.paint(candidate) {
            if color.red == red && color.green == 0 && color.blue == 0 {
                return candidate;
            }
        }
    }
    panic!("no paint in the list has red == {red}");
}

#[test]
fn a_descendant_that_states_no_clip_is_drawn_inside_its_ancestors() {
    // An ancestor that clips, and a child that says nothing about clipping at all. The contract
    // does not say what the child is drawn under; this builder says *the ancestor's clip*, and this
    // is where that stops being a paragraph and becomes an assertion.
    let mut builder = FragmentTreeBuilder::new();
    let clip = builder.clip(a_square()).expect("a non-empty clip");
    let ancestor = builder
        .push(
            None,
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(1)),
                cell: None,
            }),
        )
        .expect("the clipped ancestor");
    builder
        .push(
            Some(ancestor),
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            // **No clip.** This is the case the review's probe proved nothing constructed.
            None,
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(2)),
                cell: None,
            }),
        )
        .expect("the unclipped descendant");
    let list = scene_of(builder);
    let commands: Vec<Command> = list.commands().collect();

    let descendant_fill = Command::FillPath {
        geometry: ResourceIndex::new(0),
        paint: paint_with_red(&list, 2),
    };
    let push = position_of(&commands, &Command::PushClip(ResourceIndex::new(0)));
    let pop = position_of(&commands, &Command::Pop);
    let drawn = position_of(&commands, &descendant_fill);
    assert!(
        push < drawn && drawn < pop,
        "the descendant states no clip, so it must be drawn *inside* the clip its ancestor states \
         — the `PushClip` is at {push}, the descendant's fill at {drawn} and the `Pop` at {pop}: \
         {commands:?}"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| **command == Command::Pop)
            .count(),
        1,
        "one clip was installed, so exactly one `Pop` closes it: {commands:?}"
    );
}

#[test]
fn a_descendant_that_restates_an_inherited_clip_does_not_install_it_twice() {
    // Three deep, and it has to be: `enclosing.clip` is read only by the comparison that decides
    // whether to push, so the inheritance is observable only when a descendant *re-states* the
    // ancestor's clip across a node that states none.
    let mut builder = FragmentTreeBuilder::new();
    let clip = builder.clip(a_square()).expect("a non-empty clip");
    let ancestor = builder
        .push(
            None,
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(1)),
                cell: None,
            }),
        )
        .expect("the clipped ancestor");
    let middle = builder
        .push(
            Some(ancestor),
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            None,
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(2)),
                cell: None,
            }),
        )
        .expect("an intervening node that states no clip");
    builder
        .push(
            Some(middle),
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            // The **same** clip the ancestor stated, two levels up.
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(3)),
                cell: None,
            }),
        )
        .expect("a grandchild restating the ancestor's clip");
    let commands = commands_of(builder);

    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::PushClip(_)))
            .count(),
        1,
        "the grandchild names the clip its ancestor already installed, and a display list's clips \
         *intersect* — installing it again would narrow the region to itself for nothing and cost \
         a `Pop` the painter has to balance: {commands:?}"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| **command == Command::Pop)
            .count(),
        1,
        "one push, one pop: {commands:?}"
    );
}

#[test]
fn a_clip_is_reinstalled_when_the_transform_beneath_it_changes() {
    // The same clip identifier, in two different coordinate spaces. A `ClipId` names a rectangle in
    // the space of the node that states it, so the *same* identifier under a rotation is a
    // **different region** on the page. A builder that compared identifiers alone would install it
    // once, in the parent's space, and everything under the rotation would be clipped against the
    // wrong rectangle — invisible until something is actually cut off.
    let mut builder = FragmentTreeBuilder::new();
    let clip = builder.clip(a_square()).expect("a non-empty clip");
    let rotation = builder.transform(Transform::rotation(Angle::from_degrees(90.0)));
    let parent = builder
        .push(
            None,
            an_address(),
            a_square(),
            mjx_layout::TransformId::IDENTITY,
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(1)),
                cell: None,
            }),
        )
        .expect("the clipped parent, in page space");
    builder
        .push(
            Some(parent),
            an_address(),
            a_square(),
            rotation,
            Some(clip),
            Fragment::Box(BoxFragment {
                decoration: Some(DecorationRef::new(2)),
                cell: None,
            }),
        )
        .expect("a child under the same clip but a quarter turn round");
    let commands = commands_of(builder);

    let pushes: Vec<&Command> = commands
        .iter()
        .filter(|command| matches!(command, Command::PushClip(_)))
        .collect();
    assert_eq!(
        pushes.len(),
        2,
        "the child names the same clip *in a rotated space*, which is a different region on the \
         page, so it has to be installed again: {commands:?}"
    );
    assert!(
        pushes
            .iter()
            .all(|command| **command == Command::PushClip(ResourceIndex::new(0))),
        "both installations name the same clip record — the rectangle is identical and only the \
         space it is read in differs, so the table holds one entry: {commands:?}"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::PushTransform(_)))
            .count(),
        1,
        "one rotation, installed once: {commands:?}"
    );

    // And the second installation sits *inside* the transform, or it would be a rectangle in page
    // space wearing the child's coordinates.
    let transform_at = position_of(&commands, &Command::PushTransform(ResourceIndex::new(0)));
    let second_clip_at = commands
        .iter()
        .enumerate()
        .filter(|(_, command)| matches!(command, Command::PushClip(_)))
        .map(|(at, _)| at)
        .nth(1)
        .unwrap_or_default();
    assert!(
        transform_at < second_clip_at,
        "the re-installed clip must sit inside the transform that made it necessary: {commands:?}"
    );
}
