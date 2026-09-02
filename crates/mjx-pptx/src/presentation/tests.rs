//! Unit tests that need the module's private surface (the package's dirty state, the
//! inheritance walk). The behavioural tests live in `tests/`.

use mjx_dml::{FillSpec, IndentLevel};
use mjx_ooxml_core::{RawDocument, RawNode};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::{DML_MAIN, PML};
use mjx_opc::PartName;

use crate::geometry::ShapeBounds;
use crate::{build, constants, nav, slide};

use super::Presentation;

use mjx_dml::{ColorSchemeSlot, SchemeColor};

fn fixture() -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/sample.pptx");
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

/// Whether the slide part still holds its original bytes — a part marked dirty has none, and is
/// re-serialized from its tree on save. This is the only way to see dirtiness: re-serializing is
/// byte-identical for a well-formed part, so a needless rebuild is invisible from outside.
fn slide_is_clean(pres: &Presentation) -> bool {
    pres.package
        .entries()
        .iter()
        .find(|entry| entry.name == "ppt/slides/slide1.xml")
        .expect("the slide part")
        .bytes()
        .is_some()
}

#[test]
fn a_clear_that_finds_nothing_leaves_the_part_clean() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/text_levels.pptx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));

    let mut pres = Presentation::open(&bytes).expect("open");
    assert!(slide_is_clean(&pres));
    assert!(!pres
        .clear_shape_list_style_level(0, 1, IndentLevel::of(5))
        .expect("clear a level the shape never stated"));
    assert!(!pres
        .clear_shape_list_style_default(0, 1)
        .expect("clear a default the shape never stated"));
    assert!(
        slide_is_clean(&pres),
        "a clear that finds nothing must not rebuild the part"
    );

    // The contrast: a clear that finds something does dirty it.
    assert!(pres
        .clear_shape_list_style_level(0, 1, IndentLevel::of(2))
        .expect("clear the level the shape states"));
    assert!(!slide_is_clean(&pres));
}

#[test]
fn color_map_resolves_master_mapping() {
    // The fixture master's p:clrMap is the standard mapping (bg1=lt1, tx1=dk1, …), and slide 0
    // has no p:clrMapOvr — so the effective map is the master's.
    let mut pres = Presentation::open(&fixture()).expect("open");
    let map = pres
        .color_map(0)
        .expect("color_map")
        .expect("fixture has a color map");
    assert_eq!(
        map.resolve(SchemeColor::Background1),
        Some(ColorSchemeSlot::Light1)
    );
    assert_eq!(
        map.resolve(SchemeColor::Text1),
        Some(ColorSchemeSlot::Dark1)
    );
    assert_eq!(
        map.resolve(SchemeColor::Accent1),
        Some(ColorSchemeSlot::Accent1)
    );
}

/// Injects a `p:sp` placeholder of `ph_type` with an explicit `solidFill schemeClr {scheme}` into
/// `part`'s shape tree (the layout/master have empty trees in the fixture).
fn inject_placeholder_fill(pres: &mut Presentation, part: &PartName, ph_type: &str, scheme: &str) {
    let doc = pres.package.part_tree_mut(part).expect("part tree");
    let RawDocument { interner, root, .. } = doc;
    let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

    let ph_attrs = vec![build::attr(interner, "type", ph_type)];
    let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
    let nv_pr = build::node(
        interner,
        "p",
        PML,
        "nvPr",
        Vec::new(),
        vec![RawNode::Element(ph)],
    );
    let cnvpr_attrs = vec![
        build::attr(interner, "id", "10"),
        build::attr(interner, "name", "Injected"),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
    let nv_sp_pr = build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    );

    let clr_attrs = vec![build::attr(interner, "val", scheme)];
    let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
    let solid = build::node(
        interner,
        "a",
        DML_MAIN,
        "solidFill",
        Vec::new(),
        vec![RawNode::Element(scheme_clr)],
    );
    let sp_pr = build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(solid)],
    );
    let sp = build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
    );
    sp_tree.children.push(RawNode::Element(sp));
    sp_tree.empty = false;
}

#[test]
fn effective_fill_inherits_from_layout_placeholder() {
    let mut pres = Presentation::open(&fixture()).expect("open");
    let slide0 = pres.slide_part_checked(0).expect("slide").clone();
    let layout = pres
        .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
        .expect("rel")
        .expect("layout");

    // The layout's ctrTitle placeholder carries an explicit accent2 fill.
    inject_placeholder_fill(&mut pres, &layout, "ctrTitle", "accent2");

    // Slide 0's ctrTitle placeholder declares no fill of its own, so it inherits the layout's —
    // resolved against the real theme (accent2 = ED7D31).
    assert_eq!(
        pres.effective_shape_fill(0, 0).expect("effective fill"),
        Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
    );
}

/// Injects a `p:sp` placeholder of `ph_type` whose `spPr` holds an `a:ln` with a
/// `solidFill schemeClr {scheme}` stroke into `part`'s shape tree.
fn inject_placeholder_outline(
    pres: &mut Presentation,
    part: &PartName,
    ph_type: &str,
    scheme: &str,
) {
    let doc = pres.package.part_tree_mut(part).expect("part tree");
    let RawDocument { interner, root, .. } = doc;
    let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

    let ph_attrs = vec![build::attr(interner, "type", ph_type)];
    let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
    let nv_pr = build::node(
        interner,
        "p",
        PML,
        "nvPr",
        Vec::new(),
        vec![RawNode::Element(ph)],
    );
    let cnvpr_attrs = vec![
        build::attr(interner, "id", "11"),
        build::attr(interner, "name", "InjectedLine"),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
    let nv_sp_pr = build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    );

    let clr_attrs = vec![build::attr(interner, "val", scheme)];
    let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
    let solid = build::node(
        interner,
        "a",
        DML_MAIN,
        "solidFill",
        Vec::new(),
        vec![RawNode::Element(scheme_clr)],
    );
    let ln = build::node(
        interner,
        "a",
        DML_MAIN,
        "ln",
        Vec::new(),
        vec![RawNode::Element(solid)],
    );
    let sp_pr = build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(ln)],
    );
    let sp = build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
    );
    sp_tree.children.push(RawNode::Element(sp));
    sp_tree.empty = false;
}

#[test]
fn effective_outline_inherits_from_layout_placeholder() {
    let mut pres = Presentation::open(&fixture()).expect("open");
    let slide0 = pres.slide_part_checked(0).expect("slide").clone();
    let layout = pres
        .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
        .expect("rel")
        .expect("layout");

    // The layout's ctrTitle placeholder carries an explicit accent2 outline.
    inject_placeholder_outline(&mut pres, &layout, "ctrTitle", "accent2");

    // Slide 0's ctrTitle declares no outline of its own, so it inherits the layout's — resolved
    // against the real theme (accent2 = ED7D31).
    let effective = pres
        .effective_shape_outline(0, 0)
        .expect("effective outline")
        .expect("inherited outline");
    assert_eq!(
        effective.fill,
        Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
    );
}

#[test]
fn effective_outline_resolves_a_line_ref_against_the_theme() {
    let mut pres = Presentation::open(&fixture()).expect("open");
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::Rectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("add shape");

    // Give the shape a p:style > a:lnRef into the theme's line-style 2 (w=12700), with accent1 as
    // the phClr substitute.
    {
        let part = pres.slide_part_checked(0).expect("slide").clone();
        let doc = pres.package.part_tree_mut(&part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");
        let sp = slide::nth_shape_mut(sp_tree, interner, idx).expect("sp");
        let clr_attrs = vec![build::attr(interner, "val", "accent1")];
        let clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
        let ln_ref_attrs = vec![build::attr(interner, "idx", "2")];
        let ln_ref = build::node(
            interner,
            "a",
            DML_MAIN,
            "lnRef",
            ln_ref_attrs,
            vec![RawNode::Element(clr)],
        );
        let style = build::node(
            interner,
            "p",
            PML,
            "style",
            Vec::new(),
            vec![RawNode::Element(ln_ref)],
        );
        sp.children.push(RawNode::Element(style));
        sp.empty = false;
    }

    // The effective outline is theme line-style 2 (w=12700) with phClr baked to accent1 (4472C4).
    let effective = pres
        .effective_shape_outline(0, idx)
        .expect("effective outline")
        .expect("line-ref outline");
    assert_eq!(effective.width, Some(mjx_dml::LineWidth::from_emu(12700)));
    assert_eq!(
        effective.fill,
        Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("4472C4".into())))
    );
}

#[test]
fn a_shapes_own_list_style_beats_the_layout_and_the_master() {
    // Tier 3 of the text ladder, read from markup injected into the raw tree the way the fill
    // tests inject theirs — so the reader is exercised against a shape it did not itself author.
    // The authoring half has its own suite in `tests/shape_list_style.rs`.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/layouts.pptx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    let mut pres = Presentation::open(&bytes).expect("open");

    // `<a:lstStyle><a:lvl1pPr algn="r"><a:defRPr sz="1400"/></a:lvl1pPr></a:lstStyle>` on the
    // body placeholder of slide 0, replacing the empty one the fixture ships.
    {
        let part = pres.slide_part_checked(0).expect("slide").clone();
        let doc = pres.package.part_tree_mut(&part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");
        let sp = slide::nth_shape_mut(sp_tree, interner, 1).expect("body placeholder");
        let tx_body = nav::child_mut(sp, interner, PML, "txBody").expect("txBody");

        let def_rpr_attrs = vec![build::attr(interner, "sz", "1400")];
        let def_rpr = build::leaf(interner, "a", DML_MAIN, "defRPr", def_rpr_attrs);
        let lvl1_attrs = vec![build::attr(interner, "algn", "r")];
        let lvl1 = build::node(
            interner,
            "a",
            DML_MAIN,
            "lvl1pPr",
            lvl1_attrs,
            vec![RawNode::Element(def_rpr)],
        );
        let lst_style = build::node(
            interner,
            "a",
            DML_MAIN,
            "lstStyle",
            Vec::new(),
            vec![RawNode::Element(lvl1)],
        );
        let slot = nav::child_mut(tx_body, interner, DML_MAIN, "lstStyle")
            .expect("the fixture ships an empty a:lstStyle");
        *slot = lst_style;
    }

    let paragraph = pres
        .effective_paragraph_properties(0, 1, 0)
        .expect("effective paragraph");
    assert_eq!(paragraph.alignment(), Some(mjx_dml::TextAlignment::Right));
    // The bullet still comes from the master: tier 3 named an alignment, not a bullet.
    assert!(matches!(
        paragraph.bullet(),
        Some(mjx_dml::Bullet::Character(_))
    ));

    let run = pres
        .effective_run_properties(0, 1, 0, 0)
        .expect("effective run");
    assert_eq!(run.size_points(), Some(14.0), "the shape's own size wins");
    assert_eq!(run.is_bold(), Some(true), "still the layout's bold");
}
