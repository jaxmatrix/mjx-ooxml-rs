//! Part names — validated Open Packaging Conventions part URIs.
//!
//! A part name is an absolute, `/`-rooted URI (e.g. `/ppt/presentation.xml`). Inside the ZIP
//! container the same part is stored under a *relative* entry name with no leading slash
//! (`ppt/presentation.xml`); [`PartName`] converts between the two forms.

use crate::error::OpcError;

/// A validated OPC part name (absolute, `/`-rooted, normalized).
///
/// # Examples
///
/// ```
/// use mjx_opc::PartName;
/// let part = PartName::from_zip_name("ppt/slides/slide1.xml").unwrap();
/// assert_eq!(part.as_str(), "/ppt/slides/slide1.xml"); // absolute, `/`-rooted
/// assert_eq!(part.zip_name(), "ppt/slides/slide1.xml"); // ZIP entry form
/// assert_eq!(part.extension().as_deref(), Some("xml"));
/// ```
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

    /// Resolves a relationship `target` written in **this** part's `.rels` to the part it names.
    ///
    /// Targets are relative to the source part's *directory* (`/ppt/slides/slide1.xml` +
    /// `../media/image1.png` = `/ppt/media/image1.png`), except that a `/`-rooted target is already
    /// absolute. [`relative_target`](Self::relative_target) is the inverse.
    ///
    /// # Errors
    /// Returns [`OpcError::ExternalTarget`] if the target points outside the package (an absolute
    /// URI), or [`OpcError::TargetResolution`] if it climbs above the package root or does not
    /// validate as a part name.
    pub fn resolve(&self, target: &str) -> Result<Self, OpcError> {
        // `self` is absolute, so there is always a leading '/'; include it in the base directory.
        let dir_end = self.0.rfind('/').map_or(0, |idx| idx + 1);
        resolve_in_dir(&self.0[..dir_end], target)
    }

    /// Resolves a relationship `target` written in the **package root**'s `.rels` (`/_rels/.rels`),
    /// whose base directory is `/`.
    ///
    /// # Errors
    /// As [`resolve`](Self::resolve).
    pub fn resolve_from_root(target: &str) -> Result<Self, OpcError> {
        resolve_in_dir("/", target)
    }

    /// The relationship target to write in **this** part's `.rels` so that it resolves to `target` —
    /// the inverse of [`resolve`](Self::resolve).
    ///
    /// The result is relative to this part's directory, with one `..` segment per directory level
    /// that has to be climbed (`/ppt/slides/slide1.xml` → `/ppt/media/image1.png` =
    /// `../media/image1.png`). This is the form Office writes, and it keeps a part relocatable with
    /// its neighbours.
    #[must_use]
    pub fn relative_target(&self, target: &Self) -> String {
        let source_segments = path_segments(&self.0);
        let target_segments = path_segments(&target.0);
        // Both are absolute part names, so the last segment is the file name, not a directory.
        let source_depth = source_segments.len() - 1;
        let shared = source_segments
            .iter()
            .take(source_depth)
            .zip(target_segments.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let mut out = String::new();
        for _ in shared..source_depth {
            out.push_str("../");
        }
        for segment in &target_segments[shared..] {
            out.push_str(segment);
            out.push('/');
        }
        out.pop(); // the trailing '/' after the file name
        out
    }
}

/// The non-empty `/`-separated segments of an absolute part name.
fn path_segments(part: &str) -> Vec<&str> {
    part.split('/').filter(|s| !s.is_empty()).collect()
}

/// Whether a target points outside the package (an absolute URI).
fn is_external(target: &str) -> bool {
    target.contains("://") || target.starts_with("//")
}

fn resolve_in_dir(base_dir: &str, target: &str) -> Result<PartName, OpcError> {
    if is_external(target) {
        return Err(OpcError::ExternalTarget(target.to_owned()));
    }
    let joined = if target.starts_with('/') {
        target.to_owned()
    } else {
        format!("{base_dir}{target}")
    };
    let normalized =
        normalize(&joined).ok_or_else(|| OpcError::TargetResolution(target.to_owned()))?;
    PartName::new(&normalized)
}

/// Normalizes an absolute path, folding `.` and `..` segments. Returns `None` if `..` escapes the root.
fn normalize(path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(format!("/{}", segments.join("/")))
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

    fn part(name: &str) -> PartName {
        PartName::new(name).expect("valid part name")
    }

    #[test]
    fn resolves_a_relative_target_against_the_sources_directory() {
        assert_eq!(
            part("/ppt/presentation.xml")
                .resolve("slides/slide1.xml")
                .unwrap()
                .as_str(),
            "/ppt/slides/slide1.xml"
        );
        assert_eq!(
            part("/ppt/slides/slide1.xml")
                .resolve("../slideLayouts/slideLayout1.xml")
                .unwrap()
                .as_str(),
            "/ppt/slideLayouts/slideLayout1.xml"
        );
    }

    #[test]
    fn resolve_from_root_prepends_slash() {
        assert_eq!(
            PartName::resolve_from_root("ppt/presentation.xml")
                .unwrap()
                .as_str(),
            "/ppt/presentation.xml"
        );
    }

    #[test]
    fn resolve_rejects_root_escape_and_external_targets() {
        let err = part("/a/b.xml").resolve("../../x").unwrap_err();
        assert!(matches!(err, OpcError::TargetResolution(_)), "{err:?}");

        let err = part("/ppt/presentation.xml")
            .resolve("http://example.com/x")
            .unwrap_err();
        assert!(matches!(err, OpcError::ExternalTarget(_)), "{err:?}");
    }

    #[test]
    fn relative_target_climbs_to_a_sibling_directory() {
        assert_eq!(
            part("/ppt/slides/slide1.xml").relative_target(&part("/ppt/media/image1.png")),
            "../media/image1.png"
        );
        assert_eq!(
            part("/ppt/slides/slide1.xml").relative_target(&part("/ppt/slides/slide2.xml")),
            "slide2.xml"
        );
        assert_eq!(
            part("/ppt/presentation.xml").relative_target(&part("/ppt/slides/slide2.xml")),
            "slides/slide2.xml"
        );
        assert_eq!(
            part("/ppt/slides/slide1.xml").relative_target(&part("/docProps/app.xml")),
            "../../docProps/app.xml"
        );
    }

    #[test]
    fn relative_target_is_the_inverse_of_resolve() {
        for (source, target) in [
            ("/ppt/slides/slide1.xml", "/ppt/media/image1.png"),
            ("/ppt/presentation.xml", "/ppt/slides/slide2.xml"),
            ("/ppt/slides/slide1.xml", "/docProps/app.xml"),
            ("/a.xml", "/b.xml"),
        ] {
            let (source, target) = (part(source), part(target));
            let relative = source.relative_target(&target);
            assert_eq!(
                source.resolve(&relative).unwrap(),
                target,
                "round trip failed for {relative}"
            );
        }
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
