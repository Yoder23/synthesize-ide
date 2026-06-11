# Synthesize v10 Revision Notes

Synthesize v10 focuses on daily dogfooding while preserving the backend-governed trust model.

## Trust-boundary fixes

- `runtime_generate` no longer accepts frontend-supplied model messages.
- Runtime generation now accepts a `context_bundle_id`; the backend loads the persisted bundle, verifies session/repo/classification, recomputes `messages_sha256`, and derives the exact messages internally.
- Runtime audit events include `context_bundle_id` and `messages_sha256`.
- Context bundles now persist and display `messages_sha256` and `exact_context`.
- Context file content reads go through `RepoGuard`; package metadata reads go through `RepoGuard`.
- File-tree traversal skips symlink entries and does not descend into symlinked directories.
- Non-local endpoints require backend-persisted approval before repo context is sent.

## Dogfood UX improvements

- Runtime Control shows backend endpoint approval state.
- Context Visibility shows exact persisted context, destination classification, prompt/messages hash, and whether repo context may leave the machine.
- Chat uses backend-owned context bundles and backend runtime generation.
- Diff Queue remains keyed by `proposal_id` and keeps validation/approval/apply/rollback state separate per proposal.
- Trust footer now states that backend runtime calls derive prompts from persisted context bundles.

## Safety model retained

- Command execution remains disabled.
- Apply uses backend-persisted proposal snapshots only.
- Rollback uses backend-bound checkpoint identity only.
- Patch approval remains backend-owned.
- Repo mutation lock remains in-process.

## Build note

This environment does not provide Cargo or pnpm, so Rust/TypeScript builds could not be fully verified here. JSON files were validated and the archive was produced successfully.
