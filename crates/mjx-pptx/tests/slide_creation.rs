//! Integration tests for building a slide from a layout, over `tests/fixtures/layouts.pptx` (its
//! layouts are tabulated in `tests/layouts.rs`).
//!
//! The promise being tested is end-to-end: pick a layout, get a slide whose placeholders are already
//! there, fill them with `set_shape_text`, and have everything else — position, size, appearance —
//! still inherit from the layout.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSpec, FillSpec};
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn deck() -> Presentation {
    Presentation::open(&fixture("layouts.pptx")).expect("open")
}

#[test]
fn the_new_slide_is_built_on_the_chosen_layout() {
    let mut pres = deck();
    let before = pres.slide_count();

    // Layout 0 is "Title Slide" — not the layout either existing slide uses by default.
    let slide = pres.add_slide_from_layout(0).expect("add slide");

    assert_eq!(slide, before);
    assert_eq!(pres.slide_count(), before + 1);
    assert_eq!(pres.slide_layout(slide).expect("layout"), Some(0));
}

#[test]
fn the_new_slide_carries_the_layouts_placeholders() {
    let mut pres = deck();
    let slide = pres.add_slide_from_layout(0).expect("add slide");

    let slots: Vec<(PlaceholderType, u32, Option<String>)> =
        (0..pres.shape_count(slide).expect("count"))
            .map(|idx| {
                let ph = pres
                    .shape_placeholder(slide, idx)
                    .expect("placeholder")
                    .expect("every cloned shape is a placeholder");
                (ph.kind, ph.index, ph.name)
            })
            .collect();
    assert_eq!(
        slots,
        vec![
            (
                PlaceholderType::CenteredTitle,
                0,
                Some("Title 1".to_owned())
            ),
            (PlaceholderType::Subtitle, 1, Some("Subtitle 2".to_owned())),
        ],
        "the slots, order and names must match the layout's"
    );
    assert_eq!(pres.shape_kind(slide, 0).expect("kind"), ShapeKind::Shape);
}

#[test]
fn the_cloned_placeholders_are_fillable_immediately() {
    // The whole point: pick a layout, write your text, done.
    let mut pres = deck();
    let slide = pres.add_slide_from_layout(1).expect("add slide"); // "Title and Content"
    pres.set_shape_text(slide, 0, 0, "Quarterly results")
        .expect("set the title");
    pres.set_shape_text(slide, 1, 0, "Revenue is up")
        .expect("set the body");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.shape_text(slide, 0).expect("title text"),
        "Quarterly results"
    );
    assert_eq!(
        reopened.shape_text(slide, 1).expect("body text"),
        "Revenue is up"
    );
}

#[test]
fn the_cloned_placeholders_still_inherit_from_the_layout() {
    let mut pres = deck();
    let slide = pres.add_slide_from_layout(1).expect("add slide");

    // Nothing explicit on the new shape…
    assert_eq!(pres.shape_fill(slide, 0).expect("fill"), None);
    // …so filling the *layout* reaches it.
    let red = FillSpec::solid(ColorSpec::Srgb("C00000".into()));
    pres.set_shape_fill(Surface::Layout(1), 0, &red)
        .expect("fill the layout's title");
    assert_eq!(
        pres.effective_shape_fill(slide, 0).expect("effective fill"),
        Some(red),
        "a cloned placeholder must keep inheriting from its layout"
    );
}

#[test]
fn a_layout_with_no_placeholders_yields_an_empty_slide() {
    let mut pres = deck();
    let slide = pres.add_slide_from_layout(2).expect("add slide"); // "Blank"
    assert_eq!(pres.shape_count(slide).expect("count"), 0);
    assert_eq!(pres.slide_layout(slide).expect("layout"), Some(2));
}

#[test]
fn adding_a_slide_leaves_every_existing_part_alone() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_slide_from_layout(0).expect("add slide");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // Adding a slide must touch exactly three existing parts: the slide list, the relationship to
    // the new slide, and the new part's content type.
    let allowed = [
        "ppt/presentation.xml",
        "ppt/_rels/presentation.xml.rels",
        "[Content_Types].xml",
    ];
    for (name, orig) in &original {
        if allowed.contains(&name.as_str()) {
            continue;
        }
        assert_eq!(reopened.get(name), Some(orig), "part {name} changed");
    }
    assert!(reopened.contains_key("ppt/slides/slide3.xml"));
    assert!(reopened.contains_key("ppt/slides/_rels/slide3.xml.rels"));
}

#[test]
fn each_new_slide_gets_its_own_part_and_ids() {
    let mut pres = deck();
    let first = pres.add_slide_from_layout(0).expect("add slide");
    let second = pres.add_slide_from_layout(1).expect("add slide");
    assert_ne!(first, second);
    assert_eq!(pres.slide_layout(first).expect("layout"), Some(0));
    assert_eq!(pres.slide_layout(second).expect("layout"), Some(1));

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    for name in ["/ppt/slides/slide3.xml", "/ppt/slides/slide4.xml"] {
        assert!(
            pkg.part_names().any(|p| p.as_str() == name),
            "{name} is missing"
        );
    }
}

#[test]
fn an_out_of_range_layout_is_rejected() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");

    let err = pres.add_slide_from_layout(9).expect_err("no such layout");
    assert!(
        matches!(err, PptxError::LayoutIndexOutOfRange { index: 9, count: 3 }),
        "{err:?}"
    );
    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original,
        "a rejected layout must leave the package untouched"
    );
}
