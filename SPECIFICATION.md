# me.hdh.context COB Specification

## Overview

The `me.hdh.context` Collaborative Object (COB) type stores observation records from AI-assisted development sessions. Contexts give session metadata and development directives a first-class place in the Radicle network — replicatable, linkable to patches, issues, and plans, and governed by delegate consensus.

## Motivation

AI-assisted development produces three tiers of artifacts:

1. **Session Metadata** — Who did the work, with what tools, at what cost. Lightweight, always travels with the code via COB gossip.
2. **Development Directives** — What was intended and what was achieved. The curated record of decisions. Medium weight, optionally embedded.
3. **Full Session Transcripts** — The raw audit trail. Heavy, stays in cold storage, referenced by ID only.

The Context COB captures tiers 1 and 2. Full transcripts (tier 3) can be attached as embedded git blobs when needed.

## Type Name

```
me.hdh.context
```

Following the reverse domain notation pattern used by Radicle COBs (e.g., `xyz.radicle.issue`, `xyz.radicle.patch`).

## Design Principles

- **Agent-first.** Every field is ranked by utility to coding agents. The top-tier fields (learnings, approach, constraints, friction) all directly improve agent outcomes. Fields that serve only human management needs are deferred.
- **Minimal, complementary to Plan.** Context captures what Plans can't. Intent and outcome belong on the Plan COB. Context links back to Plans for the "what."
- **Immutable observations.** All core fields are set at creation and never modified. No CRDT conflicts on observational data.
- **Radicle-native linking.** Bidirectional links to issues, patches, and plans are the primary mechanism for connecting context to the work it describes.
- **No lifecycle status.** Contexts are observations — they exist or they don't. There is no draft/active/closed workflow.

## Data Model

### Context State

```rust
struct Context {
    title: String,                         // Brief session identifier
    description: String,                   // Free-form (for standalone contexts)
    approach: String,                      // Reasoning chain and decisions
    constraints: Vec<String>,              // Forward-looking assumptions
    learnings: LearningsSummary,           // Codebase discoveries
    friction: Vec<String>,                 // Problems encountered
    open_items: Vec<String>,               // Unfinished work, tech debt
    files_touched: BTreeSet<String>,       // Files actually modified
    related_commits: BTreeSet<String>,     // Git commit SHAs (mutable)
    related_issues: BTreeSet<ObjectId>,    // Linked issues (mutable)
    related_patches: BTreeSet<ObjectId>,   // Linked patches (mutable)
    related_plans: BTreeSet<ObjectId>,     // Linked plans (mutable)
    author: Author,                        // Radicle identity
    created_at: Timestamp,                 // Creation time
}
```

### Field Semantics

#### Immutable Fields (set at creation, never modified)

| Field | Purpose |
|-------|---------|
| `title` | Brief identifier for the session |
| `description` | Free-form description for standalone contexts without a plan |
| `approach` | What approaches were considered, what was tried, why the chosen path won, what alternatives were rejected, deliberate design decisions |
| `constraints` | Forward-looking assumptions the work depends on — "valid as long as X remains true" |
| `learnings` | What was discovered about the codebase (see LearningsSummary) |
| `friction` | What went wrong — specific, past-tense, actionable problems |
| `open_items` | Unfinished work, tech debt introduced, known gaps |
| `files_touched` | Which files were actually modified during the session |

#### Mutable Fields (set operations only)

| Field | Purpose |
|-------|---------|
| `related_commits` | Git commit SHAs this context produced |
| `related_issues` | Linked Radicle issue COBs |
| `related_patches` | Linked Radicle patch COBs |
| `related_plans` | Linked Radicle plan COBs |

### Key Fields Explained

**`approach`** — The reasoning chain. What approaches were considered, what was tried, why the chosen path won and alternatives were rejected. Also captures deliberate design decisions ("chose not to add X because Y"). A future agent working on the same area benefits enormously from this — it prevents wasted exploration and second-guessing intentional choices.

**`constraints`** — Forward-looking assumptions the work depends on. "The retry logic assumes the HTTP timeout is 30s" or "this assumes the User struct has an email field." When a parallel agent modifies something a constraint depends on, they can see the dependency. Nothing else in the model captures this "valid as long as" relationship.

**`friction`** — What went wrong during the session. "Borrow checker issues with async retry closures" tells a future agent to take a different approach in that area. Past-tense, specific, actionable.

### LearningsSummary

```rust
struct LearningsSummary {
    repo: Vec<String>,          // Repository-level patterns and conventions
    code: Vec<CodeLearning>,    // File-specific findings
}

struct CodeLearning {
    path: String,               // File path relative to repo root
    line: Option<u32>,          // Optional start line
    end_line: Option<u32>,      // Optional end line
    finding: String,            // What was discovered
}
```

### Deferred Fields

The following are excluded from the initial model. They can be added later as backward-compatible optional fields (`#[serde(default)]`) when use cases demand them:

| Field | Reason Deferred |
|-------|----------------|
| `agent` (which AI tool) | Author DID identifies who created the context; tool identity is metadata that can be added if human stakeholders require it |
| Session metrics (tokens, API calls) | Cost accounting — no agent utility |
| Attribution (agent vs human lines) | Code review provenance — no agent utility |
| Thread / comments | Discussion can happen on the linked plan/issue/patch |
| Labels | Classification can be added when needed |
| `learnings.workflow` | Process observations are mostly human-facing |

## Actions

Actions are the operations that can be applied to a Context COB. Each action is serialized as JSON and stored in the change history.

### Creation Action

| Action | Description | Authorization |
|--------|-------------|---------------|
| `open` | Create new context with all immutable fields | Any user |

### Linking Actions

| Action | Description | Authorization |
|--------|-------------|---------------|
| `link.commit` | Link a git commit SHA | Author or delegate |
| `unlink.commit` | Remove a commit link | Author or delegate |
| `link.issue` | Link a Radicle issue | Author or delegate |
| `unlink.issue` | Remove an issue link | Author or delegate |
| `link.patch` | Link a Radicle patch | Author or delegate |
| `unlink.patch` | Remove a patch link | Author or delegate |
| `link.plan` | Link a Radicle plan | Author or delegate |
| `unlink.plan` | Remove a plan link | Author or delegate |

No comment, label, edit, or status actions are defined. Contexts are immutable observations — discussion and classification happen on linked objects.

No action produces a sub-identifier (`produces_identifier()` returns `false` for all actions).

## Action JSON Schemas

### Open Action

```json
{
  "type": "open",
  "title": "Implement auth flow",
  "description": "Session to add OAuth support",
  "approach": "Used passport.js for OAuth, rejected manual token handling because...",
  "constraints": ["Assumes Redis is available for session storage"],
  "learnings": {
    "repo": ["Uses conventional commits"],
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
  "filesTouched": ["src/auth.rs", "src/middleware.rs"],
  "embeds": []
}
```

### Link Commit Action

```json
{
  "type": "link.commit",
  "sha": "abc1234def5678"
}
```

### Link Issue Action

```json
{
  "type": "link.issue",
  "issueId": "d96f02665bf896324e2f0a4c18d08a768135ef2e"
}
```

### Link Plan Action

```json
{
  "type": "link.plan",
  "planId": "d96f02665bf896324e2f0a4c18d08a768135ef2e"
}
```

## Storage

Contexts are stored under the Git refs namespace:

```
refs/cobs/me.hdh.context/<CONTEXT-ID>
```

Each Context ID is a content-addressed identifier derived from the initial change commit.

## CRDT Semantics

Like other Radicle COBs, Contexts use operation-based CRDTs:

1. **Operations are commutative**: Can be applied in any order to reach the same final state
2. **Deterministic ordering**: Topological sort of the DAG ensures consensus
3. **Offline-first**: Full local functionality, sync on reconnection

### Conflict Resolution

Because core fields are immutable (set only by the `open` action), most CRDT conflict scenarios are eliminated by design. The only mutable fields are the link sets:

- **Sets (commits, issues, patches, plans)**: Union of all additions, intersection of removals — standard OR-set semantics

## Authorization Model

Follows the same model as other Radicle COBs:

1. **Repository delegates** can perform all actions on any context
2. **Context author** can perform link/unlink operations on their own context
3. No comment or label actions exist, so no additional authorization tiers are needed

## Relationship to Plan COBs

| Aspect | Plan COB (`me.hdh.plan`) | Context COB (`me.hdh.context`) |
|--------|--------------------------|-------------------------------|
| Purpose | Coordination | Observation |
| Content | Intent, outcome, task breakdown | Approach, learnings, constraints, friction |
| Lifecycle | Draft → Approved → InProgress → Completed → Archived | Created (no status transitions) |
| Mutability | Most fields mutable | Core fields immutable, links mutable |
| Discussion | Built-in thread | Delegated to linked objects |

When a Context is linked to a Plan:
- The **"what"** comes from the Plan (intent, tasks, status)
- The **"how"** comes from the Context (approach, learnings, friction)

For standalone sessions without a plan, the Context's `description` field provides minimal framing of intent.

## CLI Usage

```bash
# Create a context from flags
rad-context create "Implement auth flow" \
  --approach "Used passport.js for OAuth" \
  --constraint "Assumes Redis for sessions" \
  --friction "Async middleware type errors"

# Create from JSON (for programmatic/agent use)
cat context.json | rad-context create --json

# List all contexts
rad-context list

# Show details
rad-context show <context-id>
rad-context show <context-id> --json

# Link to commits, issues, patches, plans
rad-context link <context-id> --commit abc1234
rad-context link <context-id> --issue <issue-id>
rad-context link <context-id> --plan <plan-id>

# Unlink
rad-context unlink <context-id> --commit abc1234
```

## Migration Path

For potential upstream inclusion:

1. **Phase 1**: Prototype as `me.hdh.context` in this repository
2. **Phase 2**: Gather community feedback via Radicle Zulip
3. **Phase 3**: If traction, propose RFC for `xyz.radicle.context`
4. **Phase 4**: Port to heartwood patterns and submit PR

## References

- [Radicle Protocol Overview](https://hackmd.io/@radicle/rJ2UH54P6)
- [radicle-cob crate](https://docs.rs/radicle-cob/)
- [heartwood repository](https://github.com/radicle-dev/heartwood)
- [radicle-plan-cob](../radicle-plan-cob/) — companion Plan COB type
