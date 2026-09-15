//! **One reading of Rust test source**, shared by the ledger's scanner and by
//! `xtask/tests/ledger_checklist.rs`, which includes this file by `#[path]`.
//!
//! It depends on `std` alone for that reason: a second reading in the test would be a second
//! definition of what the ledger counts, and the two would drift without either going red.
//!
//! Every question is asked of **normalised** source. Comments are gone, each string literal is
//! replaced by a numbered placeholder, and every run of whitespace is one space. A pattern cannot
//! be dodged by a line break, and prose inside a string or a comment cannot match one.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Source with its comments removed, its string literals numbered and its whitespace collapsed.
pub(crate) struct Normalised {
    /// The code, each string literal written as `"#<index>"`.
    pub(crate) code: String,
    /// The contents of every string literal, in order, unescaped only as far as the source is.
    pub(crate) strings: Vec<String>,
}

/// Whether `character` can be part of an identifier.
pub(crate) fn is_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Normalises one Rust source file.
pub(crate) fn normalise(source: &str) -> Normalised {
    let characters: Vec<char> = source.chars().collect();
    let mut code = String::with_capacity(source.len());
    let mut strings = Vec::new();
    let mut at = 0;
    let mut pending_space = false;
    let push = |code: &mut String, pending_space: &mut bool, text: &str| {
        if *pending_space && !code.is_empty() {
            code.push(' ');
        }
        *pending_space = false;
        code.push_str(text);
    };
    while at < characters.len() {
        let character = characters[at];
        let next = characters.get(at + 1).copied();
        let previous_is_identifier = at > 0 && is_identifier(characters[at - 1]);
        if character.is_whitespace() {
            pending_space = true;
            at += 1;
        } else if character == '/' && next == Some('/') {
            while at < characters.len() && characters[at] != '\n' {
                at += 1;
            }
            pending_space = true;
        } else if character == '/' && next == Some('*') {
            let mut depth = 0;
            while at < characters.len() {
                if characters[at] == '/' && characters.get(at + 1) == Some(&'*') {
                    depth += 1;
                    at += 2;
                } else if characters[at] == '*' && characters.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    at += 1;
                }
            }
            pending_space = true;
        } else if let Some(length) = (!previous_is_identifier)
            .then(|| raw_string_prefix(&characters[at..]))
            .flatten()
        {
            let hashes = length
                - characters[at..at + length]
                    .iter()
                    .filter(|c| **c != '#')
                    .count();
            let start = at + length;
            let mut end = start;
            while end < characters.len() {
                if characters[end] == '"'
                    && characters[end + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|c| **c == '#')
                        .count()
                        == hashes
                {
                    break;
                }
                end += 1;
            }
            strings.push(
                characters[start..end.min(characters.len())]
                    .iter()
                    .collect(),
            );
            push(
                &mut code,
                &mut pending_space,
                &format!("\"#{}\"", strings.len() - 1),
            );
            at = (end + 1 + hashes).min(characters.len());
        } else if character == '"'
            || (character == 'b' && next == Some('"') && !previous_is_identifier)
        {
            let start = if character == '"' { at + 1 } else { at + 2 };
            let mut end = start;
            while end < characters.len() && characters[end] != '"' {
                end += if characters[end] == '\\' { 2 } else { 1 };
            }
            let end = end.min(characters.len());
            strings.push(characters[start..end].iter().collect());
            push(
                &mut code,
                &mut pending_space,
                &format!("\"#{}\"", strings.len() - 1),
            );
            at = end + 1;
        } else if character == '\'' && next == Some('\\') {
            let mut end = at + 2;
            while end < characters.len() && characters[end] != '\'' {
                end += 1;
            }
            push(&mut code, &mut pending_space, "' '");
            at = end + 1;
        } else if character == '\'' && characters.get(at + 2) == Some(&'\'') {
            push(&mut code, &mut pending_space, "' '");
            at += 3;
        } else {
            let mut buffer = [0; 4];
            push(
                &mut code,
                &mut pending_space,
                character.encode_utf8(&mut buffer),
            );
            at += 1;
        }
    }
    Normalised { code, strings }
}

/// The length of a raw string's opening (`r"`, `r#"`, `br##"`, …) at the start of `characters`.
fn raw_string_prefix(characters: &[char]) -> Option<usize> {
    let mut at = 0;
    if characters.first() == Some(&'b') {
        at += 1;
    }
    if characters.get(at) != Some(&'r') {
        return None;
    }
    at += 1;
    while characters.get(at) == Some(&'#') {
        at += 1;
    }
    (characters.get(at) == Some(&'"')).then_some(at + 1)
}

/// Everything on one line before a `//` that is not inside a string literal.
pub(crate) fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if in_string => at += 1,
            b'"' => in_string = !in_string,
            b'\'' if !in_string && bytes.get(at + 2) == Some(&b'\'') => at += 2,
            b'/' if !in_string && bytes.get(at + 1) == Some(&b'/') => return &line[..at],
            _ => {}
        }
        at += 1;
    }
    line
}

/// Whether `code` names `identifier` as a whole identifier rather than as part of a longer one.
pub(crate) fn contains_identifier(code: &str, identifier: &str) -> bool {
    code.match_indices(identifier).any(|(at, _)| {
        let before = code[..at].chars().next_back().is_some_and(is_identifier);
        let after = code[at + identifier.len()..]
            .chars()
            .next()
            .is_some_and(is_identifier);
        !before && !after
    })
}

/// Whether `code` holds an `impl … Trait for` for `trait_name`, bare or path-qualified.
pub(crate) fn implements(code: &str, trait_name: &str) -> bool {
    code.match_indices(trait_name).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        if before.is_some_and(is_identifier) {
            return false;
        }
        let after = code[at + trait_name.len()..].trim_start();
        if !after
            .strip_prefix("for")
            .is_some_and(|rest| rest.starts_with(|c: char| !is_identifier(c)))
        {
            return false;
        }
        let head = &code[..at];
        head.match_indices("impl").any(|(start, _)| {
            let bounded = !head[..start].chars().next_back().is_some_and(is_identifier)
                && !head[start + 4..].chars().next().is_some_and(is_identifier);
            bounded && !head[start + 4..].contains(['{', '}', ';'])
        })
    })
}

/// Whether `code` hands `register_all` a closure, which supplies each outline from the test.
pub(crate) fn registers_a_closure(code: &str) -> bool {
    code.match_indices("register_all(").any(|(at, call)| {
        let mut depth = 0_usize;
        for character in code[at + call.len()..].chars() {
            match character {
                '(' => depth += 1,
                ')' if depth == 0 => return false,
                ')' => depth -= 1,
                '|' => return true,
                _ => {}
            }
        }
        false
    })
}

/// Every `mod name;` declared in normalised code, with its `#[path]` value where it has one.
pub(crate) fn module_declarations(source: &Normalised) -> Vec<(String, Option<String>)> {
    let code = &source.code;
    let mut declarations = Vec::new();
    for (at, _) in code.match_indices("mod ") {
        if code[..at].chars().next_back().is_some_and(is_identifier) {
            continue;
        }
        let rest = &code[at + 4..];
        let name: String = rest.chars().take_while(|c| is_identifier(*c)).collect();
        if name.is_empty() || !rest[name.len()..].trim_start().starts_with(';') {
            continue;
        }
        let mut head = code[..at].trim_end();
        for visibility in ["pub(crate)", "pub(super)", "pub"] {
            if let Some(stripped) = head.strip_suffix(visibility) {
                head = stripped.trim_end();
                break;
            }
        }
        let mut path = None;
        while head.ends_with(']') {
            let Some(open) = head.rfind("#[") else { break };
            let attribute = &head[open..];
            if let Some(index) = attribute
                .strip_prefix("#[path = \"#")
                .and_then(|rest| rest.strip_suffix("\"]"))
                .and_then(|index| index.parse::<usize>().ok())
            {
                path = source.strings.get(index).cloned();
            }
            head = head[..open].trim_end();
        }
        declarations.push((name, path));
    }
    declarations
}

/// A path with its `.` and `..` components resolved lexically.
fn lexically(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    resolved
}

/// Every module file the suite at `suite` (workspace-relative) pulls in, transitively, as `(path, source)`.
///
/// A `mod name;` that resolves to no file is an error, so a helper the scan cannot find is never skipped.
pub(crate) fn module_files(root: &Path, suite: &str) -> Result<Vec<(String, String)>, String> {
    let read = |relative: &str| {
        std::fs::read_to_string(root.join(relative))
            .map_err(|error| format!("reading {relative}: {error}"))
    };
    let mut files = Vec::new();
    let mut seen = BTreeSet::from([suite.to_owned()]);
    // (workspace-relative path, source, whether nested modules resolve beside it as a `mod.rs` does)
    let mut queue = vec![(suite.to_owned(), read(suite)?, true)];
    while let Some((file, source, directory_owner)) = queue.pop() {
        let path = PathBuf::from(&file);
        let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let owner = if directory_owner || path.file_name().is_some_and(|name| name == "mod.rs") {
            directory.clone()
        } else {
            directory.join(path.file_stem().unwrap_or_default())
        };
        for (name, attribute) in module_declarations(&normalise(&source)) {
            let candidates = match &attribute {
                Some(explicit) => vec![(lexically(&directory.join(explicit)), true)],
                None => vec![
                    (owner.join(format!("{name}.rs")), false),
                    (owner.join(&name).join("mod.rs"), true),
                ],
            };
            let Some((found, owns)) = candidates
                .into_iter()
                .find(|(candidate, _)| root.join(candidate).is_file())
            else {
                return Err(format!(
                    "`{file}` declares `mod {name};` and no file for it exists; the ledger will not \
                     guess which helper a suite compiles"
                ));
            };
            let relative = found
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if seen.insert(relative.clone()) {
                let helper = read(&relative)?;
                files.push((relative.clone(), helper.clone()));
                queue.push((relative, helper, owns));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Display-list and pixel APIs whose use means a function reads what was drawn, with the crates that own each.
///
/// An API counts only in a suite that names one of its owners, so a same-named method elsewhere — an
/// outline's `commands()` — is not mistaken for it. Producing a list or pixels is not here, and
/// neither is constructing a command: reading them is.
pub(crate) const DRAWING_APIS: &[(&str, &[&str])] = &[
    (".commands()", &["mjx_scene"]),
    ("DisplayList::from_bytes(", &["mjx_scene"]),
    (".glyph_run(", &["mjx_scene"]),
    (".record_count(", &["mjx_scene"]),
    (".section_bytes(", &["mjx_scene"]),
    (".gradient_stop(", &["mjx_scene"]),
    (".pixel(", &["mjx_paint", "mjx_render_oracle"]),
    (".covered()", &["mjx_paint", "mjx_render_oracle"]),
    (".distinct_colors()", &["mjx_paint"]),
    (".rgba", &["mjx_paint", "mjx_render_oracle"]),
    ("compare_painters(", &["mjx_paint"]),
    ("snapshot::commands(", &["mjx_render_oracle"]),
    ("compare_against_baseline(", &["mjx_render_oracle"]),
];

/// Whether `code` uses `token` with no identifier running into either end of it.
fn uses_token(code: &str, token: &str) -> bool {
    let starts_word = token.starts_with(is_identifier);
    let ends_word = token.ends_with(is_identifier);
    code.match_indices(token).any(|(at, _)| {
        let before = starts_word && code[..at].chars().next_back().is_some_and(is_identifier);
        let after = ends_word
            && code[at + token.len()..]
                .chars()
                .next()
                .is_some_and(is_identifier);
        !before && !after
    })
}

/// Every function defined in normalised code, as `(name, body)`, in source order.
pub(crate) fn functions(code: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    for (at, _) in code.match_indices("fn ") {
        if code[..at].chars().next_back().is_some_and(is_identifier) {
            continue;
        }
        let rest = &code[at + 3..];
        let length = rest.find(|c: char| !is_identifier(c)).unwrap_or(rest.len());
        if length == 0 {
            continue;
        }
        let signature_start = at + 3 + length;
        let mut depth = 0_i64;
        let mut open = None;
        for (offset, character) in code[signature_start..].char_indices() {
            match character {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                ';' if depth == 0 => break,
                '{' if depth == 0 => {
                    open = Some(signature_start + offset);
                    break;
                }
                _ => {}
            }
        }
        let Some(open) = open else { continue };
        let mut depth = 0_i64;
        let mut end = code.len();
        for (offset, character) in code[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + offset + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        found.push((&rest[..length], &code[open..end]));
    }
    found
}

/// Every nonzero loss count normalised code asserts as `.count(Kind::Variant), N`, as `(Kind::Variant, N)`.
pub(crate) fn asserted_losses(code: &str) -> Vec<(String, usize)> {
    const KINDS: [&str; 3] = ["LayoutLossKind::", "SceneLossKind::", "PainterLossKind::"];
    let mut found: Vec<(String, usize)> = Vec::new();
    for (at, marker) in code.match_indices("count(") {
        if code[..at].chars().next_back().is_some_and(is_identifier) {
            continue;
        }
        let open = at + marker.len();
        let mut depth = 1_usize;
        let mut close = None;
        for (offset, character) in code[open..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else {
            continue;
        };
        let argument: String = code[open..close]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        let Some(kind) = KINDS
            .iter()
            .find_map(|kind| argument.find(kind).map(|start| &argument[start..]))
        else {
            continue;
        };
        let Some(rest) = code[close + 1..].trim_start().strip_prefix(',') else {
            continue;
        };
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let Ok(count) = digits.parse::<usize>() else {
            continue;
        };
        if count > 0 && !found.iter().any(|(named, _)| named == kind) {
            found.push((kind.to_owned(), count));
        }
    }
    found
}

/// The names of the `#[test]` functions normalised code declares.
pub(crate) fn test_functions(code: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    for (at, marker) in code.match_indices("#[test]") {
        let mut rest = code[at + marker.len()..].trim_start();
        while rest.starts_with("#[") {
            let Some(close) = rest.find(']') else { break };
            rest = rest[close + 1..].trim_start();
        }
        for prefix in ["pub ", "async "] {
            rest = rest.strip_prefix(prefix).unwrap_or(rest);
        }
        if let Some(after) = rest.strip_prefix("fn ") {
            let length = after
                .find(|c: char| !is_identifier(c))
                .unwrap_or(after.len());
            if length > 0 {
                names.insert(&after[..length]);
            }
        }
    }
    names
}

/// The names a body calls as free or path-qualified functions, never as methods or macros.
fn calls(body: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    for (at, _) in body.match_indices('(') {
        let head = &body[..at];
        let start = head
            .char_indices()
            .rev()
            .find(|(_, character)| !is_identifier(*character))
            .map_or(0, |(index, character)| index + character.len_utf8());
        let name = &head[start..];
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if head[..start].ends_with('.') {
            continue;
        }
        names.insert(name);
    }
    names
}

/// The bodies of the function `name` and of every function defined in `code` it calls, transitively.
pub(crate) fn reach<'a>(code: &'a str, name: &str) -> Vec<&'a str> {
    let defined = functions(code);
    let mut seen = BTreeSet::from([name.to_owned()]);
    let mut queue = vec![name.to_owned()];
    let mut bodies = Vec::new();
    while let Some(next) = queue.pop() {
        for (_, body) in defined
            .iter()
            .filter(|(defined_name, _)| *defined_name == next)
        {
            bodies.push(*body);
            for callee in calls(body) {
                if defined
                    .iter()
                    .any(|(defined_name, _)| *defined_name == callee)
                    && seen.insert(callee.to_owned())
                {
                    queue.push(callee.to_owned());
                }
            }
        }
    }
    bodies
}

/// Whether the function `name`, or a function defined in `code` it calls, reads a display list or pixels.
pub(crate) fn draws(code: &str, name: &str) -> bool {
    let apis: Vec<&str> = DRAWING_APIS
        .iter()
        .filter(|(_, owners)| owners.iter().any(|owner| contains_identifier(code, owner)))
        .map(|(api, _)| *api)
        .collect();
    !apis.is_empty()
        && reach(code, name)
            .iter()
            .any(|body| apis.iter().any(|api| uses_token(body, api)))
}

/// The test functions `suite_code` declares that draw, reading calls across the whole `unit_code`.
pub(crate) fn drawing_tests(suite_code: &str, unit_code: &str) -> BTreeSet<String> {
    test_functions(suite_code)
        .into_iter()
        .filter(|name| draws(unit_code, name))
        .map(str::to_owned)
        .collect()
}

/// Assertion-macro invocations in normalised code.
pub(crate) fn count_assertions(code: &str) -> usize {
    const MACROS: [&str; 4] = ["assert!(", "assert_eq!(", "assert_ne!(", "assert_matches!("];
    MACROS
        .iter()
        .map(|name| {
            code.match_indices(name)
                .filter(|(at, _)| !code[..*at].chars().next_back().is_some_and(is_identifier))
                .count()
        })
        .sum()
}

/// A citation as its suite path and the test function it names, if it names one.
pub(crate) fn split_citation(citation: &str) -> (&str, Option<&str>) {
    match citation.split_once(".rs::") {
        Some((suite, function)) => (&citation[..suite.len() + 3], Some(function)),
        None => (citation, None),
    }
}

/// The normalised code of a suite and of the helper sources it pulls in, as one string.
pub(crate) fn unit_code(suite_code: &str, helper_sources: &[String]) -> String {
    let mut unit = suite_code.to_owned();
    for helper in helper_sources {
        unit.push(' ');
        unit.push_str(&normalise(helper).code);
    }
    unit
}

/// A suite read off the tree: its raw source, its own normalised code, and that of its whole unit.
pub(crate) struct Unit {
    /// The suite file as written.
    pub(crate) source: String,
    /// The suite file, normalised.
    pub(crate) code: String,
    /// The suite and every helper module it pulls in, normalised.
    pub(crate) unit: String,
}

/// Reads the suite at `suite`, workspace-relative, with every helper module it pulls in.
pub(crate) fn read_unit(root: &Path, suite: &str) -> Result<Unit, String> {
    let source = std::fs::read_to_string(root.join(suite))
        .map_err(|error| format!("reading {suite}: {error}"))?;
    let helpers: Vec<String> = module_files(root, suite)?
        .into_iter()
        .map(|(_, helper)| helper)
        .collect();
    let code = normalise(&source).code;
    let unit = unit_code(&code, &helpers);
    Ok(Unit { source, code, unit })
}

#[cfg(test)]
mod loss_tests {
    use super::{asserted_losses, normalise};

    #[test]
    fn a_nonzero_loss_count_is_read_out_of_an_assertion_and_nothing_else_is() {
        let code = normalise(
            "assert_eq!(losses.count(SceneLossKind::TextPaintDefaulted), 14);\n\
             assert_eq!(\n    losses.count(mjx_layout::LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink)),\n    2\n);\n\
             assert_eq!(losses.count(PainterLossKind::LineEndNotDrawn), 0);\n\
             let n = recount(SceneLossKind::ColourNotResolved), 3;\n\
             assert_eq!(losses.count(Other::Thing), 5);",
        )
        .code;
        assert_eq!(
            asserted_losses(&code),
            vec![
                ("SceneLossKind::TextPaintDefaulted".to_owned(), 14),
                (
                    "LayoutLossKind::FrameContentNotLaidOut(FrameContent::Ink)".to_owned(),
                    2
                ),
            ]
        );
    }
}
