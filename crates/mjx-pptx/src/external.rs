//! Neutralizing inaccessible external sources.
//!
//! An OOXML element can bind its content through a *linked* (external) relationship — a picture that
//! links its image, a chart that links its workbook, a linked OLE object or media file. When that
//! target is unreachable on the current platform, a consumer can fail on it. This module provides the
//! caller-driven tools to replace such a source with an in-package placeholder of the same kind, so
//! the presentation stands on its own. The library performs no external I/O and cannot judge
//! reachability itself — the caller decides which references to neutralize.
//!
//! P1 covers linked *images*; later phases extend the same idea to the workbook behind a chart, OLE
//! objects, and media, composing [`mjx_opc::Package::retarget_relationship`] for the element kinds the
//! library does not model.

/// The built-in placeholder image used when a caller does not supply their own — a tiny valid PNG,
/// embedded so a neutralized picture always resolves inside the package. Callers who want recognizable
/// artwork pass their own bytes instead.
///
/// A valid 2×2 truecolour PNG (76 bytes).
pub const DEFAULT_PLACEHOLDER_IMAGE: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// A picture that *links* its image (`a:blip@r:link`) rather than embedding it — a candidate for
/// [`Presentation::replace_linked_image_with_placeholder`](crate::Presentation::replace_linked_image_with_placeholder),
/// as reported by [`Presentation::linked_images`](crate::Presentation::linked_images).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedImage {
    /// The shape index of the picture on the surface it was found on.
    pub shape_index: usize,
    /// Where the image is linked from — the relationship target (an external path/URL, or an
    /// in-package part target for an internal link).
    pub target: String,
}

/// A chart frame that references a backing workbook (`c:externalData`) — a candidate for
/// [`Presentation::detach_chart_workbook`](crate::Presentation::detach_chart_workbook), as reported by
/// [`Presentation::chart_workbooks`](crate::Presentation::chart_workbooks). A chart renders from its
/// cached data, so detaching an inaccessible workbook leaves the chart intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartWorkbook {
    /// The shape index of the chart frame on the surface it was found on.
    pub shape_index: usize,
    /// Where the workbook is referenced from — the relationship target (an external path/URL, or an
    /// in-package part target for an embedded workbook).
    pub target: String,
    /// Whether that relationship is external (`TargetMode="External"`), i.e. the case that can be
    /// unreachable on another platform. An embedded workbook is `false`.
    pub external: bool,
}
