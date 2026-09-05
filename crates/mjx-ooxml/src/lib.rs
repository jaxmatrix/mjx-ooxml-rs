//! `mjx-ooxml` — the binding-ready public API for the mjx-ooxml-rs workspace, and its documentation
//! hub.
//!
//! `mjx-ooxml-rs` is a **pure-Rust** library for parsing, editing, generating, and (later) rendering
//! Office Open XML documents — PowerPoint (`.pptx`), Word (`.docx`), and Excel (`.xlsx`). The goal is
//! to open *any* OOXML file, load it fully into RAM, edit it at runtime, and write it back **without
//! corrupting the parts you did not touch** — with a codebase that cross-compiles cleanly to desktop,
//! Android, iOS, and WebAssembly.
//!
//! This crate is where an application starts. Four things live here and nowhere else:
//!
//! 1. [`detect_format`] — what a package *is*, read from its main part rather than its filename, so
//!    `.pptm` and `.potx` are recognized and a renamed `.docx` is not mistaken for a deck.
//! 2. [`Deck`] — the whole PowerPoint surface with concrete types: [`Surface`] and [`ShapePath`]
//!    instead of `impl Into<…>`, `u32` instead of `usize`, `&str` instead of part-name handles.
//! 3. [`Document`] — the curated Word surface, the same treatment: [`BlockPath`]/[`RunPath`] instead
//!    of `impl Into<…>`, `u32` instead of `usize`, a concrete return in place of every closure
//!    `mjx_docx::Document` takes to read or edit a part.
//! 4. [`Error`] — one error type carrying a stable [`ErrorCode`], a human message, and the indices
//!    that say *where*, with the full typed cause still reachable through
//!    [`source`](std::error::Error::source).
//!
//! Everything a caller needs to name is re-exported here, so **nothing downstream ever names
//! `mjx-dml`, `mjx-chart`, `mjx-opc` or `mjx-pptx`**.
//!
//! ```no_run
//! use mjx_ooxml::{CharacterPropertiesSpec, ColorSpec, Deck, FillSpec, ShapeBounds, SlideSize};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut deck = Deck::blank(SlideSize::widescreen())?;
//! let slide = deck.add_slide_from_layout(0)?;
//! let title = deck.add_text_box(
//!     slide.into(),
//!     "Quarterly results",
//!     ShapeBounds::from_inches(0.5, 0.4, 9.0, 1.2),
//! )?;
//! deck.set_shape_run_properties(
//!     slide.into(),
//!     title.into(),
//!     &CharacterPropertiesSpec::new()
//!         .with_size_points(40.0)
//!         .with_color(ColorSpec::Srgb("1F3864".into())),
//! )?;
//! deck.set_shape_fill(
//!     slide.into(),
//!     title.into(),
//!     &FillSpec::solid(ColorSpec::Srgb("FFFFFF".into())),
//! )?;
//! std::fs::write("out.pptx", deck.save()?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Note where the file I/O is: in the caller. This library is bytes-in and bytes-out and never
//! touches a filesystem, a clock, a thread or a random number generator — which is exactly why the
//! same calls work unchanged in a browser.
//!
//! # This crate versus `mjx-pptx`
//!
//! [`Deck`] is [`Presentation`] reshaped, not replaced. Reach past it — with
//! [`Deck::presentation_mut`] — when you want the Rust-only ergonomics it cannot carry across a
//! foreign function boundary: the [`ShapeCursor`](mjx_pptx::ShapeCursor) that states an address once
//! and applies several edits together, and the closure-taking table-style and VML readers. See the
//! [`deck`] module documentation for the complete list of what is left out and why.
//!
//! # The layered workspace
//!
//! Dependencies point strictly downward; each layer builds on the ones below it.
//!
//! - **Foundations** — [`mjx_ooxml_core`] (string [interner](mjx_ooxml_core::Interner) + the raw
//!   [preservation tree](mjx_ooxml_core::RawDocument)) and [`mjx_xml`] (the byte-preserving
//!   [`fidelity`](mjx_xml::fidelity) reader/writer — the only place `quick-xml` is used).
//! - **Packaging & compatibility** — [`mjx_opc`] (the OPC ZIP container and part graph, e.g.
//!   [`Package`](mjx_opc::Package)), [`mjx_mce`] (Markup Compatibility [`resolve`](mjx_mce::resolve) /
//!   preserve), and [`mjx_ooxml_types`] (generated, comprehensively-named simple types + namespaces).
//! - **Shared markup** — [`mjx_dml`] (DrawingML) and [`mjx_chart`] (ChartML).
//! - **Formats** — [`mjx_pptx`], then `mjx_docx` and `mjx_xlsx`.
//! - **Facade** — this crate. Nothing depends on it.
//!
//! # Fidelity model
//!
//! A package is a graph of parts kept as raw bytes; a part is materialized into a typed model only on
//! demand, and unmodified parts serialize back **verbatim**. Editing one slide cannot disturb the
//! theme, masters, or vendor parts, because they were never deserialized. See [`mjx_opc`] and
//! [`mjx_xml::fidelity`] for the mechanics.
//!
//! [`Deck::save`] inherits that model whole, validation included: it will refuse to write a deck that
//! breaks a packaging or PresentationML invariant, exactly as `Presentation::save` does.
//!
//! # Guides
//!
//! Start with [the PowerPoint guide](mjx_pptx::guide) — five pages, in reading order:
//!
//! - [Building a deck](mjx_pptx::guide::building_a_deck) — the whole story once, end to end.
//! - [Shapes and text](mjx_pptx::guide::shapes_and_text) — the one shape index space, group descent,
//!   surfaces, the four text selection scopes.
//! - [Tables, charts and pictures](mjx_pptx::guide::tables_charts_pictures) — structured content.
//! - [Inheritance, layouts and masters](mjx_pptx::guide::inheritance_and_masters) — where a property
//!   comes from when the slide does not state it.
//! - [Fidelity and the known gaps](mjx_pptx::guide::fidelity_and_gaps) — the round-trip guarantee,
//!   and an honest list of what is not modelled.
//!
//! It is written against `Presentation`; every call translates to [`Deck`] by the table in the
//! [`deck`] module. `examples/build_a_deck.rs` in this crate is the same walkthrough written through
//! the facade, naming no lower crate.
//!
//! Then [Effective properties](mjx_pptx::effective_properties) — the deep reference on what a `.pptx`
//! *states* versus what a renderer *shows*, and the inheritance ladders the `effective_*` readers
//! walk to get from one to the other.
//!
//! For Word, [`mjx_docx::guide`] carries the same shape — five pages plus a README, in the same
//! reading order:
//!
//! - [Building a document](mjx_docx::guide::building_a_document) — the whole story once, end to end.
//! - [Text, runs and annotations](mjx_docx::guide::text_and_formatting) — addressing a run, editing
//!   it precisely, and the comments, notes and bookmarks that hang off it.
//! - [Tables, sections, headers and structured content](mjx_docx::guide::tables_sections_and_headers)
//!   — structured content, and the section a paragraph sits in.
//! - [Styles, numbering and inheritance](mjx_docx::guide::styles_and_inheritance) — where a
//!   property comes from when the run does not state it.
//! - [Fidelity and the known gaps](mjx_docx::guide::fidelity_and_gaps) — the round-trip guarantee,
//!   the `wml` preserve-only ledger, and an honest list of what is not modelled.
//!
//! It is written against `mjx_docx::Document`; every call translates
//! to [`Document`] (this crate's own facade type, curated rather than a full re-export — see the
//! [`document`] module's own doc comment for exactly what is curated and why) the same way the
//! PowerPoint guide translates to [`Deck`]. `examples/build_a_document.rs` in this crate is the same
//! walkthrough — open, blank, save, paragraphs and runs, a numbered list, a hyperlink, a table, a
//! header, a comment, a footnote — written through the facade, naming no lower crate.
//! [`mjx_docx::effective_properties`] is Word's own deep reference on the ladders
//! [`Document::effective_run_properties`]/[`Document::effective_paragraph_properties`] walk.
//!
//! # Status
//!
//! Pre-release (`v0.0.x`). PowerPoint and Word are implemented and tested; Excel is detected but not
//! yet editable. See the repository `PLAN.md` and `CHANGELOG.md` for the roadmap and version
//! milestones (`v0.1` = PowerPoint, `v0.2` = Word, `v0.3` = Excel).

mod address;
pub mod deck;
pub mod document;
mod error;
mod format;
mod index;
mod references;

pub use address::{ShapePath, Surface};
pub use deck::Deck;
pub use document::{
    BlockPath, CommentSummary, Document, NoteSummary, RunPath, SectionLocation, SectionSummary,
};
pub use error::{Error, ErrorCode, ErrorDetail};
pub use format::{detect_format, Format, FormatFamily};
pub use references::{DiagramParts, ExternalLink, InkReference};

// -----------------------------------------------------------------------------------------------
// The authoring vocabulary.
//
// Everything a caller must be able to *name* to call a `Deck` method or build one of its arguments,
// re-exported so no downstream ever depends on a crate below this one.
//
// Four families are deliberately absent, because they cannot be used without machinery this crate
// does not surface: the interner-bound fidelity models (`Color`, `Fill`, `Table`, `TextBody`,
// `TableStyle`, `Theme`, and their `*Content` siblings), which need an `Interner`; `RawDocument` and
// `RawElement`, which are the preservation tree; `PartName`, which `Deck` surfaces as `&str`; and
// `Package`, which is sealed on purpose — see `Deck::presentation_mut`.
//
// The list is closed under reachability, and `tests/public_paths.rs` proves it: a type named by a
// public field, an enum payload or a builder parameter of anything re-exported here is itself
// re-exported. Ten were added when the Python and WebAssembly bindings were written, because a
// binding must construct and destructure every one of them: `AdjustHandle`, `ConnectionSite` and
// `ColorKind` (payloads), `FontSlot`, `TableStyleBorder` and `ThemeFontReference` (builder
// parameters), `GuideFormulaError` (a `GuideError` payload), `AxisKind` (a `ChartAxisData` field),
// and `AdjustmentSpec` with `AdjustmentAxis`/`AdjustmentBound` (a `BoundedAdjustment` field).
// -----------------------------------------------------------------------------------------------

/// The typed cause behind every [`Error`], recoverable by downcasting
/// [`source`](std::error::Error::source).
pub use mjx_pptx::PptxError;
/// The PresentationML surface as `mjx-pptx` states it — reach for this only through
/// [`Deck::presentation_mut`](crate::Deck::presentation_mut).
pub use mjx_pptx::Presentation;

// --- PresentationML: addressing, geometry, and the read structures -------------------------------
pub use mjx_pptx::{
    default_placeholder_audio, default_placeholder_ole, default_placeholder_video,
    ActiveXControlSpec, ActiveXPersistence, CellFormat, CellMargins, Cells, ChartAxisData,
    ChartErrorBarData, ChartLabelScope, ChartLegendData, ChartPointFormatData, ChartSeriesData,
    ChartTrendlineData, ChartWorkbook, DiagramContent, DiagramPartKind, DiagramRelationshipIds,
    Geometry, GraphicFrameKind, Hyperlink, LayoutInfo, LinkedImage, MediaKind, MediaReference,
    OleObject, OleObjectData, OleObjectSpec, PlaceholderInfo, PresentationDefect, ShapeBounds,
    ShapeInfo, ShapeKind, SlideSize, TableStyleDefinition, TableStyleFormat, TargetMode,
    DEFAULT_PLACEHOLDER_IMAGE,
};

// --- DrawingML: the interner-free authoring specs and every simple type they take ----------------
pub use mjx_dml::{
    AdjustAngle, AdjustCoordinate, AdjustHandle, Angle, AutoNumberBullet, AutonumberScheme,
    Backdrop, Bevel, BevelPreset, BlendMode, BlurEffect, BoundedAdjustment, Bullet,
    BulletCharacter, BulletColor, BulletPicture, BulletSize, BulletTypeface, Camera, CellBorder,
    CharacterPropertiesSpec, ColorKind, ColorMap, ColorSchemeSlot, ColorSpec, CompoundLine,
    ConnectionSite, CustomGeometrySpec, DrawCommand, EffectListSpec, Emu, FillOverlayEffect,
    FillSpec, FontAlignment, FontCollection, FontCollectionIndex, FontScheme, FontSchemeSlot,
    FontSize, FontSlot, Fraction, GlowEffect, GradientStopSpec, GuideContext, GuideError,
    GuideFormulaError, GuideSpec, IndentLevel, InnerShadowEffect, LightRig, LightRigDirection,
    LightRigType, LineCap, LineDash, LineEnd, LineEndLength, LineEndType, LineEndWidth, LineJoin,
    LineSpec, LineWidth, OnOffStyle, OuterShadowEffect, ParagraphPropertiesSpec, Path2DSpec,
    PathFillMode, PatternType, PenAlignment, PictureFillMode, Point, Point3D, Position,
    PresetCamera, PresetLineDash, PresetMaterial, PresetShadow, PresetShadowEffect, Rectangle,
    RectangleAlignment, ReflectionEffect, ResolvedAdjustHandle, ResolvedAdjustment, ResolvedColor,
    ResolvedConnectionSite, ResolvedCustomGeometry, ResolvedDrawCommand, ResolvedGuides,
    ResolvedPath, ResolvedPoint, ResolvedRectangle, Scene3DSpec, SchemeColor, Shape3DSpec,
    ShapeGeometry, Size, SoftEdgeEffect, SphereCoordinates, SupplementalFont, TabAlignment,
    TabStop, TablePart, TableStyleBorder, TableStyleFlags, TableStylePart, TextAlignment,
    TextAnchoring, TextCapitalization, TextDirection, TextFont, TextHorizontalOverflow, TextPoint,
    TextSpacing, TextStrike, TextUnderline, ThemeFontReference, ThemeInfo, Transform2D,
    UnderlineFill, UnderlineLine, Vector3D,
};

// --- ChartML: the chart description and every enum its parts take --------------------------------
pub use mjx_chart::{
    AxisKind, AxisOrientation, AxisPosition, BarDirection, BarGrouping, BlankDisplay, ChartData,
    ChartDataError, ChartKind, DanglingPointReference, DataLabelPosition, DataLabelSettings,
    DataLabelSpec, ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorValueType, LegendPosition,
    OfPieType, RadarStyle, ScatterStyle, SeriesGrouping, TickLabelPosition, TickMark,
    TrendlineKind, TrendlineSpec,
};

// --- WordprocessingML: the interner-free authoring/reading types the Document surface names -------
//
// Everything `crate::document`'s own submodules take or hand back that is not already one of this
// crate's own facade types (`BlockPath`, `RunPath`, `SectionLocation`, `SectionSummary`,
// `CommentSummary`, `NoteSummary`) — mirroring the DrawingML block above's own reasoning: a caller
// destructuring an `EffectiveCharacterProperties` or matching on a `HyperlinkTarget` must be able to
// name every type that appears, without depending on `mjx-docx` directly.
pub use mjx_docx::{
    CellBorderEdge, EffectiveBorder, EffectiveCharacterProperties, EffectiveColor,
    EffectiveEastAsianLayout, EffectiveFonts, EffectiveLanguages, EffectiveManualRunWidth,
    EffectiveParagraphProperties, EffectiveShading, EffectiveTabStop, EffectiveUnderline, Field,
    FieldForm, GridDiscrepancy, HeaderFooterType, HyperlinkTarget, MergedCellType, PageMargins,
    PageOrientation, PageSize, RevisionInfo, RevisionKind,
};

// --- Generated schema simple types the signatures above name -------------------------------------
pub use mjx_ooxml_types::drawingml::{
    AdjustmentAxis, AdjustmentBound, AdjustmentSpec, PresetShapeType,
};
pub use mjx_ooxml_types::presentationml::{
    Orientation, PlaceholderSize, PlaceholderType, SlideLayoutKind, SlideSizeKind,
};
pub use mjx_ooxml_types::shared::{ConformanceClass, VerticalTextPosition};
pub use mjx_ooxml_types::wordprocessingml::{
    EighthPointMeasure, EmphasisMark, FontTypeHint, HalfPointMeasure, HighlightColor,
    Justification, SignedHalfPointMeasure, SignedTwipsMeasure, TabStopLeader, TabStopType,
    TextEffect, TextScale,
};
