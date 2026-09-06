//! Print setup: the four elements every sheet *kind* carries, plus the background picture.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_PrintOptions` | 2809 | `x:printOptions` (rank 19 of `CT_Worksheet`) |
//! | `CT_PageMargins` | 2801 | `x:pageMargins` (rank 20) |
//! | `CT_PageSetup` | 2816 | `x:pageSetup` (rank 21) |
//! | `CT_HeaderFooter` | 2857 | `x:headerFooter` (rank 22) |
//! | `CT_CsPageSetup` | 3005 | a **chartsheet's** `x:pageSetup` — a different complex type |
//! | `CT_SheetBackgroundPicture` | 4347 | `x:picture` (rank 33) |
//!
//! # Why this is not in [`crate::worksheet`]
//!
//! `CT_Worksheet` is not the only type that holds these. `CT_Chartsheet` (`sml.xsd:2955`),
//! `CT_Dialogsheet` (`2150`) and `CT_Macrosheet` (`2118`) each declare `printOptions` /
//! `pageMargins` / `pageSetup` / `headerFooter` in their own sequences, and so does
//! [`CustomSheetView`](super::custom_views::CustomSheetView) — which is why the print block is a
//! *feature* rather than a worksheet slot, and why [`crate::sheets`] can build three more part
//! models on top of it without a second copy.
//!
//! The one thing that is genuinely per-kind is `pageSetup`: a chartsheet's is
//! [`ChartSheetPageSetup`] (`CT_CsPageSetup`), which is [`PageSetup`] minus the six attributes that
//! only mean something over a grid — `scale`, `fitToWidth`, `fitToHeight`, `pageOrder`,
//! `cellComments` and `errors`. Two types rather than one optional-field type, because the schema
//! declares two and a chartsheet that wrote `fitToWidth` would not validate.
//!
//! # Nothing here paginates
//!
//! `fitToWidth="2"` is **reported**. Where the page breaks fall, how many pages a sheet occupies,
//! what a margin is in device units: none of that is computed anywhere in this workspace, and the
//! constraint is the ticket's own — *"Reporting `fitToWidth` is in scope; computing where pages
//! break is rendering."* [`PageBreaks`](crate::PageBreaks) is the same rule from the other side: a
//! break is a record of where a person put one, never a repagination.
//!
//! # The printer-settings part is opaque, and this crate cannot see it
//!
//! `CT_PageSetup` and `CT_CsPageSetup` each declare `xsd:attribute ref="r:id"` — a relationship to a
//! **printer settings part**, whose content ECMA-376 Part 1 §15.2.13 places no requirement on at
//! all. It is a Windows `DEVMODE` blob in every file this project has seen. It is carried through
//! `mjx-opc`'s part-level copy-on-write untouched, and nothing here parses a byte of it. As
//! everywhere else in this crate, the `r:id` is held as **the string the file wrote**; resolving one
//! to a part is `mjx-xlsx`'s.
//!
//! # Header and footer strings are opaque, and are never re-emitted from a parse
//!
//! This is the sharpest rule in this file and it has a name: **never re-serialise**. A header string
//! carries Excel's formatting codes — `&L`, `&C`, `&R` for the three sections, `&P`, `&N`, `&D`,
//! `&T`, `&F`, `&A` for substitutions, `&"Arial,Bold"` for a font, `&G` for the drawing in
//! `drawingHF`, and `&&` for a literal ampersand. Two strings can mean the same thing and *be*
//! different strings, so parsing one into segments and rendering it back is a rewrite of the file
//! for no reason a caller asked for.
//!
//! [`HeaderFooterText`] therefore holds the element's children **exactly as the file wrote them**
//! and replays them until [`set_text`](HeaderFooterText::set_text) says otherwise — the same
//! hand-written `FromXml`/`ToXml` pair [`DefinedName`](crate::DefinedName) has, and for the same
//! reason. See [`HeaderFooterText`]'s own documentation for what that reason is; it is a real defect
//! in the derived grammar rather than a stylistic choice.
//!
//! The reading accessors ([`section_runs`](HeaderFooterText::section_runs),
//! [`unsectioned_text`](HeaderFooterText::unsectioned_text),
//! [`contains_drawing_reference`](HeaderFooterText::contains_drawing_reference)) hand back
//! **borrowed slices of the one stored string**. There is no representation to round-trip through,
//! because there is no second representation.

use mjx_ooxml_core::{
    Enumeration, FromXml, FromXmlError, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml,
};
use mjx_ooxml_types::spreadsheetml::{CellComments, PageOrder, PrintError, PrintOrientation};
use mjx_ooxml_types::support::OnOff;

use crate::leaf::{attribute_bag, bag_without_declared_attributes, relationship_reference};
use crate::worksheet::rebuild_element;

attribute_bag! {
    /// `x:printOptions` (`CT_PrintOptions`, `sml.xsd:2809`) — what a printed sheet shows beside its
    /// cells.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_PrintOptions`. Wire element: `printOptions`, rank **19** of
    /// `CT_Worksheet`.
    ///
    /// Five booleans, every one `use="optional"` with a schema default, so a bare `<printOptions/>`
    /// is legal markup meaning "all defaults".
    ///
    /// `@gridLines` prints the grid; `@gridLinesSet` is the *stored user preference* behind it and
    /// defaults to `true` even when `@gridLines` is `false`. They are two attributes and this type
    /// reports two: reconciling them would be inventing a policy ECMA-376 does not state.
    #[xml(attribute(local = "horizontalCentered", codec = OnOff, accessor = centred_horizontally, default = false))]
    #[xml(attribute(local = "verticalCentered", codec = OnOff, accessor = centred_vertically, default = false))]
    #[xml(attribute(local = "headings", codec = OnOff, accessor = prints_row_and_column_headings, default = false))]
    #[xml(attribute(local = "gridLines", codec = OnOff, accessor = prints_grid_lines, default = false))]
    #[xml(attribute(local = "gridLinesSet", codec = OnOff, accessor = grid_lines_preference_set, default = true))]
    PrintOptions, "printOptions"
}

attribute_bag! {
    /// `x:pageMargins` (`CT_PageMargins`, `sml.xsd:2801`) — the six margins of a printed page, in
    /// **inches**.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_PageMargins`. Wire element: `pageMargins`, rank **20** of
    /// `CT_Worksheet`.
    ///
    /// Every one of the six is `xsd:double` and `use="required"`, which is unusual in this schema
    /// and is why each accessor returns `Result` rather than `Option`: an absent `@left` is a defect
    /// in the file, and reporting it as "no left margin" would be inventing a value.
    ///
    /// The unit is inches and the schema does not say so — ECMA-376 Part 1 §18.3.1.62 does. It is in
    /// the accessor names because `left` alone is exactly the kind of identifier this project's
    /// naming rule exists to forbid.
    ///
    /// `@header` and `@footer` are the distance from the sheet edge to the *header* and *footer*
    /// bands, not extra margins added to `@top` and `@bottom`.
    #[xml(attribute(local = "left", codec = Number<f64>, accessor = left_inches, required))]
    #[xml(attribute(local = "right", codec = Number<f64>, accessor = right_inches, required))]
    #[xml(attribute(local = "top", codec = Number<f64>, accessor = top_inches, required))]
    #[xml(attribute(local = "bottom", codec = Number<f64>, accessor = bottom_inches, required))]
    #[xml(attribute(local = "header", codec = Number<f64>, accessor = header_inches, required))]
    #[xml(attribute(local = "footer", codec = Number<f64>, accessor = footer_inches, required))]
    PageMargins, "pageMargins"
}

attribute_bag! {
    /// `x:pageSetup` (`CT_PageSetup`, `sml.xsd:2816`) — paper, orientation, scaling and the
    /// relationship to a saved printer configuration.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_PageSetup`. Wire element: `pageSetup`, rank **21** of
    /// `CT_Worksheet`. A chartsheet's slot of the same name holds [`ChartSheetPageSetup`]
    /// (`CT_CsPageSetup`) instead.
    ///
    /// # `@scale` and `@fitToWidth`/`@fitToHeight` are two scaling modes, and the element records
    /// both
    ///
    /// `@scale` is a percentage; `@fitToWidth` and `@fitToHeight` are page counts. Which one a
    /// consumer honours is decided by `sheetPr/pageSetUpPr/@fitToPage`
    /// ([`PageSetupProperties`](crate::PageSetupProperties)), **a different element in a different
    /// slot**, and a file routinely carries values for both. Nothing here clears one because the
    /// other is set.
    ///
    /// # `@paperSize` is an index into a table this crate does not have
    ///
    /// It is an `xsd:unsignedInt` whose meaning is the Windows `DMPAPER_*` enumeration —
    /// ECMA-376 Part 1 §18.3.1.63 lists the values, and `1` is US Letter. This type reports the
    /// number the file wrote and never converts it to a size: `@paperHeight`/`@paperWidth` are the
    /// element's own answer to that question when a producer chose to write them.
    ///
    /// `@paperHeight` and `@paperWidth` are `ST_PositiveUniversalMeasure` — a number with a unit
    /// suffix (`210mm`, `8.5in`) — carried here as the text the file wrote, because a decimal
    /// conversion could not be written back without changing the bytes.
    ///
    /// `@r:id` names a **printer settings part**, reached through
    /// [`relationship_id`](Self::relationship_id) rather than declared through the attribute
    /// grammar, for the reason [`Hyperlink`](crate::Hyperlink)'s is: the prefix is the file's
    /// choice.
    #[xml(attribute(local = "paperSize", codec = Number<u32>, accessor = paper_size_index, default = 1))]
    #[xml(attribute(local = "paperHeight", codec = Text, accessor = paper_height))]
    #[xml(attribute(local = "paperWidth", codec = Text, accessor = paper_width))]
    #[xml(attribute(local = "scale", codec = Number<u32>, accessor = scale_percentage, default = 100))]
    #[xml(attribute(local = "firstPageNumber", codec = Number<u32>, accessor = first_page_number, default = 1))]
    #[xml(attribute(local = "fitToWidth", codec = Number<u32>, accessor = pages_wide, default = 1))]
    #[xml(attribute(local = "fitToHeight", codec = Number<u32>, accessor = pages_tall, default = 1))]
    #[xml(attribute(local = "pageOrder", codec = Enumeration<PageOrder>, accessor = page_order, default = PageOrder::DownThenOver))]
    #[xml(attribute(local = "orientation", codec = Enumeration<PrintOrientation>, accessor = orientation, default = PrintOrientation::Default))]
    #[xml(attribute(local = "usePrinterDefaults", codec = OnOff, accessor = uses_printer_defaults, default = true))]
    #[xml(attribute(local = "blackAndWhite", codec = OnOff, accessor = prints_black_and_white, default = false))]
    #[xml(attribute(local = "draft", codec = OnOff, accessor = prints_draft_quality, default = false))]
    #[xml(attribute(local = "cellComments", codec = Enumeration<CellComments>, accessor = cell_comment_printing, default = CellComments::None))]
    #[xml(attribute(local = "useFirstPageNumber", codec = OnOff, accessor = uses_first_page_number, default = false))]
    #[xml(attribute(local = "errors", codec = Enumeration<PrintError>, accessor = error_printing, default = PrintError::Displayed))]
    #[xml(attribute(local = "horizontalDpi", codec = Number<u32>, accessor = horizontal_dots_per_inch, default = 600))]
    #[xml(attribute(local = "verticalDpi", codec = Number<u32>, accessor = vertical_dots_per_inch, default = 600))]
    #[xml(attribute(local = "copies", codec = Number<u32>, accessor = copies, default = 1))]
    PageSetup, "pageSetup"
}

relationship_reference!(PageSetup);

attribute_bag! {
    /// `x:pageSetup` **on a chartsheet** (`CT_CsPageSetup`, `sml.xsd:3005`).
    ///
    /// **`ST_`/`CT_` symbol:** `CT_CsPageSetup`. Wire element: `pageSetup`, rank **5** of
    /// `CT_Chartsheet` and rank **1** of `CT_CustomChartsheetView`.
    ///
    /// [`PageSetup`] minus the six attributes that only mean something over a grid of cells:
    /// `@scale`, `@fitToWidth`, `@fitToHeight`, `@pageOrder`, `@cellComments` and `@errors`. A
    /// chartsheet has one chart on one page, so there is nothing to scale to a page count, no
    /// row-versus-column order to print in, no cell comments and no cell errors.
    ///
    /// It is a separate type rather than [`PageSetup`] with six unused fields because the schema
    /// declares two complex types: a chartsheet whose `pageSetup` carried `fitToWidth` would not
    /// validate, and a model that let a caller write one would be a model that produces invalid
    /// markup.
    #[xml(attribute(local = "paperSize", codec = Number<u32>, accessor = paper_size_index, default = 1))]
    #[xml(attribute(local = "paperHeight", codec = Text, accessor = paper_height))]
    #[xml(attribute(local = "paperWidth", codec = Text, accessor = paper_width))]
    #[xml(attribute(local = "firstPageNumber", codec = Number<u32>, accessor = first_page_number, default = 1))]
    #[xml(attribute(local = "orientation", codec = Enumeration<PrintOrientation>, accessor = orientation, default = PrintOrientation::Default))]
    #[xml(attribute(local = "usePrinterDefaults", codec = OnOff, accessor = uses_printer_defaults, default = true))]
    #[xml(attribute(local = "blackAndWhite", codec = OnOff, accessor = prints_black_and_white, default = false))]
    #[xml(attribute(local = "draft", codec = OnOff, accessor = prints_draft_quality, default = false))]
    #[xml(attribute(local = "useFirstPageNumber", codec = OnOff, accessor = uses_first_page_number, default = false))]
    #[xml(attribute(local = "horizontalDpi", codec = Number<u32>, accessor = horizontal_dots_per_inch, default = 600))]
    #[xml(attribute(local = "verticalDpi", codec = Number<u32>, accessor = vertical_dots_per_inch, default = 600))]
    #[xml(attribute(local = "copies", codec = Number<u32>, accessor = copies, default = 1))]
    ChartSheetPageSetup, "pageSetup"
}

relationship_reference!(ChartSheetPageSetup);

bag_without_declared_attributes! {
    /// `x:picture` (`CT_SheetBackgroundPicture`, `sml.xsd:4347`) — the image drawn behind a sheet's
    /// cells.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_SheetBackgroundPicture`. Wire element: `picture`, rank **33** of
    /// `CT_Worksheet` and rank **11** of `CT_Chartsheet`.
    ///
    /// One attribute, `xsd:attribute ref="r:id" use="required"`, naming an image part. The image
    /// itself is bytes `mjx-opc` carries verbatim; nothing here decodes one, and
    /// [`mjx_opc::ImageFormat`](https://docs.rs/mjx-opc) identifies a format from a signature
    /// without ever decoding a pixel.
    ///
    /// **A background picture is not a drawing.** It is not anchored to cells, it has no
    /// `xdr:wsDr` part, and it is unrelated to the `drawing` slot at rank 29 — which is
    /// MJXOFF-107's (E3).
    SheetBackgroundPicture, "picture"
}

relationship_reference!(SheetBackgroundPicture);

// -----------------------------------------------------------------------------------------------
// `CT_HeaderFooter` and the six opaque strings in it
// -----------------------------------------------------------------------------------------------

/// Which of a header or footer's three areas a run of text belongs to.
///
/// ECMA-376 Part 1 §18.3.1.46: *"Each header and footer is divided into three areas: a left section,
/// a center section, and a right section, which are specified, respectively, by one or more
/// left-section-specifiers, one or more center-section specifiers, and one or more right-section
/// specifiers."*
///
/// The spelling is the spec's — `&C` is a *center* section, not a centre one — because the variant
/// names a wire code rather than a piece of prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderFooterSection {
    /// `&L` — the left section.
    Left,
    /// `&C` — the center section.
    Center,
    /// `&R` — the right section.
    Right,
}

impl HeaderFooterSection {
    /// Every section, in the order the codes are listed in ECMA-376 Part 1 §18.3.1.46.
    pub const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// The single character that follows `&` in this section's specifier.
    #[must_use]
    pub fn code(self) -> char {
        match self {
            Self::Left => 'L',
            Self::Center => 'C',
            Self::Right => 'R',
        }
    }
}

/// One of `CT_HeaderFooter`'s six `s:ST_Xstring` children — `x:oddHeader`, `x:oddFooter`,
/// `x:evenHeader`, `x:evenFooter`, `x:firstHeader` or `x:firstFooter`.
///
/// # Never re-serialised
///
/// The string is Excel's formatting-code language and the *bytes* are what matters, so this type has
/// exactly one representation of its content: the decoded [`text`](Self::text), plus the element's
/// original children replayed verbatim until [`set_text`](Self::set_text) replaces them. There is no
/// parsed form to write back from, so there is no way for a round trip to change a code.
///
/// # Why this is not `#[derive(FromXml, ToXml)]` with `#[xml(text)]`
///
/// `mjx-derive`'s `#[xml(text)]` grammar decodes character data on read and re-escapes it
/// **minimally** on write — only `<` and `&`. That is right for authoring and lossy for
/// preservation: a producer that wrote `&amp;amp;L` gets `&amp;L` back, and one that wrote
/// `&amp;#38;L` gets `&amp;L` too. Same string, different bytes — and a rebuilt text node that
/// differs from the original denies its element, *and every ancestor of it*, the verbatim source
/// range subtree copy-on-write would otherwise give it.
///
/// Nothing notices while a part is untouched, because then the model never writes at all. It becomes
/// visible the moment anything **else** in the part changes. [`DefinedName`](crate::DefinedName)
/// found this first (MJXOFF-100) and solved it the same way; the epic records the gap as latent
/// until here, and a header string is `s:ST_Xstring` exactly as a defined name's content is. The
/// fix belongs in the derive and no work item owns it, so the two hand-written pairs stand — and
/// this documentation is the second half of that record.
///
/// # The reading accessors are read-only *by construction*
///
/// [`section_runs`](Self::section_runs) and [`unsectioned_text`](Self::unsectioned_text) return
/// `&str` **slices of the stored string**. They allocate nothing, own nothing, and cannot be handed
/// back: there is no `from_sections` constructor and there will not be one, because assembling a
/// code string from segments is precisely the re-emission this type exists to prevent.
/// # No attribute is declared, because the schema declares none
///
/// The six children are `type="s:ST_Xstring"` — a *simple* type, which permits no attributes at
/// all. An `xml:space` on one is the same producer divergence `mjx-schema-gate` records as a
/// tolerated deviation on `sample.xlsx`'s `sharedStrings.xml`, so this type preserves the attribute
/// vector verbatim and declares no accessor over it: a typed getter would read as a claim that the
/// attribute is legal here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFooterText {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    /// The character data, decoded — what [`text`](Self::text) answers with.
    text: String,
    /// The element's children exactly as the file wrote them, or `None` once the text has been
    /// replaced and there is nothing left to preserve.
    verbatim: Option<Vec<RawNode>>,
}

impl FromXml for HeaderFooterText {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        let mut text = String::new();
        for child in &element.children {
            match child {
                RawNode::Text(bytes) => {
                    let raw = core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                    let decoded = mjx_xml::text::unescape_text(raw)
                        .map_err(|error| FromXmlError::InvalidEntity(error.to_string()))?;
                    text.push_str(&decoded);
                }
                RawNode::CData(bytes) => {
                    text.push_str(
                        core::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?,
                    );
                }
                _ => {}
            }
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text,
            verbatim: Some(element.children.clone()),
        })
    }
}

impl HeaderFooterText {
    /// Builds a new header or footer string element named `local`, bound to `prefix` or to the
    /// default namespace, holding `text`.
    ///
    /// `local` must be one of `CT_HeaderFooter`'s six child names; nothing checks it, because the
    /// six constructors below are how a caller reaches this and each supplies its own.
    fn new(interner: &mut Interner, prefix: Option<&str>, local: &str, text: &str) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, local),
            attributes: Vec::new(),
            empty: text.is_empty(),
            text: text.to_owned(),
            verbatim: None,
        }
    }

    /// Builds the element `slot` names, bound to `prefix` or to the default namespace, holding
    /// `text`.
    ///
    /// The six named constructors below are this one with the slot supplied; this is what a caller
    /// that already has a [`HeaderFooterSlot`] in hand uses.
    #[must_use]
    pub fn named(
        interner: &mut Interner,
        prefix: Option<&str>,
        slot: HeaderFooterSlot,
        text: &str,
    ) -> Self {
        Self::new(interner, prefix, slot.wire_local(), text)
    }

    /// Builds an `x:oddHeader` holding `text`.
    #[must_use]
    pub fn odd_header(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "oddHeader", text)
    }

    /// Builds an `x:oddFooter` holding `text`.
    #[must_use]
    pub fn odd_footer(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "oddFooter", text)
    }

    /// Builds an `x:evenHeader` holding `text`.
    #[must_use]
    pub fn even_header(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "evenHeader", text)
    }

    /// Builds an `x:evenFooter` holding `text`.
    #[must_use]
    pub fn even_footer(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "evenFooter", text)
    }

    /// Builds an `x:firstHeader` holding `text`.
    #[must_use]
    pub fn first_header(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "firstHeader", text)
    }

    /// Builds an `x:firstFooter` holding `text`.
    #[must_use]
    pub fn first_footer(interner: &mut Interner, prefix: Option<&str>, text: &str) -> Self {
        Self::new(interner, prefix, "firstFooter", text)
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// The header or footer string, with entity references decoded and **nothing else changed**.
    ///
    /// Every formatting code is still in it, in the file's own order and spelling: `&&` is still two
    /// characters, `&"Arial,Bold"` still carries its quotes, and a code this library has never heard
    /// of is still there.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the whole string.
    ///
    /// **The only mutator, and deliberately the only one.** There is no `set_section`, because
    /// writing one section back means re-emitting the other two, which is the re-serialisation this
    /// type exists to prevent. A caller that wants to change one section reads
    /// [`section_runs`](Self::section_runs), builds the string it wants, and states it here.
    ///
    /// This is the point at which the preserved character data is given up; the element's name, its
    /// attributes, their order and their quoting are untouched.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.verbatim = None;
        self.empty = false;
    }

    /// Every run of text belonging to `section`, in the order the string lists them.
    ///
    /// **An iterator of runs, not one joined string.** ECMA-376 Part 1 §18.3.1.46 says an
    /// implementation *"can concatenate"* several specifiers for the same section — permitted, not
    /// required — so joining them here would be asserting an equivalence the spec leaves open. The
    /// example the spec itself gives is `&L A &C D &R G &L B &C E &R H`, whose left section is two
    /// runs.
    ///
    /// Each run is a slice of [`text`](Self::text) and runs from just after its `&L`/`&C`/`&R` code
    /// to just before the next section code — so it still holds every *other* formatting code
    /// (`&P`, `&"Arial,Bold"`, `&G`) exactly as the file wrote them. Reading is all this does.
    pub fn section_runs(&self, section: HeaderFooterSection) -> impl Iterator<Item = &str> + '_ {
        split_sections(&self.text)
            .filter(move |(run, _)| *run == Some(section))
            .map(|(_, slice)| slice)
    }

    /// The text before the string's first section code, which belongs to no section.
    ///
    /// Usually empty. ECMA-376 Part 1 §18.3.1.46 requires that *"each section specifier shall begin
    /// with a formatting code that indicates whether it is a left-, center-, or right-section
    /// specifier"*, so a string that starts with anything else is outside what the spec describes.
    /// It is reported rather than guessed at, and above all rather than assigned to a section.
    #[must_use]
    pub fn unsectioned_text(&self) -> &str {
        split_sections(&self.text)
            .next()
            .filter(|(section, _)| section.is_none())
            .map_or("", |(_, slice)| slice)
    }

    /// Whether the string carries a `&G` — the code that draws the image in the sheet's `drawingHF`
    /// part.
    ///
    /// Reported because it is the one formatting code that reaches **another part**: a `&G` with no
    /// `drawingHF` beside it draws nothing. `drawingHF` itself is unmodelled and belongs to no work
    /// item; see [`crate::worksheet`].
    ///
    /// A `&&G` is a literal ampersand followed by a `G` and is **not** a drawing reference, which is
    /// why this is a scan rather than a `contains("&G")`.
    #[must_use]
    pub fn contains_drawing_reference(&self) -> bool {
        contains_code(&self.text, 'G')
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    ///
    /// An untouched value replays the children the file held — entity spellings, CDATA sections and
    /// any comment between them included. One that [`set_text`](Self::set_text) has reached writes a
    /// single freshly escaped text node, which is what authoring should write.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = match &self.verbatim {
            Some(children) => children.clone(),
            None if self.text.is_empty() => Vec::new(),
            None => vec![RawNode::Text(
                mjx_xml::text::escape_text(&self.text).as_bytes().into(),
            )],
        };
        let empty = self.empty && children.is_empty();
        RawElement::rebuilt(self.name, self.attributes.clone(), children, empty)
    }
}

impl ToXml for HeaderFooterText {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// Walks `text` once, yielding `(section, run)` pairs in document order.
///
/// The first pair carries `None` when the string begins with text rather than with a section code;
/// it is skipped entirely when the string begins with one, so a well-formed string yields only
/// `Some(..)` pairs.
///
/// # Why this is a scanner and not three `find` calls
///
/// `&` is escaped as `&&` (§18.3.1.46: *"the character `&`"*), and a font name is quoted
/// (`&"Arial,Bold"`). So `&&L` is a literal ampersand followed by the letter `L`, and a `&L` inside
/// a quoted font name is part of the name. Both are found by `str::find("&L")` and neither is a
/// section code, so the scan consumes `&&` as one token and skips a quoted run wholesale.
fn split_sections(text: &str) -> impl Iterator<Item = (Option<HeaderFooterSection>, &str)> + '_ {
    let mut section = None;
    let mut start = 0usize;
    let mut cursor = 0usize;
    let bytes = text.as_bytes();
    core::iter::from_fn(move || {
        while cursor < bytes.len() {
            if bytes[cursor] != b'&' {
                cursor += 1;
                continue;
            }
            match bytes.get(cursor + 1) {
                // `&&` is a literal ampersand: two bytes of ordinary text.
                Some(b'&') => cursor += 2,
                // A quoted font name runs to its closing quote, wherever that is; an unterminated
                // one runs to the end of the string, which is the file's defect and not a reason to
                // start finding section codes inside it.
                Some(b'"') => {
                    cursor = bytes[cursor + 2..]
                        .iter()
                        .position(|byte| *byte == b'"')
                        .map_or(bytes.len(), |at| cursor + 3 + at);
                }
                Some(code) => {
                    let found = match code {
                        b'L' => Some(HeaderFooterSection::Left),
                        b'C' => Some(HeaderFooterSection::Center),
                        b'R' => Some(HeaderFooterSection::Right),
                        _ => None,
                    };
                    let Some(found) = found else {
                        // Every other code is two bytes of the current run — `&P`, `&D`, `&G`, and
                        // anything this library has never heard of.
                        cursor += 2;
                        continue;
                    };
                    let run = &text[start..cursor];
                    let previous = section;
                    section = Some(found);
                    cursor += 2;
                    start = cursor;
                    if previous.is_none() && run.is_empty() {
                        // The string opened with a section code: there is no unsectioned run to
                        // report, so keep scanning rather than yielding an empty one.
                        continue;
                    }
                    return Some((previous, run));
                }
                // A trailing `&` with nothing after it.
                None => cursor += 1,
            }
        }
        if start > text.len() {
            return None;
        }
        let run = &text[start..];
        start = text.len() + 1;
        (section.is_some() || !run.is_empty()).then_some((section, run))
    })
}

/// Whether `text` carries the formatting code `&<code>`, counting `&&` as a literal ampersand and
/// skipping quoted font names.
fn contains_code(text: &str, code: char) -> bool {
    let bytes = text.as_bytes();
    let wanted = u8::try_from(code).unwrap_or(0);
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'&' {
            cursor += 1;
            continue;
        }
        match bytes.get(cursor + 1) {
            Some(b'&') => cursor += 2,
            Some(b'"') => {
                cursor = bytes[cursor + 2..]
                    .iter()
                    .position(|byte| *byte == b'"')
                    .map_or(bytes.len(), |at| cursor + 3 + at);
            }
            Some(found) if *found == wanted => return true,
            Some(_) => cursor += 2,
            None => cursor += 1,
        }
    }
    false
}

/// `x:headerFooter` (`CT_HeaderFooter`, `sml.xsd:2857`) — up to six header and footer strings and
/// the four flags that say which pages use which.
///
/// **`ST_`/`CT_` symbol:** `CT_HeaderFooter`. Wire element: `headerFooter`, rank **22** of
/// `CT_Worksheet`.
///
/// # Six strings, three page classes
///
/// `@differentOddEven` and `@differentFirst` decide how many of the six are read. With both `false`,
/// `oddHeader`/`oddFooter` are the header and footer of **every** page and the other four are
/// ignored — which is why a file routinely carries all six with only two in use. Nothing here
/// removes an unused one: it is what the user typed and it comes back the moment the flag is set
/// again.
///
/// ECMA-376 Part 1 §18.3.1.46 adds the rule that catches a reader out: *"In the latter case, the
/// first page is not considered an odd page."*
///
/// `@scaleWithDoc` and `@alignWithMargins` both default to `true`, so a bare `<headerFooter/>` is
/// legal markup and means something.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "differentOddEven", codec = OnOff, accessor = odd_and_even_pages_differ, default = false))]
#[xml(attribute(local = "differentFirst", codec = OnOff, accessor = first_page_differs, default = false))]
#[xml(attribute(local = "scaleWithDoc", codec = OnOff, accessor = scales_with_document, default = true))]
#[xml(attribute(local = "alignWithMargins", codec = OnOff, accessor = aligns_with_page_margins, default = true))]
pub struct HeaderFooter {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "oddHeader", variant = OddHeader, ty = HeaderFooterText),
        child(local = "oddFooter", variant = OddFooter, ty = HeaderFooterText),
        child(local = "evenHeader", variant = EvenHeader, ty = HeaderFooterText),
        child(local = "evenFooter", variant = EvenFooter, ty = HeaderFooterText),
        child(local = "firstHeader", variant = FirstHeader, ty = HeaderFooterText),
        child(local = "firstFooter", variant = FirstFooter, ty = HeaderFooterText)
    )]
    content: Vec<HeaderFooterContent>,
}

/// One child of [`HeaderFooter`] — the six strings, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderFooterContent {
    /// `x:oddHeader` (rank 0) — and, with `@differentOddEven` unset, *every* page's header.
    OddHeader(HeaderFooterText),
    /// `x:oddFooter` (rank 1).
    OddFooter(HeaderFooterText),
    /// `x:evenHeader` (rank 2) — read only when `@differentOddEven` is set.
    EvenHeader(HeaderFooterText),
    /// `x:evenFooter` (rank 3).
    EvenFooter(HeaderFooterText),
    /// `x:firstHeader` (rank 4) — read only when `@differentFirst` is set.
    FirstHeader(HeaderFooterText),
    /// `x:firstFooter` (rank 5).
    FirstFooter(HeaderFooterText),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

/// Which of `CT_HeaderFooter`'s six string slots one value fills.
///
/// A [`HeaderFooterText`] on its own does not know: all six are the same complex type, and only the
/// element name distinguishes them — the shape [`BreakAxis`](crate::BreakAxis) has for
/// `CT_PageBreak`'s two slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderFooterSlot {
    /// `x:oddHeader` (rank 0).
    OddHeader,
    /// `x:oddFooter` (rank 1).
    OddFooter,
    /// `x:evenHeader` (rank 2).
    EvenHeader,
    /// `x:evenFooter` (rank 3).
    EvenFooter,
    /// `x:firstHeader` (rank 4).
    FirstHeader,
    /// `x:firstFooter` (rank 5).
    FirstFooter,
}

impl HeaderFooterSlot {
    /// All six, in `CT_HeaderFooter`'s own `xsd:sequence` order.
    pub const ALL: [Self; 6] = [
        Self::OddHeader,
        Self::OddFooter,
        Self::EvenHeader,
        Self::EvenFooter,
        Self::FirstHeader,
        Self::FirstFooter,
    ];

    /// The wire local name of the element this slot names.
    #[must_use]
    pub fn wire_local(self) -> &'static str {
        match self {
            Self::OddHeader => "oddHeader",
            Self::OddFooter => "oddFooter",
            Self::EvenHeader => "evenHeader",
            Self::EvenFooter => "evenFooter",
            Self::FirstHeader => "firstHeader",
            Self::FirstFooter => "firstFooter",
        }
    }
}

impl HeaderFooterContent {
    /// Which slot this child fills, or `None` for an unmodelled node.
    #[must_use]
    fn slot(&self) -> Option<HeaderFooterSlot> {
        Some(match self {
            Self::OddHeader(_) => HeaderFooterSlot::OddHeader,
            Self::OddFooter(_) => HeaderFooterSlot::OddFooter,
            Self::EvenHeader(_) => HeaderFooterSlot::EvenHeader,
            Self::EvenFooter(_) => HeaderFooterSlot::EvenFooter,
            Self::FirstHeader(_) => HeaderFooterSlot::FirstHeader,
            Self::FirstFooter(_) => HeaderFooterSlot::FirstFooter,
            Self::Raw(_) => return None,
        })
    }

    /// The string this child holds, or `None` for an unmodelled node.
    #[must_use]
    fn value(&self) -> Option<&HeaderFooterText> {
        match self {
            Self::OddHeader(value)
            | Self::OddFooter(value)
            | Self::EvenHeader(value)
            | Self::EvenFooter(value)
            | Self::FirstHeader(value)
            | Self::FirstFooter(value) => Some(value),
            Self::Raw(_) => None,
        }
    }

    /// The string this child holds, mutably.
    #[must_use]
    fn value_mut(&mut self) -> Option<&mut HeaderFooterText> {
        match self {
            Self::OddHeader(value)
            | Self::OddFooter(value)
            | Self::EvenHeader(value)
            | Self::EvenFooter(value)
            | Self::FirstHeader(value)
            | Self::FirstFooter(value) => Some(value),
            Self::Raw(_) => None,
        }
    }

    /// This child rebuilt as a node.
    ///
    /// One `match` over all seven variants rather than a `value()`/`Raw` pair, so the "every
    /// non-`Raw` child holds a string" step is made by the compiler instead of by a `panic!`.
    #[must_use]
    fn as_raw_node(&self) -> RawNode {
        match self {
            Self::OddHeader(value)
            | Self::OddFooter(value)
            | Self::EvenHeader(value)
            | Self::EvenFooter(value)
            | Self::FirstHeader(value)
            | Self::FirstFooter(value) => RawNode::Element(value.as_raw_element()),
            Self::Raw(node) => node.clone(),
        }
    }

    /// Wraps `value` in the variant `slot` names.
    #[must_use]
    fn wrap(slot: HeaderFooterSlot, value: HeaderFooterText) -> Self {
        match slot {
            HeaderFooterSlot::OddHeader => Self::OddHeader(value),
            HeaderFooterSlot::OddFooter => Self::OddFooter(value),
            HeaderFooterSlot::EvenHeader => Self::EvenHeader(value),
            HeaderFooterSlot::EvenFooter => Self::EvenFooter(value),
            HeaderFooterSlot::FirstHeader => Self::FirstHeader(value),
            HeaderFooterSlot::FirstFooter => Self::FirstFooter(value),
        }
    }
}

impl HeaderFooter {
    /// Builds an empty `x:headerFooter`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "headerFooter"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[HeaderFooterContent] {
        &self.content
    }

    /// The string in `slot`, or `None` when the element does not write it.
    #[must_use]
    pub fn string(&self, slot: HeaderFooterSlot) -> Option<&HeaderFooterText> {
        self.content
            .iter()
            .find(|child| child.slot() == Some(slot))
            .and_then(HeaderFooterContent::value)
    }

    /// The string in `slot`, mutably — the door through which
    /// [`set_text`](HeaderFooterText::set_text) is reached.
    pub fn string_mut(&mut self, slot: HeaderFooterSlot) -> Option<&mut HeaderFooterText> {
        self.content
            .iter_mut()
            .find(|child| child.slot() == Some(slot))
            .and_then(HeaderFooterContent::value_mut)
    }

    /// Sets the string in `slot`: `None` removes it, `Some(value)` replaces the existing element
    /// **where it is** or inserts a new one at its rank in `CT_HeaderFooter`'s `xsd:sequence`.
    ///
    /// The rank comes from [`mjx_ooxml_types::child_order::HEADER_FOOTER`], never from a list
    /// written here.
    pub fn set_string(&mut self, slot: HeaderFooterSlot, value: Option<HeaderFooterText>) {
        let existing = self
            .content
            .iter()
            .position(|child| child.slot() == Some(slot));
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = HeaderFooterContent::wrap(slot, value),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = mjx_ooxml_types::child_order::HEADER_FOOTER.insert_index_of_names(
                    self.content.iter().map(|child| {
                        child.slot().and_then(|slot| {
                            mjx_ooxml_types::child_order::HEADER_FOOTER
                                .rank_of(None, slot.wire_local())
                        })
                    }),
                    slot.wire_local(),
                );
                self.content
                    .insert(at, HeaderFooterContent::wrap(slot, value));
                self.empty = false;
            }
            (None, None) => {}
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(HeaderFooterContent::as_raw_node)
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for HeaderFooter {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
