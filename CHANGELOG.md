# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.2.0] - 2026-02-27

### Added

- Short-form ID resolution — All COB types (contexts, issues, patches, plans)
  and commits now accept short prefix identifiers (minimum 7 hex characters)
  instead of requiring full IDs. A single generic `resolve_cob_prefix()`
  replaces the previous type-specific resolvers.
- Auto-populate `filesTouched` and `relatedCommits` on create — The CLI now
  automatically extracts touched files from HEAD and optionally links commits
  since a given ref, so agents no longer need to fill these fields manually.
  - `--no-auto-files` flag to opt out of auto-populating `filesTouched`
  - `--auto-link-commits <ref>` to link all commits since a given ref
- `verification` field — New optional structured field for recording
  pass/fail/skip results per check, attached to the COB open action.
- `taskId` field — New optional field that links a context back to the plan
  task that produced it.

### Changed

- Strict JSON input validation — `--json` input now uses `deny_unknown_fields`,
  so misspelled keys produce a clear error listing valid field names. Title,
  description, and approach are required to be non-empty.
- Validate full SHA in commit resolution — The `resolve_commit_sha` fast path
  now validates 40-character hex strings with `Oid::from_str` instead of
  trusting them blindly.

### Fixed

- `JsonContextInput` was missing `rename_all = "camelCase"`, causing camelCase
  fields to be silently ignored.

## [0.1.0]

- Initial release.
