# sieve-form — Project Plan

## What This Is

A standalone, MIT-licensed Rust crate implementing the Sieve email filter language
(RFC 5228 base + RFC 5229 variables extension) on a uniform form representation.

**Source to extract from:** `~/PROJECT/usenet-ipfs/crates/sieve-native/`

The crate was originally named `stoa-sieve-native` inside a private monorepo.
The goal is to extract it, clean it up, and publish it to crates.io as `sieve-form`.

## Core Architectural Insight

Sieve's grammar is structurally isomorphic to Lisp forms. Every statement is a
flat sequence of typed atoms:

```
if header :contains ["From"] "boss@example.com" { fileinto "Important"; }
→ [Word("if"), Word("header"), Tag("contains"), StringList(["From"]),
   Str("boss@example.com"), Block([...])]
```

This means the **form/reader layer is complete and stable** — it can represent any
valid Sieve syntax for any extension without modification. New extensions require
only new match arms in the evaluator, not parser changes.

The three-layer pipeline:
1. `lexer` — raw bytes → `Vec<Token>`
2. `form` — tokens → `Script` = `Vec<Stmt>` = `Vec<Vec<Form>>`
3. `evaluator` — `Script` + message → `Vec<SieveAction>`

Both `lexer` and `form` should be public modules (the reader is the contribution).
The evaluator is the implementation layer.

## Work To Do

### Step 1: Copy source files

Copy these files from `~/PROJECT/usenet-ipfs/crates/sieve-native/src/` into `src/`:

```
lib.rs
lexer.rs
form.rs
evaluator.rs
message.rs
parse_error.rs
```

Do NOT copy `tests/cross_validate.rs` — see Step 3.

### Step 2: Set up Cargo.toml

Create a new `Cargo.toml`:

```toml
[package]
name = "sieve-form"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "MIT"
description = "Sieve email filter language (RFC 5228 + RFC 5229) on a uniform form representation"
repository = "https://github.com/MarkAtwood/crate-sieve-form"
keywords = ["sieve", "email", "filter", "rfc5228", "imap"]
categories = ["email", "parser-implementations"]

[dependencies]
fancy-regex = "0.13"
```

No `stoa-sieve` dependency. No path deps.

### Step 3: Fix the test suite

The original `tests/cross_validate.rs` used `stoa-sieve` (which depends on
`sieve-rs`, AGPL-licensed) as a test oracle. That path dep does not exist outside
the monorepo and the AGPL dep is unsuitable for a MIT crate.

**Replace with static test vectors.** Use RFC 5228 example scripts and known-good
outputs as the oracle. Do not derive test vectors from the code under test.

Good sources for independent test vectors:
- RFC 5228 Appendix A (example scripts)
- RFC 5229 §5 (variable extension examples)
- Dovecot's Pigeonhole test suite (publicly available, BSD-licensed examples)
- Hand-traced scripts with hardcoded expected `SieveAction` outputs

The 32 unit tests already in `src/lib.rs` that do not depend on the oracle can
stay as-is. Review each one to confirm it has an independent expected value.

### Step 4: Introduce a typed error type

The current `compile()` returns `Result<CompiledScript, String>`. Not idiomatic
for a published library. Replace with:

```rust
#[derive(Debug, Clone)]
pub struct SieveError {
    pub message: String,
    pub kind: SieveErrorKind,
}

#[derive(Debug, Clone)]
pub enum SieveErrorKind {
    Utf8,
    Lex,
    Parse,
    UnsupportedExtension(String),
    InvalidRegex(String),
}

impl std::fmt::Display for SieveError { ... }
impl std::error::Error for SieveError {}
```

Update `compile()` signature to `pub fn compile(script: &[u8]) -> Result<CompiledScript, SieveError>`.

### Step 5: Add a SieveRuntime trait (preparatory)

Add this trait to enable callers to handle side-effectful extensions later
(vacation, imap4flags, etc.) without the crate needing to implement them fully:

```rust
/// Implemented by the embedding application to handle Sieve side effects.
pub trait SieveRuntime {
    fn file_into(&mut self, folder: &str);
    fn reject(&mut self, reason: &str);
    fn discard(&mut self);
    /// Called for commands the evaluator does not recognise.
    /// Return true if handled, false to silently ignore.
    fn unknown_command(&mut self, name: &str, args: &[crate::form::Form]) -> bool {
        let _ = (name, args);
        false
    }
}
```

Also provide a `DefaultRuntime` that collects actions into a `Vec<SieveAction>`
so the existing public `evaluate()` API is unchanged.

This is preparatory scaffolding — do not refactor the entire evaluator internals
to be generic over `SieveRuntime` in this first pass. Add the trait, add
`DefaultRuntime`, wire the public `evaluate()` through it, leave the evaluator
internals for a follow-up.

### Step 6: Write the README

Must include:

- One-paragraph description
- Why this crate exists (MIT license, no AGPL dep, minimal deps, form-layer architecture)
- Quick usage example (compile + evaluate a script)
- RFC coverage table:

| Extension | RFC | Status |
|-----------|-----|--------|
| Base language | RFC 5228 | Supported |
| Variables | RFC 5229 | Supported |
| Regex (draft) | draft-ietf-sieve-regex | Supported |
| Vacation | RFC 5230 | Not yet |
| Relational | RFC 5231 | Not yet |
| IMAP flags | RFC 5232 | Not yet |
| Subaddress | RFC 5233 | Not yet |
| Body | RFC 5173 | Not yet |
| Date/Index | RFC 5260 | Not yet |
| Editheader | RFC 5293 | Not yet |
| Enotify | RFC 5435 | Not yet |

- Architecture section explaining the three-layer pipeline and the Lisp-form insight
- MSRV (1.80)
- License

### Step 7: Pre-publish checklist

```bash
cargo fmt --all
cargo clippy --all-features -- -D warnings
cargo test
cargo doc --no-deps --all-features
typos src/ README.md          # cargo install typos-cli
cargo deny check              # cargo install cargo-deny
cargo publish --dry-run
```

Verify `sieve-form` is not taken on crates.io before publishing.

### Step 8: Git and publishing

```bash
git init
git add .
git commit -m "feat: initial extraction from stoa-sieve-native"
```

Create a GitHub repo at `github.com/MarkAtwood/crate-sieve-form`, push, then:
`cargo publish`

## What NOT To Do

- Do not copy `tests/cross_validate.rs` — it has an AGPL-linked oracle
- Do not add `sieve-rs` as a dependency (AGPL-3.0-only)
- Do not add `mail-parser` — it was in `stoa-sieve`, not `stoa-sieve-native`
- Do not add extensions beyond the original scope unless explicitly planned
- Do not mock or weaken existing tests to make them pass — fix the code
- Do not leave any `stoa-` prefixed names in public API

## Known Gap to Investigate

`envelope` is part of RFC 5228 base but gated behind `require ["envelope"]`.
Check whether the evaluator handles it or rejects it as an unknown extension.
If rejected, it should be implemented — it is base spec, not an extension.
