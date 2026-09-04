//! `w:sectPr` (`CT_SectPr`) — a section's page setup, columns, breaks and line numbering — and
//! **section addressing**: which paragraphs a section governs.
//!
//! # A section's properties live at the END of the range they govern
//!
//! This is the trap MJXOFF-109's own ticket names, and it is real: a `w:sectPr` inside a paragraph's
//! `w:pPr` ([`super::paragraph_properties::ParagraphProperties::section_properties`]) **ends** a
//! section *at* that paragraph — the paragraph carrying it is that section's own *last* paragraph,
//! not the first paragraph of whatever comes next. The body-level `w:sectPr`
//! ([`super::body::Body::section_properties`]) is always the document's *last* section, governing
//! every paragraph after the last paragraph-level one (or every paragraph, if the body carries no
//! paragraph-level `w:sectPr` at all). Reading either one as "properties of the following content"
//! gets every multi-section document wrong. [`sections_in`] is where this crate resolves the
//! ambiguity once, so no caller has to reason about it directly; [`SectionSpan`] is its answer.
//!
//! A single-section document (`tests/fixtures/sample.docx`, and every document [`crate::Document::blank`]
//! produces) has exactly one paragraph-level `w:sectPr`-free body and one body-level `w:sectPr` — the
//! body-level section spans the whole body. **This shape alone cannot distinguish a correct
//! implementation from one that reads only the body-level `w:sectPr` and ignores paragraph-level ones
//! entirely** — see `crates/mjx-docx/tests/sections.rs` for the three-section fixture built
//! specifically to catch that.
//!
//! # `EG_SectPrContents`'s 19 members, all reachable off [`SectionProperties`]
//!
//! `footnotePr`, `endnotePr` (MJXOFF-124's own semantics; kept structurally opaque here),
//! `type` ([`SectionType`]), `pgSz`/`pgMar` (bridged to the shared [`crate::PageSize`]/
//! [`crate::PageMargins`] value types — see [`SectionProperties::page_size`]/`page_margins`),
//! `paperSrc` ([`PaperSource`]), `pgBorders` ([`PageBorderSet`]), `lnNumType` ([`LineNumbering`]),
//! `pgNumType` ([`PageNumbering`]), `cols` ([`Columns`]), `formProt`/`noEndnote`/`titlePg`/`bidi`/
//! `rtlGutter` (all `CT_OnOff`, reusing [`super::run_properties::Toggle`] — the same reuse
//! `paragraph_properties.rs` already makes for its own eighteen toggles), `vAlign`
//! ([`PageVerticalAlignment`]), `textDirection` (reuses
//! [`super::paragraph_properties::ParagraphTextFlowDirection`] directly: `wml.xsd` gives
//! `w:pPr/w:textDirection` and `w:sectPr/w:textDirection` the identical `CT_TextDirection` type and
//! the identical local name, so this is the same "consume, do not re-create" reuse as `Toggle`, not a
//! naming accident), `docGrid` ([`DocumentGrid`]), `printerSettings` (reuses
//! [`super::body::RelationshipReference`] — see [`SectionProperties::printer_settings`]'s own doc
//! comment for why the binary part it names is never this crate's to touch). `w:sectPrChange`
//! (`CT_SectPrChange`) stays [`super::body::Unmodeled`] — structure only; MJXOFF-126 owns its
//! semantics. `EG_HdrFtrReferences` (`headerReference`/`footerReference`, [`HeaderFooterReference`])
//! is modelled here too, per this child's own scope note: the field is reachable, but *which* header
//! or footer actually applies to a given page (the `w:titlePg` interaction in particular) is
//! MJXOFF-113's resolution to build.
//!
//! # `w:orient` does not swap `w:w`/`w:h` for you
//!
//! [`SectionProperties::set_page_size`] writes exactly the [`crate::PageSize`] it is given — width,
//! height and orientation are three independent numbers on the wire, and this crate's own
//! [`crate::PageSize::landscape`] is what performs the swap on the *value* before it ever reaches a
//! setter. A landscape A4 page is `w="16838" h="11906" orient="landscape"` together; setting `orient`
//! alone on an unswapped extent would write a self-contradicting `w:pgSz`.
//!
//! # `w:equalWidth="true"` beats an explicit `w:col` list — confirmed against ECMA-376 Part 1 §17.6.4
//!
//! A real file can carry both a non-empty `w:col` list *and* `w:equalWidth="true"` at once — the
//! ticket's own "real-file contradiction" — and the prose resolves it explicitly, not by inference:
//! "If `equalWidth` is true, then the columns are defined using the data stored as attributes of the
//! `cols` element (`num`/`space`) ... If `equalWidth` is false, then the columns are defined using the
//! presence and data on each child `col` element", with a worked example whose `w:col` children are
//! described as "ignored" once `equalWidth="1"` is set. **`equalWidth` wins**, unconditionally, even
//! over an explicit list. [`Columns`] does not resolve this itself (it has no page-margin knowledge to
//! compute a text-column width from `num`/`space`, and resolving it silently would hide the file's own
//! contradiction from a caller who might want to see it) — it exposes [`Columns::is_equal_width`] and
//! the explicit [`Columns::columns`] list independently, and this doc comment is the one place the
//! precedence is written down.

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement,
    RawName, RawNode, Text as TextCodec, ToXml,
};
use mjx_ooxml_types::child_order::{PAGE_BORDERS, SECTION_PROPERTIES};
use mjx_ooxml_types::support::OnOff;
use mjx_ooxml_types::wordprocessingml::{
    ChapterSeparator, DecimalNumber, DocumentGridType, HeaderFooterType, LineNumberRestart,
    NumberFormat, PageBorderDisplay, PageBorderOffset, PageBorderZOrder, PageOrientation,
    SectionBreakType, VerticalJustification,
};

use super::body::{wml_name, RelationshipReference, Unmodeled};
use super::run_properties::{Border, Toggle};

use super::property_macros::{toggle_property, value_property};

// -------------------------------------------------------------------------------------------
// LongHex — the same eight-digit-hex codec `styles.rs` declared for `w:rsid`-family attributes,
// reused here for `w:sectPr`'s own `AG_SectPrAttributes` (`rsidRPr`/`rsidDel`/`rsidR`/`rsidSect`).
// -------------------------------------------------------------------------------------------

use super::styles::LongHex;

// -------------------------------------------------------------------------------------------
// CT_SectType (w:type) — the section-break kind.
// -------------------------------------------------------------------------------------------

/// `CT_SectType` (`w:type`, "Section Type", §17.6.22) — which kind of section break starts this
/// section (`ST_SectionMark`: `nextPage`, `nextColumn`, `continuous`, `evenPage`, `oddPage`). `val`
/// carries no XSD default — Word's own behaviour when it is absent is `nextPage`, but that is a
/// convention this crate documents rather than a default the schema states, so
/// [`SectionType::kind`] returns `None` rather than asserting a value the file does not say.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<SectionBreakType>, accessor = kind))]
pub struct SectionType {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl SectionType {
    /// Builds a new `w:type` of `kind`.
    #[must_use]
    pub fn new(interner: &mut Interner, kind: SectionBreakType) -> Self {
        let mut value = Self {
            name: wml_name(interner, "type"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_kind(interner, Some(kind));
        value
    }
}

impl FromXml for SectionType {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for SectionType {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_PaperSource (w:paperSrc)
// -------------------------------------------------------------------------------------------

/// `CT_PaperSource` (`w:paperSrc`) — the printer tray for the first page and for the rest of the
/// section, both optional `ST_DecimalNumber` printer-defined tray codes.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "first", prefix = "w", codec = Number<DecimalNumber>, accessor = first_page_tray))]
#[xml(attribute(local = "other", prefix = "w", codec = Number<DecimalNumber>, accessor = other_pages_tray))]
pub struct PaperSource {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl PaperSource {
    /// Builds a new, empty `w:paperSrc` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "paperSrc"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for PaperSource {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PaperSource {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_PageBorder / CT_TopPageBorder / CT_BottomPageBorder — each xsd:extension base="CT_Border"
// (or, for the latter two, "CT_PageBorder"), reusing Border's own model wholesale rather than
// re-declaring its nine style attributes — see Border::extension_attributes[_mut] and this
// module's own doc comment.
// -------------------------------------------------------------------------------------------

/// `CT_PageBorder` (`xsd:extension base="CT_Border"` + optional `r:id`) — used for `w:pgBorders`'s
/// `left`/`right` slots. Wraps a [`Border`] (MJXOFF-94's own model of `CT_Border`'s nine style
/// attributes — style, colour, theme colour/tint/shade, width, spacing, shadow, frame) rather than
/// re-declaring them; see [`PageBorder::border`]/`border_mut` to reach them, and
/// [`PageBorder::relationship_id`]/`set_relationship_id` for the one attribute this extension adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageBorder {
    border: Border,
}

impl PageBorder {
    /// Builds a new page border of `style`, under wire name `local` (`"left"` or `"right"`).
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        local: &str,
        style: mjx_ooxml_types::wordprocessingml::BorderStyle,
    ) -> Self {
        Self {
            border: Border::new(interner, style).renamed(interner, local),
        }
    }

    /// This border's own style attributes (`CT_Border`'s nine) — MJXOFF-94's model, reused directly.
    #[must_use]
    pub fn border(&self) -> &Border {
        &self.border
    }

    /// [`PageBorder::border`], mutably.
    pub fn border_mut(&mut self) -> &mut Border {
        &mut self.border
    }

    /// `r:id` — an explicit relationship to a picture used as this border's image, or `None` if this
    /// border uses a plain line style instead.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "id",
            "r:id",
        )
    }

    /// Sets (or, given `None`, removes) `r:id`.
    pub fn set_relationship_id(&mut self, interner: &mut Interner, value: Option<&str>) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "id",
            value,
        );
    }
}

impl FromXml for PageBorder {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            border: Border::from_xml(element, interner)?,
        })
    }
}

impl ToXml for PageBorder {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        self.border.to_xml(interner)
    }
}

/// `CT_TopPageBorder` (`xsd:extension base="CT_PageBorder"` + `r:topLeft`/`r:topRight`) — used for
/// `w:pgBorders`'s `top` slot. Wraps a [`Border`] exactly as [`PageBorder`] does — see that type's
/// own doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopPageBorder {
    border: Border,
}

impl TopPageBorder {
    /// Builds a new top page border of `style`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        style: mjx_ooxml_types::wordprocessingml::BorderStyle,
    ) -> Self {
        Self {
            border: Border::new(interner, style).renamed(interner, "top"),
        }
    }

    /// This border's own style attributes.
    #[must_use]
    pub fn border(&self) -> &Border {
        &self.border
    }

    /// [`TopPageBorder::border`], mutably.
    pub fn border_mut(&mut self) -> &mut Border {
        &mut self.border
    }

    /// `r:id` — see [`PageBorder::relationship_id`].
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "id",
            "r:id",
        )
    }

    /// Sets (or, given `None`, removes) `r:id`.
    pub fn set_relationship_id(&mut self, interner: &mut Interner, value: Option<&str>) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "id",
            value,
        );
    }

    /// `r:topLeft` — a relationship to an image for this border's top-left corner.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn top_left_relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "topLeft",
            "r:topLeft",
        )
    }

    /// Sets (or, given `None`, removes) `r:topLeft`.
    pub fn set_top_left_relationship_id(&mut self, interner: &mut Interner, value: Option<&str>) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "topLeft",
            value,
        );
    }

    /// `r:topRight` — a relationship to an image for this border's top-right corner.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn top_right_relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "topRight",
            "r:topRight",
        )
    }

    /// Sets (or, given `None`, removes) `r:topRight`.
    pub fn set_top_right_relationship_id(&mut self, interner: &mut Interner, value: Option<&str>) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "topRight",
            value,
        );
    }
}

impl FromXml for TopPageBorder {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            border: Border::from_xml(element, interner)?,
        })
    }
}

impl ToXml for TopPageBorder {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        self.border.to_xml(interner)
    }
}

/// `CT_BottomPageBorder` (`xsd:extension base="CT_PageBorder"` + `r:bottomLeft`/`r:bottomRight`) —
/// used for `w:pgBorders`'s `bottom` slot. See [`TopPageBorder`]'s own doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BottomPageBorder {
    border: Border,
}

impl BottomPageBorder {
    /// Builds a new bottom page border of `style`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        style: mjx_ooxml_types::wordprocessingml::BorderStyle,
    ) -> Self {
        Self {
            border: Border::new(interner, style).renamed(interner, "bottom"),
        }
    }

    /// This border's own style attributes.
    #[must_use]
    pub fn border(&self) -> &Border {
        &self.border
    }

    /// [`BottomPageBorder::border`], mutably.
    pub fn border_mut(&mut self) -> &mut Border {
        &mut self.border
    }

    /// `r:id` — see [`PageBorder::relationship_id`].
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "id",
            "r:id",
        )
    }

    /// Sets (or, given `None`, removes) `r:id`.
    pub fn set_relationship_id(&mut self, interner: &mut Interner, value: Option<&str>) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "id",
            value,
        );
    }

    /// `r:bottomLeft` — a relationship to an image for this border's bottom-left corner.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn bottom_left_relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "bottomLeft",
            "r:bottomLeft",
        )
    }

    /// Sets (or, given `None`, removes) `r:bottomLeft`.
    pub fn set_bottom_left_relationship_id(
        &mut self,
        interner: &mut Interner,
        value: Option<&str>,
    ) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "bottomLeft",
            value,
        );
    }

    /// `r:bottomRight` — a relationship to an image for this border's bottom-right corner.
    ///
    /// # Errors
    /// An [`AttributeError`] if the attribute is present but malformed.
    pub fn bottom_right_relationship_id(
        &self,
        interner: &Interner,
    ) -> Result<Option<Cow<'_, str>>, AttributeError> {
        mjx_xml::attribute::read::<TextCodec>(
            self.border.extension_attributes(),
            interner,
            Some("r"),
            "bottomRight",
            "r:bottomRight",
        )
    }

    /// Sets (or, given `None`, removes) `r:bottomRight`.
    pub fn set_bottom_right_relationship_id(
        &mut self,
        interner: &mut Interner,
        value: Option<&str>,
    ) {
        mjx_xml::attribute::write::<TextCodec>(
            self.border.extension_attributes_mut(),
            interner,
            Some("r"),
            "bottomRight",
            value,
        );
    }
}

impl FromXml for BottomPageBorder {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            border: Border::from_xml(element, interner)?,
        })
    }
}

impl ToXml for BottomPageBorder {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        self.border.to_xml(interner)
    }
}

// -------------------------------------------------------------------------------------------
// CT_PageBorders (w:pgBorders) — top, left, bottom, right, each independently optional.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`PageBorderSet`]: `CT_PageBorders`' own sequence, `top, left, bottom,
/// right`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageBorderSetContent {
    /// `w:top` (`CT_TopPageBorder`).
    Top(TopPageBorder),
    /// `w:left` (`CT_PageBorder`).
    Left(PageBorder),
    /// `w:bottom` (`CT_BottomPageBorder`).
    Bottom(BottomPageBorder),
    /// `w:right` (`CT_PageBorder`).
    Right(PageBorder),
    /// Any other child — preserved verbatim.
    Raw(RawNode),
}

/// `CT_PageBorders` (`w:pgBorders`, "Page Borders") — the four borders drawn around every page in
/// this section, plus how they stack against page content (`zOrder`), which pages they appear on
/// (`display`) and whether they are measured from the page edge or the text (`offsetFrom`).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "zOrder", prefix = "w", codec = Enumeration<PageBorderZOrder>, accessor = z_order, default = PageBorderZOrder::Front))]
#[xml(attribute(local = "display", prefix = "w", codec = Enumeration<PageBorderDisplay>, accessor = display))]
#[xml(attribute(local = "offsetFrom", prefix = "w", codec = Enumeration<PageBorderOffset>, accessor = offset_from, default = PageBorderOffset::Text))]
pub struct PageBorderSet {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "top", variant = Top, ty = TopPageBorder),
        child(local = "left", variant = Left, ty = PageBorder),
        child(local = "bottom", variant = Bottom, ty = BottomPageBorder),
        child(local = "right", variant = Right, ty = PageBorder)
    )]
    content: Vec<PageBorderSetContent>,
}

impl PageBorderSet {
    /// Builds a new, empty `w:pgBorders`.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "pgBorders"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &PageBorderSetContent) -> Option<u16> {
        let local = match item {
            PageBorderSetContent::Top(_) => "top",
            PageBorderSetContent::Left(_) => "left",
            PageBorderSetContent::Bottom(_) => "bottom",
            PageBorderSetContent::Right(_) => "right",
            PageBorderSetContent::Raw(_) => return None,
        };
        PAGE_BORDERS.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&PageBorderSetContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: PageBorderSetContent) {
        let at = PAGE_BORDERS.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&PageBorderSetContent) -> bool,
        value: Option<PageBorderSetContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// `w:top` — the border above every page in this section.
    #[must_use]
    pub fn top(&self) -> Option<&TopPageBorder> {
        self.content.iter().find_map(|item| match item {
            PageBorderSetContent::Top(border) => Some(border),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:top`.
    pub fn set_top(&mut self, value: Option<TopPageBorder>) {
        let is_target = |item: &PageBorderSetContent| matches!(item, PageBorderSetContent::Top(_));
        self.set("top", is_target, value.map(PageBorderSetContent::Top));
    }

    /// `w:left` — the border to the left of every page in this section.
    #[must_use]
    pub fn left(&self) -> Option<&PageBorder> {
        self.content.iter().find_map(|item| match item {
            PageBorderSetContent::Left(border) => Some(border),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:left`.
    pub fn set_left(&mut self, value: Option<PageBorder>) {
        let is_target = |item: &PageBorderSetContent| matches!(item, PageBorderSetContent::Left(_));
        self.set("left", is_target, value.map(PageBorderSetContent::Left));
    }

    /// `w:bottom` — the border below every page in this section.
    #[must_use]
    pub fn bottom(&self) -> Option<&BottomPageBorder> {
        self.content.iter().find_map(|item| match item {
            PageBorderSetContent::Bottom(border) => Some(border),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:bottom`.
    pub fn set_bottom(&mut self, value: Option<BottomPageBorder>) {
        let is_target =
            |item: &PageBorderSetContent| matches!(item, PageBorderSetContent::Bottom(_));
        self.set("bottom", is_target, value.map(PageBorderSetContent::Bottom));
    }

    /// `w:right` — the border to the right of every page in this section.
    #[must_use]
    pub fn right(&self) -> Option<&PageBorder> {
        self.content.iter().find_map(|item| match item {
            PageBorderSetContent::Right(border) => Some(border),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:right`.
    pub fn set_right(&mut self, value: Option<PageBorder>) {
        let is_target =
            |item: &PageBorderSetContent| matches!(item, PageBorderSetContent::Right(_));
        self.set("right", is_target, value.map(PageBorderSetContent::Right));
    }
}

// -------------------------------------------------------------------------------------------
// CT_LineNumber (w:lnNumType)
// -------------------------------------------------------------------------------------------

/// `CT_LineNumber` (`w:lnNumType`, "Line Numbering Settings") — line numbering for this section:
/// which lines are numbered (`countBy`), where numbering starts, the distance from the text and the
/// restart rule.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "countBy", prefix = "w", codec = Number<DecimalNumber>, accessor = count_by))]
#[xml(attribute(local = "start", prefix = "w", codec = Number<DecimalNumber>, accessor = start, default = 1))]
#[xml(attribute(local = "distance", prefix = "w", codec = Number<u32>, accessor = distance_twips))]
#[xml(attribute(local = "restart", prefix = "w", codec = Enumeration<LineNumberRestart>, accessor = restart, default = LineNumberRestart::NewPage))]
pub struct LineNumbering {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl LineNumbering {
    /// Builds a new, empty `w:lnNumType` — every attribute absent (so every default above applies)
    /// until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "lnNumType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for LineNumbering {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for LineNumbering {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_PageNumber (w:pgNumType)
// -------------------------------------------------------------------------------------------

/// `CT_PageNumber` (`w:pgNumType`, "Page Numbering Settings") — page-number format, an optional
/// restart value, and (for East Asian chaptered documents) the chapter-heading style and separator.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "fmt", prefix = "w", codec = Enumeration<NumberFormat>, accessor = format, default = NumberFormat::Decimal))]
#[xml(attribute(local = "start", prefix = "w", codec = Number<DecimalNumber>, accessor = start))]
#[xml(attribute(local = "chapStyle", prefix = "w", codec = Number<DecimalNumber>, accessor = chapter_style))]
#[xml(attribute(local = "chapSep", prefix = "w", codec = Enumeration<ChapterSeparator>, accessor = chapter_separator, default = ChapterSeparator::Hyphen))]
pub struct PageNumbering {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl PageNumbering {
    /// Builds a new, empty `w:pgNumType` — every attribute absent (so every default above applies)
    /// until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "pgNumType"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for PageNumbering {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PageNumbering {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_Column / CT_Columns (w:col / w:cols)
// -------------------------------------------------------------------------------------------

/// `CT_Column` (`w:col`, "Column Definition") — one explicit column's width and the space after it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "w", prefix = "w", codec = Number<u32>, accessor = width_twips))]
#[xml(attribute(local = "space", prefix = "w", codec = Number<u32>, accessor = space_after_twips, default = 0))]
pub struct Column {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl Column {
    /// Builds a new `w:col` of `width_twips`, with no space stated (the schema default, `0`,
    /// applies).
    #[must_use]
    pub fn new(interner: &mut Interner, width_twips: u32) -> Self {
        let mut value = Self {
            name: wml_name(interner, "col"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_width_twips(interner, Some(width_twips));
        value
    }
}

impl FromXml for Column {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for Column {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

/// One ordered child of [`Columns`]: `CT_Columns`' own content is `col*` alone — a homogeneous,
/// repeatable list with no schema order to enforce among its members (mirrors
/// `paragraph_properties.rs`'s own [`super::paragraph_properties::TabStops`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnsContent {
    /// `w:col` (`CT_Column`).
    Column(Column),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Columns` (`w:cols`, "Column Definitions", §17.6.4) — the columns this section's text flows
/// through: either an explicit, ordered list of [`Column`]s, or (when [`Columns::is_equal_width`] is
/// true) `num` equal columns computed from `space` — see this module's own doc comment for which one
/// wins when a file states both.
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "equalWidth", prefix = "w", codec = OnOff, accessor = equal_width))]
#[xml(attribute(local = "space", prefix = "w", codec = Number<u32>, accessor = space_between_twips, default = 720))]
#[xml(attribute(local = "num", prefix = "w", codec = Number<DecimalNumber>, accessor = num, default = 1))]
#[xml(attribute(local = "sep", prefix = "w", codec = OnOff, accessor = separator_line))]
pub struct Columns {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "col", variant = Column, ty = Column))]
    content: Vec<ColumnsContent>,
}

impl Columns {
    /// Builds a new, empty `w:cols` — one column (the schema default), no explicit list.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "cols"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Whether this section's columns are computed from `w:num`/`w:space` rather than an explicit
    /// `w:col` list — `false` (the schema default) when `w:equalWidth` is absent. **When this is
    /// `true`, it wins over any explicit `w:col` list this element also carries** — see this
    /// module's own doc comment, which quotes ECMA-376 Part 1 §17.6.4 directly.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:equalWidth` is present but malformed.
    pub fn is_equal_width(&self, interner: &Interner) -> Result<bool, AttributeError> {
        Ok(self.equal_width(interner)?.unwrap_or(false))
    }

    /// The explicit column list (`w:col*`), in document order — meaningful only when
    /// [`Columns::is_equal_width`] is `false`; see this module's own doc comment.
    pub fn columns(&self) -> impl Iterator<Item = &Column> {
        self.content.iter().filter_map(|item| match item {
            ColumnsContent::Column(column) => Some(column),
            ColumnsContent::Raw(_) => None,
        })
    }

    /// How many explicit columns this list holds.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns().count()
    }

    /// Appends `column` as this list's new last explicit column.
    pub fn push_column(&mut self, column: Column) {
        self.content.push(ColumnsContent::Column(column));
        self.empty = false;
    }

    /// Removes and returns the explicit column at `index`, or `None` if there is no such column.
    pub fn remove_column(&mut self, index: usize) -> Option<Column> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, ColumnsContent::Column(_)))
            .nth(index)
            .map(|(at, _)| at)?;
        match self.content.remove(at) {
            ColumnsContent::Column(column) => Some(column),
            ColumnsContent::Raw(_) => {
                unreachable!("the filtered index only ever names a Column item")
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// CT_VerticalJc (w:vAlign)
// -------------------------------------------------------------------------------------------

/// `CT_VerticalJc` (`w:vAlign`, "Vertical Text Alignment on Page", §17.6.23) — the required vertical
/// alignment of this section's text within the page.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "val", prefix = "w", codec = Enumeration<VerticalJustification>, accessor = value, required))]
pub struct PageVerticalAlignment {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl PageVerticalAlignment {
    /// Builds a new `w:vAlign` of `value`.
    #[must_use]
    pub fn new(interner: &mut Interner, value: VerticalJustification) -> Self {
        let mut item = Self {
            name: wml_name(interner, "vAlign"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        item.set_value(interner, value);
        item
    }
}

impl FromXml for PageVerticalAlignment {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for PageVerticalAlignment {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_DocGrid (w:docGrid)
// -------------------------------------------------------------------------------------------

/// `CT_DocGrid` (`w:docGrid`, "Document Grid") — the East Asian typography grid this section's text
/// snaps to: its kind, the pitch between lines, and the spacing between characters.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<DocumentGridType>, accessor = kind))]
#[xml(attribute(local = "linePitch", prefix = "w", codec = Number<DecimalNumber>, accessor = line_pitch))]
#[xml(attribute(local = "charSpace", prefix = "w", codec = Number<DecimalNumber>, accessor = character_space))]
pub struct DocumentGrid {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl DocumentGrid {
    /// Builds a new, empty `w:docGrid` — every attribute absent until a setter states one.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "docGrid"),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        }
    }
}

impl FromXml for DocumentGrid {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for DocumentGrid {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_HdrFtrRef (w:headerReference / w:footerReference) — xsd:extension base="CT_Rel" + type.
// -------------------------------------------------------------------------------------------

/// `CT_HdrFtrRef` (`xsd:extension base="CT_Rel"` + required `type`) — a relationship to one header
/// or footer part, and which of the three kinds (`default`/`even`/`first`) it is. *Which* header or
/// footer actually applies to a given page — `w:titlePg`'s effect in particular — is MJXOFF-113's
/// resolution to build on top of this structural reference.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "id", prefix = "r", codec = TextCodec, accessor = relationship_id, required))]
#[xml(attribute(local = "type", prefix = "w", codec = Enumeration<HeaderFooterType>, accessor = kind, required))]
pub struct HeaderFooterReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    extra: Vec<RawNode>,
    empty: bool,
}

impl HeaderFooterReference {
    /// Builds a new `local` reference (`"headerReference"` or `"footerReference"`) of `kind`,
    /// pointing at `relationship_id`.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        local: &str,
        relationship_id: &str,
        kind: HeaderFooterType,
    ) -> Self {
        let mut value = Self {
            name: wml_name(interner, local),
            attributes: Vec::new(),
            extra: Vec::new(),
            empty: true,
        };
        value.set_relationship_id(interner, relationship_id);
        value.set_kind(interner, kind);
        value
    }
}

impl FromXml for HeaderFooterReference {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            extra: element.children.clone(),
            empty: element.empty,
        })
    }
}

impl ToXml for HeaderFooterReference {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let children = self.extra.clone();
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

// -------------------------------------------------------------------------------------------
// CT_SectPr (w:sectPr) — the section itself.
// -------------------------------------------------------------------------------------------

/// One ordered child of [`SectionProperties`]: `EG_HdrFtrReferences` (both variants share rank 0 —
/// the group is a repeatable `xsd:choice`, so header and footer references interleave freely ahead
/// of everything else), then `EG_SectPrContents`'s 19 members, then `w:sectPrChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionPropertyContent {
    /// `w:headerReference` (`CT_HdrFtrRef`).
    HeaderReference(HeaderFooterReference),
    /// `w:footerReference` (`CT_HdrFtrRef`).
    FooterReference(HeaderFooterReference),
    /// `w:footnotePr` (`CT_FtnProps`) — MJXOFF-124's own semantics; structurally opaque here.
    FootnoteProperties(Unmodeled),
    /// `w:endnotePr` (`CT_EdnProps`) — MJXOFF-124's own semantics; structurally opaque here.
    EndnoteProperties(Unmodeled),
    /// `w:type` (`CT_SectType`) — the section-break kind.
    Type(SectionType),
    /// `w:pgSz` (`CT_PageSz`) — structurally opaque ([`Unmodeled`]); reach it through
    /// [`SectionProperties::page_size`]/`set_page_size`/`page_size_code`/`set_page_size_code`, which
    /// bridge to the shared [`crate::PageSize`] value type rather than a second one — see this
    /// module's own doc comment.
    PageSize(Unmodeled),
    /// `w:pgMar` (`CT_PageMar`) — structurally opaque ([`Unmodeled`]); reach it through
    /// [`SectionProperties::page_margins`]/`set_page_margins`, which bridge to the shared
    /// [`crate::PageMargins`] value type.
    PageMargins(Unmodeled),
    /// `w:paperSrc` (`CT_PaperSource`).
    PaperSource(PaperSource),
    /// `w:pgBorders` (`CT_PageBorders`).
    PageBorders(PageBorderSet),
    /// `w:lnNumType` (`CT_LineNumber`).
    LineNumbering(LineNumbering),
    /// `w:pgNumType` (`CT_PageNumber`).
    PageNumbering(PageNumbering),
    /// `w:cols` (`CT_Columns`).
    Columns(Columns),
    /// `w:formProt` (§17.6.8, "Only Windows in Document") — `CT_OnOff`.
    FormProtected(Toggle),
    /// `w:vAlign` (`CT_VerticalJc`).
    VerticalAlignment(PageVerticalAlignment),
    /// `w:noEndnote` (§17.11.18, "Endnotes in Document") — `CT_OnOff`.
    NoEndnote(Toggle),
    /// `w:titlePg` (§17.10.7, "Different First Page Headers and Footers") — `CT_OnOff`. The flag
    /// alone is modelled here; *which* header/footer this selects is MJXOFF-113's.
    TitlePage(Toggle),
    /// `w:textDirection` (`CT_TextDirection`) — reuses
    /// [`super::paragraph_properties::ParagraphTextFlowDirection`] directly; see this module's own
    /// doc comment for why that is the correct reuse rather than a naming accident.
    TextDirection(super::paragraph_properties::ParagraphTextFlowDirection),
    /// `w:bidi` (§17.6.1, "Right to Left Section Layout") — `CT_OnOff`.
    RightToLeftLayout(Toggle),
    /// `w:rtlGutter` (§17.6.16, "Gutter on Right Side of Page") — `CT_OnOff`.
    RtlGutter(Toggle),
    /// `w:docGrid` (`CT_DocGrid`).
    DocumentGrid(DocumentGrid),
    /// `w:printerSettings` (`CT_Rel`) — reuses [`RelationshipReference`]; see
    /// [`SectionProperties::printer_settings`]'s own doc comment.
    PrinterSettings(RelationshipReference),
    /// `w:sectPrChange` (`CT_SectPrChange`) — structure only; MJXOFF-126 owns its semantics.
    Change(Unmodeled),
    /// Any other child — an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_SectPr` (`w:sectPr`) — a section's own page setup, columns, breaks and line numbering. Used
/// identically for a paragraph-level section break
/// ([`super::paragraph_properties::ParagraphProperties::section_properties`]) and the body-level
/// last section ([`super::body::Body::section_properties`]) — `wml.xsd` gives both the same
/// `CT_SectPr` type. `w:rsidRPr`/`w:rsidDel`/`w:rsidR`/`w:rsidSect` (`AG_SectPrAttributes`) are
/// preserved as ordinary typed attributes, not dropped to an unknown bucket, since real Word output
/// carries them on this element specifically (the ticket's own Constraints section names this).
#[derive(
    Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml, mjx_derive::XmlAttributes,
)]
#[xml(namespace = WML)]
#[xml(attribute(local = "rsidRPr", prefix = "w", codec = LongHex, accessor = rsid_run_properties))]
#[xml(attribute(local = "rsidDel", prefix = "w", codec = LongHex, accessor = rsid_deletion))]
#[xml(attribute(local = "rsidR", prefix = "w", codec = LongHex, accessor = rsid_revision))]
#[xml(attribute(local = "rsidSect", prefix = "w", codec = LongHex, accessor = rsid_section))]
pub struct SectionProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "headerReference", variant = HeaderReference, ty = HeaderFooterReference),
        child(local = "footerReference", variant = FooterReference, ty = HeaderFooterReference),
        child(local = "footnotePr", variant = FootnoteProperties, ty = Unmodeled),
        child(local = "endnotePr", variant = EndnoteProperties, ty = Unmodeled),
        child(local = "type", variant = Type, ty = SectionType),
        child(local = "pgSz", variant = PageSize, ty = Unmodeled),
        child(local = "pgMar", variant = PageMargins, ty = Unmodeled),
        child(local = "paperSrc", variant = PaperSource, ty = PaperSource),
        child(local = "pgBorders", variant = PageBorders, ty = PageBorderSet),
        child(local = "lnNumType", variant = LineNumbering, ty = LineNumbering),
        child(local = "pgNumType", variant = PageNumbering, ty = PageNumbering),
        child(local = "cols", variant = Columns, ty = Columns),
        child(local = "formProt", variant = FormProtected, ty = Toggle),
        child(local = "vAlign", variant = VerticalAlignment, ty = PageVerticalAlignment),
        child(local = "noEndnote", variant = NoEndnote, ty = Toggle),
        child(local = "titlePg", variant = TitlePage, ty = Toggle),
        child(local = "textDirection", variant = TextDirection, ty = super::paragraph_properties::ParagraphTextFlowDirection),
        child(local = "bidi", variant = RightToLeftLayout, ty = Toggle),
        child(local = "rtlGutter", variant = RtlGutter, ty = Toggle),
        child(local = "docGrid", variant = DocumentGrid, ty = DocumentGrid),
        child(local = "printerSettings", variant = PrinterSettings, ty = RelationshipReference),
        child(local = "sectPrChange", variant = Change, ty = Unmodeled)
    )]
    content: Vec<SectionPropertyContent>,
}

/// Finds the first item of `content` for which `select` returns a reference of `kind` (per
/// `w:type`), removes it and returns it — the shared body of
/// [`SectionProperties::remove_header_reference`]/`remove_footer_reference`.
fn remove_reference_of_kind(
    content: &mut Vec<SectionPropertyContent>,
    kind: HeaderFooterType,
    interner: &Interner,
    select: impl Fn(&SectionPropertyContent) -> Option<&HeaderFooterReference>,
) -> Result<Option<HeaderFooterReference>, AttributeError> {
    let mut target = None;
    for (index, item) in content.iter().enumerate() {
        if let Some(reference) = select(item) {
            if reference.kind(interner)? == kind {
                target = Some(index);
                break;
            }
        }
    }
    Ok(target.map(|index| match content.remove(index) {
        SectionPropertyContent::HeaderReference(reference)
        | SectionPropertyContent::FooterReference(reference) => reference,
        _ => unreachable!("select only ever matches a HeaderReference or FooterReference item"),
    }))
}

impl SectionProperties {
    /// Builds a new, empty `w:sectPr` — no properties, ready for this type's setters.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: wml_name(interner, "sectPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    fn rank(item: &SectionPropertyContent) -> Option<u16> {
        let local = match item {
            SectionPropertyContent::HeaderReference(_) => "headerReference",
            SectionPropertyContent::FooterReference(_) => "footerReference",
            SectionPropertyContent::FootnoteProperties(_) => "footnotePr",
            SectionPropertyContent::EndnoteProperties(_) => "endnotePr",
            SectionPropertyContent::Type(_) => "type",
            SectionPropertyContent::PageSize(_) => "pgSz",
            SectionPropertyContent::PageMargins(_) => "pgMar",
            SectionPropertyContent::PaperSource(_) => "paperSrc",
            SectionPropertyContent::PageBorders(_) => "pgBorders",
            SectionPropertyContent::LineNumbering(_) => "lnNumType",
            SectionPropertyContent::PageNumbering(_) => "pgNumType",
            SectionPropertyContent::Columns(_) => "cols",
            SectionPropertyContent::FormProtected(_) => "formProt",
            SectionPropertyContent::VerticalAlignment(_) => "vAlign",
            SectionPropertyContent::NoEndnote(_) => "noEndnote",
            SectionPropertyContent::TitlePage(_) => "titlePg",
            SectionPropertyContent::TextDirection(_) => "textDirection",
            SectionPropertyContent::RightToLeftLayout(_) => "bidi",
            SectionPropertyContent::RtlGutter(_) => "rtlGutter",
            SectionPropertyContent::DocumentGrid(_) => "docGrid",
            SectionPropertyContent::PrinterSettings(_) => "printerSettings",
            SectionPropertyContent::Change(_) => "sectPrChange",
            SectionPropertyContent::Raw(_) => return None,
        };
        SECTION_PROPERTIES.rank_of(None, local)
    }

    fn remove(&mut self, is_target: impl Fn(&SectionPropertyContent) -> bool) {
        if let Some(index) = self.content.iter().position(is_target) {
            self.content.remove(index);
        }
    }

    fn insert(&mut self, local: &str, item: SectionPropertyContent) {
        let at =
            SECTION_PROPERTIES.insert_index_of_names(self.content.iter().map(Self::rank), local);
        self.content.insert(at, item);
        self.empty = false;
    }

    fn set(
        &mut self,
        local: &str,
        is_target: impl Fn(&SectionPropertyContent) -> bool,
        value: Option<SectionPropertyContent>,
    ) {
        self.remove(is_target);
        if let Some(value) = value {
            self.insert(local, value);
        }
    }

    /// The header reference(s) this section states (`w:headerReference`), in document order —
    /// structural only; see this module's own doc comment for what MJXOFF-113 adds on top.
    pub fn header_references(&self) -> impl Iterator<Item = &HeaderFooterReference> {
        self.content.iter().filter_map(|item| match item {
            SectionPropertyContent::HeaderReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// Appends a new `w:headerReference`.
    pub fn push_header_reference(&mut self, reference: HeaderFooterReference) {
        self.insert(
            "headerReference",
            SectionPropertyContent::HeaderReference(reference),
        );
    }

    /// Removes this section's own `w:headerReference` of `kind`, if it states one, and returns it —
    /// MJXOFF-113's own removal primitive: `push_header_reference` alone (C9's) has no way to replace
    /// or drop a reference a section already carries, which "creating a header on demand" and
    /// "removing one, and cleaning up the part and relationship it leaves behind" both need.
    ///
    /// # Errors
    /// An [`AttributeError`] if a `w:headerReference@type` this scan reads is present but malformed.
    pub fn remove_header_reference(
        &mut self,
        kind: HeaderFooterType,
        interner: &Interner,
    ) -> Result<Option<HeaderFooterReference>, AttributeError> {
        remove_reference_of_kind(&mut self.content, kind, interner, |item| match item {
            SectionPropertyContent::HeaderReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// The footer reference(s) this section states (`w:footerReference`), in document order.
    pub fn footer_references(&self) -> impl Iterator<Item = &HeaderFooterReference> {
        self.content.iter().filter_map(|item| match item {
            SectionPropertyContent::FooterReference(reference) => Some(reference),
            _ => None,
        })
    }

    /// Appends a new `w:footerReference`.
    pub fn push_footer_reference(&mut self, reference: HeaderFooterReference) {
        self.insert(
            "footerReference",
            SectionPropertyContent::FooterReference(reference),
        );
    }

    /// Removes this section's own `w:footerReference` of `kind`, if it states one, and returns it —
    /// see [`SectionProperties::remove_header_reference`].
    ///
    /// # Errors
    /// An [`AttributeError`] if a `w:footerReference@type` this scan reads is present but malformed.
    pub fn remove_footer_reference(
        &mut self,
        kind: HeaderFooterType,
        interner: &Interner,
    ) -> Result<Option<HeaderFooterReference>, AttributeError> {
        remove_reference_of_kind(&mut self.content, kind, interner, |item| match item {
            SectionPropertyContent::FooterReference(reference) => Some(reference),
            _ => None,
        })
    }

    value_property!(
        SectionPropertyContent,
        break_kind,
        set_break_kind,
        Type,
        SectionType,
        "type",
        "`w:type` — this section's own break kind."
    );

    /// This section's page size and orientation (`w:pgSz`), as the shared [`crate::PageSize`] value
    /// — `None` if this section carries no `w:pgSz` at all. `w:pgSz@w`/`@h` are both optional in the
    /// schema; if either is absent, this returns `None` rather than guessing a physical extent the
    /// file does not state (use [`SectionProperties::page_size_code`] to reach the legacy paper-size
    /// code independently, which can be present even when the extent is not). `w:w`/`w:h` are read
    /// as raw `u32` twips (`Number<u32>`), not through the generated `s:ST_TwipsMeasure` wire-string
    /// wrapper — the same deliberate choice `crate::page`'s own module doc makes and explains.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:w`, `w:h` or `w:orient` is present but malformed.
    pub fn page_size(
        &self,
        interner: &Interner,
    ) -> Result<Option<crate::PageSize>, AttributeError> {
        let Some(element) = self.page_size_element() else {
            return Ok(None);
        };
        let attributes = element.attributes();
        let width_twips =
            mjx_xml::attribute::read::<Number<u32>>(attributes, interner, Some("w"), "w", "w:w")?;
        let height_twips =
            mjx_xml::attribute::read::<Number<u32>>(attributes, interner, Some("w"), "h", "w:h")?;
        let (Some(width_twips), Some(height_twips)) = (width_twips, height_twips) else {
            return Ok(None);
        };
        let orientation = mjx_xml::attribute::read::<Enumeration<PageOrientation>>(
            attributes,
            interner,
            Some("w"),
            "orient",
            "w:orient",
        )?
        .unwrap_or(PageOrientation::Portrait);
        Ok(Some(crate::PageSize {
            width_twips,
            height_twips,
            orientation,
        }))
    }

    /// Sets this section's page size and orientation (`w:pgSz@w`/`@h`/`@orient`), preserving
    /// whatever else `w:pgSz` already carries (its legacy `@code`, and any attribute this crate does
    /// not model) — `None` removes `w:pgSz` entirely. [`crate::PageOrientation::Portrait`] omits
    /// `@orient` (the schema default); any other value writes it explicitly — see
    /// `crate::page::orientation_wire_value` (crate-private; `crate::page`'s own module doc explains
    /// the rule).
    pub fn set_page_size(&mut self, interner: &mut Interner, value: Option<crate::PageSize>) {
        match value {
            None => self.remove(|item| matches!(item, SectionPropertyContent::PageSize(_))),
            Some(crate::PageSize {
                width_twips,
                height_twips,
                orientation,
            }) => {
                let element = self.page_size_element_or_insert(interner);
                let attributes = element.attributes_mut();
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "w",
                    Some(width_twips),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "h",
                    Some(height_twips),
                );
                mjx_xml::attribute::write::<Enumeration<PageOrientation>>(
                    attributes,
                    interner,
                    Some("w"),
                    "orient",
                    crate::page::orientation_wire_value(orientation).map(|_| orientation),
                );
            }
        }
    }

    /// The legacy paper-size code (`w:pgSz@code`), independent of [`SectionProperties::page_size`]
    /// — `None` if `w:pgSz` is absent or carries no `@code`.
    ///
    /// # Errors
    /// An [`AttributeError`] if `w:code` is present but malformed.
    pub fn page_size_code(&self, interner: &Interner) -> Result<Option<i64>, AttributeError> {
        match self.page_size_element() {
            Some(element) => mjx_xml::attribute::read::<Number<DecimalNumber>>(
                element.attributes(),
                interner,
                Some("w"),
                "code",
                "w:code",
            ),
            None => Ok(None),
        }
    }

    /// Sets (or, given `None`, removes) `w:pgSz@code`, creating an empty `w:pgSz` first if this
    /// section carries none yet.
    pub fn set_page_size_code(&mut self, interner: &mut Interner, value: Option<i64>) {
        let element = self.page_size_element_or_insert(interner);
        mjx_xml::attribute::write::<Number<DecimalNumber>>(
            element.attributes_mut(),
            interner,
            Some("w"),
            "code",
            value,
        );
    }

    fn page_size_element(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::PageSize(element) => Some(element),
            _ => None,
        })
    }

    fn page_size_element_or_insert(&mut self, interner: &mut Interner) -> &mut Unmodeled {
        if self.page_size_element().is_none() {
            self.insert(
                "pgSz",
                SectionPropertyContent::PageSize(Unmodeled::new(interner, "pgSz")),
            );
        }
        match self.content.iter_mut().find_map(|item| match item {
            SectionPropertyContent::PageSize(element) => Some(element),
            _ => None,
        }) {
            Some(element) => element,
            None => unreachable!("just found or inserted above"),
        }
    }

    /// This section's page margins (`w:pgMar`), as the shared [`crate::PageMargins`] value — `None`
    /// if this section carries no `w:pgMar` at all.
    ///
    /// # Errors
    /// [`AttributeError::Missing`] if `w:pgMar` is present but one of its seven required attributes
    /// is not (a non-conformant file — `CT_PageMar` declares all seven `use="required"`), or another
    /// [`AttributeError`] if one is present but malformed. `top`/`bottom` are read as `Number<i32>`
    /// (`ST_SignedTwipsMeasure` permits a negative margin, matching [`crate::PageMargins::top`]'s
    /// own `i32`); the rest are `Number<u32>`.
    pub fn page_margins(
        &self,
        interner: &Interner,
    ) -> Result<Option<crate::PageMargins>, AttributeError> {
        let Some(element) = self.page_margins_element() else {
            return Ok(None);
        };
        let attributes = element.attributes();
        let required_i32 = |local: &'static str, qualified: &'static str| {
            mjx_xml::attribute::read::<Number<i32>>(
                attributes,
                interner,
                Some("w"),
                local,
                qualified,
            )?
            .ok_or(AttributeError::Missing {
                attribute: qualified,
            })
        };
        let required_u32 = |local: &'static str, qualified: &'static str| {
            mjx_xml::attribute::read::<Number<u32>>(
                attributes,
                interner,
                Some("w"),
                local,
                qualified,
            )?
            .ok_or(AttributeError::Missing {
                attribute: qualified,
            })
        };
        Ok(Some(crate::PageMargins {
            top: required_i32("top", "w:top")?,
            right: required_u32("right", "w:right")?,
            bottom: required_i32("bottom", "w:bottom")?,
            left: required_u32("left", "w:left")?,
            header: required_u32("header", "w:header")?,
            footer: required_u32("footer", "w:footer")?,
            gutter: required_u32("gutter", "w:gutter")?,
        }))
    }

    /// Sets (or, given `None`, removes) this section's page margins (`w:pgMar`) wholesale — every
    /// one of the seven attributes is `use="required"`, so unlike [`SectionProperties::set_page_size`]
    /// there is no partial state to preserve.
    pub fn set_page_margins(&mut self, interner: &mut Interner, value: Option<crate::PageMargins>) {
        match value {
            None => self.remove(|item| matches!(item, SectionPropertyContent::PageMargins(_))),
            Some(crate::PageMargins {
                top,
                right,
                bottom,
                left,
                header,
                footer,
                gutter,
            }) => {
                let element = self.page_margins_element_or_insert(interner);
                let attributes = element.attributes_mut();
                mjx_xml::attribute::write::<Number<i32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "top",
                    Some(top),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "right",
                    Some(right),
                );
                mjx_xml::attribute::write::<Number<i32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "bottom",
                    Some(bottom),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "left",
                    Some(left),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "header",
                    Some(header),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "footer",
                    Some(footer),
                );
                mjx_xml::attribute::write::<Number<u32>>(
                    attributes,
                    interner,
                    Some("w"),
                    "gutter",
                    Some(gutter),
                );
            }
        }
    }

    fn page_margins_element(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::PageMargins(element) => Some(element),
            _ => None,
        })
    }

    fn page_margins_element_or_insert(&mut self, interner: &mut Interner) -> &mut Unmodeled {
        if self.page_margins_element().is_none() {
            self.insert(
                "pgMar",
                SectionPropertyContent::PageMargins(Unmodeled::new(interner, "pgMar")),
            );
        }
        match self.content.iter_mut().find_map(|item| match item {
            SectionPropertyContent::PageMargins(element) => Some(element),
            _ => None,
        }) {
            Some(element) => element,
            None => unreachable!("just found or inserted above"),
        }
    }

    value_property!(
        SectionPropertyContent,
        paper_source,
        set_paper_source,
        PaperSource,
        PaperSource,
        "paperSrc",
        "`w:paperSrc` — the printer tray for this section's pages."
    );
    value_property!(
        SectionPropertyContent,
        page_borders,
        set_page_borders,
        PageBorders,
        PageBorderSet,
        "pgBorders",
        "`w:pgBorders` — the four borders drawn around every page in this section."
    );
    value_property!(
        SectionPropertyContent,
        line_numbering,
        set_line_numbering,
        LineNumbering,
        LineNumbering,
        "lnNumType",
        "`w:lnNumType` — this section's line-numbering settings."
    );
    value_property!(
        SectionPropertyContent,
        page_numbering,
        set_page_numbering,
        PageNumbering,
        PageNumbering,
        "pgNumType",
        "`w:pgNumType` — this section's page-numbering settings."
    );
    value_property!(
        SectionPropertyContent,
        columns,
        set_columns,
        Columns,
        Columns,
        "cols",
        "`w:cols` — the columns this section's text flows through."
    );
    toggle_property!(
        SectionPropertyContent,
        form_protected,
        set_form_protected,
        FormProtected,
        "formProt",
        "`w:formProt` — whether this section is restricted to only allow editing form fields."
    );
    value_property!(
        SectionPropertyContent,
        vertical_alignment,
        set_vertical_alignment,
        VerticalAlignment,
        PageVerticalAlignment,
        "vAlign",
        "`w:vAlign` — this section's text's vertical alignment on the page."
    );
    toggle_property!(
        SectionPropertyContent,
        no_endnote,
        set_no_endnote,
        NoEndnote,
        "noEndnote",
        "`w:noEndnote` — whether endnotes for this section are displayed at the document's end \
         rather than the section's end."
    );
    toggle_property!(
        SectionPropertyContent,
        title_page,
        set_title_page,
        TitlePage,
        "titlePg",
        "`w:titlePg` — whether this section has a distinct first-page header/footer. The flag \
         alone; *which* header/footer that resolves to is MJXOFF-113's."
    );
    value_property!(
        SectionPropertyContent,
        text_direction,
        set_text_direction,
        TextDirection,
        super::paragraph_properties::ParagraphTextFlowDirection,
        "textDirection",
        "`w:textDirection` — this section's text flow direction."
    );
    toggle_property!(
        SectionPropertyContent,
        right_to_left_layout,
        set_right_to_left_layout,
        RightToLeftLayout,
        "bidi",
        "`w:bidi` — whether this section lays out right-to-left."
    );
    toggle_property!(
        SectionPropertyContent,
        rtl_gutter,
        set_rtl_gutter,
        RtlGutter,
        "rtlGutter",
        "`w:rtlGutter` — whether this section's page gutter is on the right rather than the left."
    );
    value_property!(
        SectionPropertyContent,
        document_grid,
        set_document_grid,
        DocumentGrid,
        DocumentGrid,
        "docGrid",
        "`w:docGrid` — this section's East Asian typography document grid."
    );

    /// A relationship to a Printer Settings part (`w:printerSettings`) carrying this section's
    /// printer configuration, or `None` if it carries none — structural only. **The binary part this
    /// names is never rewritten by this crate**: nothing in `mjx-docx` parses or regenerates printer
    /// settings payloads, so editing any other part of this (or any other) section leaves the
    /// referenced part's bytes, its relationship id and its target byte-for-byte untouched — the
    /// same copy-on-write guarantee every part this crate does not touch already gets from
    /// [`mjx_ooxml_core::ToXml::write_back`] and `mjx_opc`'s package model.
    #[must_use]
    pub fn printer_settings(&self) -> Option<&RelationshipReference> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::PrinterSettings(reference) => Some(reference),
            _ => None,
        })
    }

    /// Sets (or, given `None`, removes) `w:printerSettings`. Setting a *new* reference here only
    /// ever writes the `r:id` attribute this element carries — creating the target part and its
    /// relationship (or removing them) is the caller's own job through [`mjx_opc::Package`], exactly
    /// as this crate never authors a Printer Settings part's payload.
    pub fn set_printer_settings(&mut self, value: Option<RelationshipReference>) {
        let is_target = |item: &SectionPropertyContent| {
            matches!(item, SectionPropertyContent::PrinterSettings(_))
        };
        self.set(
            "printerSettings",
            is_target,
            value.map(SectionPropertyContent::PrinterSettings),
        );
    }

    /// The section-change tracking wrapper (`w:sectPrChange`), or `None` if this section carries
    /// none. Its semantics are MJXOFF-126's; this crate preserves it structurally.
    #[must_use]
    pub fn change(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::Change(change) => Some(change),
            _ => None,
        })
    }

    /// The footnote properties this section states (`w:footnotePr`), or `None` — MJXOFF-124's own
    /// semantics; preserved structurally here.
    #[must_use]
    pub fn footnote_properties(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::FootnoteProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The endnote properties this section states (`w:endnotePr`), or `None` — see
    /// [`SectionProperties::footnote_properties`].
    #[must_use]
    pub fn endnote_properties(&self) -> Option<&Unmodeled> {
        self.content.iter().find_map(|item| match item {
            SectionPropertyContent::EndnoteProperties(properties) => Some(properties),
            _ => None,
        })
    }
}

// -------------------------------------------------------------------------------------------
// Section addressing: which paragraphs a section governs.
// -------------------------------------------------------------------------------------------

/// One section of a document: the paragraphs it governs (in [`super::body::Body::paragraph`]'s own
/// 0-based indexing) and the [`SectionProperties`] that ends it — see this module's own doc comment
/// for why "ends" and not "starts" is the correct word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpan {
    /// The first paragraph index this section governs.
    pub first_paragraph: usize,
    /// The last paragraph index this section governs, inclusive — `None` if this section governs no
    /// paragraph at all (two section breaks with no paragraph between them, or a trailing
    /// body-level section after the body's last paragraph-ending `w:sectPr`).
    pub last_paragraph: Option<usize>,
    /// This section's own properties: the paragraph-level `w:sectPr` that ends it, or the
    /// body-level one for the document's last section. `None` only for a body with leftover
    /// paragraphs after the last paragraph-level `w:sectPr` (or with paragraphs at all and no
    /// paragraph-level `w:sectPr` anywhere) that itself carries no body-level `w:sectPr` either —
    /// schema-legal (`CT_Body/sectPr` is `minOccurs="0"`), though no fixture in this workspace's
    /// corpus is shaped this way; real Word output always writes one.
    pub properties: Option<SectionProperties>,
}

/// Walks `body`'s paragraphs in document order and returns every section, in document order.
///
/// A body with `n` paragraph-level `w:sectPr`s and a body-level one returns `n + 1` spans. Reading
/// only [`super::body::Body::section_properties`] (the body-level `w:sectPr`) and ignoring every
/// paragraph-level one — the mistake this crate's own module doc and MJXOFF-109's ticket both name
/// as the trap a single-section fixture cannot catch — collapses every document to exactly one span
/// covering every paragraph, which is wrong for any document with more than one section; see
/// `crates/mjx-docx/tests/sections.rs` for the mutation that proves it.
#[must_use]
pub(crate) fn sections_in(body: &super::body::Body) -> Vec<SectionSpan> {
    let paragraphs: Vec<_> = body.paragraphs().collect();
    let mut spans = Vec::new();
    let mut start = 0usize;
    for (index, paragraph) in paragraphs.iter().enumerate() {
        let Some(properties) = paragraph
            .properties()
            .and_then(super::paragraph_properties::ParagraphProperties::section_properties)
        else {
            continue;
        };
        spans.push(SectionSpan {
            first_paragraph: start,
            last_paragraph: Some(index),
            properties: Some(properties.clone()),
        });
        start = index + 1;
    }
    let body_properties = body.section_properties().cloned();
    if body_properties.is_some() || start < paragraphs.len() {
        let last_paragraph = if start < paragraphs.len() {
            Some(paragraphs.len() - 1)
        } else {
            None
        };
        spans.push(SectionSpan {
            first_paragraph: start,
            last_paragraph,
            properties: body_properties,
        });
    }
    spans
}

/// Which `w:sectPr` a section-editing [`super::super::Document`] method addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionLocation {
    /// The `w:sectPr` inside the given paragraph's own `w:pPr` — ending the section at that
    /// paragraph (see this module's own doc comment for why that is not "starting" one).
    Paragraph(crate::address::BlockPath),
    /// The body-level `w:sectPr` — the document's last section.
    Body,
}

impl From<crate::address::BlockPath> for SectionLocation {
    fn from(path: crate::address::BlockPath) -> Self {
        Self::Paragraph(path)
    }
}
