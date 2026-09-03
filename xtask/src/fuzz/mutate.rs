//! The mutators: how one corpus entry becomes the next candidate input.
//!
//! Byte-level mutation alone is a poor fit for XML. A random flip inside `<p:sld …>` almost always
//! produces something the tokenizer rejects in its first few bytes, so a purely byte-level campaign
//! spends its budget re-deriving that garbage is garbage. The mutators here are therefore mixed:
//! byte-level operators for the tokenizer's edges, a **token dictionary** so the generator can
//! assemble markup that is *syntactically plausible and semantically hostile*, and a **repeat**
//! operator, which is the one that finds resource bugs — depth, breadth and entity expansion are all
//! "the same token, many times", and no sequence of single-byte flips will ever build one.
//!
//! A mutator is chosen by the seeded generator, so the whole sequence is reproducible from
//! `--seed`.

use crate::fuzz::random::Random;

/// Fragments the generator splices in: the vocabulary of the formats under test.
///
/// A dictionary is what lets a black-box mutator reach code that is gated on an exact string. The
/// MCE resolver, for one, is unreachable without `mc:AlternateContent` and its namespace URI
/// appearing together, and no amount of byte flipping will assemble those.
const TOKENS: &[&[u8]] = &[
    // XML structure.
    b"<a>",
    b"</a>",
    b"<a/>",
    b"<a b='c'>",
    b"<![CDATA[",
    b"]]>",
    b"<!--",
    b"-->",
    b"<?pi ",
    b"?>",
    b"<!DOCTYPE ",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
    b"\xEF\xBB\xBF",
    b"]]",
    b"/>",
    b"&",
    b";",
    // Entities and character references — the expansion surface.
    b"&amp;",
    b"&#38;",
    b"&#x26;",
    b"&lt;",
    b"&#0;",
    b"&#xFFFFFFFF;",
    b"&undeclared;",
    b"<!ENTITY x \"yyyyyyyyyy\">",
    // Namespaces and prefixes.
    b"xmlns=",
    b"xmlns:a='urn:a'",
    b"xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006'",
    b"a:",
    b"mc:",
    b":",
    // Markup compatibility — the MCE resolver is gated on these exact names.
    b"<mc:AlternateContent>",
    b"</mc:AlternateContent>",
    b"<mc:Choice Requires='a'>",
    b"</mc:Choice>",
    b"<mc:Fallback>",
    b"</mc:Fallback>",
    b" mc:Ignorable='a'",
    b" mc:ProcessContent='a'",
    b" mc:MustUnderstand='a'",
    b" Requires=''",
    // Packaging names — the OPC opener's part-name and control-part logic.
    b"[Content_Types].xml",
    b"_rels/.rels",
    b"../",
    b"/",
    b"\\",
    b"%2e%2e%2f",
    b"ppt/presentation.xml",
    b"<Default Extension='xml' ContentType='application/xml'/>",
    b"<Override PartName='/a.xml' ContentType='application/xml'/>",
    b"<Relationship Id='rId1' Type='urn:t' Target='a.xml'/>",
    b" TargetMode='External'",
    // Non-UTF-8 and control bytes.
    b"\xff\xfe",
    b"\x00",
    b"\xC0\x80",
];

/// Bytes worth writing into a single position: the boundaries a hand-written scanner gets wrong.
const INTERESTING_BYTES: &[u8] = &[
    0x00, 0x01, 0x09, 0x0a, 0x0d, 0x20, 0x2f, 0x3c, 0x3e, 0x26, 0x22, 0x27, 0x3d, 0x3a, 0x21, 0x3f,
    0x5b, 0x5d, 0x7f, 0x80, 0xc0, 0xfe, 0xff,
];

/// The largest input the campaign will hand to a target.
///
/// A ceiling is needed, and it has to be *small*: unbounded growth would let the mutator confuse
/// "this input is enormous" with "this input triggers unbounded allocation", which is precisely the
/// distinction the memory ceiling exists to draw. 64 KiB is far more than any of these parsers needs
/// to reach any of its branches.
pub const MAXIMUM_INPUT: usize = 64 * 1024;

/// Mutates `seed` into a fresh candidate.
///
/// Applies between one and four operators, because a single operator rarely moves an input from one
/// behaviour to another, and a dozen turns any input into noise.
#[must_use]
pub fn mutate(seed: &[u8], random: &mut Random, corpus: &[Vec<u8>]) -> Vec<u8> {
    let mut candidate = seed.to_vec();
    let rounds = 1 + random.below(4);
    for _ in 0..rounds {
        apply_one(&mut candidate, random, corpus);
        if candidate.len() > MAXIMUM_INPUT {
            candidate.truncate(MAXIMUM_INPUT);
        }
    }
    candidate
}

fn apply_one(candidate: &mut Vec<u8>, random: &mut Random, corpus: &[Vec<u8>]) {
    match random.below(9) {
        0 => flip_a_bit(candidate, random),
        1 => write_an_interesting_byte(candidate, random),
        2 => delete_a_run(candidate, random),
        3 => duplicate_a_run(candidate, random),
        4 => insert_a_token(candidate, random),
        5 => overwrite_with_a_token(candidate, random),
        6 => splice_in_a_corpus_entry(candidate, random, corpus),
        7 => repeat_a_token(candidate, random),
        _ => nest_a_token_pair(candidate, random),
    }
}

fn flip_a_bit(candidate: &mut [u8], random: &mut Random) {
    if candidate.is_empty() {
        return;
    }
    let index = random.below(candidate.len());
    let bit = random.below(8);
    if let Some(byte) = candidate.get_mut(index) {
        *byte ^= 1 << bit;
    }
}

fn write_an_interesting_byte(candidate: &mut [u8], random: &mut Random) {
    if candidate.is_empty() {
        return;
    }
    let index = random.below(candidate.len());
    let Some(value) = random.pick(INTERESTING_BYTES).copied() else {
        return;
    };
    if let Some(byte) = candidate.get_mut(index) {
        *byte = value;
    }
}

fn delete_a_run(candidate: &mut Vec<u8>, random: &mut Random) {
    if candidate.len() < 2 {
        return;
    }
    let start = random.below(candidate.len());
    let length = random.short_length(candidate.len() - start);
    candidate.drain(start..start + length);
}

fn duplicate_a_run(candidate: &mut Vec<u8>, random: &mut Random) {
    if candidate.is_empty() {
        return;
    }
    let start = random.below(candidate.len());
    let length = random.short_length((candidate.len() - start).min(256));
    let run: Vec<u8> = candidate[start..start + length].to_vec();
    let at = random.below(candidate.len() + 1);
    splice(candidate, at, &run);
}

fn insert_a_token(candidate: &mut Vec<u8>, random: &mut Random) {
    let Some(token) = random.pick(TOKENS).copied() else {
        return;
    };
    let at = random.below(candidate.len() + 1);
    splice(candidate, at, token);
}

fn overwrite_with_a_token(candidate: &mut [u8], random: &mut Random) {
    let Some(token) = random.pick(TOKENS).copied() else {
        return;
    };
    if candidate.is_empty() {
        return;
    }
    let at = random.below(candidate.len());
    let length = token.len().min(candidate.len() - at);
    candidate[at..at + length].copy_from_slice(&token[..length]);
}

fn splice_in_a_corpus_entry(candidate: &mut Vec<u8>, random: &mut Random, corpus: &[Vec<u8>]) {
    let Some(other) = random.pick(corpus) else {
        return;
    };
    if other.is_empty() {
        return;
    }
    let start = random.below(other.len());
    let length = random.short_length((other.len() - start).min(512));
    let at = random.below(candidate.len() + 1);
    let run = other[start..start + length].to_vec();
    splice(candidate, at, &run);
}

/// The resource operator: one token, many times.
///
/// Depth, breadth and entity expansion all have this shape, and none of them is reachable by
/// flipping bytes. The repeat count is drawn from a wide range so the campaign covers "a hundred"
/// and "as many as fit" rather than only the small cases.
fn repeat_a_token(candidate: &mut Vec<u8>, random: &mut Random) {
    let Some(token) = random.pick(TOKENS).copied() else {
        return;
    };
    if token.is_empty() {
        return;
    }
    let budget = MAXIMUM_INPUT.saturating_sub(candidate.len()) / token.len();
    if budget == 0 {
        return;
    }
    let count = 1 + random.below(budget);
    let mut run = Vec::with_capacity(count * token.len());
    for _ in 0..count {
        run.extend_from_slice(token);
    }
    let at = random.below(candidate.len() + 1);
    splice(candidate, at, &run);
}

/// Builds `<a><a>…<a/>…</a></a>`: balanced nesting to an arbitrary depth.
///
/// The unbalanced version (which [`repeat_a_token`] also produces) is rejected early by the reader.
/// This one *parses*, so the whole tree is built, walked, serialized and dropped — which is where a
/// recursive walk over an attacker-chosen depth actually bites.
fn nest_a_token_pair(candidate: &mut Vec<u8>, random: &mut Random) {
    const OPEN: &[u8] = b"<a>";
    const CLOSE: &[u8] = b"</a>";
    let budget = MAXIMUM_INPUT.saturating_sub(candidate.len()) / (OPEN.len() + CLOSE.len());
    if budget == 0 {
        return;
    }
    let depth = 1 + random.below(budget);
    let mut run = Vec::with_capacity(depth * (OPEN.len() + CLOSE.len()));
    for _ in 0..depth {
        run.extend_from_slice(OPEN);
    }
    for _ in 0..depth {
        run.extend_from_slice(CLOSE);
    }
    *candidate = run;
}

fn splice(candidate: &mut Vec<u8>, at: usize, run: &[u8]) {
    let at = at.min(candidate.len());
    let tail = candidate.split_off(at);
    candidate.extend_from_slice(run);
    candidate.extend_from_slice(&tail);
}

#[cfg(test)]
mod tests {
    use super::{mutate, MAXIMUM_INPUT};
    use crate::fuzz::random::Random;

    #[test]
    fn mutation_is_reproducible_from_the_seed() {
        let corpus = vec![b"<a>x</a>".to_vec(), b"<b/>".to_vec()];
        let run = |seed| {
            let mut random = Random::new(seed);
            (0..500)
                .map(|_| mutate(b"<a>x</a>", &mut random, &corpus))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(11), run(11), "a campaign must be repeatable");
        assert_ne!(run(11), run(12));
    }

    #[test]
    fn no_mutant_exceeds_the_ceiling() {
        let corpus = vec![vec![b'x'; 4096]];
        let mut random = Random::new(3);
        for _ in 0..2_000 {
            assert!(mutate(&corpus[0], &mut random, &corpus).len() <= MAXIMUM_INPUT);
        }
    }

    #[test]
    fn the_mutators_reach_deep_nesting_and_long_repeats() {
        // If this ever failed, the campaign could not build the inputs that find resource bugs, and
        // its clean run would mean nothing. That is the ticket's first trap in one assertion.
        let corpus = vec![b"<a/>".to_vec()];
        let mut random = Random::new(5);
        let mut deepest = 0usize;
        for _ in 0..5_000 {
            let candidate = mutate(&corpus[0], &mut random, &corpus);
            let mut depth = 0usize;
            let mut best = 0usize;
            let mut i = 0usize;
            while i + 3 <= candidate.len() {
                if &candidate[i..i + 3] == b"<a>" {
                    depth += 1;
                    best = best.max(depth);
                    i += 3;
                } else if i + 4 <= candidate.len() && &candidate[i..i + 4] == b"</a>" {
                    depth = depth.saturating_sub(1);
                    i += 4;
                } else {
                    i += 1;
                }
            }
            deepest = deepest.max(best);
        }
        assert!(
            deepest > 1_000,
            "the mutators only reached depth {deepest}; a campaign that cannot build a deep \
             document cannot find a defect that needs one"
        );
    }
}
