/// String literal escaping and unescaping for Melbi syntax.
///
/// This module provides utilities for converting between:
/// - Runtime strings (e.g., "hello\n" with actual newline character)
/// - Melbi source code string literals (e.g., "hello\n" with backslash-n sequence)
use crate::{String, format, vec};

// TODO: Consider using single quotes if the string contains double quotes.

/// Escape special characters in strings for Melbi string literals.
///
/// Converts runtime strings to their source code representation by escaping:
/// - `"` → `\"`
/// - `\` → `\\`
/// - `\n` → `\n`
/// - `\r` → `\r`
/// - `\t` → `\t`
/// - Control characters → `\u{xxxx}`
///
/// # Example
///
/// ```ignore
/// let s = "hello\nworld";
/// assert_eq!(escape_string(s), r#"hello\nworld"#);
/// ```
pub fn escape_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            c if c.is_control() => format!("\\u{{{:04x}}}", c as u32).chars().collect(),
            c => vec![c],
        })
        .collect()
}

// TODO: Implement unescape_string for parser use
// pub fn unescape_string(s: &str) -> Result<String, UnescapeError> {
//     ...
// }
