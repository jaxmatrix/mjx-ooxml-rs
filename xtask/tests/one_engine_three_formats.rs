//! **The same chart, reached through three box models, produces the same geometry** (MJXOFF-178).
//!
//! # Why this one assertion is worth more than three per-format ones
//!
//! R23's whole premise is that a chart in a `.pptx`, a chart in a `.docx` and a chart on an `.xlsx`
//! sheet are *the same chart*, and that laying one out three times would be the largest duplication
//! in the client platform. Three suites each checking that their own format draws a plausible chart
//! would pass on three implementations that quietly disagree — which is the outcome the rank 3.55
//! exists to prevent, and which prose in a module header cannot prevent at all.
//!
//! So this compares. The same [`ChartData`] is authored into a deck, a document and a workbook —
//! independently, by each format's own `add_chart`, which writes its own `c:chartSpace` part — each
//! is laid out by its own box model, and the chart's fragments are compared **shape for shape**.
//!
//! # What is compared, and what deliberately is not
//!
//! Every fragment of the chart's subtree, in tree order, as: its kind, its rectangle **relative to
//! the chart's own frame**, and its decoration and geometry handles.
//!
//! * *Relative* rectangles, because where the chart sits is the **host's** answer and not the
//!   engine's — a slide places a graphic frame by `a:off`, a document floats a drawing against a
//!   column, a worksheet anchors one to a cell — while the *shape* of what is inside it is the one
//!   answer all three must share. A comparison of absolute rectangles would fail for a reason that
//!   has nothing to do with the chart.
//! * The **handles**, because they must match too: all three catalogues number a chart's handles
//!   from the same `CHART_HANDLE_BASE`, precisely so that a chart's fragments do not differ between
//!   hosts for a bookkeeping reason. [`the_three_hosts_number_chart_handles_alike`] asserts that the
//!   three constants really are one number, so this comparison cannot be satisfied by three
//!   coincidences.
//!
//! # Why this lives in `xtask`
//!
//! It names all three format crates and all three box models at once. `mjx-layout-chart`'s own seam
//! gate refuses every format crate in both dependency sections; each box model refuses the other two
//! by name, because an edge between two of them is sideways. The only member that may name all six
//! is the one nothing depends on — see the reason written beside `[dev-dependencies]` in
//! `xtask/Cargo.toml`.

use std::path::PathBuf;

use mjx_chart::{ChartData, ChartKind};
use mjx_docx::{ChartPlacement, Document, PageSize};
use mjx_layout::{
    BoxModel, Constraints, Fragment, FragmentId, FragmentTree, LayoutRect, PageIndex,
};
use mjx_layout_docx::{DocumentBoxModel, DocumentFlow};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Presentation, ShapeBounds, SlideSize};
use mjx_text::FontResolver;
use mjx_xlsx::drawing_geometry::{CellMarker, ResizingBehavior};
use mjx_xlsx::Workbook;

/// The chart every host is handed. Four categories, two series, no explicit series formatting at
/// all — so the palette is the document's own theme and the axis has to be scaled rather than read.
fn specimen() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("North", [13.0, 41.0, 27.0, 34.0])
        .series("South", [22.0, 18.0, 39.0, 11.0])
}

/// The frame every host places the chart in, to the EMU.
///
/// **Excel's answer, handed to the other two.** A worksheet's two-cell anchor sizes itself from the
/// grid — that is what an anchor *is* — so the sheet cannot be told to make a chart four inches
/// wide. A slide and a document can, so the sheet is laid out first and its rectangle is what the
/// other two are given. That way all three frames are the same size without any of them being told
/// a size their own format does not let them honour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Frame {
    width: i64,
    height: i64,
}

/// A resolver over the faces `mjx-text` commits, so nothing here depends on what is installed.
fn resolver() -> FontResolver {
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&bundled)
        .expect("the committed faces index")
        .build()
}

/// One fragment of a chart, in the terms this comparison is made in.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Mark {
    kind: &'static str,
    /// The rectangle, relative to the chart frame's own origin.
    rect: (i64, i64, i64, i64),
    decoration: Option<u64>,
    geometry: Option<u64>,
}

/// Every fragment under `root`, in tree order, relative to `root`'s own rectangle.
fn marks(tree: &FragmentTree, root: FragmentId) -> Vec<Mark> {
    let origin = tree
        .node(root)
        .map(|node| node.rect())
        .unwrap_or(LayoutRect::ZERO);
    let mut out = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(node) = tree.node(id) else { continue };
        let rect = node.rect();
        let (decoration, geometry) = match node.fragment() {
            Fragment::Box(fragment) => (fragment.decoration.map(|handle| handle.number()), None),
            Fragment::Shape(fragment) => (
                fragment.decoration.map(|handle| handle.number()),
                Some(fragment.geometry.number()),
            ),
            _ => (None, None),
        };
        out.push(Mark {
            kind: node.fragment().kind_name(),
            rect: (
                (rect.left - origin.left).emu(),
                (rect.top - origin.top).emu(),
                (rect.right - origin.left).emu(),
                (rect.bottom - origin.top).emu(),
            ),
            decoration,
            geometry,
        });
        // Reverse, so the stack yields children in paint order.
        let children: Vec<FragmentId> = tree.children(id).collect();
        pending.extend(children.into_iter().rev());
    }
    out
}

/// The fragment whose rectangle is the chart's frame — the object the chart was placed in.
///
/// Found by size rather than by address, because the three hosts address their objects differently
/// on purpose: a slide's shape path, a document's paragraph index and a sheet's anchor index are
/// three different vocabularies and none of them is the chart's.
fn chart_root(tree: &FragmentTree, frame: Frame) -> FragmentId {
    tree.ids()
        .find(|id| {
            tree.node(*id).is_some_and(|node| {
                let rect = node.rect();
                (rect.right - rect.left).emu() == frame.width
                    && (rect.bottom - rect.top).emu() == frame.height
            }) && tree.children(*id).count() > 0
        })
        .expect("the chart's frame is in the tree at the size it was placed at")
}

/// PowerPoint: a chart on a slide.
fn from_a_slide(frame: Frame) -> Vec<Mark> {
    let mut deck = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    deck.add_chart(
        slide,
        &specimen(),
        ShapeBounds {
            offset_x_emu: 914_400,
            offset_y_emu: 914_400,
            width_emu: frame.width,
            height_emu: frame.height,
        },
    )
    .expect("the chart is authored");

    let read = SlideDeck::read(&mut deck).expect("the deck reads");
    let constraints = constraints_for(&read);
    let mut model = SlideBoxModel::new(resolver());
    let page = model
        .layout_page(&read, PageIndex::FIRST, &constraints, None)
        .expect("the slide lays out");
    let tree = page.fragments();
    let root = chart_root(tree, frame);
    marks(tree, root)
}

/// Word: a chart floating in a document.
///
/// **Floating rather than inline**, because an inline drawing is a character of a line and the line
/// it sits on decides where it goes; a float resolves against the page's body and lands at a
/// rectangle this test can name. Both paths reach the same engine — see `emit_chart_interior` —
/// and the float is the one whose frame is stable enough to compare.
fn from_a_document(frame: Frame) -> Vec<Mark> {
    let mut document = Document::blank(PageSize::a4()).expect("a blank document");
    document
        .add_chart_placed(
            0usize,
            &specimen(),
            frame.width,
            frame.height,
            "Chart 1",
            ChartPlacement::Floating {
                offset_x_emu: 0,
                offset_y_emu: 0,
                wrap: mjx_docx::ChartWrap::None,
            },
        )
        .expect("the chart is authored");

    let flow = DocumentFlow::read(&mut document).expect("the document reads");
    let constraints = Constraints::single_column(
        mjx_layout::LayoutSize {
            width: Emu::from_inches(8.27),
            height: Emu::from_inches(11.69),
        },
        Emu::ZERO,
    );
    let mut model = DocumentBoxModel::new(resolver());
    let page = model
        .layout_page(&flow, PageIndex::FIRST, &constraints, None)
        .expect("the document lays out");
    let tree = page.fragments();
    let root = chart_root(tree, frame);
    marks(tree, root)
}

/// Excel: a chart anchored on a worksheet, and the frame its anchor gave it.
fn from_a_worksheet() -> (Vec<Mark>, Frame) {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .add_chart(
            0,
            &specimen(),
            CellMarker::new(1, 0, 1, 0),
            CellMarker::new(7, 0, 15, 0),
            "Chart 1",
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .expect("the chart is authored");

    let grid = SheetGrid::read(&workbook, 0).expect("the sheet reads");
    let constraints = Constraints::single_column(
        mjx_layout::LayoutSize {
            width: Emu::from_inches(12.0),
            height: Emu::from_inches(9.0),
        },
        Emu::ZERO,
    );
    let mut model = SheetBoxModel::new(resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, &constraints, None)
        .expect("the sheet lays out");
    let tree = page.fragments();
    // The anchor is whichever box has children and is not the page itself: a blank sheet's grid
    // fragments are leaves, and the chart's frame is the only branch there is.
    // A sheet addresses an anchored object as `[drawing marker, anchor index]`, which is a two
    // segment path no cell has: a cell is `[row, column]` under the grid's own root and a cell
    // fragment has no children. Finding the anchor by its address rather than by its size is what
    // lets this function *discover* the frame instead of being told it.
    let anchor = tree
        .ids()
        .find(|id| {
            tree.node(*id).is_some_and(|node| {
                node.source().path().depth() == 2 && matches!(node.fragment(), Fragment::Box(_))
            }) && tree.children(*id).count() > 0
        })
        .expect("the anchored chart is in the tree");
    let rect = tree
        .node(anchor)
        .map(|node| node.rect())
        .unwrap_or(LayoutRect::ZERO);
    let frame = Frame {
        width: (rect.right - rect.left).emu(),
        height: (rect.bottom - rect.top).emu(),
    };
    (marks(tree, anchor), frame)
}

/// The first difference between two mark lists, as a message a reader can act on.
fn first_difference(left: &[Mark], right: &[Mark]) -> Option<String> {
    if left.len() != right.len() {
        return Some(format!(
            "different fragment counts: {} against {}",
            left.len(),
            right.len()
        ));
    }
    left.iter()
        .zip(right)
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(at, (a, b))| format!("fragment {at}:\n  {a:?}\n  {b:?}"))
}

#[test]
fn one_chart_reaches_three_hosts_and_comes_out_the_same() {
    let (worksheet, frame) = from_a_worksheet();
    assert!(
        frame.width > 0 && frame.height > 0,
        "the worksheet's anchor gave the chart a frame of {frame:?}"
    );
    let slide = from_a_slide(frame);
    let document = from_a_document(frame);

    assert!(
        slide.len() > 20,
        "a two-series bar chart with axes and a legend is more than {} fragments — if this ever \
         collapses, the comparison below has become a comparison of two empty charts",
        slide.len()
    );

    if let Some(difference) = first_difference(&slide, &document) {
        panic!(
            "the same chart laid out differently in a slide and in a Word document.\n\n{difference}\n\n\
             R23's whole premise is that a chart is laid out once. If this fails, either one of the \
             box models has stopped calling `mjx_layout_chart::emit_into`, or one of them has grown \
             a chart implementation of its own."
        );
    }
    if let Some(difference) = first_difference(&slide, &worksheet) {
        panic!(
            "the same chart laid out differently in a slide and on a worksheet.\n\n{difference}\n\n\
             See the message above for what that means."
        );
    }
}

/// The three catalogues number a chart's handles from one base, and the number is written three
/// times because three crates cannot share a constant without one depending on another.
///
/// Without this, [`one_chart_reaches_three_hosts_and_comes_out_the_same`] could be satisfied by
/// three hosts that happen to have issued the same number of their own handles before the chart —
/// which is true today, on a blank page, and would stop being true the moment one of them drew a
/// background.
#[test]
fn the_three_hosts_number_chart_handles_alike() {
    assert_eq!(
        mjx_layout_pptx::PageCatalogue::CHART_HANDLE_BASE,
        mjx_layout_xlsx::PageCatalogue::CHART_HANDLE_BASE,
    );
    assert_eq!(
        mjx_layout_pptx::PageCatalogue::CHART_HANDLE_BASE,
        mjx_layout_docx::DecorationCatalogue::CHART_HANDLE_BASE,
    );
    assert_eq!(
        mjx_layout_pptx::PageCatalogue::CHART_HANDLE_BASE,
        1 << 32,
        "the base has to be above every handle a page of its host's own content can issue"
    );
}

/// The chart really is read by the engine rather than by three host-side readers.
///
/// The three parts were authored independently by three format crates, so they are not byte
/// identical; this asserts that what the engine makes of them is.
#[test]
fn the_three_hosts_read_one_chart_model() {
    let mut deck = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    deck.add_chart(
        slide,
        &specimen(),
        ShapeBounds {
            offset_x_emu: 0,
            offset_y_emu: 0,
            width_emu: 3_657_600,
            height_emu: 2_743_200,
        },
    )
    .expect("authored");
    let from_deck = deck
        .chart_part_bytes(slide, 0usize)
        .expect("readable")
        .map(|bytes| bytes.into_owned())
        .expect("a chart part");

    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .add_chart(
            0,
            &specimen(),
            CellMarker::new(1, 0, 1, 0),
            CellMarker::new(7, 0, 15, 0),
            "Chart 1",
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .expect("authored");
    let anchors = workbook.chart_anchor_indices(0).expect("readable");
    let from_sheet = workbook
        .chart_part_bytes(0, anchors[0])
        .expect("readable")
        .map(|bytes| bytes.into_owned())
        .expect("a chart part");

    let deck_model =
        mjx_layout_chart::ChartModel::read(&from_deck).expect("the deck's chart reads");
    let sheet_model =
        mjx_layout_chart::ChartModel::read(&from_sheet).expect("the sheet's chart reads");
    assert_eq!(
        deck_model, sheet_model,
        "two format crates authored the same chart and the engine read them differently"
    );
    assert_eq!(deck_model.series_count(), 2);
    assert_eq!(deck_model.categories(), vec!["Q1", "Q2", "Q3", "Q4"]);
}
