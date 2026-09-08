//! Every name this crate exports is reachable from outside it, and every one of them answers.
//!
//! # Why a suite for this
//!
//! A `pub use` that names a type nobody can *construct* or *inspect* from outside is a surface in
//! name only, and the compiler is perfectly happy with one. Two things go wrong that way and neither
//! shows up in a behavioural suite: a type whose fields are all private with no accessors, and a
//! re-export that quietly stops matching what the module actually offers. So this file names every
//! export, uses it, and reads something back.
//!
//! It is also the gate that would notice a re-export **disappearing**. A consumer above this crate —
//! Excel's scene companion, when it exists — is written against these names, and removing one is a
//! breaking change that should cost a deliberate edit here.

mod support;

use mjx_layout::{BoxModel, Fragment};
use mjx_layout_xlsx::{
    autofit, constraints_for, AutoFit, AutoFitCache, BorderEdge, CellBorders, CellFill, CellHit,
    CellReport, CellRunStyle, CellStyle, ColumnGeometry, ColumnSpan, Decoration, GridGeometry,
    MaximumDigitWidth, MergeIndex, MergedRegion, Overflow, OverflowDirection, PageCatalogue,
    PaneRegion, PaneSplit, PlacedLine, PlacedText, RegionEdge, RowGeometry, RowSpan, SheetBoxModel,
    SheetGrid, SheetLayoutError, TextEngine, Window,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::{
    BorderStyle, HorizontalAlignment, Pane, PaneState, VerticalAlignment,
};

use support::{grid_from, model, styles, viewport};

const SHEET: &str = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/>
<sheetViews><sheetView workbookViewId="0"><pane xSplit="1" ySplit="1" topLeftCell="B2" state="frozen"/></sheetView></sheetViews>
<sheetData>
<row r="1" ht="24" customHeight="true"><c r="A1" t="inlineStr"><is><t>Quarterly revenue by region</t></is></c></row>
<row r="2" hidden="true"><c r="A2"><v>1</v></c></row>
<row r="3" outlineLevel="1"><c r="A3" t="inlineStr"><is><t>Q1</t></is></c><c r="C3"><v>1200</v></c></row>
</sheetData>
<mergeCells count="1"><mergeCell ref="A3:B3"/></mergeCells>"#;

#[test]
fn every_exported_name_is_reachable_and_answers() {
    let grid: SheetGrid = grid_from(SHEET, &styles(&[], ""));
    let constraints = constraints_for(mjx_layout::LayoutSize {
        width: Emu::from_inches(6.0),
        height: Emu::from_inches(3.0),
    });
    assert_eq!(constraints.column_count(), 1);

    // --- the snapshot ------------------------------------------------------------------------
    assert_eq!(grid.index(), 0);
    assert_eq!(grid.part().number(), 0);
    assert!(!grid.name().is_empty());
    assert!(grid.formatting().resolver().is_ok());
    assert!(grid.worksheet().sheet_data().is_some());
    assert!(grid.format_properties().is_some());
    assert!(grid.declared_bounds().is_none(), "the fixture writes none");
    assert_eq!(grid.last_populated_row(), Some(2));
    assert_eq!(grid.used_row_count(), 3);
    assert_eq!(grid.last_populated_column(), Some(2));
    assert_eq!(grid.used_column_count(), 3);
    // A blank workbook carries an (empty) `sharedStrings.xml`; this fixture's cells are inline
    // strings, so the table is present and unused.
    assert!(grid
        .shared_strings()
        .is_none_or(mjx_sml::SharedStringTable::is_empty));
    let cell = grid.cell(0, 0).expect("A1");
    assert_eq!(
        grid.cell_text(&cell).as_deref(),
        Some("Quarterly revenue by region")
    );
    assert!(grid.cell_is_occupied(&cell));
    assert!(grid.row(0).is_some());

    // --- the two axes ------------------------------------------------------------------------
    let rows: &RowGeometry = grid.rows();
    assert_eq!(rows.default_row_height(), Emu::from_points(15.0));
    assert!(!rows.rows_hidden_by_default());
    assert_eq!(rows.height(0), Emu::from_points(24.0));
    assert!(rows.is_hidden(1));
    assert_eq!(rows.outline_level(2), 1);
    assert_eq!(rows.top(0), Emu::ZERO);
    assert_eq!(rows.row_at(Emu::ZERO), Some(0));
    assert_eq!(rows.next_visible_row(1), Some(2));
    let span: &RowSpan = rows.span(0).expect("row 1 states a height");
    assert_eq!(span.row, 0);
    assert!(span.bottom() > span.top);
    assert!(!rows.spans().is_empty());

    let digit = MaximumDigitWidth::ASSUMED;
    assert_eq!(digit.pixels(), 7);
    let columns: ColumnGeometry =
        ColumnGeometry::read(grid.worksheet(), grid.format_properties(), digit).expect("columns");
    assert!(columns.default_column_width() > Emu::ZERO);
    assert!(columns.spans().is_empty(), "the fixture writes no `col`");
    assert!(columns.span(0).is_none());
    assert!(!columns.is_hidden(0));
    assert_eq!(columns.outline_level(0), 0);
    assert!(!columns.wants_auto_fit(0));
    assert_eq!(columns.left(0), Emu::ZERO);
    assert_eq!(columns.column_at(Emu::ZERO), Some(0));
    assert_eq!(columns.next_visible_column(0), Some(0));
    let _: Option<&ColumnSpan> = columns.span(0);

    let geometry = GridGeometry::new(
        RowGeometry::read(grid.worksheet(), grid.format_properties()),
        columns,
        digit,
    );
    assert_eq!(geometry.maximum_digit_width(), digit);
    assert!(geometry.rows().default_row_height() > Emu::ZERO);
    assert!(geometry.columns().default_column_width() > Emu::ZERO);
    assert!(geometry.cell_rect(0, 0).width() > Emu::ZERO);
    assert!(geometry.block_rect(0, 0, 0, 1).width() > geometry.cell_rect(0, 0).width());

    // --- merges ------------------------------------------------------------------------------
    let merges: &MergeIndex = grid.merges();
    assert_eq!(merges.len(), 1);
    assert!(!merges.is_empty());
    assert_eq!(merges.regions().len(), 1);
    let region: MergedRegion = merges.covering(2, 1).expect("B3 is covered");
    assert!(region.contains(2, 0) && !region.is_anchor(2, 1));
    assert_eq!(region.row_span(), 1);
    assert_eq!(region.column_span(), 2);
    assert!(region.anchor().is_ok());
    assert_eq!(region.edge_cells(RegionEdge::Top).len(), 2);
    assert_eq!(RegionEdge::ALL.len(), 4);
    assert_eq!(merges.overlapping(0..3).len(), 1);

    // --- panes -------------------------------------------------------------------------------
    let split: &PaneSplit = grid.split();
    assert!(split.is_divided() && split.divides_rows() && split.divides_columns());
    assert_eq!(split.state, PaneState::Frozen);
    assert_eq!(split.active, Pane::TopLeft);
    assert_ne!(*split, PaneSplit::default());
    let window = Window {
        first_row: 0,
        first_column: 0,
        content: constraints.content,
    };
    let regions: Vec<PaneRegion> = mjx_layout_xlsx::panes::regions(&geometry, split, &window);
    assert!(!regions.is_empty());
    assert!(regions.iter().all(|region| !region.is_empty()));

    // --- the box model -----------------------------------------------------------------------
    let mut model: SheetBoxModel = model();
    assert_eq!(model.signature(), SheetBoxModel::SIGNATURE);
    assert_eq!(model.first_column(), 0);
    model.scroll_to_column(3);
    assert_eq!(model.first_column(), 3);
    model.scroll_to_column(0);
    let _ = model.fonts().manifest().substitutions().count();
    let _ = model.rasteriser_mut();
    let _ = model.geometry(&grid).expect("the geometry builds");
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    assert!(!tree.is_empty());
    assert_eq!(model.last_page(), mjx_layout::PageIndex::FIRST);

    // --- the catalogue -----------------------------------------------------------------------
    let catalogue: &PageCatalogue = model.catalogue();
    assert!(catalogue.decoration_count() > 0);
    assert!(!catalogue.regions().is_empty());
    assert!(!catalogue.rows().is_empty());
    assert!(!catalogue.columns().is_empty());
    assert!(
        catalogue.auto_fits().is_empty(),
        "no column asked to be fitted"
    );
    let report: &CellReport = catalogue
        .cells()
        .first()
        .expect("at least one cell carried text");
    assert!(report.shrink_scale > 0.0);
    assert!(catalogue.cell(report.row, report.column).is_some());
    let handle = tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) => box_fragment.decoration,
            _ => None,
        })
        .expect("a decoration handle");
    let decoration: &Decoration = catalogue.decoration(handle).expect("it resolves");
    let _: &Option<CellFill> = &decoration.fill;
    let _: &CellBorders = &decoration.borders;
    assert!(decoration.borders.is_empty());
    assert!(decoration.borders.edge(RegionEdge::Left).is_none());
    let _ = decoration.font.as_ref();
    let _ = decoration.number_format.as_ref();

    // --- the vocabulary types, constructed directly --------------------------------------------
    let mut borders = CellBorders::default();
    borders.set(
        RegionEdge::Bottom,
        Some(BorderEdge {
            style: BorderStyle::Thin,
            colour: None,
        }),
    );
    assert!(!borders.is_empty());
    let fill = CellFill {
        pattern: None,
        foreground: None,
        background: None,
        is_gradient: true,
    };
    assert!(fill.is_gradient);

    // --- overflow ------------------------------------------------------------------------------
    assert_eq!(
        OverflowDirection::of(HorizontalAlignment::Right),
        Some(OverflowDirection::Left)
    );
    assert_eq!(OverflowDirection::of(HorizontalAlignment::Fill), None);
    let overflow = Overflow::Fits;
    assert_eq!(overflow.columns(4), (4, 4));
    assert!(!overflow.spills() && !overflow.is_clipped());
    assert_eq!(
        mjx_layout_xlsx::overflow::resolve_general(
            mjx_ooxml_types::spreadsheetml::CellType::Number
        ),
        HorizontalAlignment::Right
    );

    // --- cell placement ------------------------------------------------------------------------
    let style = CellStyle::resolve(
        None,
        grid.formatting()
            .resolver()
            .expect("a resolver")
            .formats()
            .interner(),
        None,
        mjx_ooxml_types::spreadsheetml::CellType::SharedString,
    );
    assert_eq!(style.horizontal, HorizontalAlignment::Left);
    assert_eq!(style.vertical, VerticalAlignment::Bottom);
    assert!(!style.is_stacked());
    assert!(style.rotation_angle().is_none());
    let run: &CellRunStyle = &style.run;
    assert!(run.size.in_points() > 0.0);
    assert!(run.scaled(0.5).size.in_points() < run.size.in_points());
    let _ = run.request();
    let empty = PlacedText::empty(geometry.cell_rect(0, 0));
    assert_eq!(empty.height(), Emu::ZERO);
    assert!(empty.lines.is_empty());
    let _: &Vec<PlacedLine> = &empty.lines;
    assert_eq!(
        mjx_layout_xlsx::cell::center_continuous_span(4, Some(1), Some(9)),
        (2, 8)
    );

    // --- auto-fit ------------------------------------------------------------------------------
    let fit: AutoFit = model.auto_fit_width(&grid, 0).expect("A fits");
    assert!(fit.width > Emu::ZERO);
    assert!(fit.characters > 0.0);
    assert!(fit.cells_measured > 0 && fit.sampled_every_cell);
    assert_eq!(
        autofit::MAXIMUM_CELLS_SAMPLED,
        50_000,
        "the sampling guard is stated"
    );
    let mut cache = AutoFitCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    cache.clear();

    // --- addressing ----------------------------------------------------------------------------
    let hit: CellHit = tree
        .nodes()
        .find_map(|(_, node)| CellHit::from_source(node.source()))
        .expect("a cell fragment");
    assert_eq!(hit.sheet, 0);
    assert!(hit.reference().is_ok());
    assert_eq!(
        mjx_layout_xlsx::address::sheet_of(mjx_layout_xlsx::address::part_of(3)),
        3
    );
    assert_eq!(
        mjx_layout_xlsx::address::cell_path(6, 2).segments(),
        [6, 2],
        "the scheme `mjx-session` writes down"
    );
    assert!(mjx_layout_xlsx::address::sheet_path().is_root());
}

#[test]
fn the_error_type_names_every_source_it_can_have() {
    // A `#[non_exhaustive]` enum a caller cannot match on usefully is a surface in name only.
    let error = SheetLayoutError::NoSuchSheet {
        requested: 4,
        count: 1,
    };
    assert!(error.to_string().contains("has no sheet 4"));
    assert!(SheetLayoutError::NotAWorksheet { index: 2 }
        .to_string()
        .contains("sheet 2"));
    assert!(SheetLayoutError::MalformedContinuation(3)
        .to_string()
        .contains("exactly eight"));
}

#[test]
fn a_text_engine_is_constructible_from_outside_the_crate() {
    // `TextEngine` is a struct of four `&mut`, and the reason it is public is that a caller *above*
    // this crate — one that already owns a resolver and a rasteriser — can drive the measurement
    // surface with it. If its fields were private that would be impossible and the export would be
    // decoration.
    let mut fonts = support::resolver();
    let mut rasteriser = mjx_text::GlyphRasteriser::new();
    let mut shaper = mjx_text::Shaper::new();
    let features = mjx_text::FeatureSet::new();
    let engine = TextEngine {
        fonts: &mut fonts,
        rasteriser: &mut rasteriser,
        shaper: &mut shaper,
        features: &features,
    };
    assert!(format!("{engine:?}").contains("TextEngine"));
}

#[test]
fn the_documented_viewport_helper_produces_a_usable_page() {
    let constraints = viewport(6.0, 3.0);
    assert!(constraints.content.height() > Emu::ZERO);
    assert!(constraints.content.width() > Emu::ZERO);
    assert_eq!(constraints.content.left, Emu::ZERO, "no margins");
}
