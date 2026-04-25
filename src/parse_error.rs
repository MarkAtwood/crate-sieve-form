// SPDX-License-Identifier: MIT

/// A parse error from the lexer or form parser.
///
/// `line` and `col` are 1-based positions set by the lexer.  The form parser
/// does not track positions, so form-layer errors always have `line == 0`.
/// When `line == 0`, no source location is available.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // line/col are 0 when the error source has no position tracking (form layer).
        // Only include them when they carry real information.
        if self.line > 0 {
            write!(f, "parse error at {}:{}: {}", self.line, self.col, self.message)
        } else {
            write!(f, "parse error: {}", self.message)
        }
    }
}

/// A typed error returned by [`crate::compile`].
#[derive(Debug, Clone)]
pub struct SieveError {
    pub message: String,
    pub kind: SieveErrorKind,
}

/// The category of error produced by [`crate::compile`].
///
/// Marked `#[non_exhaustive]` so that adding new variants as more Sieve
/// extensions are implemented does not break callers' existing match arms.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SieveErrorKind {
    /// The script bytes are not valid UTF-8.
    Utf8,
    /// The lexer rejected the source text.
    Lex,
    /// The form parser rejected the token stream.
    Parse,
    /// The script requires an unsupported Sieve extension.
    UnsupportedExtension(String),
    /// The script uses an unsupported comparator (RFC 5228 §2.7.2).
    UnsupportedComparator(String),
    /// A regex pattern in the script failed to compile.
    InvalidRegex(String),
}

impl std::fmt::Display for SieveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SieveError {}
