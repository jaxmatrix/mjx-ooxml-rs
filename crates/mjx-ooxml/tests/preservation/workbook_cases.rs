//! Every public `&mut self` method of [`mjx_ooxml::Workbook`], and what each may do to an `.xlsx`.
//!
//! Excel is where the model for this whole suite came from: `mjx-xlsx/tests/charts.rs`'s
//! `authoring_a_chart_adds_exactly_its_own_parts_and_leaves_every_other_one_byte_identical` states
//! the added set, the changed set and byte identity for everything else, against a producer-written
//! fixture that carries a real theme. That test proves it for one method on one fixture; this file
//! states the same shape for ninety-two methods on every `.xlsx` in the corpus.

use mjx_ooxml::{
    BorderEdgeSpec, BorderSpec, CellFormatSpec, CellFormatTarget, CellInput, CellWrite, ChartData,
    ChartKind, ChartLabelScope, ChartRangeSeries, Color, DataLabelSpec, ErrorBarSpec, ErrorBarType,
    ErrorValueType, FillSpec, FontProperties, LegendPosition, LineSpec, LineWidth, PatternFillSpec,
    ResizingBehavior, TrendlineKind, TrendlineSpec, Workbook,
};

use crate::rules::{
    added, changed, changed_one, changed_up_to_one, removed, Count, Touches, ANY_EMBEDDED_CLASS,
    CALC_CHAIN, CHART, CONTENT_TYPES, DRAWING, EMBEDDED_CONTENT_TYPES, EMBEDDED_RELATIONSHIPS,
    EMBEDDED_SHARED_STRINGS, EMBEDDED_SHEET_STYLES, EMBEDDED_WORKBOOK, EMBEDDED_WORKBOOK_MAIN,
    EMBEDDED_WORKSHEET, NOTHING, PNG, RELATIONSHIPS, SHARED_STRINGS, SHEET_COMMENTS, SHEET_STYLES,
    THEME, VML_DRAWING, WORKBOOK, WORKSHEET,
};
use crate::{ran, Api, Report, Step};

/// One registered method.
pub(crate) struct Case {
    /// The method name, which must match the facade's own source.
    pub(crate) method: &'static str,
    /// What it may do.
    pub(crate) touches: Touches,
    /// The call, given a workbook and the addresses probed from its fixture.
    pub(crate) call: fn(&mut Workbook, &Addresses) -> Step,
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

/// The addresses a case may take.
#[derive(Clone)]
pub(crate) struct Addresses {
    /// A sheet index, if the workbook has one.
    pub(crate) sheet: Option<u32>,
    /// A cell that exists on that sheet, as A1 text.
    pub(crate) cell: Option<String>,
    /// A chart anchor index on that sheet.
    pub(crate) anchor: Option<u32>,
    /// A cell that carries a comment.
    pub(crate) comment_cell: Option<String>,
    /// A cell that carries a hyperlink.
    pub(crate) hyperlink_cell: Option<String>,
    /// A merged range, as A1 text.
    pub(crate) merged_range: Option<String>,
    /// A defined name.
    pub(crate) defined_name: Option<String>,
    /// The number of data-validation rules on the sheet.
    pub(crate) validation_rules: u32,
    /// A drawing-object anchor index on that sheet.
    pub(crate) drawing_object: Option<u32>,
}

impl Addresses {
    /// Reads every address `sheet` offers, using the facade's own readers.
    fn probe(workbook: &mut Workbook, index: u32) -> Self {
        let sheet = Some(index);
        let anchor = workbook
            .chart_anchor_indices(index)
            .unwrap_or_default()
            .into_iter()
            .next();
        Self {
            sheet,
            // `A1` is inside every sheet's address space whether or not the cell is written, which
            // is what a style or a comment case needs — writing one is the point.
            cell: Some("A1".to_owned()),
            anchor,
            comment_cell: workbook
                .sheet_comments(index)
                .unwrap_or_default()
                .first()
                .map(|comment| comment.cell.clone()),
            hyperlink_cell: workbook
                .sheet_hyperlinks(index)
                .unwrap_or_default()
                .first()
                .map(|link| link.range.clone()),
            merged_range: workbook
                .merged_ranges(index)
                .unwrap_or_default()
                .into_iter()
                .next(),
            defined_name: workbook
                .defined_names()
                .unwrap_or_default()
                .first()
                .map(|name| name.name.clone()),
            validation_rules: u32::try_from(
                workbook
                    .data_validation_ranges(index)
                    .unwrap_or_default()
                    .len(),
            )
            .unwrap_or(0),
            drawing_object: anchor,
        }
    }
}

/// The whole `.xlsx` surface, run against one fixture.
pub(crate) fn sweep(fixture: &str, before: &[u8], report: &mut Report) {
    let mut probe = match Workbook::open(before) {
        Ok(workbook) => workbook,
        Err(error) => {
            crate::record_unopenable(report, fixture, Api::Workbook, &error);
            return;
        }
    };
    if probe.sheet_count() == 0 {
        report.violations.push(format!(
            "{fixture}: an .xlsx with no sheets — every declaration below addresses one"
        ));
        return;
    }
    // Once per sheet, for the reason the `.pptx` sweep runs once per slide: a fixture keeps its
    // chart, its comments or its drawing on whichever tab it likes, and probing only the first
    // leaves those methods exercised by nothing while every count stays green.
    for sheet in 0..probe.sheet_count() {
        let addresses = Addresses::probe(&mut probe, sheet);
        for case in cases() {
            let (base, addresses) = prepared(case.method, before, sheet, &addresses);
            let mut workbook = Workbook::open(&base).expect("the prepared package opens");
            let step = (case.call)(&mut workbook, &addresses);
            let saved = matches!(step, Step::Ran(_)).then(|| workbook.save());
            crate::record(
                report,
                fixture,
                Api::Workbook,
                case.method,
                case.touches,
                step,
                &base,
                saved,
            );
        }
    }
}

/// The package a case is measured against, and the addresses it offers. See the `.pptx` sweep's own
/// `prepared` for why a case may edit the fixture before the comparison starts.
fn prepared(
    method: &str,
    before: &[u8],
    sheet: u32,
    addresses: &Addresses,
) -> (Vec<u8>, Addresses) {
    let Some((_, prepare)) = PREPARATIONS.iter().find(|(name, _)| *name == method) else {
        return (before.to_vec(), addresses.clone());
    };
    let Ok(mut workbook) = Workbook::open(before) else {
        return (before.to_vec(), addresses.clone());
    };
    if !matches!(prepare(&mut workbook, addresses), Step::Ran(Ok(()))) {
        return (before.to_vec(), addresses.clone());
    }
    let Ok(base) = workbook.save() else {
        return (before.to_vec(), addresses.clone());
    };
    let Ok(mut reopened) = Workbook::open(&base) else {
        return (before.to_vec(), addresses.clone());
    };
    let addresses = Addresses::probe(&mut reopened, sheet);
    (base, addresses)
}

/// A chart with an embedded workbook on this sheet: the fixture's own if it has one, a fresh one
/// otherwise. The corpus's two sheet charts both draw from a **live range**, so neither carries an
/// embedded workbook and neither can exercise the refresh, regenerate or detach doors.
fn chart_for_preparation(workbook: &mut Workbook, a: &Addresses) -> Option<u32> {
    workbook
        .add_chart(
            a.sheet?,
            &chart_data(),
            1,
            6,
            8,
            18,
            "Revenue",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .ok()
}

/// The edits that put a fixture into the state a method needs.
#[allow(
    clippy::type_complexity,
    reason = "a table of (name, edit) is what a lookup by method name is"
)]
const PREPARATIONS: &[(&str, fn(&mut Workbook, &Addresses) -> Step)] = &[
    ("drop_chart_dangling_decoration", |workbook, a| {
        // A decoration dangles when the point it is anchored to stops existing; see the `.pptx`
        // sweep's own copy.
        let anchor = need!(chart_for_preparation(workbook, a));
        let sheet = need!(a.sheet);
        if workbook
            .set_chart_point_fill(sheet, anchor, 0, 1, &fill())
            .is_err()
        {
            return Step::Skipped;
        }
        ran!(workbook.set_chart_series_values(sheet, anchor, 0, &[9.0]))
    }),
    ("detach_chart_workbook", |workbook, a| {
        need!(chart_for_preparation(workbook, a));
        Step::Ran(Ok(()))
    }),
    ("refresh_chart_workbook", |workbook, a| {
        need!(chart_for_preparation(workbook, a));
        Step::Ran(Ok(()))
    }),
    ("regenerate_chart_workbook", |workbook, a| {
        need!(chart_for_preparation(workbook, a));
        Step::Ran(Ok(()))
    }),
    ("remove_chart_data_labels", |workbook, a| {
        let anchor = need!(chart_for_preparation(workbook, a));
        ran!(workbook.set_chart_data_labels(
            need!(a.sheet),
            anchor,
            ChartLabelScope::Series { series_index: 0 },
            &DataLabelSpec::new().value(true),
        ))
    }),
    ("remove_chart_error_bars", |workbook, a| {
        let anchor = need!(chart_for_preparation(workbook, a));
        ran!(workbook.set_chart_error_bars(
            need!(a.sheet),
            anchor,
            0,
            &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
        ))
    }),
    ("remove_chart_point_format", |workbook, a| {
        let anchor = need!(chart_for_preparation(workbook, a));
        ran!(workbook.set_chart_point_fill(need!(a.sheet), anchor, 0, 0, &fill()))
    }),
    ("remove_chart_trendlines", |workbook, a| {
        let anchor = need!(chart_for_preparation(workbook, a));
        ran!(workbook.add_chart_trendline(
            need!(a.sheet),
            anchor,
            0,
            &TrendlineSpec::new(TrendlineKind::Linear)
        ))
    }),
    ("set_chart_trendline", |workbook, a| {
        let anchor = need!(chart_for_preparation(workbook, a));
        ran!(workbook.add_chart_trendline(
            need!(a.sheet),
            anchor,
            0,
            &TrendlineSpec::new(TrendlineKind::Linear)
        ))
    }),
];

// -------------------------------------------------------------------------------------------------
// Shared arguments and declarations
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

/// The two-series chart every authoring case adds.
fn chart_data() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("North", [1.0, 2.0])
}

/// One worksheet changes and nothing else does.
const WORKSHEET_ONLY: Touches = Touches {
    rules: &[changed_one(WORKSHEET)],
};

/// A chart edit that changes only the chart part.
const CHART_ONLY: Touches = Touches {
    rules: &[changed_one(CHART)],
};

/// A chart **data** edit — MJXOFF-208's declaration, on the Excel surface.
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

/// The same, for `refresh_chart_workbook`: a refresh that finds the cells already holding the
/// chart's cached data writes neither part, so the chart is `UpTo(1)` here — see the Deck file's
/// own copy for the MJXOFF-208 contract this states.
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

/// A chart arriving in a workbook: its own parts, the drawing that anchors it, and — only where the
/// package has no theme at all — the theme its scheme colours resolve against (MJXOFF-200).
const ADDS_A_CHART: Touches = Touches {
    rules: &[
        changed(WORKSHEET, Count::UpTo(1)),
        changed(DRAWING, Count::UpTo(1)),
        changed(RELATIONSHIPS, Count::Any),
        changed(CONTENT_TYPES, Count::UpTo(1)),
        changed(WORKBOOK, Count::UpTo(1)),
        changed(SHEET_STYLES, Count::UpTo(1)),
        added(CHART, Count::UpTo(1)),
        added(DRAWING, Count::UpTo(1)),
        added(RELATIONSHIPS, Count::Any),
        added(EMBEDDED_WORKBOOK, Count::UpTo(1)),
        // A workbook this call created arrives whole; see `ANY_EMBEDDED_CLASS`.
        added(ANY_EMBEDDED_CLASS, Count::Any),
        added(THEME, Count::UpTo(1)),
    ],
};

/// A picture arriving on a sheet.
const ADDS_A_PICTURE: Touches = Touches {
    rules: &[
        changed(WORKSHEET, Count::UpTo(1)),
        changed(DRAWING, Count::UpTo(1)),
        changed(RELATIONSHIPS, Count::Any),
        changed(CONTENT_TYPES, Count::UpTo(1)),
        added(DRAWING, Count::UpTo(1)),
        added(RELATIONSHIPS, Count::Any),
        added(PNG, Count::UpTo(1)),
    ],
};

/// A cell edit: the sheet, plus the two tables a value may land in.
const WRITES_A_CELL: Touches = Touches {
    rules: &[
        changed_one(WORKSHEET),
        changed(SHARED_STRINGS, Count::UpTo(1)),
        changed(RELATIONSHIPS, Count::UpTo(1)),
        changed(CONTENT_TYPES, Count::UpTo(1)),
        changed(WORKBOOK, Count::UpTo(1)),
        changed(CALC_CHAIN, Count::UpTo(1)),
        added(SHARED_STRINGS, Count::UpTo(1)),
    ],
};

/// The two-cell block a range chart draws from.
const RANGE: &str = "Sheet1!$A$1:$A$2";

// =================================================================================================
// The registry
// =================================================================================================

/// Every case of this surface. The `Workbook` surface has no feature-gated method, so this is
/// [`CASES`] itself; it exists so all three surfaces are driven through one shape.
pub(crate) fn cases() -> Vec<&'static Case> {
    CASES.iter().collect()
}

/// Every public `&mut self` method of `Workbook`.
pub(crate) const CASES: &[Case] = &[
    Case {
        method: "active_sheet",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.active_sheet()),
    },
    Case {
        method: "add_absolute_anchored_picture",
        touches: ADDS_A_PICTURE,
        call: |workbook, a| {
            ran!(workbook.add_absolute_anchored_picture(
                need!(a.sheet),
                mjx_ooxml::DEFAULT_PLACEHOLDER_IMAGE,
                "picture",
                0,
                0,
                914_400,
                914_400,
            ))
        },
    },
    Case {
        method: "add_cell_comment",
        touches: Touches {
            rules: &[
                changed(WORKSHEET, Count::UpTo(1)),
                changed(SHEET_COMMENTS, Count::UpTo(1)),
                changed(VML_DRAWING, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHEET_COMMENTS, Count::UpTo(1)),
                added(VML_DRAWING, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::Any),
            ],
        },
        call: |workbook, a| {
            ran!(workbook.add_cell_comment(
                need!(a.sheet),
                &need!(a.cell.clone()),
                "Reviewer",
                "a remark"
            ))
        },
    },
    Case {
        method: "add_chart",
        touches: ADDS_A_CHART,
        call: |workbook, a| {
            ran!(workbook.add_chart(
                need!(a.sheet),
                &chart_data(),
                1,
                6,
                8,
                18,
                "Revenue",
                ResizingBehavior::MoveWithCellsButDoNotResize,
            ))
        },
    },
    Case {
        method: "add_chart_trendline",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.add_chart_trendline(
                need!(a.sheet),
                need!(a.anchor),
                0,
                &TrendlineSpec::new(TrendlineKind::Linear),
            ))
        },
    },
    Case {
        method: "add_one_cell_anchored_picture",
        touches: ADDS_A_PICTURE,
        call: |workbook, a| {
            ran!(workbook.add_one_cell_anchored_picture(
                need!(a.sheet),
                mjx_ooxml::DEFAULT_PLACEHOLDER_IMAGE,
                "picture",
                1,
                0,
                6,
                0,
                914_400,
                914_400,
            ))
        },
    },
    Case {
        method: "add_range_chart",
        touches: ADDS_A_CHART,
        call: |workbook, a| {
            ran!(workbook.add_range_chart(
                need!(a.sheet),
                ChartKind::Bar,
                None,
                &[ChartRangeSeries::new("North", RANGE)],
                1,
                6,
                8,
                18,
                "Revenue",
                ResizingBehavior::MoveWithCellsButDoNotResize,
            ))
        },
    },
    Case {
        method: "add_sheet",
        touches: Touches {
            rules: &[
                changed_one(WORKBOOK),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORKSHEET, Count::Exactly(1)),
                added(RELATIONSHIPS, Count::Any),
            ],
        },
        call: |workbook, _| ran!(workbook.add_sheet("Preservation")),
    },
    Case {
        method: "add_two_cell_anchored_picture",
        touches: ADDS_A_PICTURE,
        call: |workbook, a| {
            ran!(workbook.add_two_cell_anchored_picture(
                need!(a.sheet),
                mjx_ooxml::DEFAULT_PLACEHOLDER_IMAGE,
                "picture",
                1,
                0,
                6,
                0,
                4,
                0,
                12,
                0,
                ResizingBehavior::MoveWithCellsButDoNotResize,
            ))
        },
    },
    Case {
        method: "append_border",
        touches: Touches {
            rules: &[
                changed(SHEET_STYLES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHEET_STYLES, Count::UpTo(1)),
            ],
        },
        call: |workbook, _| {
            ran!(workbook.append_border(&BorderSpec {
                bottom: Some(BorderEdgeSpec::styled(mjx_ooxml::BorderStyle::Thin)),
                ..BorderSpec::default()
            }))
        },
    },
    Case {
        method: "append_cell_format",
        touches: Touches {
            rules: &[
                changed(SHEET_STYLES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHEET_STYLES, Count::UpTo(1)),
            ],
        },
        call: |workbook, _| {
            ran!(workbook.append_cell_format(
                CellFormatTarget::CellFormats,
                &CellFormatSpec::skeleton_cell_format(),
            ))
        },
    },
    Case {
        method: "append_font",
        touches: Touches {
            rules: &[
                changed(SHEET_STYLES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHEET_STYLES, Count::UpTo(1)),
            ],
        },
        call: |workbook, _| {
            ran!(workbook.append_font(&FontProperties {
                bold: Some(true),
                color: Some(Color::from_opaque_rgb("1F3864")),
                ..FontProperties::default()
            }))
        },
    },
    Case {
        method: "append_pattern_fill",
        touches: Touches {
            rules: &[
                changed(SHEET_STYLES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHEET_STYLES, Count::UpTo(1)),
            ],
        },
        call: |workbook, _| {
            ran!(workbook.append_pattern_fill(&PatternFillSpec {
                pattern: Some(mjx_ooxml::SpreadsheetPatternType::Solid),
                foreground: Some(Color::from_opaque_rgb("FFF2CC")),
                background: None,
            }))
        },
    },
    Case {
        method: "calculation_settings",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.calculation_settings()),
    },
    Case {
        method: "chart_anchor_indices",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_anchor_indices(need!(a.sheet))),
    },
    Case {
        method: "chart_axes",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_axes(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_dangling_decoration",
        touches: NOTHING,
        call: |workbook, a| {
            ran!(workbook.chart_dangling_decoration(need!(a.sheet), need!(a.anchor), 0))
        },
    },
    Case {
        method: "chart_data_label_tier",
        touches: NOTHING,
        call: |workbook, a| {
            ran!(workbook.chart_data_label_tier(
                need!(a.sheet),
                need!(a.anchor),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "chart_data_labels",
        touches: NOTHING,
        call: |workbook, a| {
            ran!(workbook.chart_data_labels(need!(a.sheet), need!(a.anchor), 0, None))
        },
    },
    Case {
        method: "chart_error_bars",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_error_bars(need!(a.sheet), need!(a.anchor), 0)),
    },
    Case {
        method: "chart_kinds",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_kinds(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_legend",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_legend(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_part_bytes",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_part_bytes(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_point_formats",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_point_formats(need!(a.sheet), need!(a.anchor), 0)),
    },
    Case {
        method: "chart_point_label_text",
        touches: NOTHING,
        call: |workbook, a| {
            ran!(workbook.chart_point_label_text(need!(a.sheet), need!(a.anchor), 0, 0))
        },
    },
    Case {
        method: "chart_rel_id",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_rel_id(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_series",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_series(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_series_fill",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_series_fill(need!(a.sheet), need!(a.anchor), 0)),
    },
    Case {
        method: "chart_series_freshness",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_series_freshness(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_series_from_cells",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_series_from_cells(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_series_references",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_series_references(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_style_id",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_style_id(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_title",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_title(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "chart_trendlines",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.chart_trendlines(need!(a.sheet), need!(a.anchor), 0)),
    },
    Case {
        method: "chart_workbooks",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.chart_workbooks()),
    },
    Case {
        method: "date_system",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.date_system()),
    },
    Case {
        method: "defined_name",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.defined_name(&need!(a.defined_name.clone()))),
    },
    Case {
        method: "defined_names",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.defined_names()),
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
        call: |workbook, a| ran!(workbook.detach_chart_workbook(need!(a.sheet), need!(a.anchor))),
    },
    Case {
        method: "drop_chart_dangling_decoration",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.drop_chart_dangling_decoration(need!(a.sheet), need!(a.anchor), 0))
        },
    },
    Case {
        method: "insert_columns_into_drawing",
        touches: Touches {
            rules: &[changed(DRAWING, Count::UpTo(1))],
        },
        call: |workbook, a| ran!(workbook.insert_columns_into_drawing(need!(a.sheet), 0, 1)),
    },
    Case {
        method: "insert_rows_into_drawing",
        touches: Touches {
            rules: &[changed(DRAWING, Count::UpTo(1))],
        },
        call: |workbook, a| ran!(workbook.insert_rows_into_drawing(need!(a.sheet), 0, 1)),
    },
    Case {
        method: "intern_shared_string",
        touches: Touches {
            rules: &[
                changed(SHARED_STRINGS, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(SHARED_STRINGS, Count::UpTo(1)),
            ],
        },
        call: |workbook, _| ran!(workbook.intern_shared_string("Preservation")),
    },
    Case {
        method: "merge_cells",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.merge_cells(need!(a.sheet), "A1:B1")),
    },
    Case {
        method: "print_area",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.print_area(need!(a.sheet))),
    },
    Case {
        method: "refresh_chart_cache_from_cells",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.refresh_chart_cache_from_cells(need!(a.sheet), need!(a.anchor)))
        },
    },
    Case {
        method: "refresh_chart_workbook",
        touches: REFRESH_CHART_DATA,
        call: |workbook, a| ran!(workbook.refresh_chart_workbook(need!(a.sheet), need!(a.anchor))),
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
        call: |workbook, a| {
            ran!(workbook.regenerate_chart_workbook(need!(a.sheet), need!(a.anchor)))
        },
    },
    Case {
        method: "remove_auto_filter",
        touches: Touches {
            rules: &[
                changed_up_to_one(WORKSHEET),
                changed(WORKBOOK, Count::UpTo(1)),
            ],
        },
        call: |workbook, a| ran!(workbook.remove_auto_filter(need!(a.sheet))),
    },
    Case {
        method: "remove_cell_comment",
        touches: Touches {
            rules: &[
                changed_up_to_one(SHEET_COMMENTS),
                changed(VML_DRAWING, Count::UpTo(1)),
                changed(WORKSHEET, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(SHEET_COMMENTS, Count::UpTo(1)),
                removed(VML_DRAWING, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::Any),
            ],
        },
        call: |workbook, a| {
            ran!(workbook.remove_cell_comment(need!(a.sheet), &need!(a.comment_cell.clone())))
        },
    },
    Case {
        method: "remove_cell_hyperlink",
        touches: Touches {
            rules: &[
                changed_up_to_one(WORKSHEET),
                changed(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |workbook, a| {
            ran!(workbook.remove_cell_hyperlink(need!(a.sheet), &need!(a.hyperlink_cell.clone())))
        },
    },
    Case {
        method: "remove_chart_data_labels",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.remove_chart_data_labels(
                need!(a.sheet),
                need!(a.anchor),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "remove_chart_error_bars",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.remove_chart_error_bars(need!(a.sheet), need!(a.anchor), 0))
        },
    },
    Case {
        method: "remove_chart_point_format",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.remove_chart_point_format(need!(a.sheet), need!(a.anchor), 0, 0))
        },
    },
    Case {
        method: "remove_chart_trendlines",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.remove_chart_trendlines(need!(a.sheet), need!(a.anchor), 0))
        },
    },
    Case {
        method: "remove_columns_from_drawing",
        touches: Touches {
            rules: &[changed(DRAWING, Count::UpTo(1))],
        },
        call: |workbook, a| ran!(workbook.remove_columns_from_drawing(need!(a.sheet), 0, 1)),
    },
    Case {
        method: "remove_data_validation",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| {
            if a.validation_rules == 0 {
                return Step::Skipped;
            }
            ran!(workbook.remove_data_validation(need!(a.sheet), 0))
        },
    },
    Case {
        method: "remove_rows_from_drawing",
        touches: Touches {
            rules: &[changed(DRAWING, Count::UpTo(1))],
        },
        call: |workbook, a| ran!(workbook.remove_rows_from_drawing(need!(a.sheet), 0, 1)),
    },
    Case {
        method: "remove_sheet_drawing_object",
        touches: Touches {
            rules: &[
                changed(DRAWING, Count::UpTo(1)),
                changed(WORKSHEET, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::Any),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(CHART, Count::UpTo(1)),
                removed(EMBEDDED_WORKBOOK, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::Any),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |workbook, a| {
            ran!(workbook.remove_sheet_drawing_object(need!(a.sheet), need!(a.drawing_object)))
        },
    },
    Case {
        method: "rename_sheet",
        touches: Touches {
            rules: &[
                changed_one(WORKBOOK),
                changed(WORKSHEET, Count::Any),
                changed(CHART, Count::Any),
            ],
        },
        call: |workbook, a| ran!(workbook.rename_sheet(need!(a.sheet), "Preservation")),
    },
    Case {
        method: "resolve_range_reference",
        touches: NOTHING,
        call: |workbook, a| ran!(workbook.resolve_range_reference(need!(a.sheet), "A1:B2")),
    },
    Case {
        method: "set_cell_comment_text",
        touches: Touches {
            rules: &[changed_up_to_one(SHEET_COMMENTS)],
        },
        call: |workbook, a| {
            ran!(workbook.set_cell_comment_text(
                need!(a.sheet),
                &need!(a.comment_cell.clone()),
                "Preservation",
            ))
        },
    },
    Case {
        method: "set_cell_hyperlink_location",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_cell_hyperlink_location(
                need!(a.sheet),
                &need!(a.cell.clone()),
                "Sheet1!B2",
            ))
        },
    },
    Case {
        method: "set_cell_hyperlink_url",
        touches: Touches {
            rules: &[
                changed_one(WORKSHEET),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
            ],
        },
        call: |workbook, a| {
            ran!(workbook.set_cell_hyperlink_url(
                need!(a.sheet),
                &need!(a.cell.clone()),
                "https://example.invalid/",
            ))
        },
    },
    Case {
        method: "set_cell_style",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_cell_style(need!(a.sheet), &need!(a.cell.clone()), Some(0)))
        },
    },
    Case {
        method: "set_chart_axis_gridlines",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_axis_gridlines(need!(a.sheet), need!(a.anchor), 0, true, false))
        },
    },
    Case {
        method: "set_chart_axis_orientation",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_axis_orientation(
                need!(a.sheet),
                need!(a.anchor),
                0,
                mjx_ooxml::AxisOrientation::MaximumToMinimum,
            ))
        },
    },
    Case {
        method: "set_chart_axis_scale",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_axis_scale(
                need!(a.sheet),
                need!(a.anchor),
                0,
                Some(0.0),
                Some(10.0)
            ))
        },
    },
    Case {
        method: "set_chart_axis_title",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_axis_title(need!(a.sheet), need!(a.anchor), 0, Some("Quarter")))
        },
    },
    Case {
        method: "set_chart_data_labels",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_data_labels(
                need!(a.sheet),
                need!(a.anchor),
                ChartLabelScope::Series { series_index: 0 },
                &DataLabelSpec::new().value(true),
            ))
        },
    },
    Case {
        method: "set_chart_error_bars",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_error_bars(
                need!(a.sheet),
                need!(a.anchor),
                0,
                &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
            ))
        },
    },
    Case {
        method: "set_chart_legend",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_legend(
                need!(a.sheet),
                need!(a.anchor),
                Some(LegendPosition::Bottom)
            ))
        },
    },
    Case {
        method: "set_chart_point_explosion",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_point_explosion(
                need!(a.sheet),
                need!(a.anchor),
                0,
                0,
                Some(10)
            ))
        },
    },
    Case {
        method: "set_chart_point_fill",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_point_fill(need!(a.sheet), need!(a.anchor), 0, 0, &fill()))
        },
    },
    Case {
        method: "set_chart_point_line",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_point_line(need!(a.sheet), need!(a.anchor), 0, 0, &line()))
        },
    },
    Case {
        method: "set_chart_series_categories",
        touches: CHART_DATA,
        call: |workbook, a| {
            ran!(workbook.set_chart_series_categories(
                need!(a.sheet),
                need!(a.anchor),
                0,
                &["Alpha", "Beta"],
            ))
        },
    },
    Case {
        method: "set_chart_series_fill",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_series_fill(need!(a.sheet), need!(a.anchor), 0, &fill()))
        },
    },
    Case {
        method: "set_chart_series_line",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_series_line(need!(a.sheet), need!(a.anchor), 0, &line()))
        },
    },
    Case {
        method: "set_chart_series_values",
        touches: CHART_DATA,
        call: |workbook, a| {
            ran!(workbook.set_chart_series_values(
                need!(a.sheet),
                need!(a.anchor),
                0,
                &[41.0, 42.0]
            ))
        },
    },
    Case {
        method: "set_chart_title",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_title(need!(a.sheet), need!(a.anchor), Some("Preservation")))
        },
    },
    Case {
        method: "set_chart_trendline",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.set_chart_trendline(
                need!(a.sheet),
                need!(a.anchor),
                0,
                0,
                &TrendlineSpec::new(TrendlineKind::Logarithmic),
            ))
        },
    },
    Case {
        method: "set_column_hidden",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_column_hidden(need!(a.sheet), 2, 2, true)),
    },
    Case {
        method: "set_column_outline_level",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_column_outline_level(need!(a.sheet), 0, 1, 2)),
    },
    Case {
        method: "set_column_width",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_column_width(need!(a.sheet), 0, 1, Some(18.0), true)),
    },
    Case {
        method: "set_row_height",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_row_height(need!(a.sheet), 2, Some(24.0), true)),
    },
    Case {
        method: "set_row_hidden",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_row_hidden(need!(a.sheet), 3, true)),
    },
    Case {
        method: "set_row_outline_level",
        touches: WORKSHEET_ONLY,
        call: |workbook, a| ran!(workbook.set_row_outline_level(need!(a.sheet), 2, 1)),
    },
    Case {
        method: "suppress_chart_data_labels",
        touches: CHART_ONLY,
        call: |workbook, a| {
            ran!(workbook.suppress_chart_data_labels(
                need!(a.sheet),
                need!(a.anchor),
                ChartLabelScope::Series { series_index: 0 },
            ))
        },
    },
    Case {
        method: "unmerge_cells",
        touches: Touches {
            rules: &[changed_up_to_one(WORKSHEET)],
        },
        call: |workbook, a| {
            ran!(workbook.unmerge_cells(need!(a.sheet), &need!(a.merged_range.clone())))
        },
    },
    Case {
        method: "window_views",
        touches: NOTHING,
        call: |workbook, _| ran!(workbook.window_views()),
    },
    Case {
        method: "workbook_mut",
        touches: NOTHING,
        call: |workbook, _| {
            let _ = workbook.workbook_mut();
            Step::Ran(Ok(()))
        },
    },
    Case {
        method: "write_cells",
        touches: WRITES_A_CELL,
        call: |workbook, a| {
            ran!(workbook.write_cells(
                need!(a.sheet),
                &[CellWrite::new(
                    &need!(a.cell.clone()),
                    CellInput::Number(42.0)
                )],
            ))
        },
    },
];
