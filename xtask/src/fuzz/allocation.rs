//! The campaign's memory ceiling: a counting global allocator.
//!
//! **Why this exists.** An input that makes a reader allocate without bound is a defect even when it
//! never panics, and the failure mode without instrumentation is the worst one available: the kernel
//! kills the process, the campaign log ends mid-line, and the operator reads it as a hang. Measuring
//! allocation directly turns that into a *finding* — an input, a peak figure, and a ceiling it
//! crossed — which is what [`crate::fuzz`] reports.
//!
//! Two ceilings, because one cannot do both jobs.
//!
//! * The **soft ceiling** is checked by the driver *after* an execution returns, against the peak
//!   this module recorded. Crossing it is a reported finding, and the campaign carries on.
//! * The **hard ceiling** is checked *inside* the allocator, on the allocation itself, because a
//!   soft ceiling cannot stop an allocation already in flight. Crossing it aborts the process, and
//!   the input that did it is on disk: the driver writes every input to the in-flight file before
//!   running it, precisely so this abort names its cause.
//!
//! # Safety
//!
//! `unsafe_code` is `deny` workspace-wide and this module is the third place in the repository to
//! `allow` it locally — the other two are the binding crates. The justification is narrow and
//! checkable: the one `unsafe` block per method forwards, unchanged, to [`System`], the allocator
//! that would have served the request anyway. Every `ptr`, `layout` and size argument is passed
//! through untouched, so each call has exactly the contract `System`'s does and this type adds no
//! new obligation on the caller. Nothing here dereferences a pointer, and no pointer is constructed.
//! The bookkeeping around the forwarded call is safe code operating on atomics.
//!
//! `xtask` is a host-only developer binary that no shipped crate depends on, so nothing this module
//! does can reach a published artefact.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Bytes currently allocated by the process.
static LIVE: AtomicUsize = AtomicUsize::new(0);
/// The high-water mark since the last [`reset_peak`].
static PEAK: AtomicUsize = AtomicUsize::new(0);
/// The hard ceiling, in bytes. `usize::MAX` disables it.
static HARD_CEILING: AtomicUsize = AtomicUsize::new(usize::MAX);
/// Set once the hard ceiling has fired, so the abort path cannot re-enter itself.
static ABORTING: AtomicBool = AtomicBool::new(false);

/// A [`System`] allocator that keeps a live-byte count and a high-water mark, and aborts the process
/// rather than let a single execution run away with the machine.
#[derive(Debug)]
pub struct Counting;

impl Counting {
    /// Records `bytes` more live, updating the peak, and aborts past the hard ceiling.
    #[inline]
    fn note_allocation(bytes: usize) {
        let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
        PEAK.fetch_max(live, Ordering::Relaxed);
        if live > HARD_CEILING.load(Ordering::Relaxed) && !ABORTING.swap(true, Ordering::Relaxed) {
            // Deliberately terse and allocation-free: we are inside the allocator, past its ceiling,
            // and anything richer risks recursing back into this function. The input that caused it
            // is in the in-flight file the driver wrote before this execution began.
            eprintln!("\nfuzz: hard memory ceiling crossed; see the in-flight input. Aborting.");
            std::process::abort();
        }
    }

    /// Records `bytes` freed.
    #[inline]
    fn note_deallocation(bytes: usize) {
        LIVE.fetch_sub(bytes, Ordering::Relaxed);
    }
}

// SAFETY: see the module docs. Each method forwards its arguments unchanged to `System`, which is
// the allocator that would otherwise have served the call, so the safety contract is exactly
// `System`'s and no additional obligation is introduced. No pointer is dereferenced or synthesised
// here; the counting is safe code on atomics.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            Self::note_allocation(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        Self::note_deallocation(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            Self::note_allocation(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            Self::note_deallocation(layout.size());
            Self::note_allocation(new_size);
        }
        new_ptr
    }
}

/// Arms the hard ceiling at `bytes`.
pub fn set_hard_ceiling(bytes: usize) {
    HARD_CEILING.store(bytes, Ordering::Relaxed);
}

/// Resets the high-water mark to the current live figure, and returns that figure.
///
/// Called immediately before each execution so the peak the driver reads afterwards belongs to that
/// execution rather than to the campaign's own corpus.
pub fn reset_peak() -> usize {
    let live = LIVE.load(Ordering::Relaxed);
    PEAK.store(live, Ordering::Relaxed);
    live
}

/// The high-water mark since the last [`reset_peak`].
pub fn peak() -> usize {
    PEAK.load(Ordering::Relaxed)
}
