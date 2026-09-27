//! What every suite here needs: a deterministic font tier, a workbook built to order, and a band
//! laid out and resolved.
//!
//! # No system fonts, ever
//!
//! A suite that resolved faces through the platform would assert one thing on a developer's machine
//! and another on CI. So the resolver these suites build indexes **only** the faces `mjx-text`
//! commits under `assets/fonts/`.
//!
//! # Why a workbook is authored from markup rather than through the `Workbook` surface
//!
//! Everything this crate translates is stated in `xl/styles.xml` — a fill, a border, a font colour,
//! an indexed palette — and the authoring surface writes cell *values*, not cell *formats*. Building
//! the two parts as bytes and replacing them in a blank package is the shortest honest route to a
//! sheet that says what a suite needs it to say, and it keeps the fixture legible: a test that
//! asserts something about `rgb="80FF0000"` has those bytes visible three lines above the assertion.
//! `crates/mjx-layout-xlsx/tests/support/mod.rs` does the same thing for the same reason.

#![allow(dead_code)]

use std::path::PathBuf;

use mjx_layout::{BoxModel, Constraints, FragmentTree, LayoutSize, PageIndex};
use mjx_layout_xlsx::{constraints_for, SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_scene_xlsx::{SheetPalette, SheetResources};
use mjx_text::FontResolver;
use mjx_xlsx::{PartName, Workbook};

/// A family every bundled face answers.
pub(crate) const FAMILY: &str = "Liberation Sans";

/// The first worksheet part of a blank workbook.
pub(crate) const SHEET_PART: &str = "/xl/worksheets/sheet1.xml";

/// The styles part of a blank workbook.
pub(crate) const STYLES_PART: &str = "/xl/styles.xml";

/// The bundled tier's directory — the faces `mjx-text` commits.
pub(crate) fn resolver() -> FontResolver {
    let fonts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

/// A `styles.xml` whose fills, borders, fonts and `cellXfs` are the caller's.
///
/// Every block is written whole so that a suite can state exactly what it means; the `fonts` block
/// always begins with the Normal font, because `cellXfs[0]` resolves to it and a column's width is
/// quoted in its digits.
pub(crate) fn styles(fonts: &str, fills: &str, borders: &str, formats: &[&str]) -> Vec<u8> {
    let mut cell_formats = String::from(r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>"#);
    for format in formats {
        cell_formats.push_str(format);
    }
    let count = formats.len() + 1;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="9"><font><sz val="11"/><name val="{FAMILY}"/></font>{fonts}</fonts>
<fills count="9"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>{fills}</fills>
<borders count="9"><border><left/><right/><top/><bottom/><diagonal/></border>{borders}</borders>
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

/// A viewport of `width` by `height` inches.
pub(crate) fn viewport(width: f64, height: f64) -> Constraints {
    constraints_for(LayoutSize {
        width: Emu::from_inches(width),
        height: Emu::from_inches(height),
    })
}

/// One band of one sheet, laid out and paired with the resolver that answers its handles.
pub(crate) struct Resolved {
    pub tree: FragmentTree,
    pub resources: SheetResources,
    pub model: SheetBoxModel,
}

/// Lays out the first band of `book`'s sheet `index` and builds the resolver for it.
///
/// The palette comes from the workbook itself — the indexed table out of `styles.xml` and the theme
/// out of `xl/theme/theme1.xml` through [`Workbook::theme_colors`] — which is the division the whole
/// crate is shaped around: the caller holds the package and the resolver holds neither.
pub(crate) fn resolve(book: &mut Workbook, index: usize, constraints: &Constraints) -> Resolved {
    let theme = book.theme_colors().expect("the theme part reads");
    let grid = SheetGrid::read(book, index).expect("the sheet reads");
    let mut model = SheetBoxModel::new(resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, constraints, None)
        .expect("the band lays out");
    let (tree, _) = page.into_parts();

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
    Resolved {
        tree,
        resources,
        model,
    }
}

/// A committed fixture's bytes.
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    mjx_fixtures::fixture(name)
}
