// SPDX-License-Identifier: MIT

//! Tokenizer for the Sieve scripting language (RFC 5228).

use crate::parse_error::ParseError;

/// Tokens produced by the Sieve lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// An identifier: `[a-zA-Z_][a-zA-Z0-9_]*`
    Word(String),
    /// A tagged argument with the leading `:` stripped: `:is` → `Tag("is")`
    Tag(String),
    /// A string literal with escape sequences resolved.
    StringLit(String),
    /// A numeric literal, with optional size multiplier already applied.
    Number(u64),
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Comma,
}

/// Tokenize a Sieve script source string into a flat token list.
///
/// # Errors
///
/// Returns a [`ParseError`] on any unrecognised character or malformed token.
pub fn tokenize(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut chars = src.chars().peekable();
    // 1-based line/col tracking for error messages.
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! err {
        ($msg:expr) => {
            return Err(ParseError {
                message: $msg.into(),
                line: Some(line),
                col: Some(col),
            })
        };
    }

    let mut tokens: Vec<Token> = Vec::new();

    while let Some(&ch) = chars.peek() {
        // --- Whitespace ---
        if ch.is_ascii_whitespace() {
            advance(&mut chars, &mut line, &mut col);
            continue;
        }

        // --- Line comment ---
        if ch == '#' {
            while chars.peek().is_some_and(|&c| c != '\n') {
                advance(&mut chars, &mut line, &mut col);
            }
            continue;
        }

        // --- Block comment ---
        // Consume '/' first, then check if next char is '*'.
        if ch == '/' {
            let err_line = line;
            let err_col = col;
            advance(&mut chars, &mut line, &mut col); // consume '/'
            if chars.peek() == Some(&'*') {
                advance(&mut chars, &mut line, &mut col); // consume '*'
                loop {
                    if chars.peek().is_none() {
                        return Err(ParseError {
                            message: "unterminated block comment".to_owned(),
                            line: Some(err_line),
                            col: Some(err_col),
                        });
                    }
                    let c = advance(&mut chars, &mut line, &mut col);
                    if c == '*' && chars.peek() == Some(&'/') {
                        advance(&mut chars, &mut line, &mut col); // consume '/'
                        break;
                    }
                }
                continue;
            }
            // Not a block comment — fall through to the error path.
            return Err(ParseError {
                message: "unexpected character '/'".to_owned(),
                line: Some(err_line),
                col: Some(err_col),
            });
        }

        // --- Punctuation ---
        match ch {
            '[' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::LBracket);
                continue;
            }
            ']' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::RBracket);
                continue;
            }
            '(' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::LParen);
                continue;
            }
            ')' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::RParen);
                continue;
            }
            '{' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::LBrace);
                continue;
            }
            '}' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::RBrace);
                continue;
            }
            ';' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::Semicolon);
                continue;
            }
            ',' => {
                advance(&mut chars, &mut line, &mut col);
                tokens.push(Token::Comma);
                continue;
            }
            _ => {}
        }

        // --- Tag: colon followed by identifier ---
        if ch == ':' {
            let err_line = line;
            let err_col = col;
            advance(&mut chars, &mut line, &mut col); // consume ':'
            if !chars
                .peek()
                .is_some_and(|&c| c.is_ascii_alphabetic() || c == '_')
            {
                return Err(ParseError {
                    message: "expected identifier after ':'".to_owned(),
                    line: Some(err_line),
                    col: Some(err_col),
                });
            }
            let mut ident = String::new();
            while chars
                .peek()
                .is_some_and(|&c| c.is_ascii_alphanumeric() || c == '_')
            {
                ident.push(advance(&mut chars, &mut line, &mut col));
            }
            tokens.push(Token::Tag(ident));
            continue;
        }

        // --- Number ---
        if ch.is_ascii_digit() {
            let mut num_str = String::new();
            while chars.peek().is_some_and(|&c| c.is_ascii_digit()) {
                num_str.push(advance(&mut chars, &mut line, &mut col));
            }
            let base: u64 = num_str.parse().map_err(|_| ParseError {
                message: format!("number overflow: {num_str}"),
                line: Some(line),
                col: Some(col),
            })?;
            let multiplier: u64 = match chars.peek().copied() {
                Some('K' | 'k') => {
                    advance(&mut chars, &mut line, &mut col);
                    1024
                }
                Some('M' | 'm') => {
                    advance(&mut chars, &mut line, &mut col);
                    1024 * 1024
                }
                Some('G' | 'g') => {
                    advance(&mut chars, &mut line, &mut col);
                    1024 * 1024 * 1024
                }
                _ => 1,
            };
            let value = base.checked_mul(multiplier).ok_or_else(|| ParseError {
                message: format!("number overflow applying multiplier to {base}"),
                line: Some(line),
                col: Some(col),
            })?;
            tokens.push(Token::Number(value));
            continue;
        }

        // --- Word: identifier ---
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut word = String::new();
            while chars
                .peek()
                .is_some_and(|&c| c.is_ascii_alphanumeric() || c == '_')
            {
                word.push(advance(&mut chars, &mut line, &mut col));
            }
            // Check for multiline string: word "text" followed immediately by ':'
            if word == "text" && chars.peek() == Some(&':') {
                let err_line = line;
                let err_col = col;
                advance(&mut chars, &mut line, &mut col); // consume ':'
                                                          // RFC 5228 §2.3.1: optional whitespace then optional hash
                                                          // comment are permitted on the `text:` header line before
                                                          // the mandatory newline.  Consume `[ \t]*` then `#[^\n]*`.
                while chars.peek().is_some_and(|&c| c == ' ' || c == '\t') {
                    advance(&mut chars, &mut line, &mut col);
                }
                if chars.peek() == Some(&'#') {
                    while chars.peek().is_some_and(|&c| c != '\n') {
                        advance(&mut chars, &mut line, &mut col);
                    }
                }
                // Consume optional CR/LF or CRLF to end the `text:` header line.
                if chars.peek() == Some(&'\r') {
                    advance(&mut chars, &mut line, &mut col);
                }
                if chars.peek() == Some(&'\n') {
                    advance(&mut chars, &mut line, &mut col);
                } else {
                    return Err(ParseError {
                        message: "expected newline after 'text:'".to_owned(),
                        line: Some(err_line),
                        col: Some(err_col),
                    });
                }
                // Collect lines until a line that is exactly "." (RFC 5228 §2.3.1).
                let mut content = String::new();
                loop {
                    if chars.peek().is_none() {
                        return Err(ParseError {
                            message: "unterminated multiline string (missing '.' terminator)"
                                .to_owned(),
                            line: Some(err_line),
                            col: Some(err_col),
                        });
                    }
                    // Read a full line.
                    let mut line_buf = String::new();
                    while chars.peek().is_some_and(|&c| c != '\n') {
                        line_buf.push(advance(&mut chars, &mut line, &mut col));
                    }
                    // Consume the newline.
                    if chars.peek().is_some() {
                        advance(&mut chars, &mut line, &mut col); // '\n'
                    }
                    // Strip trailing CR if present (CRLF line ending).
                    let line_trimmed = line_buf.strip_suffix('\r').unwrap_or(&line_buf);
                    // Terminator line.
                    if line_trimmed == "." {
                        break;
                    }
                    // Dot-stuffing: a leading ".." becomes ".".
                    let stored = if let Some(rest) = line_trimmed.strip_prefix("..") {
                        format!(".{rest}")
                    } else {
                        line_trimmed.to_string()
                    };
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&stored);
                }
                tokens.push(Token::StringLit(content));
                continue;
            }
            tokens.push(Token::Word(word));
            continue;
        }

        // --- Quoted string ---
        if ch == '"' {
            let err_line = line;
            let err_col = col;
            advance(&mut chars, &mut line, &mut col); // consume opening '"'
            let mut s = String::new();
            loop {
                if chars.peek().is_none() {
                    return Err(ParseError {
                        message: "unterminated string literal".to_owned(),
                        line: Some(err_line),
                        col: Some(err_col),
                    });
                }
                let c = advance(&mut chars, &mut line, &mut col);
                if c == '"' {
                    break;
                }
                if c == '\\' {
                    if chars.peek().is_none() {
                        return Err(ParseError {
                            message: "unexpected end of input after backslash".to_owned(),
                            line: Some(err_line),
                            col: Some(err_col),
                        });
                    }
                    let escaped = advance(&mut chars, &mut line, &mut col);
                    match escaped {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        // RFC 5228 §2.3.1: only \" and \\ are defined escape sequences;
                        // other backslash sequences pass through unchanged.
                        other => {
                            s.push('\\');
                            s.push(other);
                        }
                    }
                    continue;
                }
                s.push(c);
            }
            tokens.push(Token::StringLit(s));
            continue;
        }

        err!(format!("unexpected character '{ch}'"));
    }

    Ok(tokens)
}

// Consume one character from the iterator, updating line/col tracking.
// called only when chars.peek().is_some() is guaranteed
fn advance(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    line: &mut usize,
    col: &mut usize,
) -> char {
    let ch = chars.next().expect("advance called on empty iterator");
    if ch == '\n' {
        *line += 1;
        *col = 1;
    } else {
        *col += 1;
    }
    ch
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_text_with_hash_comment() {
        // RFC 5228 §2.3.1 allows an optional hash comment on the text: line
        let src = "text: # optional comment\nfoo\n.\n";
        let tokens = tokenize(src).expect("should tokenize");
        assert_eq!(tokens, vec![Token::StringLit("foo".into())]);
    }
}
