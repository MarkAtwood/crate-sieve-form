// SPDX-License-Identifier: MIT

//! Sieve script evaluator (RFC 5228 + RFC 5229 variables extension).

use crate::form::{Form, Script, Stmt};
use crate::message;
use crate::message::AddressPart;
use crate::SieveAction;
use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::HashMap;

/// Extensions that this evaluator implements.
///
/// Consulted by `compile()` to enforce the RFC 5228 §2.6 rule that unknown
/// extensions in `require` must cause a compile-time failure.  Adding a new
/// extension to the evaluator requires adding its name here — the compile
/// step will then accept scripts that declare it.
pub(crate) const KNOWN_EXTENSIONS: &[&str] =
    &["fileinto", "reject", "envelope", "variables", "regex"];

/// Build the cache key for a case-sensitive anchored regex pattern.
pub(crate) fn regex_base_key(pattern: &str) -> String {
    format!("(?s)\\A(?:{pattern})\\z")
}

/// Build the cache key for a case-insensitive anchored regex pattern.
pub(crate) fn regex_ci_key(base_key: &str) -> String {
    format!("(?i){base_key}")
}

/// Build the cache key for a case-sensitive glob pattern.
pub(crate) fn glob_base_key(pattern: &str) -> String {
    format!("glob:{pattern}")
}

/// Build the cache key for a case-insensitive glob pattern.
pub(crate) fn glob_ci_key(base_key: &str) -> String {
    format!("(?i){base_key}")
}

// ---------------------------------------------------------------------------
// Evaluation context
// ---------------------------------------------------------------------------

/// Per-evaluation context threaded through all evaluator functions.
///
/// Created fresh on each [`eval_script`] call and dropped at the end.
struct Ctx<'a> {
    /// Parsed header list in document order.  A `Vec` (not a `HashMap`)
    /// because RFC 5228 permits multiple headers with the same name and
    /// because the test evaluators need to iterate all matching values.
    headers: Vec<(String, String)>,
    message_size: usize,
    envelope_from: &'a str,
    envelope_to: &'a str,
    /// Variable bindings (lowercase names) set by the `set` command.
    /// Only consulted when `variables_enabled` is true.
    variables: HashMap<String, String>,
    /// Whether `require ["variables"]` was declared (RFC 5229).
    /// `${name}` substitution is only active when this is true.
    variables_enabled: bool,
    /// Pre-compiled regex cache from [`crate::compile`].  Read-only
    /// during evaluation; all patterns are compiled once at compile time.
    regex_cache: &'a HashMap<String, fancy_regex::Regex>,
}

// ---------------------------------------------------------------------------
// Internal result type
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum StmtResult {
    Continue,
    Keep,
    Discard,
    FileInto(String),
    Reject(String),
    Redirect(String),
    Stop,
}

#[derive(Debug)]
enum SizeDir {
    Over,
    Under,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Evaluate a compiled Sieve [`Script`] against a raw RFC 5322 message.
///
/// Returns the list of actions the script requests.  If the script produces
/// no explicit disposition, `[Keep]` is appended per RFC 5228 §2.10.2.
pub fn eval_script(
    script: &Script,
    regex_cache: &HashMap<String, fancy_regex::Regex>,
    variables_enabled: bool,
    raw_message: &[u8],
    envelope_from: &str,
    envelope_to: &str,
) -> Vec<SieveAction> {
    let headers = message::extract_headers(raw_message);

    let mut ctx = Ctx {
        headers,
        message_size: raw_message.len(),
        envelope_from,
        envelope_to,
        variables: HashMap::new(),
        variables_enabled,
        regex_cache,
    };

    let action = match eval_stmt_list(script, &mut ctx) {
        Some(StmtResult::Discard) => SieveAction::Discard,
        Some(StmtResult::FileInto(folder)) => SieveAction::FileInto(folder),
        Some(StmtResult::Reject(reason)) => SieveAction::Reject(reason),
        Some(StmtResult::Redirect(addr)) => SieveAction::Redirect(addr),
        _ => SieveAction::Keep,
    };
    vec![action]
}

// ---------------------------------------------------------------------------
// Statement list / statement dispatch
// ---------------------------------------------------------------------------

fn eval_stmt_list(stmts: &[Stmt], ctx: &mut Ctx<'_>) -> Option<StmtResult> {
    for stmt in stmts {
        match eval_stmt(stmt, ctx) {
            StmtResult::Continue => {}
            other => return Some(other),
        }
    }
    None
}

fn eval_stmt(stmt: &Stmt, ctx: &mut Ctx<'_>) -> StmtResult {
    match stmt.as_slice() {
        // require — validated at compile time; ignore at eval time.
        [Form::Word(w), ..] if w == "require" => StmtResult::Continue,

        // if / elsif / else chain.
        [Form::Word(w), rest @ ..] if w == "if" => eval_if(rest, ctx),

        // fileinto "folder"
        [Form::Word(w), Form::Str(folder)] if w == "fileinto" => {
            StmtResult::FileInto(expand_vars(folder, ctx).into_owned())
        }

        // reject "reason"
        [Form::Word(w), Form::Str(reason)] if w == "reject" => {
            StmtResult::Reject(expand_vars(reason, ctx).into_owned())
        }

        // redirect "address"  (RFC 5228 §4.4 — base action, no extension required)
        [Form::Word(w), Form::Str(addr)] if w == "redirect" => {
            StmtResult::Redirect(expand_vars(addr, ctx).into_owned())
        }

        // discard
        [Form::Word(w)] if w == "discard" => StmtResult::Discard,

        // keep
        [Form::Word(w)] if w == "keep" => StmtResult::Keep,

        // stop
        [Form::Word(w)] if w == "stop" => StmtResult::Stop,

        // set [modifiers...] "name" "value"  (RFC 5229 §4)
        [Form::Word(w), rest @ ..] if w == "set" => {
            // Collect leading Tag modifiers, then expect Str(name) Str(value).
            let n_tags = rest
                .iter()
                .position(|f| !matches!(f, Form::Tag(_)))
                .unwrap_or(rest.len());
            let modifier_names: Vec<&str> = rest[..n_tags]
                .iter()
                .map(|f| {
                    let Form::Tag(t) = f else { unreachable!() };
                    t.as_str()
                })
                .collect();
            let operands = &rest[n_tags..];
            if let (Some(Form::Str(name)), Some(Form::Str(value))) =
                (operands.first(), operands.get(1))
            {
                let expanded = expand_vars(value, ctx).into_owned();
                let modified = apply_set_modifiers(expanded, &modifier_names);
                ctx.variables.insert(name.to_lowercase(), modified);
            }
            StmtResult::Continue
        }

        // RFC 5228 §2.9: implementations MUST silently ignore unknown commands.
        _ => StmtResult::Continue,
    }
}

// ---------------------------------------------------------------------------
// if / elsif / else
// ---------------------------------------------------------------------------

/// Evaluate an if/elsif/else chain iteratively.
///
/// `rest` is the slice of forms *after* the leading `Word("if")`:
/// `[test_form0, test_form1, ..., Block(then_stmts), optional elsif/else ...]`
///
/// Uses a loop instead of mutual recursion to avoid stack overflow on
/// adversarial scripts with many `elsif` branches.
fn eval_if(mut rest: &[Form], ctx: &mut Ctx<'_>) -> StmtResult {
    loop {
        let block_pos = match rest.iter().position(|f| matches!(f, Form::Block(_))) {
            Some(p) => p,
            None => return StmtResult::Continue, // malformed
        };

        let test_forms = &rest[..block_pos];
        let block = match &rest[block_pos] {
            Form::Block(stmts) => stmts,
            _ => return StmtResult::Continue,
        };
        let after_block = &rest[block_pos + 1..];

        if eval_test(test_forms, ctx) {
            return match eval_stmt_list(block, ctx) {
                None | Some(StmtResult::Continue) => StmtResult::Continue,
                Some(other) => other,
            };
        }

        // Test failed — advance to the next elsif/else or stop.
        match after_block {
            [] => return StmtResult::Continue,
            [Form::Word(w), tail @ ..] if w == "elsif" => rest = tail,
            [Form::Word(w), Form::Block(stmts), ..] if w == "else" => {
                return match eval_stmt_list(stmts, ctx) {
                    None | Some(StmtResult::Continue) => StmtResult::Continue,
                    Some(other) => other,
                };
            }
            _ => return StmtResult::Continue,
        }
    }
}

// ---------------------------------------------------------------------------
// Test evaluation
// ---------------------------------------------------------------------------

fn eval_test(forms: &[Form], ctx: &mut Ctx<'_>) -> bool {
    match forms {
        [Form::Word(w), rest @ ..] => match w.as_str() {
            "header" => eval_header_test(rest, ctx),
            "address" => eval_address_test(rest, ctx),
            "envelope" => eval_envelope_test(rest, ctx),
            "exists" => eval_exists_test(rest, ctx),
            "size" => eval_size_test(rest, ctx.message_size),
            "allof" => eval_allof(rest, ctx),
            "anyof" => eval_anyof(rest, ctx),
            "not" => !eval_test(rest, ctx),
            "true" => true,
            "false" => false,
            _ => false, // unknown test — fail-safe
        },
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// allof / anyof
// ---------------------------------------------------------------------------

fn eval_allof(rest: &[Form], ctx: &mut Ctx<'_>) -> bool {
    match rest {
        [Form::TestList(tests)] => tests.iter().all(|t| eval_test(t, ctx)),
        _ => false,
    }
}

fn eval_anyof(rest: &[Form], ctx: &mut Ctx<'_>) -> bool {
    match rest {
        [Form::TestList(tests)] => tests.iter().any(|t| eval_test(t, ctx)),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Match type / comparator / address-part extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchType {
    Is,
    Contains,
    Matches,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Comparator {
    AsciiCasemap,
    Octet,
}

/// Arguments extracted from a test's form list in a single pass.
struct TestArgs<'a> {
    mt: MatchType,
    cmp: Comparator,
    part: AddressPart,
    names: Vec<&'a str>,
    keys: Vec<&'a str>,
}

/// Extract match type, comparator, address part, and the two string-list
/// operands (names and keys) from a test's form slice in a single pass,
/// with no intermediate Vec allocations.
fn extract_test_args<'a>(forms: &'a [Form]) -> TestArgs<'a> {
    let mut mt = MatchType::Is;
    let mut cmp = Comparator::AsciiCasemap;
    let mut part = AddressPart::All;
    let mut names: Vec<&'a str> = Vec::new();
    let mut keys: Vec<&'a str> = Vec::new();
    let mut string_count = 0usize;
    let mut i = 0;
    while i < forms.len() {
        match &forms[i] {
            Form::Tag(t) => match t.as_str() {
                "is" => mt = MatchType::Is,
                "contains" => mt = MatchType::Contains,
                "matches" => mt = MatchType::Matches,
                "regex" => mt = MatchType::Regex,
                "comparator" => {
                    if let Some(Form::Str(s)) = forms.get(i + 1) {
                        if s == "i;octet" {
                            cmp = Comparator::Octet;
                        }
                        i += 1;
                    }
                }
                "localpart" => part = AddressPart::LocalPart,
                "domain" => part = AddressPart::Domain,
                "all" => part = AddressPart::All,
                _ => {}
            },
            Form::Str(s) if string_count < 2 => {
                if string_count == 0 {
                    names.push(s.as_str());
                } else {
                    keys.push(s.as_str());
                }
                string_count += 1;
            }
            Form::StringList(v) if string_count < 2 => {
                let target = if string_count == 0 {
                    &mut names
                } else {
                    &mut keys
                };
                target.extend(v.iter().map(String::as_str));
                string_count += 1;
            }
            _ => {}
        }
        i += 1;
    }
    TestArgs {
        mt,
        cmp,
        part,
        names,
        keys,
    }
}

// ---------------------------------------------------------------------------
// String matching helpers
// ---------------------------------------------------------------------------

fn str_is(a: &str, b: &str, comparator: Comparator) -> bool {
    if comparator == Comparator::AsciiCasemap {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn str_contains(haystack: &str, needle: &str, comparator: Comparator) -> bool {
    if comparator != Comparator::AsciiCasemap {
        return haystack.contains(needle);
    }
    if needle.is_empty() {
        return true;
    }
    // i;ascii-casemap: byte-level case-insensitive search.
    // Safe for ASCII-only case mapping per RFC 4790.
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.len() > hb.len() {
        return false;
    }
    hb.windows(nb.len()).any(|w| {
        w.iter()
            .zip(nb.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

/// Sieve glob matching (RFC 5228 §2.7.1).
/// `*` = zero or more chars, `?` = exactly one char, `\*`/`\?` = literals.
fn str_matches_glob(
    value: &str,
    pattern: &str,
    comparator: Comparator,
    regex_cache: &HashMap<String, fancy_regex::Regex>,
) -> bool {
    // Fast path: glob patterns are pre-compiled into the cache at compile()
    // time under the key "glob:{pattern}" (or "(?i)glob:{pattern}" for
    // case-insensitive).  Avoid calling sieve_glob_to_regex at eval time.
    let base_key = glob_base_key(pattern);
    if comparator == Comparator::AsciiCasemap {
        let ci_key = glob_ci_key(&base_key);
        if let Some(re) = regex_cache.get(&ci_key) {
            return re.is_match(value).unwrap_or(false);
        }
    } else if let Some(re) = regex_cache.get(&base_key) {
        return re.is_match(value).unwrap_or(false);
    }
    // Defensive fallback: compile on the fly (should not occur for validated
    // scripts whose patterns were pre-cached at compile time).
    let regex_pat = sieve_glob_to_regex(pattern);
    str_matches_regex_pat(value, &regex_pat, comparator, regex_cache)
}

/// Convert a Sieve glob pattern to an anchored regex string.
pub(crate) fn sieve_glob_to_regex(pattern: &str) -> String {
    let mut out = String::from("(?s)\\A");
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    chars.next();
                    match next {
                        '*' | '?' => {
                            let mut buf = [0u8; 4];
                            let s = next.encode_utf8(&mut buf);
                            out.push_str(&fancy_regex::escape(s));
                        }
                        other => {
                            let mut buf = [0u8; 4];
                            let s = ch.encode_utf8(&mut buf);
                            out.push_str(&fancy_regex::escape(s));
                            let mut buf2 = [0u8; 4];
                            let s2 = other.encode_utf8(&mut buf2);
                            out.push_str(&fancy_regex::escape(s2));
                        }
                    }
                } else {
                    out.push_str(&fancy_regex::escape("\\"));
                }
            }
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            other => {
                let mut buf = [0u8; 4];
                let s = other.encode_utf8(&mut buf);
                out.push_str(&fancy_regex::escape(s));
            }
        }
    }
    out.push_str("\\z");
    out
}

/// Match `value` against a regex extension pattern (anchored to whole value).
fn str_matches_regex(
    value: &str,
    pattern: &str,
    comparator: Comparator,
    regex_cache: &HashMap<String, fancy_regex::Regex>,
) -> bool {
    let anchored = regex_base_key(pattern);
    str_matches_regex_pat(value, &anchored, comparator, regex_cache)
}

fn str_matches_regex_pat(
    value: &str,
    anchored: &str,
    comparator: Comparator,
    regex_cache: &HashMap<String, fancy_regex::Regex>,
) -> bool {
    // SECURITY: fancy-regex uses backtracking for extended patterns.
    // Untrusted :regex patterns can cause catastrophic backtracking (ReDoS).
    // Callers should validate pattern complexity or restrict who can supply
    // :regex tests.
    if comparator == Comparator::AsciiCasemap {
        let pat = regex_ci_key(anchored);
        if let Some(re) = regex_cache.get(&pat) {
            return re.is_match(value).unwrap_or(false);
        }
        // Fallback: compile on the fly (should not occur for validated scripts).
        match fancy_regex::Regex::new(&pat) {
            Ok(re) => re.is_match(value).unwrap_or(false),
            Err(_) => false,
        }
    } else {
        if let Some(re) = regex_cache.get(anchored) {
            return re.is_match(value).unwrap_or(false);
        }
        // Fallback: compile on the fly (should not occur for validated scripts).
        match fancy_regex::Regex::new(anchored) {
            Ok(re) => re.is_match(value).unwrap_or(false),
            Err(_) => false,
        }
    }
}

fn apply_match(
    value: &str,
    key: &str,
    mt: MatchType,
    comparator: Comparator,
    regex_cache: &HashMap<String, fancy_regex::Regex>,
) -> bool {
    match mt {
        MatchType::Is => str_is(value, key, comparator),
        MatchType::Contains => str_contains(value, key, comparator),
        MatchType::Matches => str_matches_glob(value, key, comparator, regex_cache),
        MatchType::Regex => str_matches_regex(value, key, comparator, regex_cache),
    }
}

// ---------------------------------------------------------------------------
// Individual test implementations
// ---------------------------------------------------------------------------

fn eval_header_test(forms: &[Form], ctx: &Ctx<'_>) -> bool {
    let args = extract_test_args(forms);
    // RFC 5229 §3: variable substitution applies to all string args in tests.
    let field_names: Vec<Cow<'_, str>> = args.names.iter().map(|s| expand_vars(s, ctx)).collect();
    let keys: Vec<Cow<'_, str>> = args.keys.iter().map(|s| expand_vars(s, ctx)).collect();

    for (hdr_name, hdr_value) in &ctx.headers {
        for fname in &field_names {
            if hdr_name.eq_ignore_ascii_case(fname.as_ref()) {
                for key in &keys {
                    if apply_match(hdr_value, key.as_ref(), args.mt, args.cmp, ctx.regex_cache) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn eval_address_test(forms: &[Form], ctx: &Ctx<'_>) -> bool {
    let args = extract_test_args(forms);
    // RFC 5229 §3: variable substitution applies to all string args in tests.
    let field_names: Vec<Cow<'_, str>> = args.names.iter().map(|s| expand_vars(s, ctx)).collect();
    let keys: Vec<Cow<'_, str>> = args.keys.iter().map(|s| expand_vars(s, ctx)).collect();

    for (hdr_name, hdr_value) in &ctx.headers {
        for fname in &field_names {
            if hdr_name.eq_ignore_ascii_case(fname.as_ref()) {
                // RFC 5228 §5.1: multi-address headers must be split and each
                // address tested independently.
                for raw_addr in message::split_addresses(hdr_value) {
                    let addr = message::address_part(&raw_addr, args.part);
                    for key in &keys {
                        if apply_match(&addr, key.as_ref(), args.mt, args.cmp, ctx.regex_cache) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn eval_envelope_test(forms: &[Form], ctx: &Ctx<'_>) -> bool {
    let args = extract_test_args(forms);
    // RFC 5229 §3: variable substitution applies to all string args in tests.
    let part_names: Vec<Cow<'_, str>> = args.names.iter().map(|s| expand_vars(s, ctx)).collect();
    let keys: Vec<Cow<'_, str>> = args.keys.iter().map(|s| expand_vars(s, ctx)).collect();

    for pname in &part_names {
        let lower = pname.as_ref().to_ascii_lowercase();
        let raw_addr = match lower.as_str() {
            "from" => ctx.envelope_from,
            "to" => ctx.envelope_to,
            _ => continue,
        };
        let addr = message::address_part(raw_addr, args.part);
        for key in &keys {
            if apply_match(&addr, key.as_ref(), args.mt, args.cmp, ctx.regex_cache) {
                return true;
            }
        }
    }
    false
}

fn eval_exists_test(forms: &[Form], ctx: &Ctx<'_>) -> bool {
    // Iterate directly to avoid collecting intermediate Vecs.
    // RFC 5229 §3: variable substitution applies to all string args in tests.
    //
    // An empty argument list has no header names to check, so there is
    // nothing that "exists" — return false rather than vacuously true.
    let mut found_any_name = false;
    for f in forms {
        let raw_names: &[String] = match f {
            Form::Str(s) => std::slice::from_ref(s),
            Form::StringList(v) => v.as_slice(),
            _ => continue,
        };
        for s in raw_names {
            found_any_name = true;
            let name = expand_vars(s, ctx);
            if !ctx
                .headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case(name.as_ref()))
            {
                return false;
            }
        }
    }
    found_any_name
}

fn eval_size_test(forms: &[Form], message_size: usize) -> bool {
    let mut dir: Option<SizeDir> = None;
    let mut limit: Option<u64> = None;

    for f in forms {
        match f {
            Form::Tag(t) if t == "over" => dir = Some(SizeDir::Over),
            Form::Tag(t) if t == "under" => dir = Some(SizeDir::Under),
            Form::Num(n) => limit = Some(*n),
            _ => {}
        }
    }

    let limit = match limit {
        // On 32-bit targets, limits exceeding u32::MAX saturate to usize::MAX,
        // making :over always false and :under always true for such limits.
        // This is benign in practice (no real email exceeds 4 GB).
        Some(l) => usize::try_from(l).unwrap_or(usize::MAX),
        None => return false,
    };

    match dir {
        Some(SizeDir::Over) => message_size > limit,
        Some(SizeDir::Under) => message_size < limit,
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Variable substitution (RFC 5229)
// ---------------------------------------------------------------------------

/// Replace `${varname}` with values from `ctx.variables`.  `\$` → literal `$`.
///
/// Substitution is skipped entirely when `ctx.variables_enabled` is false
/// (i.e. `require ["variables"]` was not declared — RFC 5229 §3).
/// Returns `Cow::Borrowed(s)` when no substitution occurs (zero allocation).
fn expand_vars<'a>(s: &'a str, ctx: &Ctx<'_>) -> Cow<'a, str> {
    if !ctx.variables_enabled {
        return Cow::Borrowed(s);
    }
    // Fast path: if there are no trigger characters, return borrowed.
    if !s.contains('$') && !s.contains('\\') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut modified = false;
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if chars.peek() == Some(&'$') {
                chars.next();
                out.push('$');
                modified = true;
            } else {
                out.push('\\');
            }
            continue;
        }
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut name = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                name.push(inner);
            }
            if closed {
                // Variable names are case-insensitive (RFC 5229 §3).
                // RFC 5229 §2 restricts names to [A-Za-z][A-Za-z0-9_-]*,
                // so make_ascii_lowercase() is correct and avoids a heap allocation.
                name.make_ascii_lowercase();
                let val = ctx.variables.get(&name).map(String::as_str).unwrap_or("");
                out.push_str(val);
                modified = true;
            } else {
                // Unclosed brace — emit literally.
                out.push_str("${");
                out.push_str(&name);
            }
            continue;
        }
        out.push(ch);
    }
    if modified {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(s)
    }
}

// ---------------------------------------------------------------------------
// set modifier application (RFC 5229 §4)
// ---------------------------------------------------------------------------

/// Apply RFC 5229 §4 modifiers to a value in precedence order.
///
/// Modifiers may appear in any order in the script; they are always applied
/// in precedence order regardless:
///
/// | Precedence | Modifier        |
/// |------------|-----------------|
/// | 40         | `:lower`/`:upper` |
/// | 30         | `:length`       |
/// | 20         | `:quotewildcard` |
/// | 10         | `:firstline`    |
fn apply_set_modifiers(value: String, modifiers: &[&str]) -> String {
    // Sort modifiers by precedence (highest first = applied first).
    fn precedence(m: &str) -> u8 {
        match m {
            "lower" | "upper" => 40,
            "length" => 30,
            "quotewildcard" => 20,
            "firstline" => 10,
            _ => 0,
        }
    }

    let mut sorted: Vec<&str> = modifiers.to_vec();
    sorted.sort_unstable_by_key(|m| Reverse(precedence(m)));

    let mut v = value;
    for m in sorted {
        v = match m {
            "lower" => v.to_ascii_lowercase(),
            "upper" => v.to_ascii_uppercase(),
            "length" => v.chars().count().to_string(),
            "quotewildcard" => v
                .replace('\\', "\\\\")
                .replace('*', "\\*")
                .replace('?', "\\?"),
            "firstline" => {
                // Truncate at the first \n or \r\n.
                if let Some(pos) = v.find('\n') {
                    let end = if pos > 0 && v.as_bytes()[pos - 1] == b'\r' {
                        pos - 1
                    } else {
                        pos
                    };
                    v.truncate(end);
                    v
                } else {
                    v
                }
            }
            // Unknown modifiers are ignored (fail-safe).
            _ => v,
        };
    }
    v
}
