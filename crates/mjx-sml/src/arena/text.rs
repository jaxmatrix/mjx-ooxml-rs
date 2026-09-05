//! The store's byte space: one address for source bytes and edited bytes alike.
//!
//! # Why every preserved value in the store is a byte range
//!
//! A worksheet is the one part of OOXML where the *cell* count decides whether the library is
//! usable. `docs/BENCHMARKS.md` measures a 300,000-cell worksheet costing **≈ 913 bytes of peak
//! resident set per cell** once it is materialised as a [`RawElement`](mjx_ooxml_core::RawElement)
//! tree — and records that the gap is not the element struct (72 B) but the two small heap
//! allocations every element carries for its children and its attributes, times well over a million
//! of them.
//!
//! So the store owns no per-cell allocation at all. Every value it preserves — a cell's `<v>` text,
//! a row's attribute run, an `extLst` nobody models — is a `(start, length)` pair into one byte
//! space, and the pair is eight bytes with no pointer, no capacity and no destructor.
//!
//! # One address space, two backings
//!
//! The byte space is the concatenation of two buffers:
//!
//! ```text
//! 0                       source.len()                      source.len() + edits.len()
//! ├──────── the part's source bytes ────────┼──────── bytes this store authored ────────┤
//! ```
//!
//! An address below `source.len()` resolves into the part's own bytes, which the store shares with
//! the package through an [`Arc`] and never copies; an address at or above it resolves into the
//! store's own `edits` vector, which starts empty and grows only when something is edited or
//! authored. That is copy-on-write stated at value scale: **a worksheet nobody has touched holds no
//! bytes of its own**, and an edited one holds exactly the bytes of the edits.
//!
//! Making it one address space rather than two, with an origin flag, is worth stating: a flag would
//! have to live somewhere, and the only somewhere available is the record being kept small — a bit
//! stolen from a length, or a byte that rounds the struct up by four. The concatenation needs
//! neither, because the boundary is a property of the arena rather than of each span.
//!
//! # The four-gigabyte limit, and why it is checked rather than assumed
//!
//! A span is two `u32`s, so the whole byte space must stay under [`u32::MAX`]. That is not a
//! constraint anyone will meet — a `.xlsx` part above 4 GiB decompressed is beyond what any producer
//! writes, and `mjx-xml`'s own reader already stops recording source ranges past that point — but it
//! is checked on the way in and on every store, and reported as
//! [`SmlError::PackedStoreTooLarge`](crate::SmlError::PackedStoreTooLarge). Untrusted input does not get
//! to decide whether an index is in range.

use std::sync::Arc;

use crate::error::SmlError;

/// The highest byte address the arena can hand out. One below [`u32::MAX`], which is reserved as
/// [`TextSpan::NONE`]'s start.
pub(crate) const MAXIMUM_ADDRESS: u32 = u32::MAX - 1;

/// A range of bytes in a [`TextArena`], or nothing at all.
///
/// Eight bytes, [`Copy`], no destructor — which is the point, since the cell record holding three of
/// them is multiplied by a million.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSpan {
    start: u32,
    length: u32,
}

/// **Absent**, not "empty at address zero".
///
/// The distinction is not academic: `CellExtras` is `Default`-constructed for every cell that turns
/// out to need one, and a derived `Default` here would give each of its fields a *present* span over
/// zero bytes at address zero. Every cell would then look as though it carried an empty attribute
/// run, which reads back as a cell with no attributes at all.
impl Default for TextSpan {
    fn default() -> Self {
        Self::NONE
    }
}

impl TextSpan {
    /// The absent span — what a cell with no `<v>` or a row with no trailing content holds.
    ///
    /// Distinct from an *empty* span: `<v></v>` is present and empty, and must come back as
    /// `<v></v>` rather than as nothing at all. [`u32::MAX`] is free to be the sentinel because
    /// [`MAXIMUM_ADDRESS`] is one below it.
    pub(crate) const NONE: Self = Self {
        start: u32::MAX,
        length: 0,
    };

    /// A span over `length` bytes beginning at `start`.
    pub(crate) fn new(start: u32, length: u32) -> Self {
        Self { start, length }
    }

    /// Whether this is [`NONE`](Self::NONE) — absent, as opposed to present and empty.
    pub(crate) fn is_none(self) -> bool {
        self.start == u32::MAX
    }

    /// The first byte address, meaningless when [`is_none`](Self::is_none).
    pub(crate) fn start(self) -> u32 {
        self.start
    }

    /// One past the last byte address, meaningless when [`is_none`](Self::is_none).
    pub(crate) fn end(self) -> u32 {
        self.start.saturating_add(self.length)
    }

    /// How many bytes this span covers.
    pub(crate) fn length(self) -> u32 {
        self.length
    }
}

/// The store's byte space: the part's own bytes, then whatever the store has authored.
///
/// See the [module docs](self) for the layout and why it is one address space rather than two.
#[derive(Debug, Default)]
pub(crate) struct TextArena {
    /// The part's source bytes, shared with the package rather than copied. `None` for a sheet built
    /// from a tree that no longer remembers where it came from — every span is then an edit.
    source: Option<Arc<[u8]>>,
    /// `source.len()`, cached as the `u32` the addressing arithmetic needs.
    source_length: u32,
    /// Bytes this store authored: edited values, regenerated attribute runs, and any preserved node
    /// that had no source range to point at.
    edits: Vec<u8>,
}

impl TextArena {
    /// An arena over `source`, or over nothing at all when a sheet was built from a tree with no
    /// buffer behind it.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if the buffer is too large for a `u32` address space.
    pub(crate) fn new(source: Option<Arc<[u8]>>) -> Result<Self, SmlError> {
        let source_length = match &source {
            Some(bytes) => {
                u32::try_from(bytes.len()).map_err(|_| SmlError::PackedStoreTooLarge {
                    bytes: bytes.len() as u64,
                })?
            }
            None => 0,
        };
        if source_length > MAXIMUM_ADDRESS {
            return Err(SmlError::PackedStoreTooLarge {
                bytes: u64::from(source_length),
            });
        }
        Ok(Self {
            source,
            source_length,
            edits: Vec::new(),
        })
    }

    /// How many bytes of its own the store has authored. Zero for a worksheet nobody has edited,
    /// which is the property the copy-on-write rule is about.
    pub(crate) fn edited_bytes(&self) -> usize {
        self.edits.len()
    }

    /// The bytes `span` covers.
    ///
    /// A span that does not fit resolves to the empty slice rather than panicking: spans are built
    /// from ranges a file supplied, and this is called on the write path where a panic would be a
    /// crash on untrusted input. An out-of-range span therefore degrades to "nothing here", never to
    /// somebody else's bytes.
    pub(crate) fn bytes(&self, span: TextSpan) -> &[u8] {
        if span.is_none() {
            return &[];
        }
        let (start, end) = (span.start(), span.end());
        if start < self.source_length {
            let Some(source) = self.source.as_deref() else {
                return &[];
            };
            let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
                return &[];
            };
            return source.get(start..end).unwrap_or_default();
        }
        let start = (start - self.source_length) as usize;
        let end = start.saturating_add(span.length() as usize);
        self.edits.get(start..end).unwrap_or_default()
    }

    /// A span over the source range `start..end`, or [`TextSpan::NONE`] if that range is not one
    /// this arena can address.
    ///
    /// Used to turn a [`RawElement::source_span`](mjx_ooxml_core::RawElement::source_span) into a
    /// span of the store's own — the step that lets an untouched row re-emit without being
    /// re-serialised.
    pub(crate) fn span_in_source(&self, start: u32, end: u32) -> TextSpan {
        if end < start || end > self.source_length {
            return TextSpan::NONE;
        }
        TextSpan::new(start, end - start)
    }

    /// A span over `start..end` **anywhere** in the byte space — the part's own bytes or this
    /// arena's own — or [`TextSpan::NONE`] if that is not a range this arena can address.
    ///
    /// [`span_in_source`](Self::span_in_source) is the narrower question, and the two are not
    /// interchangeable. A store reading a part asks the narrower one, because a range past the
    /// source is a range the file did not have. A store reading back markup it has *just authored*
    /// asks this one, because those bytes live in the second half of the address space by
    /// construction — and refusing them there would make an authored item the one item with no
    /// bytes behind it, which is exactly the state the design exists to not have.
    pub(crate) fn span_over(&self, start: u32, end: u32) -> TextSpan {
        if end < start || u64::from(end) > self.total_length() {
            return TextSpan::NONE;
        }
        TextSpan::new(start, end - start)
    }

    /// How many bytes the whole address space currently holds.
    pub(crate) fn total_length(&self) -> u64 {
        u64::from(self.source_length) + self.edits.len() as u64
    }

    /// Appends `bytes` and returns the span covering them.
    ///
    /// # Errors
    ///
    /// [`SmlError::PackedStoreTooLarge`] if the byte space would grow past what a `u32` can address.
    pub(crate) fn store(&mut self, bytes: &[u8]) -> Result<TextSpan, SmlError> {
        let length = u32::try_from(bytes.len()).map_err(|_| SmlError::PackedStoreTooLarge {
            bytes: bytes.len() as u64,
        })?;
        let start = self
            .source_length
            .checked_add(u32::try_from(self.edits.len()).map_err(|_| {
                SmlError::PackedStoreTooLarge {
                    bytes: self.edits.len() as u64,
                }
            })?)
            .ok_or(SmlError::PackedStoreTooLarge { bytes: u64::MAX })?;
        if start
            .checked_add(length)
            .is_none_or(|end| end > MAXIMUM_ADDRESS)
        {
            return Err(SmlError::PackedStoreTooLarge {
                bytes: u64::from(start) + u64::from(length),
            });
        }
        self.edits.extend_from_slice(bytes);
        Ok(TextSpan::new(start, length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena(source: &[u8]) -> TextArena {
        TextArena::new(Some(Arc::from(source))).expect("a small buffer is addressable")
    }

    #[test]
    fn a_source_span_and_an_edited_span_live_in_one_address_space() {
        let mut arena = arena(b"<c r=\"A1\"><v>12</v></c>");
        let from_source = arena.span_in_source(13, 15);
        assert_eq!(arena.bytes(from_source), b"12");

        let authored = arena.store(b"99").expect("the arena has room");
        assert_eq!(arena.bytes(authored), b"99");
        assert!(
            authored.start() >= 23,
            "an authored span addresses past the source, not into it: {authored:?}"
        );
        // The two resolve differently through one `bytes` call, with no origin flag anywhere.
        assert_ne!(arena.bytes(from_source), arena.bytes(authored));
    }

    #[test]
    fn an_untouched_arena_owns_no_bytes() {
        let arena = arena(b"<sheetData/>");
        assert_eq!(
            arena.edited_bytes(),
            0,
            "a store that has not been edited must hold no bytes of its own"
        );
    }

    #[test]
    fn absent_and_empty_are_different_things() {
        let arena = arena(b"<v></v>");
        assert!(TextSpan::NONE.is_none());
        assert_eq!(arena.bytes(TextSpan::NONE), b"");
        let empty = arena.span_in_source(3, 3);
        assert!(!empty.is_none(), "`<v></v>` is present and empty");
        assert_eq!(empty.length(), 0);
        assert_eq!(arena.bytes(empty), b"");
    }

    #[test]
    fn a_span_that_does_not_fit_resolves_to_nothing_rather_than_panicking() {
        let arena = arena(b"<v>1</v>");
        assert_eq!(arena.bytes(TextSpan::new(3, 10_000)), b"");
        assert_eq!(arena.bytes(TextSpan::new(9_999, 1)), b"");
        assert_eq!(
            arena.span_in_source(5, 2),
            TextSpan::NONE,
            "an inverted range is not a range"
        );
        assert_eq!(
            arena.span_in_source(0, 9_999),
            TextSpan::NONE,
            "a range past the buffer is not a range"
        );
    }

    #[test]
    fn a_span_over_authored_bytes_is_addressable_and_a_span_past_the_end_is_not() {
        let mut arena = arena(b"<si><t>a</t></si>");
        let authored = arena
            .store(b"<si><t>b</t></si>")
            .expect("the arena has room");
        // The exact question the authoring path asks: is `[start, end)` of what was just stored a
        // range this arena can hand back?
        let over = arena.span_over(authored.start(), authored.end());
        assert_eq!(arena.bytes(over), b"<si><t>b</t></si>");
        assert_eq!(
            arena.span_in_source(authored.start(), authored.end()),
            TextSpan::NONE,
            "the source-only constructor must refuse an authored range, which is why both exist"
        );
        assert_eq!(arena.span_over(0, 9_999), TextSpan::NONE);
        assert_eq!(arena.span_over(5, 2), TextSpan::NONE);
    }

    #[test]
    fn spans_are_eight_bytes() {
        assert_eq!(
            core::mem::size_of::<TextSpan>(),
            8,
            "a cell record holds three of these; their size is the design"
        );
    }
}
