//! The typed surfaces for content a `.pptx` carries that is not DrawingML: ink (InkML), SmartArt
//! diagrams, OLE objects and ActiveX controls.
//!
//! Each of these lives in its own part (or four of them), referenced from the slide by relationship
//! id. What a caller needs is the *graph*: which shape points at which part, and what the part is.
//! These types are that graph, plus the specs the authoring methods take.

use mjx_opc::PartName;
use mjx_xml::text::escape_text;

// ---------------------------------------------------------------------------------------------
// Ink
// ---------------------------------------------------------------------------------------------

/// An ink (InkML) part and the content part that references it, as reported by
/// [`Presentation::ink_references`](crate::Presentation::ink_references).
///
/// Ink is referenced from the shape tree by a `p:contentPart` (PresentationML) or a
/// `p14:contentPart` (the Office 2010 extension, which producers wrap in `mc:AlternateContent`).
/// Only the first is a shape in the one shape index space, which is why `shape_index` is optional:
/// an `mc:AlternateContent` sits beside the shapes rather than among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InkReference {
    /// The shape index of the `p:contentPart` on the surface, or `None` when the reference lives
    /// inside an `mc:AlternateContent` and so is out of the shape index space.
    pub shape_index: Option<usize>,
    /// The relationship id the content part names (`@r:id`).
    pub rel_id: String,
    /// The InkML part that relationship resolves to, or `None` when it resolves outside the package.
    pub part: Option<PartName>,
}

// ---------------------------------------------------------------------------------------------
// SmartArt / diagrams
// ---------------------------------------------------------------------------------------------

/// The relationship ids a SmartArt frame's `dgm:relIds` names — ECMA-376 Part 1 §21.4.2.22
/// *relIds (Explicit Relationships to Diagram Parts)*.
///
/// The four are required by the schema, so a well-formed frame answers `Some` for all four; a
/// malformed one is reported as it is rather than rejected.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagramRelationshipIds {
    /// `@r:dm` — the relationship to the Diagram Data part.
    pub data: Option<String>,
    /// `@r:lo` — the relationship to the Diagram Layout Definition part.
    pub layout: Option<String>,
    /// `@r:qs` — the relationship to the Diagram Style part.
    pub style: Option<String>,
    /// `@r:cs` — the relationship to the Diagram Colors part.
    pub colors: Option<String>,
}

/// The parts of one SmartArt diagram, resolved to part names — the relationship graph behind a
/// `p:graphicFrame` whose [`GraphicFrameKind`](crate::GraphicFrameKind) is
/// [`Diagram`](crate::GraphicFrameKind::Diagram).
///
/// The first four come from the frame's `dgm:relIds`; `drawing` comes from the **data** part's own
/// relationships (`.../2007/relationships/diagramDrawing`), which is where PowerPoint caches the
/// laid-out shapes so a consumer that cannot run the layout engine still has something to draw.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagramParts {
    /// The Diagram Data part (`dgm:dataModel`) — the nodes and the connections between them.
    pub data: Option<PartName>,
    /// The Diagram Layout Definition part (`dgm:layoutDef`) — how those nodes are arranged.
    pub layout: Option<PartName>,
    /// The Diagram Style part (`dgm:styleDef`) — the quick style applied to them.
    pub style: Option<PartName>,
    /// The Diagram Colors part (`dgm:colorsDef`) — the colour transform applied to them.
    pub colors: Option<PartName>,
    /// The cached drawing (`dsp:drawing`), hung off the **data** part rather than the frame. An
    /// Office extension, absent from decks that never had one.
    pub drawing: Option<PartName>,
}

impl DiagramParts {
    /// Every part of the diagram that resolved, in the order they are declared — the graph as a flat
    /// list, for a caller that wants to sweep or copy all of them.
    #[must_use]
    pub fn all(&self) -> Vec<PartName> {
        [
            self.data.as_ref(),
            self.layout.as_ref(),
            self.style.as_ref(),
            self.colors.as_ref(),
            self.drawing.as_ref(),
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }
}

/// Which of a diagram's parts a read or an edit is aimed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagramPartKind {
    /// The Diagram Data part (`dgm:dataModel`).
    Data,
    /// The Diagram Layout Definition part (`dgm:layoutDef`).
    Layout,
    /// The Diagram Style part (`dgm:styleDef`).
    Style,
    /// The Diagram Colors part (`dgm:colorsDef`).
    Colors,
    /// The cached drawing (`dsp:drawing`).
    Drawing,
}

/// The four documents a SmartArt diagram is made of, as bytes — what
/// [`add_diagram`](crate::Presentation::add_diagram) writes into the package.
///
/// Supply your own with [`from_parts`](Self::from_parts) — copied from a deck you already have, or
/// generated — or build a working one from a list of labels with
/// [`vertical_list`](Self::vertical_list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramContent {
    /// The Diagram Data part (`dgm:dataModel`).
    pub data: Vec<u8>,
    /// The Diagram Layout Definition part (`dgm:layoutDef`).
    pub layout: Vec<u8>,
    /// The Diagram Style part (`dgm:styleDef`).
    pub style: Vec<u8>,
    /// The Diagram Colors part (`dgm:colorsDef`).
    pub colors: Vec<u8>,
}

/// The `uniqueId` the built-in layout definition declares, and the `loTypeId` the data model points
/// at — they must agree for a consumer to bind the two.
const LAYOUT_UNIQUE_ID: &str = "urn:mjx-ooxml/layout/verticalList";
/// The `uniqueId` of the built-in quick style.
const STYLE_UNIQUE_ID: &str = "urn:mjx-ooxml/quickStyle/simple";
/// The `uniqueId` of the built-in colour transform.
const COLORS_UNIQUE_ID: &str = "urn:mjx-ooxml/colors/accent1";

impl DiagramContent {
    /// Takes the four documents as they are. Nothing is parsed or rewritten: whatever you hand in is
    /// what the package carries.
    #[must_use]
    pub fn from_parts(data: Vec<u8>, layout: Vec<u8>, style: Vec<u8>, colors: Vec<u8>) -> Self {
        Self {
            data,
            layout,
            style,
            colors,
        }
    }

    /// A working diagram drawing `labels` as a vertical list of rounded rectangles.
    ///
    /// All four documents are generated: a data model holding one node per label (with the
    /// `parTrans`/`sibTrans` points and `parOf` connections PowerPoint expects), a layout definition
    /// that stacks them top to bottom, a quick style, and a colour transform that fills each node
    /// from the theme's first accent colour. Every one validates against `dml-diagram.xsd`.
    ///
    /// An empty `labels` yields a diagram with no nodes — valid, and drawn as an empty frame.
    #[must_use]
    pub fn vertical_list(labels: &[&str]) -> Self {
        Self {
            data: vertical_list_data(labels),
            layout: VERTICAL_LIST_LAYOUT.as_bytes().to_vec(),
            style: SIMPLE_STYLE.as_bytes().to_vec(),
            colors: ACCENT_COLORS.as_bytes().to_vec(),
        }
    }
}

/// The XML declaration every part this module writes opens with, matching what Office writes.
const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// Builds the `dgm:dataModel` for [`DiagramContent::vertical_list`].
///
/// Model ids are assigned from a fixed arithmetic series (`1` for the document node, then a block of
/// four per label), so the document is deterministic — the same labels always produce the same bytes,
/// which is what lets a test assert on them.
fn vertical_list_data(labels: &[&str]) -> Vec<u8> {
    let mut out = String::with_capacity(1024 + labels.len() * 256);
    out.push_str(DECLARATION);
    out.push_str(concat!(
        "<dgm:dataModel xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"",
        " xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
    ));
    out.push_str("<dgm:ptLst><dgm:pt modelId=\"1\" type=\"doc\"><dgm:prSet");
    out.push_str(&format!(
        " loTypeId=\"{LAYOUT_UNIQUE_ID}\" loCatId=\"list\""
    ));
    out.push_str(&format!(
        " qsTypeId=\"{STYLE_UNIQUE_ID}\" qsCatId=\"simple\""
    ));
    out.push_str(&format!(
        " csTypeId=\"{COLORS_UNIQUE_ID}\" csCatId=\"accent1\"/>"
    ));
    out.push_str(EMPTY_TEXT_BODY);
    out.push_str("</dgm:pt>");

    for (index, label) in labels.iter().enumerate() {
        let base = 10 + index as u32 * 10;
        let (node, par_trans, connection, sib_trans) = (base, base + 1, base + 2, base + 3);
        let text = escape_text(label);
        out.push_str(&format!("<dgm:pt modelId=\"{node}\">"));
        out.push_str(&format!("<dgm:prSet phldrT=\"{text}\"/>"));
        out.push_str("<dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>");
        out.push_str(&text);
        out.push_str("</a:t></a:r></a:p></dgm:t></dgm:pt>");
        out.push_str(&format!(
            "<dgm:pt modelId=\"{par_trans}\" type=\"parTrans\" cxnId=\"{connection}\">"
        ));
        out.push_str(EMPTY_TEXT_BODY);
        out.push_str("</dgm:pt>");
        out.push_str(&format!(
            "<dgm:pt modelId=\"{sib_trans}\" type=\"sibTrans\" cxnId=\"{connection}\">"
        ));
        out.push_str(EMPTY_TEXT_BODY);
        out.push_str("</dgm:pt>");
    }
    out.push_str("</dgm:ptLst><dgm:cxnLst>");
    for index in 0..labels.len() {
        let base = 10 + index as u32 * 10;
        let (node, par_trans, connection, sib_trans) = (base, base + 1, base + 2, base + 3);
        out.push_str(&format!(
            "<dgm:cxn modelId=\"{connection}\" srcId=\"1\" destId=\"{node}\" srcOrd=\"{index}\" \
             destOrd=\"0\" parTransId=\"{par_trans}\" sibTransId=\"{sib_trans}\"/>"
        ));
    }
    out.push_str("</dgm:cxnLst><dgm:whole/></dgm:dataModel>");
    out.into_bytes()
}

/// An empty `dgm:t` — a DrawingML text body with one empty paragraph, which `a:CT_TextBody`
/// requires. Every point that carries no text still needs one.
const EMPTY_TEXT_BODY: &str =
    "<dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></dgm:t>";

/// The layout definition [`DiagramContent::vertical_list`] writes: the linear algorithm running top
/// to bottom over the document node's children, each drawn as a rounded rectangle sized to its text.
const VERTICAL_LIST_LAYOUT: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"",
    " xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
    " uniqueId=\"urn:mjx-ooxml/layout/verticalList\">",
    "<dgm:title val=\"Vertical list\"/>",
    "<dgm:desc val=\"A vertical list of nodes, one per top-level item.\"/>",
    "<dgm:catLst><dgm:cat type=\"list\" pri=\"1000\"/></dgm:catLst>",
    "<dgm:sampData><dgm:dataModel><dgm:ptLst/></dgm:dataModel></dgm:sampData>",
    "<dgm:layoutNode name=\"diagram\">",
    "<dgm:varLst><dgm:chMax val=\"0\"/><dgm:dir val=\"norm\"/>",
    "<dgm:resizeHandles val=\"exact\"/></dgm:varLst>",
    "<dgm:alg type=\"lin\"><dgm:param type=\"linDir\" val=\"fromT\"/></dgm:alg>",
    "<dgm:shape r:blip=\"\"><dgm:adjLst/></dgm:shape>",
    "<dgm:presOf/>",
    "<dgm:constrLst>",
    "<dgm:constr type=\"w\" for=\"ch\" ptType=\"node\" refType=\"w\"/>",
    "<dgm:constr type=\"h\" for=\"ch\" ptType=\"node\" op=\"equ\"/>",
    "<dgm:constr type=\"sibSp\" refType=\"h\" refFor=\"ch\" refPtType=\"node\" fact=\"0.1\"/>",
    "</dgm:constrLst>",
    "<dgm:forEach name=\"nodes\" axis=\"ch\" ptType=\"node\">",
    "<dgm:layoutNode name=\"node\">",
    "<dgm:alg type=\"tx\"/>",
    "<dgm:shape type=\"roundRect\" r:blip=\"\"><dgm:adjLst/></dgm:shape>",
    "<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>",
    "<dgm:constrLst>",
    "<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>",
    "<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>",
    "</dgm:constrLst>",
    "<dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>",
    "</dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>",
);

/// The quick style [`DiagramContent::vertical_list`] writes: one `node0` label taking the theme's
/// second line reference and first fill reference.
const SIMPLE_STYLE: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<dgm:styleDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"",
    " xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
    " uniqueId=\"urn:mjx-ooxml/quickStyle/simple\">",
    "<dgm:title val=\"Simple fill\"/>",
    "<dgm:desc val=\"A plain filled node with an outline.\"/>",
    "<dgm:catLst><dgm:cat type=\"simple\" pri=\"10100\"/></dgm:catLst>",
    "<dgm:scene3d><a:camera prst=\"orthographicFront\"/>",
    "<a:lightRig rig=\"threePt\" dir=\"t\"/></dgm:scene3d>",
    "<dgm:styleLbl name=\"node0\">",
    "<dgm:scene3d><a:camera prst=\"orthographicFront\"/>",
    "<a:lightRig rig=\"threePt\" dir=\"t\"/></dgm:scene3d>",
    "<dgm:sp3d/><dgm:txPr/>",
    "<dgm:style>",
    "<a:lnRef idx=\"2\"><a:scrgbClr r=\"0\" g=\"0\" b=\"0\"/></a:lnRef>",
    "<a:fillRef idx=\"1\"><a:scrgbClr r=\"0\" g=\"0\" b=\"0\"/></a:fillRef>",
    "<a:effectRef idx=\"0\"><a:scrgbClr r=\"0\" g=\"0\" b=\"0\"/></a:effectRef>",
    "<a:fontRef idx=\"minor\"><a:schemeClr val=\"lt1\"/></a:fontRef>",
    "</dgm:style></dgm:styleLbl></dgm:styleDef>",
);

/// The colour transform [`DiagramContent::vertical_list`] writes: every node filled from the theme's
/// first accent colour and outlined in its first light colour.
const ACCENT_COLORS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<dgm:colorsDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"",
    " xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"",
    " uniqueId=\"urn:mjx-ooxml/colors/accent1\">",
    "<dgm:title val=\"Accent 1\"/>",
    "<dgm:desc val=\"Fills every node from the theme's first accent colour.\"/>",
    "<dgm:catLst><dgm:cat type=\"accent1\" pri=\"11000\"/></dgm:catLst>",
    "<dgm:styleLbl name=\"node0\">",
    "<dgm:fillClrLst meth=\"repeat\"><a:schemeClr val=\"accent1\"/></dgm:fillClrLst>",
    "<dgm:linClrLst meth=\"repeat\"><a:schemeClr val=\"lt1\"/></dgm:linClrLst>",
    "<dgm:effectClrLst/><dgm:txLinClrLst/><dgm:txFillClrLst/><dgm:txEffectClrLst/>",
    "</dgm:styleLbl></dgm:colorsDef>",
);

// ---------------------------------------------------------------------------------------------
// OLE objects
// ---------------------------------------------------------------------------------------------

/// How an authored OLE object's data is carried — [`OleObjectSpec::data`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OleObjectData<'a> {
    /// A raw OLE compound-file stream, stored as `ppt/embeddings/oleObjectN.bin`. This is what
    /// PowerPoint writes for an object whose application has no OOXML form.
    EmbeddedStream(&'a [u8]),
    /// A whole embedded Office package — an `.xlsx` worksheet, a `.docx` document — stored beside the
    /// stream embeddings with the extension and content type you name.
    EmbeddedPackage {
        /// The package's bytes.
        bytes: &'a [u8],
        /// Its file extension, without the leading dot (`xlsx`, `docx`).
        extension: &'a str,
        /// Its content type, e.g.
        /// `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`.
        content_type: &'a str,
    },
    /// A link to data outside the package. The relationship is written `TargetMode="External"` and
    /// nothing is stored.
    Linked(&'a str),
}

/// What [`add_ole_object`](crate::Presentation::add_ole_object) writes.
///
/// An OLE object is never executed by a consumer: it is drawn from its **snapshot image**, and its
/// data is opened only when the user activates it. So the snapshot is required — an object without
/// one is a frame that renders nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OleObjectSpec<'a> {
    /// The program id of the application that owns the object (`p:oleObj@progId`, e.g.
    /// `Excel.Sheet.12`).
    pub prog_id: &'a str,
    /// Where the object's data comes from.
    pub data: OleObjectData<'a>,
    /// The image a consumer draws in place of the object. Its format is sniffed from the bytes.
    pub snapshot_image: &'a [u8],
    /// The object's display name (`p:oleObj@name`), or `None` for none.
    pub name: Option<&'a str>,
    /// Whether the object is drawn as an icon rather than its content (`p:oleObj@showAsIcon`).
    pub show_as_icon: bool,
}

impl<'a> OleObjectSpec<'a> {
    /// An embedded object owned by `prog_id`, carrying `data` as a raw OLE stream and drawn as
    /// `snapshot_image`.
    #[must_use]
    pub fn embedded_stream(prog_id: &'a str, data: &'a [u8], snapshot_image: &'a [u8]) -> Self {
        Self {
            prog_id,
            data: OleObjectData::EmbeddedStream(data),
            snapshot_image,
            name: None,
            show_as_icon: false,
        }
    }

    /// Gives the object a display name (`p:oleObj@name`).
    #[must_use]
    pub fn named(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }
}

// ---------------------------------------------------------------------------------------------
// ActiveX controls
// ---------------------------------------------------------------------------------------------

/// How an ActiveX control's state is persisted — `ax:ocx@ax:persistence`.
///
/// The four wire values come from the ActiveX part's own markup (`ax:ocx`, content type
/// `application/vnd.ms-office.activeX+xml`), which is a Microsoft extension outside ECMA-376; each
/// variant records the exact token it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActiveXPersistence {
    /// The state is a compound-file storage in the control's `.bin`. Wire value `persistStorage`.
    Storage,
    /// The state is a stream in the control's `.bin`. Wire value `persistStream`.
    Stream,
    /// The state is a stream preceded by its length. Wire value `persistStreamInit`.
    StreamWithLength,
    /// The state is a property bag written inline in the control part. Wire value
    /// `persistPropertyBag`.
    PropertyBag,
}

impl ActiveXPersistence {
    /// The exact wire value for this persistence.
    #[must_use]
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::Storage => "persistStorage",
            Self::Stream => "persistStream",
            Self::StreamWithLength => "persistStreamInit",
            Self::PropertyBag => "persistPropertyBag",
        }
    }

    /// The persistence a wire value names, or `None` for one the ActiveX part does not define.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "persistStorage" => Some(Self::Storage),
            "persistStream" => Some(Self::Stream),
            "persistStreamInit" => Some(Self::StreamWithLength),
            "persistPropertyBag" => Some(Self::PropertyBag),
            _ => None,
        }
    }
}

/// What [`add_activex_control`](crate::Presentation::add_activex_control) writes.
///
/// Like an OLE object, a control is never executed by a consumer: it is drawn from its snapshot
/// image, which is therefore required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveXControlSpec<'a> {
    /// The control's name on the slide (`p:control@name`, e.g. `CommandButton1`).
    pub name: &'a str,
    /// The class id of the control's COM component (`ax:ocx@ax:classid`), braces included — e.g.
    /// `{D7053240-CE69-11CD-A777-00DD01143C57}` for the Forms 2.0 command button.
    pub class_id: &'a str,
    /// How the control's state is persisted.
    pub persistence: ActiveXPersistence,
    /// The control's persisted state, stored as `ppt/activeX/activeXN.bin`, or `None` for a control
    /// that persists none.
    pub state: Option<&'a [u8]>,
    /// The image a consumer draws in place of the control. Its format is sniffed from the bytes.
    pub snapshot_image: &'a [u8],
}

impl<'a> ActiveXControlSpec<'a> {
    /// A control of `class_id` named `name`, persisting `state` as a storage and drawn as
    /// `snapshot_image`.
    #[must_use]
    pub fn new(
        name: &'a str,
        class_id: &'a str,
        state: &'a [u8],
        snapshot_image: &'a [u8],
    ) -> Self {
        Self {
            name,
            class_id,
            persistence: ActiveXPersistence::Storage,
            state: Some(state),
            snapshot_image,
        }
    }
}

/// The bytes of an `ax:ocx` control part naming `class_id`, `persistence` and the relationship
/// `binary_rel_id` under which its state is stored.
///
/// The part carries the relationship reference only when there is a state part to reference: a
/// control that persists nothing writes no `r:id`, which is what makes its `.rels` unnecessary.
pub(crate) fn activex_part_bytes(
    class_id: &str,
    persistence: ActiveXPersistence,
    binary_rel_id: Option<&str>,
) -> Vec<u8> {
    let mut out = String::with_capacity(320);
    out.push_str(DECLARATION);
    out.push_str("<ax:ocx xmlns:ax=\"http://schemas.microsoft.com/office/2006/activeX\"");
    out.push_str(
        " xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"",
    );
    out.push_str(&format!(" ax:classid=\"{}\"", escape_text(class_id)));
    out.push_str(&format!(" ax:persistence=\"{}\"", persistence.to_wire()));
    if let Some(rel_id) = binary_rel_id {
        out.push_str(&format!(" r:id=\"{}\"", escape_text(rel_id)));
    }
    out.push_str("/>");
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vertical_list_data_model_names_every_label_once() {
        let bytes = vertical_list_data(&["Alpha", "Beta & Co"]);
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(text.contains("<a:t>Alpha</a:t>"));
        assert!(
            text.contains("<a:t>Beta &amp; Co</a:t>"),
            "label text is escaped: {text}"
        );
        assert_eq!(
            text.matches("<dgm:cxn ").count(),
            2,
            "one parOf connection per label"
        );
        assert!(
            text.contains("srcOrd=\"1\""),
            "the second label is ordered after the first"
        );
        assert!(
            text.contains(LAYOUT_UNIQUE_ID),
            "the data model points at the layout it was generated for"
        );
    }

    #[test]
    fn an_empty_vertical_list_is_still_a_document_node() {
        let text = String::from_utf8(vertical_list_data(&[])).expect("utf-8");
        assert!(text.contains("type=\"doc\""));
        assert!(!text.contains("<dgm:cxn "));
    }

    #[test]
    fn the_generated_documents_agree_on_their_unique_ids() {
        // The data model binds itself to the layout, style and colour documents by `uniqueId`. If
        // these fall out of step the diagram loses its formatting silently.
        let data = String::from_utf8(vertical_list_data(&["x"])).expect("utf-8");
        for id in [LAYOUT_UNIQUE_ID, STYLE_UNIQUE_ID, COLORS_UNIQUE_ID] {
            assert!(data.contains(id), "the data model names {id}");
        }
        assert!(VERTICAL_LIST_LAYOUT.contains(LAYOUT_UNIQUE_ID));
        assert!(SIMPLE_STYLE.contains(STYLE_UNIQUE_ID));
        assert!(ACCENT_COLORS.contains(COLORS_UNIQUE_ID));
    }

    #[test]
    fn an_activex_part_omits_the_relationship_when_there_is_no_state() {
        let with_state = String::from_utf8(activex_part_bytes(
            "{D7053240-CE69-11CD-A777-00DD01143C57}",
            ActiveXPersistence::Storage,
            Some("rId1"),
        ))
        .expect("utf-8");
        assert!(with_state.contains("r:id=\"rId1\""));
        assert!(with_state.contains("ax:persistence=\"persistStorage\""));

        let without = String::from_utf8(activex_part_bytes(
            "{D7053240-CE69-11CD-A777-00DD01143C57}",
            ActiveXPersistence::PropertyBag,
            None,
        ))
        .expect("utf-8");
        assert!(!without.contains("r:id="), "no state, no reference");
        assert!(without.contains("ax:persistence=\"persistPropertyBag\""));
    }

    #[test]
    fn every_persistence_round_trips_through_its_wire_value() {
        for persistence in [
            ActiveXPersistence::Storage,
            ActiveXPersistence::Stream,
            ActiveXPersistence::StreamWithLength,
            ActiveXPersistence::PropertyBag,
        ] {
            assert_eq!(
                ActiveXPersistence::from_wire(persistence.to_wire()),
                Some(persistence)
            );
        }
        assert_eq!(ActiveXPersistence::from_wire("persistNothing"), None);
    }
}
