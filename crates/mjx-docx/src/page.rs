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

use crate::error::DocxError;

/// A page's extent and orientation (`w:pgSz`), in twips.
///
/// Only size and orientation are caller-supplied here — margins are fixed at `PageMargins::NORMAL`
/// (this module's own private constant), Word's own "Normal" template default, regardless of page
/// size. The full `w:sectPr` model (headers/footers, columns, line numbering, …) is MJXOFF-106's;
/// this is the minimum [`crate::Document::blank`] needs, kept small on purpose so that child can
/// replace it rather than extend it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    /// `w:pgSz@w`, in twips (1/1440 inch). The *page* width — for [`PageOrientation::Landscape`]
    /// this is the larger of the two physical dimensions, matching how Word itself swaps `w`/`h`
    /// rather than leaving them at their portrait values and relying on `@orient` alone.
    pub width_twips: u32,
    /// `w:pgSz@h`, in twips.
    pub height_twips: u32,
    /// `w:pgSz@orient`. [`PageOrientation::Portrait`] is written by omitting the attribute (its
    /// schema default), matching every committed fixture and real Office output.
    pub orientation: PageOrientation,
}

/// `ST_PageOrientation` (`wml.xsd`) — `w:pgSz@orient`'s two wire values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOrientation {
    /// Wire value `portrait` — also the schema default, so [`PageSize`] omits the attribute rather
    /// than writing it out.
    Portrait,
    /// Wire value `landscape`.
    Landscape,
}

impl PageOrientation {
    /// The wire token, or `None` for [`Portrait`](Self::Portrait) — see [`PageSize`]'s own doc
    /// comment for why the schema default is omitted rather than spelled out.
    #[must_use]
    pub fn to_wire(self) -> Option<&'static str> {
        match self {
            Self::Portrait => None,
            Self::Landscape => Some("landscape"),
        }
    }
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
/// [`crate::blank`]'s mutation gate proves this struct's fields, not `w:pgSz`'s (`CT_PageSz`'s `w`
/// and `h` are both `use="optional"`, contrary to what this ticket's brief originally claimed —
/// see this crate's `CHANGELOG.md` entry for MJXOFF-98).
///
/// Not caller-configurable in [`crate::Document::blank`] — only page size and orientation are, per
/// this child's own scope. [`NORMAL`](Self::NORMAL) is Word's own "Normal" template default, used
/// regardless of page size or orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageMargins {
    pub(crate) top: i32,
    pub(crate) right: u32,
    pub(crate) bottom: i32,
    pub(crate) left: u32,
    pub(crate) header: u32,
    pub(crate) footer: u32,
    pub(crate) gutter: u32,
}

impl PageMargins {
    /// Word's "Normal" template margins: 1 inch on every side, ½ inch header/footer, no gutter.
    pub(crate) const NORMAL: Self = Self {
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
        assert!(PageOrientation::Portrait.to_wire().is_none());
        assert_eq!(PageOrientation::Landscape.to_wire(), Some("landscape"));
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
