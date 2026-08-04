# Contributor notes

This crate defines a Radicle Context COB and the `rad-context` CLI.
Keep changes small, synchronous, and easy to bisect.

## Project layout

- `src/lib.rs` — library root, COB registration, store, errors
- `src/state.rs` — `Context` state and accessors
- `src/actions.rs` — COB mutation `Action`s
- `src/main.rs` — CLI and inline CLI tests

Minimum Rust version: **1.85.0**.

## Required checks

Every commit should pass:

```sh
cargo build
cargo clippy --tests
cargo fmt --check
cargo test
cargo doc --no-deps
```

## Rust standards

- Library code uses typed errors with `thiserror`; CLI entry points use
  `anyhow::Result`.
- Public error enums should be `#[non_exhaustive]`.
- Public types and functions need `///` docs.
- Public accessors and constructors should use `#[must_use]` when ignoring the
  result would likely be a bug.
- Prefer `where` clauses for multi-bound generics.
- Do not add async; keep the crate synchronous.
- Use `log` rather than `tracing` if logging is needed.

## Panic and indexing policy

- Do not panic in non-test library code.
- Avoid direct indexing (`value[i]`, `s[..n]`); use `.get()`, iterators, or
  pattern matching.
- In non-test code, any `unwrap`/`expect` needs a nearby `// SAFETY:` comment
  explaining the invariant.
- In tests, `unwrap` is acceptable for straightforward fixture setup; prefer
  `expect`/`expect_err` when it improves failure diagnostics.

## Clippy policy

The lint policy lives in `[lints.clippy]` in `Cargo.toml`. Keep it in sync with
these notes. In particular, the project denies direct indexing, wildcard enum
match arms, `must_use_candidate`, and other Heartwood-style footgun lints.

Use narrowly-scoped `#[allow(...)]` only when the alternative would make the API
or protocol representation worse; include a short reason when it is not obvious.

## Serialization and protocol shape

- Derive `Serialize`/`Deserialize` with serde.
- Use camelCase JSON fields unless compatibility requires otherwise.
- Treat COB action/state shapes as protocol-facing. Avoid changing serialized
  forms accidentally, and document intentional compatibility breaks.

## Dependencies

- Radicle crates are git dependencies for portability.
- Other dependencies are declared directly in `Cargo.toml`.
- Do not add dependencies without a clear reason; mention that reason in the
  commit or patch description.

## Tests

- Keep unit tests inline with the source file they cover.
- Use RFC 2606 domains such as `radicle.example.com` in tests.
- Prefer tests that document behavior over tests that mirror implementation
  details.

## Commit messages

Use `<scope>: <Imperative summary>`.

Common scopes: `lib`, `cli`, `state`, `actions`, `docs`, `ci`, `chore`.
Keep the subject under 50 characters when practical; explain non-obvious
motivation in the body.
