# radicle-context-cob Engineering Standards

Engineering practices for this crate, adapted from the Radicle Heartwood
conventions. Follow these when contributing code.

## Project structure

Single Cargo crate with a library and a binary:

- `src/lib.rs` — library root (COB type registration, store, error types)
- `src/state.rs` — `Context` state struct and accessor methods
- `src/actions.rs` — `Action` enum (all COB mutations)
- `src/main.rs` — `rad-context` CLI binary

Minimum Rust version: **1.85.0**.

## Build, lint, and test

```sh
cargo build
cargo clippy --tests                      # must pass cleanly
cargo fmt --check                         # must pass cleanly
cargo test                                # all tests must pass
cargo doc                                 # no warnings
```

Every individual commit must pass all of the above. This enables `git bisect`.

## Clippy lints

These are enforced in `[lints.clippy]` in `Cargo.toml`:

| Lint                        | Level |
|-----------------------------|-------|
| `type_complexity`           | allow |
| `enum_variant_names`        | allow |
| `indexing_slicing`          | deny  |
| `fallible_impl_from`        | deny  |
| `wildcard_enum_match_arm`   | deny  |
| `unneeded_field_pattern`    | deny  |
| `fn_params_excessive_bools` | deny  |
| `must_use_candidate`        | deny  |

Do not use direct indexing (`[i]`) — use `.get()`, iterators, or pattern
matching. All match arms over enums must be exhaustive (no `_ =>`).

## Error handling

- **Library code**: use `thiserror` with `#[derive(thiserror::Error, Debug)]`.
  Compose errors via `#[error(transparent)]` and `#[from]`. Add helper methods
  like `is_not_found()` where useful.
- **CLI code**: use `anyhow::Result` for top-level error context.
- **Never panic in library code**. Use `unwrap` only when:
  1. Static analysis proves it cannot fail.
  2. Failure indicates an unrecoverable bug.
  3. Inside `#[cfg(test)]` code.
- In non-test code, document every `unwrap`/`expect` call site with a
  `// SAFETY:` comment explaining why it is safe. Use `expect` only when an
  invariant was violated and include the expectation in the message.
- In tests, prefer `expect`/`expect_err` with a clear fixture or assertion
  message when that improves diagnostics. Bare `unwrap` is acceptable for
  straightforward test setup where the enclosing test already explains the
  invariant.

## Module and import organization

Modules are declared at the top of the file, before imports. Private modules
come first, then public modules, separated by a blank line:

```rust
mod git;
mod storage;

pub mod refs;

use std::collections::HashMap;      // 1. std
use std::process;

use serde_json::Value;               // 2. External crates

use crate::crypto::PublicKey;        // 3. Crate-local
use crate::storage::refs::Refs;
```

Re-export commonly used types at the crate root via `pub use`.

## Type system patterns

### Newtypes and type aliases

- Use **type aliases** for simple renamings: `pub type ContextId = ObjectId;`
- Use **newtype structs** when you need distinct trait impls or API constraints.

### Generics

Prefer trait bounds with `where` clauses for readability when there are
multiple bounds:

```rust
pub fn create<G>(
    &mut self,
    title: String,
    signer: &Device<G>,
) -> Result<(ObjectId, Context), Error>
where
    G: crypto::signature::Signer<crypto::Signature>,
{ ... }
```

Use single-letter type parameters (`G`, `S`, `T`, `R`) by convention.

## Trait design

- Use **associated types** (`type Repository`) when there is exactly one
  meaningful implementation per type (e.g., storage traits).
- Mark error types `#[non_exhaustive]` when they are part of a public API.

## Concurrency

- **No async/await** — synchronous code only.

## Serialization

- Derive `Serialize` and `Deserialize` from `serde`. Use `#[serde(rename_all = "camelCase")]` for JSON field names.
- For types with string representations, use
  `#[serde(into = "String", try_from = "String")]`.

## Logging

Use the `log` crate (not `tracing`). Always include a `target`:

```rust
log::trace!(target: "context", "Applying {} {action:?}", op.id);
```

Check the file you are working in for the appropriate target name. Most logs
should be at the `debug` level. Logging level is controlled by `RUST_LOG`.

## Testing

### Organization

- **Unit tests**: inline `#[cfg(test)] mod tests { ... }` within source files.
- Tests exist in `state.rs`, `actions.rs`, and `main.rs`.

### Test infrastructure

- Feature-gated test utilities: `#[cfg(any(test, feature = "test"))] pub mod test;`
  can be used to expose test helpers for downstream crates.

## Naming conventions

- **Types**: `PascalCase` — `Context`, `Action`, `ContextMut`
- **Functions**: `snake_case` — keep names concise; use doc comments for detail
- **Constants**: `UPPER_SNAKE_CASE` — `TYPENAME`, `MIN_PREFIX_LEN`
- **Type parameters**: single uppercase letter — `G`, `S`, `T`, `R`
- **Modules**: lowercase — `actions`, `state`

### Variable naming

- 1-letter names for tiny scopes: `if let Some(e) = result.err() { ... }`
- 1-word names for function parameters: `repo`, `sig`, `signer`
- Descriptive names for globals and constants

## Commit messages

Format: `<scope>: <Summary in imperative mood>`

- Scopes: `lib:`, `cli:`, `state:`, `actions:`, `docs:`, `ci:`, `chore:`.
- Subject line: capitalized, imperative, no period, under 50 chars.
- Body: wrap at 72 chars, explain *why* not just *what*.
- Each commit must pass all tests and lints independently.
- Squash fixups — the final patch should be the minimal diff needed.

## Dependencies

- Radicle crates use git dependencies for portability.
- Other dependencies are declared directly in `Cargo.toml`.
- Do not add dependencies without maintainer approval.

## Feature flags

- Currently none defined. A `test` feature may be added to expose test
  utilities for downstream crates.

## Documentation

- Document all public types and functions with `///` doc comments.
- Use `[`TypeName`]` cross-references in doc comments.
- Module-level docs use `//!`.
- Code comments should be full English sentences that add missing context,
  not restate what the code does.
- In tests, use RFC 2606 domains (e.g., `radicle.example.com`), not
  `radicle.xyz`.

## Architecture principles

1. **Type-level safety** — use the type system to make invalid states
   unrepresentable (verified/unverified, bounded collections, newtypes).
2. **Trait-based abstraction** — define storage, signing, and node interfaces
   as traits for testability and flexibility.
3. **Minimal public API** — keep protocol internals private; expose only what
   consumers need.
4. **No async** — synchronous code only.
5. **Composable errors** — layer error types with `#[from]` and
   `#[error(transparent)]` to preserve context across module boundaries.
6. **Minimal dependencies** — every dependency is a liability. Audit what
   you add.
