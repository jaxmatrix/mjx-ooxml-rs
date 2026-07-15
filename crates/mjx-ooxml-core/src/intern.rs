//! A small, pure-Rust string interner for the hot, repeated strings in OOXML: namespace URIs and
//! element/attribute prefixes and local names. Each distinct string is stored once; everything else
//! holds a cheap 4-byte [`Symbol`], and equality is an integer compare.
//!
//! Scope: **one interner per parsed part/document** (see [`crate::raw::RawDocument`]). A [`Symbol`]
//! is only meaningful via the interner that produced it.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

/// A handle to an interned string. Uses a `NonZero` niche so `Option<Symbol>` is still 4 bytes —
/// important because `prefix`/`namespace` are `Option<Symbol>` and appear on every name.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Symbol(NonZeroU32);

impl Symbol {
    /// Builds a symbol from a 0-based storage index (stored 1-based to use the niche).
    fn from_index(index: usize) -> Self {
        let one_based = u32::try_from(index + 1).expect("interner exceeded u32 capacity");
        // `index + 1 >= 1`, so this is always non-zero.
        Self(NonZeroU32::new(one_based).expect("one-based index is non-zero"))
    }

    /// The 0-based storage index.
    fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

/// Interns strings, mapping each to a stable [`Symbol`].
///
/// `Send + Sync` (all fields are). Interning takes `&mut self`; any shared use across threads wraps
/// the interner in a lock at a higher layer — the core stays lock-free.
#[derive(Debug, Default)]
pub struct Interner {
    by_id: Vec<Arc<str>>,
    by_str: HashMap<Arc<str>, Symbol>,
}

impl Interner {
    /// Creates an empty interner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the symbol for `s`, interning it if new. The stored `Arc<str>` is shared between the
    /// index and the lookup map, so no string is allocated twice.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.by_str.get(s) {
            return sym;
        }
        let arc: Arc<str> = Arc::from(s);
        let sym = Symbol::from_index(self.by_id.len());
        self.by_id.push(Arc::clone(&arc));
        self.by_str.insert(arc, sym);
        sym
    }

    /// Returns the symbol for `s` if it was already interned, without inserting.
    #[must_use]
    pub fn get(&self, s: &str) -> Option<Symbol> {
        self.by_str.get(s).copied()
    }

    /// Resolves a symbol back to its string.
    ///
    /// # Panics
    /// Panics if `sym` was not produced by this interner (a programming error).
    #[must_use]
    pub fn resolve(&self, sym: Symbol) -> &str {
        &self.by_id[sym.index()]
    }

    /// The number of distinct interned strings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether nothing has been interned yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interns_and_deduplicates() {
        let mut i = Interner::new();
        let a = i.intern("w:val");
        let b = i.intern("w:val");
        let c = i.intern("w:b");
        assert_eq!(a, b, "same string interns to the same symbol");
        assert_ne!(a, c);
        assert_eq!(i.len(), 2);
        assert_eq!(i.resolve(a), "w:val");
        assert_eq!(i.resolve(c), "w:b");
    }

    #[test]
    fn get_does_not_insert() {
        let mut i = Interner::new();
        assert_eq!(i.get("x"), None);
        let s = i.intern("x");
        assert_eq!(i.get("x"), Some(s));
        assert_eq!(i.len(), 1);
    }

    #[test]
    fn option_symbol_is_four_bytes() {
        assert_eq!(core::mem::size_of::<Option<Symbol>>(), 4);
    }

    #[test]
    fn interner_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Interner>();
        assert_send_sync::<Symbol>();
    }
}
