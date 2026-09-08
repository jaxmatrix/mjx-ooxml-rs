//! Every public `&mut self` method of [`mjx_ooxml::Deck`], and what each may do to a `.pptx`.
//!
//! The addresses each call takes are **probed from the fixture** rather than assumed: the surface is
//! its first slide, the shape is the first shape that slide holds, the table is the first graphic
//! frame that frames one, and so on. A fixture offering no address for a call yields
//! [`Step::Skipped`], which is counted and is never a silent pass — a method skipped by every
//! fixture is caught by `NEVER_EXERCISED`.

use mjx_ooxml::{
    ActiveXControlSpec, ActiveXPersistence, CellBorder, CellFormat, CellMargins, Cells,
    CharacterPropertiesSpec, ChartData, ChartKind, ChartLabelScope, DataLabelSpec, Deck,
    DiagramContent, DiagramPartKind, EffectListSpec, Emu, ErrorBarSpec, ErrorBarType,
    ErrorValueType, FillSpec, Geometry, GraphicFrameKind, GuideContext, Hyperlink, IndentLevel,
    LegendPosition, LineSpec, LineWidth, OleObjectData, OleObjectSpec, ParagraphPropertiesSpec,
    PlaceholderType, PresetShapeType, Scene3DSpec, Shape3DSpec, ShapeBounds, ShapeKind, ShapePath,
    Surface, TablePart, TableStyleDefinition, TableStyleFormat, TableStylePart, TargetMode,
    TextAlignment, TextAnchoring, TextDirection, TrendlineKind, TrendlineSpec,
    DEFAULT_PLACEHOLDER_IMAGE,
};

use crate::rules::{
    added, changed, changed_one, changed_up_to_one, removed, Count, Touches, ACTIVEX,
    ACTIVEX_BINARY, ANY_EMBEDDED_CLASS, CHART, CONTENT_TYPES, EMBEDDED_CONTENT_TYPES,
    EMBEDDED_RELATIONSHIPS, EMBEDDED_SHARED_STRINGS, EMBEDDED_SHEET_STYLES, EMBEDDED_WORKBOOK,
    EMBEDDED_WORKBOOK_MAIN, EMBEDDED_WORKSHEET, INKML, JPEG, NOTES_SLIDE, NOTHING, OLE_OBJECT, PNG,
    PRESENTATION, RELATIONSHIPS, SLIDE, SLIDE_LAYOUT, TABLE_STYLES, THEME, VML_DRAWING,
};
use crate::{ran, Api, Report, Step};

/// One registered method.
pub(crate) struct Case {
    /// The method name, which must match the facade's own source.
    pub(crate) method: &'static str,
    /// What it may do.
    pub(crate) touches: Touches,
    /// The call, given a deck and the addresses probed from its fixture.
    pub(crate) call: fn(&mut Deck, &Addresses) -> Step,
}

/// Yields [`Step::Skipped`] when the fixture offers no address.
macro_rules! need {
    ($option:expr) => {
        match $option {
            Some(value) => value,
            None => return Step::Skipped,
        }
    };
}

/// The addresses a case may take, probed from the fixture.
#[derive(Clone)]
pub(crate) struct Addresses {
    /// The surface every surface-addressed call uses: the first slide.
    pub(crate) surface: Surface,
    /// The index of that slide.
    pub(crate) slide: Option<u32>,
    /// A layout index, if the deck has one.
    pub(crate) layout: Option<u32>,
    /// A master index, if the deck has one.
    pub(crate) master: Option<u32>,
    /// The first shape of `surface`, whatever kind it is.
    pub(crate) shape: Option<ShapePath>,
    /// The first `p:sp` — the kind that carries text.
    pub(crate) text_shape: Option<ShapePath>,
    /// A second shape, for the calls that take two.
    pub(crate) second_shape: Option<ShapePath>,
    /// The first graphic frame framing a table.
    pub(crate) table: Option<ShapePath>,
    /// The first graphic frame framing a chart.
    pub(crate) chart: Option<ShapePath>,
    /// The first picture.
    pub(crate) picture: Option<ShapePath>,
    /// The first group.
    pub(crate) group: Option<ShapePath>,
    /// The first frame holding an OLE object.
    pub(crate) ole: Option<ShapePath>,
    /// The first frame holding a SmartArt diagram.
    pub(crate) diagram: Option<ShapePath>,
    /// The shape index of the first InkML content part.
    pub(crate) ink_shape: Option<u32>,
    /// The index of the first ActiveX control.
    pub(crate) activex: Option<u32>,
    /// A media relationship id on `surface`.
    pub(crate) media_rel: Option<String>,
    /// A shape and paragraph that hold at least one field.
    pub(crate) field: Option<(ShapePath, u32)>,
    /// A table cell that is part of a merge, as `(row, column)` of its anchor.
    pub(crate) merged_cell: Option<(u32, u32)>,
    /// The style id `table` names, if it names one.
    pub(crate) table_style_id: Option<String>,
    /// An external relationship, as `(source, id)`.
    pub(crate) external_link: Option<(Option<String>, String)>,
}

impl Addresses {
    /// Reads every address `surface` offers, using the facade's own readers.
    fn probe(deck: &mut Deck, surface: Surface) -> Self {
        let slide = Some(surface.index());
        let master = (deck.master_count() > 0).then_some(0);
        let mut addresses = Self {
            surface,
            slide,
            layout: (deck.layout_count() > 0).then_some(0),
            master,
            shape: None,
            text_shape: None,
            second_shape: None,
            table: None,
            chart: None,
            picture: None,
            group: None,
            ole: None,
            diagram: None,
            ink_shape: None,
            activex: None,
            media_rel: None,
            field: None,
            merged_cell: None,
            table_style_id: None,
            external_link: deck
                .external_links()
                .into_iter()
                .next()
                .map(|link| (link.source, link.id)),
        };

        let shapes = deck.shapes(surface).unwrap_or_default();
        for info in &shapes {
            let path = ShapePath::from(info.index);
            if addresses.shape.is_none() {
                addresses.shape = Some(path.clone());
            } else if addresses.second_shape.is_none() {
                addresses.second_shape = Some(path.clone());
            }
            match info.kind {
                ShapeKind::Shape if addresses.text_shape.is_none() => {
                    addresses.text_shape = Some(path.clone());
                }
                ShapeKind::Picture if addresses.picture.is_none() => {
                    addresses.picture = Some(path.clone());
                }
                ShapeKind::GroupShape if addresses.group.is_none() => {
                    addresses.group = Some(path.clone());
                }
                ShapeKind::GraphicFrame => match deck.graphic_frame_kind(surface, path.clone()) {
                    Ok(Some(GraphicFrameKind::Table)) if addresses.table.is_none() => {
                        addresses.table = Some(path.clone());
                    }
                    Ok(Some(GraphicFrameKind::Chart)) if addresses.chart.is_none() => {
                        addresses.chart = Some(path.clone());
                    }
                    Ok(Some(GraphicFrameKind::OleObject)) if addresses.ole.is_none() => {
                        addresses.ole = Some(path.clone());
                    }
                    Ok(Some(GraphicFrameKind::Diagram)) if addresses.diagram.is_none() => {
                        addresses.diagram = Some(path.clone());
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(table) = addresses.table.clone() {
            addresses.table_style_id = deck.table_style_id(surface, table.clone()).ok().flatten();
            if let Ok((rows, columns)) = deck.table_dimensions(surface, table.clone()) {
                'cells: for row in 0..rows {
                    for column in 0..columns {
                        if deck
                            .cell_span(surface, table.clone(), row, column)
                            .is_ok_and(|(down, across)| down > 1 || across > 1)
                        {
                            addresses.merged_cell = Some((row, column));
                            break 'cells;
                        }
                    }
                }
            }
        }
        // A field lives in some paragraph of some shape, and no fixture puts it in the first of
        // either — probing for it is what makes `paragraph_field_text` an exercised method rather
        // than fifteen refusals.
        'fields: for info in &shapes {
            let path = ShapePath::from(info.index);
            let Ok(paragraphs) = deck.paragraph_count(surface, path.clone()) else {
                continue;
            };
            for paragraph in 0..paragraphs {
                if deck
                    .paragraph_field_count(surface, path.clone(), paragraph)
                    .is_ok_and(|count| count > 0)
                {
                    addresses.field = Some((path, paragraph));
                    break 'fields;
                }
            }
        }
        addresses.ink_shape = deck
            .ink_references(surface)
            .unwrap_or_default()
            .into_iter()
            .find_map(|reference| reference.shape_index);
        addresses.activex = deck
            .activex_control_count(surface)
            .ok()
            .and_then(|count| (count > 0).then_some(0));
        addresses.media_rel = deck
            .media_references(surface)
            .unwrap_or_default()
            .into_iter()
            .find(|reference| !reference.external)
            .map(|reference| reference.rel_id);
        addresses
    }
}

/// The whole `.pptx` surface, run against one fixture.
pub(crate) fn sweep(fixture: &str, before: &[u8], report: &mut Report) {
    let mut probe = match Deck::open(before) {
        Ok(deck) => deck,
        Err(error) => {
            crate::record_unopenable(report, fixture, Api::Deck, &error);
            return;
        }
    };
    if probe.slide_count() == 0 {
        // Every declaration below is written for a *slide* — `changed_one(SLIDE)` and its family.
        // A deck with no slides would have to be swept over a master instead, and every one of
        // those clauses would then name the wrong class. The corpus holds no such deck; saying so
        // here means the day one lands it fails by name rather than quietly proving nothing.
        report.violations.push(format!(
            "{fixture}: a .pptx with no slides — the Deck declarations are written for a slide \
             surface and need a master variant before this fixture can be swept"
        ));
        return;
    }
    // `charts.pptx` keeps its chart on slide 2. Probing only the first slide found no chart in any
    // fixture of the corpus and left twenty-six chart methods exercised by nothing at all, with
    // every count still green — so the sweep runs once per slide.
    for index in 0..probe.slide_count() {
        let surface = Surface::Slide(index);
        let addresses = Addresses::probe(&mut probe, surface);
        for case in cases() {
            let (base, addresses) = prepared(case.method, before, surface, &addresses);
            let mut deck = Deck::open(&base).expect("the prepared package opens");
            let step = (case.call)(&mut deck, &addresses);
            let saved = matches!(step, Step::Ran(_)).then(|| deck.save());
            crate::record(
                report,
                fixture,
                Api::Deck,
                case.method,
                case.touches,
                step,
                &base,
                saved,
            );
        }
    }
}

/// The package a case is measured against, and the addresses it offers.
///
/// # Why a case may edit the fixture before the comparison starts
///
/// A `clear_*` on a shape that has nothing to clear succeeds and changes nothing, and a
/// `remove_chart_trendlines` on a chart that has no trendline does the same. Both are *correct*, and
/// both leave the method proving nothing: the declaration is checked in both directions only when
/// something actually happened. The committed corpus has no shape with a 3-D scene and no chart with
/// a trendline, so more than fifty methods sat in exactly that state.
///
/// So a case may name a **preparation**: an edit made first, saved, and reopened. Its effect lands in
/// the `before` bytes, so the diff still isolates the call under test — the preparation is part of
/// the file the method is handed, not part of what the method did. A preparation that refuses or
/// finds no address leaves the fixture as it was, which is the honest outcome: the method is then
/// simply not applicable here.
fn prepared(
    method: &str,
    before: &[u8],
    surface: Surface,
    addresses: &Addresses,
) -> (Vec<u8>, Addresses) {
    let Some((_, prepare)) = PREPARATIONS.iter().find(|(name, _)| *name == method) else {
        return (before.to_vec(), addresses.clone());
    };
    let mut deck = match Deck::open(before) {
        Ok(deck) => deck,
        Err(_) => return (before.to_vec(), addresses.clone()),
    };
    if !matches!(prepare(&mut deck, addresses), Step::Ran(Ok(()))) {
        return (before.to_vec(), addresses.clone());
    }
    let Ok(base) = deck.save() else {
        return (before.to_vec(), addresses.clone());
    };
    let Ok(mut reopened) = Deck::open(&base) else {
        return (before.to_vec(), addresses.clone());
    };
    let addresses = Addresses::probe(&mut reopened, surface);
    (base, addresses)
}

/// The chart every chart preparation works on: the fixture's own if it has one, a fresh one if not.
fn chart_for_preparation(deck: &mut Deck, a: &Addresses) -> Option<ShapePath> {
    if let Some(chart) = a.chart.clone() {
        return Some(chart);
    }
    deck.add_chart(a.surface, &chart_data(), bounds())
        .ok()
        .map(ShapePath::from)
}

/// Leaves the first text shape's first paragraph holding **two adjacent runs a coalesce can merge**
/// — the state no committed fixture is in, and the one both `coalesce_*` methods need to do
/// anything (MJXOFF-233).
///
/// It takes two edits, and the order matters. Formatting the first character alone *splits* the
/// paragraph's opening run in two — that is what `set_text_range_properties` does — and then giving
/// the whole shape one spec makes the two halves carry identical `a:rPr`. Restyling alone was what
/// this preparation used to do, and it left one run per paragraph with nothing to merge, which is
/// why both methods sat in `NEVER_EXERCISED` until now.
fn two_mergeable_runs(
    deck: &mut Deck,
    surface: Surface,
    shape: ShapePath,
) -> Result<(), mjx_ooxml::Error> {
    deck.set_text_range_properties(surface, shape.clone(), 0, 0..1, &characters())?;
    deck.set_shape_run_properties(surface, shape, &characters())
}

/// The edits that put a fixture into the state a method needs. See [`prepared`].
#[allow(
    clippy::type_complexity,
    reason = "a table of (name, edit) is what a lookup by method name is"
)]
const PREPARATIONS: &[(&str, fn(&mut Deck, &Addresses) -> Step)] = &[
    ("coalesce_paragraph_runs", |deck, a| {
        ran!(two_mergeable_runs(
            deck,
            a.surface,
            need!(a.text_shape.clone())
        ))
    }),
    ("coalesce_shape_runs", |deck, a| {
        ran!(two_mergeable_runs(
            deck,
            a.surface,
            need!(a.text_shape.clone())
        ))
    }),
    ("drop_chart_dangling_decoration", |deck, a| {
        // A decoration dangles when the point it is anchored to stops existing: format point 1,
        // then shorten the series to one point.
        let chart = need!(chart_for_preparation(deck, a));
        if deck
            .set_chart_point_fill(a.surface, chart.clone(), 0, 1, &fill())
            .is_err()
        {
            return Step::Skipped;
        }
        ran!(deck.set_chart_series_values(a.surface, chart, 0, &[9.0]))
    }),
    ("clear_cell_border", |deck, a| {
        ran!(deck.set_cell_border(
            a.surface,
            need!(a.table.clone()),
            0,
            0,
            CellBorder::Bottom,
            &line()
        ))
    }),
    ("clear_cell_fill", |deck, a| {
        ran!(deck.set_cell_fill(a.surface, need!(a.table.clone()), 0, 0, &fill()))
    }),
    ("clear_shape_3d_properties", |deck, a| {
        ran!(deck.set_shape_3d_properties(a.surface, need!(a.shape.clone()), &Shape3DSpec::new()))
    }),
    ("clear_shape_hyperlink", |deck, a| {
        ran!(deck.set_shape_hyperlink(a.surface, need!(a.shape.clone()), &hyperlink()))
    }),
    ("clear_shape_list_style", |deck, a| {
        ran!(deck.set_shape_list_style_default(
            a.surface,
            need!(a.text_shape.clone()),
            &paragraphs()
        ))
    }),
    ("clear_shape_list_style_default", |deck, a| {
        ran!(deck.set_shape_list_style_default(
            a.surface,
            need!(a.text_shape.clone()),
            &paragraphs()
        ))
    }),
    ("clear_shape_list_style_level", |deck, a| {
        ran!(deck.set_shape_list_style_level(
            a.surface,
            need!(a.text_shape.clone()),
            IndentLevel::new(0).expect("level 0"),
            &paragraphs(),
        ))
    }),
    ("clear_shape_scene_3d", |deck, a| {
        ran!(deck.set_shape_scene_3d(a.surface, need!(a.shape.clone()), &scene_3d()))
    }),
    ("diagram_parts", |deck, a| {
        ran!(deck.add_diagram(
            a.surface,
            &DiagramContent::vertical_list(&["Plan", "Build"]),
            bounds()
        ))
    }),
    ("diagram_relationship_ids", |deck, a| {
        ran!(deck.add_diagram(
            a.surface,
            &DiagramContent::vertical_list(&["Plan", "Build"]),
            bounds()
        ))
    }),
    ("set_diagram_part", |deck, a| {
        ran!(deck.add_diagram(
            a.surface,
            &DiagramContent::vertical_list(&["Plan", "Build"]),
            bounds()
        ))
    }),
    ("ink_part_for_shape", |deck, a| {
        ran!(deck.add_ink(a.surface, INK))
    }),
    ("set_ink_content", |deck, a| {
        ran!(deck.add_ink(a.surface, INK))
    }),
    ("shape_for_ink_part", |deck, a| {
        ran!(deck.add_ink(a.surface, INK))
    }),
    ("picture_image_bytes", |deck, a| {
        ran!(deck.add_picture(a.surface, DEFAULT_PLACEHOLDER_IMAGE, bounds()))
    }),
    ("picture_image_link_target", |deck, a| {
        ran!(deck.add_picture(a.surface, DEFAULT_PLACEHOLDER_IMAGE, bounds()))
    }),
    ("set_picture_image", |deck, a| {
        ran!(deck.add_picture(a.surface, DEFAULT_PLACEHOLDER_IMAGE, bounds()))
    }),
    ("unmerge_cells", |deck, a| {
        ran!(deck.merge_cells(a.surface, need!(a.table.clone()), Cells::row(0)))
    }),
    ("refresh_chart_workbook", |deck, a| {
        need!(chart_for_preparation(deck, a));
        Step::Ran(Ok(()))
    }),
    ("detach_chart_workbook", |deck, a| {
        need!(chart_for_preparation(deck, a));
        Step::Ran(Ok(()))
    }),
    ("remove_chart_data_labels", |deck, a| {
        let chart = need!(chart_for_preparation(deck, a));
        ran!(deck.set_chart_data_labels(
            a.surface,
            chart,
            ChartLabelScope::Series { series_index: 0 },
            &DataLabelSpec::new().value(true),
        ))
    }),
    ("remove_chart_error_bars", |deck, a| {
        let chart = need!(chart_for_preparation(deck, a));
        ran!(deck.set_chart_error_bars(
            a.surface,
            chart,
            0,
            &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
        ))
    }),
    ("remove_chart_point_format", |deck, a| {
        let chart = need!(chart_for_preparation(deck, a));
        ran!(deck.set_chart_point_fill(a.surface, chart, 0, 0, &fill()))
    }),
    ("remove_chart_trendlines", |deck, a| {
        let chart = need!(chart_for_preparation(deck, a));
        ran!(deck.add_chart_trendline(
            a.surface,
            chart,
            0,
            &TrendlineSpec::new(TrendlineKind::Linear)
        ))
    }),
    ("set_chart_trendline", |deck, a| {
        let chart = need!(chart_for_preparation(deck, a));
        ran!(deck.add_chart_trendline(
            a.surface,
            chart,
            0,
            &TrendlineSpec::new(TrendlineKind::Linear)
        ))
    }),
];

// -------------------------------------------------------------------------------------------------
// The arguments every mutating case supplies. Deliberately uniform: the gate is about which parts a
// call touches, not about the values it writes, and a value chosen per fixture would make one
// fixture's diff mean something different from another's.
// -------------------------------------------------------------------------------------------------

/// A solid navy fill.
fn fill() -> FillSpec {
    FillSpec::solid(mjx_ooxml::ColorSpec::Srgb("1F3864".to_owned()))
}

/// A one-point navy line.
fn line() -> LineSpec {
    LineSpec::solid(
        LineWidth::from_points(1.0),
        mjx_ooxml::ColorSpec::Srgb("1F3864".to_owned()),
    )
}

/// Bounds well inside any slide.
fn bounds() -> ShapeBounds {
    ShapeBounds::new(100_000, 100_000, 900_000, 400_000)
}

/// A character-properties spec that states one thing.
fn characters() -> CharacterPropertiesSpec {
    CharacterPropertiesSpec::new().with_bold(true)
}

/// A paragraph-properties spec that states one thing.
fn paragraphs() -> ParagraphPropertiesSpec {
    ParagraphPropertiesSpec::new().with_alignment(TextAlignment::Center)
}

/// The two-series chart every authoring case adds.
fn chart_data() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("North", [1.0, 2.0])
}

/// A hyperlink target that leaves the package.
fn hyperlink() -> Hyperlink {
    Hyperlink::Url("https://example.invalid/".to_owned())
}

/// A second valid image, distinct from [`DEFAULT_PLACEHOLDER_IMAGE`] — a 1×1 red PNG. `set_picture_image`
/// handed the bytes the picture already holds is a legitimate no-op, and a no-op proves the
/// declaration nothing, so the case replaces one image with a *different* one.
const OTHER_IMAGE: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// The InkML an ink case writes.
const INK: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>0 0, 5 9, 11 3</inkml:trace></inkml:ink>"#;

/// A second InkML document, for the same reason [`OTHER_IMAGE`] exists: `set_ink_content` handed the
/// bytes the part already holds changes nothing.
const OTHER_INK: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<inkml:ink xmlns:inkml="http://www.w3.org/2003/InkML"><inkml:trace>3 4, 7 1</inkml:trace></inkml:ink>"#;

/// The VML a legacy-drawing case writes.
#[cfg(feature = "vml")]
const VML: &[u8] = br#"<xml xmlns:v="urn:schemas-microsoft-com:vml"><v:shape id="_x0000_s2050" style="width:1pt;height:1pt"/></xml>"#;

/// A Windows command button's class id — the one ActiveX control this suite authors.
const COMMAND_BUTTON: &str = "{D7053240-CE69-11CD-A777-00DD01143C57}";

/// The style id a table-style case creates.
const NEW_TABLE_STYLE: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";

// -------------------------------------------------------------------------------------------------
// Declarations shared by a family of methods
// -------------------------------------------------------------------------------------------------

/// The commonest declaration on this surface: the addressed slide changes and nothing else does.
const SLIDE_ONLY: Touches = Touches {
    rules: &[changed_one(SLIDE)],
};

/// A chart edit that changes only the chart part.
const CHART_ONLY: Touches = Touches {
    rules: &[changed_one(CHART)],
};

/// A chart **data** edit: the chart part, and the cells of the embedded workbook the chart's own
/// `c:f` names. Nothing else of the producer's workbook may move — that is MJXOFF-208, stated.
const CHART_DATA: Touches = Touches {
    rules: &[
        changed_one(CHART),
        changed_up_to_one(EMBEDDED_WORKBOOK),
        changed(EMBEDDED_WORKSHEET, Count::UpTo(1)),
        changed(EMBEDDED_SHARED_STRINGS, Count::UpTo(1)),
        changed(EMBEDDED_WORKBOOK_MAIN, Count::UpTo(1)),
        changed(EMBEDDED_CONTENT_TYPES, Count::UpTo(1)),
        changed(EMBEDDED_RELATIONSHIPS, Count::UpTo(2)),
        changed(EMBEDDED_SHEET_STYLES, Count::UpTo(1)),
    ],
};

/// The same, for `refresh_chart_workbook`, whose contract differs in one clause: a refresh writes
/// the chart's cached data into the cells its `c:f` names, and where the cached data has not moved
/// it writes the chart part not at all. So the chart is `UpTo(1)` here and `Exactly(1)` for a
/// setter, which is what MJXOFF-208 landed — a no-op refresh stays a no-op in the host's bytes.
const REFRESH_CHART_DATA: Touches = Touches {
    rules: &[
        changed_up_to_one(CHART),
        changed_up_to_one(EMBEDDED_WORKBOOK),
        changed(EMBEDDED_WORKSHEET, Count::UpTo(1)),
        changed(EMBEDDED_SHARED_STRINGS, Count::UpTo(1)),
        changed(EMBEDDED_WORKBOOK_MAIN, Count::UpTo(1)),
        changed(EMBEDDED_CONTENT_TYPES, Count::UpTo(1)),
        changed(EMBEDDED_RELATIONSHIPS, Count::UpTo(2)),
        changed(EMBEDDED_SHEET_STYLES, Count::UpTo(1)),
    ],
};

/// A picture or media part arriving in the package.
const ADDS_AN_IMAGE: Touches = Touches {
    rules: &[
        changed_one(SLIDE),
        added(PNG, Count::UpTo(1)),
        changed(RELATIONSHIPS, Count::UpTo(1)),
        changed(CONTENT_TYPES, Count::UpTo(1)),
    ],
};

// =================================================================================================
// The registry, in the order the facade's own enumeration reports.
// =================================================================================================

/// The cases the `vml` feature adds. `add_vml_drawing` exists only when that feature is on, so it
/// cannot sit in [`CASES`] — and [`crate::enumeration`] applies the same `cfg` when it reads the
/// facade's source, so the two lists agree in both feature modes rather than only in one.
#[cfg(feature = "vml")]
pub(crate) const VML_CASES: &[Case] = &[Case {
    method: "add_vml_drawing",
    touches: Touches {
        rules: &[
            changed(RELATIONSHIPS, Count::UpTo(1)),
            changed(CONTENT_TYPES, Count::UpTo(1)),
            added(VML_DRAWING, Count::UpTo(1)),
        ],
    },
    call: |deck, a| ran!(deck.add_vml_drawing(a.surface, VML)),
}];

/// Every case of this surface, in the feature mode this binary was built in.
pub(crate) fn cases() -> Vec<&'static Case> {
    #[allow(
        unused_mut,
        reason = "the `vml` feature extends this list; with the feature off, nothing does"
    )]
    let mut all: Vec<&'static Case> = CASES.iter().collect();
    #[cfg(feature = "vml")]
    all.extend(VML_CASES.iter());
    all
}

/// Every public `&mut self` method of `Deck` that exists in every feature mode.
pub(crate) const CASES: &[Case] = &[
    Case {
        method: "activex_class_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_class_id(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_control_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_control_count(a.surface)),
    },
    Case {
        method: "activex_control_name",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_control_name(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_control_shape_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_control_shape_id(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_part_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_part_bytes(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_persistence",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_persistence(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_snapshot_image_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_snapshot_image_bytes(a.surface, need!(a.activex))),
    },
    Case {
        method: "activex_state_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.activex_state_bytes(a.surface, need!(a.activex))),
    },
    Case {
        method: "add_activex_control",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(ACTIVEX, Count::UpTo(1)),
                added(ACTIVEX_BINARY, Count::UpTo(1)),
                added(PNG, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(2)),
            ],
        },
        call: |deck, a| {
            ran!(deck.add_activex_control(
                a.surface,
                &ActiveXControlSpec {
                    name: "Button1",
                    class_id: COMMAND_BUTTON,
                    persistence: ActiveXPersistence::Stream,
                    state: Some(b"initial state"),
                    snapshot_image: DEFAULT_PLACEHOLDER_IMAGE,
                },
                bounds(),
            ))
        },
    },
    Case {
        method: "add_chart",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(CHART, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
                added(EMBEDDED_WORKBOOK, Count::UpTo(1)),
                // A workbook this call created arrives whole; see `ANY_EMBEDDED_CLASS`.
                added(ANY_EMBEDDED_CLASS, Count::Any),
                added(THEME, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.add_chart(a.surface, &chart_data(), bounds())),
    },
    Case {
        method: "add_chart_trendline",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.add_chart_trendline(
                a.surface,
                need!(a.chart.clone()),
                0,
                &TrendlineSpec::new(TrendlineKind::Linear),
            ))
        },
    },
    Case {
        method: "add_diagram",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(crate::rules::ANY_CLASS, Count::Any),
                added(RELATIONSHIPS, Count::Any),
            ],
        },
        call: |deck, a| {
            ran!(deck.add_diagram(
                a.surface,
                &DiagramContent::vertical_list(&["Plan", "Build"]),
                bounds(),
            ))
        },
    },
    Case {
        method: "add_image",
        touches: Touches {
            rules: &[
                added(PNG, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.add_image(a.surface, DEFAULT_PLACEHOLDER_IMAGE)),
    },
    Case {
        method: "add_ink",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(INKML, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.add_ink(a.surface, INK)),
    },
    Case {
        method: "add_ole_object",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(PNG, Count::UpTo(1)),
                added(OLE_OBJECT, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.add_ole_object(
                a.surface,
                &OleObjectSpec {
                    prog_id: "Excel.Sheet.12",
                    data: OleObjectData::Linked("file:///elsewhere/book.xlsx"),
                    snapshot_image: DEFAULT_PLACEHOLDER_IMAGE,
                    name: None,
                    show_as_icon: true,
                },
                bounds(),
            ))
        },
    },
    Case {
        method: "add_picture",
        touches: ADDS_AN_IMAGE,
        call: |deck, a| ran!(deck.add_picture(a.surface, DEFAULT_PLACEHOLDER_IMAGE, bounds())),
    },
    Case {
        method: "add_shape",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.add_shape(a.surface, PresetShapeType::Ellipse, bounds())),
    },
    Case {
        method: "add_slide",
        touches: Touches {
            rules: &[
                changed_one(PRESENTATION),
                changed(RELATIONSHIPS, Count::UpTo(2)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(SLIDE_LAYOUT, Count::UpTo(1)),
                added(SLIDE, Count::Exactly(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |deck, _| ran!(deck.add_slide()),
    },
    Case {
        method: "add_slide_from_layout",
        touches: Touches {
            rules: &[
                changed_one(PRESENTATION),
                changed(RELATIONSHIPS, Count::UpTo(2)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(SLIDE_LAYOUT, Count::UpTo(1)),
                added(SLIDE, Count::Exactly(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.add_slide_from_layout(need!(a.layout))),
    },
    Case {
        method: "add_slide_with_text",
        touches: Touches {
            rules: &[
                changed_one(PRESENTATION),
                changed(RELATIONSHIPS, Count::UpTo(2)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(SLIDE_LAYOUT, Count::UpTo(1)),
                added(SLIDE, Count::Exactly(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |deck, _| ran!(deck.add_slide_with_text("Preservation", bounds())),
    },
    Case {
        method: "add_table",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(PRESENTATION, Count::UpTo(1)),
                added(TABLE_STYLES, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.add_table(a.surface, 2, 2, bounds())),
    },
    Case {
        method: "add_text_box",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.add_text_box(a.surface, "Preservation", bounds())),
    },
    Case {
        method: "cell_anchor",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_anchor(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_border",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.cell_border(a.surface, need!(a.table.clone()), 0, 0, CellBorder::Bottom))
        },
    },
    Case {
        method: "cell_end_run_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.cell_end_run_properties(a.surface, need!(a.table.clone()), 0, 0, 0))
        },
    },
    Case {
        method: "cell_fill",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_fill(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_headers",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_headers(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_margins",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_margins(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_paragraph_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_paragraph_count(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_paragraph_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.cell_paragraph_properties(a.surface, need!(a.table.clone()), 0, 0, 0))
        },
    },
    Case {
        method: "cell_paragraph_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_paragraph_text(a.surface, need!(a.table.clone()), 0, 0, 0)),
    },
    Case {
        method: "cell_run_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_run_count(a.surface, need!(a.table.clone()), 0, 0, 0)),
    },
    Case {
        method: "cell_run_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.cell_run_properties(a.surface, need!(a.table.clone()), 0, 0, 0, 0))
        },
    },
    Case {
        method: "cell_run_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_run_text(a.surface, need!(a.table.clone()), 0, 0, 0, 0)),
    },
    Case {
        method: "cell_span",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_span(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_text(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "cell_text_direction",
        touches: NOTHING,
        call: |deck, a| ran!(deck.cell_text_direction(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "chart_axes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_axes(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_dangling_decoration",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_dangling_decoration(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "chart_data_label_tier",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.chart_data_label_tier(
                a.surface,
                need!(a.chart.clone()),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "chart_data_labels",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_data_labels(a.surface, need!(a.chart.clone()), 0, None)),
    },
    Case {
        method: "chart_error_bars",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_error_bars(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "chart_kinds",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_kinds(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_legend",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_legend(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_part_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_part_bytes(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_point_formats",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_point_formats(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "chart_point_label_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_point_label_text(a.surface, need!(a.chart.clone()), 0, 0)),
    },
    Case {
        method: "chart_series",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_series(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_series_fill",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_series_fill(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "chart_series_references",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_series_references(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_style_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_style_id(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_title",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_title(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "chart_trendlines",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_trendlines(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "chart_workbooks",
        touches: NOTHING,
        call: |deck, a| ran!(deck.chart_workbooks(a.surface)),
    },
    Case {
        method: "clear_cell_border",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.clear_cell_border(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                CellBorder::Bottom
            ))
        },
    },
    Case {
        method: "clear_cell_fill",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.clear_cell_fill(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "clear_notes",
        touches: Touches {
            rules: &[
                // Clearing a slide's notes retires the notes slide part with it, which is what
                // PowerPoint itself does — and takes the relationship and the content-type override
                // that reached it. Nothing outside that may move.
                changed_up_to_one(NOTES_SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(NOTES_SLIDE, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.clear_notes(need!(a.slide))),
    },
    Case {
        method: "clear_run_hyperlink",
        touches: Touches {
            rules: &[changed_one(SLIDE), changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| {
            ran!(deck.clear_run_hyperlink(a.surface, need!(a.text_shape.clone()), 0, 0))
        },
    },
    Case {
        method: "clear_shape_3d_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.clear_shape_3d_properties(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "clear_shape_hyperlink",
        touches: Touches {
            rules: &[changed_one(SLIDE), changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| ran!(deck.clear_shape_hyperlink(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "clear_shape_list_style",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.clear_shape_list_style(a.surface, need!(a.text_shape.clone()))),
    },
    Case {
        method: "clear_shape_list_style_default",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.clear_shape_list_style_default(a.surface, need!(a.text_shape.clone())))
        },
    },
    Case {
        method: "clear_shape_list_style_level",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.clear_shape_list_style_level(
                a.surface,
                need!(a.text_shape.clone()),
                IndentLevel::new(0).expect("level 0"),
            ))
        },
    },
    Case {
        method: "clear_shape_scene_3d",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.clear_shape_scene_3d(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "coalesce_paragraph_runs",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.coalesce_paragraph_runs(a.surface, need!(a.text_shape.clone()), 0))
        },
    },
    Case {
        method: "coalesce_shape_runs",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.coalesce_shape_runs(a.surface, need!(a.text_shape.clone()))),
    },
    Case {
        method: "color_map",
        touches: NOTHING,
        call: |deck, a| ran!(deck.color_map(a.surface)),
    },
    Case {
        method: "column_width",
        touches: NOTHING,
        call: |deck, a| ran!(deck.column_width(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "create_table_style",
        touches: Touches {
            rules: &[
                changed(TABLE_STYLES, Count::UpTo(1)),
                changed(PRESENTATION, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(TABLE_STYLES, Count::UpTo(1)),
            ],
        },
        call: |deck, _| ran!(deck.create_table_style(NEW_TABLE_STYLE, "Preservation Style")),
    },
    Case {
        method: "detach_chart_workbook",
        touches: Touches {
            rules: &[
                changed_one(CHART),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(EMBEDDED_WORKBOOK, Count::UpTo(1)),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| ran!(deck.detach_chart_workbook(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "diagram_parts",
        touches: NOTHING,
        call: |deck, a| ran!(deck.diagram_parts(a.surface, need!(a.diagram.clone()))),
    },
    Case {
        method: "diagram_relationship_ids",
        touches: NOTHING,
        call: |deck, a| ran!(deck.diagram_relationship_ids(a.surface, need!(a.diagram.clone()))),
    },
    Case {
        method: "drop_chart_dangling_decoration",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.drop_chart_dangling_decoration(a.surface, need!(a.chart.clone()), 0))
        },
    },
    Case {
        method: "effective_cell_border",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.effective_cell_border(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                CellBorder::Bottom
            ))
        },
    },
    Case {
        method: "effective_cell_fill",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_cell_fill(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "effective_cell_run_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.effective_cell_run_properties(a.surface, need!(a.table.clone()), 0, 0, 0, 0))
        },
    },
    Case {
        method: "effective_paragraph_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.effective_paragraph_properties(a.surface, need!(a.text_shape.clone()), 0))
        },
    },
    Case {
        method: "effective_run_properties",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.effective_run_properties(a.surface, need!(a.text_shape.clone()), 0, 0))
        },
    },
    Case {
        method: "effective_shape_bounds",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_shape_bounds(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "effective_shape_effects",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_shape_effects(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "effective_shape_fill",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_shape_fill(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "effective_shape_outline",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_shape_outline(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "effective_shape_transform",
        touches: NOTHING,
        call: |deck, a| ran!(deck.effective_shape_transform(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "end_run_properties",
        touches: NOTHING,
        call: |deck, a| ran!(deck.end_run_properties(a.surface, need!(a.text_shape.clone()), 0)),
    },
    Case {
        method: "format_cell_paragraphs",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.format_cell_paragraphs(
                a.surface,
                need!(a.table.clone()),
                Cells::all(),
                &paragraphs(),
            ))
        },
    },
    Case {
        method: "format_cell_text",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.format_cell_text(
                a.surface,
                need!(a.table.clone()),
                Cells::all(),
                &characters(),
            ))
        },
    },
    Case {
        method: "format_cells",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.format_cells(
                a.surface,
                need!(a.table.clone()),
                Cells::all(),
                &CellFormat::new().with_fill(fill()),
            ))
        },
    },
    Case {
        method: "format_inline_table_style_part",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.format_inline_table_style_part(
                a.surface,
                need!(a.table.clone()),
                TableStylePart::WholeTable,
                &TableStyleFormat::new().with_fill(fill()),
            ))
        },
    },
    Case {
        method: "format_table_style_part",
        touches: Touches {
            rules: &[changed_up_to_one(TABLE_STYLES)],
        },
        call: |deck, a| {
            ran!(deck.format_table_style_part(
                &need!(a.table_style_id.clone()),
                TableStylePart::WholeTable,
                &TableStyleFormat::new().with_fill(fill()),
            ))
        },
    },
    Case {
        method: "graphic_frame_kind",
        touches: NOTHING,
        call: |deck, a| ran!(deck.graphic_frame_kind(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "group_shapes",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            let first = need!(a.shape.clone());
            let second = need!(a.second_shape.clone());
            ran!(deck.group_shapes(a.surface, &[first, second]))
        },
    },
    Case {
        method: "ink_part_for_shape",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ink_part_for_shape(a.surface, need!(a.ink_shape))),
    },
    Case {
        method: "ink_references",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ink_references(a.surface)),
    },
    Case {
        method: "insert_column",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.insert_column(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "insert_row",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.insert_row(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "layout_kind",
        touches: NOTHING,
        call: |deck, a| ran!(deck.layout_kind(need!(a.layout))),
    },
    Case {
        method: "layout_name",
        touches: NOTHING,
        call: |deck, a| ran!(deck.layout_name(need!(a.layout))),
    },
    Case {
        method: "layouts",
        touches: NOTHING,
        call: |deck, _| ran!(deck.layouts()),
    },
    Case {
        method: "linked_images",
        touches: NOTHING,
        call: |deck, a| ran!(deck.linked_images(a.surface)),
    },
    Case {
        method: "master_name",
        touches: NOTHING,
        call: |deck, a| ran!(deck.master_name(need!(a.master))),
    },
    Case {
        method: "media_references",
        touches: NOTHING,
        call: |deck, a| ran!(deck.media_references(a.surface)),
    },
    Case {
        method: "merge_cells",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.merge_cells(a.surface, need!(a.table.clone()), Cells::row(0))),
    },
    Case {
        method: "merged_cell_anchor",
        touches: NOTHING,
        call: |deck, a| ran!(deck.merged_cell_anchor(a.surface, need!(a.table.clone()), 0, 0)),
    },
    Case {
        method: "move_shape_into_group",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            let shape = need!(a.shape.clone());
            let group = need!(a.group.clone());
            ran!(deck.move_shape_into_group(a.surface, shape, group))
        },
    },
    Case {
        method: "move_shape_out_of_group",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            let member = need!(a.group.clone()).child(0);
            ran!(deck.move_shape_out_of_group(a.surface, member))
        },
    },
    Case {
        method: "notes_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.notes_text(need!(a.slide))),
    },
    Case {
        method: "ole_legacy_shape_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ole_legacy_shape_id(a.surface, need!(a.ole.clone()))),
    },
    Case {
        method: "ole_object_part_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ole_object_part_bytes(a.surface, need!(a.ole.clone()))),
    },
    Case {
        method: "ole_objects",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ole_objects(a.surface)),
    },
    Case {
        method: "ole_prog_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ole_prog_id(a.surface, need!(a.ole.clone()))),
    },
    Case {
        method: "ole_snapshot_image_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.ole_snapshot_image_bytes(a.surface, need!(a.ole.clone()))),
    },
    Case {
        method: "paragraph_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.paragraph_count(a.surface, need!(a.text_shape.clone()))),
    },
    Case {
        method: "paragraph_field_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.paragraph_field_count(a.surface, need!(a.text_shape.clone()), 0)),
    },
    Case {
        method: "paragraph_field_text",
        touches: NOTHING,
        call: |deck, a| {
            let (shape, paragraph) = need!(a.field.clone());
            ran!(deck.paragraph_field_text(a.surface, shape, paragraph, 0))
        },
    },
    Case {
        method: "paragraph_field_type",
        touches: NOTHING,
        call: |deck, a| {
            let (shape, paragraph) = need!(a.field.clone());
            ran!(deck.paragraph_field_type(a.surface, shape, paragraph, 0))
        },
    },
    Case {
        method: "paragraph_properties",
        touches: NOTHING,
        call: |deck, a| ran!(deck.paragraph_properties(a.surface, need!(a.text_shape.clone()), 0)),
    },
    Case {
        method: "paragraph_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.paragraph_text(a.surface, need!(a.text_shape.clone()), 0)),
    },
    Case {
        method: "picture_image_bytes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.picture_image_bytes(a.surface, need!(a.picture.clone()))),
    },
    Case {
        method: "picture_image_link_target",
        touches: NOTHING,
        call: |deck, a| ran!(deck.picture_image_link_target(a.surface, need!(a.picture.clone()))),
    },
    Case {
        method: "presentation_mut",
        touches: NOTHING,
        call: |deck, _| {
            let _ = deck.presentation_mut();
            Step::Ran(Ok(()))
        },
    },
    Case {
        method: "refresh_chart_workbook",
        touches: REFRESH_CHART_DATA,
        call: |deck, a| ran!(deck.refresh_chart_workbook(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "regenerate_chart_workbook",
        touches: Touches {
            rules: &[
                changed_up_to_one(EMBEDDED_WORKBOOK),
                changed(crate::rules::ANY_CLASS, Count::Any),
                added(crate::rules::ANY_CLASS, Count::Any),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| ran!(deck.regenerate_chart_workbook(a.surface, need!(a.chart.clone()))),
    },
    Case {
        method: "remove_activex_control",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| ran!(deck.remove_activex_control(a.surface, need!(a.activex))),
    },
    Case {
        method: "remove_chart_data_labels",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.remove_chart_data_labels(
                a.surface,
                need!(a.chart.clone()),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "remove_chart_error_bars",
        touches: CHART_ONLY,
        call: |deck, a| ran!(deck.remove_chart_error_bars(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "remove_chart_point_format",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.remove_chart_point_format(a.surface, need!(a.chart.clone()), 0, 0))
        },
    },
    Case {
        method: "remove_chart_trendlines",
        touches: CHART_ONLY,
        call: |deck, a| ran!(deck.remove_chart_trendlines(a.surface, need!(a.chart.clone()), 0)),
    },
    Case {
        method: "remove_column",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.remove_column(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "remove_row",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.remove_row(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "remove_shape",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.remove_shape(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "remove_slide",
        touches: Touches {
            rules: &[
                changed_one(PRESENTATION),
                // Any number of *other* slides may change, and any number of `.rels` with them:
                // MJXOFF-212. A slide that hyperlinks to the removed one loses that hyperlink and
                // the relationship naming it, because a package that kept either could never be
                // saved. "As many as name it" is the contract, which is what buys `Any` here — and
                // the presentation part is still pinned at exactly one, so the wildcard cannot hide
                // a sweep over the deck.
                changed(SLIDE, Count::Any),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(SLIDE, Count::Exactly(1)),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| ran!(deck.remove_slide(need!(a.slide))),
    },
    Case {
        method: "remove_unused_parts",
        touches: Touches {
            rules: &[
                // The whole contract is "as many orphans as there are", so the class is a wildcard
                // and the count unbounded. What the gate still says here is that a *sweep* may
                // remove and may not change: an orphan sweep that rewrote a part would fail.
                removed(crate::rules::ANY_CLASS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::Any),
            ],
        },
        call: |deck, _| ran!(deck.remove_unused_parts()),
    },
    Case {
        method: "replace_linked_image_with_placeholder",
        touches: Touches {
            rules: &[
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed_up_to_one(SLIDE),
                added(PNG, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.replace_linked_image_with_placeholder(
                a.surface,
                need!(a.picture.clone()),
                None
            ))
        },
    },
    Case {
        method: "replace_media_with_placeholder",
        touches: Touches {
            rules: &[
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(PNG, Count::UpTo(1)),
                changed(JPEG, Count::UpTo(1)),
                added(PNG, Count::UpTo(1)),
                added(crate::rules::ANY_CLASS, Count::Any),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| {
            ran!(deck.replace_media_with_placeholder(a.surface, &need!(a.media_rel.clone()), None))
        },
    },
    Case {
        method: "replace_ole_object_with_placeholder",
        touches: Touches {
            rules: &[
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(OLE_OBJECT, Count::UpTo(1)),
                added(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| {
            ran!(deck.replace_ole_object_with_placeholder(a.surface, need!(a.ole.clone()), None))
        },
    },
    Case {
        method: "retarget_external_link",
        touches: Touches {
            rules: &[changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| {
            let (source, id) = need!(a.external_link.clone());
            ran!(deck.retarget_external_link(
                source.as_deref(),
                &id,
                "https://example.invalid/moved",
                TargetMode::External,
            ))
        },
    },
    Case {
        method: "row_height",
        touches: NOTHING,
        call: |deck, a| ran!(deck.row_height(a.surface, need!(a.table.clone()), 0)),
    },
    Case {
        method: "run_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.run_count(a.surface, need!(a.text_shape.clone()), 0)),
    },
    Case {
        method: "run_hyperlink",
        touches: NOTHING,
        call: |deck, a| ran!(deck.run_hyperlink(a.surface, need!(a.text_shape.clone()), 0, 0)),
    },
    Case {
        method: "run_properties",
        touches: NOTHING,
        call: |deck, a| ran!(deck.run_properties(a.surface, need!(a.text_shape.clone()), 0, 0)),
    },
    Case {
        method: "run_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.run_text(a.surface, need!(a.text_shape.clone()), 0, 0)),
    },
    Case {
        method: "set_activex_control_name",
        touches: Touches {
            rules: &[changed_up_to_one(SLIDE), changed(ACTIVEX, Count::UpTo(1))],
        },
        call: |deck, a| ran!(deck.set_activex_control_name(a.surface, need!(a.activex), "Renamed")),
    },
    Case {
        method: "set_activex_control_shape_id",
        touches: Touches {
            rules: &[
                changed_up_to_one(SLIDE),
                changed(VML_DRAWING, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.set_activex_control_shape_id(a.surface, need!(a.activex), "_x0000_s2049"))
        },
    },
    Case {
        method: "set_activex_snapshot_image",
        touches: Touches {
            rules: &[
                changed(PNG, Count::UpTo(1)),
                changed(JPEG, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed_up_to_one(SLIDE),
                added(PNG, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.set_activex_snapshot_image(
                a.surface,
                need!(a.activex),
                DEFAULT_PLACEHOLDER_IMAGE
            ))
        },
    },
    Case {
        method: "set_activex_state",
        touches: Touches {
            rules: &[
                changed(ACTIVEX_BINARY, Count::UpTo(1)),
                changed(ACTIVEX, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                added(ACTIVEX_BINARY, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.set_activex_state(a.surface, need!(a.activex), b"state")),
    },
    Case {
        method: "set_cell_anchor",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_anchor(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                TextAnchoring::Center
            ))
        },
    },
    Case {
        method: "set_cell_border",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_border(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                CellBorder::Bottom,
                &line(),
            ))
        },
    },
    Case {
        method: "set_cell_end_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_end_run_properties(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                0,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_cell_fill",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_cell_fill(a.surface, need!(a.table.clone()), 0, 0, &fill())),
    },
    Case {
        method: "set_cell_headers",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_headers(a.surface, need!(a.table.clone()), 0, 0, &["h1"]))
        },
    },
    Case {
        method: "set_cell_margins",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_margins(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                CellMargins::uniform(Emu::from_emu(45_720)),
            ))
        },
    },
    Case {
        method: "set_cell_paragraph_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_paragraph_properties(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                0,
                &paragraphs(),
            ))
        },
    },
    Case {
        method: "set_cell_paragraph_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_paragraph_run_properties(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                0,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_cell_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_run_properties(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                0,
                0,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_cell_run_properties_all",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_run_properties_all(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_cell_text",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_text(a.surface, need!(a.table.clone()), 0, 0, 0, "Preservation"))
        },
    },
    Case {
        method: "set_cell_text_direction",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_text_direction(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                TextDirection::Vertical,
            ))
        },
    },
    Case {
        method: "set_cell_text_range_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_cell_text_range_properties(
                a.surface,
                need!(a.table.clone()),
                0,
                0,
                0,
                0..1,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_chart_axis_gridlines",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_axis_gridlines(a.surface, need!(a.chart.clone()), 0, true, false))
        },
    },
    Case {
        method: "set_chart_axis_orientation",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_axis_orientation(
                a.surface,
                need!(a.chart.clone()),
                0,
                mjx_ooxml::AxisOrientation::MaximumToMinimum,
            ))
        },
    },
    Case {
        method: "set_chart_axis_scale",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_axis_scale(
                a.surface,
                need!(a.chart.clone()),
                0,
                Some(0.0),
                Some(10.0)
            ))
        },
    },
    Case {
        method: "set_chart_axis_title",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_axis_title(a.surface, need!(a.chart.clone()), 0, Some("Quarter")))
        },
    },
    Case {
        method: "set_chart_data_labels",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_data_labels(
                a.surface,
                need!(a.chart.clone()),
                ChartLabelScope::Series { series_index: 0 },
                &DataLabelSpec::new().value(true),
            ))
        },
    },
    Case {
        method: "set_chart_error_bars",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_error_bars(
                a.surface,
                need!(a.chart.clone()),
                0,
                &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
            ))
        },
    },
    Case {
        method: "set_chart_legend",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_legend(
                a.surface,
                need!(a.chart.clone()),
                Some(LegendPosition::Bottom)
            ))
        },
    },
    Case {
        method: "set_chart_point_explosion",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_point_explosion(a.surface, need!(a.chart.clone()), 0, 0, Some(10)))
        },
    },
    Case {
        method: "set_chart_point_fill",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_point_fill(a.surface, need!(a.chart.clone()), 0, 0, &fill()))
        },
    },
    Case {
        method: "set_chart_point_line",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_point_line(a.surface, need!(a.chart.clone()), 0, 0, &line()))
        },
    },
    Case {
        method: "set_chart_series_categories",
        touches: CHART_DATA,
        call: |deck, a| {
            ran!(deck.set_chart_series_categories(
                a.surface,
                need!(a.chart.clone()),
                0,
                &["Alpha", "Beta"]
            ))
        },
    },
    Case {
        method: "set_chart_series_fill",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_series_fill(a.surface, need!(a.chart.clone()), 0, &fill()))
        },
    },
    Case {
        method: "set_chart_series_line",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_series_line(a.surface, need!(a.chart.clone()), 0, &line()))
        },
    },
    Case {
        method: "set_chart_series_values",
        touches: CHART_DATA,
        call: |deck, a| {
            ran!(deck.set_chart_series_values(a.surface, need!(a.chart.clone()), 0, &[41.0, 42.0]))
        },
    },
    Case {
        method: "set_chart_title",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_title(a.surface, need!(a.chart.clone()), Some("Preservation")))
        },
    },
    Case {
        method: "set_chart_trendline",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.set_chart_trendline(
                a.surface,
                need!(a.chart.clone()),
                0,
                0,
                &TrendlineSpec::new(TrendlineKind::Logarithmic),
            ))
        },
    },
    Case {
        method: "set_column_width",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_column_width(
                a.surface,
                need!(a.table.clone()),
                0,
                Emu::from_emu(1_234_567)
            ))
        },
    },
    Case {
        method: "set_diagram_part",
        touches: Touches {
            rules: &[changed(crate::rules::ANY_CLASS, Count::UpTo(1))],
        },
        call: |deck, a| {
            ran!(deck.set_diagram_part(
                a.surface,
                need!(a.diagram.clone()),
                DiagramPartKind::Data,
                b"<dgm/>".to_vec(),
            ))
        },
    },
    Case {
        method: "set_end_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_end_run_properties(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                &characters()
            ))
        },
    },
    Case {
        method: "set_ink_content",
        touches: Touches {
            rules: &[changed(INKML, Count::UpTo(1))],
        },
        call: |deck, a| ran!(deck.set_ink_content(a.surface, need!(a.ink_shape), OTHER_INK)),
    },
    Case {
        method: "set_inline_table_style",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_inline_table_style(
                a.surface,
                need!(a.table.clone()),
                &TableStyleDefinition::new()
                    .with_id(NEW_TABLE_STYLE)
                    .with_name("Preservation Style"),
            ))
        },
    },
    Case {
        method: "set_notes_text",
        touches: Touches {
            rules: &[
                changed_up_to_one(NOTES_SLIDE),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed(PRESENTATION, Count::UpTo(1)),
                added(NOTES_SLIDE, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::Any),
                added(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |deck, a| ran!(deck.set_notes_text(need!(a.slide), "Preservation")),
    },
    Case {
        method: "set_ole_legacy_shape_id",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_ole_legacy_shape_id(a.surface, need!(a.ole.clone()), "_x0000_s2050"))
        },
    },
    Case {
        method: "set_ole_object_data",
        touches: Touches {
            rules: &[
                changed(OLE_OBJECT, Count::UpTo(1)),
                changed(crate::rules::ANY_CLASS, Count::UpTo(1)),
            ],
        },
        call: |deck, a| ran!(deck.set_ole_object_data(a.surface, need!(a.ole.clone()), b"data")),
    },
    Case {
        method: "set_ole_prog_id",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_ole_prog_id(a.surface, need!(a.ole.clone()), "Word.Document.12"))
        },
    },
    Case {
        method: "set_ole_snapshot_image",
        touches: Touches {
            rules: &[
                changed(PNG, Count::UpTo(1)),
                changed(JPEG, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed_up_to_one(SLIDE),
                added(PNG, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.set_ole_snapshot_image(
                a.surface,
                need!(a.ole.clone()),
                DEFAULT_PLACEHOLDER_IMAGE
            ))
        },
    },
    Case {
        method: "set_paragraph_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_paragraph_properties(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                &paragraphs()
            ))
        },
    },
    Case {
        method: "set_paragraph_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_paragraph_run_properties(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_picture_image",
        touches: Touches {
            rules: &[
                changed(PNG, Count::UpTo(1)),
                changed(JPEG, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                changed_up_to_one(SLIDE),
                added(PNG, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.set_picture_image(a.surface, need!(a.picture.clone()), OTHER_IMAGE))
        },
    },
    Case {
        method: "set_row_height",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_row_height(a.surface, need!(a.table.clone()), 0, Emu::from_emu(654_321)))
        },
    },
    Case {
        method: "set_run_hyperlink",
        touches: Touches {
            rules: &[changed_one(SLIDE), changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| {
            ran!(deck.set_run_hyperlink(a.surface, need!(a.text_shape.clone()), 0, 0, &hyperlink()))
        },
    },
    Case {
        method: "set_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_run_properties(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                0,
                &characters()
            ))
        },
    },
    Case {
        method: "set_shape_3d_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_3d_properties(
                a.surface,
                need!(a.shape.clone()),
                &Shape3DSpec::new()
            ))
        },
    },
    Case {
        method: "set_shape_bounds",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_bounds(a.surface, need!(a.shape.clone()), bounds())),
    },
    Case {
        method: "set_shape_effects",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_effects(a.surface, need!(a.shape.clone()), &EffectListSpec::new()))
        },
    },
    Case {
        method: "set_shape_fill",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_fill(a.surface, need!(a.shape.clone()), &fill())),
    },
    Case {
        method: "set_shape_geometry",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_geometry(
                a.surface,
                need!(a.text_shape.clone()),
                Geometry::Preset(mjx_ooxml::ShapeGeometry::RoundedRectangle {
                    corner_radius: mjx_ooxml::Fraction::from_ratio(0.1),
                }),
            ))
        },
    },
    Case {
        method: "set_shape_hyperlink",
        touches: Touches {
            rules: &[changed_one(SLIDE), changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| {
            ran!(deck.set_shape_hyperlink(a.surface, need!(a.shape.clone()), &hyperlink()))
        },
    },
    Case {
        method: "set_shape_list_style_default",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_list_style_default(
                a.surface,
                need!(a.text_shape.clone()),
                &paragraphs()
            ))
        },
    },
    Case {
        method: "set_shape_list_style_level",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_list_style_level(
                a.surface,
                need!(a.text_shape.clone()),
                IndentLevel::new(0).expect("level 0"),
                &paragraphs(),
            ))
        },
    },
    Case {
        method: "set_shape_no_effects",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_no_effects(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "set_shape_no_fill",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_no_fill(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "set_shape_no_outline",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_no_outline(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "set_shape_outline",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.set_shape_outline(a.surface, need!(a.shape.clone()), &line())),
    },
    Case {
        method: "set_shape_run_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_run_properties(
                a.surface,
                need!(a.text_shape.clone()),
                &characters()
            ))
        },
    },
    Case {
        method: "set_shape_scene_3d",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_scene_3d(a.surface, need!(a.shape.clone()), &scene_3d()))
        },
    },
    Case {
        method: "set_shape_text",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_text(a.surface, need!(a.text_shape.clone()), 0, "Preservation"))
        },
    },
    Case {
        method: "set_shape_text_content",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_shape_text_content(
                a.surface,
                need!(a.text_shape.clone()),
                "Preservation"
            ))
        },
    },
    Case {
        method: "set_shape_transform",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            let shape = need!(a.shape.clone());
            let mut transform = match deck.shape_transform(a.surface, shape.clone()) {
                Ok(Some(transform)) => transform,
                Ok(None) => return Step::Skipped,
                Err(error) => return Step::Ran(Err(error)),
            };
            // Written back unchanged this is a no-op by construction, and a no-op proves the
            // declaration nothing. Flipping the shape makes it an edit.
            transform.flip_horizontal = Some(!transform.flip_horizontal.unwrap_or(false));
            ran!(deck.set_shape_transform(a.surface, shape, &transform))
        },
    },
    Case {
        method: "set_table_part",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_table_part(a.surface, need!(a.table.clone()), TablePart::FirstRow, true))
        },
    },
    Case {
        method: "set_table_style",
        touches: Touches {
            rules: &[
                changed_one(SLIDE),
                changed(TABLE_STYLES, Count::UpTo(1)),
                changed(PRESENTATION, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(TABLE_STYLES, Count::UpTo(1)),
            ],
        },
        call: |deck, a| {
            ran!(deck.set_table_style(a.surface, need!(a.table.clone()), NEW_TABLE_STYLE))
        },
    },
    Case {
        method: "set_text_range_hyperlink",
        touches: Touches {
            rules: &[changed_one(SLIDE), changed(RELATIONSHIPS, Count::UpTo(1))],
        },
        call: |deck, a| {
            ran!(deck.set_text_range_hyperlink(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                0..1,
                &hyperlink(),
            ))
        },
    },
    Case {
        method: "set_text_range_properties",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_text_range_properties(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                0..1,
                &characters(),
            ))
        },
    },
    Case {
        method: "set_text_range_properties_by_grapheme",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            ran!(deck.set_text_range_properties_by_grapheme(
                a.surface,
                need!(a.text_shape.clone()),
                0,
                0..1,
                &characters(),
            ))
        },
    },
    Case {
        method: "shape_3d_properties",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_3d_properties(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_adjustments",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.shape_adjustments(
                a.surface,
                need!(a.shape.clone()),
                GuideContext::from_extents(Emu::from_emu(900_000), Emu::from_emu(400_000)),
            ))
        },
    },
    Case {
        method: "shape_bounds",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_bounds(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_count(a.surface)),
    },
    Case {
        method: "shape_effects",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_effects(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_fill",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_fill(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_for_ink_part",
        touches: NOTHING,
        call: |deck, a| {
            let part = need!(deck.ink_part_names().first().cloned());
            ran!(deck.shape_for_ink_part(a.surface, &part))
        },
    },
    Case {
        method: "shape_for_placeholder",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_for_placeholder(a.surface, PlaceholderType::Title)),
    },
    Case {
        method: "shape_geometry",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_geometry(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_hyperlink",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_hyperlink(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_kind",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_kind(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_list_style_default",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_list_style_default(a.surface, need!(a.text_shape.clone()))),
    },
    Case {
        method: "shape_list_style_level",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.shape_list_style_level(
                a.surface,
                need!(a.text_shape.clone()),
                IndentLevel::new(0).expect("level 0"),
            ))
        },
    },
    Case {
        method: "shape_member_count",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_member_count(a.surface, need!(a.group.clone()))),
    },
    Case {
        method: "shape_outline",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_outline(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_placeholder",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_placeholder(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_scene_3d",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_scene_3d(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shape_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_text(a.surface, need!(a.text_shape.clone()))),
    },
    Case {
        method: "shape_transform",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shape_transform(a.surface, need!(a.shape.clone()))),
    },
    Case {
        method: "shapes",
        touches: NOTHING,
        call: |deck, a| ran!(deck.shapes(a.surface)),
    },
    Case {
        method: "slide_size",
        touches: NOTHING,
        call: |deck, _| ran!(deck.slide_size()),
    },
    Case {
        method: "suppress_chart_data_labels",
        touches: CHART_ONLY,
        call: |deck, a| {
            ran!(deck.suppress_chart_data_labels(
                a.surface,
                need!(a.chart.clone()),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "table_dimensions",
        touches: NOTHING,
        call: |deck, a| ran!(deck.table_dimensions(a.surface, need!(a.table.clone()))),
    },
    Case {
        method: "table_part",
        touches: NOTHING,
        call: |deck, a| {
            ran!(deck.table_part(a.surface, need!(a.table.clone()), TablePart::FirstRow))
        },
    },
    Case {
        method: "table_style_id",
        touches: NOTHING,
        call: |deck, a| ran!(deck.table_style_id(a.surface, need!(a.table.clone()))),
    },
    Case {
        method: "theme",
        touches: NOTHING,
        call: |deck, a| ran!(deck.theme(a.surface)),
    },
    Case {
        method: "ungroup",
        touches: SLIDE_ONLY,
        call: |deck, a| ran!(deck.ungroup(a.surface, need!(a.group.clone()))),
    },
    Case {
        method: "unmerge_cells",
        touches: SLIDE_ONLY,
        call: |deck, a| {
            let (row, column) = need!(a.merged_cell);
            ran!(deck.unmerge_cells(a.surface, need!(a.table.clone()), row, column))
        },
    },
    Case {
        method: "visible_cell_text",
        touches: NOTHING,
        call: |deck, a| ran!(deck.visible_cell_text(a.surface, need!(a.table.clone()), 0, 0)),
    },
];

/// The 3-D scene one case sets.
fn scene_3d() -> Scene3DSpec {
    Scene3DSpec {
        camera: mjx_ooxml::Camera {
            preset: mjx_ooxml::PresetCamera::OrthographicFront,
            field_of_view: None,
            zoom: None,
            rotation: None,
        },
        light_rig: mjx_ooxml::LightRig {
            rig: mjx_ooxml::LightRigType::ThreePoint,
            direction: mjx_ooxml::LightRigDirection::Top,
            rotation: None,
        },
    }
}
