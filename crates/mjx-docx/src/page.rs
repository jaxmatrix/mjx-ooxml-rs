//! A document's page geometry (`w:sectPr/w:pgSz` and `w:pgMar`), in twips (1/1440 inch).
//!
//! # `ST_TwipsMeasure` has no numeric range — unlike PresentationML's `ST_SlideSizeCoordinate`
//!
//! `mjx_pptx::SlideSize` refuses an extent outside `914400..=51206400` because `p:sldSz`'s type,
//! `ST_SlideSizeCoordinate`, is an `xsd:restriction` of `a:ST_PositiveCoordinate32` with an explicit
//! `minInclusive`/`maxInclusive` (`pml.xsd`). `w:pgSz`'s `w`/`h` are typed `s:ST_TwipsMeasure`
//! instead (`wml.xsd`), and that type is a bare `xsd:union` of `ST_UnsignedDecimalNumber`
//! (`xsd:unsignedLong`) and `ST_PositiveUniversalMeasure` — **no `minInclusive`/`maxInclusive`
//! anywhere in its definition**. So "validated against `ST_TwipsMeasure`'s range" is not a claim the
//! schema itself supports: any non-negative integer is schema-legal for `w:pgSz@w`/`@h`, and
//! [`PageSize`]'s `u32` fields already rule out negative values by construction.
//!
//! What [`PageSize::validate`] enforces instead is a physically meaningful pair of conditions that
//! mirror the real trigger for Word's own "the margins are set outside the printable area of the
//! page" repair prompt: a page must have positive area, and this crate's fixed margins (see
//! [`PageMargins::NORMAL`]) must leave a positive printable area inside it. Both are refused with
//! [`crate::DocxError::InvalidPageSize`] before any markup is written, exactly as
//! `mjx_pptx::blank::validate` refuses a slide size `p:sldSz` cannot express.
//!
//! # `PageOrientation` is the generated type — not a second one
//!
//! MJXOFF-98 originally hand-wrote a two-variant `PageOrientation` enum here, duplicating
//! `mjx_ooxml_types::wordprocessingml::PageOrientation` (`ST_PageOrientation`) variant for variant.
//! That was a genuine "consume, do not re-create" defect (caught in MJXOFF-109's own pre-dispatch
//! review): the generated type already exists, already carries `Portrait`/`Landscape` with its own
//! `from_wire`/`to_wire`, and is what `w:pgSz@orient`'s codec (`Enumeration<PageOrientation>`,
//! `crates/mjx-docx/src/document/sections.rs`) reads and writes. This module now re-exports the
//! generated enum (see [`PageOrientation`]) instead of shadowing it. The one piece of real behaviour
//! the old duplicate carried — "omit the attribute entirely for `Portrait`, since that is the schema
//! default every fixture and every real Office file already omits it for" — was not schema-level
//! knowledge the generated enum should carry (a generated `ST_*` wrapper never encodes "and omit me
//! when the value is X"), so it now lives in [`orientation_wire_value`], a small helper, rather than
//! in a second enum.

use crate::error::DocxError;

/// `ST_PageOrientation` (`wml.xsd`) — `w:pgSz@orient`'s two wire values. Re-exported from
/// `mjx_ooxml_types::wordprocessingml` rather than restated here — see this module's own doc comment.
pub use mjx_ooxml_types::wordprocessingml::PageOrientation;

/// The wire token for `w:pgSz@orient`, or `None` for [`PageOrientation::Portrait`] — the schema
/// default every committed fixture and every real Office file omits the attribute for, restated as a
/// helper (not a method on the generated enum, which this crate does not own) so
/// `SectionProperties`'s writer and this module's own [`PageSize`] documentation agree on the one
/// place this rule is stated.
#[must_use]
pub(crate) fn orientation_wire_value(value: PageOrientation) -> Option<&'static str> {
    match value {
        PageOrientation::Portrait => None,
        PageOrientation::Landscape => Some("landscape"),
    }
}

/// A page's extent, orientation and (optional, legacy) paper-size code (`w:pgSz`), in twips.
///
/// The **caller-facing value** for a page size, shared between [`crate::Document::blank`] (which
/// only ever needs an extent and an orientation to write a fresh `word/document.xml`) and
/// [`crate::SectionProperties::page_size`]/`set_page_size` (which read and write this same type
/// against a real `w:pgSz` element, preserving whatever else that element carries — see that
/// method's own doc comment). Mirrors `mjx_pptx::SlideSize`'s own shape and role exactly: a
/// plain value struct, not a wire element — no `Interner` is needed to build one, which is what lets
/// [`PageSize::a4`]/[`PageSize::us_letter`] stay `const fn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    /// `w:pgSz@w`, in twips (1/1440 inch). The *page* width — for [`PageOrientation::Landscape`]
    /// this is the larger of the two physical dimensions, matching how Word itself swaps `w`/`h`
    /// rather than leaving them at their portrait values and relying on `@orient` alone.
    pub width_twips: u32,
    /// `w:pgSz@h`, in twips.
    pub height_twips: u32,
    /// `w:pgSz@orient`. [`PageOrientation::Portrait`] is written by omitting the attribute (its
    /// schema default), matching every committed fixture and real Office output — see
    /// `orientation_wire_value` (crate-private; this module's own doc comment explains the rule).
    pub orientation: PageOrientation,
}

impl PageSize {
    /// ISO 216 A4, portrait: 210 × 297 mm, `11906 × 16838` twips — confirmed against
    /// `tests/fixtures/sample.docx`'s own `w:pgSz`, LibreOffice's A4 default.
    #[must_use]
    pub const fn a4() -> Self {
        Self {
            width_twips: 11_906,
            height_twips: 16_838,
            orientation: PageOrientation::Portrait,
        }
    }

    /// US Letter, portrait: 8.5 × 11 in, `12240 × 15840` twips (`1440` twips per inch) — the other
    /// named default `w:pgSz` needs, matching Word's own "US Letter" preset.
    #[must_use]
    pub const fn us_letter() -> Self {
        Self {
            width_twips: 12_240,
            height_twips: 15_840,
            orientation: PageOrientation::Portrait,
        }
    }

    /// The same physical page, rotated: width and height swapped, orientation set to
    /// [`PageOrientation::Landscape`]. Idempotent is not offered — calling it twice swaps back to
    /// portrait-shaped dimensions while still claiming `Landscape`, so call it at most once per
    /// value, the same discipline `SlideSize`'s own constructors rely on callers to keep.
    #[must_use]
    pub const fn landscape(self) -> Self {
        Self {
            width_twips: self.height_twips,
            height_twips: self.width_twips,
            orientation: PageOrientation::Landscape,
        }
    }

    /// A page of an arbitrary extent, in twips, with the given orientation. No range check happens
    /// here — [`Document::blank`](crate::Document::blank) checks the result before writing anything
    /// — see this module's own doc comment for why `ST_TwipsMeasure` carries no schema range to
    /// check against, and what this crate checks instead.
    #[must_use]
    pub const fn from_twips(
        width_twips: u32,
        height_twips: u32,
        orientation: PageOrientation,
    ) -> Self {
        Self {
            width_twips,
            height_twips,
            orientation,
        }
    }

    /// Refuses a page this crate's fixed margins cannot fit inside, rather than emitting a
    /// `w:sectPr` Word repairs on open. See the [module docs](self) for why this — not a
    /// `ST_TwipsMeasure` numeric range — is the real constraint.
    ///
    /// # Errors
    /// Returns [`DocxError::InvalidPageSize`] if either extent is zero, or if
    /// [`PageMargins::NORMAL`]'s left+right (or top+bottom) margins leave no positive printable
    /// width (or height).
    pub(crate) fn validate(self) -> Result<(), DocxError> {
        let fail = |reason: &'static str| {
            Err(DocxError::InvalidPageSize {
                width_twips: self.width_twips,
                height_twips: self.height_twips,
                reason,
            })
        };
        if self.width_twips == 0 {
            return fail("page width is zero");
        }
        if self.height_twips == 0 {
            return fail("page height is zero");
        }
        let margins = PageMargins::NORMAL;
        let horizontal_margins = margins.left + margins.right;
        if horizontal_margins >= self.width_twips {
            return fail("the left and right margins leave no printable width");
        }
        // `top`/`bottom` are `i32` (`ST_SignedTwipsMeasure` permits a negative margin); `NORMAL` is
        // always non-negative, and a negative caller-unreachable value here would only shrink the
        // sum, which can only make this check *more* likely to reject rather than silently pass a
        // page it should not — `try_from` returning `0` for a negative input is therefore safe.
        let vertical_margins =
            u32::try_from(margins.top).unwrap_or(0) + u32::try_from(margins.bottom).unwrap_or(0);
        if vertical_margins >= self.height_twips {
            return fail("the top and bottom margins leave no printable height");
        }
        Ok(())
    }
}

/// `w:pgMar`'s seven attributes (`CT_PageMar`), all `use="required"` in `wml.xsd` — the one place in
/// a blank document's `word/document.xml` the schema genuinely requires something, which is why
/// `crate::blank`'s mutation gate proves this struct's fields, not `w:pgSz`'s (`CT_PageSz`'s `w`
/// and `h` are both `use="optional"`, contrary to what MJXOFF-98's ticket originally claimed — see
/// this crate's `CHANGELOG.md` entry for MJXOFF-98).
///
/// The **caller-facing value** for a section's page margins — mirrors [`PageSize`]'s own role and
/// shape exactly, shared between `crate::blank` (which only ever writes [`PageMargins::NORMAL`]) and
/// [`crate::SectionProperties::page_margins`]/`set_page_margins` (which read and write this same
/// type against a real `w:pgMar` element). `header`/`footer` are measured **from the
/// page edge**, not from the body text margin — confirmed directly against ECMA-376 Part 1 §17.6.11
/// ("`header` ... Specifies the distance ... from the top edge of the page to the top edge of the
/// header"; "`footer` ... Specifies the distance ... from the bottom edge of the page to the bottom
/// edge of the footer") — a common misreading this module's own field docs restate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMargins {
    /// `w:pgMar@top`, in twips. Signed (`ST_SignedTwipsMeasure`): a negative value lets the main
    /// document story overlap the header, measured from the page's top edge.
    pub top: i32,
    /// `w:pgMar@right`, in twips, from the page's right edge.
    pub right: u32,
    /// `w:pgMar@bottom`, in twips. Signed, the same way `top` is, for overlapping the footer.
    pub bottom: i32,
    /// `w:pgMar@left`, in twips, from the page's left edge.
    pub left: u32,
    /// `w:pgMar@header`, in twips, **from the top edge of the page** to the top edge of the header
    /// — not from the body text margin. See this type's own doc comment.
    pub header: u32,
    /// `w:pgMar@footer`, in twips, **from the bottom edge of the page** to the bottom edge of the
    /// footer — not from the body text margin. See this type's own doc comment.
    pub footer: u32,
    /// `w:pgMar@gutter`, in twips — extra space added to `left` (or, with `w:rtlGutter`/mirrored
    /// margins, the binding-side margin) for a document being bound.
    pub gutter: u32,
}

impl PageMargins {
    /// Word's "Normal" template margins: 1 inch on every side, ½ inch header/footer, no gutter.
    pub const NORMAL: Self = Self {
        top: 1440,
        right: 1440,
        bottom: 1440,
        left: 1440,
        header: 720,
        footer: 720,
        gutter: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_and_us_letter_are_portrait_by_default() {
        assert_eq!(PageSize::a4().orientation, PageOrientation::Portrait);
        assert_eq!(PageSize::us_letter().orientation, PageOrientation::Portrait);
        assert!(orientation_wire_value(PageOrientation::Portrait).is_none());
        assert_eq!(
            orientation_wire_value(PageOrientation::Landscape),
            Some("landscape")
        );
    }

    #[test]
    fn landscape_swaps_width_and_height() {
        let portrait = PageSize::us_letter();
        let landscape = portrait.landscape();
        assert_eq!(landscape.width_twips, portrait.height_twips);
        assert_eq!(landscape.height_twips, portrait.width_twips);
        assert_eq!(landscape.orientation, PageOrientation::Landscape);
    }

    #[test]
    fn a4_and_us_letter_validate() {
        assert!(PageSize::a4().validate().is_ok());
        assert!(PageSize::us_letter().validate().is_ok());
        assert!(PageSize::a4().landscape().validate().is_ok());
    }

    #[test]
    fn a_zero_extent_is_refused() {
        let zero_width = PageSize::from_twips(0, 16_838, PageOrientation::Portrait);
        match zero_width.validate() {
            Err(DocxError::InvalidPageSize { reason, .. }) => {
                assert_eq!(reason, "page width is zero");
            }
            other => panic!("expected InvalidPageSize, got {other:?}"),
        }
        let zero_height = PageSize::from_twips(11_906, 0, PageOrientation::Portrait);
        assert!(matches!(
            zero_height.validate(),
            Err(DocxError::InvalidPageSize { .. })
        ));
    }

    #[test]
    fn a_page_smaller_than_the_fixed_margins_is_refused() {
        // 1440 + 1440 = 2880 twips of horizontal margin; a page narrower than that has no printable
        // width left, which is exactly the condition Word's own repair dialog reports.
        let too_narrow = PageSize::from_twips(2_880, 16_838, PageOrientation::Portrait);
        match too_narrow.validate() {
            Err(DocxError::InvalidPageSize { reason, .. }) => {
                assert_eq!(
                    reason,
                    "the left and right margins leave no printable width"
                );
            }
            other => panic!("expected InvalidPageSize, got {other:?}"),
        }
        let too_short = PageSize::from_twips(11_906, 2_880, PageOrientation::Portrait);
        match too_short.validate() {
            Err(DocxError::InvalidPageSize { reason, .. }) => {
                assert_eq!(
                    reason,
                    "the top and bottom margins leave no printable height"
                );
            }
            other => panic!("expected InvalidPageSize, got {other:?}"),
        }
        // One twip more than the margin sum is the boundary, and it is legal.
        let just_enough = PageSize::from_twips(2_881, 16_838, PageOrientation::Portrait);
        assert!(just_enough.validate().is_ok());
    }
}
