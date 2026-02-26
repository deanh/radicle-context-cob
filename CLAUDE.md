# CLAUDE.md

## Build & Test

```
cargo build          # compile
cargo test           # run all tests (lib + main)
cargo clippy         # lint (lib warnings are pre-existing)
```

## Project Structure

- `src/lib.rs` — COB type definition, actions, state, store (library crate)
- `src/main.rs` — CLI binary (`rad-context`), helper functions, inline tests
- No separate `tests/` directory — all tests are `#[cfg(test)] mod tests` inline

## Smoke Testing on the Command Line

COBs are append-only — there is no delete command. Every `rad-context create` leaves a permanent context in the repo's COB store. Keep this in mind when testing manually.

Pattern for smoke tests:
- Use obviously-named titles like `"smoke-test: <what you're testing>"` so they're easy to spot in `rad-context list`
- Verify with `rad-context show <short-id>` after creation
- Accept that test contexts accumulate; they're harmless but can't be removed without low-level git ref surgery

## Key Conventions

- `repo.backend` is a `git2::Repository` — access git2 types via `radicle::git::raw::*` (not `git2::` directly)
- Short IDs: minimum 7 hex chars for prefix resolution
- Errors use `Box<dyn std::error::Error>` in the CLI; typed `Error` enum in the library
