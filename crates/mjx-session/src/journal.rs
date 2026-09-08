//! The journal — every operation, recorded the instant it happens, and framed so a crash can be read
//! back.
//!
//! # Layer 1 is never batched
//!
//! `docs/client-platform/SESSION_AND_PERSISTENCE.md` §1: batching the record loses work, batching the
//! apply makes the editor feel broken, and batching the commit is pure win. [`Journal::append`] is
//! layer 1. It pushes onto a `Vec` and adds up two integers; it never touches a sink, never
//! serialises a part, and never allocates beyond the operation's own payload once the `Vec` has
//! grown. `tests/journal_allocation.rs` measures that with the workspace's counting allocator rather
//! than asserting it.
//!
//! # Why the records are framed, and what a torn tail means
//!
//! A crash does not wait for a record to finish being written. So each record carries its own length
//! and a checksum over its body, and [`decode`] stops at the first record that is short or does not
//! check out, reporting how many bytes it could not use. That is the difference between a recovery
//! test that proves something and one that replays a file written by the same code that reads it:
//! the last record is *expected* to be missing or half-written, and everything before it must still
//! be there.
//!
//! The encoding is hand-written, like every other de/serialisation in this workspace, and it is
//! deliberately dull: fixed-width little-endian integers, length-prefixed text, no self-describing
//! container. A journal is written on the millisecond scale and read once, after a crash.

use mjx_layout::{PartId, SourcePath, SourceRef};
use mjx_ooxml_core::measure::Emu;

use crate::error::SessionError;
use crate::operation::{Operation, OperationKind, Value};
use crate::schedule::Timestamp;
use crate::undo::UndoUnitId;

/// The four bytes every journal starts with.
pub const MAGIC: [u8; 4] = *b"MJXJ";

/// The encoding version those four bytes introduce.
pub const VERSION: u16 = 1;

/// How deep a path a record may carry.
///
/// `SourcePath` itself has no limit — past six segments it spills to a shared allocation — so this
/// is a bound on *untrusted input* rather than on the type: a recovered journal is a file, and a
/// file that claims a four-billion-segment path must be refused rather than allocated for.
const MAXIMUM_DECODED_PATH_DEPTH: usize = 256;

/// Which of the three things an entry is.
///
/// Undo and redo are journalled as entries of their own rather than by rewinding the journal,
/// because the journal is a *history of what happened* and an undo happened. Recovery replays them
/// in order and lands where the user was.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EntryKind {
    /// An edit the caller made.
    Edit,
    /// One step of an undo.
    Undo,
    /// One step of a redo.
    Redo,
}

impl EntryKind {
    const fn tag(self) -> u8 {
        match self {
            Self::Edit => 0,
            Self::Undo => 1,
            Self::Redo => 2,
        }
    }

    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Edit),
            1 => Some(Self::Undo),
            2 => Some(Self::Redo),
            _ => None,
        }
    }
}

/// A monotonically increasing number for one recorded operation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OperationId(u64);

impl OperationId {
    /// The operation numbered `number`.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// One recorded operation, with everything needed to undo it and to replay it.
#[derive(Clone, PartialEq, Debug)]
pub struct JournalEntry {
    /// Which operation this is, in the order they happened.
    pub id: OperationId,
    /// Which undo unit it belongs to. **Independent of the commit window** — see [`crate::undo`].
    pub unit: UndoUnitId,
    /// Whether it was an edit, an undo or a redo.
    pub kind: EntryKind,
    /// When it happened.
    pub at: Timestamp,
    /// What was done.
    pub operation: Operation,
    /// What undoes it.
    pub inverse: Operation,
}

impl JournalEntry {
    /// Bytes this entry holds on the heap.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.operation.heap_bytes() + self.inverse.heap_bytes()
    }
}

/// Everything recorded since the last commit, and how much of it has reached the sink.
#[derive(Debug, Default)]
pub struct Journal {
    entries: Vec<JournalEntry>,
    /// The index of the first entry not yet handed to the sink. Everything before it is durable once
    /// the sink's own flush returned.
    flushed: usize,
    /// Whether the sink has already been given this journal file's header. It is written once per
    /// *file*, so a commit — which truncates the sink — puts it back to `false`.
    header_written: bool,
    heap_bytes: usize,
    next_id: u64,
}

impl Journal {
    /// An empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty journal with room for `operations` before it grows.
    ///
    /// The record path's budget is *allocation-free in the common case*, and a `Vec` that has to
    /// grow is the one allocation in it. A host that knows a burst is coming — a paste, a replay —
    /// reserves once here rather than paying a doubling in the middle of one.
    #[must_use]
    pub fn with_capacity(operations: usize) -> Self {
        Self {
            entries: Vec::with_capacity(operations),
            ..Self::default()
        }
    }

    /// Appends an operation and its inverse, and returns the entry's number.
    ///
    /// This is layer 1: it does not touch a sink and does not serialise anything.
    pub fn append(
        &mut self,
        kind: EntryKind,
        unit: UndoUnitId,
        at: Timestamp,
        operation: Operation,
        inverse: Operation,
    ) -> OperationId {
        let id = OperationId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let entry = JournalEntry {
            id,
            unit,
            kind,
            at,
            operation,
            inverse,
        };
        self.heap_bytes = self.heap_bytes.saturating_add(entry.heap_bytes());
        self.entries.push(entry);
        id
    }

    /// Everything recorded since the last commit.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// How many operations are recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Bytes the recorded operations hold on the heap — the number the memory bound is read against.
    #[must_use]
    pub const fn heap_bytes(&self) -> usize {
        self.heap_bytes
    }

    /// How many entries have not yet been handed to the sink — the **exposure window**, in
    /// operations.
    #[must_use]
    pub fn unflushed(&self) -> usize {
        self.entries.len() - self.flushed
    }

    /// Encodes everything not yet flushed into `into`, preceded by the file header if this is the
    /// first flush since the journal was last truncated, and marks it flushed.
    ///
    /// The caller is responsible for the sink actually accepting the bytes; this reports what it
    /// encoded so a refused write can be retried through [`retract_flush`](Self::retract_flush)
    /// without losing anything. `into` is a caller-owned buffer, so a session flushing every 250 ms
    /// reuses one allocation for the life of the document.
    pub fn encode_unflushed(&mut self, into: &mut Vec<u8>) -> usize {
        let count = self.unflushed();
        if count == 0 {
            return 0;
        }
        if !self.header_written {
            into.extend_from_slice(&MAGIC);
            into.extend_from_slice(&VERSION.to_le_bytes());
            self.header_written = true;
        }
        for entry in &self.entries[self.flushed..] {
            encode_entry(entry, into);
        }
        self.flushed = self.entries.len();
        count
    }

    /// Un-marks `count` entries as flushed, because the sink refused the write.
    ///
    /// If that takes the mark back to nothing, the header is un-marked too: a sink that accepted no
    /// bytes has no header either, and a journal file whose first record arrives without one cannot
    /// be read back.
    pub fn retract_flush(&mut self, count: usize) {
        self.flushed = self.flushed.saturating_sub(count);
        if self.flushed == 0 {
            self.header_written = false;
        }
    }

    /// Whether the next flush will write this journal file's header.
    #[must_use]
    pub const fn wants_header(&self) -> bool {
        !self.header_written
    }

    /// Forgets everything, because the document it described has been committed.
    ///
    /// Operation numbers keep counting: they number what happened, not what is held.
    pub fn truncate(&mut self) {
        self.entries.clear();
        self.flushed = 0;
        self.header_written = false;
        self.heap_bytes = 0;
    }
}

/// The four-byte magic and the version, which a journal file starts with.
#[must_use]
pub fn header() -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes
}

/// Encodes one entry as a framed record: a length, a body, and a checksum over the body.
pub fn encode_entry(entry: &JournalEntry, into: &mut Vec<u8>) {
    let start = into.len();
    into.extend_from_slice(&0_u32.to_le_bytes());
    let body_start = into.len();

    into.push(entry.kind.tag());
    into.extend_from_slice(&entry.id.0.to_le_bytes());
    into.extend_from_slice(&entry.unit.value().to_le_bytes());
    into.extend_from_slice(&entry.at.millis().to_le_bytes());
    encode_operation(&entry.operation, into);
    encode_operation(&entry.inverse, into);

    let body_length = into.len() - body_start;
    // The body is bounded by one operation's payload, which the record path caps far below `u32`.
    let stated = u32::try_from(body_length).unwrap_or(u32::MAX);
    into[start..body_start].copy_from_slice(&stated.to_le_bytes());
    let checksum = checksum(&into[body_start..]);
    into.extend_from_slice(&checksum.to_le_bytes());
}

fn encode_operation(operation: &Operation, into: &mut Vec<u8>) {
    let address = operation.address();
    into.extend_from_slice(&address.part().number().to_le_bytes());
    let segments = address.path().segments();
    // Capped on the way in as well as on the way out: a path deeper than this cannot be encoded, and
    // truncating one silently would move the edit somewhere else.
    let depth = segments.len().min(MAXIMUM_DECODED_PATH_DEPTH);
    // At most 256, so it needs two bytes rather than one; the `min` above is what makes the
    // conversion total.
    into.extend_from_slice(&u16::try_from(depth).unwrap_or(u16::MAX).to_le_bytes());
    for segment in &segments[..depth] {
        into.extend_from_slice(&segment.to_le_bytes());
    }
    let characters = address.characters();
    into.extend_from_slice(&characters.start.to_le_bytes());
    into.extend_from_slice(&characters.end.to_le_bytes());

    into.push(operation.kind().tag());
    match operation.kind() {
        OperationKind::SetValue(value) => encode_value(value, into),
        OperationKind::SetBounds(bounds) => {
            for edge in [bounds.left, bounds.top, bounds.right, bounds.bottom] {
                into.extend_from_slice(&edge.emu().to_le_bytes());
            }
        }
    }
}

fn encode_value(value: &Value, into: &mut Vec<u8>) {
    into.push(value.tag());
    match value {
        Value::Empty => {}
        Value::Text(text)
        | Value::NumberText(text)
        | Value::Error(text)
        | Value::ComputedText(text) => {
            let bytes = text.as_bytes();
            let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
            into.extend_from_slice(&length.to_le_bytes());
            into.extend_from_slice(&bytes[..length as usize]);
        }
        Value::PooledText(index) => into.extend_from_slice(&index.to_le_bytes()),
        Value::Number(number) => into.extend_from_slice(&number.to_bits().to_le_bytes()),
        Value::Boolean(flag) => into.push(u8::from(*flag)),
    }
}

/// What [`decode`] made of a journal's bytes.
#[derive(Clone, PartialEq, Debug)]
pub struct DecodedJournal {
    /// Every record that was whole and checked out, in order.
    pub entries: Vec<JournalEntry>,
    /// How many trailing bytes could not be read — a record the crash caught mid-write.
    ///
    /// Non-zero is **not** an error. It is the expected shape of a journal whose process was killed,
    /// and it is the number that distinguishes "the last operation did not reach the disk" from "the
    /// file is rubbish".
    pub torn_tail_bytes: usize,
}

/// Reads a journal back.
///
/// # Errors
/// [`SessionError::UnreadableJournal`] if the file does not begin with this encoding's magic and
/// version. A truncated or corrupt *record* is not an error — see
/// [`torn_tail_bytes`](DecodedJournal::torn_tail_bytes).
pub fn decode(bytes: &[u8]) -> Result<DecodedJournal, SessionError> {
    let Some(rest) = bytes.strip_prefix(&MAGIC) else {
        return Err(SessionError::UnreadableJournal {
            reason: "it does not begin with the journal magic",
        });
    };
    let Some((version, after_header)) = rest.split_first_chunk::<2>() else {
        return Err(SessionError::UnreadableJournal {
            reason: "it ends before its version",
        });
    };
    if u16::from_le_bytes(*version) != VERSION {
        return Err(SessionError::UnreadableJournal {
            reason: "it states a version this build does not write",
        });
    }
    let mut rest = after_header;

    let mut entries = Vec::new();
    loop {
        let Some(consumed) = read_record(rest, &mut entries) else {
            return Ok(DecodedJournal {
                entries,
                torn_tail_bytes: rest.len(),
            });
        };
        rest = &rest[consumed..];
    }
}

/// Reads one framed record, appending it, and returns how many bytes it consumed — or `None` when
/// the remaining bytes are not a whole, checked record.
fn read_record(bytes: &[u8], into: &mut Vec<JournalEntry>) -> Option<usize> {
    let (length, rest) = bytes.split_first_chunk::<4>()?;
    let body_length = u32::from_le_bytes(*length) as usize;
    if rest.len() < body_length.checked_add(4)? {
        return None;
    }
    let body = &rest[..body_length];
    let stated = u32::from_le_bytes(rest[body_length..body_length + 4].try_into().ok()?);
    if checksum(body) != stated {
        return None;
    }
    let entry = read_entry(body)?;
    into.push(entry);
    Some(4 + body_length + 4)
}

fn read_entry(body: &[u8]) -> Option<JournalEntry> {
    let mut reader = Reader::new(body);
    let kind = EntryKind::from_tag(reader.u8()?)?;
    let id = OperationId(reader.u64()?);
    let unit = UndoUnitId::new(reader.u64()?);
    let at = Timestamp::from_millis(reader.u64()?);
    let operation = read_operation(&mut reader)?;
    let inverse = read_operation(&mut reader)?;
    // A record with bytes left over is a record this build does not understand.
    reader.at_end().then_some(JournalEntry {
        id,
        unit,
        kind,
        at,
        operation,
        inverse,
    })
}

fn read_operation(reader: &mut Reader<'_>) -> Option<Operation> {
    let part = PartId::new(reader.u32()?);
    let depth = usize::from(reader.u16()?);
    if depth > MAXIMUM_DECODED_PATH_DEPTH {
        return None;
    }
    let mut segments = Vec::with_capacity(depth);
    for _ in 0..depth {
        segments.push(reader.u32()?);
    }
    let start = reader.u32()?;
    let end = reader.u32()?;
    let address = SourceRef::new(part, SourcePath::new(&segments), start..end);
    match reader.u8()? {
        0 => Some(Operation::set_value(address, read_value(reader)?)),
        1 => {
            let left = Emu::from_emu(reader.i64()?);
            let top = Emu::from_emu(reader.i64()?);
            let right = Emu::from_emu(reader.i64()?);
            let bottom = Emu::from_emu(reader.i64()?);
            Some(Operation::set_bounds(
                address,
                mjx_layout::LayoutRect::from_edges(left, top, right, bottom),
            ))
        }
        _ => None,
    }
}

fn read_value(reader: &mut Reader<'_>) -> Option<Value> {
    match reader.u8()? {
        0 => Some(Value::Empty),
        1 => Some(Value::Text(reader.text()?)),
        2 => Some(Value::PooledText(reader.u32()?)),
        3 => Some(Value::Number(f64::from_bits(reader.u64()?))),
        4 => Some(Value::NumberText(reader.text()?)),
        5 => Some(Value::Boolean(reader.u8()? != 0)),
        6 => Some(Value::Error(reader.text()?)),
        7 => Some(Value::ComputedText(reader.text()?)),
        _ => None,
    }
}

/// A bounds-checked cursor. Every read returns `None` rather than panicking, because the bytes came
/// off a disk after a crash and are untrusted input like any file this workspace reads.
struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn at_end(&self) -> bool {
        self.bytes.is_empty()
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        if self.bytes.len() < count {
            return None;
        }
        let (head, tail) = self.bytes.split_at(count);
        self.bytes = tail;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|it| it[0])
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2)
            .and_then(|it| it.try_into().ok())
            .map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .and_then(|it| it.try_into().ok())
            .map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8)
            .and_then(|it| it.try_into().ok())
            .map(u64::from_le_bytes)
    }

    fn i64(&mut self) -> Option<i64> {
        self.u64().map(|it| it as i64)
    }

    /// A length-prefixed string.
    ///
    /// No separate cap on the length is needed and none is imposed: [`take`](Self::take) refuses
    /// before it reads, so a corrupt length can only make this return `None`. The string is built
    /// from a slice that already exists, so nothing is allocated for a length that is not there —
    /// which is the property a cap would have been standing in for.
    fn text(&mut self) -> Option<Box<str>> {
        let length = self.u32()? as usize;
        let bytes = self.take(length)?;
        core::str::from_utf8(bytes).ok().map(Box::from)
    }
}

/// FNV-1a, 32-bit.
///
/// A checksum here answers one question — *did this record reach the disk whole* — and the failure
/// it guards against is a torn write, not an adversary. FNV is four lines, has no table and no
/// dependency, and catches a truncated or partially-overwritten record every time.
fn checksum(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_layout::LayoutRect;

    fn address(segments: &[u32]) -> SourceRef {
        SourceRef::new(PartId::new(7), SourcePath::new(segments), 3..9)
    }

    fn entry(id: u64, operation: Operation, inverse: Operation) -> JournalEntry {
        JournalEntry {
            id: OperationId(id),
            unit: UndoUnitId::new(id / 2),
            kind: EntryKind::Edit,
            at: Timestamp::from_millis(id * 17),
            operation,
            inverse,
        }
    }

    /// Every value shape, every entry kind, a shallow path and a path past the inline depth.
    fn every_shape() -> Vec<JournalEntry> {
        let values = [
            Value::Empty,
            Value::text("hello \u{1f600}"),
            Value::PooledText(4_000_000_000),
            Value::Number(-0.5),
            Value::NumberText("0.30000000000000004".into()),
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Error("#DIV/0!".into()),
            Value::ComputedText("42".into()),
        ];
        let mut entries: Vec<JournalEntry> = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                entry(
                    index as u64,
                    Operation::set_value(address(&[1, 2, 3]), value),
                    Operation::set_value(address(&[1, 2, 3]), Value::Empty),
                )
            })
            .collect();
        entries.push(entry(
            100,
            Operation::set_bounds(
                address(&[1, 2, 3, 4, 5, 6, 7, 8]),
                LayoutRect::from_edges(
                    Emu::from_emu(-5),
                    Emu::from_emu(0),
                    Emu::from_emu(914_400),
                    Emu::from_emu(1_828_800),
                ),
            ),
            Operation::set_bounds(address(&[]), LayoutRect::ZERO),
        ));
        let mut undone = entries[0].clone();
        undone.kind = EntryKind::Undo;
        entries.push(undone);
        let mut redone = entries[0].clone();
        redone.kind = EntryKind::Redo;
        entries.push(redone);
        entries
    }

    fn encode_all(entries: &[JournalEntry]) -> Vec<u8> {
        let mut bytes = header();
        for entry in entries {
            encode_entry(entry, &mut bytes);
        }
        bytes
    }

    #[test]
    fn every_shape_survives_a_round_trip() {
        let entries = every_shape();
        let decoded = decode(&encode_all(&entries)).expect("a readable journal");
        assert_eq!(decoded.torn_tail_bytes, 0);
        assert_eq!(decoded.entries, entries);
    }

    #[test]
    fn a_journal_with_no_records_decodes_to_nothing() {
        let decoded = decode(&header()).expect("a readable journal");
        assert!(decoded.entries.is_empty());
        assert_eq!(decoded.torn_tail_bytes, 0);
    }

    #[test]
    fn a_torn_final_record_costs_only_that_record() {
        let entries = every_shape();
        let whole = encode_all(&entries);
        // Cut the file at every byte inside the last record and check that everything before it
        // survives, whatever the cut lands in the middle of.
        let last_record_start = {
            let mut without_last = header();
            for entry in &entries[..entries.len() - 1] {
                encode_entry(entry, &mut without_last);
            }
            without_last.len()
        };
        for cut in last_record_start..whole.len() {
            let decoded = decode(&whole[..cut]).expect("a readable journal");
            assert_eq!(
                decoded.entries,
                entries[..entries.len() - 1],
                "cut at {cut}"
            );
            assert_eq!(decoded.torn_tail_bytes, cut - last_record_start);
        }
    }

    #[test]
    fn a_flipped_byte_in_a_record_stops_the_read_there() {
        let entries = every_shape();
        let mut bytes = encode_all(&entries);
        // The first record's body starts after the six-byte header and the four-byte length.
        bytes[11] ^= 0xff;
        let decoded = decode(&bytes).expect("a readable journal");
        assert!(decoded.entries.is_empty());
        assert!(decoded.torn_tail_bytes > 0);
    }

    #[test]
    fn a_file_that_is_not_a_journal_is_refused_rather_than_read() {
        assert!(matches!(
            decode(b""),
            Err(SessionError::UnreadableJournal { .. })
        ));
        assert!(matches!(
            decode(b"MJX"),
            Err(SessionError::UnreadableJournal { .. })
        ));
        assert!(matches!(
            decode(b"NOPE\x01\x00"),
            Err(SessionError::UnreadableJournal { .. })
        ));
        assert!(matches!(
            decode(b"MJXJ"),
            Err(SessionError::UnreadableJournal { .. })
        ));
        assert!(matches!(
            decode(b"MJXJ\x63\x00"),
            Err(SessionError::UnreadableJournal { .. })
        ));
    }

    #[test]
    fn a_record_claiming_an_absurd_length_is_refused_and_not_allocated_for() {
        let mut bytes = header();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(b"short");
        let decoded = decode(&bytes).expect("a readable journal");
        assert!(decoded.entries.is_empty());
        assert_eq!(decoded.torn_tail_bytes, 9);
    }

    #[test]
    fn a_record_with_bytes_left_over_is_refused() {
        let entries = every_shape();
        let mut body = Vec::new();
        encode_entry(&entries[0], &mut body);
        // Splice one extra byte into the body and restate the length and checksum around it.
        let body_length = u32::from_le_bytes(body[..4].try_into().expect("four bytes")) as usize;
        let mut extended = body[4..4 + body_length].to_vec();
        extended.push(0);
        let mut bytes = header();
        bytes.extend_from_slice(&(extended.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&extended);
        bytes.extend_from_slice(&checksum(&extended).to_le_bytes());
        let decoded = decode(&bytes).expect("a readable journal");
        assert!(decoded.entries.is_empty());
    }

    #[test]
    fn the_journal_counts_bytes_and_forgets_them_on_truncate() {
        let mut journal = Journal::with_capacity(4);
        assert!(journal.is_empty());
        assert!(journal.wants_header());
        let first = journal.append(
            EntryKind::Edit,
            UndoUnitId::new(0),
            Timestamp::ORIGIN,
            Operation::set_value(address(&[0]), Value::text("abcde")),
            Operation::set_value(address(&[0]), Value::Empty),
        );
        assert_eq!(first.value(), 0);
        assert!(journal.wants_header(), "nothing has been flushed yet");
        assert_eq!(journal.heap_bytes(), 5);
        assert_eq!(journal.len(), 1);
        assert_eq!(journal.unflushed(), 1);

        let second = journal.append(
            EntryKind::Edit,
            UndoUnitId::new(0),
            Timestamp::ORIGIN,
            Operation::set_value(address(&[0]), Value::text("fg")),
            Operation::set_value(address(&[0]), Value::text("abcde")),
        );
        assert_eq!(second.value(), 1);
        assert_eq!(journal.heap_bytes(), 5 + 2 + 5);

        journal.truncate();
        assert!(journal.is_empty());
        assert_eq!(journal.heap_bytes(), 0);
        assert_eq!(journal.unflushed(), 0);
        // Ids keep counting across a commit: they number what happened, not what is held.
        let third = journal.append(
            EntryKind::Edit,
            UndoUnitId::new(1),
            Timestamp::ORIGIN,
            Operation::set_value(address(&[0]), Value::Empty),
            Operation::set_value(address(&[0]), Value::Empty),
        );
        assert_eq!(third.value(), 2);
    }

    #[test]
    fn encoding_the_unflushed_tail_moves_the_mark_and_a_refusal_moves_it_back() {
        let mut journal = Journal::new();
        for index in 0..3_u32 {
            journal.append(
                EntryKind::Edit,
                UndoUnitId::new(0),
                Timestamp::ORIGIN,
                Operation::set_value(address(&[index]), Value::Number(f64::from(index))),
                Operation::set_value(address(&[index]), Value::Empty),
            );
        }
        let mut buffer = Vec::new();
        assert!(journal.wants_header());
        assert_eq!(journal.encode_unflushed(&mut buffer), 3);
        assert!(
            !journal.wants_header(),
            "the header is written once per file"
        );
        assert_eq!(journal.unflushed(), 0);
        assert_eq!(
            journal.encode_unflushed(&mut buffer),
            0,
            "a flush with nothing to flush writes nothing"
        );

        let decoded = decode(&buffer).expect("a readable journal");
        assert_eq!(decoded.entries.len(), 3);

        // A refused write puts the mark — and the header — back, so the next attempt rewrites the
        // whole file rather than a headerless tail of it.
        journal.retract_flush(3);
        assert_eq!(journal.unflushed(), 3);
        assert!(journal.wants_header());
        let mut retried = Vec::new();
        assert_eq!(journal.encode_unflushed(&mut retried), 3);
        assert_eq!(retried, buffer);
    }

    #[test]
    fn a_commit_truncates_the_journal_and_the_next_file_gets_its_own_header() {
        let mut journal = Journal::new();
        journal.append(
            EntryKind::Edit,
            UndoUnitId::new(0),
            Timestamp::ORIGIN,
            Operation::set_value(address(&[0]), Value::Empty),
            Operation::set_value(address(&[0]), Value::Empty),
        );
        let mut first = Vec::new();
        journal.encode_unflushed(&mut first);
        assert!(!journal.wants_header());

        journal.truncate();
        assert!(journal.wants_header(), "the sink was truncated too");
        journal.append(
            EntryKind::Edit,
            UndoUnitId::new(1),
            Timestamp::ORIGIN,
            Operation::set_value(address(&[1]), Value::Empty),
            Operation::set_value(address(&[1]), Value::Empty),
        );
        let mut second = Vec::new();
        journal.encode_unflushed(&mut second);
        assert!(second.starts_with(&MAGIC));
        assert_eq!(
            decode(&second).expect("a readable journal").entries.len(),
            1
        );
    }
}
