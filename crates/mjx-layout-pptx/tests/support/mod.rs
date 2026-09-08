//! What every suite here needs: a deterministic font tier, a deck built to order, and a snapshot of
//! a fragment tree.
//!
//! # No system fonts, ever
//!
//! A layout gate that resolved faces through the platform would assert one thing on a developer's
//! machine and another on CI, and the failure would look like a layout regression. So the resolver
//! these suites build indexes **only** the four metric-compatible faces `mjx-text` commits under
//! `assets/fonts/`, and every fixture asks for a family they cover. That is the same corpus
//! `mjx-render-oracle`'s PDF tier draws from, and it is committed, so a run here is a run anywhere.

#![allow(dead_code)]

use std::path::PathBuf;

use mjx_layout::{Fragment, FragmentId, FragmentTree};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_pptx::{Presentation, ShapeBounds, SlideSize};
use mjx_text::FontResolver;

/// The bundled tier's directory — the faces `mjx-text` commits.
pub(crate) fn bundled_fonts() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts")
}

/// A resolver with the bundled faces and nothing else.
pub(crate) fn resolver() -> FontResolver {
    FontResolver::builder()
        .with_bundled_font_directory(&bundled_fonts())
        .expect("the committed faces index")
        .build()
}

/// A box model over the bundled faces.
pub(crate) fn model() -> SlideBoxModel {
    SlideBoxModel::new(resolver())
}

/// A family every bundled face answers, so a fixture never depends on what is installed.
pub(crate) const FAMILY: &str = "Liberation Sans";

/// A committed fixture's bytes.
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    mjx_fixtures::fixture(name)
}

/// A blank widescreen deck with one slide.
pub(crate) fn blank_deck() -> (Presentation, usize) {
    let mut deck = Presentation::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    (deck, slide)
}

/// A text box on `slide`, at `bounds`, holding `text`.
pub(crate) fn text_box(
    deck: &mut Presentation,
    slide: usize,
    text: &str,
    bounds: ShapeBounds,
) -> usize {
    deck.add_text_box(slide, text, bounds).expect("a text box")
}

/// Reads `deck` and lays out slide `index`, answering the fragment tree.
pub(crate) fn lay_out(
    model: &mut SlideBoxModel,
    deck: &mut Presentation,
    index: usize,
) -> FragmentTree {
    let read = SlideDeck::read(deck).expect("the deck reads");
    lay_out_read(model, &read, index)
}

/// Lays out slide `index` of an already-read deck.
pub(crate) fn lay_out_read(
    model: &mut SlideBoxModel,
    deck: &SlideDeck,
    index: usize,
) -> FragmentTree {
    use mjx_layout::{BoxModel, Checkpoint, PageFragments, PageIndex};

    let constraints = constraints_for(deck);
    let mut resume: Option<Checkpoint> = None;
    let mut page: Option<PageFragments> = None;
    for number in 0..=index {
        let laid = model
            .layout_page(
                deck,
                PageIndex::new(u32::try_from(number).expect("a small deck")),
                &constraints,
                resume.as_ref(),
            )
            .expect("the slide lays out");
        resume = laid.continuation().cloned();
        page = Some(laid);
    }
    page.expect("at least one page").into_parts().0
}

/// Every glyph run of a tree, in tree order, as `(source path, run-local range, glyph count)`.
pub(crate) fn glyph_runs(tree: &FragmentTree) -> Vec<(Vec<u32>, std::ops::Range<u32>, usize)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::GlyphRun(run) => Some((
                node.source().path().segments().to_vec(),
                node.source().characters(),
                run.run.glyphs().len(),
            )),
            _ => None,
        })
        .collect()
}

/// Every line fragment of a tree, in tree order.
pub(crate) fn lines(tree: &FragmentTree) -> Vec<FragmentId> {
    tree.nodes()
        .filter_map(|(id, node)| matches!(node.fragment(), Fragment::Line(_)).then_some(id))
        .collect()
}

/// Every shape fragment of a tree, in tree order.
pub(crate) fn shapes(tree: &FragmentTree) -> Vec<FragmentId> {
    tree.nodes()
        .filter_map(|(id, node)| matches!(node.fragment(), Fragment::Shape(_)).then_some(id))
        .collect()
}

/// The tree as text a person can read and a diff can name a line of — the same shape
/// `mjx-render-oracle`'s tier one uses, written out here because a ranked crate may not depend on
/// the oracle (it links `mjx-paint`, and a test build that links Vulkan is still a build that links
/// Vulkan).
///
/// **A glyph run's line carries its glyph *count* and never a glyph id**, for the reason the oracle
/// gives: a golden file holding glyph ids is a golden file of a shaper version, and a patch release
/// of `swash` would expire every baseline here for a reason that has nothing to do with layout.
pub(crate) fn snapshot(tree: &FragmentTree) -> String {
    let mut lines = vec![format!(
        "fragment-tree nodes={} roots={}",
        tree.len(),
        tree.roots().len()
    )];
    let mut stack: Vec<(FragmentId, usize)> = tree
        .roots()
        .iter()
        .rev()
        .map(|root| (*root, 0_usize))
        .collect();
    while let Some((id, depth)) = stack.pop() {
        let Some(node) = tree.node(id) else { continue };
        let rect = node.rect();
        let path: Vec<String> = node
            .source()
            .path()
            .segments()
            .iter()
            .map(u32::to_string)
            .collect();
        let characters = node.source().characters();
        lines.push(format!(
            "{:indent$}#{} {} rect=[{},{},{},{}] transform={:?} clip={} part={} path=/{} \
             chars={}..{} {}",
            "",
            id.index(),
            node.fragment().kind_name(),
            rect.left.emu(),
            rect.top.emu(),
            rect.right.emu(),
            rect.bottom.emu(),
            node.transform(),
            node.clip()
                .map_or_else(|| "none".to_owned(), |clip| format!("{clip:?}")),
            node.source().part().number(),
            path.join("/"),
            characters.start,
            characters.end,
            detail(node.fragment()),
            indent = depth * 2,
        ));
        let mut children: Vec<FragmentId> = tree.children(id).collect();
        children.reverse();
        for child in children {
            stack.push((child, depth + 1));
        }
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn detail(fragment: &Fragment) -> String {
    match fragment {
        Fragment::Box(box_fragment) => format!(
            "decoration={}{}",
            box_fragment
                .decoration
                .map_or(-1_i128, |handle| i128::from(handle.number())),
            // A cell's coordinates and spans are in the snapshot because a merge that stopped
            // working would otherwise move no line of it: the covered positions simply would not
            // appear, and a baseline of eight cells and one of twelve differ only in length.
            box_fragment.cell.map_or_else(String::new, |cell| format!(
                " cell=r{}c{} span={}x{}",
                cell.row, cell.column, cell.row_span, cell.column_span
            ))
        ),
        Fragment::Line(line) => format!(
            "baseline={} ascent={} descent={} direction={:?} hanging={}",
            line.baseline.emu(),
            line.ascent.emu(),
            line.descent.emu(),
            line.base_direction,
            line.hanging_width.emu()
        ),
        Fragment::GlyphRun(run) => format!(
            "face={} origin=[{},{}] direction={:?} level={:?} glyphs={}",
            run.face.as_u32(),
            run.origin.x.emu(),
            run.origin.y.emu(),
            run.direction,
            run.level,
            run.run.glyphs().len()
        ),
        Fragment::Image(image) => format!("image={}", image.image.number()),
        Fragment::Shape(shape) => format!(
            "geometry={} decoration={}",
            shape.geometry.number(),
            shape
                .decoration
                .map_or(-1_i128, |handle| i128::from(handle.number()))
        ),
        Fragment::Table(table) => format!("columns={} rows={:?}", table.columns, table.rows),
    }
}
