//! Word-specific value classes: page geometry, effective-property summaries, fields, comments,
//! notes and revisions — the TypeScript sibling of `mjx-python`'s own `word.rs`, whose module doc
//! (in particular the "curated subset, not the whole ladder" reasoning for `EffectiveCharacter/
//! ParagraphProperties`) is authoritative for this file too.

use wasm_bindgen::prelude::*;

use mjx_ooxml as ooxml;

use crate::enums::{FieldForm, Justification, PageOrientation, RevisionKind};

value_class! {
    /// A page's extent, orientation and (optional) paper-size code, in twips (1/1440 inch).
    PageSize(ooxml::PageSize), derive(Copy, PartialEq, Eq);

    /// A section's page margins, in twips.
    PageMargins(ooxml::PageMargins), derive(Copy, PartialEq, Eq);

    /// A resolved colour: `"auto"`, or a concrete `RRGGBB` hex value.
    EffectiveColor(ooxml::EffectiveColor), derive(PartialEq, Eq);

    /// A resolved font reference, per script slot.
    EffectiveFonts(ooxml::EffectiveFonts), derive(PartialEq, Eq);

    /// The curated subset of a run's effective character formatting.
    EffectiveCharacterProperties(ooxml::EffectiveCharacterProperties), derive(PartialEq);

    /// The curated subset of a paragraph's effective layout.
    EffectiveParagraphProperties(ooxml::EffectiveParagraphProperties), derive(PartialEq);

    /// A resolved cell/table shading.
    EffectiveShading(ooxml::EffectiveShading), derive(PartialEq, Eq);

    /// A resolved cell/table border edge.
    EffectiveBorder(ooxml::EffectiveBorder), derive(PartialEq, Eq);

    /// One section: the paragraphs it governs, and its own page geometry.
    SectionSummary(ooxml::SectionSummary), derive(PartialEq, Eq);

    /// One comment: its id, author, initials and text.
    CommentSummary(ooxml::CommentSummary), derive(PartialEq, Eq);

    /// One user-visible footnote or endnote: its id and text.
    NoteSummary(ooxml::NoteSummary), derive(PartialEq, Eq);

    /// One tracked-change marker: its kind, author, date and id.
    RevisionInfo(ooxml::RevisionInfo), derive(PartialEq, Eq);

    /// A hyperlink's click target: an external URL, or an in-document bookmark anchor.
    HyperlinkTarget(ooxml::HyperlinkTarget), derive(PartialEq, Eq);

    /// One field: its form, instruction, cached result and any fields nested inside it.
    Field(ooxml::Field), derive(PartialEq);

    /// One way a table's grid and its rows disagree with each other.
    GridDiscrepancy(ooxml::GridDiscrepancy), derive(PartialEq, Eq);
}

#[wasm_bindgen]
impl PageSize {
    /// ISO 216 A4, portrait: 210 x 297 mm.
    #[wasm_bindgen(js_name = "a4")]
    pub fn a4() -> Self {
        Self(ooxml::PageSize::a4())
    }

    /// US Letter, portrait: 8.5 x 11 in.
    #[wasm_bindgen(js_name = "usLetter")]
    pub fn us_letter() -> Self {
        Self(ooxml::PageSize::us_letter())
    }

    /// An arbitrary page extent, in twips, with the given orientation.
    #[wasm_bindgen(js_name = "fromTwips")]
    pub fn from_twips(width_twips: u32, height_twips: u32, orientation: PageOrientation) -> Self {
        Self(ooxml::PageSize::from_twips(
            width_twips,
            height_twips,
            orientation.into(),
        ))
    }

    /// The same physical page, rotated: width and height swapped, orientation set to landscape.
    #[wasm_bindgen(js_name = "landscape")]
    pub fn landscape(&self) -> Self {
        Self(self.0.landscape())
    }

    /// The page width, in twips — the larger dimension when landscape.
    #[wasm_bindgen(getter, js_name = "widthTwips")]
    pub fn width_twips(&self) -> u32 {
        self.0.width_twips
    }

    /// The page height, in twips.
    #[wasm_bindgen(getter, js_name = "heightTwips")]
    pub fn height_twips(&self) -> u32 {
        self.0.height_twips
    }

    /// The page's stated orientation.
    #[wasm_bindgen(getter, js_name = "orientation")]
    pub fn orientation(&self) -> Result<PageOrientation, JsValue> {
        PageOrientation::from_model(self.0.orientation)
    }
}

#[wasm_bindgen]
impl PageMargins {
    /// Word's "Normal" template margins: 1 inch on every side, half an inch header/footer, no
    /// gutter.
    #[wasm_bindgen(js_name = "normal")]
    pub fn normal() -> Self {
        Self(ooxml::PageMargins::NORMAL)
    }

    /// The top margin, in twips — signed, so a negative value overlaps the header.
    #[wasm_bindgen(getter, js_name = "top")]
    pub fn top(&self) -> i32 {
        self.0.top
    }

    /// The right margin, in twips.
    #[wasm_bindgen(getter, js_name = "right")]
    pub fn right(&self) -> u32 {
        self.0.right
    }

    /// The bottom margin, in twips — signed, so a negative value overlaps the footer.
    #[wasm_bindgen(getter, js_name = "bottom")]
    pub fn bottom(&self) -> i32 {
        self.0.bottom
    }

    /// The left margin, in twips.
    #[wasm_bindgen(getter, js_name = "left")]
    pub fn left(&self) -> u32 {
        self.0.left
    }

    /// The header's distance from the page's top edge, in twips.
    #[wasm_bindgen(getter, js_name = "header")]
    pub fn header(&self) -> u32 {
        self.0.header
    }

    /// The footer's distance from the page's bottom edge, in twips.
    #[wasm_bindgen(getter, js_name = "footer")]
    pub fn footer(&self) -> u32 {
        self.0.footer
    }

    /// Extra binding-side space added to the left margin, in twips.
    #[wasm_bindgen(getter, js_name = "gutter")]
    pub fn gutter(&self) -> u32 {
        self.0.gutter
    }
}

#[wasm_bindgen]
impl EffectiveColor {
    /// Whether the document leaves this colour to the renderer.
    #[wasm_bindgen(getter, js_name = "isAuto")]
    pub fn is_auto(&self) -> bool {
        matches!(self.0, ooxml::EffectiveColor::Auto)
    }

    /// The concrete `RRGGBB` hex value, uppercase, when this is not `auto`.
    #[wasm_bindgen(getter, js_name = "hex")]
    pub fn hex(&self) -> Option<String> {
        match &self.0 {
            ooxml::EffectiveColor::Hex(hex) => Some(hex.clone()),
            ooxml::EffectiveColor::Auto => None,
        }
    }
}

#[wasm_bindgen]
impl EffectiveFonts {
    /// The Latin/ASCII-range typeface.
    #[wasm_bindgen(getter, js_name = "ascii")]
    pub fn ascii(&self) -> Option<String> {
        self.0.ascii.clone()
    }

    /// The Latin "High ANSI" typeface.
    #[wasm_bindgen(getter, js_name = "highAnsi")]
    pub fn high_ansi(&self) -> Option<String> {
        self.0.high_ansi.clone()
    }

    /// The East Asian typeface.
    #[wasm_bindgen(getter, js_name = "eastAsian")]
    pub fn east_asian(&self) -> Option<String> {
        self.0.east_asian.clone()
    }

    /// The complex-script typeface.
    #[wasm_bindgen(getter, js_name = "complexScript")]
    pub fn complex_script(&self) -> Option<String> {
        self.0.complex_script.clone()
    }
}

#[wasm_bindgen]
impl EffectiveCharacterProperties {
    /// Bold, resolved.
    #[wasm_bindgen(getter, js_name = "bold")]
    pub fn bold(&self) -> Option<bool> {
        self.0.bold
    }

    /// Italic, resolved.
    #[wasm_bindgen(getter, js_name = "italic")]
    pub fn italic(&self) -> Option<bool> {
        self.0.italic
    }

    /// Single strikethrough, resolved.
    #[wasm_bindgen(getter, js_name = "strikethrough")]
    pub fn strikethrough(&self) -> Option<bool> {
        self.0.strikethrough
    }

    /// Hidden text, resolved.
    #[wasm_bindgen(getter, js_name = "hidden")]
    pub fn hidden(&self) -> Option<bool> {
        self.0.hidden
    }

    /// All capitals, resolved.
    #[wasm_bindgen(getter, js_name = "allCapitals")]
    pub fn all_capitals(&self) -> Option<bool> {
        self.0.all_capitals
    }

    /// Small capitals, resolved.
    #[wasm_bindgen(getter, js_name = "smallCaps")]
    pub fn small_caps(&self) -> Option<bool> {
        self.0.small_caps
    }

    /// The font size, in half-points, as the raw wire string.
    #[wasm_bindgen(getter, js_name = "fontSizeHalfPoints")]
    pub fn font_size_half_points(&self) -> Option<String> {
        self.0
            .font_size
            .as_ref()
            .map(|value| value.to_wire().to_owned())
    }

    /// The resolved colour, its theme reference already baked to concrete `RRGGBB`.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> Option<EffectiveColor> {
        self.0.color.clone().map(EffectiveColor)
    }

    /// The resolved font reference, per script slot.
    #[wasm_bindgen(getter, js_name = "fonts")]
    pub fn fonts(&self) -> Option<EffectiveFonts> {
        self.0.fonts.clone().map(EffectiveFonts)
    }
}

#[wasm_bindgen]
impl EffectiveParagraphProperties {
    /// `w:keepNext`, resolved.
    #[wasm_bindgen(getter, js_name = "keepWithNext")]
    pub fn keep_with_next(&self) -> Option<bool> {
        self.0.keep_with_next
    }

    /// `w:keepLines`, resolved.
    #[wasm_bindgen(getter, js_name = "keepLinesTogether")]
    pub fn keep_lines_together(&self) -> Option<bool> {
        self.0.keep_lines_together
    }

    /// `w:pageBreakBefore`, resolved.
    #[wasm_bindgen(getter, js_name = "pageBreakBefore")]
    pub fn page_break_before(&self) -> Option<bool> {
        self.0.page_break_before
    }

    /// `w:widowControl`, resolved.
    #[wasm_bindgen(getter, js_name = "widowControl")]
    pub fn widow_control(&self) -> Option<bool> {
        self.0.widow_control
    }

    /// The paragraph's resolved alignment.
    #[wasm_bindgen(getter, js_name = "alignment")]
    pub fn alignment(&self) -> Result<Option<Justification>, JsValue> {
        match self.0.alignment {
            Some(value) => Justification::from_model(value).map(Some),
            None => Ok(None),
        }
    }

    /// The resolved outline level, `0`-based; absent for body text.
    #[wasm_bindgen(getter, js_name = "outlineLevel")]
    pub fn outline_level(&self) -> Option<f64> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an outline level is a tiny number; JavaScript has one numeric type"
        )]
        self.0.outline_level.map(|value| value as f64)
    }
}

#[wasm_bindgen]
impl EffectiveShading {
    /// The shading pattern's own colour.
    #[wasm_bindgen(getter, js_name = "patternColor")]
    pub fn pattern_color(&self) -> Option<EffectiveColor> {
        self.0.pattern_color.clone().map(EffectiveColor)
    }

    /// The background colour the pattern draws over.
    #[wasm_bindgen(getter, js_name = "fill")]
    pub fn fill(&self) -> Option<EffectiveColor> {
        self.0.fill.clone().map(EffectiveColor)
    }
}

#[wasm_bindgen]
impl EffectiveBorder {
    /// The border's resolved colour.
    #[wasm_bindgen(getter, js_name = "color")]
    pub fn color(&self) -> EffectiveColor {
        EffectiveColor(self.0.color.clone())
    }

    /// The border's width, in eighths of a point, if stated.
    #[wasm_bindgen(getter, js_name = "widthEighthsOfAPoint")]
    pub fn width_eighths_of_a_point(&self) -> Option<u32> {
        self.0
            .width_eighths_of_a_point
            .and_then(|value| u32::try_from(value).ok())
    }
}

#[wasm_bindgen]
impl SectionSummary {
    /// The first paragraph index this section governs.
    #[wasm_bindgen(getter, js_name = "firstParagraph")]
    pub fn first_paragraph(&self) -> u32 {
        self.0.first_paragraph
    }

    /// The last paragraph index this section governs, inclusive — `undefined` if it governs no
    /// paragraph at all.
    #[wasm_bindgen(getter, js_name = "lastParagraph")]
    pub fn last_paragraph(&self) -> Option<u32> {
        self.0.last_paragraph
    }

    /// This section's page size, if it states one.
    #[wasm_bindgen(getter, js_name = "pageSize")]
    pub fn page_size(&self) -> Option<PageSize> {
        self.0.page_size.map(PageSize)
    }

    /// This section's page margins, if it states one.
    #[wasm_bindgen(getter, js_name = "pageMargins")]
    pub fn page_margins(&self) -> Option<PageMargins> {
        self.0.page_margins.map(PageMargins)
    }
}

#[wasm_bindgen]
impl CommentSummary {
    /// The comment's own id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a comment id is far below 2^53; JavaScript has one numeric type"
        )]
        {
            self.0.id as f64
        }
    }

    /// The comment's author.
    #[wasm_bindgen(getter, js_name = "author")]
    pub fn author(&self) -> String {
        self.0.author.clone()
    }

    /// The comment author's initials, if stated.
    #[wasm_bindgen(getter, js_name = "initials")]
    pub fn initials(&self) -> Option<String> {
        self.0.initials.clone()
    }

    /// The comment's own text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.0.text.clone()
    }
}

#[wasm_bindgen]
impl NoteSummary {
    /// The note's own id.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "a note id is far below 2^53")]
        {
            self.0.id as f64
        }
    }

    /// The note's own text.
    #[wasm_bindgen(getter, js_name = "text")]
    pub fn text(&self) -> String {
        self.0.text.clone()
    }
}

#[wasm_bindgen]
impl RevisionInfo {
    /// What kind of tracked change this is.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> Result<RevisionKind, JsValue> {
        RevisionKind::from_model(self.0.kind)
    }

    /// The author, if stated and well-formed.
    #[wasm_bindgen(getter, js_name = "author")]
    pub fn author(&self) -> Option<String> {
        self.0.author.clone()
    }

    /// The date/time stamp, the file's own wire string, unparsed.
    #[wasm_bindgen(getter, js_name = "date")]
    pub fn date(&self) -> Option<String> {
        self.0.date.clone()
    }

    /// The revision's own id, if stated and well-formed.
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn id(&self) -> Option<f64> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a revision id is far below 2^53"
        )]
        self.0.id.map(|value| value as f64)
    }
}

#[wasm_bindgen]
impl HyperlinkTarget {
    /// An external URL target.
    #[wasm_bindgen(js_name = "url")]
    pub fn url(url: String) -> Self {
        Self(ooxml::HyperlinkTarget::Url(url))
    }

    /// An in-document bookmark anchor target.
    #[wasm_bindgen(js_name = "anchor")]
    pub fn anchor(name: String) -> Self {
        Self(ooxml::HyperlinkTarget::Anchor(name))
    }

    /// Whether this is an external URL target.
    #[wasm_bindgen(getter, js_name = "isUrl")]
    pub fn is_url(&self) -> bool {
        matches!(self.0, ooxml::HyperlinkTarget::Url(_))
    }

    /// The URL, when this is a `url` target.
    #[wasm_bindgen(getter, js_name = "urlValue")]
    pub fn url_value(&self) -> Option<String> {
        match &self.0 {
            ooxml::HyperlinkTarget::Url(url) => Some(url.clone()),
            ooxml::HyperlinkTarget::Anchor(_) => None,
        }
    }

    /// The bookmark name, when this is an `anchor` target.
    #[wasm_bindgen(getter, js_name = "anchorValue")]
    pub fn anchor_value(&self) -> Option<String> {
        match &self.0 {
            ooxml::HyperlinkTarget::Anchor(name) => Some(name.clone()),
            ooxml::HyperlinkTarget::Url(_) => None,
        }
    }
}

#[wasm_bindgen]
impl Field {
    /// Which wire form this field was read from.
    #[wasm_bindgen(getter, js_name = "form")]
    pub fn form(&self) -> Result<FieldForm, JsValue> {
        FieldForm::from_model(self.0.form())
    }

    /// The field's instruction, verbatim, excluding any nested field's own instruction.
    #[wasm_bindgen(getter, js_name = "instruction")]
    pub fn instruction(&self) -> String {
        self.0.instruction().to_owned()
    }

    /// The instruction's own field-type keyword, if recognizable.
    #[wasm_bindgen(getter, js_name = "fieldName")]
    pub fn field_name(&self) -> Option<String> {
        self.0.field_name().map(str::to_owned)
    }

    /// The instruction with the field-type keyword removed, verbatim.
    #[wasm_bindgen(getter, js_name = "arguments")]
    pub fn arguments(&self) -> String {
        self.0.arguments().to_owned()
    }

    /// The field's cached result, excluding any nested field's own result.
    #[wasm_bindgen(getter, js_name = "cachedResult")]
    pub fn cached_result(&self) -> Option<String> {
        self.0.cached_result().map(str::to_owned)
    }

    /// Every field nested inside this one, in document order.
    #[wasm_bindgen(getter, js_name = "nestedFields")]
    pub fn nested_fields(&self) -> Vec<Field> {
        self.0.nested_fields().iter().cloned().map(Field).collect()
    }
}

#[wasm_bindgen]
impl GridDiscrepancy {
    /// Which kind of discrepancy this is: `"RowWidthMismatch"`, `"OrphanedVerticalMerge"` or
    /// `"EmptyRow"`.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch { .. } => "RowWidthMismatch",
            ooxml::GridDiscrepancy::OrphanedVerticalMerge { .. } => "OrphanedVerticalMerge",
            ooxml::GridDiscrepancy::EmptyRow { .. } => "EmptyRow",
            _ => "Unknown",
        }
        .to_owned()
    }

    /// The row involved, for every kind.
    #[wasm_bindgen(getter, js_name = "row")]
    pub fn row(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch { row, .. }
            | ooxml::GridDiscrepancy::OrphanedVerticalMerge { row, .. }
            | ooxml::GridDiscrepancy::EmptyRow { row } => u32::try_from(row).ok(),
            _ => None,
        }
    }

    /// The grid column involved, for `OrphanedVerticalMerge` only.
    #[wasm_bindgen(getter, js_name = "column")]
    pub fn column(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::OrphanedVerticalMerge { column, .. } => {
                u32::try_from(column).ok()
            }
            _ => None,
        }
    }

    /// The grid's declared column count, for `RowWidthMismatch` only.
    #[wasm_bindgen(getter, js_name = "declaredColumns")]
    pub fn declared_columns(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch {
                declared_columns, ..
            } => u32::try_from(declared_columns).ok(),
            _ => None,
        }
    }

    /// What the row's cells actually sum to, for `RowWidthMismatch` only.
    #[wasm_bindgen(getter, js_name = "spannedColumns")]
    pub fn spanned_columns(&self) -> Option<u32> {
        match self.0 {
            ooxml::GridDiscrepancy::RowWidthMismatch {
                spanned_columns, ..
            } => u32::try_from(spanned_columns).ok(),
            _ => None,
        }
    }
}
