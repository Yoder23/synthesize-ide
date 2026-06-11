# Synthesize v14.2 revision notes

V14.2 is a narrow governed-task hardening pass built on v14.1.

## What changed

- Fixed likely Rust test string issues by converting multiline test patches to raw string literals where needed.
- Bound approved task commands to the canonical repo root captured from the backend-detected task snapshot.
- `approve_task` now verifies the stored task snapshot repo root matches the requested repo root before approval.
- `run_approved_task` now verifies the approved command repo root matches the requested repo root before execution.
- Approved commands now persist `repo_root`, `requires_network`, and `may_modify_files`.
- `run_approved_task` reclassifies the stored command immediately before spawning and refuses commands that become network/destructive/blocked or whose risk changes.
- Task snapshot deletion is now scoped by `session_id + repo_root`, not just session.
- Manifest detection for Cargo/Python/Go tasks now uses `RepoGuard` rather than direct path checks.
- Added focused tests/helpers for task repo-root binding and guarded manifest detection.

## Safety model

Task execution now follows the same backend-owned pattern as patch application:

```text
detect_tasks
  -> backend persists task snapshot with canonical repo_root

approve_task
  -> frontend sends task_id only
  -> backend loads snapshot
  -> backend verifies repo binding
  -> backend reclassifies
  -> backend persists approved command snapshot

run_approved_task
  -> frontend sends command_id
  -> backend loads approved command
  -> backend verifies repo binding
  -> backend reclassifies again
  -> backend runs argv-only with guarded cwd, env scrub, timeout, bounded output, and audit
```

## Still required before production-ready claims

- Generate and commit `pnpm-lock.yaml`.
- Run `./scripts/release-check.sh` in a real Rust/pnpm environment.
- Smoke-test fake runtime, managed llama.cpp, manual local model server, patch apply/rollback, and governed tasks.

