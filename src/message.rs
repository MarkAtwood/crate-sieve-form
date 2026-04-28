// SPDX-License-Identifier: MIT

//! RFC 5322 message header extraction utilities.

use std::borrow::Cow;

/// The part of an RFC 5322 address to extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddressPart {
    All,
    LocalPart,
    Domain,
}

/// Extract all headers from raw RFC 5322 message bytes.
///
/// Returns `Vec<(lowercase_name, value)>`.  Folds continuation lines
/// (lines whose first character is whitespace) into the preceding header's
/// value.  Stops at the first blank line (the header/body separator).
///
/// Non-UTF-8 bytes are replaced with the Unicode replacement character so
/// that the function never fails on a structurally legal but non-ASCII
/// message.
pub fn extract_headers(raw: &[u8]) -> Vec<(String, String)> {
    // Find the header/body separator so we convert only the header section.
    // Prefer \r\n\r\n; fall back to \n\n.
    let header_bytes = if let Some(pos) = find_header_end(raw) {
        &raw[..pos]
    } else {
        raw
    };
    let text = String::from_utf8_lossy(header_bytes);
    let mut headers: Vec<(String, String)> = Vec::new();

    for line in text.split('\n') {
        // Strip a trailing CR so we handle both CRLF and LF line endings.
        let line = line.strip_suffix('\r').unwrap_or(line);

        // Blank line = end of headers.
        if line.is_empty() {
            break;
        }

        // Continuation line: starts with whitespace.
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = headers.last_mut() {
                last.1.push(' ');
                last.1.push_str(line.trim());
            }
            continue;
        }

        // New header: must contain ':'.
        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim().to_ascii_lowercase();
            let value = line[colon + 1..].trim().to_string();
            if !name.is_empty() {
                headers.push((name, value));
            }
        }
        // Lines with no ':' and no leading whitespace are malformed; skip.
    }

    headers
}

/// Extract one part of an RFC 5322 address string.
///
/// The address is first stripped of angle-bracket delimiters and any
/// display-name prefix before extracting the part.  Returns `""` on a
/// malformed address when `LocalPart` or `Domain` was requested.
pub(crate) fn address_part(addr: &str, part: AddressPart) -> String {
    // Normalise: strip display name and angle brackets.
    let bare = bare_address(addr);

    match part {
        AddressPart::LocalPart => bare
            .rfind('@')
            .map_or_else(String::new, |at| bare[..at].to_string()),
        AddressPart::Domain => bare
            .rfind('@')
            .map_or_else(String::new, |at| bare[at + 1..].to_string()),
        AddressPart::All => bare.into_owned(),
    }
}

/// Return the bare `local@domain` address from an RFC 5322 address string,
/// stripping any display name, angle brackets, and trailing comments.
fn bare_address(addr: &str) -> Cow<'_, str> {
    let addr = addr.trim();
    // If there is a `<...>` section, use what is inside it.
    if let Some(start) = addr.rfind('<') {
        if let Some(end) = addr[start..].find('>') {
            return Cow::Owned(addr[start + 1..start + end].trim().to_string());
        }
    }
    // No angle brackets — strip any trailing RFC 5322 comment `(...)` or
    // display name that follows the address.  A bare `(` not inside a quoted
    // string means everything from there onward is a comment.
    strip_trailing_comment(addr)
}

/// Strip a trailing RFC 5322 comment `(...)` from an unquoted address string.
///
/// Finds the first `(` that is not inside a `"..."` quoted string and
/// truncates the string there, then trims any remaining whitespace.
fn strip_trailing_comment(s: &str) -> Cow<'_, str> {
    let mut in_quotes = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '(' if !in_quotes => return Cow::Owned(s[..i].trim().to_string()),
            _ => {}
        }
    }
    Cow::Borrowed(s)
}

/// Find the byte offset of the header/body separator in a raw RFC 5322
/// message.  Returns the offset of the *first byte* of the separator
/// (i.e. the caller should use `&raw[..pos]` to get just the headers).
///
/// Searches for `\r\n\r\n` first, then `\n\n`.
fn find_header_end(raw: &[u8]) -> Option<usize> {
    // Look for \r\n\r\n first.
    let crlf = (0..raw.len().saturating_sub(3)).find(|&i| {
        raw[i] == b'\r' && raw[i + 1] == b'\n' && raw[i + 2] == b'\r' && raw[i + 3] == b'\n'
    });
    if crlf.is_some() {
        return crlf;
    }
    // Fall back to \n\n.
    (0..raw.len().saturating_sub(1)).find(|&i| raw[i] == b'\n' && raw[i + 1] == b'\n')
}

/// Split a header value containing one or more RFC 5322 addresses into
/// individual address strings.
///
/// Splits on `,` while respecting `"..."` quoted strings and `(...)`
/// comments — commas inside those are not treated as separators.
pub(crate) fn split_addresses(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0; // nesting depth of `(...)` comments
    let mut in_quotes = false;

    for ch in s.chars() {
        match ch {
            '"' if depth == 0 => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '(' if !in_quotes => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_quotes && depth > 0 => {
                depth -= 1;
                current.push(ch);
            }
            ',' if !in_quotes && depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }
    result
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- split_addresses (issue 10p.33) ---

    #[test]
    fn split_addresses_single() {
        // Single address — no splitting occurs.
        let got = split_addresses("alice@example.com");
        assert_eq!(got, vec!["alice@example.com"]);
    }

    #[test]
    fn split_addresses_two_bare() {
        // Two bare addresses separated by comma+space.
        let got = split_addresses("alice@example.com, bob@example.com");
        assert_eq!(got, vec!["alice@example.com", "bob@example.com"]);
    }

    #[test]
    fn split_addresses_angle_bracket_form() {
        // Angle-bracket form with display names.
        let got = split_addresses("Alice <alice@example.com>, Bob <bob@example.com>");
        assert_eq!(
            got,
            vec!["Alice <alice@example.com>", "Bob <bob@example.com>"]
        );
    }

    #[test]
    fn split_addresses_comma_inside_quoted_display_name() {
        // Comma inside a quoted display name must not split.
        let got = split_addresses("\"Doe, Jane\" <jane@example.com>, bob@example.com");
        assert_eq!(
            got,
            vec!["\"Doe, Jane\" <jane@example.com>", "bob@example.com"]
        );
    }

    #[test]
    fn split_addresses_comma_inside_comment() {
        // Comma inside a `(...)` comment must not split.
        let got = split_addresses("alice@example.com (Alice, A.), bob@example.com");
        assert_eq!(
            got,
            vec!["alice@example.com (Alice, A.)", "bob@example.com"]
        );
    }

    // --- bare_address / strip_trailing_comment (issue 10p.34) ---

    #[test]
    fn address_part_bare_with_trailing_comment() {
        // "user@example.com (Display Name)" — comment must be stripped.
        let got = address_part("user@example.com (Display Name)", AddressPart::All);
        assert_eq!(got, "user@example.com");
    }

    #[test]
    fn address_part_bare_with_no_comment() {
        // Plain bare address — unchanged.
        let got = address_part("user@example.com", AddressPart::All);
        assert_eq!(got, "user@example.com");
    }

    #[test]
    fn address_part_angle_bracket_ignores_display_name_comment() {
        // Angle-bracket form — display name and outer comment are stripped by
        // extracting from inside <>.
        let got = address_part("Display Name <user@example.com>", AddressPart::All);
        assert_eq!(got, "user@example.com");
    }
}
