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
    // Find the last `<` that is NOT inside a `(...)` comment.
    // Scan forward tracking paren depth; record position each time we see a
    // `<` at depth 0.  This prevents a `<` embedded inside a comment such as
    // `user@example.com (secretary < corp > dept)` from being treated as an
    // angle-bracket delimiter.
    let mut paren_depth: usize = 0;
    let mut last_angle: Option<usize> = None;
    for (i, ch) in addr.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' if paren_depth > 0 => paren_depth -= 1,
            '<' if paren_depth == 0 => last_angle = Some(i),
            _ => {}
        }
    }
    if let Some(start) = last_angle {
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
/// Searches for both `\r\n\r\n` and `\n\n` and returns whichever occurs
/// first, so that a later occurrence of the other pattern in the message
/// body is never mistaken for the header/body separator.
fn find_header_end(raw: &[u8]) -> Option<usize> {
    let crlf = raw.windows(4).position(|w| w == b"\r\n\r\n");
    let lf = raw.windows(2).position(|w| w == b"\n\n");
    match (crlf, lf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// Split a header value containing one or more RFC 5322 addresses into
/// individual address strings.
///
/// Splits on `,` while respecting `"..."` quoted strings and `(...)`
/// comments — commas inside those are not treated as separators.
pub(crate) fn split_addresses(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;
    let mut in_quotes = false;
    let mut prev_backslash = false;

    for ch in s.chars() {
        if prev_backslash {
            prev_backslash = false;
            current.push(ch);
            continue;
        }
        match ch {
            '\\' if in_quotes => {
                prev_backslash = true;
                current.push(ch);
            }
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

    #[test]
    fn split_addresses_backslash_escaped_quote_in_display_name() {
        // \"Doe, Jane\" has a comma inside an escaped-quote display name.
        // The escaped \" must not close the quoted context.
        let got = split_addresses("\"Doe, \\\"Jane\\\"\" <jane@example.com>, bob@example.com");
        assert_eq!(
            got,
            vec![
                "\"Doe, \\\"Jane\\\"\" <jane@example.com>",
                "bob@example.com"
            ]
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

    #[test]
    fn find_header_end_lf_body_contains_crlf() {
        // Regression: message uses bare LF line endings, but the body
        // happens to contain \r\n\r\n.  The separator must be the \n\n
        // at offset 20, not the \r\n\r\n at offset 30 in the body.
        let msg = b"Subject: test\nFoo: bar\n\nBody starts here.\r\n\r\nMore body.";
        let pos = find_header_end(msg).unwrap();
        // \n\n is at byte offset 22 ("Subject: test\nFoo: bar" = 22 bytes)
        assert_eq!(pos, 22, "should pick the earlier \\n\\n, not the later \\r\\n\\r\\n");
        assert_eq!(&msg[..pos], b"Subject: test\nFoo: bar");
    }

    #[test]
    fn find_header_end_crlf_only() {
        let msg = b"Subject: test\r\nFoo: bar\r\n\r\nBody here.";
        let pos = find_header_end(msg).unwrap();
        assert_eq!(&msg[..pos], b"Subject: test\r\nFoo: bar");
    }

    #[test]
    fn find_header_end_lf_only() {
        let msg = b"Subject: test\nFoo: bar\n\nBody here.";
        let pos = find_header_end(msg).unwrap();
        assert_eq!(&msg[..pos], b"Subject: test\nFoo: bar");
    }

    #[test]
    fn find_header_end_no_separator() {
        let msg = b"Subject: test\nFoo: bar";
        assert_eq!(find_header_end(msg), None);
    }

    #[test]
    fn bare_address_ignores_angle_in_comment() {
        // The < inside the comment must not be used as an angle-bracket delimiter
        let got = address_part(
            "user@example.com (secretary < corp > dept)",
            AddressPart::All,
        );
        assert_eq!(got, "user@example.com");
    }
}
