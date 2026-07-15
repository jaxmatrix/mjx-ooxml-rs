//! A namespace-prefix scope stack, rebuilt from the tree's `xmlns` attributes while descending.
//!
//! MCE control attributes (`Requires`, `Ignorable`, …) name namespaces by *prefix*, so resolving
//! them to URIs needs the prefix bindings in effect at each element.

use mjx_ooxml_core::{Interner, RawElement};

/// A stack of prefix→URI binding frames (one frame per open element).
#[derive(Debug, Default)]
pub struct NamespaceScope {
    frames: Vec<Vec<(String, String)>>,
}

impl NamespaceScope {
    /// A new, empty scope.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes the `xmlns` declarations of `element` as a new frame. `xmlns:foo` binds prefix `foo`;
    /// a default `xmlns` binds the empty prefix.
    pub fn push_element(&mut self, element: &RawElement, interner: &Interner) {
        let mut frame = Vec::new();
        for attr in &element.attributes {
            let local = interner.resolve(attr.name.local);
            let is_prefixed_decl = attr
                .name
                .prefix
                .is_some_and(|p| interner.resolve(p) == "xmlns");
            if is_prefixed_decl {
                frame.push((local.to_owned(), value_string(&attr.value)));
            } else if attr.name.prefix.is_none() && local == "xmlns" {
                frame.push((String::new(), value_string(&attr.value)));
            }
        }
        self.frames.push(frame);
    }

    /// Pops the most recently pushed frame.
    pub fn pop(&mut self) {
        self.frames.pop();
    }

    /// Resolves a prefix to its bound URI (innermost binding wins).
    #[must_use]
    pub fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        for frame in self.frames.iter().rev() {
            if let Some((_, uri)) = frame.iter().rev().find(|(p, _)| p == prefix) {
                return Some(uri);
            }
        }
        None
    }
}

fn value_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
