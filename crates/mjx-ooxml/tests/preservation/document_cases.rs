//! Every public `&mut self` method of [`mjx_ooxml::Document`], and what each may do to a `.docx`.
//!
//! The Word surface is the one MJXOFF-209 broke: three edits — removing a header or footer, removing
//! the last comment, removing a drawing — ran a **package-wide** orphan sweep, so an edit about a
//! header could delete an unrelated image whose relationship target this library could not resolve.
//! That is why `remove_header`, `remove_footer`, `remove_comment` and `remove_drawing` below declare
//! exactly which classes they may remove, and why `percent_encoded_targets.docx` — the fixture that
//! carries a space in a media filename — is swept like every other.

use mjx_ooxml::{
    ChartData, ChartKind, ChartLabelScope, ChartWrap, ConformanceClass, DataLabelSpec, Document,
    ErrorBarSpec, ErrorBarType, ErrorValueType, FillSpec, HeaderFooterType, HyperlinkTarget,
    LegendPosition, LineSpec, LineWidth, MergedCellType, PageMargins, PageSize, SectionLocation,
    TrendlineKind, TrendlineSpec,
};

use crate::rules::{
    added, changed, changed_one, changed_up_to_one, removed, Count, Touches, ANY_EMBEDDED_CLASS,
    CHART, CONTENT_TYPES, EMBEDDED_CONTENT_TYPES, EMBEDDED_RELATIONSHIPS, EMBEDDED_SHARED_STRINGS,
    EMBEDDED_SHEET_STYLES, EMBEDDED_WORKBOOK, EMBEDDED_WORKBOOK_MAIN, EMBEDDED_WORKSHEET, NOTHING,
    PNG, RELATIONSHIPS, THEME, WORD_COMMENTS, WORD_DOCUMENT, WORD_ENDNOTES, WORD_FOOTER,
    WORD_FOOTNOTES, WORD_HEADER, WORD_NUMBERING, WORD_SETTINGS,
};
use crate::{ran, Api, Report, Step};

/// One registered method.
pub(crate) struct Case {
    /// The method name, which must match the facade's own source.
    pub(crate) method: &'static str,
    /// What it may do.
    pub(crate) touches: Touches,
    /// The call, given a document and the addresses probed from its fixture.
    pub(crate) call: fn(&mut Document, &Addresses) -> Step,
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
    /// A paragraph index, if the body holds one.
    pub(crate) paragraph: Option<u32>,
    /// A paragraph that holds at least one run, and that run's index.
    pub(crate) run: Option<(u32, u32)>,
    /// A paragraph that holds at least one field.
    pub(crate) field_paragraph: Option<u32>,
    /// A table index.
    pub(crate) table: Option<u32>,
    /// A chart's drawing id.
    pub(crate) chart: Option<u32>,
    /// A comment id.
    pub(crate) comment: Option<i64>,
    /// A footnote id.
    pub(crate) footnote: Option<i64>,
    /// An endnote id.
    pub(crate) endnote: Option<i64>,
    /// A section index that has a default header.
    pub(crate) header_section: Option<u32>,
    /// A section index that has a default footer.
    pub(crate) footer_section: Option<u32>,
    /// A style id the document defines.
    pub(crate) style: Option<String>,
    /// A paragraph and run that carry a hyperlink.
    pub(crate) hyperlink: Option<(u32, u32)>,
    /// The header kind the body section actually carries.
    pub(crate) header_kind: Option<HeaderFooterType>,
    /// The footer kind the body section actually carries.
    pub(crate) footer_kind: Option<HeaderFooterType>,
    /// A drawing id that frames something — a chart's, when there is one.
    pub(crate) drawing: Option<u32>,
}

impl Addresses {
    /// Reads every address out of the fixture, using the facade's own readers.
    fn probe(document: &mut Document) -> Self {
        let paragraph_count = document.paragraph_count().unwrap_or(0);
        let paragraph = (paragraph_count > 0).then_some(0);
        let mut run = None;
        let mut field_paragraph = None;
        for index in 0..paragraph_count {
            if run.is_none() && document.run_count(index).is_ok_and(|count| count > 0) {
                run = Some((index, 0));
            }
            if field_paragraph.is_none()
                && document
                    .fields(index)
                    .is_ok_and(|fields| !fields.is_empty())
            {
                field_paragraph = Some(index);
            }
            if run.is_some() && field_paragraph.is_some() {
                break;
            }
        }

        let chart = document
            .chart_drawing_ids()
            .unwrap_or_default()
            .into_iter()
            .next();
        // A run that is a hyperlink, so `remove_hyperlink` edits one rather than refusing on a
        // plain run — which is all it did before this probe existed.
        let mut hyperlink = None;
        'links: for paragraph in 0..paragraph_count {
            let Ok(runs) = document.run_count(paragraph) else {
                continue;
            };
            for run in 0..runs {
                if document
                    .hyperlink_target(paragraph, run)
                    .is_ok_and(|target| target.is_some())
                {
                    hyperlink = Some((paragraph, run));
                    break 'links;
                }
            }
        }

        let sections = document.section_count().unwrap_or(0);
        let mut header_section = None;
        let mut footer_section = None;
        for index in 0..sections {
            if header_section.is_none()
                && document
                    .header_text(index, HeaderFooterType::Default)
                    .is_ok_and(|text| text.is_some())
            {
                header_section = Some(index);
            }
            if footer_section.is_none()
                && document
                    .footer_text(index, HeaderFooterType::Default)
                    .is_ok_and(|text| text.is_some())
            {
                footer_section = Some(index);
            }
        }

        // Which *kind* the last section carries. `remove_header`/`remove_footer` address
        // `SectionLocation::Body`, and a fixture whose footer is a `First` or `Even` one was a
        // twenty-fixture no-op until the kind was probed rather than assumed.
        let body = sections.saturating_sub(1);
        let kinds = [
            HeaderFooterType::Default,
            HeaderFooterType::First,
            HeaderFooterType::Even,
        ];
        let header_kind = kinds.into_iter().find(|kind| {
            document
                .header_text(body, *kind)
                .is_ok_and(|text| text.is_some())
        });
        let footer_kind = kinds.into_iter().find(|kind| {
            document
                .footer_text(body, *kind)
                .is_ok_and(|text| text.is_some())
        });

        Self {
            paragraph,
            run,
            field_paragraph,
            hyperlink,
            header_kind,
            footer_kind,
            table: document
                .table_count()
                .ok()
                .and_then(|count| (count > 0).then_some(0)),
            chart,
            comment: document
                .comments()
                .unwrap_or_default()
                .first()
                .map(|comment| comment.id),
            footnote: document
                .footnotes()
                .unwrap_or_default()
                .first()
                .map(|note| note.id),
            endnote: document
                .endnotes()
                .unwrap_or_default()
                .first()
                .map(|note| note.id),
            header_section,
            footer_section,
            style: document.style_ids().unwrap_or_default().into_iter().next(),
            drawing: chart,
        }
    }
}

/// The whole `.docx` surface, run against one fixture.
pub(crate) fn sweep(fixture: &str, before: &[u8], report: &mut Report) {
    let mut probe = match Document::open(before) {
        Ok(document) => document,
        Err(error) => {
            crate::record_unopenable(report, fixture, Api::Document, &error);
            return;
        }
    };
    let addresses = Addresses::probe(&mut probe);
    drop(probe);

    for case in cases() {
        let (base, addresses) = prepared(case.method, before, &addresses);
        let mut document = Document::open(&base).expect("the prepared package opens");
        let step = (case.call)(&mut document, &addresses);
        let saved = matches!(step, Step::Ran(_)).then(|| document.save());
        crate::record(
            report,
            fixture,
            Api::Document,
            case.method,
            case.touches,
            step,
            &base,
            saved,
        );
    }
}

/// The package a case is measured against, and the addresses it offers. See the `.pptx` sweep's own
/// `prepared` for why a case may edit the fixture before the comparison starts.
fn prepared(method: &str, before: &[u8], addresses: &Addresses) -> (Vec<u8>, Addresses) {
    let Some((_, prepare)) = PREPARATIONS.iter().find(|(name, _)| *name == method) else {
        return (before.to_vec(), addresses.clone());
    };
    let Ok(mut document) = Document::open(before) else {
        return (before.to_vec(), addresses.clone());
    };
    if !matches!(prepare(&mut document, addresses), Step::Ran(Ok(()))) {
        return (before.to_vec(), addresses.clone());
    }
    let Ok(base) = document.save() else {
        return (before.to_vec(), addresses.clone());
    };
    let Ok(mut reopened) = Document::open(&base) else {
        return (before.to_vec(), addresses.clone());
    };
    let addresses = Addresses::probe(&mut reopened);
    (base, addresses)
}

/// The chart every chart preparation works on: the fixture's own if it has one, a fresh one if not.
fn chart_for_preparation(document: &mut Document, a: &Addresses) -> Option<u32> {
    if let Some(chart) = a.chart {
        return Some(chart);
    }
    document
        .add_chart(
            a.paragraph?.into(),
            &chart_data(),
            4_572_000,
            2_743_200,
            "Revenue",
        )
        .ok()
}

/// The edits that put a fixture into the state a method needs.
#[allow(
    clippy::type_complexity,
    reason = "a table of (name, edit) is what a lookup by method name is"
)]
const PREPARATIONS: &[(&str, fn(&mut Document, &Addresses) -> Step)] = &[
    ("drop_chart_dangling_decoration", |document, a| {
        // A decoration dangles when the point it is anchored to stops existing; see the `.pptx`
        // sweep's own copy.
        let chart = need!(chart_for_preparation(document, a));
        if document.set_chart_point_fill(chart, 0, 1, &fill()).is_err() {
            return Step::Skipped;
        }
        ran!(document.set_chart_series_values(chart, 0, &[9.0]))
    }),
    ("comment_range_text", |document, a| {
        ran!(document.add_comment(need!(a.paragraph), "Reviewer", Some("R"), "a remark"))
    }),
    ("remove_comment", |document, a| {
        ran!(document.add_comment(need!(a.paragraph), "Reviewer", Some("R"), "a remark"))
    }),
    ("remove_endnote", |document, a| {
        ran!(document.add_endnote(need!(a.paragraph), "an endnote"))
    }),
    ("remove_footnote", |document, a| {
        ran!(document.add_footnote(need!(a.paragraph), "a note"))
    }),
    ("remove_footer", |document, _| {
        ran!(document.set_footer_text(SectionLocation::Body, HeaderFooterType::Default, "Footer"))
    }),
    ("remove_header", |document, _| {
        ran!(document.set_header_text(SectionLocation::Body, HeaderFooterType::Default, "Header"))
    }),
    ("set_chart_legend", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.set_chart_legend(chart, None))
    }),
    ("remove_chart_data_labels", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.set_chart_data_labels(
            chart,
            ChartLabelScope::Series { series_index: 0 },
            &DataLabelSpec::new().value(true),
        ))
    }),
    ("remove_chart_error_bars", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.set_chart_error_bars(
            chart,
            0,
            &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
        ))
    }),
    ("remove_chart_point_format", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.set_chart_point_fill(chart, 0, 0, &fill()))
    }),
    ("remove_chart_trendlines", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.add_chart_trendline(chart, 0, &TrendlineSpec::new(TrendlineKind::Linear)))
    }),
    ("set_chart_trendline", |document, a| {
        let chart = need!(chart_for_preparation(document, a));
        ran!(document.add_chart_trendline(chart, 0, &TrendlineSpec::new(TrendlineKind::Linear)))
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

/// The main document part changes and nothing else does.
const DOCUMENT_ONLY: Touches = Touches {
    rules: &[changed_one(WORD_DOCUMENT)],
};

/// A chart edit that changes only the chart part.
const CHART_ONLY: Touches = Touches {
    rules: &[changed_one(CHART)],
};

/// A chart **data** edit — MJXOFF-208's declaration, on the Word surface.
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

/// A chart arriving in a document: its own three parts, the run that frames it, and — only where the
/// package has no theme at all — the theme its scheme colours resolve against (MJXOFF-200). A theme
/// the document already carried is **not** on this list, which is the assertion that fails if the
/// writer ever stops checking first.
const ADDS_A_CHART: Touches = Touches {
    rules: &[
        changed_one(WORD_DOCUMENT),
        changed(RELATIONSHIPS, Count::UpTo(1)),
        changed(CONTENT_TYPES, Count::UpTo(1)),
        added(CHART, Count::UpTo(1)),
        // Two: the chart part's own, and — for a document that arrived with no relationships at
        // all, which two fixtures of the corpus did — `word/_rels/document.xml.rels` itself.
        added(RELATIONSHIPS, Count::UpTo(2)),
        added(EMBEDDED_WORKBOOK, Count::UpTo(1)),
        // A workbook this call created arrives whole; see `ANY_EMBEDDED_CLASS`.
        added(ANY_EMBEDDED_CLASS, Count::Any),
        added(THEME, Count::UpTo(1)),
    ],
};

// =================================================================================================
// The registry
// =================================================================================================

/// Every case of this surface. The `Document` surface has no feature-gated method, so this is
/// [`CASES`] itself; it exists so all three surfaces are driven through one shape.
pub(crate) fn cases() -> Vec<&'static Case> {
    CASES.iter().collect()
}

/// Every public `&mut self` method of `Document`.
pub(crate) const CASES: &[Case] = &[
    Case {
        method: "add_chart",
        touches: ADDS_A_CHART,
        call: |document, a| {
            ran!(document.add_chart(
                need!(a.paragraph).into(),
                &chart_data(),
                4_572_000,
                2_743_200,
                "Revenue",
            ))
        },
    },
    Case {
        method: "add_chart_trendline",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.add_chart_trendline(
                need!(a.chart),
                0,
                &TrendlineSpec::new(TrendlineKind::Linear)
            ))
        },
    },
    Case {
        method: "add_comment",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_COMMENTS, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORD_COMMENTS, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            ran!(document.add_comment(need!(a.paragraph), "Reviewer", Some("R"), "a remark"))
        },
    },
    Case {
        method: "add_endnote",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_ENDNOTES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORD_ENDNOTES, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| ran!(document.add_endnote(need!(a.paragraph), "an endnote")),
    },
    Case {
        method: "add_floating_chart",
        touches: ADDS_A_CHART,
        call: |document, a| {
            ran!(document.add_floating_chart(
                need!(a.paragraph).into(),
                &chart_data(),
                0,
                0,
                4_572_000,
                2_743_200,
                ChartWrap::Square(mjx_ooxml::WrapText::BothSides),
                "Revenue",
            ))
        },
    },
    Case {
        method: "add_footnote",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_FOOTNOTES, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORD_FOOTNOTES, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| ran!(document.add_footnote(need!(a.paragraph), "a note")),
    },
    Case {
        method: "add_inline_picture",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(PNG, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            ran!(document.add_inline_picture(
                need!(a.paragraph),
                mjx_ooxml::DEFAULT_PLACEHOLDER_IMAGE.to_vec(),
                "image/png",
                "png",
                914_400,
                914_400,
                "picture",
            ))
        },
    },
    Case {
        method: "append_paragraph",
        touches: DOCUMENT_ONLY,
        call: |document, _| ran!(document.append_paragraph()),
    },
    Case {
        method: "append_run",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.append_run(need!(a.paragraph), "Preservation")),
    },
    Case {
        method: "append_table",
        touches: DOCUMENT_ONLY,
        call: |document, _| ran!(document.append_table(2, 2)),
    },
    Case {
        method: "attach_paragraph_to_list",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.attach_paragraph_to_list(need!(a.paragraph), 1, 0)),
    },
    Case {
        method: "cell_span",
        touches: NOTHING,
        call: |document, a| ran!(document.cell_span(need!(a.table), 0, 0)),
    },
    Case {
        method: "cell_text",
        touches: NOTHING,
        call: |document, a| ran!(document.cell_text(need!(a.table), 0, 0)),
    },
    Case {
        method: "chart_axes",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_axes(need!(a.chart))),
    },
    Case {
        method: "chart_dangling_decoration",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_dangling_decoration(need!(a.chart), 0)),
    },
    Case {
        method: "chart_data_label_tier",
        touches: NOTHING,
        call: |document, a| {
            ran!(document
                .chart_data_label_tier(need!(a.chart), ChartLabelScope::Series { series_index: 0 }))
        },
    },
    Case {
        method: "chart_data_labels",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_data_labels(need!(a.chart), 0, None)),
    },
    Case {
        method: "chart_drawing_ids",
        touches: NOTHING,
        call: |document, _| ran!(document.chart_drawing_ids()),
    },
    Case {
        method: "chart_error_bars",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_error_bars(need!(a.chart), 0)),
    },
    Case {
        method: "chart_kinds",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_kinds(need!(a.chart))),
    },
    Case {
        method: "chart_legend",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_legend(need!(a.chart))),
    },
    Case {
        method: "chart_part_bytes",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_part_bytes(need!(a.chart))),
    },
    Case {
        method: "chart_point_formats",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_point_formats(need!(a.chart), 0)),
    },
    Case {
        method: "chart_point_label_text",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_point_label_text(need!(a.chart), 0, 0)),
    },
    Case {
        method: "chart_rel_id",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_rel_id(need!(a.chart))),
    },
    Case {
        method: "chart_series",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_series(need!(a.chart))),
    },
    Case {
        method: "chart_series_fill",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_series_fill(need!(a.chart), 0)),
    },
    Case {
        method: "chart_series_references",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_series_references(need!(a.chart))),
    },
    Case {
        method: "chart_style_id",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_style_id(need!(a.chart))),
    },
    Case {
        method: "chart_title",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_title(need!(a.chart))),
    },
    Case {
        method: "chart_trendlines",
        touches: NOTHING,
        call: |document, a| ran!(document.chart_trendlines(need!(a.chart), 0)),
    },
    Case {
        method: "chart_workbooks",
        touches: NOTHING,
        call: |document, _| ran!(document.chart_workbooks()),
    },
    Case {
        method: "comment_range_text",
        touches: NOTHING,
        call: |document, a| ran!(document.comment_range_text(need!(a.comment))),
    },
    Case {
        method: "comments",
        touches: NOTHING,
        call: |document, _| ran!(document.comments()),
    },
    Case {
        method: "conformance",
        touches: NOTHING,
        call: |document, _| ran!(document.conformance()),
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
        call: |document, a| ran!(document.detach_chart_workbook(need!(a.chart))),
    },
    Case {
        method: "detach_paragraph_from_list",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.detach_paragraph_from_list(need!(a.paragraph))),
    },
    Case {
        method: "document_mut",
        touches: NOTHING,
        call: |document, _| {
            let _ = document.document_mut();
            Step::Ran(Ok(()))
        },
    },
    Case {
        method: "drop_chart_dangling_decoration",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.drop_chart_dangling_decoration(need!(a.chart), 0)),
    },
    Case {
        method: "effective_cell_border",
        touches: NOTHING,
        call: |document, a| {
            ran!(document.effective_cell_border(
                need!(a.table),
                0,
                0,
                mjx_ooxml::CellBorderEdge::Bottom
            ))
        },
    },
    Case {
        method: "effective_cell_fill",
        touches: NOTHING,
        call: |document, a| ran!(document.effective_cell_fill(need!(a.table), 0, 0)),
    },
    Case {
        method: "effective_cell_run_properties",
        touches: NOTHING,
        call: |document, a| {
            ran!(document.effective_cell_run_properties(need!(a.table), 0, 0, 0, 0))
        },
    },
    Case {
        method: "effective_paragraph_properties",
        touches: NOTHING,
        call: |document, a| ran!(document.effective_paragraph_properties(need!(a.paragraph))),
    },
    Case {
        method: "effective_run_properties",
        touches: NOTHING,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.effective_run_properties(paragraph, run))
        },
    },
    Case {
        method: "endnotes",
        touches: NOTHING,
        call: |document, _| ran!(document.endnotes()),
    },
    Case {
        method: "even_and_odd_headers",
        touches: NOTHING,
        call: |document, _| ran!(document.even_and_odd_headers()),
    },
    Case {
        method: "fields",
        touches: NOTHING,
        call: |document, a| ran!(document.fields(need!(a.paragraph))),
    },
    Case {
        method: "footer_text",
        touches: NOTHING,
        call: |document, a| {
            ran!(document.footer_text(need!(a.footer_section), HeaderFooterType::Default))
        },
    },
    Case {
        method: "footnotes",
        touches: NOTHING,
        call: |document, _| ran!(document.footnotes()),
    },
    Case {
        method: "header_text",
        touches: NOTHING,
        call: |document, a| {
            ran!(document.header_text(need!(a.header_section), HeaderFooterType::Default))
        },
    },
    Case {
        method: "hyperlink_target",
        touches: NOTHING,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.hyperlink_target(paragraph, run))
        },
    },
    Case {
        method: "insert_column",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.insert_column(need!(a.table), 0)),
    },
    Case {
        method: "insert_hyperlink",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.insert_hyperlink(
                paragraph,
                run,
                "example",
                &HyperlinkTarget::Url("https://example.invalid/".to_owned()),
            ))
        },
    },
    Case {
        method: "insert_paragraph",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.insert_paragraph(need!(a.paragraph))),
    },
    Case {
        method: "insert_row",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.insert_row(need!(a.table), 0)),
    },
    Case {
        method: "insert_run",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.insert_run(paragraph, run, "Preservation"))
        },
    },
    Case {
        method: "merged_cell_anchor",
        touches: NOTHING,
        call: |document, a| ran!(document.merged_cell_anchor(need!(a.table), 0, 0)),
    },
    Case {
        method: "paragraph_count",
        touches: NOTHING,
        call: |document, _| ran!(document.paragraph_count()),
    },
    Case {
        method: "paragraph_text",
        touches: NOTHING,
        call: |document, a| ran!(document.paragraph_text(need!(a.paragraph))),
    },
    Case {
        method: "refresh_chart_workbook",
        touches: REFRESH_CHART_DATA,
        call: |document, a| ran!(document.refresh_chart_workbook(need!(a.chart))),
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
        call: |document, a| ran!(document.regenerate_chart_workbook(need!(a.chart))),
    },
    Case {
        method: "remove_chart_data_labels",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.remove_chart_data_labels(
                need!(a.chart),
                ChartLabelScope::Series { series_index: 0 }
            ))
        },
    },
    Case {
        method: "remove_chart_error_bars",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.remove_chart_error_bars(need!(a.chart), 0)),
    },
    Case {
        method: "remove_chart_point_format",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.remove_chart_point_format(need!(a.chart), 0, 0)),
    },
    Case {
        method: "remove_chart_trendlines",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.remove_chart_trendlines(need!(a.chart), 0)),
    },
    Case {
        method: "remove_column",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.remove_column(need!(a.table), 0)),
    },
    Case {
        method: "remove_comment",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_COMMENTS, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                // Removing the *last* comment retires the comments part with it — and nothing else.
                // Before MJXOFF-209 this call ran a package-wide orphan sweep, so an unrelated
                // image could vanish here; that is what this clause refuses.
                removed(WORD_COMMENTS, Count::UpTo(1)),
            ],
        },
        call: |document, a| ran!(document.remove_comment(need!(a.comment))),
    },
    Case {
        method: "remove_drawing",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(CHART, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::UpTo(1)),
                removed(EMBEDDED_WORKBOOK, Count::UpTo(1)),
                removed(crate::rules::ANY_CLASS, Count::Any),
            ],
        },
        call: |document, a| ran!(document.remove_drawing(need!(a.drawing))),
    },
    Case {
        method: "remove_endnote",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_ENDNOTES, Count::UpTo(1)),
            ],
        },
        call: |document, a| ran!(document.remove_endnote(need!(a.endnote))),
    },
    Case {
        method: "remove_footer",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(WORD_FOOTER, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            ran!(document.remove_footer(SectionLocation::Body, need!(a.footer_kind)))
        },
    },
    Case {
        method: "remove_footnote",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_FOOTNOTES, Count::UpTo(1)),
            ],
        },
        call: |document, a| ran!(document.remove_footnote(need!(a.footnote))),
    },
    Case {
        method: "remove_header",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                removed(WORD_HEADER, Count::UpTo(1)),
                removed(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            ran!(document.remove_header(SectionLocation::Body, need!(a.header_kind)))
        },
    },
    Case {
        method: "remove_hyperlink",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, a| {
            let (paragraph, run) = need!(a.hyperlink);
            ran!(document.remove_hyperlink(paragraph, run))
        },
    },
    Case {
        method: "remove_paragraph",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.remove_paragraph(need!(a.paragraph))),
    },
    Case {
        method: "remove_row",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.remove_row(need!(a.table), 0)),
    },
    Case {
        method: "remove_run",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.remove_run(paragraph, run))
        },
    },
    Case {
        method: "remove_section_properties",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            ran!(document
                .remove_section_properties(SectionLocation::Paragraph(need!(a.paragraph).into())))
        },
    },
    Case {
        method: "remove_table",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.remove_table(need!(a.table))),
    },
    Case {
        method: "revisions",
        touches: NOTHING,
        call: |document, _| ran!(document.revisions()),
    },
    Case {
        method: "run_count",
        touches: NOTHING,
        call: |document, a| ran!(document.run_count(need!(a.paragraph))),
    },
    Case {
        method: "run_text",
        touches: NOTHING,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.run_text(paragraph, run))
        },
    },
    Case {
        method: "section_count",
        touches: NOTHING,
        call: |document, _| ran!(document.section_count()),
    },
    Case {
        method: "sections",
        touches: NOTHING,
        call: |document, _| ran!(document.sections()),
    },
    Case {
        method: "set_cell_span",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.set_cell_span(need!(a.table), 0, 0, Some(2))),
    },
    Case {
        method: "set_cell_text",
        touches: DOCUMENT_ONLY,
        call: |document, a| ran!(document.set_cell_text(need!(a.table), 0, 0, "Preservation")),
    },
    Case {
        method: "set_cell_vertical_merge",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            ran!(document.set_cell_vertical_merge(
                need!(a.table),
                0,
                0,
                Some(MergedCellType::Restart)
            ))
        },
    },
    Case {
        method: "set_chart_axis_gridlines",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_axis_gridlines(need!(a.chart), 0, true, false)),
    },
    Case {
        method: "set_chart_axis_orientation",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_axis_orientation(
                need!(a.chart),
                0,
                mjx_ooxml::AxisOrientation::MaximumToMinimum,
            ))
        },
    },
    Case {
        method: "set_chart_axis_scale",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_axis_scale(need!(a.chart), 0, Some(0.0), Some(10.0)))
        },
    },
    Case {
        method: "set_chart_axis_title",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_axis_title(need!(a.chart), 0, Some("Quarter"))),
    },
    Case {
        method: "set_chart_data_labels",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_data_labels(
                need!(a.chart),
                ChartLabelScope::Series { series_index: 0 },
                &DataLabelSpec::new().value(true),
            ))
        },
    },
    Case {
        method: "set_chart_error_bars",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_error_bars(
                need!(a.chart),
                0,
                &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.5),
            ))
        },
    },
    Case {
        method: "set_chart_legend",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_legend(need!(a.chart), Some(LegendPosition::Bottom)))
        },
    },
    Case {
        method: "set_chart_point_explosion",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_point_explosion(need!(a.chart), 0, 0, Some(10)))
        },
    },
    Case {
        method: "set_chart_point_fill",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_point_fill(need!(a.chart), 0, 0, &fill())),
    },
    Case {
        method: "set_chart_point_line",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_point_line(need!(a.chart), 0, 0, &line())),
    },
    Case {
        method: "set_chart_series_categories",
        touches: CHART_DATA,
        call: |document, a| {
            ran!(document.set_chart_series_categories(need!(a.chart), 0, &["Alpha", "Beta"]))
        },
    },
    Case {
        method: "set_chart_series_fill",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_series_fill(need!(a.chart), 0, &fill())),
    },
    Case {
        method: "set_chart_series_line",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_series_line(need!(a.chart), 0, &line())),
    },
    Case {
        method: "set_chart_series_values",
        touches: CHART_DATA,
        call: |document, a| {
            ran!(document.set_chart_series_values(need!(a.chart), 0, &[41.0, 42.0]))
        },
    },
    Case {
        method: "set_chart_title",
        touches: CHART_ONLY,
        call: |document, a| ran!(document.set_chart_title(need!(a.chart), Some("Preservation"))),
    },
    Case {
        method: "set_chart_trendline",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.set_chart_trendline(
                need!(a.chart),
                0,
                0,
                &TrendlineSpec::new(TrendlineKind::Logarithmic)
            ))
        },
    },
    Case {
        method: "set_conformance",
        touches: Touches {
            rules: &[
                changed_one(WORD_DOCUMENT),
                changed(WORD_SETTINGS, Count::UpTo(1)),
            ],
        },
        call: |document, _| ran!(document.set_conformance(Some(ConformanceClass::Transitional))),
    },
    Case {
        method: "set_field_cached_result_text",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            ran!(document.set_field_cached_result_text(need!(a.field_paragraph), &[0], "cached"))
        },
    },
    Case {
        method: "set_field_instruction",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            ran!(document.set_field_instruction(need!(a.field_paragraph), &[0], "PAGE"))
        },
    },
    Case {
        method: "set_footer_text",
        touches: Touches {
            rules: &[
                changed_up_to_one(WORD_DOCUMENT),
                changed(WORD_FOOTER, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORD_FOOTER, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, _| {
            ran!(document.set_footer_text(
                SectionLocation::Body,
                HeaderFooterType::Default,
                "Footer",
            ))
        },
    },
    Case {
        method: "set_header_text",
        touches: Touches {
            rules: &[
                changed_up_to_one(WORD_DOCUMENT),
                changed(WORD_HEADER, Count::UpTo(1)),
                changed(RELATIONSHIPS, Count::UpTo(1)),
                changed(CONTENT_TYPES, Count::UpTo(1)),
                added(WORD_HEADER, Count::UpTo(1)),
                added(RELATIONSHIPS, Count::UpTo(1)),
            ],
        },
        call: |document, _| {
            ran!(document.set_header_text(
                SectionLocation::Body,
                HeaderFooterType::Default,
                "Header",
            ))
        },
    },
    Case {
        method: "set_run_text",
        touches: DOCUMENT_ONLY,
        call: |document, a| {
            let (paragraph, run) = need!(a.run);
            ran!(document.set_run_text(paragraph, run, "Preservation"))
        },
    },
    Case {
        method: "set_section_page_margins",
        touches: DOCUMENT_ONLY,
        call: |document, _| {
            ran!(document
                .set_section_page_margins(SectionLocation::Body, Some(PageMargins::NORMAL),))
        },
    },
    Case {
        method: "set_section_page_size",
        touches: DOCUMENT_ONLY,
        call: |document, _| {
            ran!(document.set_section_page_size(SectionLocation::Body, Some(PageSize::us_letter())))
        },
    },
    Case {
        method: "style_ids",
        touches: NOTHING,
        call: |document, _| ran!(document.style_ids()),
    },
    Case {
        method: "style_name",
        touches: NOTHING,
        call: |document, a| ran!(document.style_name(&need!(a.style.clone()))),
    },
    Case {
        method: "suppress_chart_data_labels",
        touches: CHART_ONLY,
        call: |document, a| {
            ran!(document.suppress_chart_data_labels(
                need!(a.chart),
                ChartLabelScope::Series { series_index: 0 }
            ))
        },
    },
    Case {
        method: "table_count",
        touches: NOTHING,
        call: |document, _| ran!(document.table_count()),
    },
    Case {
        method: "table_dimensions",
        touches: NOTHING,
        call: |document, a| ran!(document.table_dimensions(need!(a.table))),
    },
    Case {
        method: "table_grid_discrepancies",
        touches: NOTHING,
        call: |document, a| ran!(document.table_grid_discrepancies(need!(a.table))),
    },
    Case {
        method: "text_with_revisions_accepted",
        touches: NOTHING,
        call: |document, _| ran!(document.text_with_revisions_accepted()),
    },
    Case {
        method: "text_with_revisions_rejected",
        touches: NOTHING,
        call: |document, _| ran!(document.text_with_revisions_rejected()),
    },
];

/// The numbering part is named by `attach_paragraph_to_list`'s declaration only when a document
/// gains one; naming the constant here keeps the import list honest until it does.
#[allow(
    dead_code,
    reason = "named by attach_paragraph_to_list's documentation"
)]
const _NUMBERING: &str = WORD_NUMBERING;
