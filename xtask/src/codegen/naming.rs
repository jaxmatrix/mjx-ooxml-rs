//! The naming engine: turns cryptic OOXML symbols into comprehensive, self-explanatory Rust
//! identifiers (see the naming convention in `PLAN.md`).
//!
//! Strategy: curated overrides win; otherwise split the token into words (on separators and
//! camelCase/letter-digit humps), expand known abbreviations, PascalCase, and sanitize into a valid
//! Rust identifier. Wire tokens themselves are never altered — only the Rust-facing name.

/// Curated naming data for a code-generation slice.
pub struct NameEngine {
    /// `ST_*` type name → Rust type name.
    pub type_overrides: &'static [(&'static str, &'static str)],
    /// (`ST_*` type, wire value) → Rust variant name.
    pub variant_overrides: &'static [(&'static str, &'static str, &'static str)],
    /// lowercase word → PascalCase expansion (e.g. `alg` → `Algorithm`).
    pub abbreviations: &'static [(&'static str, &'static str)],
}

impl NameEngine {
    /// The comprehensive Rust type name for an `ST_*` type.
    pub fn type_name(&self, st_name: &str) -> String {
        if let Some((_, rust)) = self.type_overrides.iter().find(|(k, _)| *k == st_name) {
            return (*rust).to_owned();
        }
        let base = st_name.strip_prefix("ST_").unwrap_or(st_name);
        sanitize_ident(expand_pascal(base, self.abbreviations))
    }

    /// The comprehensive Rust variant name for an enum wire value.
    pub fn variant_name(&self, st_name: &str, wire: &str) -> String {
        if let Some((_, _, rust)) = self
            .variant_overrides
            .iter()
            .find(|(t, w, _)| *t == st_name && *w == wire)
        {
            return (*rust).to_owned();
        }
        sanitize_ident(expand_pascal(wire, self.abbreviations))
    }
}

/// Splits a token into words on separators and camelCase / letter↔digit boundaries.
pub fn split_words(token: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in token.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            let boundary = (p.is_lowercase() && c.is_uppercase())
                || (p.is_alphabetic() && c.is_ascii_digit())
                || (p.is_ascii_digit() && c.is_alphabetic());
            if boundary && !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
        prev = Some(c);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

fn pascal_word(word: &str) -> String {
    let mut out = String::new();
    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
        for c in chars {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Splits, expands abbreviations, and PascalCases a token.
fn expand_pascal(token: &str, abbreviations: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for word in split_words(token) {
        let lower = word.to_lowercase();
        if let Some((_, expansion)) = abbreviations.iter().find(|(k, _)| *k == lower) {
            out.push_str(expansion);
        } else {
            out.push_str(&pascal_word(&word));
        }
    }
    out
}

/// Ensures a name is a valid, non-reserved Rust identifier.
fn sanitize_ident(name: String) -> String {
    if name.is_empty() {
        return "Empty".to_owned();
    }
    let mut name = name;
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        name = format!("N{name}");
    }
    // `Self`/`Super`/`Crate` are reserved even when capitalized and cannot be raw identifiers.
    match name.as_str() {
        "Self" | "Super" | "Crate" => format!("{name}Value"),
        _ => name,
    }
}

/// Converts a token to `SCREAMING_SNAKE_CASE` (used for constant names).
pub fn screaming_snake(token: &str) -> String {
    split_words(token)
        .iter()
        .map(|w| w.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABBR: &[(&str, &str)] = &[("alg", "Algorithm"), ("prov", "Provider")];
    const ENGINE: NameEngine = NameEngine {
        type_overrides: &[("ST_AlgType", "AlgorithmType")],
        variant_overrides: &[("ST_CalendarType", "gregorianUs", "GregorianUnitedStates")],
        abbreviations: ABBR,
    };

    #[test]
    fn expands_camel_and_abbreviations() {
        assert_eq!(ENGINE.type_name("ST_AlgClass"), "AlgorithmClass");
        assert_eq!(ENGINE.type_name("ST_CryptProv"), "CryptProvider"); // Crypt not in dict here
    }

    #[test]
    fn override_wins_for_type_and_variant() {
        assert_eq!(ENGINE.type_name("ST_AlgType"), "AlgorithmType");
        assert_eq!(
            ENGINE.variant_name("ST_CalendarType", "gregorianUs"),
            "GregorianUnitedStates"
        );
    }

    #[test]
    fn variant_falls_back_to_pascal() {
        assert_eq!(ENGINE.variant_name("ST_X", "superscript"), "Superscript");
        assert_eq!(ENGINE.variant_name("ST_X", "rsaAES"), "RsaAes");
    }

    #[test]
    fn sanitizes_hazards() {
        assert_eq!(sanitize_ident("Self".to_owned()), "SelfValue");
        assert_eq!(sanitize_ident("35mm".to_owned()), "N35mm");
    }

    #[test]
    fn screaming_snake_from_camel() {
        assert_eq!(screaming_snake("wordprocessingml"), "WORDPROCESSINGML");
        assert_eq!(
            screaming_snake("shared-commonSimpleTypes"),
            "SHARED_COMMON_SIMPLE_TYPES"
        );
    }
}
