# radicle-context-cob

A Radicle Collaborative Object (COB) type for storing AI session context.

Contexts are observation records from development sessions — what was tried, what was learned, what constraints exist, and what went wrong. They complement Plan COBs: Plans capture *coordination* (what should be done), Contexts capture *observation* (how the work was done).

## Installation

```bash
cargo install --path .
```

This installs the `rad-context` binary to your Cargo bin directory.

## Usage

### Create a context

```bash
# From flags
rad-context create "Implement auth flow" \
  --description "Session to add OAuth support" \
  --approach "Used passport.js for OAuth, rejected manual token handling" \
  --constraint "Assumes Redis is available for session storage" \
  --friction "Type errors with async middleware closures" \
  --open-item "Refresh token rotation not implemented" \
  --file src/auth.rs --file src/middleware.rs
```

### Create from JSON (programmatic use)

```bash
cat <<'EOF' | rad-context create --json
{
  "title": "Implement auth flow",
  "description": "Session to add OAuth support",
  "approach": "Used passport.js for OAuth, rejected manual token handling",
  "constraints": ["Assumes Redis is available for session storage"],
  "learnings": {
    "repo": ["Uses conventional commits", "Error types follow thiserror pattern"],
    "code": [
      {
        "path": "src/auth.rs",
        "line": 42,
        "finding": "Auth middleware expects Request to carry session state"
      }
    ]
  },
  "friction": ["Type errors with async middleware closures"],
  "openItems": ["Refresh token rotation not implemented"],
  "filesTouched": ["src/auth.rs", "src/middleware.rs"]
}
EOF
```

### List contexts

```bash
rad-context list
```

### Show context details

```bash
rad-context show <context-id>
rad-context show <context-id> --json
```

### Link to commits, issues, patches, or plans

```bash
rad-context link <context-id> --commit abc1234
rad-context link <context-id> --issue <issue-id>
rad-context link <context-id> --patch <patch-id>
rad-context link <context-id> --plan <plan-id>
```

### Unlink

```bash
rad-context unlink <context-id> --commit abc1234
rad-context unlink <context-id> --issue <issue-id>
```

## COB Type

The COB type name is `me.hdh.context` following Radicle's reverse domain notation pattern.

See [SPECIFICATION.md](SPECIFICATION.md) for full documentation of the data model, actions, and design rationale.

## Relationship to Plan COBs

Context COBs complement Plan COBs — they don't duplicate them:

- **Plan COB** (`me.hdh.plan`) = *coordination* — what should be done, task breakdown, status tracking, intent, outcome
- **Context COB** (`me.hdh.context`) = *observation* — how the work was done, what was learned, what constraints exist, what went wrong

Intent and outcome live on the Plan. Context captures everything the Plan can't. When a Context is linked to a Plan, the "what" comes from the Plan and the "how" comes from the Context. For standalone sessions without a plan, the `description` field provides minimal framing.

## Local Development

For local development with a heartwood checkout, create `.cargo/config.toml`:

```toml
[patch."https://seed.radicle.xyz/z3gqcJUoA1n9HaHKufZs5FCSGazv5.git"]
radicle = { path = "../heartwood/crates/radicle" }
radicle-cob = { path = "../heartwood/crates/radicle-cob" }
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
