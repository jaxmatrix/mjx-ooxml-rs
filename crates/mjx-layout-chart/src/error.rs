//! What can go wrong between a chart part's bytes and its fragments.
//!
//! Every variant is something a *file* can cause. Nothing here reports a defect in this crate,
//! because a chart with no series, no axes, no values or values that are all the same number is a
//! chart this engine lays out rather than one it refuses — see [`crate::geometry`]'s account of
//! degenerate data.

use mjx_ooxml_core::FromXmlError;
use mjx_xml::XmlError;

/// A chart or diagram that could not be laid out.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ChartLayoutError {
    /// The part is not well-formed XML.
    #[error("the chart part is not well-formed XML: {0}")]
    Malformed(#[from] XmlError),
    /// The part is well-formed but its root is not a `c:chartSpace`, or something inside it that
    /// this tier models is malformed.
    #[error("the chart part is not a readable `c:chartSpace`: {0}")]
    NotReadable(#[from] FromXmlError),
    /// A `c:chartSpace` with no `c:chart` in it. The schema requires one, so a part without it is a
    /// file defect reported rather than repaired.
    #[error("the chart part has a `c:chartSpace` with no `c:chart` inside it")]
    NoChart,
    /// A diagram whose layout definition names an algorithm this engine does not evaluate.
    ///
    /// **Named rather than approximated.** ECMA-376 Part 1 §21.4 declares ten layout algorithms and
    /// this engine evaluates three of them (see [`crate::diagram`]); standing in for one of the
    /// other seven with a linear row would put a customer's organisation chart on a screen in a
    /// shape their file does not describe.
    #[error("this engine does not evaluate the `{algorithm}` diagram layout algorithm")]
    DiagramAlgorithmNotEvaluated {
        /// The `dgm:alg@type` wire token.
        algorithm: &'static str,
    },
    /// A diagram whose data model is empty, or whose points are all of a kind that draws nothing.
    #[error("the diagram data model has no drawable point")]
    DiagramHasNoPoints,
}
