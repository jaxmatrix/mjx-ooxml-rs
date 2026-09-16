//! **RC03's acceptance render for Excel** (MJXOFF-300): the corporate workbook, travelling the
//! whole pipeline with **no test double** anywhere in it.
//!
//! ```text
//! Workbook → SheetGrid → SheetBoxModel → FragmentTree
//!          → SheetResources + SheetGeometry → build_page → DisplayList
//!          → SoftwarePainter → pixels
//! ```
//!
//! # What makes this different from `a_real_worksheet_reaches_pixels.rs`
//!
//! That suite proves the plumbing on fixtures written to break an algorithm. This one renders a
//! workbook shaped like one somebody would actually send — a styled table, accounting and date
//! formats in narrow columns, a colour scale, a data bar, an icon set, rich text, a chart, a
//! picture, a text box, diagonal and dashed borders, and a frozen pane.
//!
//! **No `NoImages`**: the picture bytes come out of the package. **No test-supplied theme**: the
//! palette takes the workbook's own `xl/theme/theme1.xml`, which is the division `SheetPalette` is
//! shaped for — a SpreadsheetML colour addresses the scheme by *position*, so `<color theme="4"/>`
//! means nothing without the part, and reading it out of the file this test opened is the caller's
//! half of the contract rather than a prop.
//!
//! [`the_journey_uses_no_test_double`] holds that as a property of the file, and runs without the
//! fixture — so it is what keeps the ignored test below from being vacuous while the workbook is
//! still being authored.
//!
//! # ⚠ This is not parity
//!
//! Whether the pixels look like Excel's is a question only a human sitting against real Microsoft
//! Excel on Windows answers (`docs/validation/07-the-reference-pack.md`).

use std::path::{Path, PathBuf};

use mjx_layout::{
    BoxModel, Constraints, FragmentTree, FrameContent, LayoutLossKind, LayoutLosses, LayoutSize,
    PageIndex,
};
use mjx_layout_xlsx::{constraints_for, SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_paint::{
    render_offscreen, DrawReport, EncodedImages, Pixels, Resources, SoftwarePainter,
    SOFTWARE_PAINTER,
};
use mjx_scene::{build_page, DisplayList, LossCategory, SceneOptions, SceneRect};
use mjx_scene_xlsx::{SheetGeometry, SheetPalette, SheetResources};
use mjx_text::{FontResolver, GlyphAtlas};
use mjx_xlsx::Workbook;

// The fixture RC03 commits. This suite is red until it exists.
const FIXTURE: &str = "corporate.xlsx";

// The viewport the corporate sheet is rendered into, in inches. A band rather than a page: Excel
// has no page of its own, so the caller states one.
const VIEWPORT_INCHES: (f64, f64) = (8.0, 5.0);

// The page's whole loss vector, layout then scene then painter. **Filled from the first green run
// and pinned there** — an empty vector is the claim that the corporate sheet renders losslessly,
// and it is false today. Never widen this to make a run pass.
const EXPECTED_LOSSES: &[(LossCategory, usize)] = &[
    // The drawing's four anchored objects — the chart, the picture and the text box among them —
    // are framed and not laid out: a worksheet's drawing layer is RC28 (MJXOFF-321).
    (
        LossCategory::Layout(LayoutLossKind::FrameContentNotLaidOut(
            FrameContent::Picture,
        )),
        4,
    ),
    // One thing the sheet states that the reader does not carry to the box model.
    (LossCategory::Layout(LayoutLossKind::DroppedByReader), 1),
    // Three values the grid approximated rather than resolved exactly.
    (LossCategory::Layout(LayoutLossKind::ValueApproximated), 3),
];

// Every labelled placeholder, as (label, [left, top, right, bottom]) in unzoomed device pixels, in
// paint order. Filled from the first green run and pinned there, on the same terms.
const EXPECTED_PLACEHOLDERS: &[(&str, [i32; 4])] = &[
    // The four anchored objects the drawing layer does not place (RC28, MJXOFF-321), and the one
    // thing the reader dropped — each a labelled box in the cell band it was anchored to.
    ("Picture not rendered", [328, 40, 389, 60]),
    ("Content not read", [450, 40, 511, 60]),
    ("Picture not rendered", [328, 60, 389, 80]),
    ("Picture not rendered", [328, 80, 389, 100]),
    ("Picture not rendered", [328, 100, 389, 120]),
];

// How many draws used stand-in geometry rather than the document's own shape. A worksheet issues no
// outline handle at all today, so this is expected to stay zero; it is pinned like the rest.
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

// A viewport of `width` by `height` inches.
fn viewport(width: f64, height: f64) -> Constraints {
    constraints_for(LayoutSize {
        width: Emu::from_inches(width),
        height: Emu::from_inches(height),
    })
}

// What the corporate workbook's first band produced on its way to pixels.
struct Journey {
    tree: FragmentTree,
    layout_losses: LayoutLosses,
    list: DisplayList,
    drawn: DrawReport,
    pixels: Pixels,
}

// Runs the first band of sheet 0 all the way to pixels.
fn journey() -> Journey {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let mut book = Workbook::open(&bytes).expect("the corporate workbook opens");

    // The workbook's own theme part, not one this test made up.
    let theme = book.theme_colors().expect("the theme part reads");
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = viewport(VIEWPORT_INCHES.0, VIEWPORT_INCHES.1);

    let mut model = SheetBoxModel::new(resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the band lays out");

    let formatting = grid.formatting();
    let interner = formatting
        .resolver()
        .expect("a format resolver")
        .formats()
        .interner();
    let mut palette = SheetPalette::from_stylesheet(formatting.stylesheet(), interner);
    if let Some(theme) = theme {
        palette = palette.with_theme(theme);
    }
    let resources = SheetResources::new(model.catalogue().clone(), palette);
    let geometry = SheetGeometry::new();

    let options = SceneOptions::new(constraints.page);
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

    // The pictures the corporate sheet carries reach the painter as their own encoded bytes; an
    // empty table here would be the answer "there are no pictures", which is the double this
    // journey refuses.
    let images = sheet_images(&book);

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

// Every picture the sheet's drawing names, as the encoded bytes the package holds.
//
// Not a double: nothing in the library reads a host's image bytes for it. A worksheet's fragment
// tree carries no picture today, so this is empty in practice and becomes the real table the moment
// MJXOFF-321 puts a drawing on the page — at which point a journey that had hard-coded emptiness
// would start lying.
fn sheet_images(book: &Workbook) -> EncodedImages {
    let mut images = EncodedImages::new();
    let Ok(Some(drawing)) = book.sheet_drawing(0) else {
        return images;
    };
    for (handle, object) in drawing.objects.iter().enumerate() {
        let Some(part) = object.image.as_ref() else {
            continue;
        };
        if let Some(bytes) = book.package().part_payload(part) {
            images.insert(handle as u64, bytes.into_owned());
        }
    }
    images
}

/// The corporate workbook reaches pixels, and every loss it takes on the way is named.
#[test]
#[ignore = "acceptance render: run with --ignored (MJXOFF-300)"]
fn the_corporate_workbook_reaches_pixels_with_its_losses_named() {
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
        "a worksheet issues no outline handle, so no draw may fall back to a stand-in"
    );

    let png = mjx_paint::export::png(
        journey.pixels.width,
        journey.pixels.height,
        &journey.pixels.rgba,
    );
    let path = plate("xlsx.png", &png);
    assert!(
        path.exists(),
        "the plate must be written for a person to look at"
    );

    // A worksheet is mostly empty grid, so the floor is far below a slide's — and it is a floor
    // rather than nothing, because a blank render is the vacuous pass it exists to refuse.
    let covered = journey.pixels.covered();
    let total = journey.pixels.width as usize * journey.pixels.height as usize;
    assert!(total > 0, "the render has a size");
    let fraction = covered as f64 / total as f64;
    assert!(
        fraction > 0.0005 && fraction < 0.90,
        "{:.4}% of the band has ink on it; a blank render and a flooded one are the two vacuous \
         passes this assertion exists to refuse",
        fraction * 100.0
    );
    assert!(
        journey.tree.nodes().count() > 1,
        "the corporate band laid out as its root and nothing else"
    );
}

/// **This journey substitutes nothing**, held as a property of the file rather than a promise in it.
///
/// It needs no fixture, so it is live while the corporate workbook is still being authored.
#[test]
fn the_journey_uses_no_test_double() {
    // Spelled in halves so the needle itself is not a match.
    let forbidden = [(
        concat!("No", "Images"),
        "a resolver that answers 'there are no pictures' is a double; read the package's own bytes \
         instead",
    )];
    // Code only. This file's own prose names the needle — it has to, to say what it refuses — and a
    // rule about what the journey *does* must not be decided by what its documentation *says*.
    let code: String = include_str!("an_acceptance_render_of_xlsx.rs")
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

    // The theme must come out of the workbook rather than be built here. `SheetPalette::with_theme`
    // is legitimate — it is *how* the document's own theme is applied — so what is checked is that
    // the value handed to it came from the package. This doubles as the scanner's own instrument: a
    // filter that had thrown the whole file away would fail here rather than pass everything above.
    assert!(
        code.contains("book.theme_colors()"),
        "the palette must take the workbook's own theme part, not one this suite constructed"
    );
}
