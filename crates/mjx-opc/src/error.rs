//! Error type for the OPC layer.

/// Errors produced while opening, parsing, or saving an OPC package.
#[derive(Debug, thiserror::Error)]
pub enum OpcError {
    /// The underlying ZIP container could not be read or written.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// An I/O error occurred while reading or writing container bytes.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The package violates a packaging invariant, so [`Package::save`](crate::Package::save)
    /// refused to write it. See [`PackageDefect`](crate::PackageDefect).
    #[error(transparent)]
    Invalid(#[from] crate::PackageDefect),

    /// An XML control part (`[Content_Types].xml` or a `.rels` part) was malformed.
    #[error("xml error: {0}")]
    Xml(#[from] mjx_xml::XmlError),

    /// The package violated an Open Packaging Conventions rule.
    #[error("malformed package: {0}")]
    Malformed(String),

    /// A part was addressed that does not exist in the package.
    #[error("unknown part: {0}")]
    UnknownPart(String),

    /// A relationship target points outside the package (an absolute URI such as `http://…`), so it
    /// names no part. Such a relationship is legitimate — it is simply not resolvable to a part name.
    #[error("relationship target is external to the package: {0}")]
    ExternalTarget(String),

    /// A relationship target could not be resolved to a part name — it climbed above the package root
    /// with `..`, or the result failed part-name validation.
    #[error("could not resolve relationship target: {0}")]
    TargetResolution(String),

    /// A control part (`[Content_Types].xml` or a `.rels` part) was addressed through the generic
    /// part-tree API. Control parts are edited only through the dedicated content-type and
    /// relationship helpers, so their parsed navigation views can never drift from the raw tree.
    #[error("control part cannot be edited as a generic part tree: {0}")]
    ControlPart(String),

    /// A [`DocumentTimestamp`](crate::doc_props::DocumentTimestamp) field was outside its range —
    /// e.g. a month of `13` — refused rather than emitting a `dcterms:created` / `dcterms:modified`
    /// value no conforming consumer accepts.
    #[error("document timestamp field `{field}` is {value}, outside its range {min}..={max}")]
    InvalidDocumentTimestamp {
        /// The field name (`"year"`, `"month"`, `"day"`, `"hour"`, `"minute"`, or `"second"`).
        field: &'static str,
        /// The out-of-range value supplied.
        value: u32,
        /// The smallest value the field accepts.
        min: u32,
        /// The largest value the field accepts.
        max: u32,
    },
}

impl OpcError {
    pub(crate) fn malformed(msg: impl Into<String>) -> Self {
        Self::Malformed(msg.into())
    }

    pub(crate) fn unknown_part(name: &str) -> Self {
        Self::UnknownPart(name.to_owned())
    }

    pub(crate) fn control_part(name: &str) -> Self {
        Self::ControlPart(name.to_owned())
    }
}
