//! **The shared-string table's memory gate (MJXOFF-97).** One binary, one `main`, one thread, one
//! counting global allocator — and two figures: a per-entry byte bound, and the property that
//! actually separates this design from the obvious one.
//!
//! # Why this is a second target rather than a case in `cell_store_allocation.rs`
//!
//! A `#[global_allocator]` is installed for a whole process, and a peak measured beside other work
//! is that other work's peak. MJXOFF-95's gate solved that with `harness = false` and one `main`;
//! adding a case to it would put two independent measurements back into one process, where the
//! second one's zero depends on what the first one left live. Two binaries are two processes, each
//! with its own allocator and its own zero, which is what makes either figure mean anything.
//!
//! # The bound alone would not be a gate, and this is the interesting part
//!
//! The obvious design is `Vec<String>`: one `String` per entry, each owning its text. Against
//! *short* strings that costs 24 bytes of header plus the text — which for a twelve-character entry
//! is **less** than this table's 48-byte record. A bytes-per-entry bound measured on short strings
//! would therefore pass a design this one exists to reject, which is precisely the shape of gate
//! this project keeps having to remediate.
//!
//! So the load-bearing assertion here is not the bound. It is that **the table costs the same for
//! long strings as for short ones**: two tables with the same number of entries and text differing
//! by an order of magnitude in length retain the same bytes, to the byte. An entry that holds a
//! `(start, length)` pair into the part's own buffer has that property; an entry that owns its text
//! cannot have it at any string length. `case 3` measures it.
//!
//! The bound is still worth having, for the other half: it is what catches a table that reserved for
//! something other than its entries.
//!
//! # What is measured, and what is deliberately not
//!
//! The part's own bytes are allocated **before** the measurement starts and handed to the table
//! shared, because that is what actually happens: `mjx-opc` already holds a part's buffer for its own
//! copy-on-write, and this table points into it rather than duplicating it. A `Vec<String>` design
//! would hold the process's **second** copy of every string in the workbook; that cost is invisible
//! to the figures below and is the larger half of the argument.

use std::sync::Arc;

use mjx_sml::SharedStringTable;

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// How many entries the measured tables hold.
///
/// **A power of two on purpose.** `Vec` grows by doubling, so a length that is not a power of two
/// leaves the vector holding up to twice the capacity it needs and the measured per-entry figure
/// becomes a fact about `Vec`'s growth policy rather than about the record. At exactly 65,536
/// pushes the capacity is exactly 65,536 and the figure is the record's own size.
///
/// It is also a realistic order: Excel's ceiling is 1,048,576 unique strings, and a text-heavy
/// workbook reaches tens of thousands.
const ENTRY_COUNT: usize = 65_536;

/// The bound on what the table retains per entry, in bytes.
///
/// `PackedStringItem` is 48 bytes by design, and `strings/record.rs` asserts that size separately so
/// that a field added without a decision fails there rather than here. The eight bytes of slack are
/// for the three other vectors (runs, phonetic runs, side records), all empty for a table of plain
/// entries, and for the table's own fixed cost.
const BYTES_PER_ENTRY_BOUND: usize = 56;

/// The bound on a table holding exactly one entry.
///
/// Generous on purpose: it is not a regression bound on the exact figure, it is the line between
/// "this table costs what its entries cost" and "this table costs what its *part* costs".
const SINGLE_ENTRY_BOUND: usize = 8 * 1024;

fn main() {
    println!("MJXOFF-97 — the shared-string table's memory gate\n");
    let one = a_table_of_one_entry_costs_what_one_entry_costs();
    let many = a_realistic_table_costs_far_less_than_the_tree_it_was_read_from();
    let (short, long) = an_entry_costs_the_same_whatever_its_text_says();
    let untouched = an_untouched_table_owns_no_bytes_of_its_own();
    println!("\nall four cases passed");
    // Keep the figures alive to the end of `main`, so nothing above can be optimised away on the
    // strength of the tables being dropped early.
    assert!(one > 0 && many > 0 && short > 0 && long > 0 && untouched == 0);
}

/// A table with one entry holds one record, not a copy of the part.
fn a_table_of_one_entry_costs_what_one_entry_costs() -> usize {
    let markup: Arc<[u8]> = Arc::from(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1"><si><t>only</t></si></sst>"#
            .to_vec(),
    );
    let before = mjx_allocation_counter::reset_peak();
    let document = mjx_xml::fidelity::parse_shared(Arc::clone(&markup)).expect("the table parses");
    let table = SharedStringTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an sst");
    drop(document);
    let live = mjx_allocation_counter::live() - before;
    let peak = mjx_allocation_counter::peak() - before;

    println!("case 1 — one entry");
    println!(
        "  raw XML                        {:>12} bytes",
        markup.len()
    );
    println!("  table, live                    {live:>12} bytes");
    println!("  peak across parse + read       {peak:>12} bytes");
    println!("  bound                          {SINGLE_ENTRY_BOUND:>12} bytes");

    assert_eq!(table.len(), 1);
    assert_eq!(
        table.item(0).expect("the entry").text().expect("decodes"),
        "only",
        "the bound was not met by failing to read anything"
    );
    assert!(
        live <= SINGLE_ENTRY_BOUND,
        "a one-entry table retains {live} bytes, over the {SINGLE_ENTRY_BOUND}-byte bound"
    );
    live
}

/// A table the size a real workbook carries, next to the tree it was read from.
fn a_realistic_table_costs_far_less_than_the_tree_it_was_read_from() -> usize {
    let markup: Arc<[u8]> = Arc::from(table_markup(SHORT_ENTRY).into_bytes());

    // The buffer is allocated *before* the measurement and handed over shared, so what follows is
    // the table's own cost rather than a second copy of the part.
    let before = mjx_allocation_counter::reset_peak();
    let document = mjx_xml::fidelity::parse_shared(Arc::clone(&markup)).expect("the table parses");
    let tree_live = mjx_allocation_counter::live() - before;
    let tree_peak = mjx_allocation_counter::peak() - before;

    let table = SharedStringTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an sst");
    drop(document);
    let live = mjx_allocation_counter::live() - before;
    let peak = mjx_allocation_counter::peak() - before;

    println!("\ncase 2 — {ENTRY_COUNT} entries");
    println!(
        "  raw XML                        {:>12} bytes",
        markup.len()
    );
    println!(
        "  RawElement tree, live          {tree_live:>12} bytes  ({:.0} B/entry)",
        tree_live as f64 / ENTRY_COUNT as f64
    );
    println!("  RawElement tree, peak          {tree_peak:>12} bytes");
    println!(
        "  string table, live             {live:>12} bytes  ({:.1} B/entry)",
        live as f64 / ENTRY_COUNT as f64
    );
    println!("  peak across parse + read       {peak:>12} bytes");
    println!("  bound                          {BYTES_PER_ENTRY_BOUND:>12} B/entry");

    assert_eq!(table.len(), ENTRY_COUNT);
    assert_eq!(
        table.edited_bytes(),
        0,
        "a table nobody has edited must own no bytes of its own"
    );
    // Read back through the generator's own formula, so the expectation is the input rather than a
    // value copied out of a passing run.
    let probe = ENTRY_COUNT / 3;
    assert_eq!(
        table
            .item(probe as u32)
            .expect("the entry is there")
            .text()
            .expect("decodes"),
        entry_text(probe, SHORT_ENTRY),
        "the figure was not met by dropping the contents"
    );

    let per_entry = live / ENTRY_COUNT;
    assert!(
        per_entry <= BYTES_PER_ENTRY_BOUND,
        "the table retains {live} bytes for {ENTRY_COUNT} entries — {per_entry} B/entry, over the \
         {BYTES_PER_ENTRY_BOUND} B/entry bound"
    );
    assert!(
        live < tree_live,
        "the table ({live} bytes) must cost less than the tree it was read from ({tree_live} bytes)"
    );
    live
}

/// **The discriminating case.** The table costs the same whether its strings are short or long.
///
/// This is the assertion a `Vec<String>` cannot pass at any string length, and the reason the
/// bytes-per-entry bound above is not on its own a gate: against twelve-character entries, a
/// `String` per entry costs *less* than a 48-byte record.
fn an_entry_costs_the_same_whatever_its_text_says() -> (usize, usize) {
    let short = table_live_bytes(SHORT_ENTRY);
    let long = table_live_bytes(LONG_ENTRY);

    println!("\ncase 3 — the same entries, text an order of magnitude longer");
    println!(
        "  short entries ({SHORT_ENTRY:>2} chars)        {short:>12} bytes  ({:.1} B/entry)",
        short as f64 / ENTRY_COUNT as f64
    );
    println!(
        "  long entries  ({LONG_ENTRY:>3} chars)       {long:>12} bytes  ({:.1} B/entry)",
        long as f64 / ENTRY_COUNT as f64
    );
    println!(
        "  a String per entry would differ by at least {:>7} bytes",
        ENTRY_COUNT * (LONG_ENTRY - SHORT_ENTRY)
    );

    assert_eq!(
        short, long,
        "the table retained {short} bytes for short text and {long} for text {}x longer. An entry \
         holds a (start, length) pair into the part's own buffer, so its cost cannot depend on what \
         the string says — a design that owns its text fails here at every string length, including \
         the lengths at which it would pass a bytes-per-entry bound.",
        LONG_ENTRY / SHORT_ENTRY
    );
    (short, long)
}

/// The copy-on-write rule as a number: a table nobody has touched authors nothing.
fn an_untouched_table_owns_no_bytes_of_its_own() -> usize {
    let markup: Arc<[u8]> = Arc::from(table_markup(SHORT_ENTRY).into_bytes());
    let document = mjx_xml::fidelity::parse_shared(Arc::clone(&markup)).expect("the table parses");
    let mut table = SharedStringTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an sst");
    drop(document);

    let untouched = table.edited_bytes();
    assert_eq!(untouched, 0);
    assert_eq!(
        table.to_part_bytes(),
        markup.to_vec(),
        "and it re-emits the part it never edited"
    );

    // One edit, and only the edited entry's bytes are authored.
    table.set_text(1, "edited").expect("edits");
    let after = table.edited_bytes();
    println!("\ncase 4 — copy-on-write");
    println!("  bytes authored, untouched      {untouched:>12}");
    println!("  bytes authored after one edit  {after:>12}");
    assert!(
        after > 0 && after < 256,
        "one edit must author one entry's worth of bytes, not the part's: {after}"
    );
    untouched
}

/// What the table retains for `ENTRY_COUNT` entries whose text is `length` characters long.
fn table_live_bytes(length: usize) -> usize {
    let markup: Arc<[u8]> = Arc::from(table_markup(length).into_bytes());
    let before = mjx_allocation_counter::reset_peak();
    let document = mjx_xml::fidelity::parse_shared(Arc::clone(&markup)).expect("the table parses");
    let table = SharedStringTable::read_part(&document)
        .expect("the table reads")
        .expect("the root is an sst");
    drop(document);
    let live = mjx_allocation_counter::live() - before;
    assert_eq!(table.len(), ENTRY_COUNT);
    assert_eq!(
        table
            .item(7)
            .expect("the entry is there")
            .text()
            .expect("decodes")
            .len(),
        length,
        "the text really is {length} bytes long, so the figure means what it says"
    );
    drop(table);
    live
}

/// The short entry length, in characters — the order a `Vec<String>` looks *cheaper* at.
const SHORT_ENTRY: usize = 12;
/// The long entry length, in characters.
const LONG_ENTRY: usize = 120;

/// A `sharedStrings.xml` in the shape a text-heavy workbook carries: plain entries, no whitespace
/// between them, `count` and `uniqueCount` written.
fn table_markup(length: usize) -> String {
    let mut xml = String::with_capacity(ENTRY_COUNT * (length + 16) + 256);
    xml.push_str(&format!(
        "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
         count=\"{ENTRY_COUNT}\" uniqueCount=\"{ENTRY_COUNT}\">"
    ));
    for index in 0..ENTRY_COUNT {
        xml.push_str("<si><t>");
        xml.push_str(&entry_text(index, length));
        xml.push_str("</t></si>");
    }
    xml.push_str("</sst>");
    xml
}

/// The text of entry `index`, `length` ASCII characters long — one function the generator and the
/// assertions both read, so no expectation here is a value copied out of a passing run.
fn entry_text(index: usize, length: usize) -> String {
    let mut text = format!("Cust {index:06} ");
    while text.len() < length {
        text.push(char::from(b'a' + (text.len() % 26) as u8));
    }
    text.truncate(length);
    text
}
