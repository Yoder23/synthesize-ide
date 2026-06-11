# Revision Notes V7 — Patch Lifecycle Hardening

V7 is a targeted hardening pass on top of v6. It does not add llama.cpp, model downloading, embeddings, autocomplete, command execution, or new product surface.

## Changes

- Fixed `RepoGuard::resolve_for_write_path` so new files can be created inside new nested directories.
- `RepoGuard` now canonicalizes the nearest existing ancestor, rejects traversal/absolute/prefix components, then rebuilds the intended target path under the repo.
- Added/updated tests for nested new-file writes and rollback deletion of created nested files.
- Made approval insertion and `validated -> approved` lifecycle transition atomic in a single SQLite transaction.
- Made apply more robust when checkpoint persistence, status transition, or audit finalization fails after file writes: Synthesize attempts checkpoint restore, transitions to `apply_failed`, and audits the failure best-effort.
- Made rollback finalization transaction-shaped: status transition to `rolled_back` and `patch.rolled_back` audit event commit together. If finalization fails after restore, Synthesize attempts to mark `rollback_failed` and returns a clear error.
- Canonicalized repo-root comparisons for checkpoint records instead of comparing raw path strings.
- Cleaned stale v5 UI/backend messages in command classification and runtime/model scaffolding.
- Added unit-test coverage for approval transaction rollback on audit failure and canonical repo-root comparison.

## Known limitations

- Cargo is not available in this execution environment, so Rust tests/checks could not be run here.
- `pnpm` is not available in this execution environment, so frontend build/typecheck could not be run here.
- The unified-diff applier remains custom and limited.
- Filesystem transactionality remains checkpoint/restore based, not OS-atomic.
- Repo mutation lock remains in-process only, not cross-process.
- Command execution remains disabled.
- Real llama.cpp/GGUF runtime remains unimplemented.

## Next hardening target

Before real model/runtime work, replace or deeply harden the custom unified-diff applier and add CI-backed lifecycle integration tests that exercise DB failure, post-write restore, rollback failure, and multi-file transactional behavior.
