//! What a surface holds, and what a caller hands it: the read structures, the two legacy
//! specifications, and hyperlinks.
//!
//! # The two owned specifications
//!
//! `mjx_ooxml::OleObjectSpec` and `mjx_ooxml::ActiveXControlSpec` borrow every byte and string they
//! carry, which is right for Rust — the caller already owns the bytes and nothing needs copying —
//! and impossible across a foreign function boundary, where the arguments' lifetime ends with the
//! call. So [`OleObjectSpec`] and [`ActiveXControlSpec`] here own their data and lend a borrowed
//! view for the one call that uses it.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::support::to_bytes;

use crate::enums::{
    ActiveXPersistence, MediaKind, Orientation, PlaceholderSize, PlaceholderType, ShapeKind,
    SlideLayoutKind,
};

value_class! {
    /// One slide layout: where it sits, which master it belongs to, its name and its kind.
    LayoutInfo(ooxml::LayoutInfo), derive(PartialEq, Eq);

    /// One shape on a surface: its index, its kind, and the placeholder it fills.
    ShapeInfo(ooxml::ShapeInfo), derive(PartialEq, Eq);

    /// What a placeholder shape declares itself to be.
    PlaceholderInfo(ooxml::PlaceholderInfo), derive(PartialEq, Eq);

    /// One audio, video or media reference on a surface.
    MediaReference(ooxml::MediaReference), derive(PartialEq, Eq);

    /// A picture whose image lies outside the package.
    LinkedImage(ooxml::LinkedImage), derive(PartialEq, Eq);

    /// One embedded or linked OLE object on a surface.
    OleObject(ooxml::OleObject), derive(PartialEq, Eq);

    /// A relationship whose target lies outside the package, and the part that holds it.
    ExternalLink(ooxml::ExternalLink), derive(PartialEq, Eq);

    /// One ink (InkML) reference on a surface.
    InkReference(ooxml::InkReference), derive(PartialEq, Eq);

    /// The five parts a SmartArt frame names.
    DiagramParts(ooxml::DiagramParts), derive(PartialEq, Eq);

    /// The four relationship ids a SmartArt frame carries.
    DiagramRelationshipIds(ooxml::DiagramRelationshipIds), derive(PartialEq, Eq);

    /// The four parts a SmartArt diagram is built from.
    DiagramContent(ooxml::DiagramContent), derive(PartialEq, Eq);

    /// Where a hyperlink goes: out to a URL, or in to another slide.
    Hyperlink(ooxml::Hyperlink), derive(PartialEq, Eq);
}

// ---------------------------------------------------------------------------------------------
// Read structures
// ---------------------------------------------------------------------------------------------

#[wasm_bindgen]
impl LayoutInfo {
    /// The layout's index in the deck's one flat layout space.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index as u32
    }

    /// The index of the master this layout belongs to.
    #[wasm_bindgen(getter, js_name = "masterIndex")]
    pub fn master_index(&self) -> u32 {
        self.0.master_index as u32
    }

    /// The layout's name, when it states one.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }

    /// Which of the thirty-six layout kinds `p:sldLayout@type` names.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<SlideLayoutKind, JsValue> {
        SlideLayoutKind::from_model(self.0.kind)
    }
}

#[wasm_bindgen]
impl ShapeInfo {
    /// The shape's top-level index on its surface.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index as u32
    }

    /// Which kind of shape this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<ShapeKind, JsValue> {
        ShapeKind::from_model(self.0.kind)
    }

    /// What placeholder the shape fills, when it fills one.
    #[wasm_bindgen(getter, js_name = "placeholder")]
    pub fn placeholder(&self) -> Option<PlaceholderInfo> {
        self.0.placeholder.clone().map(PlaceholderInfo)
    }
}

#[wasm_bindgen]
impl PlaceholderInfo {
    /// Which of the sixteen placeholder kinds.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<PlaceholderType, JsValue> {
        PlaceholderType::from_model(self.0.kind)
    }

    /// The placeholder's index, which is what pairs a slide's placeholder with its layout's.
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// Full, half or quarter.
    #[wasm_bindgen(getter, js_name = "size")]
    pub fn size(&self) -> Result<PlaceholderSize, JsValue> {
        PlaceholderSize::from_model(self.0.size)
    }

    /// Horizontal or vertical.
    #[wasm_bindgen(getter, js_name = "orientation")]
    pub fn orientation(&self) -> Result<Orientation, JsValue> {
        Orientation::from_model(self.0.orientation)
    }

    /// The shape's own name, when it states one.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.0.name.clone()
    }
}

#[wasm_bindgen]
impl MediaReference {
    /// The relationship id, which is how `replace_media_with_placeholder` names it.
    #[wasm_bindgen(getter, js_name = "relId")]
    pub fn rel_id(&self) -> String {
        self.0.rel_id.clone()
    }

    /// Audio, video, or the generic media relationship.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<MediaKind, JsValue> {
        MediaKind::from_model(self.0.kind)
    }

    /// Where the media is — a part name inside the package, or a URI outside it.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> String {
        self.0.target.clone()
    }

    /// Whether the media lies outside the package.
    #[wasm_bindgen(getter, js_name = "external")]
    pub fn external(&self) -> bool {
        self.0.external
    }
}

#[wasm_bindgen]
impl LinkedImage {
    /// The top-level index of the picture whose image is linked.
    #[wasm_bindgen(getter, js_name = "shapeIndex")]
    pub fn shape_index(&self) -> u32 {
        self.0.shape_index as u32
    }

    /// Where the image is, exactly as the relationship records it.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> String {
        self.0.target.clone()
    }
}

#[wasm_bindgen]
impl OleObject {
    /// The top-level index of the graphic frame that holds the object.
    #[wasm_bindgen(getter, js_name = "shapeIndex")]
    pub fn shape_index(&self) -> u32 {
        self.0.shape_index as u32
    }

    /// Where the object's data is.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> String {
        self.0.target.clone()
    }

    /// Whether the data lies outside the package.
    #[wasm_bindgen(getter, js_name = "external")]
    pub fn external(&self) -> bool {
        self.0.external
    }

    /// The programmatic identifier of the application that owns the object, when stated.
    #[wasm_bindgen(getter, js_name = "progId")]
    pub fn prog_id(&self) -> Option<String> {
        self.0.prog_id.clone()
    }
}

#[wasm_bindgen]
impl ExternalLink {
    /// The part whose relationships hold this one, or `None` for the package root.
    #[wasm_bindgen(getter, js_name = "source")]
    pub fn source(&self) -> Option<String> {
        self.0.source.clone()
    }

    /// The relationship id, unique within its source.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> String {
        self.0.id.clone()
    }

    /// The relationship type URI, which says what kind of external source it binds.
    #[wasm_bindgen(getter, js_name = "relType")]
    pub fn rel_type(&self) -> String {
        self.0.rel_type.clone()
    }

    /// The external target, exactly as recorded.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> String {
        self.0.target.clone()
    }
}

#[wasm_bindgen]
impl InkReference {
    /// The top-level index of the content part that names the ink, when a shape does.
    #[wasm_bindgen(getter, js_name = "shapeIndex")]
    pub fn shape_index(&self) -> Option<u32> {
        self.0.shape_index
    }

    /// The relationship id the content part carries.
    #[wasm_bindgen(getter, js_name = "relId")]
    pub fn rel_id(&self) -> String {
        self.0.rel_id.clone()
    }

    /// The ink part the relationship resolves to, when it resolves to one.
    #[wasm_bindgen(getter, js_name = "part")]
    pub fn part(&self) -> Option<String> {
        self.0.part.clone()
    }
}

#[wasm_bindgen]
impl DiagramParts {
    /// The data model part, when the frame names one that resolves.
    #[wasm_bindgen(getter, js_name = "data")]
    pub fn data(&self) -> Option<String> {
        self.0.data.clone()
    }

    /// The layout definition part.
    #[wasm_bindgen(getter, js_name = "layout")]
    pub fn layout(&self) -> Option<String> {
        self.0.layout.clone()
    }

    /// The style definition part.
    #[wasm_bindgen(getter, js_name = "style")]
    pub fn style(&self) -> Option<String> {
        self.0.style.clone()
    }

    /// The colour transform part.
    #[wasm_bindgen(getter, js_name = "colors")]
    pub fn colors(&self) -> Option<String> {
        self.0.colors.clone()
    }

    /// The cached drawing part, which renderers that do not lay out SmartArt fall back on.
    #[wasm_bindgen(getter, js_name = "drawing")]
    pub fn drawing(&self) -> Option<String> {
        self.0.drawing.clone()
    }
}

#[wasm_bindgen]
impl DiagramRelationshipIds {
    /// The data model relationship id (`dgm:relIds@dm`).
    #[wasm_bindgen(getter, js_name = "data")]
    pub fn data(&self) -> Option<String> {
        self.0.data.clone()
    }

    /// The layout definition relationship id (`@lo`).
    #[wasm_bindgen(getter, js_name = "layout")]
    pub fn layout(&self) -> Option<String> {
        self.0.layout.clone()
    }

    /// The style definition relationship id (`@qs`).
    #[wasm_bindgen(getter, js_name = "style")]
    pub fn style(&self) -> Option<String> {
        self.0.style.clone()
    }

    /// The colour transform relationship id (`@cs`).
    #[wasm_bindgen(getter, js_name = "colors")]
    pub fn colors(&self) -> Option<String> {
        self.0.colors.clone()
    }
}

#[wasm_bindgen]
impl DiagramContent {
    /// A diagram built from four part payloads you already have.
    #[wasm_bindgen(js_name = "fromParts")]
    pub fn from_parts(data: Vec<u8>, layout: Vec<u8>, style: Vec<u8>, colors: Vec<u8>) -> Self {
        Self(ooxml::DiagramContent::from_parts(
            data, layout, style, colors,
        ))
    }

    /// A minimal vertical list of labels — enough to write a SmartArt frame that opens.
    #[wasm_bindgen(js_name = "verticalList")]
    pub fn vertical_list(labels: Vec<String>) -> Self {
        let borrowed: Vec<&str> = labels.iter().map(String::as_str).collect();
        Self(ooxml::DiagramContent::vertical_list(&borrowed))
    }

    /// The data model part's bytes.
    #[wasm_bindgen(getter, js_name = "data")]
    pub fn data(&self) -> Vec<u8> {
        to_bytes(&self.0.data)
    }

    /// The layout definition part's bytes.
    #[wasm_bindgen(getter, js_name = "layout")]
    pub fn layout(&self) -> Vec<u8> {
        to_bytes(&self.0.layout)
    }

    /// The style definition part's bytes.
    #[wasm_bindgen(getter, js_name = "style")]
    pub fn style(&self) -> Vec<u8> {
        to_bytes(&self.0.style)
    }

    /// The colour transform part's bytes.
    #[wasm_bindgen(getter, js_name = "colors")]
    pub fn colors(&self) -> Vec<u8> {
        to_bytes(&self.0.colors)
    }
}

#[wasm_bindgen]
impl Hyperlink {
    /// A link out to a URL.
    pub fn url(url: &str) -> Self {
        Self(ooxml::Hyperlink::Url(url.to_owned()))
    }

    /// A link to another slide in the same deck, by index.
    pub fn slide(index: u32) -> Self {
        Self(ooxml::Hyperlink::Slide(index as usize))
    }

    /// Which kind this is: `"url"` or `"slide"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            ooxml::Hyperlink::Url(_) => "url".to_owned(),
            ooxml::Hyperlink::Slide(_) => "slide".to_owned(),
        }
    }

    /// The URL, when this links out.
    #[wasm_bindgen(getter, js_name = "target")]
    pub fn target(&self) -> Option<String> {
        match &self.0 {
            ooxml::Hyperlink::Url(url) => Some(url.clone()),
            ooxml::Hyperlink::Slide(_) => None,
        }
    }

    /// The slide index, when this links in.
    #[wasm_bindgen(getter, js_name = "slideIndex")]
    pub fn slide_index(&self) -> Option<u32> {
        match &self.0 {
            ooxml::Hyperlink::Slide(index) => Some(*index as u32),
            ooxml::Hyperlink::Url(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The two owned specifications
// ---------------------------------------------------------------------------------------------

/// What an OLE object's data is, owned.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnedOleData {
    /// A raw stream, embedded in the package.
    EmbeddedStream(Vec<u8>),
    /// A packaged file, embedded with its own extension and content type.
    EmbeddedPackage {
        /// The file's bytes.
        bytes: Vec<u8>,
        /// The extension the part carries.
        extension: String,
        /// The content type the part is declared as.
        content_type: String,
    },
    /// A file outside the package, named by URI.
    Linked(String),
}

/// Where an OLE object's data lives.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OleObjectData(OwnedOleData);

#[wasm_bindgen]
impl OleObjectData {
    /// A raw stream, embedded in the package.
    #[wasm_bindgen(js_name = "embeddedStream")]
    pub fn embedded_stream(bytes: Vec<u8>) -> Self {
        Self(OwnedOleData::EmbeddedStream(bytes))
    }

    /// A packaged file, embedded with its own extension and content type — a `.docx` inside a
    /// `.pptx`, say.
    #[wasm_bindgen(js_name = "embeddedPackage")]
    pub fn embedded_package(bytes: Vec<u8>, extension: &str, content_type: &str) -> Self {
        Self(OwnedOleData::EmbeddedPackage {
            bytes,
            extension: extension.to_owned(),
            content_type: content_type.to_owned(),
        })
    }

    /// A file outside the package, named by URI.
    pub fn linked(target: &str) -> Self {
        Self(OwnedOleData::Linked(target.to_owned()))
    }

    /// Which kind this is: `"embedded_stream"`, `"embedded_package"` or `"linked"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match &self.0 {
            OwnedOleData::EmbeddedStream(_) => "embedded_stream".to_owned(),
            OwnedOleData::EmbeddedPackage { .. } => "embedded_package".to_owned(),
            OwnedOleData::Linked(_) => "linked".to_owned(),
        }
    }
}

impl OleObjectData {
    /// The borrowed view the model takes, valid for as long as this value is.
    pub fn borrowed(&self) -> ooxml::OleObjectData<'_> {
        match &self.0 {
            OwnedOleData::EmbeddedStream(bytes) => ooxml::OleObjectData::EmbeddedStream(bytes),
            OwnedOleData::EmbeddedPackage {
                bytes,
                extension,
                content_type,
            } => ooxml::OleObjectData::EmbeddedPackage {
                bytes,
                extension,
                content_type,
            },
            OwnedOleData::Linked(target) => ooxml::OleObjectData::Linked(target),
        }
    }
}

/// An OLE object to add to a surface: what application owns it, what its data is, and the picture
/// PowerPoint shows in its place.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OleObjectSpec {
    prog_id: String,
    data: OleObjectData,
    snapshot_image: Vec<u8>,
    name: Option<String>,
    show_as_icon: bool,
}

#[wasm_bindgen]
impl OleObjectSpec {
    /// An OLE object.
    #[wasm_bindgen(constructor)]
    pub fn new(
        prog_id: &str,
        data: &OleObjectData,
        snapshot_image: Vec<u8>,
        name: Option<String>,
        show_as_icon: bool,
    ) -> Self {
        Self {
            prog_id: prog_id.to_owned(),
            data: data.clone(),
            snapshot_image,
            name,
            show_as_icon,
        }
    }

    /// An embedded-stream object, the common case.
    #[wasm_bindgen(js_name = "embeddedStream")]
    pub fn embedded_stream(prog_id: &str, data: Vec<u8>, snapshot_image: Vec<u8>) -> Self {
        Self {
            prog_id: prog_id.to_owned(),
            data: OleObjectData::embedded_stream(data),
            snapshot_image,
            name: None,
            show_as_icon: false,
        }
    }

    /// This object with the given display name.
    pub fn named(&self, name: &str) -> Self {
        let mut spec = self.clone();
        spec.name = Some(name.to_owned());
        spec
    }

    /// This object shown as an icon rather than as its snapshot.
    #[wasm_bindgen(js_name = "shownAsIcon")]
    pub fn shown_as_icon(&self, show_as_icon: bool) -> Self {
        let mut spec = self.clone();
        spec.show_as_icon = show_as_icon;
        spec
    }

    /// The programmatic identifier of the owning application.
    #[wasm_bindgen(getter, js_name = "progId")]
    pub fn prog_id(&self) -> String {
        self.prog_id.clone()
    }

    /// Where the object's data lives.
    #[wasm_bindgen(getter, js_name = "data")]
    pub fn data(&self) -> OleObjectData {
        self.data.clone()
    }

    /// The picture shown in the object's place.
    #[wasm_bindgen(getter, js_name = "snapshotImage")]
    pub fn snapshot_image(&self) -> Vec<u8> {
        to_bytes(&self.snapshot_image)
    }

    /// The display name, when one is set.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Whether the object is shown as an icon.
    #[wasm_bindgen(getter, js_name = "showAsIcon")]
    pub fn show_as_icon(&self) -> bool {
        self.show_as_icon
    }
}

impl OleObjectSpec {
    /// The borrowed specification the model takes, valid for as long as this value is.
    pub(crate) fn borrowed(&self) -> ooxml::OleObjectSpec<'_> {
        ooxml::OleObjectSpec {
            prog_id: &self.prog_id,
            data: self.data.borrowed(),
            snapshot_image: &self.snapshot_image,
            name: self.name.as_deref(),
            show_as_icon: self.show_as_icon,
        }
    }
}

/// An ActiveX control to add to a surface.
#[wasm_bindgen]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveXControlSpec {
    name: String,
    class_id: String,
    persistence: ActiveXPersistence,
    state: Option<Vec<u8>>,
    snapshot_image: Vec<u8>,
}

#[wasm_bindgen]
impl ActiveXControlSpec {
    /// A control: its name, its class identifier (a GUID in braces), its persisted state, and the
    /// picture PowerPoint shows in its place.
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: &str,
        class_id: &str,
        state: Vec<u8>,
        snapshot_image: Vec<u8>,
        persistence: ActiveXPersistence,
    ) -> Self {
        Self {
            name: name.to_owned(),
            class_id: class_id.to_owned(),
            persistence,
            state: Some(state),
            snapshot_image,
        }
    }

    /// The control's name.
    #[wasm_bindgen(getter, js_name = "name")]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The control's class identifier.
    #[wasm_bindgen(getter, js_name = "classId")]
    pub fn class_id(&self) -> String {
        self.class_id.clone()
    }

    /// How the control's state is persisted.
    #[wasm_bindgen(getter, js_name = "persistence")]
    pub fn persistence(&self) -> ActiveXPersistence {
        self.persistence
    }

    /// The persisted state, when there is any.
    #[wasm_bindgen(getter, js_name = "state")]
    pub fn state(&self) -> Option<Vec<u8>> {
        self.state.as_ref().map(|state| to_bytes(state))
    }

    /// The picture shown in the control's place.
    #[wasm_bindgen(getter, js_name = "snapshotImage")]
    pub fn snapshot_image(&self) -> Vec<u8> {
        to_bytes(&self.snapshot_image)
    }
}

impl ActiveXControlSpec {
    /// The borrowed specification the model takes, valid for as long as this value is.
    pub(crate) fn borrowed(&self) -> ooxml::ActiveXControlSpec<'_> {
        ooxml::ActiveXControlSpec {
            name: &self.name,
            class_id: &self.class_id,
            persistence: self.persistence.into(),
            state: self.state.as_deref(),
            snapshot_image: &self.snapshot_image,
        }
    }
}

/// A silent, zero-length WAV — an audio placeholder every consumer accepts.
#[wasm_bindgen(js_name = "defaultPlaceholderAudio")]
#[must_use]
pub fn default_placeholder_audio() -> Vec<u8> {
    ooxml::default_placeholder_audio()
}

/// A one-frame, zero-length MP4 — a video placeholder every consumer accepts.
#[wasm_bindgen(js_name = "defaultPlaceholderVideo")]
#[must_use]
pub fn default_placeholder_video() -> Vec<u8> {
    ooxml::default_placeholder_video()
}

/// An empty compound file — an OLE object placeholder every consumer accepts.
#[wasm_bindgen(js_name = "defaultPlaceholderOle")]
#[must_use]
pub fn default_placeholder_ole() -> Vec<u8> {
    ooxml::default_placeholder_ole()
}
