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

    /// An XML control part (`[Content_Types].xml` or a `.rels` part) was malformed.
    #[error("xml error: {0}")]
    Xml(#[from] mjx_xml::XmlError),

    /// The package violated an Open Packaging Conventions rule.
    #[error("malformed package: {0}")]
    Malformed(String),
}

impl OpcError {
    pub(crate) fn malformed(msg: impl Into<String>) -> Self {
        Self::Malformed(msg.into())
    }
}
