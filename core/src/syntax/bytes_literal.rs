/// Bytes literal escaping and unescaping for Melbi syntax.
///
/// This module provides utilities for converting between:
/// - Runtime bytes (e.g., `[104, 101, 108, 108, 111]` for "hello")
/// - Melbi source code bytes literals (e.g., `b"hello"` or `b"\x68\x65\x6c\x6c\x6f"`)

use core::fmt;

// TODO: Consider using single quotes if the bytes literal contains double quotes.

/// Escape bytes for Melbi bytes literal representation.
///
/// This function formats bytes to be human-readable when possible:
/// - Printable ASCII characters (0x20-0x7E) are shown directly
/// - Common escape sequences use backslash notation (`\n`, `\r`, `\t`, `\"`, `\\`)
/// - Non-printable bytes use hex notation (`\xNN`)
///
/// # Example
///
/// ```ignore
/// let bytes = b"hello\nworld";
/// let mut output = String::new();
/// escape_bytes(&mut output, bytes).unwrap();
/// assert_eq!(output, r#"hello\nworld"#);
/// ```
pub fn escape_bytes(f: &mut impl fmt::Write, bytes: &[u8]) -> fmt::Result {
    for &byte in bytes {
        match byte {
            b'"' => write!(f, "\\\"")?,
            b'\\' => write!(f, "\\\\")?,
            b'\n' => write!(f, "\\n")?,
            b'\r' => write!(f, "\\r")?,
            b'\t' => write!(f, "\\t")?,
            // Printable ASCII characters (excluding control characters)
            0x20..=0x7E => write!(f, "{}", byte as char)?,
            // Non-printable bytes as hex
            _ => write!(f, "\\x{:02x}", byte)?,
        }
    }
    Ok(())
}

// TODO: Implement unescape_bytes for parser use
// pub fn unescape_bytes(s: &str) -> Result<Vec<u8>, UnescapeError> {
//     ...
// }
