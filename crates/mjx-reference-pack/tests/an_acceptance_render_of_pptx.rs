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
//! * **no test-local outline closure** — `SlideGeometry` is left unregistered, because the closure
//!   the two older suites each carry a copy of is exactly the production hole RC03 exists to
//!   expose. An outline nothing can answer is asserted as a **stand-in**, by count, rather than
//!   filled in here.
//!
//! [`the_journey_uses_no_test_double`] holds that last paragraph as a property of this file rather
//! than as a promise in it, and it runs without the fixture — so it is the assertion that keeps the
//! ignored one below from being vacuous while the fixture is still being authored.
//!
//! # ⚠ This is not parity
//!
//! It proves that a corporate-shaped deck reaches pixels and that **every loss is named**. Whether
//! the pixels look like PowerPoint's is a question only a human sitting against real Microsoft
//! Office on Windows answers (`docs/validation/07-the-reference-pack.md`).

use std::path::{Path, PathBuf};

use mjx_layout::{BoxModel, FragmentTree, LayoutLosses, PageIndex};
use mjx_layout_pptx::{constraints_for, PageCatalogue, SlideBoxModel, SlideDeck};
use mjx_paint::{
    render_offscreen, DrawReport, EncodedImages, Pixels, Resources, SoftwarePainter,
    SOFTWARE_PAINTER,
};
use mjx_pptx::Presentation;
use mjx_scene::{build_page, DisplayList, LossCategory, SceneOptions, SceneRect};
use mjx_scene_pptx::{SlideGeometry, SlideResources};
use mjx_text::{FontResolver, GlyphAtlas};

// The fixture RC03 commits. This suite is red until it exists.
const FIXTURE: &str = "corporate.pptx";

// The page's whole loss vector, layout then scene then painter. **Filled from the first green run
// and pinned there** — an empty vector is the claim that the corporate deck renders losslessly, and
// it is false today. Never widen this to make a run pass.
const EXPECTED_LOSSES: &[(LossCategory, usize)] = &[];

// Every labelled placeholder, as (label, [left, top, right, bottom]) in unzoomed device pixels, in
// paint order. Filled from the first green run and pinned there, on the same terms.
const EXPECTED_PLACEHOLDERS: &[(&str, [i32; 4])] = &[];

// How many draws used stand-in geometry rather than the document's own shape, because this journey
// registers no outline. Filled from the first green run and pinned there, on the same terms.
const EXPECTED_STAND_INS: usize = 0;

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

    // Deliberately unregistered: see this module's own documentation.
    let geometry = SlideGeometry::new();
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
        journey.drawn.loss_placeholders,
        EXPECTED_PLACEHOLDERS.len(),
        "the painter drew one placeholder per loss the list carries"
    );
    assert_eq!(
        journey.drawn.placeholders, EXPECTED_STAND_INS,
        "how many draws fell back to stand-in geometry, because this journey registers no outline"
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
            concat!("register", "_all"),
            "registering outlines from a test-local closure is the production hole RC03 exists to \
             expose; assert the stand-in count instead",
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
}
