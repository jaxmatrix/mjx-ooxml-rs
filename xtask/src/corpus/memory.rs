//! Peak resident set size, read from the kernel rather than counted by the allocator (MJXOFF-147).
//!
//! MJXOFF-146 already has a pure-Rust answer for "how many bytes has the app asked the allocator
//! for" (`xtask::fuzz::allocation::Counting`, an `unsafe impl GlobalAlloc` that forwards to
//! `std::alloc::System`). That is a different question from the one this ticket asks: "**peak
//! resident set**" is `/proc/self/status`'s own term (`VmHWM`, "high water mark"), and it is the
//! figure `getrusage`/`/usr/bin/time -v` report too — the kernel's account of pages actually backed
//! by RAM, which is larger than "bytes the allocator handed out" by whatever the allocator holds
//! but has not returned to the OS, and *can* be smaller than a large `Vec::with_capacity` whose
//! pages were reserved but never touched. A counting allocator cannot see either effect; the kernel
//! can, because it is the thing deciding residency. So this reads the kernel's own number instead of
//! re-deriving an approximation of it — no `unsafe`, no new dependency, Linux-only (this is
//! `xtask`: host-only developer tooling, same as the rest of this crate).
//!
//! `cargo run -p xtask -- corpus --mem <pptx|docx|xlsx>` measures **one format per process**, on
//! purpose: `VmHWM` is a high-water mark *since process start*, so the only way to attribute a peak
//! to one format is to give it a process with nothing else in its history (running all three in one
//! process would let an earlier, larger scenario's peak leak into a later, smaller one's reading).
//! Within that one process, the four checkpoints (open / first-mutation / edit / save) are read in
//! sequence as each operation completes, so each is the cumulative peak reached by every checkpoint
//! up to and including itself — which is the right thing to report, since a real caller's memory use
//! is exactly that running total, not each stage in isolation.

use anyhow::{bail, Context, Result};

/// This process's peak resident set size so far (`VmHWM`), in KiB.
///
/// # Errors
/// Returns an error if `/proc/self/status` cannot be read or carries no `VmHWM` line (not running
/// on Linux).
pub fn peak_rss_kib() -> Result<u64> {
    let status =
        std::fs::read_to_string("/proc/self/status").context("reading /proc/self/status")?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let digits = rest.split_whitespace().next().unwrap_or_default();
            return digits
                .parse::<u64>()
                .with_context(|| format!("parsing VmHWM value {digits:?}"));
        }
    }
    bail!("no VmHWM line in /proc/self/status — this tool is Linux-only")
}

/// One `(label, peak-RSS-so-far)` reading, printed as a running table.
pub fn checkpoint(label: &str) -> Result<()> {
    let kib = peak_rss_kib()?;
    println!("  {label:<32} {kib:>10} KiB peak RSS so far");
    Ok(())
}
