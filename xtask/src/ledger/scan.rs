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
