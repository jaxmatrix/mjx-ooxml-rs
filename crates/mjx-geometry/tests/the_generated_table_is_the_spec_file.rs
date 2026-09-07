//! The generated table is `presetShapeDefinitions.xml`, in the quantities that file has.
//!
//! # Why counting is the gate here
//!
//! MJXOFF-201 §6: *"all 187 shapes resolve without error" is satisfied by 187 wrong shapes*, and
//! nobody reads 3 923 guide formulas to notice. Resolution succeeding proves almost nothing about
//! an extraction; what proves something is that **the quantities agree with the file's own**, and
//! that every reconciliation is written out as arithmetic a reader can check rather than as a
//! number somebody once measured.
//!
//! So every count below is stated twice: once as what the file contains — obtained by counting XML
//! elements, which is a different reading of the same bytes from the one the extractor does — and
//! once as what the committed table contains, with the difference between them accounted for
//! exactly. `xtask`'s own `preset_geometry_reconciles_with_the_file` re-derives the first half from
//! `References/` where that tree exists; here the numbers are literal, because CI has no
//! `References/` and a table that has silently lost a shape must still fail.
//!
//! **A count in a doc comment is a fact that expires. These are assertions.**

mod common;

use std::collections::{HashMap, HashSet};

use common::{box_on_the_page, extents_of_the_box};
use mjx_geometry::{
    definition_of, preset_outline, seeded_shapes, Derivation, PresetPathStep, PresetShapeType,
    Size, PRESETS_WITHOUT_GEOMETRY,
};
use mjx_ooxml_types::drawingml::adjustment_bound_guides_of;
use mjx_scene::PathCommand;

// -------------------------------------------------------------------------------------------
// What the file holds, counted as XML elements
// -------------------------------------------------------------------------------------------

/// Every `a:gd` element in `presetShapeDefinitions.xml`, in both `avLst` and `gdLst`, counting the
/// duplicated `upDownArrow` block twice — the figure `xtask/src/codegen/geometry.rs` quotes.
const GUIDE_ELEMENTS_IN_THE_FILE: usize = 3_923;

/// Of those, the ones in an `a:avLst`.
const AVLST_ELEMENTS_IN_THE_FILE: usize = 300;

/// Of those, the ones in an `a:gdLst`. `AVLST + GDLST == GUIDE_ELEMENTS_IN_THE_FILE`, asserted.
const GDLST_ELEMENTS_IN_THE_FILE: usize = 3_623;

/// `upDownArrow` is written twice, byte-identically. The second block is dropped, and this is what
/// it contains: two `avLst` guides and eleven `gdLst` guides.
const DUPLICATED_AVLST_GUIDES: usize = 2;

/// As [`DUPLICATED_AVLST_GUIDES`].
const DUPLICATED_GDLST_GUIDES: usize = 11;

/// The duplicated block's one `a:path`.
const DUPLICATED_PATHS: usize = 1;

/// The duplicated block's steps: one `moveTo`, nine `lnTo`, one `close`.
const DUPLICATED_STEPS: StepCounts = StepCounts {
    moves: 1,
    lines: 9,
    arcs: 0,
    quadratics: 0,
    cubics: 0,
    closes: 1,
};

/// Every `a:path` element in the file.
const PATHS_IN_THE_FILE: usize = 320;

/// Every drawing step in the file, by element.
const STEPS_IN_THE_FILE: StepCounts = StepCounts {
    moves: 446,
    lines: 1_698,
    arcs: 393,
    quadratics: 33,
    cubics: 28,
    closes: 320,
};

/// Every `a:rect` element in the file — the text rectangles, one per shape block that has one.
///
/// **182, out of 187 shape blocks**, and the file's own shape element named `rect` is *not* one of
/// them: a text rectangle is written `<rect l= t= r= b=/>` and the shape is written `<rect>`, so a
/// count that matched on the element name alone would say 183. Counting attributes rather than
/// names is what tells the two apart.
const RECT_ELEMENTS_IN_THE_FILE: usize = 182;

/// The duplicated `upDownArrow` block's own `a:rect`.
const DUPLICATED_RECTS: usize = 1;

/// Every `a:cxnLst` element in the file — one per shape block that declares connection sites.
const CXNLST_ELEMENTS_IN_THE_FILE: usize = 174;

/// The duplicated `upDownArrow` block's own `a:cxnLst`.
const DUPLICATED_CXNLSTS: usize = 1;

/// Every `a:cxn` element in the file.
const CXN_ELEMENTS_IN_THE_FILE: usize = 864;

/// The duplicated `upDownArrow` block's eight connection sites.
const DUPLICATED_CXNS: usize = 8;

/// How many values `ST_ShapeType` declares — `crates/mjx-ooxml-types/src/generated/drawingml.rs`.
const SHAPE_TYPE_VALUES: usize = 187;

/// How many of the file's shape elements survive de-duplication, and therefore how many rows the
/// table has.
const SHAPES_IN_THE_TABLE: usize = 186;

/// How many `gdLst` guides `adjustment_bound_guides_of` carries, across every shape.
///
/// The *other* table the same extractor emits, from the same parse: the transitive closure of the
/// guides an adjustment's domain bounds depend on. It is a subset of this table's guides, and
/// [`the_two_tables_of_one_extractor_agree_guide_for_guide`] is why that is worth saying.
///
/// **334, and `xtask/src/codegen/geometry.rs` said 335 until MJXOFF-203 counted it.** The figure had
/// been carried in prose since the module was written, which is exactly the expiry this file exists
/// to prevent.
const BOUND_GUIDES_IN_THE_OTHER_TABLE: usize = 334;

/// A tally of drawing steps by kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct StepCounts {
    moves: usize,
    lines: usize,
    arcs: usize,
    quadratics: usize,
    cubics: usize,
    closes: usize,
}

impl StepCounts {
    /// `self` minus `other`, kind by kind.
    const fn less(self, other: Self) -> Self {
        Self {
            moves: self.moves - other.moves,
            lines: self.lines - other.lines,
            arcs: self.arcs - other.arcs,
            quadratics: self.quadratics - other.quadratics,
            cubics: self.cubics - other.cubics,
            closes: self.closes - other.closes,
        }
    }

    /// The tally of every step in the committed table.
    fn of_the_table() -> Self {
        let mut counts = Self::default();
        for step in seeded_shapes()
            .iter()
            .flat_map(|definition| definition.paths)
            .flat_map(|path| path.steps)
        {
            match step {
                PresetPathStep::MoveTo(_) => counts.moves += 1,
                PresetPathStep::LineTo(_) => counts.lines += 1,
                PresetPathStep::ArcTo { .. } => counts.arcs += 1,
                PresetPathStep::QuadBezierTo { .. } => counts.quadratics += 1,
                PresetPathStep::CubicBezierTo { .. } => counts.cubics += 1,
                PresetPathStep::Close => counts.closes += 1,
            }
        }
        counts
    }
}

// -------------------------------------------------------------------------------------------
// The reconciliation
// -------------------------------------------------------------------------------------------

#[test]
fn the_file_s_own_totals_add_up() {
    // Not a tautology: the three constants above were counted separately, out of the same file, and
    // this is the line that would fail if one of them were mistyped when it was written down.
    assert_eq!(
        AVLST_ELEMENTS_IN_THE_FILE + GDLST_ELEMENTS_IN_THE_FILE,
        GUIDE_ELEMENTS_IN_THE_FILE,
        "the file's `avLst` and `gdLst` guide counts do not add up to its `a:gd` total"
    );
    assert_eq!(
        SHAPES_IN_THE_TABLE + PRESETS_WITHOUT_GEOMETRY.len(),
        SHAPE_TYPE_VALUES,
        "the table's shapes plus the presets with no geometry are not the whole of `ST_ShapeType`"
    );
}

#[test]
fn the_table_holds_every_guide_of_every_shape_the_file_defines() {
    assert_eq!(
        seeded_shapes().len(),
        SHAPES_IN_THE_TABLE,
        "the table has {} rows and the file defines {SHAPES_IN_THE_TABLE} shapes",
        seeded_shapes().len()
    );

    let guides: usize = seeded_shapes()
        .iter()
        .map(|definition| definition.guides.len())
        .sum();
    let values: usize = seeded_shapes()
        .iter()
        .map(|definition| definition.adjustment_values.len())
        .sum();

    // The file's `gdLst` guides, minus the ones in `upDownArrow`'s duplicate block.
    assert_eq!(
        guides,
        GDLST_ELEMENTS_IN_THE_FILE - DUPLICATED_GDLST_GUIDES,
        "the table carries {guides} `gdLst` guides; the file has {GDLST_ELEMENTS_IN_THE_FILE} and \
         {DUPLICATED_GDLST_GUIDES} of them are the duplicated `upDownArrow` block"
    );
    assert_eq!(
        values,
        AVLST_ELEMENTS_IN_THE_FILE - DUPLICATED_AVLST_GUIDES,
        "the table carries {values} `avLst` guides; the file has {AVLST_ELEMENTS_IN_THE_FILE} and \
         {DUPLICATED_AVLST_GUIDES} of them are the duplicated `upDownArrow` block"
    );

    let paths: usize = seeded_shapes()
        .iter()
        .map(|definition| definition.paths.len())
        .sum();
    assert_eq!(paths, PATHS_IN_THE_FILE - DUPLICATED_PATHS);

    assert_eq!(
        StepCounts::of_the_table(),
        STEPS_IN_THE_FILE.less(DUPLICATED_STEPS),
        "the table's drawing steps are not the file's, less the duplicated `upDownArrow` block"
    );

    // MJXOFF-204's two elements, reconciled the same way. A `rect` or a `cxnLst` silently dropped
    // by the reader would resolve perfectly — the shape would simply lay its text against its box
    // and offer nowhere to attach a connector — which is why the arithmetic is here and not left to
    // the resolver.
    let rectangles = seeded_shapes()
        .iter()
        .filter(|definition| definition.text_rectangle.is_some())
        .count();
    assert_eq!(
        rectangles,
        RECT_ELEMENTS_IN_THE_FILE - DUPLICATED_RECTS,
        "the table carries {rectangles} text rectangles; the file has \
         {RECT_ELEMENTS_IN_THE_FILE} and {DUPLICATED_RECTS} of them is the duplicated \
         `upDownArrow` block"
    );

    let lists = seeded_shapes()
        .iter()
        .filter(|definition| !definition.connection_sites.is_empty())
        .count();
    assert_eq!(
        lists,
        CXNLST_ELEMENTS_IN_THE_FILE - DUPLICATED_CXNLSTS,
        "the table carries {lists} connection-site lists; the file has \
         {CXNLST_ELEMENTS_IN_THE_FILE} and {DUPLICATED_CXNLSTS} of them is the duplicated \
         `upDownArrow` block"
    );

    let sites: usize = seeded_shapes()
        .iter()
        .map(|definition| definition.connection_sites.len())
        .sum();
    assert_eq!(
        sites,
        CXN_ELEMENTS_IN_THE_FILE - DUPLICATED_CXNS,
        "the table carries {sites} connection sites; the file has {CXN_ELEMENTS_IN_THE_FILE} and \
         {DUPLICATED_CXNS} of them are the duplicated `upDownArrow` block"
    );

    // **A limit of this comparison, stated rather than hidden.** `connection_sites` is a slice, so
    // the table cannot say *"declares an `a:cxnLst`, and it is empty"* — that case and *"declares
    // none"* are both `is_empty()`, and the list count above would then be one short of the element
    // count while the site count still matched. The file writes no empty `a:cxnLst`, and it is
    // `xtask`'s `preset_geometry_reconciles_with_the_file` that can *check* that, because it has
    // the parse and this suite has only the table. It does check it.
}

#[test]
fn the_two_tables_of_one_extractor_agree_guide_for_guide() {
    // `adjustment_bound_guides_of` and this table come out of the same parse of the same file, four
    // ranks apart, and they overlap: every bound guide is a `gdLst` guide. A cross-check between
    // them catches a de-duplication or an ordering mistake in either — and it is not vacuous,
    // because the two are rendered by different code paths into different crates.
    let mut checked = 0usize;
    for definition in seeded_shapes() {
        let whole: HashMap<&str, &str> = definition
            .guides
            .iter()
            .map(|guide| (guide.wire_name, guide.formula))
            .collect();
        for bound in adjustment_bound_guides_of(definition.preset) {
            let token = definition.preset.to_wire();
            let mine = whole.get(bound.wire_name).unwrap_or_else(|| {
                panic!(
                    "`{token}`'s bound guide `{}` is not in its gdLst",
                    bound.wire_name
                )
            });
            assert_eq!(
                *mine, bound.formula,
                "`{token}`'s guide `{}` reads {mine:?} here and {:?} in \
                 `adjustment_bound_guides_of`",
                bound.wire_name, bound.formula
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked, BOUND_GUIDES_IN_THE_OTHER_TABLE,
        "the bound-guide table has {checked} rows across the shapes this table knows, and \
         {BOUND_GUIDES_IN_THE_OTHER_TABLE} were expected"
    );
}

#[test]
fn every_preset_is_either_in_the_table_or_named_as_missing_from_the_file() {
    // The requirement is *"every shape the extractor cannot handle is listed and named, never
    // silently dropped"*. There is exactly one, it is not this workspace's doing, and it is named.
    assert_eq!(
        PRESETS_WITHOUT_GEOMETRY,
        &[PresetShapeType::UpArrow],
        "the presets with no geometry are no longer just `upArrow`"
    );

    let mut seen: HashSet<PresetShapeType> = HashSet::new();
    for definition in seeded_shapes() {
        let token = definition.preset.to_wire();
        assert!(
            seen.insert(definition.preset),
            "`{token}` appears twice in the table"
        );
        assert!(
            definition_of(definition.preset).is_some(),
            "`{token}` is a row of the table and `definition_of` cannot find it"
        );
        assert!(
            !definition.paths.is_empty(),
            "`{token}` has no paths at all, which is a shape that draws nothing"
        );
        assert_eq!(
            definition.derivation,
            Derivation::ExtractedFromTheGeometryFile,
            "`{token}` is in the generated table but claims another derivation"
        );
        assert!(
            definition.source.contains("presetShapeDefinitions.xml"),
            "`{token}` does not say which file it came out of"
        );
    }

    for preset in PRESETS_WITHOUT_GEOMETRY {
        assert!(
            !seen.contains(preset),
            "`{}` is named as having no geometry and is in the table",
            preset.to_wire()
        );
        assert!(
            definition_of(*preset).is_none(),
            "`{}` is named as having no geometry and `definition_of` answers with some",
            preset.to_wire()
        );
    }
}

// -------------------------------------------------------------------------------------------
// What the resolver does with the table, counted the same way
// -------------------------------------------------------------------------------------------

/// The shapes whose `a:pathLst` writes a drawing step straight after an `a:close`, with no
/// intervening `a:moveTo`.
///
/// ECMA-376 permits it and MJXOFF-202 handles it by restating the closed contour's start point —
/// and recorded that **no seeded shape had one, so deleting that line was a green mutation.** These
/// six are the shapes that make it not one. Each gains exactly one `MoveTo` over what its table row
/// holds.
const SHAPES_THAT_RESTATE_A_START: &[PresetShapeType] = &[
    PresetShapeType::AccentBorderCallout1,
    PresetShapeType::AccentBorderCallout2,
    PresetShapeType::AccentBorderCallout3,
    PresetShapeType::AccentCallout1,
    PresetShapeType::AccentCallout2,
    PresetShapeType::AccentCallout3,
];

/// The shape with a contour ECMA-376 marks neither filled nor stroked.
///
/// `flowChartMultidocument`'s third `a:path` is `fill="none" stroke="false"` — it draws nothing at
/// all, and [`mjx_geometry::outline_of_definition`] leaves it out. It is the whole of the evidence
/// that `@fill` and `@stroke` are read rather than merely stored; `the_flags_reach_a_consumer.rs`
/// is where that is gated in detail.
const SHAPE_WITH_A_CONTOUR_THAT_DRAWS_NOTHING: PresetShapeType =
    PresetShapeType::FlowChartMultidocument;

#[test]
fn the_resolver_neither_loses_a_step_nor_invents_one() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let (mut moves, mut closes) = (0usize, 0usize);
    let mut restated: Vec<PresetShapeType> = Vec::new();
    let mut dropped: Vec<PresetShapeType> = Vec::new();

    for definition in seeded_shapes() {
        let token = definition.preset.to_wire();
        let outline = preset_outline(definition.preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));

        let table_moves = definition
            .paths
            .iter()
            .flat_map(|path| path.steps)
            .filter(|step| matches!(step, PresetPathStep::MoveTo(_)))
            .count();
        let resolved_moves = outline
            .commands
            .iter()
            .filter(|command| matches!(command, PathCommand::MoveTo(_)))
            .count();
        match resolved_moves.cmp(&table_moves) {
            std::cmp::Ordering::Greater => restated.push(definition.preset),
            std::cmp::Ordering::Less => dropped.push(definition.preset),
            std::cmp::Ordering::Equal => {}
        }
        moves += resolved_moves;
        closes += outline
            .commands
            .iter()
            .filter(|command| matches!(command, PathCommand::Close))
            .count();
    }

    assert_eq!(
        restated, SHAPES_THAT_RESTATE_A_START,
        "the shapes that gain a restated start point are not the six the file writes that way"
    );
    assert_eq!(
        dropped,
        vec![SHAPE_WITH_A_CONTOUR_THAT_DRAWS_NOTHING],
        "the shapes that lose a contour are not the one whose contour draws nothing"
    );

    let table = StepCounts::of_the_table();
    assert_eq!(
        moves,
        table.moves + SHAPES_THAT_RESTATE_A_START.len() - 1,
        "the resolved `MoveTo`s are not the table's, plus one per restated start, minus the one \
         belonging to the contour that draws nothing"
    );
    assert_eq!(
        closes,
        table.closes - 1,
        "the resolved `Close`s are not the table's, minus the one belonging to the contour that \
         draws nothing"
    );
}

#[test]
fn every_shape_opens_a_contour_before_it_draws_and_never_closes_one_twice() {
    // The universal structural invariants, and the only ones that *are* universal over 186 shapes.
    // A subpath left open is not one of them: 64 of the presets end a contour without an `a:close`,
    // which is what a stroked outline (`fill="none"`) is, and `line` is literally a line segment.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut contours = 0usize;
    for definition in seeded_shapes() {
        let token = definition.preset.to_wire();
        let outline = preset_outline(definition.preset, extents, &[], within)
            .unwrap_or_else(|error| panic!("`{token}` did not resolve: {error}"));
        assert!(
            !outline.commands.is_empty(),
            "`{token}` resolved to no commands at all, which is the one answer the seam forbids"
        );

        let mut open = false;
        for command in &outline.commands {
            match command {
                PathCommand::MoveTo(_) => {
                    open = true;
                    contours += 1;
                }
                PathCommand::Close => {
                    assert!(open, "`{token}` closed a subpath that was never opened");
                    open = false;
                }
                _ => assert!(
                    open,
                    "`{token}` drew a step before any `MoveTo`; a path that starts with a line has \
                     no start point and is dropped by the tessellator"
                ),
            }
        }
    }
    // A walker that found no contours would satisfy every assertion above by finding no violations.
    assert_eq!(
        contours,
        StepCounts::of_the_table().moves + SHAPES_THAT_RESTATE_A_START.len() - 1,
        "the walk saw {contours} contours across the 186 shapes"
    );
}

#[test]
fn no_shape_draws_a_coordinate_that_is_not_a_number() {
    // A guide list that divides by a zero the size made zero produces an infinity, and an infinite
    // coordinate in a display list is worse than a wrong one — it poisons a bounding box, a cull
    // and a tessellation all at once. Four sizes, including the three degenerate ones, and none of
    // the 186 may produce one at any of them.
    let within = box_on_the_page();
    for extents in [
        extents_of_the_box(),
        Size::from_emu(0, 0),
        Size::from_emu(0, 120 * 12_700),
        Size::from_emu(160 * 12_700, 0),
        Size::from_emu(1, 1),
    ] {
        for definition in seeded_shapes() {
            let token = definition.preset.to_wire();
            let outline =
                preset_outline(definition.preset, extents, &[], within).unwrap_or_else(|error| {
                    panic!(
                        "`{token}` did not resolve at {} × {} EMU: {error}",
                        extents.width.emu(),
                        extents.height.emu()
                    )
                });
            for point in common::points_of(&outline.commands) {
                assert!(
                    point.x.is_finite() && point.y.is_finite(),
                    "`{token}` drew {point:?} at {} × {} EMU",
                    extents.width.emu(),
                    extents.height.emu()
                );
            }
        }
    }
}
