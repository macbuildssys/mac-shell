use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// Built-in command names always available regardless of PATH.
const BUILTINS: &[&str] = &[
    "ls", "cat", "cp", "mv", "rm", "cd", "pwd", "echo", "exit", "mkdir", "clear",
];

/// Extra PATH directories that may not be in the user's login PATH but hold
/// essential system tools like `ifconfig`, `ip`, `update-alternatives`, etc.
const EXTRA_PATH: &[&str] = &[
    "/usr/local/sbin",
    "/usr/local/bin",
    "/usr/sbin",
    "/usr/bin",
    "/sbin",
    "/bin",
];

/// Commands that wrap another command as their first argument (like sudo, env,
/// time, nohup). After these the NEXT word is also a command name.
const COMMAND_WRAPPERS: &[&str] = &["sudo", "env", "time", "nohup", "xargs"];

/// Build the full deduplicated PATH string: env PATH + EXTRA_PATH additions.
fn full_path() -> String {
    let base = env::var("PATH").unwrap_or_default();
    let mut dirs: Vec<&str> = base.split(':').collect();
    for extra in EXTRA_PATH {
        if !dirs.contains(extra) {
            dirs.push(extra);
        }
    }
    dirs.join(":")
}

/// Return all completions for `line` (what the user has typed so far).
///
/// Rules:
///  - First word on the line, OR first word after a separator (`;` `&` `&&`),
///    OR first word after a command-wrapper like `sudo`  → command completion.
///  - Everything else → filesystem path completion.
pub fn complete(line: &str) -> Vec<String> {
    let (before_word, word) = split_last_word(line);

    if is_command_position(before_word) {
        let mut matches = complete_commands(word);
        matches.sort();
        matches.dedup();
        matches
    } else {
        let mut matches = complete_path(word);
        matches.sort();
        matches
    }
}

/// Return just the current (last) word being typed.
pub fn last_word(line: &str) -> &str {
    split_last_word(line).1
}

/// Split into (everything-before-last-word, last-word).
fn split_last_word(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut i = bytes.len();
    while i > 0 && !bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    (&line[..i], &line[i..])
}

/// Decide whether we are completing a command name or an argument.
///
/// `before` is everything on the line before the current word being typed.
/// We are in command position when:
///   - `before` is empty (first word)
///   - `before` after stripping whitespace ends with a separator: `;` `&` `&&`
///   - the last meaningful token in `before` is a command-wrapper like `sudo`
fn is_command_position(before: &str) -> bool {
    let trimmed = before.trim();

    if trimmed.is_empty() {
        return true;
    }

    // If the line ends (after whitespace) with a separator character, the next
    // word is a new command.
    if trimmed.ends_with(';') || trimmed.ends_with('&') {
        return true;
    }

    // If the last non-whitespace token is a command wrapper, the word being
    // typed is the wrapped command name.
    let last_tok = trimmed.split_whitespace().last().unwrap_or("");
    // Strip trailing separator chars that might be glued to the token
    let last_tok = last_tok.trim_end_matches(|c| c == ';' || c == '&');
    if COMMAND_WRAPPERS.contains(&last_tok) {
        return true;
    }

    false
}

/// Find all executables on PATH (including extra sbin dirs) that start with `prefix`.
fn complete_commands(prefix: &str) -> Vec<String> {
    let mut matches: Vec<String> = BUILTINS
        .iter()
        .filter(|b| b.starts_with(prefix))
        .map(|b| b.to_string())
        .collect();

    let path_str = full_path();
    let mut seen = std::collections::HashSet::new();

    for dir in path_str.split(':') {
        if dir.is_empty() { continue; }
        let Ok(entries) = fs::read_dir(dir) else { continue };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(prefix) { continue; }
            if !seen.insert(name.clone()) { continue; }
            // Only list executable files
            if let Ok(meta) = entry.metadata() {
                if !meta.is_dir() && meta.permissions().mode() & 0o111 != 0 {
                    matches.push(name);
                }
            }
        }
    }

    matches
}

/// Complete a filesystem path. Appends `/` to directories so the next Tab
/// continues into the directory automatically.
fn complete_path(prefix: &str) -> Vec<String> {
    let (dir_part, file_prefix) = if let Some(pos) = prefix.rfind('/') {
        (&prefix[..=pos], &prefix[pos + 1..])
    } else {
        ("", prefix)
    };

    let search_dir = if dir_part.is_empty() { "." } else { dir_part };

    let mut matches = Vec::new();
    let Ok(entries) = fs::read_dir(search_dir) else { return matches };

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden files unless the prefix explicitly starts with '.'
        if name.starts_with('.') && !file_prefix.starts_with('.') {
            continue;
        }
        if name.starts_with(file_prefix) {
            let mut full = format!("{}{}", dir_part, name);
            if entry.path().is_dir() {
                full.push('/');
            }
            matches.push(full);
        }
    }

    matches
}
