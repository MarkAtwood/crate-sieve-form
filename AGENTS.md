# Agent Instructions — sieve-form

## Read first

Before doing any work:
1. Read `PROJECT.md` — full extraction plan and rationale
2. Read `CLAUDE.md` — coding rules and architecture
3. Run `bd ready` — check for open issues

## Source material

The original crate lives at `~/PROJECT/usenet-ipfs/crates/sieve-native/`.
Read those files to understand the existing implementation before modifying anything.

Key files to read:
- `src/form.rs` — the uniform form representation (core insight)
- `src/evaluator.rs` — dispatch on form head word
- `src/lib.rs` — public API and unit tests
- `src/lexer.rs` — tokenizer
- `src/parse_error.rs` — current (weak) error type

Do NOT read or copy `tests/cross_validate.rs` — it has an AGPL-linked dependency.

## Subagent guidance

Spawn subagents for:
- Reading and summarising source files in parallel
- Writing independent test vectors (each RFC section can be a separate subagent)
- Drafting README sections in parallel

Do not spawn subagents for:
- Tasks that require shared mutable state (e.g., editing the same file)
- Tasks fewer than ~20 lines of work

## What the evaluator dispatch looks like

`eval_stmt` in `evaluator.rs` is a pattern match on `[Form::Word(w), rest @ ..]`.
Adding a new extension means adding a new match arm for the command word and/or
a new arm in `eval_test` for test words. The form layer never changes.

## Known issues going in

1. `compile()` returns `Result<CompiledScript, String>` — needs a typed error type
2. Cross-validation tests (`tests/cross_validate.rs`) have an AGPL oracle — replace
   with static RFC vectors
3. `envelope` extension may not be implemented despite being RFC 5228 base
4. `lexer` and `form` modules may not be `pub` — check and fix
5. No `SieveRuntime` trait yet — add it as scaffolding (see PROJECT.md Step 5)

## Restrictions

- Do not add `sieve-rs` as any kind of dependency
- Do not use `TodoWrite` or markdown task lists — use `bd create` for all tracking
- Do not commit or push without explicit user approval
- Do not add features not listed in PROJECT.md

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
