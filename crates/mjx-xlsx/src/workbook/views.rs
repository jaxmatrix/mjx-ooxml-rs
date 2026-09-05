//! `x:bookViews` — how a consumer opens the workbook window: which tab is active, where the window
//! sits, how wide the tab strip is.
//!
//! **Filled by MJXOFF-100 (D06).** Nothing here yet: MJXOFF-91 (D02) builds the package and the part
//! graph and models nothing at all.
//!
//! What belongs here: `CT_BookViews` and `CT_BookView`/`CT_CustomWorkbookView` — `x:workbookView`'s
//! `activeTab`, `firstSheet`, the window geometry `tests/fixtures/sample.xlsx` carries
//! (`xWindow`/`yWindow`/`windowWidth`/`windowHeight`/`tabRatio`), the scroll-bar and tab-strip
//! toggles, and the custom-view list beside them.
//!
//! It is a file rather than a section of [`super`] for the reason the whole directory exists: the
//! workbook part has four unrelated subjects (the sheet list, the views, the properties, the defined
//! names) and no one of them should have to be read past to reach another.
