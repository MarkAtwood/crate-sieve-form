// SPDX-License-Identifier: MIT

/// A parse error from the lexer or form parser.
///
/// `line` and `col` are 1-based positions set by the lexer.  The form parser
/// does not track positions, so form-layer errors always have `line == None`.
/// When `line == None`, no source location is available.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // line/col are None when the error source has no position tracking (form layer).
        // Only include them when they carry real information.
        if let Some(line) = self.line {
            write!(
                f,
                "parse error at {}:{}: {}",
                line,
                self.col.unwrap_or(0),
                self.message
            )
        } else {
            write!(f, "parse error: {}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// A typed error returned by [`crate::compile`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SieveError {
    /// Human-readable description of the error, suitable for display to an end user.
    pub message: String,
    /// Structured category of the error, suitable for programmatic matching.
    /// Use this field — not the message string — to distinguish error kinds.
    pub kind: SieveErrorKind,
    /// The underlying parse error, if any.
    ///
    /// Accessible to external callers via [`std::error::Error::source`], which
    /// returns it as `&dyn std::error::Error`.  Use
    /// `err.source().and_then(|e| e.downcast_ref::<ParseError>())` if you need
    /// the concrete type (requires re-exporting `ParseError` as a public type,
    /// which this crate does via `pub use`).
    pub(crate) source: Option<ParseError>,
}

/// The category of error produced by [`crate::compile`].
///
/// Marked `#[non_exhaustive]` so that adding new variants as more Sieve
/// extensions are implemented does not break callers' existing match arms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// A command requiring an extension was used without a `require` declaration
    /// (RFC 5228 §6.1). The contained string names the extension.
    MissingRequire(String),
    /// A regex pattern in the script failed to compile.
    InvalidRegex(String),
}

impl std::fmt::Display for SieveErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SieveErrorKind::Utf8 => write!(f, "invalid UTF-8"),
            SieveErrorKind::Lex => write!(f, "lexer error"),
            SieveErrorKind::Parse => write!(f, "parse error"),
            SieveErrorKind::UnsupportedExtension(ext) => {
                write!(f, "unsupported extension: {ext}")
            }
            SieveErrorKind::UnsupportedComparator(cmp) => {
                write!(f, "unsupported comparator: {cmp}")
            }
            SieveErrorKind::MissingRequire(ext) => {
                write!(f, "missing require declaration for: {ext}")
            }
            SieveErrorKind::InvalidRegex(pat) => {
                write!(f, "invalid regex pattern: {pat:?}")
            }
        }
    }
}

impl std::fmt::Display for SieveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SieveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

impl From<ParseError> for SieveError {
    fn from(e: ParseError) -> Self {
        // Invariant: the lexer always sets line/col; the form parser never does
        // (it has no position tracking). So line.is_some() reliably identifies
        // the error source. See ParseError doc comment.
        let kind = if e.line.is_some() {
            SieveErrorKind::Lex
        } else {
            SieveErrorKind::Parse
        };
        SieveError {
            message: e.to_string(),
            kind,
            source: Some(e),
        }
    }
}
