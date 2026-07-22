//! [`Hyperlink`] — where a click hyperlink (`a:hlinkClick`) on a run or a shape points.
//!
//! A hyperlink names a **relationship** by `r:id`, not a target directly: an external URL is an
//! `External` relationship the browser follows, and a slide jump is an `Internal` relationship to the
//! target slide part. This type is the resolved form a caller reads and writes — a URL string or a
//! slide index — so the relationship indirection stays inside [`Presentation`](crate::Presentation),
//! exactly as it does for an embedded image.
//!
//! Links this build does not model — mouse-over actions, tooltips, and the `ppaction://` show jumps
//! (first / last / next / previous slide) — are **preserved byte-for-byte** on parts it does not
//! rewrite; they simply have no API here yet.

/// Where a click hyperlink points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hyperlink {
    /// An external target — a web URL, `mailto:` address, or file path — followed as-is. Stored as an
    /// `External` relationship whose target is this string.
    Url(String),
    /// A jump to another slide in the same deck, addressed by its index (as in
    /// [`slide_count`](crate::Presentation::slide_count)). Stored as an `Internal` relationship to
    /// that slide part plus the action `ppaction://hlinksldjump`.
    Slide(usize),
}
