//! Turning an XML attribute's wire name into a Rust accessor name, deterministically.
//!
//! Only the *shape* of the name is derived here — `rtlCol` → `rtl_col`. Making a name
//! **self-explanatory** is not something a mechanical transform can do (`algn` would become `algn`),
//! so the grammar takes `accessor = alignment` and this module is the fallback for the names that are
//! already words.

/// The Rust identifier text for a wire local name.
///
/// * camel case becomes snake case (`rtlCol` → `rtl_col`, `bwMode` → `bw_mode`), with runs of capitals
///   broken before the last one (`IDName` → `id_name`);
/// * `-`, `.` and `:` — legal in an XML name, not in a Rust one — become `_`, and runs of `_` collapse;
/// * a name that would not start a Rust identifier is prefixed with `_`;
/// * a Rust keyword gains a trailing `_` (`type` → `type_`), because the setter's name is built by
///   prefixing `set_` and `set_r#type` is not a thing that parses.
pub(crate) fn snake_case(local: &str) -> String {
    let chars: Vec<char> = local.chars().collect();
    let mut out = String::with_capacity(local.len() + 4);

    for (index, &character) in chars.iter().enumerate() {
        if matches!(character, '-' | '.' | ':') {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            continue;
        }
        if character.is_uppercase() {
            let previous_is_lower_or_digit = index
                .checked_sub(1)
                .and_then(|i| chars.get(i))
                .is_some_and(|c| c.is_lowercase() || c.is_ascii_digit());
            let next_is_lower = chars.get(index + 1).is_some_and(|c| c.is_lowercase());
            if !out.is_empty()
                && !out.ends_with('_')
                && (previous_is_lower_or_digit || next_is_lower)
            {
                out.push('_');
            }
            out.extend(character.to_lowercase());
            continue;
        }
        out.push(character);
    }

    if out.is_empty() {
        out.push('_');
    }
    if !out.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        out.push('_');
    }
    out
}

/// Whether `word` is a Rust keyword (including the reserved ones), which an identifier may not be.
fn is_rust_keyword(word: &str) -> bool {
    matches!(
        word,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::snake_case;

    #[test]
    fn camel_case_becomes_snake_case() {
        // The five gate attributes, plus the shapes DrawingML and WordprocessingML actually use.
        for (wire, expected) in [
            ("val", "val"),
            ("x", "x"),
            ("cap", "cap"),
            ("rtlCol", "rtl_col"),
            ("bwMode", "bw_mode"),
            ("spcFirstLastPara", "spc_first_last_para"),
            ("IDName", "id_name"),
            ("HTML", "html"),
            ("noProof", "no_proof"),
            ("allowincell", "allowincell"),
            ("cx", "cx"),
            ("algn", "algn"),
        ] {
            assert_eq!(snake_case(wire), expected, "{wire}");
        }
    }

    #[test]
    fn punctuation_legal_in_xml_becomes_underscores() {
        for (wire, expected) in [
            ("allow-in-cell", "allow_in_cell"),
            ("v-text-anchor", "v_text_anchor"),
            ("a.b", "a_b"),
            ("a--b", "a_b"),
        ] {
            assert_eq!(snake_case(wire), expected, "{wire}");
        }
    }

    #[test]
    fn a_name_that_would_not_parse_is_made_to() {
        // Keywords gain a trailing underscore rather than becoming raw identifiers, because the
        // setter is `set_` + this name and `set_r#type` does not parse.
        assert_eq!(snake_case("type"), "type_");
        assert_eq!(snake_case("ref"), "ref_");
        assert_eq!(snake_case("Self"), "self_");
        // Digit-leading and empty are defended against even though XML forbids both.
        assert_eq!(snake_case("2d"), "_2d");
        assert_eq!(snake_case(""), "_");
        assert_eq!(snake_case("-"), "_");
    }
}
