# Synthesize v6 Revision Notes

V6 locks down the governed patch lifecycle so validation, approval, apply, checkpoint identity, rollback, lifecycle transitions, and audit are backend-owned.

## What changed

- Rollback no longer accepts a frontend-supplied `checkpoint_dir`.
- Rollback now accepts only `repo_root`, `proposal_id`, and `session_id`.
- Apply persists checkpoint identity in `patch_proposals` and `patch_checkpoints`.
- Added explicit lifecycle states:
  - `proposed`
  - `validated`
  - `rejected`
  - `approved`
  - `applying`
  - `applied`
  - `apply_failed`
  - `rolling_back`
  - `rolled_back`
  - `rollback_failed`
- Added a backend transition helper that rejects illegal state transitions.
- Apply now transitions `approved -> applying -> applied` or `apply_failed`.
- Rollback now transitions `applied -> rolling_back -> rolled_back` or `rollback_failed`.
- Added an in-process repo mutation lock for apply/rollback.
- Strengthened checkpoint manifest validation before rollback.
- Patch apply remains checkpoint/restore transaction-shaped and now records checkpoint identity durably.
- Frontend rollback now sends only `repo_root` and `proposal_id`; checkpoint IDs are display-only.
- Command execution remains disabled.
- Fake runtime remains the only active runtime.

## Backend-owned rollback

The frontend no longer provides a checkpoint path. The backend loads the applied proposal, reads the persisted checkpoint ID, verifies the checkpoint record, derives the checkpoint directory internally, validates the manifest, and then rolls back.

## Known limitations

- Fake runtime only.
- llama.cpp/GGUF runtime is not implemented.
- Command execution is disabled.
- No OS-level network sandbox exists yet.
- Repo mutation lock is in-process only, not cross-process.
- Filesystem transactionality is checkpoint/restore based, not OS atomic.
- The custom unified-diff applier is still intentionally limited.
- Approval remains whole-proposal only.
