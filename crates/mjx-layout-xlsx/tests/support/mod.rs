//! What every suite here needs: a deterministic font tier, a worksheet built to order, and a
//! snapshot of a fragment tree.
//!
//! # No system fonts, ever
//!
//! A layout gate that resolved faces through the platform would assert one thing on a developer's
//! machine and another on CI, and the failure would look like a layout regression. So the resolver
//! these suites build indexes **only** the faces `mjx-text` commits under `assets/fonts/`, and every
//! fixture asks for a family they cover.
//!
//! # Why a worksheet is authored from markup rather than through the `Workbook` surface
//!
//! Almost every behaviour this crate implements is stated in `xl/styles.xml` — an alignment, a wrap
//! flag, a rotation, a border — and the authoring surface writes cell *values*, not cell *formats*.
//! Building the two parts as bytes and replacing them in a blank package is therefore the shortest
//! honest route to a sheet that says what a suite needs it to say, and it keeps the fixture legible:
//! a test that asserts something about `wrapText` has `wrapText` visible three lines above the
//! assertion.

#![allow(dead_code)]

use std::path::PathBuf;

use mjx_layout::{Fragment, FragmentId, FragmentTree};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_text::FontResolver;
use mjx_xlsx::{PartName, Workbook};

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
pub(crate) fn model() -> SheetBoxModel {
    SheetBoxModel::new(resolver())
}

/// A family every bundled face answers, so a fixture never depends on what is installed.
pub(crate) const FAMILY: &str = "Liberation Sans";

/// A committed fixture's bytes.
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    mjx_fixtures::fixture(name)
}

/// The first worksheet part of a blank workbook.
pub(crate) const SHEET_PART: &str = "/xl/worksheets/sheet1.xml";

/// The styles part of a blank workbook.
pub(crate) const STYLES_PART: &str = "/xl/styles.xml";

/// The `<borders>` block a sheet with no stated borders carries — index 0, and nothing else.
pub(crate) const PLAIN_BORDERS: &str =
    r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#;

/// A `styles.xml` with one font at 11 pt in [`FAMILY`] and the `cellXfs` records `formats` states.
///
/// `formats` is a list of the `xf` element bodies to write after the default record, so index 0 is
/// always the plain one and index 1 is the first the caller asked for.
pub(crate) fn styles(formats: &[&str], borders: &str) -> Vec<u8> {
    let borders = if borders.is_empty() {
        PLAIN_BORDERS
    } else {
        borders
    };
    let mut cell_formats = String::from(r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>"#);
    for format in formats {
        cell_formats.push_str(format);
    }
    let count = formats.len() + 1;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="1"><font><sz val="11"/><name val="{FAMILY}"/></font></fonts>
<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
{borders}
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="{count}">{cell_formats}</cellXfs>
</styleSheet>"#
    )
    .into_bytes()
}

/// A `<worksheet>` part whose body is `body`.
pub(crate) fn worksheet(body: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
{body}
</worksheet>"#
    )
    .into_bytes()
}

/// A workbook whose first sheet is `worksheet_markup` and whose styles are `styles_markup`.
pub(crate) fn workbook(worksheet_markup: &[u8], styles_markup: &[u8]) -> Workbook {
    let bytes = Workbook::blank()
        .expect("a blank workbook")
        .save_unchecked()
        .expect("blank saves");
    let mut package = mjx_xlsx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new(SHEET_PART).expect("a valid part name"),
            worksheet_markup.to_vec(),
        )
        .expect("the worksheet is replaceable");
    package
        .replace_part_bytes(
            &PartName::new(STYLES_PART).expect("a valid part name"),
            styles_markup.to_vec(),
        )
        .expect("the styles part is replaceable");
    Workbook::from_package(package).expect("the authored package resolves")
}

/// The snapshot of the first sheet of a workbook authored from `body` and `styles_markup`.
pub(crate) fn grid_from(body: &str, styles_markup: &[u8]) -> SheetGrid {
    let book = workbook(&worksheet(body), styles_markup);
    SheetGrid::read(&book, 0).expect("the sheet reads")
}

/// The snapshot of the first sheet of `file` in the committed corpus.
pub(crate) fn grid_of(file: &str, sheet: usize) -> SheetGrid {
    let book = Workbook::open(&fixture(file)).unwrap_or_else(|error| panic!("{file}: {error}"));
    SheetGrid::read(&book, sheet).unwrap_or_else(|error| panic!("{file} sheet {sheet}: {error}"))
}

/// A viewport of `width` by `height` inches with no margins.
pub(crate) fn viewport(width: f64, height: f64) -> mjx_layout::Constraints {
    mjx_layout_xlsx::constraints_for(mjx_layout::LayoutSize {
        width: mjx_ooxml_core::measure::Emu::from_inches(width),
        height: mjx_ooxml_core::measure::Emu::from_inches(height),
    })
}

/// Lays out band `page` of `grid`, reached the way a reader reaches it — page by page, through each
/// checkpoint — so that a suite about page two is also a check that resumption works.
pub(crate) fn lay_out(
    model: &mut SheetBoxModel,
    grid: &SheetGrid,
    constraints: &mjx_layout::Constraints,
    page: u32,
) -> FragmentTree {
    use mjx_layout::{BoxModel, Checkpoint, PageFragments, PageIndex};

    let mut resume: Option<Checkpoint> = None;
    let mut laid: Option<PageFragments> = None;
    for number in 0..=page {
        let fragments = model
            .layout_page(grid, PageIndex::new(number), constraints, resume.as_ref())
            .unwrap_or_else(|error| panic!("band {number}: {error}"));
        resume = fragments.continuation().cloned();
        laid = Some(fragments);
    }
    laid.expect("at least one band").into_parts().0
}

/// Every cell box of a tree, as `(row, column, rect)`.
pub(crate) fn cells(tree: &FragmentTree) -> Vec<(u32, u16, mjx_layout::LayoutRect)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) => box_fragment
                .cell
                .map(|cell| (cell.row, cell.column, node.rect())),
            _ => None,
        })
        .collect()
}

/// Every table fragment of a tree, in tree order.
pub(crate) fn tables(tree: &FragmentTree) -> Vec<FragmentId> {
    tree.nodes()
        .filter_map(|(id, node)| matches!(node.fragment(), Fragment::Table(_)).then_some(id))
        .collect()
}

/// Every line fragment of a tree, in tree order.
pub(crate) fn lines(tree: &FragmentTree) -> Vec<(FragmentId, mjx_layout::LayoutRect)> {
    tree.nodes()
        .filter_map(|(id, node)| {
            matches!(node.fragment(), Fragment::Line(_)).then_some((id, node.rect()))
        })
        .collect()
}

/// The lines belonging to one cell, by its source path.
pub(crate) fn lines_of(
    tree: &FragmentTree,
    row: u32,
    column: u16,
) -> Vec<(FragmentId, mjx_layout::LayoutRect)> {
    tree.nodes()
        .filter_map(|(id, node)| {
            if !matches!(node.fragment(), Fragment::Line(_)) {
                return None;
            }
            let segments = node.source().path().segments();
            (segments == [row, u32::from(column)]).then_some((id, node.rect()))
        })
        .collect()
}

/// How far right the drawn text of one cell reaches — the natural line extent bounded by the clip
/// it is drawn under, which is what a reader actually sees.
///
/// A line's own `rect` is deliberately the text's *unclipped* extent (a click past the visible end
/// of a truncated label still belongs to that cell), so a suite asserting what is on screen has to
/// intersect the two.
pub(crate) fn drawn_right(tree: &FragmentTree, row: u32, column: u16) -> i64 {
    tree.nodes()
        .filter_map(|(_, node)| {
            if !matches!(node.fragment(), Fragment::Line(_)) {
                return None;
            }
            if node.source().path().segments() != [row, u32::from(column)] {
                return None;
            }
            let right = node.rect().right;
            let bounded = node
                .clip()
                .and_then(|clip| tree.clip(clip))
                .map_or(right, |clip| right.minimum(clip.right));
            Some(bounded.emu())
        })
        .max()
        .unwrap_or(0)
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
    let mut out = vec![format!(
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
        out.push(format!(
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
    let mut text = out.join("\n");
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
            box_fragment.cell.map_or_else(String::new, |cell| format!(
                " cell=r{}c{} span={}x{}",
                cell.row, cell.column, cell.row_span, cell.column_span
            ))
        ),
        Fragment::Line(line) => format!(
            "baseline={} ascent={} descent={} direction={:?}",
            line.baseline.emu(),
            line.ascent.emu(),
            line.descent.emu(),
            line.base_direction
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
        Fragment::Table(table) => format!(
            "columns={} rows={:?} headers={} continued={} continues={}",
            table.columns,
            table.rows,
            table.header_rows,
            table.continued_from_previous_page,
            table.continues_on_next_page
        ),
    }
}
