//! [`Operation`] — one edit, addressed the way a fragment is addressed and describing itself.
//!
//! # Why the address is a [`SourceRef`] and not a document path
//!
//! `mjx-layout` already answers *"which node of the document did this fragment come from"* with a
//! part number, a path of small integers and a character range, and **no OOXML type appears in it**
//! (see that crate's `source` module). An editor's every question — where the caret is, what is
//! selected, which run this keystroke changes — is asked in exactly that vocabulary, because it is
//! the vocabulary a hit test answers in.
//!
//! Writing operations in the same terms buys three things at once and costs nothing:
//!
//! * a journal a **non-OOXML** box model can produce and consume, which is what stops this crate
//!   becoming "the `.pptx` session";
//! * an invalidation the render pipeline can already read, because it is the address the fragment
//!   tree is keyed by; and
//! * a record a future collaborative transport can carry unchanged — self-describing and
//!   address-based, which `docs/client-platform/SESSION_AND_PERSISTENCE.md` §6 asks for and which is
//!   free to arrange now and expensive to retrofit.
//!
//! # Every operation is an assignment, and that is deliberate
//!
//! There is no `InsertText`-at-an-offset here. Each operation states what the addressed node's value
//! or box **becomes**, never how it differs from what was there. That makes replay **idempotent**:
//! applying the journal tail onto a document that already received some of it lands in the same
//! state as applying it to one that did not. Recovery (`crate::recover`) leans on that directly —
//! the document is committed first and the journal truncated second, so a crash between the two
//! leaves a tail that will be replayed over work already written, and idempotence is what makes that
//! safe rather than merely likely.

use mjx_layout::{LayoutRect, SourceRef};

/// One edit: what is being changed, and what it becomes.
#[derive(Clone, PartialEq, Debug)]
pub struct Operation {
    address: SourceRef,
    kind: OperationKind,
}

impl Operation {
    /// The operation that sets the value of the node at `address`.
    #[must_use]
    pub fn set_value(address: SourceRef, value: Value) -> Self {
        Self {
            address,
            kind: OperationKind::SetValue(value),
        }
    }

    /// The operation that places the box at `address` — a move, a resize, the settled end of a drag.
    #[must_use]
    pub fn set_bounds(address: SourceRef, bounds: LayoutRect) -> Self {
        Self {
            address,
            kind: OperationKind::SetBounds(bounds),
        }
    }

    /// Which node it changes.
    #[must_use]
    pub const fn address(&self) -> &SourceRef {
        &self.address
    }

    /// What it does to that node.
    #[must_use]
    pub const fn kind(&self) -> &OperationKind {
        &self.kind
    }

    /// Bytes this operation holds on the heap, for the journal's memory bound.
    ///
    /// The address's path spills to a shared allocation only past six segments, and the whole point
    /// of the bound is to catch a journal growing without limit, so an approximation that counts the
    /// payload and not the `Arc` header is the right instrument: it tracks the thing that actually
    /// grows.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.kind.heap_bytes()
    }
}

/// What an operation does to the node it addresses.
///
/// Two shapes, because they are the two an editor's inner loop produces: typing changes a value,
/// dragging changes a box. Anything a format crate can do that neither expresses is reached through
/// its own API and is not journalled — this crate wraps editing, it does not re-implement it.
#[derive(Clone, PartialEq, Debug)]
pub enum OperationKind {
    /// The node's value becomes this.
    SetValue(Value),
    /// The node's box becomes this rectangle, in EMU, in the same absolute space a fragment is
    /// positioned in.
    SetBounds(LayoutRect),
}

impl OperationKind {
    /// A stable tag, for the journal encoding and for the undo-unit comparison.
    #[must_use]
    pub(crate) const fn tag(&self) -> u8 {
        match self {
            Self::SetValue(_) => 0,
            Self::SetBounds(_) => 1,
        }
    }

    fn heap_bytes(&self) -> usize {
        match self {
            Self::SetValue(value) => value.heap_bytes(),
            Self::SetBounds(_) => 0,
        }
    }
}

/// A node's value, in terms no document format owns.
///
/// # Why there are eight of these and not two
///
/// An undo has to put back **what was there**, and a vocabulary narrower than the document's makes
/// that impossible without inventing something. A spreadsheet cell holding the number `1.0` is not
/// the same bytes as one holding `1`; a cell holding an index into the document's string pool is not
/// the same part as one holding its own text; a cell holding `#DIV/0!` is not a cell holding the
/// characters `#DIV/0!`. Collapsing those into [`Text`](Self::Text) would make every undo a small
/// act of authoring, which is exactly the fidelity failure this project exists to prevent.
///
/// None of the eight names a format. A string pool, an exact numeric spelling and an error value are
/// ideas many document formats have; a box model that has none of them simply never produces one.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    /// No value at all. The node survives — a blank cell still carries its style, an emptied run
    /// still carries its properties.
    Empty,
    /// Text held by the node itself.
    Text(Box<str>),
    /// Text held in a document-level string pool, addressed by index.
    PooledText(u32),
    /// A number.
    Number(f64),
    /// A number with an exact spelling to preserve — `1.0`, `1e-7`, `0.30000000000000004`.
    NumberText(Box<str>),
    /// A boolean.
    Boolean(bool),
    /// An error value, spelled the way the document spells it.
    Error(Box<str>),
    /// The text result of a computation the document stores alongside its formula.
    ComputedText(Box<str>),
}

impl Value {
    /// The value that is a node's own text.
    #[must_use]
    pub fn text(text: impl Into<Box<str>>) -> Self {
        Self::Text(text.into())
    }

    /// A stable tag, for the journal encoding.
    #[must_use]
    pub(crate) const fn tag(&self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Text(_) => 1,
            Self::PooledText(_) => 2,
            Self::Number(_) => 3,
            Self::NumberText(_) => 4,
            Self::Boolean(_) => 5,
            Self::Error(_) => 6,
            Self::ComputedText(_) => 7,
        }
    }

    fn heap_bytes(&self) -> usize {
        match self {
            Self::Empty | Self::PooledText(_) | Self::Number(_) | Self::Boolean(_) => 0,
            Self::Text(text)
            | Self::NumberText(text)
            | Self::Error(text)
            | Self::ComputedText(text) => text.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_layout::{PartId, SourcePath};

    fn address() -> SourceRef {
        SourceRef::node(PartId::PRIMARY, SourcePath::new(&[0, 1]))
    }

    #[test]
    fn a_value_reports_only_what_it_holds_on_the_heap() {
        assert_eq!(Value::Empty.heap_bytes(), 0);
        assert_eq!(Value::Number(1.5).heap_bytes(), 0);
        assert_eq!(Value::PooledText(9).heap_bytes(), 0);
        assert_eq!(Value::Boolean(true).heap_bytes(), 0);
        assert_eq!(Value::text("hello").heap_bytes(), 5);
        assert_eq!(Value::NumberText("1.0".into()).heap_bytes(), 3);
        assert_eq!(Value::Error("#N/A".into()).heap_bytes(), 4);
        assert_eq!(Value::ComputedText("ok".into()).heap_bytes(), 2);
    }

    #[test]
    fn every_value_tag_is_distinct() {
        let values = [
            Value::Empty,
            Value::text(""),
            Value::PooledText(0),
            Value::Number(0.0),
            Value::NumberText("".into()),
            Value::Boolean(false),
            Value::Error("".into()),
            Value::ComputedText("".into()),
        ];
        let mut tags: Vec<u8> = values.iter().map(Value::tag).collect();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), values.len(), "two values share a wire tag");
    }

    #[test]
    fn an_operation_carries_its_address_and_its_kind() {
        let operation = Operation::set_value(address(), Value::text("hi"));
        assert_eq!(operation.address().path().segments(), [0, 1]);
        assert_eq!(operation.kind().tag(), 0);
        assert_eq!(operation.heap_bytes(), 2);

        let placed = Operation::set_bounds(address(), LayoutRect::default());
        assert_eq!(placed.kind().tag(), 1);
        assert_eq!(placed.heap_bytes(), 0);
    }
}
