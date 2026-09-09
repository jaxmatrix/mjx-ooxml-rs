//! The two caches a viewport needs, and the counters that prove they hit.
//!
//! # Why two, and why the second is the smaller win
//!
//! A screen of a spreadsheet is a few hundred cells sharing a handful of format codes. So the cache
//! that matters is the **compiled** one: `#,##0.00` is parsed once for the whole sheet rather than
//! once per cell, and parsing is where the cost is.
//!
//! The **result** cache — keyed by `(code, value)` — is the smaller win, because a column of
//! distinct numbers shares no results. It earns its place on the sheets people actually build:
//! repeated dates down a schedule, a column of zeroes, a status column of ones. Only numeric values
//! are cached; formatting a text value is a walk of a literal list and is not worth a hash.
//!
//! # It is bounded, because a viewport scrolls
//!
//! Both tables are cleared wholesale when they pass their bound rather than evicted one entry at a
//! time. A scroll through a hundred thousand rows would otherwise grow a table nothing ever reads
//! again, and an LRU here would cost more to maintain than the misses it saves — the population is
//! small and the access pattern is a moving window rather than a working set.
//!
//! # The counters are the gate
//!
//! [`FormatCache::requests`], [`FormatCache::compilations`] and [`FormatCache::evaluations`] are
//! public so a suite can assert the hit rather than assume it.
//! `tests/the_format_cache_hits.rs` lays out a band whose cells share formats and asserts that the
//! compilation count is the number of *distinct codes* rather than the number of cells — which is
//! the measurement that tells a cache from a field nothing reads.

use std::collections::HashMap;

use mjx_xlsx::DateSystem;

use super::{evaluate, CellValue, CompiledFormat, FormattedValue};

/// How many distinct format codes are held before the table is cleared.
///
/// A workbook's whole `numFmts` table is typically under twenty entries and Excel's own ceiling is
/// in the low hundreds, so this is a bound against a pathological file rather than a working limit.
const MAX_COMPILED_FORMATS: usize = 512;

/// How many formatted results are held before the table is cleared.
///
/// Sized so that several screens' worth of a date column survive a scroll, and so that the table
/// cannot grow past a few hundred kilobytes.
const MAX_CACHED_RESULTS: usize = 8192;

/// The compiled-format and formatted-value caches, with the counters that prove they hit.
#[derive(Debug, Clone, Default)]
pub struct FormatCache {
    /// Which compiled format each code string resolved to.
    by_code: HashMap<Box<str>, usize>,
    /// The compiled formats, addressed by the index `by_code` answers with.
    formats: Vec<CompiledFormat>,
    /// `(format index, value bits) -> what it rendered`.
    ///
    /// The key is the value's **bit pattern** rather than the value: a `f64` is not `Eq` and two
    /// values that differ only in their last bit format differently often enough to matter. `NaN`
    /// never reaches here — [`CellValue::read`] refuses a non-finite `<v>`.
    results: HashMap<(usize, u64), FormattedValue>,
    requests: u64,
    compilations: u64,
    evaluations: u64,
}

impl FormatCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Formats `value` through `code` under `dates`.
    ///
    /// `code` is the format code in force — `None` means the cell states none, which is `General`.
    #[must_use]
    pub fn format(
        &mut self,
        code: Option<&str>,
        value: CellValue<'_>,
        dates: DateSystem,
    ) -> FormattedValue {
        self.requests += 1;
        let index = self.compiled(code);
        // Text, booleans and errors are a literal walk; hashing them would cost more than rendering
        // them, and the table would fill with strings a scroll never sees again.
        let bits = match value {
            CellValue::Number(number) => Some(number.to_bits()),
            _ => None,
        };
        if let Some(bits) = bits {
            if let Some(hit) = self.results.get(&(index, bits)) {
                return hit.clone();
            }
        }
        self.evaluations += 1;
        let rendered = match self.formats.get(index) {
            Some(format) => evaluate(format, value, dates),
            // Unreachable while `compiled` answers with an index it has just pushed; answering
            // `General` rather than refusing is what keeps this method total.
            None => evaluate(&CompiledFormat::general(), value, dates),
        };
        if let Some(bits) = bits {
            if self.results.len() >= MAX_CACHED_RESULTS {
                self.results.clear();
            }
            self.results.insert((index, bits), rendered.clone());
        }
        rendered
    }

    /// The index of the compiled form of `code`, compiling it on first sight.
    fn compiled(&mut self, code: Option<&str>) -> usize {
        let code = code.unwrap_or("General");
        if let Some(index) = self.by_code.get(code) {
            return *index;
        }
        if self.by_code.len() >= MAX_COMPILED_FORMATS {
            self.by_code.clear();
            self.formats.clear();
            self.results.clear();
        }
        self.compilations += 1;
        let index = self.formats.len();
        self.formats.push(CompiledFormat::compile(code));
        self.by_code.insert(code.into(), index);
        index
    }

    /// How many values have been asked for.
    #[must_use]
    pub fn requests(&self) -> u64 {
        self.requests
    }

    /// How many format codes have actually been parsed.
    ///
    /// One per **distinct code**, not one per cell. That difference is the cache.
    #[must_use]
    pub fn compilations(&self) -> u64 {
        self.compilations
    }

    /// How many values have actually been rendered.
    #[must_use]
    pub fn evaluations(&self) -> u64 {
        self.evaluations
    }

    /// How many distinct codes are held.
    #[must_use]
    pub fn compiled_count(&self) -> usize {
        self.formats.len()
    }

    /// How many formatted results are held.
    #[must_use]
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Empties both tables and both counters.
    pub fn clear(&mut self) {
        self.by_code.clear();
        self.formats.clear();
        self.results.clear();
        self.requests = 0;
        self.compilations = 0;
        self.evaluations = 0;
    }
}
