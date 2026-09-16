//! **RC03's acceptance render for Word** (MJXOFF-300) — and the one of the three that **cannot
//! reach pixels**.
//!
//! ```text
//! Document → DocumentFlow → DocumentBoxModel → PageFragments
//!          → ✗ there is no scene companion, so the chain stops here
//! ```
//!
//! PowerPoint's companion is `mjx-scene-pptx` and Excel's is `mjx-scene-xlsx`, both at rank 3.7.
//! **`mjx-scene-docx` does not exist**, and until RC09 builds it a Word `FragmentTree` has no route
//! into a `DisplayList` and therefore none into a painter. This suite asserts that state concretely
//! rather than describing it: [`word_has_no_scene_companion`] fails the day the crate lands, which
//! is the day this file has to grow the other half.
//!
//! So what is asserted here is the **whole loss vector of the last stage Word reaches** — the box
//! model's own — plus the fragment tree that vector is about. That is not a lesser assertion than
//! the other two journeys make; it is the same assertion, stopped where the pipeline stops.
//!
//! # The plate
//!
//! A format that cannot rasterise still owes a person something to look at, so the plate here is the
//! **fragment tree**, written under `target/` as text. It is what a reviewer reads to see where Word
//! put things, and it is what the pixel plate will be diffed against once RC09 exists.

use std::path::{Path, PathBuf};

use mjx_docx::Document;
use mjx_layout::{BoxModel, Fragment, FragmentTree, LayoutLossKind, LayoutLosses, PageIndex};
use mjx_layout_docx::{constraints_for, DocumentBoxModel, DocumentFlow};
use mjx_text::FontResolver;

// The fixture RC03 commits. This suite is red until it exists.
const FIXTURE: &str = "corporate.docx";

// Layout losses as [chart, diagram, object, ink, picture, shape, unshaped, not read, approximated].
// **Filled from the first green run and pinned there** — all zeroes is the claim that the corporate
// document lays out losslessly, and it is false today. Never widen this to make a run pass.
//
// The three are the document's pictures: the inline logo, the floating one and the header's. Word's
// box model places a frame and does not lay its content out, which is RC11 (MJXOFF-306) for the
// pixels and RC09 (MJXOFF-255) for the scene companion that would carry them.
const EXPECTED_LAYOUT_LOSSES: [usize; 9] = [0, 0, 0, 0, 3, 0, 0, 0, 0];

// How many fragments of each kind page 0 holds, as [boxes, lines, glyphs, images, shapes, tables].
// Filled from the first green run and pinned there, on the same terms.
//
// No image, shape or table fragment: Word's box model frames a picture without laying its content
// out (the three picture losses above), and the table reaches page 0 as boxes and lines rather than
// as a `Fragment::Table`. Both are what RC20 (MJXOFF-314) and RC11 (MJXOFF-306) change.
const EXPECTED_FRAGMENTS: [usize; 6] = [41, 25, 23, 0, 0, 0];

// The bundled faces only, so the layout does not depend on what the machine has installed. Word's
// pagination is emergent — one substituted face moves every page boundary in the document — so this
// matters more here than in either other journey.
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

// The repository root, from this crate's manifest directory.
fn repository_root() -> PathBuf {
    mjx_fixtures::workspace_root()
}

// What the corporate document's first page produced, at the last stage Word reaches.
struct Journey {
    tree: FragmentTree,
    losses: LayoutLosses,
}

// Lays page 0 of the corporate document out.
fn journey() -> Journey {
    let bytes = mjx_fixtures::fixture(FIXTURE);
    let mut document = Document::open(&bytes).expect("the corporate document opens");
    let flow = DocumentFlow::read(&mut document).expect("the document resolves");
    let section = flow
        .formatting()
        .sections()
        .first()
        .expect("the corporate document states at least one section");
    let constraints = constraints_for(section);

    let mut model = DocumentBoxModel::new(resolver());
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints, None)
        .expect("the first page lays out");
    let losses = page.losses().clone();
    let (tree, _) = page.into_parts();
    Journey { tree, losses }
}

// Layout losses as the nine-kind vector, with the whole set checked against the total so a loss of
// a kind this vector does not name cannot hide in it.
fn layout_vector(losses: &LayoutLosses) -> [usize; 9] {
    use mjx_layout::FrameContent;
    let counts = [
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Chart),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Diagram),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::EmbeddedObject),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Picture),
        LayoutLossKind::FrameContentNotLaidOut(FrameContent::Shape),
        LayoutLossKind::TextMeasuredNotShaped,
        LayoutLossKind::DroppedByReader,
        LayoutLossKind::ValueApproximated,
    ]
    .map(|kind| losses.count(kind));
    assert_eq!(
        counts.iter().sum::<usize>(),
        losses.len(),
        "a layout loss of a kind this vector does not name"
    );
    counts
}

// How many fragments of each kind a tree holds.
fn fragment_vector(tree: &FragmentTree) -> [usize; 6] {
    let mut counts = [0; 6];
    for (_, node) in tree.nodes() {
        match node.fragment() {
            Fragment::Box(_) => counts[0] += 1,
            Fragment::Line(_) => counts[1] += 1,
            Fragment::GlyphRun(_) => counts[2] += 1,
            Fragment::Image(_) => counts[3] += 1,
            Fragment::Shape(_) => counts[4] += 1,
            Fragment::Table(_) => counts[5] += 1,
        }
    }
    counts
}

/// The corporate document lays out, and every loss it takes on the way is named.
#[test]
#[ignore = "acceptance render: run with --ignored (MJXOFF-300)"]
fn the_corporate_document_lays_out_with_its_losses_named() {
    let journey = journey();

    assert_eq!(
        layout_vector(&journey.losses),
        EXPECTED_LAYOUT_LOSSES,
        "the whole loss vector of the last stage Word reaches. Pin this from the first green run; \
         never widen it to make a run pass."
    );
    assert_eq!(
        fragment_vector(&journey.tree),
        EXPECTED_FRAGMENTS,
        "how many fragments of each kind page 0 holds, as [boxes, lines, glyphs, images, shapes, \
         tables]. A loss vector is about a tree, and a tree that emptied would satisfy one."
    );

    // The plate a person reads, because there are no pixels to look at.
    let mut text = String::from(
        "# RC03 acceptance plate: corporate.docx page 0\n\
         # Word has no scene companion, so this fragment tree is the whole render (MJXOFF-300).\n",
    );
    for (id, node) in journey.tree.nodes() {
        let rect = node.rect();
        text.push_str(&format!(
            "{id:?}\t{:?}\t{}\t{}\t{}\t{}\n",
            std::mem::discriminant(node.fragment()),
            rect.left.emu(),
            rect.top.emu(),
            rect.right.emu(),
            rect.bottom.emu(),
        ));
    }
    let path = plate("docx.txt", text.as_bytes());
    assert!(
        path.exists(),
        "the plate must be written for a person to look at"
    );
}

/// **Word cannot reach pixels, and this is where that stops being a sentence.**
///
/// It needs no fixture, so it is live now — and it fails the day RC09 lands `mjx-scene-docx`, which
/// is exactly when this file must grow the display-list and painter halves the other two journeys
/// already have.
#[test]
fn word_has_no_scene_companion() {
    let root = repository_root();
    assert!(
        !root.join("crates/mjx-scene-docx").exists(),
        "`crates/mjx-scene-docx` exists. Word can now reach a display list, so \
         `the_corporate_document_lays_out_with_its_losses_named` must be extended to build one and \
         render it, exactly as the PowerPoint and Excel journeys do."
    );

    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("the workspace manifest is committed");
    assert!(
        !manifest.contains("crates/mjx-scene-docx"),
        "the workspace manifest lists `crates/mjx-scene-docx`; see the assertion above"
    );

    // And the two companions that *do* exist are named, so this test cannot pass by reading a
    // manifest it failed to parse.
    for present in ["crates/mjx-scene-pptx", "crates/mjx-scene-xlsx"] {
        assert!(
            manifest.contains(present),
            "the workspace manifest does not list `{present}`, so this reader is not reading the \
             members list and its negative above proves nothing"
        );
    }
}
