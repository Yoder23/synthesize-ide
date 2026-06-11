# Revision Notes V5 — Backend-Authoritative Governed Patch Loop

V5 intentionally does **not** add llama.cpp, GGUF loading, command execution, or broad new UI surface. The milestone is backend authority for patch validation, approval, application, and rollback.

## What changed

- Fixed the Tauri/Cargo project shape:
  - removed the stale `[lib]` declaration from `apps/desktop/src-tauri/Cargo.toml`, because this app is currently a binary-only Tauri backend;
  - added `apps/desktop/src-tauri/build.rs` calling `tauri_build::build()`.
- Added durable backend-owned patch proposal storage in SQLite:
  - `patch_proposals`
  - `patch_files`
  - `patch_approvals`
- Changed `validate_patch_proposal` so validation snapshots the proposal in backend storage and computes a canonical `operation_sha256`.
- Added `approve_patch_proposal`.
- Changed `apply_approved_patch` so it accepts only `repo_root`, `proposal_id`, and `approval_id`; it reloads the persisted snapshot and never trusts a frontend-sent patch body at apply time.
- Made patch application transaction-shaped across multiple files:
  - validate all target files first;
  - read originals;
  - apply all diffs in memory;
  - reject no-op files;
  - create checkpoint manifest before writes;
  - write files;
  - attempt checkpoint restore if any write fails.
- Extended checkpoint manifests for created files:
  - tracks `existed_before`, `backup_path`, and `before_sha256`;
  - rollback restores existing files and deletes files created by the patch.
- Updated the frontend flow:
  - Validate + snapshot;
  - Approve persisted proposal;
  - Apply approved snapshot;
  - Rollback checkpoint.
- Kept command execution disabled. Command classification remains visible only.
- Kept fake runtime as the deterministic milestone runtime. Real llama.cpp/GGUF loading is intentionally not implemented in v5.

## Trust footer update

The footer now reflects enforceable v5 behavior:

- Runtime: fake wired; llama.cpp/GGUF not implemented in v5
- Command execution: disabled; classification only
- Repo boundary: enforced for read/validate/approve/apply
- Patch approval: backend-owned
- Patch apply: checkpointed transaction

## Known limitations

- The filesystem transaction is checkpoint/restore based, not OS-level atomic.
- The unified-diff applier is still intentionally minimal and text-file focused.
- Real command sandboxing is not implemented.
- Real model runtime loading is not implemented.
- Existing v4 `.synthesize` databases are migrated by renaming legacy patch tables, but this is a lightweight compatibility bridge rather than a full migration system.
