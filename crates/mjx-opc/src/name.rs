//! Part names — validated Open Packaging Conventions part URIs.
//!
//! A part name is an absolute, `/`-rooted URI (e.g. `/ppt/presentation.xml`). Inside the ZIP
//! container the same part is stored under a *relative* entry name with no leading slash
//! (`ppt/presentation.xml`); [`PartName`] converts between the two forms.

use crate::error::OpcError;

/// A validated OPC part name (absolute, `/`-rooted, normalized).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartName(String);

impl PartName {
    /// Validates and constructs a part name from its absolute (`/`-rooted) form.
    ///
    /// # Errors
    /// Returns [`OpcError::Malformed`] if the name is not absolute, ends with `/`, contains an
    /// empty segment (`//`), or contains a `.` / `..` segment.
    pub fn new(s: &str) -> Result<Self, OpcError> {
        if !s.starts_with('/') {
            return Err(OpcError::malformed(format!(
                "part name must be absolute: {s:?}"
            )));
        }
        if s.len() == 1 {
            return Err(OpcError::malformed(
                "`/` is the package root, not a part name",
            ));
        }
        if s.ends_with('/') {
            return Err(OpcError::malformed(format!(
                "part name must not end with '/': {s:?}"
            )));
        }
        for segment in s[1..].split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(OpcError::malformed(format!(
                    "invalid segment in part name: {s:?}"
                )));
            }
        }
        Ok(Self(s.to_owned()))
    }

    /// Constructs a part name from a ZIP entry name (relative, no leading slash).
    ///
    /// # Errors
    /// Propagates validation errors from [`PartName::new`].
    pub fn from_zip_name(name: &str) -> Result<Self, OpcError> {
        Self::new(&format!("/{name}"))
    }

    /// The absolute part name, including the leading slash.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The corresponding ZIP entry name (relative form, no leading slash).
    #[must_use]
    pub fn zip_name(&self) -> &str {
        &self.0[1..]
    }

    /// The lowercased file extension of the final segment, if any.
    #[must_use]
    pub fn extension(&self) -> Option<String> {
        let last = self.0.rsplit('/').next()?;
        let dot = last.rfind('.')?;
        // OPC takes the substring after the final '.' as the extension. This includes leading-dot
        // names like `.rels`, whose extension is `rels`. Only a trailing dot yields no extension.
        let ext = &last[dot + 1..];
        if ext.is_empty() {
            return None;
        }
        Some(ext.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names_round_trip_between_forms() {
        let p = PartName::from_zip_name("ppt/presentation.xml").unwrap();
        assert_eq!(p.as_str(), "/ppt/presentation.xml");
        assert_eq!(p.zip_name(), "ppt/presentation.xml");
        assert_eq!(p.extension().as_deref(), Some("xml"));
    }

    #[test]
    fn extension_is_lowercased() {
        let p = PartName::new("/xl/media/image1.PNG").unwrap();
        assert_eq!(p.extension().as_deref(), Some("png"));
    }

    #[test]
    fn leading_dot_name_has_extension() {
        // `.rels` parts must resolve their `rels` extension for the content-type default.
        let p = PartName::new("/_rels/.rels").unwrap();
        assert_eq!(p.extension().as_deref(), Some("rels"));
    }

    #[test]
    fn rejects_invalid_names() {
        assert!(PartName::new("ppt/presentation.xml").is_err()); // not absolute
        assert!(PartName::new("/").is_err()); // package root
        assert!(PartName::new("/ppt/").is_err()); // trailing slash
        assert!(PartName::new("/ppt//x.xml").is_err()); // empty segment
        assert!(PartName::new("/ppt/../x.xml").is_err()); // dot-dot segment
    }
}
