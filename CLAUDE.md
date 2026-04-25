# sieve-form

MIT-licensed Rust implementation of the Sieve email filter language (RFC 5228 +
RFC 5229) built on a uniform form representation.

Read PROJECT.md before starting any work. It contains the full extraction plan,
rationale, and explicit list of what not to do.

## Architecture

Three layers — do not collapse them:

1. **`lexer`** — raw bytes → `Vec<Token>`. Public module.
2. **`form`** — tokens → `Script` (`Vec<Vec<Form>>`). Public module. This is the
   core contribution: a uniform, recursive representation equivalent to Lisp forms.
3. **`evaluator`** — `Script` + message → `Vec<SieveAction>`. Internal module,
   exposed only through the public `compile`/`evaluate` API.

The form layer is stable. New Sieve extensions require only new match arms in the
evaluator — the parser never changes.

## Build & Test

```bash
cargo test
cargo clippy --all-features -- -D warnings
cargo fmt --check
cargo doc --no-deps --all-features
```

## Task Tracking

Uses Beads. Run `bd prime` for full workflow context.

```bash
bd ready           # Find available work
bd show <id>       # View issue details
bd update <id> --claim
bd close <id>
```

## Coding Rules

- Public API: `compile(bytes) -> Result<CompiledScript, SieveError>` and
  `evaluate(script, msg, from, to) -> Vec<SieveAction>`
- Error type must be a proper struct/enum, not `String`
- `lexer` and `form` modules must be `pub`
- No `stoa-` prefixed names anywhere in public API
- No `sieve-rs` dependency (AGPL)
- MSRV: 1.80

## Test Integrity

Never weaken, skip, or mock tests to make them pass. Fix the code.

Test vectors must have an independent oracle — not derived from the code under
test. Acceptable oracles: RFC appendix examples, Dovecot Pigeonhole test scripts,
hand-traced expected outputs.

## Commit Messages

- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`
- Subject line under 50 characters
- No "Generated with" footers, no Claude attribution
- Ask before committing or pushing


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
