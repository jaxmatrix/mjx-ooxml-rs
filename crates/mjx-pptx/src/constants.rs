//! PresentationML relationship-type and content-type URI constants.
//!
//! These are the *transitional* (Office-emitted) URIs, which the fixtures use. Relationship lookup
//! (`Relationships::by_type`) matches the exact string.

/// The relationship type from the package root to the main presentation part.
pub const REL_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// The relationship type from the presentation part to a slide part.
pub const REL_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

/// The relationship type from a slide part to its slide layout.
pub const REL_SLIDE_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";

/// The relationship type from a slide layout to its slide master.
pub const REL_SLIDE_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";

/// The relationship type from a slide master (or the presentation) to its theme.
pub const REL_THEME: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

/// The relationship type from a part to an external hyperlink target (a URL). Always
/// `TargetMode="External"`.
pub const REL_HYPERLINK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";

/// The relationship type from a slide to its notes slide.
pub const REL_NOTES_SLIDE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";

/// The relationship type from the presentation (or a notes slide) to the notes master.
pub const REL_NOTES_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";

/// The relationship type from a part (e.g. a slide) to an embedded image part.
pub const REL_IMAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// The relationship type from the presentation part to its `tableStyles.xml`.
pub const REL_TABLE_STYLES: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles";

/// The relationship type from a slide (or other part) to a chart part (`/ppt/charts/chartN.xml`) —
/// the target a chart frame's `c:chart@r:id` names.
pub const REL_CHART: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";

/// The relationship type from a slide to an embedded OLE object part (`/ppt/embeddings/oleObjectN.bin`)
/// — the target a `p:oleObj@r:id` names.
pub const REL_OLE_OBJECT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject";

/// The relationship type from a slide to an embedded Office **package** (`.xlsx`/`.docx`/…), the other
/// way an OLE object's data is carried (an embedded document rather than a raw OLE `.bin` stream).
pub const REL_PACKAGE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";

/// The relationship type from a slide to a **video** part — the target an `a:videoFile@r:link` names.
pub const REL_VIDEO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";

/// The relationship type from a slide to an **audio** part — the target an `a:audioFile@r:link` or a
/// `p:snd@r:embed` (transition/timing sound) names.
pub const REL_AUDIO: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";

/// The relationship type from a slide to a **media** part — the generic reference an `a14:media`
/// extension (`mc:AlternateContent` fallback, MS Office 2007) names for the same audio or video.
pub const REL_MEDIA: &str = "http://schemas.microsoft.com/office/2007/relationships/media";

/// The relationship type from a slide to an ActiveX control part (`/ppt/activeX/activeXN.xml`) — the
/// target a `p:control@r:id` names.
pub const REL_CONTROL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/control";

/// The relationship type from an ActiveX control part to its binary blob (`/ppt/activeX/activeXN.bin`),
/// which holds the control's persisted state.
pub const REL_ACTIVEX_CONTROL_BINARY: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/activeXControlBinary";

/// The relationship type from a slide to an ink (InkML) part (`/ppt/ink/inkN.xml`) — the target a
/// `p14:contentPart@r:id` names. Ink reuses the shared "customXml" relationship type (MS-ODRAWXML
/// §2.1.4 Ink Content Part).
pub const REL_INK: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";

/// The content type of the main presentation part.
pub const CONTENT_TYPE_PRESENTATION: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

/// The content type of a slide part.
pub const CONTENT_TYPE_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";

/// The content type of a slide master part.
pub const CONTENT_TYPE_SLIDE_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";

/// The content type of a slide layout part.
pub const CONTENT_TYPE_SLIDE_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";

/// The content type of a theme part.
pub const CONTENT_TYPE_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

/// The content type of a notes slide part.
pub const CONTENT_TYPE_NOTES_SLIDE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";

/// The content type of a notes master part.
pub const CONTENT_TYPE_NOTES_MASTER: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml";

/// The content type of the `tableStyles.xml` part. Shares the `xml` extension with every other part,
/// so it is registered as a per-part Override, not a Default.
pub const CONTENT_TYPE_TABLE_STYLES: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml";

/// The content type of a chart part (`/ppt/charts/chartN.xml`). Shares the `xml` extension with every
/// other part, so it is registered as a per-part Override, not a Default.
pub const CONTENT_TYPE_CHART: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";

/// The content type of a raw OLE object part (`/ppt/embeddings/oleObjectN.bin`). Its `.bin` extension
/// is shared with other binary parts (e.g. `printerSettings`), so it is registered as a per-part
/// Override, not a Default.
pub const CONTENT_TYPE_OLE_OBJECT: &str = "application/vnd.openxmlformats-officedocument.oleObject";

/// The content type of an ActiveX control part (`/ppt/activeX/activeXN.xml`, `ax:ocx` markup). Shares
/// the `xml` extension with every other part, so it is registered as a per-part Override, not a Default.
pub const CONTENT_TYPE_ACTIVEX: &str = "application/vnd.ms-office.activeX+xml";

/// The content type of a WAV audio part (`.wav`), used for the built-in audio placeholder.
pub const CONTENT_TYPE_WAV: &str = "audio/x-wav";

/// The content type of an MP4 video part (`.mp4`), used for the built-in video placeholder.
pub const CONTENT_TYPE_MP4: &str = "video/mp4";

/// The content type of an ActiveX control's binary blob (`/ppt/activeX/activeXN.bin`). Its `.bin`
/// extension is shared with other binary parts, so it is registered as a per-part Override, not a
/// Default.
pub const CONTENT_TYPE_ACTIVEX_BINARY: &str = "application/vnd.ms-office.activeX";

/// The content type of an ink (InkML) part (`/ppt/ink/inkN.xml`). Shares the `xml` extension with every
/// other part, so it is registered as a per-part Override, not a Default.
pub const CONTENT_TYPE_INKML: &str = "application/inkml+xml";

/// The relationship type from a slide to a SmartArt **Diagram Data** part
/// (`/ppt/diagrams/dataN.xml`) — the target a `dgm:relIds@r:dm` names (ECMA-376 Part 1 §21.4.2.22).
pub const REL_DIAGRAM_DATA: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData";

/// The relationship type from a slide to a SmartArt **Diagram Layout Definition** part
/// (`/ppt/diagrams/layoutN.xml`) — the target a `dgm:relIds@r:lo` names.
pub const REL_DIAGRAM_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramLayout";

/// The relationship type from a slide to a SmartArt **Diagram Style** part
/// (`/ppt/diagrams/quickStyleN.xml`) — the target a `dgm:relIds@r:qs` names. Note the mismatch the
/// spec itself carries: the relationship says `diagramQuickStyle`, the content type says
/// `diagramStyle`.
pub const REL_DIAGRAM_QUICK_STYLE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramQuickStyle";

/// The relationship type from a slide to a SmartArt **Diagram Colors** part
/// (`/ppt/diagrams/colorsN.xml`) — the target a `dgm:relIds@r:cs` names.
pub const REL_DIAGRAM_COLORS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramColors";

/// The relationship type from a diagram's **data** part to the cached drawing
/// (`/ppt/diagrams/drawingN.xml`) that holds the laid-out shapes. An Office 2007 extension outside
/// ECMA-376, hung off the data part rather than the frame.
pub const REL_DIAGRAM_DRAWING: &str =
    "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing";

/// The content type of a SmartArt Diagram Data part (`dgm:dataModel` markup).
pub const CONTENT_TYPE_DIAGRAM_DATA: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml";

/// The content type of a SmartArt Diagram Layout Definition part (`dgm:layoutDef` markup).
pub const CONTENT_TYPE_DIAGRAM_LAYOUT: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml";

/// The content type of a SmartArt Diagram Style part (`dgm:styleDef` markup).
pub const CONTENT_TYPE_DIAGRAM_STYLE: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml";

/// The content type of a SmartArt Diagram Colors part (`dgm:colorsDef` markup).
pub const CONTENT_TYPE_DIAGRAM_COLORS: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml";

/// The content type of a SmartArt cached drawing part (`dsp:drawing` markup). A Microsoft extension.
pub const CONTENT_TYPE_DIAGRAM_DRAWING: &str =
    "application/vnd.ms-office.drawingml.diagramDrawing+xml";

/// The namespace of the Office 2010 PowerPoint extensions, where `p14:contentPart` lives — the
/// element producers wrap in `mc:AlternateContent` to reference an ink part.
pub const POWERPOINT_2010_NAMESPACE: &str =
    "http://schemas.microsoft.com/office/powerpoint/2010/main";

/// The namespace of the InkML markup an ink part is rooted in (`inkml:ink`), used to check that
/// bytes handed to [`add_ink`](crate::Presentation::add_ink) really are ink.
pub const INKML_NAMESPACE: &str = "http://www.w3.org/2003/InkML";

/// The namespace of an ActiveX control part's markup (`ax:ocx`). A Microsoft extension outside
/// ECMA-376.
pub const ACTIVEX_NAMESPACE: &str = "http://schemas.microsoft.com/office/2006/activeX";
