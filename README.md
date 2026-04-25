# sieve-form

A standalone, MIT-licensed Rust implementation of the Sieve email filter
language ([RFC 5228](https://www.rfc-editor.org/rfc/rfc5228) base language +
[RFC 5229](https://www.rfc-editor.org/rfc/rfc5229) variables extension) built
on a uniform form representation. The crate compiles a Sieve script to an
internal form tree and evaluates it against a raw RFC 5322 message, returning
a list of disposition actions.

## Why this crate exists

- **MIT license** — no AGPL dependency; safe to embed in commercial products.
- **No `sieve-rs` dependency** — avoids the AGPL-licensed `sieve-rs` crate entirely.
- **Minimal dependencies** — only `fancy-regex` (for the regex extension).
- **Stable form layer** — the three-layer pipeline (lexer → form → evaluator)
  means new Sieve extensions require only new match arms in the evaluator; the
  parser never changes.

## Usage

```rust
use sieve_form::{compile, evaluate, SieveAction};

let script = compile(
    b"require [\"fileinto\"]; \
      if header :contains \"X-Spam\" \"yes\" { fileinto \"Spam\"; }",
)
.unwrap();

let msg = b"From: spammer@example.com\r\nX-Spam: yes\r\n\r\nBody\r\n";
let actions = evaluate(&script, msg, "spammer@example.com", "user@example.com");
assert_eq!(actions, vec![SieveAction::FileInto("Spam".to_string())]);
```

## RFC coverage

| Extension     | RFC                    | Status    |
|---------------|------------------------|-----------|
| Base language | RFC 5228               | Supported |
| Variables     | RFC 5229               | Supported |
| Regex (draft) | draft-ietf-sieve-regex | Supported |
| Vacation      | RFC 5230               | Not yet   |
| Relational    | RFC 5231               | Not yet   |
| IMAP flags    | RFC 5232               | Not yet   |
| Subaddress    | RFC 5233               | Not yet   |
| Body          | RFC 5173               | Not yet   |
| Date/Index    | RFC 5260               | Not yet   |
| Editheader    | RFC 5293               | Not yet   |
| Enotify       | RFC 5435               | Not yet   |

## Architecture

The crate is structured as three distinct layers:

```
raw bytes  →  lexer::tokenize  →  Vec<Token>
Vec<Token> →  form::read_script →  Script (Vec<Vec<Form>>)
Script     →  evaluator        →  Vec<SieveAction>
```

**Layer 1 — lexer** (`sieve_form::lexer`): converts raw UTF-8 source bytes into
a flat `Vec<Token>`. Handles quoted strings, multiline strings (`text:…\n.\n`),
tagged arguments (`:name`), numeric literals with size multipliers (`K`, `M`,
`G`), line comments (`#`), and block comments (`/* */`).

**Layer 2 — form** (`sieve_form::form`): converts the token stream into a
`Script`, which is a `Vec<Stmt>` where each `Stmt` is a `Vec<Form>`. The `Form`
enum is:

```rust
pub enum Form {
    Word(String),           // identifier keyword
    Tag(String),            // :tagged argument (colon stripped)
    Str(String),            // string literal
    Num(u64),               // numeric literal
    StringList(Vec<String>),// ["a", "b"]
    TestList(Vec<Stmt>),    // (test1, test2)
    Block(Vec<Stmt>),       // { stmt; stmt; }
}
```

The key insight is that the Sieve grammar is structurally isomorphic to Lisp
forms: every command, test, and argument maps cleanly into a flat list of typed
atoms, with nesting expressed only through `Block` and `TestList`. For example:

```
if header :contains ["From"] "boss@example.com" { fileinto "Important"; }
```

becomes one `Stmt`:

```
[Word("if"), Word("header"), Tag("contains"),
 StringList(["From"]), Str("boss@example.com"),
 Block([[Word("fileinto"), Str("Important")]])]
```

**Layer 3 — evaluator** (internal): walks the form tree, maintains variable
state (RFC 5229), and returns a `Vec<SieveAction>`. Defaults to
`[SieveAction::Keep]` when the script produces no explicit disposition (RFC
5228 §2.10.2).

Because the form representation is uniform and stable, adding support for a new
Sieve extension requires only new match arms in the evaluator — the lexer and
form layers do not change.

## MSRV

Rust 1.80

## License

MIT
