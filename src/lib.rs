// SPDX-License-Identifier: MIT

//! Sieve email filter language (RFC 5228 + RFC 5229 variables extension).
//!
//! The core contribution of this crate is the *form layer*: a uniform,
//! recursive representation of Sieve scripts inspired by Lisp forms.  Every
//! Sieve statement is a flat `Vec<Form>`, making the parsed AST easy to
//! inspect, serialize, and extend without touching the parser.
//!
//! The only external dependency is [`fancy-regex`](https://crates.io/crates/fancy-regex),
//! used for the `:regex` match type and Sieve glob-to-regex conversion.
//!
//! ## Internal pipeline
//!
//! 1. [`lexer::tokenize`] — raw source → `Vec<Token>`
//! 2. [`form::read_script`] — tokens → `Script` (a uniform form tree)
//! 3. evaluator — `Script` + message → `Vec<SieveAction>` (internal, not pub)

use std::collections::HashMap;
use std::sync::Arc;

pub mod form;
pub mod lexer;
pub mod parse_error;

pub use parse_error::{ParseError, SieveError, SieveErrorKind};

mod evaluator;
mod message;

/// A compiled Sieve script, ready for evaluation.
///
/// Opaque to callers; contains the parsed form tree and a pre-compiled regex
/// cache.  `Send + Sync` because all contained types are `Send + Sync`.
#[derive(Clone)]
pub struct CompiledScript {
    script: Arc<form::Script>,
    regex_cache: Arc<HashMap<String, fancy_regex::Regex>>,
    pub(crate) variables_enabled: bool,
}

impl std::fmt::Debug for CompiledScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledScript")
            .field(
                "script",
                &format!("Script {{ commands: {} }}", self.script.len()),
            )
            .field(
                "regex_cache",
                &format!("<{} compiled patterns>", self.regex_cache.len()),
            )
            .finish()
    }
}

// Explicit assertion that CompiledScript is Send + Sync.
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<CompiledScript>();
    }
    let _ = check;
};

/// Disposition returned after evaluating a Sieve script against a message.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SieveAction {
    Keep,
    FileInto(String),
    Discard,
    Reject(String),
    /// Forward the message to an SMTP envelope recipient address.
    ///
    /// The contained string is the SMTP envelope recipient address.  Forwarding
    /// to that address is mandatory per RFC 5228 §4.4.  The address has not been
    /// validated beyond what the Sieve script provides.
    Redirect(String),
}

/// Compile a Sieve script from raw source bytes.
///
/// The bytes must be valid UTF-8.  Returns `Err` with a human-readable
/// description on parse or compile failure, including unknown `require`
/// extensions.
///
/// # Errors
///
/// Returns `Err` if the script is not valid UTF-8, if tokenising or parsing
/// fails, or if the script requires an unsupported extension.
///
/// # Implementation notes
///
/// Unknown commands encountered during evaluation are silently ignored per
/// RFC 5228 §2.9.  Callers should not rely on unknown commands having any
/// effect.
///
/// There are no built-in size limits on script size; callers are responsible
/// for bounding inputs before calling this function.
///
/// # Security
///
/// The `:regex` match type uses `fancy-regex`, which supports backtracking
/// for extended patterns.  Untrusted `:regex` patterns can cause catastrophic
/// backtracking (ReDoS).  Callers should validate pattern complexity or
/// restrict who can supply `:regex` tests.
pub fn compile(script: &[u8]) -> Result<CompiledScript, SieveError> {
    let source = std::str::from_utf8(script).map_err(|e| SieveError {
        message: format!("invalid UTF-8: {e}"),
        kind: SieveErrorKind::Utf8,
        source: None,
    })?;
    let tokens = lexer::tokenize(source).map_err(SieveError::from)?;
    let parsed = form::read_script(&tokens).map_err(SieveError::from)?;

    // Collect declared extensions and validate them against the set the
    // evaluator implements.  The canonical list lives in the evaluator so
    // adding a new extension only requires updating one place.
    let mut declared_extensions: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for stmt in &parsed {
        if let [form::Form::Word(w), rest @ ..] = stmt.as_slice() {
            if w == "require" {
                for f in rest {
                    match f {
                        form::Form::Str(s) => {
                            if !evaluator::KNOWN_EXTENSIONS.contains(&s.as_str()) {
                                return Err(SieveError {
                                    message: format!("unsupported Sieve extension: {s}"),
                                    kind: SieveErrorKind::UnsupportedExtension(s.to_string()),
                                    source: None,
                                });
                            }
                            declared_extensions.insert(s.clone());
                        }
                        form::Form::StringList(v) => {
                            for s in v {
                                if !evaluator::KNOWN_EXTENSIONS.contains(&s.as_str()) {
                                    return Err(SieveError {
                                        message: format!("unsupported Sieve extension: {s}"),
                                        kind: SieveErrorKind::UnsupportedExtension(s.to_string()),
                                        source: None,
                                    });
                                }
                                declared_extensions.insert(s.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // RFC 5228 §6.1: extension commands must be declared before use.
    // Base commands (keep, discard, stop, redirect, if, elsif, else, require,
    // allof, anyof, not, header, address, envelope, exists, size) need no
    // require declaration.
    for stmt in &parsed {
        check_extension_use(stmt, &declared_extensions)?;
    }

    let variables_enabled = declared_extensions.contains("variables");

    validate_script(&parsed)?;

    let regex_cache = build_regex_cache(&parsed);

    Ok(CompiledScript {
        script: Arc::new(parsed),
        regex_cache,
        variables_enabled,
    })
}

/// Check that extension commands used in `stmt` have been declared in the
/// script's `require` list (RFC 5228 §6.1).
///
/// Extension commands that must be declared: `fileinto`, `reject`.
/// Recurses into blocks and test lists.
fn check_extension_use(
    stmt: &form::Stmt,
    declared: &std::collections::HashSet<String>,
) -> Result<(), SieveError> {
    // Extension commands that require a prior require declaration.
    const EXTENSION_COMMANDS: &[&str] = &["fileinto", "reject"];

    if let [form::Form::Word(w), ..] = stmt.as_slice() {
        if EXTENSION_COMMANDS.contains(&w.as_str()) && !declared.contains(w.as_str()) {
            return Err(SieveError {
                message: format!("extension command \"{w}\" used without require declaration"),
                kind: SieveErrorKind::MissingRequire(w.clone()),
                source: None,
            });
        }
    }

    // Recurse into blocks and test lists.
    for form in stmt.as_slice() {
        match form {
            form::Form::Block(stmts) => {
                for inner in stmts {
                    check_extension_use(inner, declared)?;
                }
            }
            form::Form::TestList(tests) => {
                for test in tests {
                    check_extension_use(test, declared)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Walk every statement in a script (recursing into blocks and test lists)
/// and enforce compile-time constraints:
///
/// - RFC 5228 §2.7.2: unknown comparator names must fail the script.
///   Known comparators: `"i;ascii-casemap"` and `"i;octet"`.
/// - Regex extension: invalid regex patterns must fail the script so that
///   broken patterns are caught early rather than silently failing at eval time.
fn validate_script(script: &form::Script) -> Result<(), SieveError> {
    for stmt in script {
        validate_stmt(stmt)?;
    }
    Ok(())
}

fn validate_regex_in_clause(clause: &[form::Form]) -> Result<(), SieveError> {
    let last_str_pos = clause
        .iter()
        .rposition(|f| matches!(f, form::Form::Str(_) | form::Form::StringList(_)));
    if let Some(pos) = last_str_pos {
        match &clause[pos] {
            form::Form::Str(pattern) => {
                let anchored = format!("(?s)\\A(?:{pattern})\\z");
                fancy_regex::Regex::new(&anchored).map_err(|e| SieveError {
                    message: format!("invalid regex pattern {pattern:?}: {e}"),
                    kind: SieveErrorKind::InvalidRegex(pattern.clone()),
                    source: None,
                })?;
            }
            form::Form::StringList(patterns) => {
                for pattern in patterns {
                    let anchored = format!("(?s)\\A(?:{pattern})\\z");
                    fancy_regex::Regex::new(&anchored).map_err(|e| SieveError {
                        message: format!("invalid regex pattern {pattern:?}: {e}"),
                        kind: SieveErrorKind::InvalidRegex(pattern.clone()),
                        source: None,
                    })?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_stmt(stmt: &form::Stmt) -> Result<(), SieveError> {
    // Scan the flat form list for Tag("comparator") followed by Str(name).
    // Also detect Tag("regex") and validate all Str patterns in the stmt.
    //
    // An if/elsif/else chain is represented as a single flat Stmt where clauses
    // are separated by Form::Block.  Regex state is scoped per clause so that
    // an invalid pattern in one clause is not shadowed by strings from another.
    let mut has_regex_tag = false;
    let mut clause_start = 0;
    let mut i = 0;
    while i < stmt.len() {
        match &stmt[i] {
            form::Form::Tag(t) if t == "comparator" => {
                // The next form must be the comparator name string.
                if let Some(form::Form::Str(name)) = stmt.get(i + 1) {
                    const KNOWN_COMPARATORS: &[&str] = &["i;ascii-casemap", "i;octet"];
                    if !KNOWN_COMPARATORS.contains(&name.as_str()) {
                        return Err(SieveError {
                            message: format!("unsupported comparator: {name}"),
                            kind: SieveErrorKind::UnsupportedComparator(name.clone()),
                            source: None,
                        });
                    }
                    i += 2; // consume tag + name
                    continue;
                }
                // No Str follows the tag; advance past the tag only.
            }
            form::Form::Tag(t) if t == "regex" => {
                has_regex_tag = true;
            }
            form::Form::Block(stmts) => {
                // End of a clause: validate regex patterns collected since
                // clause_start, recurse into the block, then reset for the
                // next clause (elsif/else).
                if has_regex_tag {
                    validate_regex_in_clause(&stmt[clause_start..i])?;
                }
                for inner in stmts {
                    validate_stmt(inner)?;
                }
                has_regex_tag = false;
                clause_start = i + 1;
            }
            form::Form::TestList(tests) => {
                // Recurse into parenthesised test lists.
                for test in tests {
                    validate_stmt(test)?;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Validate regex for any trailing clause (after the last Block, or the
    // whole stmt when there is no Block).
    if has_regex_tag {
        validate_regex_in_clause(&stmt[clause_start..])?;
    }

    Ok(())
}

/// Whether patterns in a clause are raw regexes or Sieve glob patterns.
#[derive(Clone, Copy)]
enum PatternKind {
    Regex,
    Glob,
}

fn build_regex_cache(script: &form::Script) -> Arc<HashMap<String, fancy_regex::Regex>> {
    let mut cache = HashMap::new();
    for stmt in script {
        collect_regex_patterns(stmt, &mut cache);
    }
    Arc::new(cache)
}

fn cache_patterns_in_clause(
    clause: &[form::Form],
    kind: PatternKind,
    cache: &mut HashMap<String, fancy_regex::Regex>,
) {
    let last_pos = clause
        .iter()
        .rposition(|f| matches!(f, form::Form::Str(_) | form::Form::StringList(_)));
    if let Some(pos) = last_pos {
        match &clause[pos] {
            form::Form::Str(p) => match kind {
                PatternKind::Glob => cache_glob_pattern(p, cache),
                PatternKind::Regex => cache_pattern(p, cache),
            },
            form::Form::StringList(ps) => {
                for p in ps {
                    match kind {
                        PatternKind::Glob => cache_glob_pattern(p, cache),
                        PatternKind::Regex => cache_pattern(p, cache),
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_regex_patterns(stmt: &form::Stmt, cache: &mut HashMap<String, fancy_regex::Regex>) {
    let mut has_regex_tag = false;
    let mut has_matches_tag = false;
    let mut clause_start = 0;
    let mut i = 0;
    while i < stmt.len() {
        match &stmt[i] {
            form::Form::Tag(t) if t == "regex" => {
                has_regex_tag = true;
            }
            form::Form::Tag(t) if t == "matches" => {
                has_matches_tag = true;
            }
            form::Form::Block(stmts) => {
                if has_regex_tag {
                    cache_patterns_in_clause(&stmt[clause_start..i], PatternKind::Regex, cache);
                } else if has_matches_tag {
                    cache_patterns_in_clause(&stmt[clause_start..i], PatternKind::Glob, cache);
                }
                for inner in stmts {
                    collect_regex_patterns(inner, cache);
                }
                has_regex_tag = false;
                has_matches_tag = false;
                clause_start = i + 1;
            }
            form::Form::TestList(tests) => {
                for test in tests {
                    collect_regex_patterns(test, cache);
                }
            }
            _ => {}
        }
        i += 1;
    }
    if has_regex_tag {
        cache_patterns_in_clause(&stmt[clause_start..], PatternKind::Regex, cache);
    } else if has_matches_tag {
        cache_patterns_in_clause(&stmt[clause_start..], PatternKind::Glob, cache);
    }
}

fn cache_pattern(pattern: &str, cache: &mut HashMap<String, fancy_regex::Regex>) {
    let base = format!("(?s)\\A(?:{pattern})\\z");
    // Errors were already caught by validate_script; ignore here.
    if let Ok(re) = fancy_regex::Regex::new(&base) {
        cache.insert(base.clone(), re);
    }
    let ci = format!("(?i){base}");
    if let Ok(re) = fancy_regex::Regex::new(&ci) {
        cache.insert(ci, re);
    }
}

fn cache_glob_pattern(pattern: &str, cache: &mut HashMap<String, fancy_regex::Regex>) {
    let regex_str = evaluator::sieve_glob_to_regex(pattern);
    let base_key = format!("glob:{pattern}");
    if let Ok(re) = fancy_regex::Regex::new(&regex_str) {
        cache.insert(base_key.clone(), re);
    }
    let ci_str = format!("(?i){regex_str}");
    let ci_key = format!("(?i){base_key}");
    if let Ok(re) = fancy_regex::Regex::new(&ci_str) {
        cache.insert(ci_key, re);
    }
}

/// Evaluate a compiled Sieve script against a raw RFC 5322 message.
///
/// `envelope_from` and `envelope_to` are the SMTP envelope addresses.
/// Returns the list of actions the script requests; defaults to `[Keep]`
/// when the script produces no explicit disposition (RFC 5228 §2.10.2).
///
/// # Implementation notes
///
/// The current implementation returns at most one action.  A script that
/// takes an explicit action (`fileinto`, `discard`, `reject`, `redirect`)
/// returns that action; a script with no explicit action returns
/// `SieveAction::Keep`.  RFC 5228 permits multiple simultaneous actions
/// (e.g., keep + fileinto), but this is not yet implemented.
///
/// There are no built-in size limits on message size; callers are responsible
/// for bounding inputs before calling this function.
pub fn evaluate(
    script: &CompiledScript,
    raw_message: &[u8],
    envelope_from: &str,
    envelope_to: &str,
) -> Vec<SieveAction> {
    evaluator::eval_script(
        &script.script,
        &script.regex_cache,
        script.variables_enabled,
        raw_message,
        envelope_from,
        envelope_to,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use form::Form;
    use lexer::{tokenize, Token};

    // -----------------------------------------------------------------------
    // Lexer tests
    // -----------------------------------------------------------------------

    #[test]
    fn tokenize_basic() {
        let src = r#"if header :contains "Subject" "test" { keep; }"#;
        let tokens = tokenize(src).expect("tokenize failed");
        assert_eq!(
            tokens,
            vec![
                Token::Word("if".into()),
                Token::Word("header".into()),
                Token::Tag("contains".into()),
                Token::StringLit("Subject".into()),
                Token::StringLit("test".into()),
                Token::LBrace,
                Token::Word("keep".into()),
                Token::Semicolon,
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn tokenize_number_multipliers() {
        let tokens = tokenize("1K 2M").expect("tokenize failed");
        assert_eq!(
            tokens,
            vec![Token::Number(1024), Token::Number(2 * 1024 * 1024)]
        );
    }

    #[test]
    fn tokenize_quoted_string_escapes() {
        // Source: "hello \"world\""
        let tokens = tokenize(r#""hello \"world\"""#).expect("tokenize failed");
        assert_eq!(tokens, vec![Token::StringLit("hello \"world\"".into())]);
    }

    #[test]
    fn tokenize_line_comment_skipped() {
        let src = "keep # this is a comment\n;";
        let tokens = tokenize(src).expect("tokenize failed");
        assert_eq!(tokens, vec![Token::Word("keep".into()), Token::Semicolon]);
    }

    #[test]
    fn tokenize_block_comment_skipped() {
        let src = "keep /* ignore this */ ;";
        let tokens = tokenize(src).expect("tokenize failed");
        assert_eq!(tokens, vec![Token::Word("keep".into()), Token::Semicolon]);
    }

    // -----------------------------------------------------------------------
    // Multiline string test
    // -----------------------------------------------------------------------

    #[test]
    fn parse_multiline_string() {
        // RFC 5228 §2.3.1 multiline literal: text:\nfoo\n.\n
        let src = "text:\nfoo\n.\n";
        let tokens = tokenize(src).expect("tokenize failed");
        assert_eq!(tokens, vec![Token::StringLit("foo".into())]);
    }

    // -----------------------------------------------------------------------
    // Form / script parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_script_simple_if() {
        let src = r#"if header :contains "Subject" "x" { keep; }"#;
        let tokens = tokenize(src).expect("tokenize failed");
        let script = form::read_script(&tokens).expect("read_script failed");
        assert_eq!(script.len(), 1, "expected exactly 1 top-level statement");
        let stmt = &script[0];
        // First form is Word("if")
        assert!(matches!(&stmt[0], Form::Word(w) if w == "if"));
        // Second form is Word("header")
        assert!(matches!(&stmt[1], Form::Word(w) if w == "header"));
        // Third form is Tag("contains")
        assert!(matches!(&stmt[2], Form::Tag(t) if t == "contains"));
        // Fourth is Str("Subject"), fifth is Str("x")
        assert!(matches!(&stmt[3], Form::Str(s) if s == "Subject"));
        assert!(matches!(&stmt[4], Form::Str(s) if s == "x"));
        // Sixth form is Block containing [keep]
        assert!(matches!(&stmt[5], Form::Block(_)));
        if let Form::Block(block) = &stmt[5] {
            assert_eq!(block.len(), 1);
            assert!(matches!(&block[0][0], Form::Word(w) if w == "keep"));
        }
    }

    #[test]
    fn parse_error_unclosed_brace() {
        let src = "if true {";
        let tokens = tokenize(src).expect("tokenize failed");
        let result = form::read_script(&tokens);
        assert!(result.is_err(), "expected ParseError for unclosed brace");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("unclosed") || err.message.contains("missing"),
            "unexpected error message: {}",
            err.message
        );
    }

    #[test]
    fn parse_require() {
        let src = r#"require ["fileinto", "reject"];"#;
        let tokens = tokenize(src).expect("tokenize failed");
        let script = form::read_script(&tokens).expect("read_script failed");
        assert_eq!(script.len(), 1);
        let stmt = &script[0];
        assert!(matches!(&stmt[0], Form::Word(w) if w == "require"));
        assert!(
            matches!(&stmt[1], Form::StringList(v) if v == &["fileinto", "reject"]),
            "expected StringList([\"fileinto\", \"reject\"]), got {:?}",
            &stmt[1]
        );
    }

    // -----------------------------------------------------------------------
    // compile() integration smoke test
    // -----------------------------------------------------------------------

    #[test]
    fn compile_simple_script() {
        let src = b"require [\"fileinto\"];\nif header :contains \"X-Spam\" \"yes\" { fileinto \"Spam\"; }";
        let result = compile(src);
        assert!(result.is_ok(), "compile failed: {:?}", result.err());
    }

    #[test]
    fn compile_invalid_utf8() {
        let result = compile(b"\xff\xfe");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("UTF-8"));
    }

    // -----------------------------------------------------------------------
    // Evaluator tests
    // -----------------------------------------------------------------------

    fn make_msg(subject: &str) -> Vec<u8> {
        format!(
            "From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: {subject}\r\n\r\nBody.\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn eval_implicit_keep_empty_script() {
        let script = compile(b"").unwrap();
        let actions = evaluate(
            &script,
            &make_msg("test"),
            "sender@example.com",
            "recip@example.com",
        );
        assert_eq!(actions, vec![SieveAction::Keep]);
    }

    #[test]
    fn eval_explicit_keep() {
        let script = compile(b"keep;").unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::Keep]);
    }

    #[test]
    fn eval_discard() {
        let script = compile(b"discard;").unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::Discard]);
    }

    #[test]
    fn eval_fileinto_subject_match() {
        let script = compile(
            b"require [\"fileinto\"]; if header :contains \"Subject\" \"URGENT\" { fileinto \"INBOX.Urgent\"; }",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("URGENT: fix this"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("INBOX.Urgent".into())]);
    }

    #[test]
    fn eval_fileinto_subject_no_match() {
        let script = compile(
            b"require [\"fileinto\"]; if header :contains \"Subject\" \"URGENT\" { fileinto \"INBOX.Urgent\"; }",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("Normal message"), "", "");
        assert_eq!(actions, vec![SieveAction::Keep]);
    }

    #[test]
    fn eval_reject() {
        let script = compile(b"require [\"reject\"]; reject \"Not wanted\";").unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::Reject("Not wanted".into())]);
    }

    #[test]
    fn eval_header_is_case_insensitive() {
        let script = compile(b"if header :is \"subject\" \"exact match\" { discard; }").unwrap();
        let actions = evaluate(&script, &make_msg("exact match"), "", "");
        assert_eq!(actions, vec![SieveAction::Discard]);
    }

    #[test]
    fn eval_size_over_true() {
        let script =
            compile(b"require [\"fileinto\"]; if size :over 10 { fileinto \"Big\"; }").unwrap();
        let msg = make_msg("test"); // should be > 10 bytes
        let actions = evaluate(&script, &msg, "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("Big".into())]);
    }

    #[test]
    fn eval_exists_header_present() {
        let script =
            compile(b"require [\"fileinto\"]; if exists \"X-Spam-Flag\" { fileinto \"Spam\"; }")
                .unwrap();
        let msg = b"X-Spam-Flag: YES\r\nSubject: test\r\n\r\nBody\r\n";
        let actions = evaluate(&script, msg, "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("Spam".into())]);
    }

    #[test]
    fn eval_unknown_extension_compile_error() {
        let result = compile(b"require [\"erewhon\"];");
        assert!(result.is_err());
    }

    #[test]
    fn eval_unknown_comparator_compile_error() {
        let result =
            compile(b"if header :is :comparator \"i;invalid\" \"Subject\" \"test\" { keep; }");
        assert!(result.is_err(), "unknown comparator must fail at compile");
        assert!(
            result.unwrap_err().message.contains("comparator"),
            "error must mention comparator"
        );
    }

    #[test]
    fn compile_invalid_regex_pattern_fails() {
        let result =
            compile(b"require [\"regex\"]; if header :regex \"Subject\" \"[invalid\" { keep; }");
        assert!(result.is_err(), "invalid regex must fail at compile");
    }

    #[test]
    fn compile_regex_does_not_validate_header_name_as_pattern() {
        // "X[Special]" is a valid header name but not a valid regex character class.
        // Only the match keys (last Str/StringList) are validated as regex; the
        // header field name must not be.
        let result =
            compile(b"require [\"regex\"]; if header :regex \"X[Special]\" \"test.*\" { keep; }");
        assert!(
            result.is_ok(),
            "header name should not be validated as regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn compile_regex_validates_pattern_in_string_list() {
        // When the key-list is a StringList, each pattern in it must be validated.
        let result = compile(
            b"require [\"regex\"]; if header :regex \"Subject\" [\"ok.*\", \"[invalid\"] { keep; }",
        );
        assert!(
            result.is_err(),
            "invalid regex in key StringList must fail at compile"
        );
    }

    #[test]
    fn compile_regex_invalid_in_if_clause_with_elsif() {
        // Regression for ruj.31: the invalid pattern is in the if clause; the
        // elsif clause must not shadow it by providing a valid Str later in the
        // combined flat stmt.
        let result = compile(
            b"require [\"regex\"]; if header :regex \"Subject\" \"[invalid\" { keep; } elsif header :is \"To\" \"key\" { keep; }",
        );
        assert!(
            result.is_err(),
            "invalid regex in if clause must fail even when elsif is present"
        );
    }

    // -----------------------------------------------------------------------
    // RFC 5229 variables extension tests
    // -----------------------------------------------------------------------

    #[test]
    fn eval_variables_set_and_fileinto() {
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set \"folder\" \"INBOX.Work\"; fileinto \"${folder}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("INBOX.Work".into())]);
    }

    #[test]
    fn eval_variables_modifier_lower() {
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set :lower \"folder\" \"INBOX.WORK\"; fileinto \"${folder}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("inbox.work".into())]);
    }

    #[test]
    fn eval_variables_modifier_upper() {
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set :upper \"folder\" \"inbox.work\"; fileinto \"${folder}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("INBOX.WORK".into())]);
    }

    #[test]
    fn eval_variables_modifier_length() {
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set :length \"len\" \"hello\"; fileinto \"${len}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("5".into())]);
    }

    #[test]
    fn eval_variables_modifier_firstline() {
        // The \n here is a real newline byte in the Sieve quoted string.
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set :firstline \"f\" \"line1\nline2\"; fileinto \"${f}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("line1".into())]);
    }

    #[test]
    fn eval_variables_case_insensitive_name() {
        let script = compile(
            b"require [\"variables\", \"fileinto\"]; set \"MyVar\" \"hello\"; fileinto \"${myvar}\";",
        )
        .unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::FileInto("hello".into())]);
    }

    #[test]
    fn eval_no_variables_require_no_substitution() {
        // Without require ["variables"], ${reason} is literal text (RFC 5229 §3).
        let script = compile(b"require [\"reject\"]; reject \"${reason}\";").unwrap();
        let actions = evaluate(&script, &make_msg("test"), "", "");
        assert_eq!(actions, vec![SieveAction::Reject("${reason}".into())]);
    }
}
