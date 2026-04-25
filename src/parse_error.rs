// SPDX-License-Identifier: MIT

/// A parse error with source location.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at {}:{}: {}",
            self.line, self.col, self.message
        )
    }
}

/// A typed error returned by [`crate::compile`].
#[derive(Debug, Clone)]
pub struct SieveError {
    pub message: String,
    pub kind: SieveErrorKind,
}

/// The category of error produced by [`crate::compile`].
#[derive(Debug, Clone)]
pub enum SieveErrorKind {
    /// The script bytes are not valid UTF-8.
    Utf8,
    /// The lexer rejected the source text.
    Lex,
    /// The form parser rejected the token stream.
    Parse,
    /// The script requires an unsupported Sieve extension.
    UnsupportedExtension(String),
    /// A regex pattern in the script failed to compile.
    InvalidRegex(String),
}

impl std::fmt::Display for SieveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SieveError {}
