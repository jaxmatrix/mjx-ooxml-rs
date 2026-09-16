//! **RC03's acceptance render for PowerPoint** (MJXOFF-300): the corporate deck, travelling the
//! whole pipeline with **no test double** anywhere in it.
//!
//! ```text
//! Presentation → SlideDeck → SlideBoxModel → FragmentTree
//!              → SlideResources + SlideGeometry → build_page → DisplayList
//!              → SoftwarePainter → pixels
//! ```
//!
//! # What makes this different from `a_real_deck_reaches_pixels.rs`
//!
//! That suite proves the plumbing on fixtures written to break an algorithm. This one renders a
//! file shaped like a document somebody would actually send, and it refuses the three props the
//! older suites lean on:
//!
//! * **no `NoImages`** — the picture bytes come out of the package itself;
//! * **no test-supplied theme** — the deck's own theme part is the only one;
//! * **no test-local outline closure** — the outlines come from
//!   [`mjx_reference_pack::outlines::shape_outline`], the production reader, registered into the
//!   real [`mjx_geometry::PresetGeometryProvider`] that `SlideGeometry` wraps. Until RC03's
//!   implementation half, this journey left the provider unregistered and counted every shape as a
//!   stand-in; that measured the test's own abstinence rather than the library's reach.
//!
//! What remains a **stand-in** is therefore only what the real provider genuinely cannot answer: a
//! `custGeom` shape, whose path tables RC24 (MJXOFF-318) owns. [`EXPECTED_STAND_INS`] is that
//! count, pinned, and a stand-in is never mistaken for the document's own geometry because
//! `mjx_paint::DrawReport::placeholders` counts it.
//!
//! [`the_journey_uses_no_test_double`] holds that as a property of this file rather than as a
//! promise in it, and it runs without the fixture — so it is the assertion that keeps the ignored
//! one below from being vacuous while the fixture is still being authored.
//!
//! # ⚠ This is not parity
//!
//! It proves that a corporate-shaped deck reaches pixels and that **every loss is named**. Whether
//! the pixels look like PowerPoint's is a question only a human sitting against real Microsoft
//! Office on Windows answers (`docs/validation/07-the-reference-pack.md`).

use std::path::{Path, PathBuf};

use mjx_dml::Size;
use mjx_layout::{BoxModel, FragmentTree, FrameContent, LayoutLossKind, LayoutLosses, PageIndex};
use mjx_layout_pptx::{constraints_for, PageCatalogue, SlideBoxModel, SlideDeck};
use mjx_paint::{
    render_offscreen, DrawReport, EncodedImages, Pixels, Resources, SoftwarePainter,
    SOFTWARE_PAINTER,
};
use mjx_pptx::{Presentation, Surface};
use mjx_reference_pack::outlines::shape_outline;
use mjx_scene::{
    build_page, DisplayList, LossCategory, PainterLossKind, SceneLossKind, SceneOptions, SceneRect,
};
use mjx_scene_pptx::{SlideGeometry, SlideResources};
use mjx_text::{FontResolver, GlyphAtlas};

// The fixture RC03 commits. This suite is red until it exists.
const FIXTURE: &str = "corporate.pptx";

// The page's whole loss vector, layout then scene then painter. **Filled from the first green run
// and pinned there** — an empty vector is the claim that the corporate deck renders losslessly, and
// it is false today. Never widen this to make a run pass.
const EXPECTED_LOSSES: &[(LossCategory, usize)] = &[
    // The SmartArt frame: its cached drawing is carried and nothing lays the diagram out (RC28).
    (
        LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
            FrameContent::Diagram,
        )),
        1,
    ),
    // One text body measured with nominal metrics rather than shaped.
    (
        LossCategory::Layout(LayoutLossKind::TextMeasuredNotShaped),
        1,
    ),
    // The chart part is reached and no chart engine draws it yet (RC06).
    (LossCategory::Scene(SceneLossKind::ChartNotResolved), 1),
    // Every run on the slide takes a default colour: the companion carries no run paint (RC16).
    (LossCategory::Scene(SceneLossKind::TextPaintDefaulted), 21),
    // The two pictures: the decoder that turns their bytes into pixels is RC11.
    (LossCategory::Paint(PainterLossKind::ImageWithNoPixels), 2),
    // The connector's arrowhead (RC14) and the custom geometry's outline (RC24).
    (LossCategory::Paint(PainterLossKind::LineEndNotDrawn), 1),
    (LossCategory::Paint(PainterLossKind::OutlineUnresolved), 1),
];

// Every labelled placeholder, as (label, [left, top, right, bottom]) in unzoomed device pixels, in
// paint order. Filled from the first green run and pinned there, on the same terms.
const EXPECTED_PLACEHOLDERS: &[(&str, [i32; 4])] = &[
    // The chart frame and the SmartArt frame, each drawn as a labelled grey box where its content
    // would go. RC06 (MJXOFF-302) and RC28 (MJXOFF-321) are what replace them with a render.
    ("Chart not rendered", [58, 394, 557, 643]),
    ("Diagram not rendered", [586, 394, 874, 643]),
];

// How many draws used stand-in geometry rather than the document's own shape. The provider is the
// real one, so this is what it cannot answer — a `custGeom` until RC24 (MJXOFF-318) lands its path
// tables. Filled from the first green run and pinned there, on the same terms.
//
// **One**, and it is the `custGeom` icon. Every other shape on the slide — the title box, the two
// pictures, the overlay, the connector, the table and the list — resolves to the document's own
// `a:prstGeom` through `mjx-geometry`'s real preset tables. A custom path has no preset to look up,
// so it reaches the stand-in policy and is counted here rather than mistaken for the document's
// own geometry. RC24 (MJXOFF-318) is what makes this zero.
const EXPECTED_STAND_INS: usize = 1;

// How many labelled placeholders the **painter** drew, which is not the same number as the display
// list carries and must not be asserted as though it were.
//
// The list's [`EXPECTED_PLACEHOLDERS`] are the two the *scene* could not resolve — the chart frame
// and the diagram frame. The painter draws those two and then its own, for losses that arise below
// the display list: this deck's two pictures reach it as bytes it cannot decode (RC11). A page with
// no painter-tier loss makes the two counts equal, which is why the Excel journey asserts them
// against one constant; a page with one does not, and equating them there would either hide a
// painter placeholder or demand a scene one that does not exist.
const EXPECTED_LOSS_PLACEHOLDERS: usize = 5;

// How many outline handles the production reader could not answer, counted before anything was
// painted — which is **not** the number of stand-ins drawn, and the difference is the point.
//
// Three handles go unanswered: the slide's two inherited placeholders, which state no `a:prstGeom`
// of their own and take their geometry from the layout (RC24's `pptx-placeholder-geometry`), and
// the `custGeom` icon. Only the icon is drawn, so [`EXPECTED_STAND_INS`] is one: an outline nobody
// asks for costs no pixels. Asserting these two as one number would either demand a draw that does
// not happen or hide a shape the reader silently failed to address.
const EXPECTED_UNREGISTERED_OUTLINES: usize = 3;

// The bundled faces only, so the render does not depend on what the machine has installed.
fn resolver() -> FontResolver {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

// Writes one plate under `target/`, where nothing is committed from.
fn plate(name: &str, bytes: &[u8]) -> PathBuf {
    let directory = mjx_fixtures::workspace_root().join("target/rc03-acceptance");
    std::fs::create_dir_all(&directory).expect("the plate folder is creatable");
    let path = directory.join(name);
    std::fs::write(&path, bytes).expect("the plate is writable");
    path
}

// A scene rectangle, rounded to whole device pixels.
fn rounded(rect: SceneRect) -> [i32; 4] {
    [rect.left, rect.top, rect.right, rect.bottom].map(|value| value.round() as i32)
}

// What the corporate deck's first slide produced on its way to pixels.
struct Journey {
    tree: FragmentTree,
    layout_losses: LayoutLosses,
    list: DisplayList,
    drawn: DrawReport,
    pixels: Pixels,
    // What the provider knew it could not answer, counted before a pixel was drawn.
    stand_ins_before_painting: usize,
}

// The encoded bytes of every picture the page asks for, read out of the package itself.
//
// Not a double: nothing in the library reads a host's image bytes for it, so supplying them *is*
// the caller's half of the contract. What would be a double is answering "there are none".
fn package_images(presentation: &mut Presentation, catalogue: &PageCatalogue) -> EncodedImages {
    let mut images = EncodedImages::new();
    for (handle, request) in catalogue.images().iter().enumerate() {
        let surface = request.surface_index as usize;
        let shapes = presentation.shape_count(surface).unwrap_or(0);
        for shape in 0..shapes {
            let rel_id = presentation
                .picture_image_rel_id(surface, shape)
                .ok()
                .flatten();
            if rel_id.as_deref() != Some(request.image_rel_id.as_str()) {
                continue;
            }
            if let Ok(Some(bytes)) = presentation.picture_image_bytes(surface, shape) {
                images.insert(handle as u64, bytes.into_owned());
            }
            break;
        }
    }
    images
}

// Runs slide 0 of the corporate deck all the way to pixels.
fn journey() -> Journey {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let mut presentation = Presentation::open(&bytes).expect("the corporate deck opens");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let constraints = constraints_for(&deck);

    let mut model = SlideBoxModel::new(resolver());
    let page = model
        .layout_page(&deck, PageIndex::new(0), &constraints, None)
        .expect("the slide lays out");

    // The production reader answers what each handle draws; nothing about it is local to this file.
    let mut geometry = SlideGeometry::new();
    geometry.register_all(model.catalogue(), |request| {
        let extents = Size::from_emu(request.rect.width().emu(), request.rect.height().emu());
        shape_outline(
            &mut presentation,
            Surface::Slide(request.surface_index as usize),
            &request.shape,
            extents,
        )
    });
    let stand_ins_before_painting = geometry.unregistered();
    let images = package_images(&mut presentation, model.catalogue());
    let options = SceneOptions::new(constraints.page);
    let resources = SlideResources::new(model.catalogue().clone(), options.device_scale);
    let mut atlas = GlyphAtlas::new();
    let list = build_page(
        &page,
        &resources,
        model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("the fragment tree becomes a display list");
    let layout_losses = page.losses().clone();
    let (tree, _) = page.into_parts();

    let (page_width, page_height) = list.page_size();
    let width = page_width.ceil().max(1.0) as u32;
    let height = page_height.ceil().max(1.0) as u32;

    let mut painter = SoftwarePainter::new();
    let mut paint_resources = Resources::new(&mut atlas, &geometry, &images);
    let render = render_offscreen(
        &mut painter,
        &list,
        width,
        height,
        1.0,
        &mut paint_resources,
    )
    .expect("the display list rasterises");
    assert_eq!(
        render.painter, SOFTWARE_PAINTER,
        "this gate must run on the pure-Rust painter, so that it needs no graphics stack"
    );

    Journey {
        tree,
        layout_losses,
        list,
        drawn: render.drawn,
        pixels: render.pixels,
        stand_ins_before_painting,
    }
}

/// The corporate deck reaches pixels, and every loss it takes on the way is named.
#[test]
#[ignore = "acceptance render: run with --ignored (MJXOFF-300)"]
fn the_corporate_deck_reaches_pixels_with_its_losses_named() {
    let journey = journey();
    let losses = journey.drawn.page_losses(&journey.list);

    assert_eq!(
        losses.vector(),
        EXPECTED_LOSSES.to_vec(),
        "the page's whole loss vector, layout, scene and painter together. Pin this from the first \
         green run; never widen it to make a run pass."
    );
    assert_eq!(
        losses
            .iter()
            .filter(|loss| matches!(loss.category, LossCategory::Layout(_)))
            .count(),
        journey.layout_losses.len(),
        "the display list carries every loss the layout recorded"
    );

    let drawn_placeholders: Vec<(String, [i32; 4])> = journey
        .list
        .placeholders()
        .into_iter()
        .map(|placeholder| (placeholder.label, rounded(placeholder.rect)))
        .collect();
    let expected: Vec<(String, [i32; 4])> = EXPECTED_PLACEHOLDERS
        .iter()
        .map(|(label, rect)| ((*label).to_owned(), *rect))
        .collect();
    assert_eq!(
        drawn_placeholders, expected,
        "every labelled placeholder, with the label it reads and the rectangle it covers"
    );
    assert_eq!(
        journey.drawn.loss_placeholders, EXPECTED_LOSS_PLACEHOLDERS,
        "the painter drew a placeholder for every loss it could not draw through, which is the two \
         the list carries plus its own; see this constant's own note"
    );
    assert!(
        EXPECTED_LOSS_PLACEHOLDERS >= EXPECTED_PLACEHOLDERS.len(),
        "the painter drew fewer placeholders than the display list carries, so a scene loss reached \
         the page without being drawn at all"
    );
    assert_eq!(
        journey.drawn.placeholders, EXPECTED_STAND_INS,
        "how many draws fell back to stand-in geometry, with the real provider registered"
    );
    // The provider's own count, taken before anything was painted. It is the larger of the two:
    // see this constant's own note for which shapes make up the difference.
    assert_eq!(
        journey.stand_ins_before_painting, EXPECTED_UNREGISTERED_OUTLINES,
        "the production reader answered a different number of outline handles than it did on the \
         run this was pinned from"
    );
    assert!(
        journey.stand_ins_before_painting >= journey.drawn.placeholders,
        "the painter drew more stand-ins ({}) than the provider had unanswered handles ({}), which \
         means a shape whose outline *was* read still fell back",
        journey.drawn.placeholders,
        journey.stand_ins_before_painting
    );

    let png = mjx_paint::export::png(
        journey.pixels.width,
        journey.pixels.height,
        &journey.pixels.rgba,
    );
    let path = plate("pptx.png", &png);
    assert!(
        path.exists(),
        "the plate must be written for a person to look at"
    );

    // A deck of a title, a table, pictures and a chart is neither blank nor a page flooded by a
    // painter that lost its clip. The bounds are wide because the exact figure is the fixture's to
    // fix; what they refuse is the two vacuous passes.
    let covered = journey.pixels.covered();
    let total = journey.pixels.width as usize * journey.pixels.height as usize;
    assert!(total > 0, "the render has a size");
    let fraction = covered as f64 / total as f64;
    assert!(
        fraction > 0.01 && fraction < 0.98,
        "{:.2}% of the page has ink on it; a blank render and a flooded one are the two vacuous \
         passes this assertion exists to refuse",
        fraction * 100.0
    );

    // The fragment tree is what the ink is measured against, so an empty one would make the
    // coverage assertion above meaningless.
    assert!(
        journey.tree.nodes().count() > 1,
        "the corporate slide laid out as its root and nothing else"
    );
}

/// **This journey substitutes nothing**, held as a property of the file rather than a promise in it.
///
/// It needs no fixture, so it is live while the corporate deck is still being authored — which is
/// what stops the ignored test above from being the only thing standing behind the claim.
#[test]
fn the_journey_uses_no_test_double() {
    // Spelled in halves so the needle itself is not a match.
    let forbidden = [
        (
            concat!("No", "Images"),
            "a resolver that answers 'there are no pictures' is a double; read the package's own \
             bytes instead",
        ),
        (
            concat!("fn ", "outline_of"),
            "an outline reader defined in this file is a second implementation of the production \
             one; call `mjx_reference_pack::outlines::shape_outline` instead",
        ),
        (
            concat!("ShapeOutline", " {"),
            "an outline built here is geometry this suite invented rather than read out of the \
             document; the production reader is what answers that",
        ),
    ];
    // Code only. This file's own prose names both needles — it has to, to say what it refuses — and
    // a rule about what the journey *does* must not be decided by what its documentation *says*.
    let code: String = include_str!("an_acceptance_render_of_pptx.rs")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for (needle, why) in forbidden {
        let hits = code.matches(needle).count();
        assert_eq!(
            hits, 0,
            "this journey's code names `{needle}` {hits} time(s), and it must not: {why}"
        );
    }

    // And the scanner is looking at something: a needle this file's code certainly does carry, so a
    // filter that had thrown the whole file away would fail here rather than pass everything above.
    assert!(
        code.contains("SlideGeometry::new()"),
        "the code scan found none of this journey's own source, so its refusals above are vacuous"
    );
    // The positive half of the rule: the production reader is not merely un-replaced, it is called.
    assert!(
        code.contains("shape_outline("),
        "this journey registers no outline from the production reader, so the provider it renders \
         through is not the one the library ships"
    );
}
