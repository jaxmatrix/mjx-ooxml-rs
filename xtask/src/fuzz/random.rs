//! A deterministic pseudo-random generator, so a campaign is a reproducible experiment.
//!
//! The whole point of seeding the driver rather than reaching for the operating system's entropy is
//! that `--seed 7` twice is the same campaign twice: an operator handed a finding can re-run the
//! exact sequence that produced it, and a fix can be shown to close it rather than to move it. No
//! dependency is needed for that — `SplitMix64` is nine lines and has the statistical quality this
//! job asks for, which is "spread the mutation choices out", not cryptography.

/// A `SplitMix64` generator (Steele, Lea & Flood, 2014).
#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    /// A generator seeded with `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`. Returns `0` when `bound` is `0`, so a caller need not special-case an
    /// empty collection.
    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % bound as u64).unwrap_or(0)
    }

    /// A length in `1..=bound`, biased hard towards short.
    ///
    /// This matters more than it looks. A uniform length over "everything left in the buffer" makes
    /// a **one-byte** edit vanishingly rare on a realistic input, and one byte is exactly what turns
    /// `Requires="n"` into `Requires=""`. The campaign's planted defect went unfound for 300,000
    /// executions against a uniform draw and was found against this one — the harness's own proof
    /// that planting a defect was worth the trouble.
    ///
    /// The shape is a power-of-two ladder: an exponent is drawn uniformly, so half the draws land in
    /// `1..=2`, three quarters in `1..=4`, and the long tail is still reachable.
    pub fn short_length(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return bound;
        }
        let ceiling = (usize::BITS - bound.leading_zeros()) as usize;
        let exponent = self.below(ceiling);
        1 + self.below((1usize << exponent).min(bound))
    }

    /// One element of `items`, or `None` if it is empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        let index = self.below(items.len());
        items.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn the_same_seed_is_the_same_sequence() {
        let sequence = |seed| {
            let mut generator = Random::new(seed);
            (0..32).map(|_| generator.next_u64()).collect::<Vec<_>>()
        };
        assert_eq!(sequence(7), sequence(7), "a campaign must be reproducible");
        // And different seeds diverge, or every campaign would explore the same path.
        assert_ne!(sequence(7), sequence(8));
    }

    #[test]
    fn short_lengths_are_short_but_the_tail_is_still_reachable() {
        let mut generator = Random::new(9);
        let draws: Vec<usize> = (0..10_000).map(|_| generator.short_length(4096)).collect();
        assert!(draws.iter().all(|length| (1..=4096).contains(length)));
        let ones = draws.iter().filter(|length| **length == 1).count();
        assert!(
            ones > 1_000,
            "only {ones} single-byte draws in 10,000 — a one-byte edit is what flips an attribute \
             value, and a uniform draw makes it unreachable"
        );
        assert!(
            draws.iter().any(|length| *length > 512),
            "the long tail must still be drawn, or large structural edits become impossible"
        );
    }

    #[test]
    fn below_stays_in_range_and_tolerates_an_empty_bound() {
        let mut generator = Random::new(1);
        assert_eq!(generator.below(0), 0);
        for _ in 0..1_000 {
            assert!(generator.below(5) < 5);
        }
    }
}
