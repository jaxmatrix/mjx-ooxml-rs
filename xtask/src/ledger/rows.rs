//! **The ledger's rows** — what to look at, and never what the answer is.
//!
//! # What is hand-written here, and what is not
//!
//! A [`Capability`] declares four things: an identifier, which section of
//! `docs/client-platform/OFFICE_FEATURE_INVENTORY.md` it comes from, what sort of thing it is, and
//! **the suites that are its evidence**. It has no state field, and that absence is the design:
//! there is nowhere in this file to write `implemented`, so nobody can. The state is derived by
//! [`super::assess`] from what the suites actually contain.
//!
//! The evidence paths are pointers, and a pointer can rot. It cannot rot *silently*: a path that
//! names no suite fails the build, by name, at the lookup. That is the liveness check the ticket
//! asks for and the defect `UNCOVERED_SCHEMAS` shipped with.
//!
//! # Where the rows come from, and what that means for any fraction
//!
//! The rows are a partition of the inventory: §2's exclusions, §3's shared subsystems, §4's three
//! application surfaces, §5's calculation engine and §6's third axis. **A partition is a reading,
//! not a census.** The number of rows is therefore not a count of anything published — not the
//! 11,869 in-scope controls and not the 3,404 declared elements — and the generated document says so
//! where a reader would otherwise be tempted to divide.
//!
//! # Adding a row
//!
//! Write the capability and point it at the suites that cover it. If nothing covers it, give it no
//! evidence and let it be `not-started`: **a row with no suite is the most useful row in the
//! table**, because it is the one that says where the work is. Do not delete a row to make the
//! summary look better, and do not point one at a suite that does not exercise it — the assertion
//! count is derived, so an empty suite promotes nothing anyway.

/// Which part of the inventory a row comes from.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Section {
    /// §2 — excluded by decision.
    Excluded,
    /// §3.1 — text and typography.
    SharedText,
    /// §3.2 — paragraphs.
    SharedParagraphs,
    /// §3.3 — drawing and shapes.
    SharedDrawing,
    /// §3.4 — pictures.
    SharedPictures,
    /// §3.5 — charts.
    SharedCharts,
    /// §3.6 — SmartArt and diagrams.
    SharedDiagrams,
    /// §3.7 — mathematics.
    SharedMath,
    /// §3.8 — tables.
    SharedTables,
    /// §3.9 — cross-cutting document surfaces.
    SharedDocument,
    /// §4.1 — Word.
    Word,
    /// §4.2 — Excel.
    Excel,
    /// §4.3 — PowerPoint.
    PowerPoint,
    /// §5 — the calculation engine.
    Calculation,
    /// §6 — behaviour with neither markup nor a button.
    ThirdAxis,
}

impl Section {
    /// Every section, in inventory order.
    pub(crate) const ALL: [Self; 15] = [
        Self::Excluded,
        Self::SharedText,
        Self::SharedParagraphs,
        Self::SharedDrawing,
        Self::SharedPictures,
        Self::SharedCharts,
        Self::SharedDiagrams,
        Self::SharedMath,
        Self::SharedTables,
        Self::SharedDocument,
        Self::Word,
        Self::Excel,
        Self::PowerPoint,
        Self::Calculation,
        Self::ThirdAxis,
    ];

    /// The heading this section is printed under.
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Self::Excluded => "§2 · Excluded by decision",
            Self::SharedText => "§3.1 · Text and typography",
            Self::SharedParagraphs => "§3.2 · Paragraphs",
            Self::SharedDrawing => "§3.3 · Drawing and shapes",
            Self::SharedPictures => "§3.4 · Pictures",
            Self::SharedCharts => "§3.5 · Charts",
            Self::SharedDiagrams => "§3.6 · SmartArt and diagrams",
            Self::SharedMath => "§3.7 · Mathematics",
            Self::SharedTables => "§3.8 · Tables",
            Self::SharedDocument => "§3.9 · Cross-cutting document surfaces",
            Self::Word => "§4.1 · Word",
            Self::Excel => "§4.2 · Excel",
            Self::PowerPoint => "§4.3 · PowerPoint",
            Self::Calculation => "§5 · The calculation engine",
            Self::ThirdAxis => "§6 · The third axis — behaviour with neither markup nor a button",
        }
    }
}

/// What sort of thing a row is, which decides **what question to ask of the evidence**.
///
/// This is not a state and it is not an answer. It says whether "does anything draw it?" is a
/// meaningful question for this capability, because for a §6 behaviour it is not: an undo stack has
/// no markup and nothing paints it, and calling that `preserved-not-rendered` would be nonsense
/// rather than honesty.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// Markup a reader expects to see drawn. Needs rendering-tier evidence to be `implemented`;
    /// with model-tier evidence alone it is `preserved-not-rendered`.
    Rendered,
    /// Behaviour with no markup — §6, and the platform's own machinery. The rendering tier is not
    /// the question.
    Behaviour,
    /// Excluded by decision. Carries a reason and consults no suite.
    Excluded,
}

/// One row of the ledger. **It declares no state.**
#[derive(Clone, Debug)]
pub(crate) struct Capability {
    /// A stable, unique identifier.
    pub(crate) id: &'static str,
    /// Which part of the inventory it comes from.
    pub(crate) section: Section,
    /// What it is, in one line.
    pub(crate) capability: &'static str,
    /// What sort of thing it is.
    pub(crate) kind: Kind,
    /// Why it is excluded. Required for [`Kind::Excluded`] and forbidden otherwise.
    pub(crate) excluded_because: Option<&'static str>,
    /// The suites that cover it, as workspace-relative paths.
    pub(crate) evidence: &'static [&'static str],
}

/// Shorthand for a document capability.
const fn rendered(
    id: &'static str,
    section: Section,
    capability: &'static str,
    evidence: &'static [&'static str],
) -> Capability {
    Capability {
        id,
        section,
        capability,
        kind: Kind::Rendered,
        excluded_because: None,
        evidence,
    }
}

/// Shorthand for a behaviour with no markup.
const fn behaviour(
    id: &'static str,
    section: Section,
    capability: &'static str,
    evidence: &'static [&'static str],
) -> Capability {
    Capability {
        id,
        section,
        capability,
        kind: Kind::Behaviour,
        excluded_because: None,
        evidence,
    }
}

/// Shorthand for an exclusion.
///
/// It takes a section because §2 is not the only place one lives: `PLAN.md` puts the calculation
/// engine out of scope for v1, and a row that stated the exclusion under §2's heading would file it
/// under the wrong decision.
const fn excluded(
    id: &'static str,
    section: Section,
    capability: &'static str,
    because: &'static str,
) -> Capability {
    Capability {
        id,
        section,
        capability,
        kind: Kind::Excluded,
        excluded_because: Some(because),
        evidence: &[],
    }
}

/// Every row of the parity ledger.
pub(crate) const CAPABILITIES: &[Capability] = &[
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §2 · Excluded by decision. Rows rather than silent omissions, so a later reader can reopen
    // the decision instead of rediscovering the gap.
    // ─────────────────────────────────────────────────────────────────────────────────────────
    excluded(
        "excluded-add-in-host",
        Section::Excluded,
        "Add-in host — `TabAddIns`, `GroupOfficeExtension`",
        "an add-in runtime is a host for third-party code, not the application's own document \
         surface",
    ),
    excluded(
        "excluded-cloud-intelligence",
        Section::Excluded,
        "Cloud intelligence — `GroupAIAssistance`, `GroupIdeas`, `GroupDesignerOptions`",
        "a service call, not a document feature; the result it inserts is ordinary markup this \
         ledger already covers",
    ),
    excluded(
        "excluded-cloud-collaboration",
        Section::Excluded,
        "Cloud collaboration — `GroupCollaborate`, `TabConflicts`, `TabMerge`",
        "co-authoring is a tenant and transport concern; the merge of two documents is not the \
         application's rendering surface",
    ),
    excluded(
        "excluded-tenant-licensing",
        Section::Excluded,
        "Tenant, licensing and labelling — `GroupClassifyLabelProtect`, `TabSyntex`",
        "an organisation's policy surface, bound to a tenant this library has no notion of",
    ),
    excluded(
        "excluded-external-data-and-bi",
        Section::Excluded,
        "External data and BI — `GroupPowerQuery*` (233 controls in Excel alone), `GroupPowerBI`",
        "a query engine against external sources; the workbook it lands in is SpreadsheetML this \
         ledger already covers",
    ),
    excluded(
        "excluded-automation-runtimes",
        Section::Excluded,
        "Automation runtimes — `TabDeveloper`, `GroupMacros`, `GroupOfficeScripts`, `GroupPythonChunk`",
        "executing code embedded in a document is a security surface this project does not open; \
         VBA parts are preserved as opaque bytes and never run",
    ),
    excluded(
        "excluded-speech-and-translation",
        Section::Excluded,
        "Speech and translation services — `GroupVoiceTools`, `GroupLiveSubtitles`",
        "a service call with no document markup of its own",
    ),
    excluded(
        "excluded-external-publishing",
        Section::Excluded,
        "External publishing — `TabBlogPost`, `GroupInsertBarcode`",
        "publishing to a third-party endpoint, not an editing or rendering surface",
    ),
    excluded(
        "excluded-help-and-community",
        Section::Excluded,
        "Help and community — `HelpTab`, `GroupExcelCommunity`, `GroupResearch`",
        "application chrome around the product rather than the product",
    ),
    excluded(
        "excluded-mail-merge",
        Section::Excluded,
        "Mail merge — `TabMailings`, 51 controls",
        "**flagged for revisiting.** The merge engine belongs to Word and only its data source is \
         external; excluded from the first pass because it is self-contained enough to add later \
         without disturbing anything",
    ),
    excluded(
        "excluded-ink",
        Section::Excluded,
        "Ink — `TabDrawInk`, 32–52 controls per application",
        "**flagged for revisiting.** Ink is a genuine document feature stored in the file and a \
         stylus is first-class on the mobile target; the markup is already preserved \
         (`crates/mjx-pptx/tests/ink.rs`) and nothing lays it out or draws it",
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.1 · Text and typography
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "run-formatting",
        Section::SharedText,
        "Run formatting — family, size, colour, bold, italic, underline, strikethrough, caps",
        &[
            "crates/mjx-dml/tests/character_model.rs",
            "crates/mjx-pptx/tests/text_formatting.rs",
            "crates/mjx-docx/tests/run_properties.rs",
            "crates/mjx-layout-pptx/tests/a_slide_becomes_fragments.rs",
            "crates/mjx-layout-docx/tests/a_document_becomes_fragments.rs",
        ],
    ),
    rendered(
        "text-shaping",
        Section::SharedText,
        "Shaping and script itemisation",
        &[
            "crates/mjx-text/tests/shaping.rs",
            "crates/mjx-text/tests/itemisation.rs",
        ],
    ),
    rendered(
        "bidirectional-text",
        Section::SharedText,
        "Bidirectional text — the UBA, and a bidi-aware run order",
        &["crates/mjx-text/tests/bidirectional_text.rs"],
    ),
    rendered(
        "line-breaking",
        Section::SharedText,
        "Line breaking — UAX #14 classes and the break opportunities a layout consumes",
        &[
            "crates/mjx-text/tests/line_breaking.rs",
            "crates/mjx-layout-docx/tests/break_opportunities_that_matter.rs",
        ],
    ),
    rendered(
        "hyphenation-and-segmentation",
        Section::SharedText,
        "Hyphenation, and grapheme/word segmentation",
        &["crates/mjx-text/tests/segmentation_and_hyphenation.rs"],
    ),
    rendered(
        "font-resolution",
        Section::SharedText,
        "Font resolution — the three tiers, metric-compatible substitution, the per-document manifest",
        &[
            "crates/mjx-text/tests/metric_compatibility.rs",
            "crates/mjx-text/tests/substitution_manifest.rs",
            "crates/mjx-text/tests/empty_system_tier.rs",
        ],
    ),
    rendered(
        "glyph-rasterisation",
        Section::SharedText,
        "Glyph rasterisation and the atlas",
        &[
            "crates/mjx-text/tests/glyph_rasterisation.rs",
            "crates/mjx-text/tests/glyph_atlas.rs",
        ],
    ),
    rendered(
        "glyph-outlines",
        Section::SharedText,
        "Glyph outlines as paths, tessellated for the vector painters",
        &["crates/mjx-scene/tests/glyph_outlines_tessellate_as_paths.rs"],
    ),
    rendered(
        "font-subsetting",
        Section::SharedText,
        "Font subsetting, for an export that embeds only what it uses",
        &["crates/mjx-text/tests/a_subset_is_a_font.rs"],
    ),
    rendered(
        "text-measurement",
        Section::SharedText,
        "Character spacing, scaling, kerning and position — the measures a line is composed from",
        &[
            "crates/mjx-dml/tests/text_measures.rs",
            "crates/mjx-layout/tests/text_composition.rs",
        ],
    ),
    rendered(
        "text-effects",
        Section::SharedText,
        "Text effects — shadow, glow, reflection, bevel",
        &[
            "crates/mjx-dml/tests/effect_model.rs",
            "crates/mjx-scene-pptx/tests/every_effect_reaches_the_root.rs",
        ],
    ),
    rendered(
        "east-asian-typography",
        Section::SharedText,
        "East Asian typography — ruby, vertical text, `kinsoku` line-break rules",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.2 · Paragraphs
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "paragraph-alignment",
        Section::SharedParagraphs,
        "Alignment and justification, including the positions a justified line resolves to",
        &["crates/mjx-layout-docx/tests/a_justified_line_has_positions.rs"],
    ),
    rendered(
        "paragraph-spacing-and-indents",
        Section::SharedParagraphs,
        "Indentation, spacing before and after, line spacing, and five kinds of tab stop",
        &[
            "crates/mjx-layout-docx/tests/spacing_tabs_and_indents.rs",
            "crates/mjx-docx/tests/paragraph_properties.rs",
        ],
    ),
    rendered(
        "bullets-and-numbering",
        Section::SharedParagraphs,
        "Bullets and multilevel numbered lists, with restart and continuation",
        &[
            "crates/mjx-layout-docx/tests/a_list_composes_its_marker.rs",
            "crates/mjx-layout-pptx/tests/bullets_and_indent_levels.rs",
            "crates/mjx-docx/tests/numbering.rs",
        ],
    ),
    rendered(
        "paragraph-borders-and-rules",
        Section::SharedParagraphs,
        "Paragraph borders and shading, at every weight the format allows",
        &["crates/mjx-layout-docx/tests/no_rule_rounds_to_nothing.rs"],
    ),
    rendered(
        "pagination-controls",
        Section::SharedParagraphs,
        "Widow and orphan control, keep-with-next, keep-lines-together, page-break-before",
        &["crates/mjx-layout-docx/tests/each_constraint_moves_a_paragraph.rs"],
    ),
    rendered(
        "paragraph-hierarchy",
        Section::SharedParagraphs,
        "Paragraph and list level hierarchy in a shape's text body",
        &["crates/mjx-pptx/tests/paragraph_hierarchy.rs"],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.3 · Drawing and shapes
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "preset-geometry",
        Section::SharedDrawing,
        "The preset shape geometries and their path tables",
        &[
            "crates/mjx-geometry/tests/a_preset_renders_as_itself.rs",
            "crates/mjx-geometry/tests/every_preset_is_structurally_sound.rs",
            "crates/mjx-geometry/tests/every_preset_stands_where_its_box_is.rs",
            "crates/mjx-pptx/tests/geometry.rs",
        ],
    ),
    rendered(
        "shape-adjustments",
        Section::SharedDrawing,
        "Adjustment handles, and the guide formulas they drive",
        &[
            "crates/mjx-geometry/tests/an_adjustment_moves_the_shape.rs",
            "crates/mjx-geometry/tests/every_adjustment_moves_its_shape.rs",
            "crates/mjx-dml/tests/guide_formula.rs",
            "crates/mjx-pptx/tests/preset_adjustments.rs",
        ],
    ),
    rendered(
        "custom-geometry",
        Section::SharedDrawing,
        "Custom freeform geometry — `a:custGeom` paths",
        &[
            "crates/mjx-dml/tests/custom_geometry_model.rs",
            "crates/mjx-geometry/tests/the_third_route_is_the_parser.rs",
            "crates/mjx-pptx/tests/custom_geometry.rs",
        ],
    ),
    rendered(
        "connectors",
        Section::SharedDrawing,
        "Connectors and their connection sites",
        &["crates/mjx-geometry/tests/a_connector_lands_on_the_outline.rs"],
    ),
    rendered(
        "fills",
        Section::SharedDrawing,
        "Fills — solid, gradient with stop and tile semantics, 54 preset patterns, picture, texture",
        &[
            "crates/mjx-dml/tests/fill_model.rs",
            "crates/mjx-pptx/tests/fill.rs",
            "crates/mjx-paint/tests/the_tables_are_tables.rs",
        ],
    ),
    rendered(
        "outlines",
        Section::SharedDrawing,
        "Outlines — weight, dash, cap, join, compound, arrowheads",
        &[
            "crates/mjx-dml/tests/line_model.rs",
            "crates/mjx-paint/tests/the_tables_are_tables.rs",
        ],
    ),
    rendered(
        "effects",
        Section::SharedDrawing,
        "Effects — `outerShdw`, `innerShdw`, `glow`, `softEdge`, `reflection`, `blur`, and the effect DAG",
        &[
            "crates/mjx-dml/tests/effect_model.rs",
            "crates/mjx-scene-pptx/tests/every_effect_reaches_the_root.rs",
            "crates/mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs",
        ],
    ),
    rendered(
        "colour-resolution",
        Section::SharedDrawing,
        "Colour resolution — scheme colours and the transform chain",
        &[
            "crates/mjx-dml/tests/color_model.rs",
            "crates/mjx-dml/tests/resolve_model.rs",
            "crates/mjx-scene-xlsx/tests/the_alpha_survives.rs",
        ],
    ),
    rendered(
        "three-dimensional-shapes",
        Section::SharedDrawing,
        "3-D rotation and extrusion — `a:scene3d` and `a:sp3d`",
        &[
            "crates/mjx-dml/tests/shape3d_model.rs",
            "crates/mjx-pptx/tests/shape_3d.rs",
        ],
    ),
    rendered(
        "shape-styles",
        Section::SharedDrawing,
        "Shape styles and theme style references",
        &[
            "crates/mjx-dml/tests/style_model.rs",
            "crates/mjx-pptx/tests/shape_list_style.rs",
        ],
    ),
    rendered(
        "grouping-and-transforms",
        Section::SharedDrawing,
        "Grouping, nested group transforms, flipping and rotation",
        &[
            "crates/mjx-dml/tests/transform_model.rs",
            "crates/mjx-pptx/tests/groups.rs",
            "crates/mjx-pptx/tests/grouping.rs",
            "crates/mjx-layout-pptx/tests/nested_group_transforms_compose.rs",
        ],
    ),
    rendered(
        "z-order-and-placement",
        Section::SharedDrawing,
        "Z-order, size and position, alignment and distribution",
        &[
            "crates/mjx-pptx/tests/placement.rs",
            "crates/mjx-pptx/tests/transform.rs",
        ],
    ),
    rendered(
        "text-in-a-shape",
        Section::SharedDrawing,
        "The text body inside a shape — insets, anchoring, and the geometry that bounds it",
        &[
            "crates/mjx-dml/tests/text_model.rs",
            "crates/mjx-geometry/tests/text_goes_inside_the_shape.rs",
            "crates/mjx-layout-pptx/tests/the_body_geometry_is_honoured.rs",
        ],
    ),
    rendered(
        "wordart",
        Section::SharedDrawing,
        "WordArt — `TabSetWordArtTools`, and the text-warp preset geometries",
        &[],
    ),
    rendered(
        "snapping-and-guides",
        Section::SharedDrawing,
        "Snapping, alignment guides and the editing affordances around a shape",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.4 · Pictures
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "picture-insertion",
        Section::SharedPictures,
        "Pictures — insertion, the media part graph, and the relationship that reaches the bytes",
        &[
            "crates/mjx-pptx/tests/pictures.rs",
            "crates/mjx-pptx/tests/images.rs",
        ],
    ),
    rendered(
        "picture-anchoring",
        Section::SharedPictures,
        "The anchoring models — inline, floating, one-cell, two-cell, absolute",
        &[
            "crates/mjx-dml/tests/spreadsheet_drawing_model.rs",
            "crates/mjx-sml/tests/anchor_geometry.rs",
            "crates/mjx-layout-xlsx/tests/three_anchor_modes_move_differently.rs",
            "crates/mjx-docx/tests/drawing_placement.rs",
        ],
    ),
    rendered(
        "picture-cropping",
        Section::SharedPictures,
        "Cropping, including crop-to-shape and aspect fill",
        &[],
    ),
    rendered(
        "picture-corrections",
        Section::SharedPictures,
        "Corrections and colour — brightness, contrast, saturation, recolour, artistic effects",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.5 · Charts
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "chart-model",
        Section::SharedCharts,
        "The chart model — plot types, series, and the parts that carry them",
        &[
            "crates/mjx-chart/tests/plot_types.rs",
            "crates/mjx-chart/tests/bar_model.rs",
            "crates/mjx-chart/tests/author.rs",
            "crates/mjx-chart/tests/edit.rs",
        ],
    ),
    rendered(
        "chart-series-geometry",
        Section::SharedCharts,
        "Series geometry — every chart family producing marks in the right places",
        &[
            "crates/mjx-layout-chart/tests/series_geometry_is_asserted.rs",
            "crates/mjx-layout-chart/tests/every_family_produces_marks.rs",
            "crates/mjx-layout-chart/tests/degenerate_data_is_defined.rs",
        ],
    ),
    rendered(
        "chart-axes-and-scales",
        Section::SharedCharts,
        "Axes, scales and tick selection",
        &[
            "crates/mjx-layout-chart/tests/ticks_are_arithmetic.rs",
            "crates/mjx-layout-chart/tests/the_furniture_is_drawn.rs",
        ],
    ),
    rendered(
        "chart-furniture",
        Section::SharedCharts,
        "Titles, legends, data labels and gridlines",
        &[
            "crates/mjx-chart/tests/furniture.rs",
            "crates/mjx-layout-chart/tests/the_furniture_is_drawn.rs",
        ],
    ),
    rendered(
        "chart-decoration",
        Section::SharedCharts,
        "Series and data-point formatting, and the document palette a series inherits",
        &[
            "crates/mjx-chart/tests/decoration.rs",
            "crates/mjx-layout-chart/tests/the_document_palette_wins.rs",
        ],
    ),
    rendered(
        "chart-embedded-workbook",
        Section::SharedCharts,
        "The embedded workbook that backs the data, and its external-data alternative",
        &[
            "crates/mjx-chart/tests/data_sources.rs",
            "crates/mjx-chart/tests/external_data.rs",
            "crates/mjx-pptx/tests/chart_workbook.rs",
        ],
    ),
    rendered(
        "chart-unmodelled-plot-types",
        Section::SharedCharts,
        "The plot types `mjx-chart` preserves without modelling",
        &["crates/mjx-chart/tests/unmodelled_plot_types.rs"],
    ),
    rendered(
        "chart-in-three-formats",
        Section::SharedCharts,
        "A chart reached through PowerPoint's, Word's and Excel's box models",
        &[
            "crates/mjx-layout-chart/tests/fragments_reach_the_tree.rs",
            "crates/mjx-docx/tests/charts.rs",
            "crates/mjx-xlsx/tests/charts.rs",
            "crates/mjx-pptx/tests/charts.rs",
        ],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.6 · SmartArt and diagrams
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "diagram-parts",
        Section::SharedDiagrams,
        "The four-part diagram model — data, layout, style, colours",
        &[
            "crates/mjx-pptx/tests/diagrams.rs",
            "crates/mjx-pptx/tests/diagram_read_back.rs",
        ],
    ),
    rendered(
        "diagram-layout",
        Section::SharedDiagrams,
        "Diagram layout — a hierarchy solved into boxes",
        &["crates/mjx-layout-chart/tests/a_hierarchy_is_laid_out.rs"],
    ),
    rendered(
        "diagram-layout-catalogue",
        Section::SharedDiagrams,
        "The ~200 published layouts across eight categories, and their algorithm evaluation",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.7 · Mathematics
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "omml-model",
        Section::SharedMath,
        "The OMML model — fractions, radicals, n-ary operators, matrices, delimiters, accents",
        &[
            "crates/mjx-omml/tests/deep_nesting.rs",
            "crates/mjx-docx/tests/equations.rs",
        ],
    ),
    rendered(
        "math-typesetting",
        Section::SharedMath,
        "Typesetting an equation — bar positions, script scale, delimiter growth, array alignment",
        &["crates/mjx-layout-docx/tests/an_equation_is_typeset.rs"],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.8 · Tables
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "table-model",
        Section::SharedTables,
        "Table structure — rows, columns, cells, insertion and deletion",
        &[
            "crates/mjx-dml/tests/table_model.rs",
            "crates/mjx-pptx/tests/tables.rs",
            "crates/mjx-pptx/tests/table_structure.rs",
            "crates/mjx-docx/tests/tables.rs",
        ],
    ),
    rendered(
        "table-styles",
        Section::SharedTables,
        "Table styles and the six conditional-formatting bands",
        &[
            "crates/mjx-dml/tests/table_style.rs",
            "crates/mjx-pptx/tests/table_styles.rs",
            "crates/mjx-pptx/tests/table_effective.rs",
        ],
    ),
    rendered(
        "table-merging",
        Section::SharedTables,
        "Merged and split cells, and the walk that must not be naive",
        &[
            "crates/mjx-pptx/tests/table_merging.rs",
            "crates/mjx-layout-pptx/tests/merged_cells_are_not_a_naive_walk.rs",
        ],
    ),
    rendered(
        "table-grid-solving",
        Section::SharedTables,
        "Column sizing — the fixed and autofit layout algorithms",
        &["crates/mjx-layout-docx/tests/a_table_grid_is_solved.rs"],
    ),
    rendered(
        "table-splitting",
        Section::SharedTables,
        "Tables that split across pages, with repeated header rows",
        &["crates/mjx-layout-docx/tests/a_table_splits_across_a_page.rs"],
    ),
    rendered(
        "table-borders",
        Section::SharedTables,
        "Cell borders and shading, with the resolution precedence between them",
        &[
            "crates/mjx-pptx/tests/table_formatting.rs",
            "crates/mjx-docx/tests/table_formatting.rs",
            "crates/mjx-layout-pptx/tests/a_cell_border_is_a_band_that_covers_pixels.rs",
        ],
    ),
    // A selection is not markup, so `preserved-not-rendered` would be nonsense for it: there is
    // nothing to draw and nothing failing to be drawn. It is an editing behaviour that happens to
    // live in §3.8, which is why `Kind` and `Section` are separate.
    behaviour(
        "table-selection",
        Section::SharedTables,
        "Formatting a region of cells in one call, and the smaller cell surfaces beside it — accessibility headers, visible cell text",
        &[
            "crates/mjx-pptx/tests/table_selection.rs",
            "crates/mjx-pptx/tests/table_gaps.rs",
        ],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §3.9 · Cross-cutting document surfaces
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "themes",
        Section::SharedDocument,
        "Themes — colour schemes, font schemes, effect schemes",
        &[
            "crates/mjx-dml/tests/theme_model.rs",
            "crates/mjx-pptx/tests/theme.rs",
        ],
    ),
    rendered(
        "styles-and-inheritance",
        Section::SharedDocument,
        "Styles, style sets, and every inheritance ladder a property resolves through",
        &[
            "crates/mjx-docx/tests/styles.rs",
            "crates/mjx-docx/tests/effective.rs",
            "crates/mjx-pptx/tests/text_inheritance.rs",
            "crates/mjx-pptx/tests/transform_inheritance.rs",
            "crates/mjx-layout-pptx/tests/the_ladder_is_consumed.rs",
            "crates/mjx-layout-docx/tests/the_ladder_is_consumed.rs",
            "crates/mjx-layout-xlsx/tests/the_ladder_is_consumed.rs",
        ],
    ),
    rendered(
        "comments",
        Section::SharedDocument,
        "Comments, both legacy and threaded",
        &[
            "crates/mjx-docx/tests/annotations.rs",
            "crates/mjx-xlsx/tests/comments.rs",
        ],
    ),
    rendered(
        "hyperlinks",
        Section::SharedDocument,
        "Hyperlinks, and the relationships that carry their targets",
        &[
            "crates/mjx-pptx/tests/hyperlinks.rs",
            "crates/mjx-sml/tests/worksheet_hyperlinks.rs",
            "crates/mjx-xlsx/tests/hyperlinks.rs",
        ],
    ),
    behaviour(
        "round-trip-fidelity",
        Section::SharedDocument,
        "The round-trip contract — untouched parts re-emitted byte for byte",
        &[
            "crates/mjx-opc/tests/roundtrip.rs",
            "crates/mjx-opc/tests/tree_roundtrip.rs",
            "crates/mjx-pptx/tests/roundtrip.rs",
            "crates/mjx-docx/tests/roundtrip.rs",
            "crates/mjx-xlsx/tests/roundtrip.rs",
            "crates/mjx-xlsx/tests/preserved_parts.rs",
        ],
    ),
    behaviour(
        "markup-compatibility",
        Section::SharedDocument,
        "MCE — `mc:AlternateContent`, `Ignorable`, `ProcessContent`",
        &[
            "crates/mjx-mce/tests/resolve.rs",
            "crates/mjx-mce/tests/untrusted_input.rs",
        ],
    ),
    behaviour(
        "schema-conformance",
        Section::SharedDocument,
        "ECMA-376 schema validity and child order, for everything this library writes",
        &[
            "crates/mjx-schema-gate/tests/ordering.rs",
            "crates/mjx-schema-gate/tests/category_rule.rs",
            "crates/mjx-docx/tests/schema_gate.rs",
            "crates/mjx-xlsx/tests/schema_gate.rs",
        ],
    ),
    behaviour(
        "untrusted-input",
        Section::SharedDocument,
        "Untrusted input — a malformed file is a typed error and never a panic",
        &[
            "crates/mjx-xml/tests/untrusted_input.rs",
            "crates/mjx-opc/tests/untrusted_input.rs",
            "crates/mjx-text/tests/untrusted_faces.rs",
            "crates/mjx-text/tests/untrusted_text.rs",
            "crates/mjx-layout-docx/tests/no_panic_on_a_layout_path.rs",
            "crates/mjx-layout-pptx/tests/no_panic_on_a_layout_path.rs",
            "crates/mjx-layout-xlsx/tests/no_panic_on_a_layout_path.rs",
            "crates/mjx-layout-chart/tests/no_panic_on_a_layout_path.rs",
        ],
    ),
    rendered(
        "document-properties",
        Section::SharedDocument,
        "Document properties and metadata",
        &["crates/mjx-opc/tests/package_validation.rs"],
    ),
    behaviour(
        "export-pdf-and-svg",
        Section::SharedDocument,
        "Export — PDF with selectable text, and SVG, both from the display list",
        &[
            "crates/mjx-paint/tests/a_document_is_a_document.rs",
            "crates/mjx-render-oracle/tests/the_pdf_tiers_work_on_our_own_exports.rs",
        ],
    ),
    rendered(
        "find-and-replace",
        Section::SharedDocument,
        "Find and replace, including formatting and wildcards",
        &[],
    ),
    rendered(
        "spelling-and-grammar",
        Section::SharedDocument,
        "Spell check, grammar, and the proofing language settings",
        &[],
    ),
    behaviour(
        "clipboard",
        Section::SharedDocument,
        "The clipboard, its four flavours, and paste-special",
        &[],
    ),
    behaviour(
        "protection-and-signatures",
        Section::SharedDocument,
        "Document protection, encryption and digital signatures",
        &[],
    ),
    rendered(
        "print-and-page-setup",
        Section::SharedDocument,
        "Print and page setup, across the three applications",
        &[
            "crates/mjx-layout-xlsx/tests/print_layout_paginates.rs",
            "crates/mjx-sml/tests/print_and_sheet_kinds.rs",
        ],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §4.1 · Word
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "word-flow-and-pagination",
        Section::Word,
        "Flow and pagination — a document becoming pages, consistently",
        &[
            "crates/mjx-layout-docx/tests/a_document_becomes_fragments.rs",
            "crates/mjx-layout-docx/tests/a_long_document_paginates_consistently.rs",
            "crates/mjx-layout-docx/tests/the_fragments_match_their_baselines.rs",
        ],
    ),
    rendered(
        "word-sections",
        Section::Word,
        "Sections — page size, orientation, margins, and section-scoped numbering",
        &[
            "crates/mjx-docx/tests/sections.rs",
            "crates/mjx-layout-docx/tests/a_section_changes_the_page.rs",
        ],
    ),
    rendered(
        "word-columns",
        Section::Word,
        "Multiple columns, and the balance at a continuous break",
        &["crates/mjx-layout-docx/tests/columns_balance_at_a_continuous_break.rs"],
    ),
    rendered(
        "word-headers-and-footers",
        Section::Word,
        "Headers and footers — first page, odd and even, and per section",
        &[
            "crates/mjx-docx/tests/headers.rs",
            "crates/mjx-layout-docx/tests/the_headers_differ_by_page.rs",
        ],
    ),
    rendered(
        "word-floating-objects",
        Section::Word,
        "Floating objects and text wrapping — `square`, `tight`, `through`, `topAndBottom`, behind, in front",
        &["crates/mjx-layout-docx/tests/text_wraps_around_a_float.rs"],
    ),
    rendered(
        "word-footnotes-and-endnotes",
        Section::Word,
        "Footnotes and endnotes — their own reflow, and the mark on the line",
        &[
            "crates/mjx-layout-docx/tests/a_footnote_moves_the_body.rs",
            "crates/mjx-layout-docx/tests/endnotes_flow_at_the_end_of_their_scope.rs",
            "crates/mjx-layout-docx/tests/a_generated_mark_is_measured.rs",
        ],
    ),
    rendered(
        "word-fields",
        Section::Word,
        "Fields — the 90+ types, their computation, and the fixed point an update reaches",
        &[
            "crates/mjx-docx/tests/fields.rs",
            "crates/mjx-layout-docx/tests/a_field_fixed_point_terminates.rs",
            "crates/mjx-layout-docx/tests/a_stale_field_is_recomputed.rs",
        ],
    ),
    rendered(
        "word-track-changes",
        Section::Word,
        "Track changes — insertions, deletions, formatting revisions, and their effect on layout",
        &[
            "crates/mjx-docx/tests/revisions.rs",
            "crates/mjx-layout-docx/tests/a_deletion_changes_the_page.rs",
        ],
    ),
    rendered(
        "word-line-numbers",
        Section::Word,
        "Line numbers, restarting per the four modes",
        &["crates/mjx-layout-docx/tests/line_numbers_restart_per_mode.rs"],
    ),
    rendered(
        "word-content-controls",
        Section::Word,
        "Content controls, bookmarks and structured document tags",
        &[
            "crates/mjx-docx/tests/structured_content.rs",
            "crates/mjx-docx/tests/content_model.rs",
        ],
    ),
    behaviour(
        "word-resumable-layout",
        Section::Word,
        "The checkpoint that makes flow layout resumable, across tables and floats",
        &[
            "crates/mjx-layout-docx/tests/a_checkpoint_is_work_not_output.rs",
            "crates/mjx-layout-docx/tests/a_checkpoint_survives_tables_and_floats.rs",
            "crates/mjx-layout-docx/tests/termination.rs",
        ],
    ),
    rendered(
        "word-reaches-pixels",
        Section::Word,
        "A Word document reaching a display list, and then pixels",
        &[],
    ),
    rendered(
        "word-drop-caps-and-frames",
        Section::Word,
        "Drop caps, text frames and watermarks",
        &[],
    ),
    rendered(
        "word-toc-and-index",
        Section::Word,
        "Tables of contents, indexes, tables of authorities and captions, as generated content",
        &[],
    ),
    rendered(
        "word-outline-and-master-documents",
        Section::Word,
        "Outline view and master documents — `TabOutlining`",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §4.2 · Excel
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "excel-number-format-engine",
        Section::Excel,
        "The number-format engine — `numFmt` to display string, and its degradation on a bad code",
        &[
            "crates/mjx-layout-xlsx/tests/the_format_language_is_evaluated.rs",
            "crates/mjx-layout-xlsx/tests/a_cell_shows_its_formatted_value.rs",
            "crates/mjx-layout-xlsx/tests/a_broken_format_degrades.rs",
        ],
    ),
    rendered(
        "excel-grid-layout",
        Section::Excel,
        "Grid layout — row and column sizing, a windowed and sparse read of a large sheet",
        &[
            "crates/mjx-layout-xlsx/tests/a_sheet_becomes_fragments.rs",
            "crates/mjx-layout-xlsx/tests/the_grid_is_windowed_and_sparse.rs",
            "crates/mjx-layout-xlsx/tests/sparsity_is_measured.rs",
        ],
    ),
    rendered(
        "excel-merged-regions",
        Section::Excel,
        "Merged regions, rendered once",
        &["crates/mjx-layout-xlsx/tests/merged_regions_render_once.rs"],
    ),
    rendered(
        "excel-text-overflow",
        Section::Excel,
        "Text overflowing into empty neighbours, and the rules that clip it",
        &["crates/mjx-layout-xlsx/tests/text_overflows_into_empty_neighbours.rs"],
    ),
    rendered(
        "excel-panes",
        Section::Excel,
        "Frozen and split panes over one geometry",
        &["crates/mjx-layout-xlsx/tests/frozen_panes_share_one_geometry.rs"],
    ),
    rendered(
        "excel-conditional-formatting",
        Section::Excel,
        "Conditional formatting — every rule kind, graded interpolation, and Excel's precedence",
        &[
            "crates/mjx-sml/tests/conditional_formatting.rs",
            "crates/mjx-xlsx/tests/conditional_formatting.rs",
            "crates/mjx-layout-xlsx/tests/every_rule_kind_fires_and_does_not.rs",
            "crates/mjx-layout-xlsx/tests/graded_rules_interpolate.rs",
            "crates/mjx-layout-xlsx/tests/precedence_composes_in_excels_order.rs",
            "crates/mjx-layout-xlsx/tests/the_conditional_ledger_is_computed.rs",
            "crates/mjx-scene-xlsx/tests/a_conditional_format_changes_a_pixel.rs",
        ],
    ),
    rendered(
        "excel-cell-borders",
        Section::Excel,
        "Cell borders — every weight reaching a pixel, and the dash that does not",
        &[
            "crates/mjx-layout-xlsx/tests/no_border_rounds_to_nothing.rs",
            "crates/mjx-scene-xlsx/tests/the_dash_is_lost_at_the_band.rs",
        ],
    ),
    rendered(
        "excel-cell-formatting",
        Section::Excel,
        "The effective cell format — the `xf` chain, fills, fonts and alignment",
        &[
            "crates/mjx-sml/tests/effective_cell_format.rs",
            "crates/mjx-sml/tests/style_resources.rs",
            "crates/mjx-xlsx/tests/effective_format.rs",
            "crates/mjx-scene-xlsx/tests/a_red_negative_reaches_the_paint_table.rs",
        ],
    ),
    rendered(
        "excel-print-layout",
        Section::Excel,
        "Print layout — page breaks, print areas, repeated rows and scaling",
        &["crates/mjx-layout-xlsx/tests/print_layout_paginates.rs"],
    ),
    rendered(
        "excel-tables-and-filters",
        Section::Excel,
        "Tables, autofilters and sorting",
        &[
            "crates/mjx-sml/tests/worksheet_tables.rs",
            "crates/mjx-sml/tests/validation_and_filters.rs",
            "crates/mjx-xlsx/tests/worksheet_tables.rs",
            "crates/mjx-xlsx/tests/validation_and_filters.rs",
        ],
    ),
    rendered(
        "excel-drawings",
        Section::Excel,
        "Cell drawings and worksheet objects",
        &[
            "crates/mjx-sml/tests/worksheet_objects.rs",
            "crates/mjx-xlsx/tests/worksheet_drawings.rs",
        ],
    ),
    rendered(
        "excel-reaches-pixels",
        Section::Excel,
        "A worksheet reaching a display list, and then pixels",
        &[
            "crates/mjx-scene-xlsx/tests/a_real_sheet_resolves.rs",
            "crates/mjx-reference-pack/tests/a_real_worksheet_reaches_pixels.rs",
        ],
    ),
    behaviour(
        "excel-cell-store",
        Section::Excel,
        "The packed cell store — what holding a large sheet costs, measured rather than asserted",
        &[
            "crates/mjx-sml/tests/cell_store_allocation.rs",
            "crates/mjx-sml/tests/cell_store_fidelity.rs",
            "crates/mjx-sml/tests/shared_string_allocation.rs",
        ],
    ),
    rendered(
        "excel-gridlines",
        Section::Excel,
        "Sheet gridlines — the ruled lines a worksheet shows where no border is set",
        &[],
    ),
    rendered(
        "excel-sparklines",
        Section::Excel,
        "Sparklines — `x14:sparklineGroups`, preserved in an `extLst` and unmodelled",
        &[],
    ),
    rendered(
        "excel-pivot-tables",
        Section::Excel,
        "Pivot tables and pivot charts — `TabSetPivotTableTools`, 570 controls across the two",
        &[],
    ),
    rendered(
        "excel-data-tools",
        Section::Excel,
        "Data tools — text to columns, flash fill, remove duplicates, what-if analysis",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §4.3 · PowerPoint
    // ─────────────────────────────────────────────────────────────────────────────────────────
    rendered(
        "pptx-slide-layout",
        Section::PowerPoint,
        "A slide becoming fragments, against a committed baseline",
        &[
            "crates/mjx-layout-pptx/tests/a_slide_becomes_fragments.rs",
            "crates/mjx-layout-pptx/tests/the_fragments_match_their_baselines.rs",
        ],
    ),
    rendered(
        "pptx-master-and-layout",
        Section::PowerPoint,
        "The master, layout, notes and handout hierarchy, and placeholder inheritance",
        &[
            "crates/mjx-pptx/tests/layouts.rs",
            "crates/mjx-pptx/tests/surfaces.rs",
            "crates/mjx-layout-pptx/tests/the_ladder_is_consumed.rs",
        ],
    ),
    rendered(
        "pptx-autofit",
        Section::PowerPoint,
        "Autofit — the search that makes text fit its body",
        &["crates/mjx-layout-pptx/tests/autofit_is_a_search.rs"],
    ),
    rendered(
        "pptx-notes",
        Section::PowerPoint,
        "Notes pages, laid out as slides by another name",
        &[
            "crates/mjx-pptx/tests/notes.rs",
            "crates/mjx-layout-pptx/tests/a_notes_page_is_a_slide_by_another_name.rs",
        ],
    ),
    // Authoring is an editing behaviour, not markup waiting to be drawn — so the rendering tier is
    // not the question for it, and `preserved-not-rendered` would be the wrong answer to a question
    // nobody asked.
    behaviour(
        "pptx-slide-authoring",
        Section::PowerPoint,
        "Creating, removing and reordering slides",
        &[
            "crates/mjx-pptx/tests/slide_creation.rs",
            "crates/mjx-pptx/tests/removal.rs",
            "crates/mjx-pptx/tests/blank_document.rs",
        ],
    ),
    rendered(
        "pptx-reaches-pixels",
        Section::PowerPoint,
        "A deck reaching a display list, and then pixels, through two independent painters",
        &[
            "crates/mjx-scene-pptx/tests/every_effect_reaches_the_root.rs",
            "crates/mjx-paint/tests/a_page_becomes_pixels.rs",
            "crates/mjx-paint/tests/two_painters_agree.rs",
            "crates/mjx-reference-pack/tests/a_real_deck_reaches_pixels.rs",
        ],
    ),
    behaviour(
        "pptx-hit-testing",
        Section::PowerPoint,
        "A point finding its run, and the addressing agreeing with the session's",
        &[
            "crates/mjx-layout-pptx/tests/a_point_finds_its_run.rs",
            "crates/mjx-layout-pptx/tests/the_addressing_agrees_with_the_session.rs",
        ],
    ),
    rendered(
        "pptx-media",
        Section::PowerPoint,
        "Audio and video — the parts, the timing markup, and playback",
        &["crates/mjx-pptx/tests/media.rs"],
    ),
    rendered(
        "pptx-ole-and-activex",
        Section::PowerPoint,
        "OLE objects and ActiveX controls",
        &[
            "crates/mjx-pptx/tests/ole.rs",
            "crates/mjx-pptx/tests/activex.rs",
        ],
    ),
    rendered(
        "vml-legacy",
        Section::PowerPoint,
        "VML — the legacy drawing markup, with shape-level references",
        &[
            "crates/mjx-vml/tests/drawing.rs",
            "crates/mjx-pptx/tests/vml.rs",
        ],
    ),
    rendered(
        "pptx-animation-and-timing",
        Section::PowerPoint,
        "Animation and timing — `p:timing`, the trigger tree, and the runtime that evaluates it",
        &[],
    ),
    rendered(
        "pptx-transitions",
        Section::PowerPoint,
        "Transitions — ~48 types with their effect options and advance rules",
        &[],
    ),
    behaviour(
        "pptx-slide-show",
        Section::PowerPoint,
        "Slide show — presenter view, rehearsed timings, custom shows, hidden slides",
        &[],
    ),
    rendered(
        "pptx-sections-and-morph",
        Section::PowerPoint,
        "Sections, slide sorter, zoom links and morph",
        &[],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §5 · The calculation engine
    // ─────────────────────────────────────────────────────────────────────────────────────────
    excluded(
        "calculation-engine",
        Section::Calculation,
        "The calculation engine — the dependency graph, ~500 functions, dynamic arrays, the error lattice",
        "`PLAN.md`'s *Explicitly out of scope for v1* names it, and \
         `crates/mjx-xlsx/docs/guide/deliberate_limitations.md` gathers what follows. The inventory \
         §5 treats it as its own programme rather than a feature; it gates Excel **editing** only, \
         since rendering an unedited workbook uses the cached `<v>` already in the file",
    ),
    rendered(
        "cached-values-are-rendered",
        Section::Calculation,
        "A formula's cached `<v>` reaching the screen, which is what makes a viewer possible without an engine",
        &[
            "crates/mjx-sml/tests/formulas.rs",
            "crates/mjx-xlsx/tests/formulas.rs",
            "crates/mjx-layout-xlsx/tests/a_cell_shows_its_formatted_value.rs",
        ],
    ),
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // §6 · The third axis
    // ─────────────────────────────────────────────────────────────────────────────────────────
    behaviour(
        "hit-testing",
        Section::ThirdAxis,
        "Hit testing — the spatial index that makes it a query rather than a walk",
        &[
            "crates/mjx-layout/tests/spatial_index.rs",
            "crates/mjx-layout/tests/fragment_tree.rs",
        ],
    ),
    behaviour(
        "selection-and-caret",
        Section::ThirdAxis,
        "Selection and caret — grapheme granularity, and a point resolving to a document position",
        &[
            "crates/mjx-layout/tests/text_composition.rs",
            "crates/mjx-layout-pptx/tests/a_point_finds_its_run.rs",
        ],
    ),
    behaviour(
        "undo-and-redo",
        Section::ThirdAxis,
        "Undo and redo — command granularity, and the coalescing of consecutive keystrokes",
        &[
            "crates/mjx-session/tests/undo_granularity.rs",
            "crates/mjx-session/tests/batching.rs",
            "crates/mjx-session/tests/journal_allocation.rs",
        ],
    ),
    behaviour(
        "document-lifecycle",
        Section::ThirdAxis,
        "Open, autosave, crash recovery, dirty tracking, and the residency budget a session holds",
        &[
            "crates/mjx-session/tests/recovery.rs",
            "crates/mjx-session/tests/fidelity.rs",
            "crates/mjx-session/tests/residency_budget.rs",
        ],
    ),
    behaviour(
        "viewport-and-scrolling",
        Section::ThirdAxis,
        "The viewport — windowing, invalidation, scroll stability and the frame schedule",
        &[
            "crates/mjx-view/tests/windowing.rs",
            "crates/mjx-view/tests/invalidation.rs",
            "crates/mjx-view/tests/scroll_stability.rs",
            "crates/mjx-view/tests/frame_schedule.rs",
        ],
    ),
    behaviour(
        "memory-budgets",
        Section::ThirdAxis,
        "The declared memory budgets — resident documents, the glyph atlas, the mesh and texture pools",
        &[
            "crates/mjx-view/tests/resident_memory.rs",
            "crates/mjx-view/tests/the_declared_budgets_add_up.rs",
            "crates/mjx-text/tests/glyph_atlas_allocation.rs",
            "crates/mjx-scene/tests/the_mesh_cache_holds_its_budget.rs",
            "crates/mjx-paint/tests/the_texture_pool_holds_its_budget.rs",
        ],
    ),
    behaviour(
        "the-fidelity-oracle",
        Section::ThirdAxis,
        "The oracle — three assertion tiers, failing independently, over approved baselines",
        &[
            "crates/mjx-render-oracle/tests/the_tiers_fail_independently.rs",
            "crates/mjx-render-oracle/tests/an_unapproved_baseline_fails.rs",
            "crates/mjx-render-oracle/tests/a_regression_arrives_with_its_picture.rs",
            "crates/mjx-render-oracle/tests/regenerating_a_baseline_is_explicit.rs",
        ],
    ),
    behaviour(
        "the-reference-pack",
        Section::ThirdAxis,
        "The reference pack — the artefacts one Windows sitting needs, and the ingest that reads them back",
        &[
            "crates/mjx-reference-pack/tests/the_instructions_are_complete.rs",
            "crates/mjx-reference-pack/tests/the_decks_generate_reproducibly.rs",
            "crates/mjx-reference-pack/tests/the_office_corpus_ships_empty.rs",
            "crates/mjx-reference-pack/tests/the_readers_answer_from_a_real_export.rs",
        ],
    ),
    behaviour(
        "the-box-model-seam",
        Section::ThirdAxis,
        "The seam — above a `FragmentTree`, nothing has heard of OOXML, and a foreign box model works",
        &[
            "crates/mjx-layout/tests/the_seam_holds.rs",
            "crates/mjx-layout/tests/foreign_box_model.rs",
            "crates/mjx-scene/tests/fragments_alone_drive_the_builder.rs",
            "crates/mjx-paint/tests/the_seam_holds.rs",
        ],
    ),
    behaviour(
        "design-tokens",
        Section::ThirdAxis,
        "One design-token source reaching three generated consumers, which agree with each other",
        &["crates/mjx-tokens/tests/artefacts_agree.rs"],
    ),
    behaviour(
        "input-and-ime",
        Section::ThirdAxis,
        "Input — pointer, touch, pen with pressure, the shortcut map, and IME composition",
        &[],
    ),
    behaviour(
        "editing-feel",
        Section::ThirdAxis,
        "Editing feel — drag with snapping, live resize previews, marching ants, fling scrolling",
        &[],
    ),
    behaviour(
        "accessibility-tree",
        Section::ThirdAxis,
        "An accessibility tree derived from the fragment tree, and keyboard-only operation",
        &[],
    ),
    behaviour(
        "responsive-chrome",
        Section::ThirdAxis,
        "Desktop, tablet and phone presentations of the same commands, and command demotion",
        &[],
    ),
    behaviour(
        "localisation",
        Section::ThirdAxis,
        "UI strings, RTL mirroring of the chrome, and locale-aware numbers, dates and typography",
        &[],
    ),
];
